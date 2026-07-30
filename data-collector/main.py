"""Orquestrador da coleta de dados (Fase 2).

Lê config.yaml, chama cada fonte configurada e grava os resultados direto no
SQLite compartilhado com o app Tauri. Chamado pelo comando Rust
`run_stock_collector` (src-tauri/src/commands/collector.rs) como subprocess,
mas também roda direto (`python3 main.py`) pra depurar sem precisar do app.
"""

import sqlite3
import sys
from datetime import datetime, timezone
from pathlib import Path

import yaml
from dotenv import load_dotenv

from sources import (
    acoes_bolsai,
    acoes_yahoo,
    cripto_coingecko,
    cripto_defillama,
    cripto_ultrasound,
    cvm_dfp,
)

BASE_DIR = Path(__file__).parent
DB_PATH = BASE_DIR / "practice_valuation.db"
CONFIG_PATH = BASE_DIR / "config.yaml"


def load_config() -> dict:
    with open(CONFIG_PATH) as f:
        return yaml.safe_load(f)


def collect_stock_quotes(tickers: list[str]) -> list[dict]:
    if not tickers:
        return []

    quotes = acoes_yahoo.fetch_quotes(tickers)

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    now = datetime.now(timezone.utc).isoformat()
    conn.executemany(
        "INSERT INTO stock_quotes (ticker, price, name, exchange, currency, source, fetched_at) "
        "VALUES (?, ?, ?, ?, ?, ?, ?)",
        [
            (
                quote["ticker"],
                quote["price"],
                quote["name"],
                quote["exchange"],
                quote["currency"],
                "yahoo_finance",
                now,
            )
            for quote in quotes
        ],
    )
    conn.commit()
    conn.close()

    return quotes


def collect_stock_fundamentals(tickers: list[str]) -> list[dict]:
    """lpa/vpa/shares_outstanding/cvm_code vêm da bolsai (conferidos OK), mas
    o `roe` dela mistura lucro trimestral com TTM dependendo da empresa sem
    avisar qual é qual (achado real testando o BPAC11, Sessão 16: bolsai
    devolveu 3,54% quando o real reportado é 26,6%) — por isso `roe` é
    sobrescrito pelo cálculo direto na CVM (`cvm_dfp.fetch_roe`, mesma
    fonte/zip que `collect_stock_dcf_fundamentals` já usa pro DCF).
    Ticker sem ROE extraível na CVM é descartado inteiro, não só o roe —
    evita reintroduzir silenciosamente o valor da bolsai que está sendo
    corrigido aqui.

    `payout` (Sessão 16) é diferente: nunca teve fonte automática nenhuma
    (campo 100% manual no formulário Banks), então é só um acréscimo — um
    ticker sem payout extraível na CVM (`cvm_dfp.fetch_payout`) continua
    sendo gravado normalmente, só com `payout=None` (vira NULL), pra não
    perder lpa/vpa/roe por causa de um caso de borda no payout.
    """
    if not tickers:
        return []

    fundamentals = acoes_bolsai.fetch_fundamentals(tickers)

    ticker_cvm_codes = {f["ticker"]: f["cvm_code"] for f in fundamentals}
    roe_by_ticker = {
        item["ticker"]: item["roe"] for item in cvm_dfp.fetch_roe(ticker_cvm_codes)
    }
    payout_by_ticker = {
        item["ticker"]: item["payout"]
        for item in cvm_dfp.fetch_payout(ticker_cvm_codes)
    }

    fundamentals = [f for f in fundamentals if f["ticker"] in roe_by_ticker]
    for item in fundamentals:
        item["roe"] = roe_by_ticker[item["ticker"]]
        item["payout"] = payout_by_ticker.get(item["ticker"])

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    now = datetime.now(timezone.utc).isoformat()
    conn.executemany(
        "INSERT INTO stock_fundamentals (ticker, lpa, vpa, roe, payout, source, fetched_at) "
        "VALUES (?, ?, ?, ?, ?, ?, ?)",
        [
            (
                item["ticker"],
                item["lpa"],
                item["vpa"],
                item["roe"],
                item["payout"],
                "bolsai+cvm_dfp",
                now,
            )
            for item in fundamentals
        ],
    )
    conn.commit()
    conn.close()

    return fundamentals


def collect_stock_dividends_avg(tickers: list[str]) -> list[dict]:
    if not tickers:
        return []

    dividends = acoes_yahoo.fetch_dividends_avg(tickers)

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    now = datetime.now(timezone.utc).isoformat()
    conn.executemany(
        "INSERT INTO stock_dividends_avg (ticker, avg_dividend_5y, source, fetched_at) "
        "VALUES (?, ?, ?, ?)",
        [
            (item["ticker"], item["avg_dividend_5y"], "yahoo_finance", now)
            for item in dividends
        ],
    )
    conn.commit()
    conn.close()

    return dividends


def collect_stock_dcf_fundamentals(fundamentals: list[dict]) -> list[dict]:
    """Recebe a lista já buscada por `collect_stock_fundamentals` (que já
    inclui `cvm_code` e `shares_outstanding`, ver `acoes_bolsai.py`) e
    completa com os campos vindos da CVM (EBIT, D&A, Capex, ΔNWC, dívida,
    caixa). `shares_outstanding` vem da bolsai, não da CVM — ver nota em
    `cvm_dfp.py`.
    """
    ticker_cvm_codes = {f["ticker"]: f["cvm_code"] for f in fundamentals}
    if not ticker_cvm_codes:
        return []

    shares_outstanding_millions = {
        f["ticker"]: f["shares_outstanding"] / 1_000_000 for f in fundamentals
    }

    records = cvm_dfp.fetch_dcf_fundamentals(ticker_cvm_codes)
    for record in records:
        record["shares_outstanding"] = shares_outstanding_millions[record["ticker"]]

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    now = datetime.now(timezone.utc).isoformat()
    conn.executemany(
        "INSERT INTO stock_dcf_fundamentals (ticker, reference_year, ebit, "
        "depreciation_amortization, capex, nwc_change, total_debt, cash, "
        "shares_outstanding, source, fetched_at, tax_rate, revenue, inventory) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        [
            (
                r["ticker"],
                r["reference_year"],
                r["ebit"],
                r["depreciation_amortization"],
                r["capex"],
                r["nwc_change"],
                r["total_debt"],
                r["cash"],
                r["shares_outstanding"],
                "cvm_dfp",
                now,
                r["tax_rate"],
                r["revenue"],
                r["inventory"],
            )
            for r in records
        ],
    )
    conn.commit()
    conn.close()

    return records


def collect_stock_technicals(tickers: list[str]) -> list[dict]:
    if not tickers:
        return []

    technicals = acoes_yahoo.fetch_technicals(tickers)

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    now = datetime.now(timezone.utc).isoformat()
    conn.executemany(
        "INSERT INTO stock_technicals (ticker, sma_50, sma_100, sma_200, "
        "cagr_5y, cagr_10y, source, fetched_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        [
            (
                item["ticker"],
                item["sma_50"],
                item["sma_100"],
                item["sma_200"],
                item["cagr_5y"],
                item["cagr_10y"],
                "yahoo_finance",
                now,
            )
            for item in technicals
        ],
    )
    conn.commit()
    conn.close()

    return technicals


def collect_stock_dividend_payments(tickers: list[str]) -> list[dict]:
    """Grava o histórico de pagamentos individuais (não o resumo em
    `stock_dividends_avg`) — usa `INSERT OR IGNORE` num índice único
    `(ticker, payment_date)` (ver migration) em vez do `INSERT` simples do
    resto do coletor, porque isso é fato histórico que não muda: rodar de
    novo (ex. botão "Refresh data" da tela) não deve duplicar pagamento já
    gravado, só acrescentar um pagamento novo desde a última coleta.
    """
    if not tickers:
        return []

    payments = acoes_yahoo.fetch_dividend_payments(tickers)

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    now = datetime.now(timezone.utc).isoformat()
    changes_before = conn.total_changes
    conn.executemany(
        "INSERT OR IGNORE INTO stock_dividend_payments "
        "(ticker, payment_date, amount, price_at_payment, yield_pct, source, fetched_at) "
        "VALUES (?, ?, ?, ?, ?, ?, ?)",
        [
            (
                item["ticker"],
                item["payment_date"],
                item["amount"],
                item["price_at_payment"],
                item["yield_pct"],
                "yahoo_finance",
                now,
            )
            for item in payments
        ],
    )
    # `INSERT OR IGNORE` faz `executemany` silenciosamente pular duplicatas —
    # `conn.total_changes` (não `cursor.rowcount`, que não é confiável com
    # `executemany`) é o jeito certo de saber quantas linhas realmente novas
    # entraram, pro print de baixo não anunciar "atualizado" pagamento que já
    # estava salvo.
    new_count = conn.total_changes - changes_before
    conn.commit()
    conn.close()

    print(
        f"Fetched {len(payments)} dividend payment(s) from source, "
        f"{new_count} new (rest already saved)"
    )
    return payments


def collect_price_history(tickers: list[str]) -> list[dict]:
    """Backfill da série diária de fechamento (Fase 10.3, base do TWR).

    Usa `INSERT OR IGNORE` num índice único `(ticker, price_date)` (ver
    migration `create_stock_price_history`), mesmo padrão de
    `collect_stock_dividend_payments` — rodar o backfill de novo não
    duplica pregão já salvo, só acrescenta dia novo (ex: rodar de novo
    daqui a um mês só grava as ~20 novas linhas desde a última vez).
    """
    if not tickers:
        return []

    prices = acoes_yahoo.fetch_price_history(tickers)

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    now = datetime.now(timezone.utc).isoformat()
    changes_before = conn.total_changes
    conn.executemany(
        "INSERT OR IGNORE INTO stock_price_history "
        "(ticker, price_date, close_price, source, fetched_at) VALUES (?, ?, ?, ?, ?)",
        [
            (item["ticker"], item["price_date"], item["close_price"], "yahoo_finance", now)
            for item in prices
        ],
    )
    new_count = conn.total_changes - changes_before
    conn.commit()
    conn.close()

    print(f"Fetched {len(prices)} daily price point(s), {new_count} new (rest already saved)")
    return prices


def main_price_history(tickers: list[str]) -> int:
    load_dotenv(BASE_DIR / ".env")
    collect_price_history(tickers)
    return 0


def _classify_signal(raw_value: float, green_boundary: float, red_boundary: float) -> str:
    """Mirrors `src-tauri/src/domain/crypto_score.rs::classify`.

    Duplicated here (not called via Tauri) because Python writes straight to
    the shared SQLite file with no IPC into Rust — same "no API between the
    two processes" architecture used for every other collector table. Keep
    both in sync if the classification rule ever changes.
    """
    higher_is_better = green_boundary > red_boundary
    if higher_is_better:
        if raw_value >= green_boundary:
            return "GREEN"
        if raw_value <= red_boundary:
            return "RED"
        return "NEUTRAL"
    if raw_value <= green_boundary:
        return "GREEN"
    if raw_value >= red_boundary:
        return "RED"
    return "NEUTRAL"


def _record_crypto_indicator(indicator: str, source: str, raw_value: float) -> dict:
    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")

    threshold = conn.execute(
        "SELECT green_boundary, red_boundary FROM indicator_thresholds WHERE indicator = ?",
        (indicator,),
    ).fetchone()
    if threshold is None:
        conn.close()
        raise RuntimeError(f"No threshold configured for '{indicator}' — run migrations first")
    green_boundary, red_boundary = threshold

    signal = _classify_signal(raw_value, green_boundary, red_boundary)
    reading_date = datetime.now(timezone.utc).date().isoformat()
    now = datetime.now(timezone.utc).isoformat()

    conn.execute(
        "INSERT INTO crypto_indicators "
        "(coin, indicator, reading_date, raw_value, signal, source, created_at) "
        "VALUES (?, ?, ?, ?, ?, ?, ?)",
        ("ETH", indicator, reading_date, raw_value, signal, source, now),
    )
    conn.commit()
    conn.close()

    return {"indicator": indicator, "raw_value": raw_value, "signal": signal}


def collect_crypto_tvl_trend() -> dict:
    raw_value = cripto_defillama.fetch_tvl_trend_mom()
    return _record_crypto_indicator("tvl_trend", "defillama", raw_value)


def collect_crypto_net_issuance() -> dict:
    raw_value = cripto_ultrasound.fetch_net_issuance_annualized_pct()
    return _record_crypto_indicator("net_issuance", "ultrasound.money", raw_value)


def collect_crypto_fees_vs_emission() -> dict:
    raw_value = cripto_ultrasound.fetch_fees_vs_emission_ratio()
    return _record_crypto_indicator("fees_vs_emission", "ultrasound.money", raw_value)


def collect_crypto_nvt_ratio() -> dict:
    raw_value = cripto_coingecko.fetch_nvt_ratio_vs_ma90()
    return _record_crypto_indicator("nvt_ratio", "coingecko", raw_value)


def main_crypto() -> int:
    tvl = collect_crypto_tvl_trend()
    print(f"TVL Trend (MoM): {tvl['raw_value']:.2f}% -> {tvl['signal']}")

    net_issuance = collect_crypto_net_issuance()
    print(
        f"Net Issuance (annualized): {net_issuance['raw_value']:.2f}% -> {net_issuance['signal']}"
    )

    fees_vs_emission = collect_crypto_fees_vs_emission()
    print(
        f"Fees vs Emission: {fees_vs_emission['raw_value']:.4f} -> {fees_vs_emission['signal']}"
    )

    nvt_ratio = collect_crypto_nvt_ratio()
    print(f"NVT Ratio (vs MA90): {nvt_ratio['raw_value']:.4f} -> {nvt_ratio['signal']}")
    return 0


def main(ticker: str | None = None) -> int:
    load_dotenv(BASE_DIR / ".env")

    if ticker:
        tickers = [ticker]
    else:
        config = load_config()
        tickers = config.get("stocks", [])
        if not tickers:
            print("No stocks configured in config.yaml")
            return 0

    quotes = collect_stock_quotes(tickers)
    for quote in quotes:
        print(f"{quote['ticker']}: R$ {quote['price']}")
    print(f"Updated {len(quotes)} quote(s)")

    # bolsai requires a signed-up API key — if it's missing, skip everything
    # that depends on it (dividends, DCF via cvm_code) instead of failing the
    # whole run and losing the quotes collected above.
    try:
        fundamentals = collect_stock_fundamentals(tickers)
        for item in fundamentals:
            payout = item["payout"]
            print(
                f"{item['ticker']}: LPA {item['lpa']} / VPA {item['vpa']} / "
                f"ROE {item['roe']}% / Payout {'n/a' if payout is None else f'{payout}%'}"
            )
        print(f"Updated {len(fundamentals)} fundamentals record(s)")
    except RuntimeError as err:
        print(f"Skipping bolsai collection: {err}")
        fundamentals = []

    if not fundamentals:
        return 0

    # Yahoo Finance's chart API already skips failing/dividend-less tickers
    # internally (see acoes_yahoo.py) — no try/except needed here, unlike
    # the bolsai calls above.
    dividends = collect_stock_dividends_avg(tickers)
    for item in dividends:
        print(f"{item['ticker']}: avg dividend/share (5y) R$ {item['avg_dividend_5y']:.4f}")
    print(f"Updated {len(dividends)} dividend average record(s)")

    dcf_fundamentals = collect_stock_dcf_fundamentals(fundamentals)
    for item in dcf_fundamentals:
        da = item["depreciation_amortization"]
        capex = item["capex"]
        tax_rate = item["tax_rate"]
        print(
            f"{item['ticker']}: EBIT {item['ebit']:.1f} / "
            f"Tax rate {'n/a' if tax_rate is None else f'{tax_rate:.1f}%'} / "
            f"D&A {'n/a' if da is None else f'{da:.1f}'} / "
            f"Capex {'n/a' if capex is None else f'{capex:.1f}'} / "
            f"ΔNWC {item['nwc_change']:.1f} / Debt {item['total_debt']:.1f} / "
            f"Cash {item['cash']:.1f} / Revenue {item['revenue']:.1f} / "
            f"Inventory {item['inventory']:.1f} (R$ millions)"
        )
    print(f"Updated {len(dcf_fundamentals)} DCF fundamentals record(s)")

    technicals = collect_stock_technicals(tickers)
    for item in technicals:
        sma_200 = item["sma_200"]
        cagr_10y = item["cagr_10y"]
        print(
            f"{item['ticker']}: SMA200 "
            f"{'n/a' if sma_200 is None else f'R$ {sma_200:.2f}'} / "
            f"CAGR 10y {'n/a' if cagr_10y is None else f'{cagr_10y:.1f}%'}"
        )
    print(f"Updated {len(technicals)} technicals record(s)")

    collect_stock_dividend_payments(tickers)

    return 0


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "crypto":
        sys.exit(main_crypto())
    if len(sys.argv) > 1 and sys.argv[1] == "--ticker":
        if len(sys.argv) < 3 or not sys.argv[2].strip():
            print("Usage: python main.py --ticker <TICKER>")
            sys.exit(1)
        sys.exit(main(sys.argv[2].strip().upper()))
    if len(sys.argv) > 1 and sys.argv[1] == "--price-history":
        if len(sys.argv) < 3 or not sys.argv[2].strip():
            print("Usage: python main.py --price-history <TICKER1,TICKER2,...>")
            sys.exit(1)
        tickers = [t.strip().upper() for t in sys.argv[2].split(",") if t.strip()]
        sys.exit(main_price_history(tickers))
    sys.exit(main())

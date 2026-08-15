"""Orquestrador da coleta de dados (Fase 2).

Lê config.yaml, chama cada fonte configurada e grava os resultados direto no
SQLite compartilhado com o app Tauri. Chamado pelo comando Rust
`run_stock_collector` (src-tauri/src/commands/collector.rs) como subprocess,
mas também roda direto (`python3 main.py`) pra depurar sem precisar do app.
"""

import json
import sqlite3
import sys
from datetime import datetime, timezone
from pathlib import Path

import yaml
from dotenv import load_dotenv

from sources import (
    acoes_bolsai,
    acoes_yahoo,
    bcb_sgs,
    cripto_coingecko,
    cripto_defillama,
    cripto_feargreed,
    cripto_ultrasound,
    cvm_dfp,
    cvm_fii,
    metais_yahoo,
    sec_edgar,
)

BASE_DIR = Path(__file__).parent
DB_PATH = BASE_DIR / "anchor.db"
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


def collect_benchmark_returns() -> dict:
    """Fase 13.5 — série fixa dos 4 benchmarks com fonte gratuita (CDI/IPCA
    via BCB SGS, IBOV/IVVB11 via Yahoo — IFIX/SMLL/IDIV ficam de fora, sem
    fonte histórica gratuita achada, ver PHASE.md). CDI/IPCA usam INSERT OR
    REPLACE (não IGNORE, diferente do resto do coletor): a BCB pode revisar
    um valor recém-publicado do mês corrente, e rodar o backfill de novo
    deve refletir o número oficial mais atual — os outros coletores lidam
    com histórico fechado (pregão já aconteceu), este não.
    """
    cdi = bcb_sgs.fetch_monthly_series(4391)
    ipca = bcb_sgs.fetch_monthly_series(433)

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    now = datetime.now(timezone.utc).isoformat()
    changes_before = conn.total_changes
    conn.executemany(
        "INSERT OR REPLACE INTO macro_index_monthly "
        "(index_code, year_month, value_pct, source, fetched_at) VALUES (?, ?, ?, ?, ?)",
        [("cdi", item["year_month"], item["value_pct"], "bcb_sgs", now) for item in cdi]
        + [("ipca", item["year_month"], item["value_pct"], "bcb_sgs", now) for item in ipca],
    )
    macro_new = conn.total_changes - changes_before
    conn.commit()
    conn.close()

    # IBOV (^BVSP, sem sufixo — é índice, não papel B3) e IVVB11 (ETF B3
    # comum, sufixo .SA padrão) reaproveitam 100% o path de
    # collect_us_price_history/collect_price_history — mesma tabela
    # genérica stock_price_history, zero coletor novo.
    ibov = collect_us_price_history(["^BVSP"])
    ivvb11 = collect_price_history(["IVVB11"])

    print(f"CDI/IPCA: {len(cdi)}/{len(ipca)} mês(es), {macro_new} novo/atualizado")
    print(f"IBOV: {len(ibov)} pregão(ões); IVVB11: {len(ivvb11)} pregão(ões)")
    return {"cdi": len(cdi), "ipca": len(ipca), "ibov": len(ibov), "ivvb11": len(ivvb11)}


def main_benchmark_returns() -> int:
    load_dotenv(BASE_DIR / ".env")
    collect_benchmark_returns()
    return 0


def collect_fii_cvm_data(cnpjs: list[str]) -> tuple[list[dict], list[dict]]:
    """Indicadores mensais + imóveis (vacância/inadimplência) direto da CVM,
    pra uma lista de CNPJs já resolvidos e salvos em `assets.cnpj` (ver
    `resolve_fii_cnpj` — a bolsai não é chamada aqui, só a CVM). Mesmo
    padrão `INSERT OR IGNORE` num índice único de `collect_price_history` —
    rodar de novo não duplica período já salvo, só acrescenta o mês/trimestre
    novo desde a última coleta.
    """
    if not cnpjs:
        return [], []

    monthly = cvm_fii.fetch_monthly_indicators(cnpjs)
    properties = cvm_fii.fetch_property_data(cnpjs)

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    now = datetime.now(timezone.utc).isoformat()
    changes_before = conn.total_changes
    conn.executemany(
        "INSERT OR IGNORE INTO fii_cvm_monthly "
        "(cnpj, reference_date, patrimonio_liquido, valor_patrimonial_cota, "
        "numero_cotistas, dividend_yield_mes, rentabilidade_efetiva_mes, source, fetched_at) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        [
            (
                item["cnpj"],
                item["reference_date"],
                item["patrimonio_liquido"],
                item["valor_patrimonial_cota"],
                item["numero_cotistas"],
                item["dividend_yield_mes"],
                item["rentabilidade_efetiva_mes"],
                "cvm_fii",
                now,
            )
            for item in monthly
        ],
    )
    conn.executemany(
        "INSERT OR IGNORE INTO fii_cvm_properties "
        "(cnpj, reference_date, nome_imovel, endereco, area_m2, percentual_vacancia, "
        "percentual_inadimplencia, percentual_receitas_fii, percentual_locado, "
        "source, fetched_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        [
            (
                item["cnpj"],
                item["reference_date"],
                item["nome_imovel"],
                item["endereco"],
                item["area_m2"],
                item["percentual_vacancia"],
                item["percentual_inadimplencia"],
                item["percentual_receitas_fii"],
                item["percentual_locado"],
                "cvm_fii",
                now,
            )
            for item in properties
        ],
    )
    new_count = conn.total_changes - changes_before
    conn.commit()
    conn.close()

    print(
        f"Fetched {len(monthly)} monthly indicator(s) and {len(properties)} propert(y/ies) "
        f"from CVM, {new_count} new row(s) (rest already saved)"
    )
    return monthly, properties


def main_fii_resolve_cnpj(ticker: str) -> int:
    """Resolve e imprime. `commands/fii.rs::resolve_fii_cnpj` lê a última
    linha de stdout como o único JSON impresso (nenhum outro print acontece
    neste modo).

    Consulta `fii_cnpj_cache` (por `ticker`) antes de qualquer coisa —
    necessário desde que a tela de Pesquisa (Sessão 43) passou a chamar isto
    pra qualquer busca de FII, não só no cadastro de Ativo: sem cache, cada
    pesquisa repetida do mesmo ticker chamaria a bolsai de novo, quebrando a
    promessa feita ao dono do projeto na Sessão 42 ("depois de resolvido, a
    bolsai nunca mais é chamada"). Só grava no cache quando a resolução
    realmente encontra um match confiável — um `None` (não achou) não é
    cacheado, pra não travar permanentemente algo que pode ter sido só uma
    falha temporária da bolsai.
    """
    load_dotenv(BASE_DIR / ".env")

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    cached = conn.execute(
        "SELECT cnpj, fund_name FROM fii_cnpj_cache WHERE ticker = ?", (ticker,)
    ).fetchone()
    if cached is not None:
        conn.close()
        print(json.dumps({"cnpj": cached[0], "fund_name": cached[1]}))
        return 0

    result = cvm_fii.resolve_cnpj(ticker)
    if result is not None:
        now = datetime.now(timezone.utc).isoformat()
        conn.execute(
            "INSERT OR IGNORE INTO fii_cnpj_cache (ticker, cnpj, fund_name, resolved_at) "
            "VALUES (?, ?, ?, ?)",
            (ticker, result["cnpj"], result["fund_name"], now),
        )
        conn.commit()
    conn.close()

    print(json.dumps(result))
    return 0


def main_fii_cvm_data(cnpjs: list[str]) -> int:
    load_dotenv(BASE_DIR / ".env")
    collect_fii_cvm_data(cnpjs)
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


def collect_crypto_ticker(symbol: str) -> dict:
    """Quote + price history for one crypto ticker (Fase 10, item 8 — Crypto
    virou classe de ativo no Portfolio, Sessão 51).

    Unrelated to `main_crypto()`/`_record_crypto_indicator` above (those are
    the Ethereum-only cycle-top score, hardcoded `coin="ETH"`) — this is the
    generic "any coin" quote/price-history pair that Assets/Research use for
    portfolio valuation and TWR, same role `collect_stock_quotes`/
    `collect_price_history` play for B3 tickers, just backed by CoinGecko
    instead of Yahoo since crypto isn't listed on the B3 `.SA` endpoint.
    One `market_chart` call gives both the latest price (written to
    `stock_quotes`) and the full daily series (written to
    `stock_price_history`, `INSERT OR IGNORE` on the same unique
    `(ticker, price_date)` index Yahoo-sourced rows use — safe to call this
    again later, only new days get inserted).
    """
    coin = cripto_coingecko.resolve_coin_id(symbol)
    if coin is None:
        raise RuntimeError(f"No CoinGecko coin found for symbol '{symbol}'")

    points = cripto_coingecko.fetch_market_chart(coin["id"])
    if not points:
        raise RuntimeError(f"CoinGecko returned no price history for '{symbol}' ({coin['id']})")

    latest = points[-1]
    now = datetime.now(timezone.utc).isoformat()

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute(
        "INSERT INTO stock_quotes (ticker, price, name, exchange, currency, source, fetched_at) "
        "VALUES (?, ?, ?, ?, ?, ?, ?)",
        (symbol, latest["price"], coin["name"], "CoinGecko", "USD", "coingecko", now),
    )
    changes_before = conn.total_changes
    conn.executemany(
        "INSERT OR IGNORE INTO stock_price_history "
        "(ticker, price_date, close_price, source, fetched_at) VALUES (?, ?, ?, ?, ?)",
        [(symbol, p["price_date"], p["price"], "coingecko", now) for p in points],
    )
    new_history_count = conn.total_changes - changes_before

    # Fear & Greed é global de mercado (não por coin) — atualizado junto de
    # qualquer refresh de ticker cripto, pedido explícito do dono do
    # projeto pra aparecer em toda tela de cripto. `INSERT OR IGNORE` no
    # índice único de `reading_date`: só grava a primeira vez no dia, buscar
    # nesse mesmo dia de novo (outro ticker, outro refresh) é no-op esperado.
    fear_greed = cripto_feargreed.fetch_latest()
    conn.execute(
        "INSERT OR IGNORE INTO crypto_fear_greed "
        "(value, classification, reading_date, source, fetched_at) VALUES (?, ?, ?, ?, ?)",
        (fear_greed["value"], fear_greed["classification"], fear_greed["reading_date"], "alternative.me", now),
    )

    conn.commit()
    conn.close()

    return {
        "ticker": symbol,
        "name": coin["name"],
        "price": latest["price"],
        "history_points": len(points),
        "new_history_points": new_history_count,
        "fear_greed": fear_greed,
    }


def main_crypto_ticker(symbol: str) -> int:
    load_dotenv(BASE_DIR / ".env")
    try:
        result = collect_crypto_ticker(symbol)
    except RuntimeError as err:
        print(str(err))
        return 1

    print(f"{result['ticker']} ({result['name']}): US$ {result['price']:.2f}")
    print(
        f"Updated price history: {result['history_points']} point(s), "
        f"{result['new_history_points']} new"
    )
    fg = result["fear_greed"]
    print(f"Fear & Greed: {fg['value']} ({fg['classification']}) — {fg['reading_date']}")
    return 0


def collect_metal_ticker(ticker: str) -> dict:
    """Quote + price history pra um metal (Fase 10, item 8, Sessão 55).

    Mesmo papel que `collect_crypto_ticker` cumpre pra cripto: uma chamada
    só devolve preço atual + série histórica, gravados nas mesmas tabelas
    genéricas `stock_quotes`/`stock_price_history` que todo o resto do app
    já lê (TWR, Research, Ativos) — preço já convertido pra USD/grama por
    `metais_yahoo.fetch_quote_and_history` (COMEX cota em onça troy).
    """
    result = metais_yahoo.fetch_quote_and_history(ticker)
    now = datetime.now(timezone.utc).isoformat()

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute(
        "INSERT INTO stock_quotes (ticker, price, name, exchange, currency, source, fetched_at) "
        "VALUES (?, ?, ?, ?, ?, ?, ?)",
        (ticker, result["price"], result["name"], "COMEX", "USD", "yahoo_finance", now),
    )
    changes_before = conn.total_changes
    conn.executemany(
        "INSERT OR IGNORE INTO stock_price_history "
        "(ticker, price_date, close_price, source, fetched_at) VALUES (?, ?, ?, ?, ?)",
        [(ticker, p["price_date"], p["close_price"], "yahoo_finance", now) for p in result["history"]],
    )
    new_history_count = conn.total_changes - changes_before
    conn.commit()
    conn.close()

    return {
        "ticker": ticker,
        "name": result["name"],
        "price": result["price"],
        "history_points": len(result["history"]),
        "new_history_points": new_history_count,
    }


def main_metal_ticker(ticker: str) -> int:
    load_dotenv(BASE_DIR / ".env")
    try:
        result = collect_metal_ticker(ticker)
    except RuntimeError as err:
        print(str(err))
        return 1

    print(f"{result['ticker']} ({result['name']}): US$ {result['price']:.2f}/oz")
    print(
        f"Updated price history: {result['history_points']} point(s), "
        f"{result['new_history_points']} new"
    )
    return 0


def collect_us_stock_quotes(tickers: list[str]) -> list[dict]:
    if not tickers:
        return []

    quotes = acoes_yahoo.fetch_quotes(tickers, suffix="")

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


def collect_us_stock_dividends_avg(tickers: list[str]) -> list[dict]:
    if not tickers:
        return []

    dividends = acoes_yahoo.fetch_dividends_avg(tickers, suffix="")

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


def collect_us_stock_technicals(tickers: list[str]) -> list[dict]:
    if not tickers:
        return []

    technicals = acoes_yahoo.fetch_technicals(tickers, suffix="")

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


def collect_us_stock_dividend_payments(tickers: list[str]) -> list[dict]:
    if not tickers:
        return []

    payments = acoes_yahoo.fetch_dividend_payments(tickers, suffix="")

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
    new_count = conn.total_changes - changes_before
    conn.commit()
    conn.close()

    print(
        f"Fetched {len(payments)} dividend payment(s) from source, "
        f"{new_count} new (rest already saved)"
    )
    return payments


def collect_us_price_history(tickers: list[str]) -> list[dict]:
    if not tickers:
        return []

    prices = acoes_yahoo.fetch_price_history(tickers, suffix="")

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


def collect_us_stock_fundamentals(tickers: list[str]) -> list[dict]:
    """Fase 10, item 8, Fatia 2 — LPA/VPA/ROE/Payout de ação americana via
    SEC EDGAR (`sec_edgar.py`). Mais simples que a versão BR
    (`collect_stock_fundamentals`): não precisa de uma etapa separada
    corrigindo o ROE, a SEC já dá o dado calculado direto de
    NetIncomeLoss/StockholdersEquity, sem a instabilidade trimestral-vs-TTM
    que o campo `roe` da bolsai tinha."""
    if not tickers:
        return []

    fundamentals = sec_edgar.fetch_fundamentals(tickers)

    ticker_ciks = {f["ticker"]: f["cik"] for f in fundamentals}
    payout_by_ticker = {
        item["ticker"]: item["payout"] for item in sec_edgar.fetch_payout(ticker_ciks)
    }
    for item in fundamentals:
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
                "sec_edgar",
                now,
            )
            for item in fundamentals
        ],
    )
    conn.commit()
    conn.close()

    return fundamentals


def collect_us_stock_dcf_fundamentals(fundamentals: list[dict]) -> list[dict]:
    """Fase 10, item 8, Fatia 2 — espelho de `collect_stock_dcf_fundamentals`
    (mesmas colunas/tabela `stock_dcf_fundamentals`), completando com os
    campos vindos da SEC EDGAR. `shares_outstanding` vem de `fundamentals`
    (já buscado por `collect_us_stock_fundamentals`), não da SEC diretamente
    — mesmo desenho do par bolsai/CVM."""
    ticker_ciks = {f["ticker"]: f["cik"] for f in fundamentals}
    if not ticker_ciks:
        return []

    shares_outstanding_millions = {
        f["ticker"]: f["shares_outstanding"] / 1_000_000 for f in fundamentals
    }

    records = sec_edgar.fetch_dcf_fundamentals(ticker_ciks)
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
                "sec_edgar",
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


def collect_reit_fundamentals(tickers: list[str]) -> list[dict]:
    """Fase 10, item 8 — indicadores imobiliários de REIT via SEC EDGAR
    (`sec_edgar.fetch_reit_fundamentals`). Grava em `reit_fundamentals`,
    tabela própria (não `stock_fundamentals`/`stock_dcf_fundamentals` — REIT
    não usa os 8 modelos de valuation, ver Sessão de implementação). Mesmo
    padrão time-series de `collect_us_stock_fundamentals`: `INSERT` simples,
    nunca sobrescreve, leitura sempre pega a linha mais recente."""
    if not tickers:
        return []

    ciks = sec_edgar.resolve_ciks(tickers)
    fundamentals = sec_edgar.fetch_reit_fundamentals(ciks)

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA journal_mode=WAL")
    now = datetime.now(timezone.utc).isoformat()
    conn.executemany(
        "INSERT INTO reit_fundamentals (ticker, reference_year, revenue, "
        "real_estate_property_net, real_estate_property_at_cost, "
        "stockholders_equity, net_income, eps_diluted, source, fetched_at) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        [
            (
                item["ticker"],
                item["reference_year"],
                item["revenue"],
                item["real_estate_property_net"],
                item["real_estate_property_at_cost"],
                item["stockholders_equity"],
                item["net_income"],
                item["eps_diluted"],
                "sec_edgar",
                now,
            )
            for item in fundamentals
        ],
    )
    conn.commit()
    conn.close()

    return fundamentals


def main_etf_us(ticker: str) -> int:
    """ETF americano (Fase 10, item 8) — cotação/técnicos/dividendos/
    histórico de preço via Yahoo sem sufixo (idêntico ao bloco inicial de
    `main_us_stock`/`main_reit`). Sem fundamentos/DCF nem indicadores
    dedicados — ETF não tem demonstração financeira própria na SEC, mesmo
    motivo pelo qual `etf_br` também não busca fundamentos."""
    load_dotenv(BASE_DIR / ".env")
    tickers = [ticker]

    quotes = collect_us_stock_quotes(tickers)
    for quote in quotes:
        print(f"{quote['ticker']}: US$ {quote['price']}")
    print(f"Updated {len(quotes)} quote(s)")

    dividends = collect_us_stock_dividends_avg(tickers)
    for item in dividends:
        print(f"{item['ticker']}: avg dividend/share (5y) US$ {item['avg_dividend_5y']:.4f}")
    print(f"Updated {len(dividends)} dividend average record(s)")

    technicals = collect_us_stock_technicals(tickers)
    for item in technicals:
        sma_200 = item["sma_200"]
        cagr_10y = item["cagr_10y"]
        print(
            f"{item['ticker']}: SMA200 "
            f"{'n/a' if sma_200 is None else f'US$ {sma_200:.2f}'} / "
            f"CAGR 10y {'n/a' if cagr_10y is None else f'{cagr_10y:.1f}%'}"
        )
    print(f"Updated {len(technicals)} technicals record(s)")

    collect_us_stock_dividend_payments(tickers)
    collect_us_price_history(tickers)

    return 0


def main_reit(ticker: str) -> int:
    """REIT (Fase 10, item 8) — cotação/técnicos/dividendos/histórico de
    preço via Yahoo sem sufixo (idêntico a `main_us_stock`, REIT é só mais
    um ticker NYSE/NASDAQ) + indicadores imobiliários via SEC EDGAR. Sem os
    8 modelos de valuation (não encaixam bem em imobiliário) — por isso não
    chama `collect_us_stock_fundamentals`/`collect_us_stock_dcf_fundamentals`,
    só a função dedicada acima."""
    load_dotenv(BASE_DIR / ".env")
    tickers = [ticker]

    quotes = collect_us_stock_quotes(tickers)
    for quote in quotes:
        print(f"{quote['ticker']}: US$ {quote['price']}")
    print(f"Updated {len(quotes)} quote(s)")

    dividends = collect_us_stock_dividends_avg(tickers)
    for item in dividends:
        print(f"{item['ticker']}: avg dividend/share (5y) US$ {item['avg_dividend_5y']:.4f}")
    print(f"Updated {len(dividends)} dividend average record(s)")

    technicals = collect_us_stock_technicals(tickers)
    for item in technicals:
        sma_200 = item["sma_200"]
        cagr_10y = item["cagr_10y"]
        print(
            f"{item['ticker']}: SMA200 "
            f"{'n/a' if sma_200 is None else f'US$ {sma_200:.2f}'} / "
            f"CAGR 10y {'n/a' if cagr_10y is None else f'{cagr_10y:.1f}%'}"
        )
    print(f"Updated {len(technicals)} technicals record(s)")

    collect_us_stock_dividend_payments(tickers)
    collect_us_price_history(tickers)

    # Mesmo contrato "pula barato, não derruba o resto" de `main_us_stock` —
    # o único RuntimeError aqui é `SEC_EDGAR_CONTACT_EMAIL` ausente.
    try:
        reit_fundamentals = collect_reit_fundamentals(tickers)
        for item in reit_fundamentals:
            net_income = item["net_income"]
            print(
                f"{item['ticker']}: Revenue {item['revenue']:.1f} / "
                f"Equity {item['stockholders_equity']:.1f} / EPS {item['eps_diluted']} / "
                f"Net income {'n/a' if net_income is None else f'{net_income:.1f}'} "
                f"(US$ millions, FY{item['reference_year']})"
            )
        print(f"Updated {len(reit_fundamentals)} REIT fundamentals record(s)")
    except RuntimeError as err:
        print(f"Skipping SEC EDGAR collection: {err}")

    return 0


def main_us_stock(ticker: str) -> int:
    """Ação americana (Fase 10, item 8). Fatia 1 deu cotação/técnicos/
    dividendos/histórico de preço via Yahoo, sem sufixo `.SA`. Fatia 2
    (esta) acrescenta fundamentos + DCF via SEC EDGAR (`sec_edgar.py`),
    mesmo papel que `acoes_bolsai`/`cvm_dfp` cumprem pra ação BR — só que
    aqui uma fonte só cobre os dois (LPA/VPA/ROE/Payout e os 9 campos do
    DCF), ao contrário do par bolsai+CVM.
    """
    load_dotenv(BASE_DIR / ".env")
    tickers = [ticker]

    quotes = collect_us_stock_quotes(tickers)
    for quote in quotes:
        print(f"{quote['ticker']}: US$ {quote['price']}")
    print(f"Updated {len(quotes)} quote(s)")

    dividends = collect_us_stock_dividends_avg(tickers)
    for item in dividends:
        print(f"{item['ticker']}: avg dividend/share (5y) US$ {item['avg_dividend_5y']:.4f}")
    print(f"Updated {len(dividends)} dividend average record(s)")

    technicals = collect_us_stock_technicals(tickers)
    for item in technicals:
        sma_200 = item["sma_200"]
        cagr_10y = item["cagr_10y"]
        print(
            f"{item['ticker']}: SMA200 "
            f"{'n/a' if sma_200 is None else f'US$ {sma_200:.2f}'} / "
            f"CAGR 10y {'n/a' if cagr_10y is None else f'{cagr_10y:.1f}%'}"
        )
    print(f"Updated {len(technicals)} technicals record(s)")

    collect_us_stock_dividend_payments(tickers)
    collect_us_price_history(tickers)

    # Fundamentos/DCF via SEC EDGAR (Fatia 2) — o único caso de RuntimeError
    # aqui é `SEC_EDGAR_CONTACT_EMAIL` ausente (não existe "chave de API" pra
    # SEC), mesmo contrato de "pula barato, não derruba o resto" que o
    # bloco bolsai/CVM já tem em `main()`.
    try:
        us_fundamentals = collect_us_stock_fundamentals(tickers)
        for item in us_fundamentals:
            payout = item["payout"]
            print(
                f"{item['ticker']}: LPA US$ {item['lpa']} / VPA US$ {item['vpa']} / "
                f"ROE {item['roe']}% / Payout {'n/a' if payout is None else f'{payout}%'}"
            )
        print(f"Updated {len(us_fundamentals)} fundamentals record(s)")
    except RuntimeError as err:
        print(f"Skipping SEC EDGAR collection: {err}")
        us_fundamentals = []

    if not us_fundamentals:
        return 0

    us_dcf_fundamentals = collect_us_stock_dcf_fundamentals(us_fundamentals)
    for item in us_dcf_fundamentals:
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
            f"Inventory {item['inventory']:.1f} (US$ millions)"
        )
    print(f"Updated {len(us_dcf_fundamentals)} DCF fundamentals record(s)")

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

    # Coletores só-Yahoo, genéricos por ticker (Ação BR ou FII — mesmo
    # endpoint `.SA`) — rodam sempre, **antes** do bloco da bolsai abaixo.
    # Corrigido na Sessão 44: antes ficavam depois do `if not fundamentals:
    # return 0`, então um FII (que nunca tem fundamento de ação na bolsai —
    # 404 sempre, ela só cobre `/fiis/{ticker}` à parte, usado por
    # `cvm_fii.py`) nunca coletava proventos/técnicos — achado testando a
    # busca de FII na tela de Pesquisa no app real, "histórico de proventos"
    # sempre vazio. Yahoo Finance's chart API already skips failing/
    # dividend-less tickers internally (see acoes_yahoo.py) — no try/except
    # needed here, unlike the bolsai calls below.
    dividends = collect_stock_dividends_avg(tickers)
    for item in dividends:
        print(f"{item['ticker']}: avg dividend/share (5y) R$ {item['avg_dividend_5y']:.4f}")
    print(f"Updated {len(dividends)} dividend average record(s)")

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

    # Preço histórico (Fase 10, item 8, Sessão 52) — antes só populado pelo
    # backfill manual do Portfolio (`--price-history`, botão "Update price
    # history"). Buscar por ticker na Pesquisa passa a alimentar
    # `stock_price_history` de graça também, mesmo padrão que cripto já
    # ganhou na Sessão 51 (`collect_crypto_ticker` faz isso numa chamada só)
    # — sem isso, o gráfico de preço pedido pro dono do projeto ficaria
    # vazio pra qualquer ticker nunca "backfillado" manualmente antes.
    collect_price_history(tickers)

    # bolsai (fundamentos de ação + DCF) — só faz sentido pra Ação BR (FII
    # sempre devolve vazio aqui, sem quebrar nada, ver nota acima) e requer
    # chave cadastrada; se faltar, pula só esta parte em vez de falhar tudo.
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

    return 0


if __name__ == "__main__":
    # Fase 11.3 — sidecar empacotado não tem `BASE_DIR`-relativo confiável
    # (PyInstaller extrai pra um diretório temporário), então o Rust passa o
    # caminho certo (`app_data_dir()`) explicitamente. Sem esse argumento
    # (uso via `.venv` em dev, como sempre), `DB_PATH` continua relativo ao
    # próprio script — comportamento intocado.
    if len(sys.argv) > 2 and sys.argv[1] == "--db-path":
        DB_PATH = Path(sys.argv[2])
        sys.argv = [sys.argv[0]] + sys.argv[3:]

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
    if len(sys.argv) > 1 and sys.argv[1] == "--fii-resolve-cnpj":
        if len(sys.argv) < 3 or not sys.argv[2].strip():
            print("Usage: python main.py --fii-resolve-cnpj <TICKER>")
            sys.exit(1)
        sys.exit(main_fii_resolve_cnpj(sys.argv[2].strip().upper()))
    if len(sys.argv) > 1 and sys.argv[1] == "--crypto-ticker":
        if len(sys.argv) < 3 or not sys.argv[2].strip():
            print("Usage: python main.py --crypto-ticker <SYMBOL>")
            sys.exit(1)
        sys.exit(main_crypto_ticker(sys.argv[2].strip().upper()))
    if len(sys.argv) > 1 and sys.argv[1] == "--metal-ticker":
        if len(sys.argv) < 3 or not sys.argv[2].strip():
            print("Usage: python main.py --metal-ticker <TICKER>")
            sys.exit(1)
        sys.exit(main_metal_ticker(sys.argv[2].strip().upper()))
    if len(sys.argv) > 1 and sys.argv[1] == "--us-ticker":
        if len(sys.argv) < 3 or not sys.argv[2].strip():
            print("Usage: python main.py --us-ticker <TICKER>")
            sys.exit(1)
        sys.exit(main_us_stock(sys.argv[2].strip().upper()))
    if len(sys.argv) > 1 and sys.argv[1] == "--reit-ticker":
        if len(sys.argv) < 3 or not sys.argv[2].strip():
            print("Usage: python main.py --reit-ticker <TICKER>")
            sys.exit(1)
        sys.exit(main_reit(sys.argv[2].strip().upper()))
    if len(sys.argv) > 1 and sys.argv[1] == "--etf-us-ticker":
        if len(sys.argv) < 3 or not sys.argv[2].strip():
            print("Usage: python main.py --etf-us-ticker <TICKER>")
            sys.exit(1)
        sys.exit(main_etf_us(sys.argv[2].strip().upper()))
    if len(sys.argv) > 1 and sys.argv[1] == "--benchmark-returns":
        sys.exit(main_benchmark_returns())
    if len(sys.argv) > 1 and sys.argv[1] == "--fii-cvm-data":
        if len(sys.argv) < 3 or not sys.argv[2].strip():
            print("Usage: python main.py --fii-cvm-data <CNPJ1,CNPJ2,...>")
            sys.exit(1)
        cnpjs = [c.strip() for c in sys.argv[2].split(",") if c.strip()]
        sys.exit(main_fii_cvm_data(cnpjs))
    sys.exit(main())

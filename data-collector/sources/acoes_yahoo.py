"""Cliente da API não-oficial do Yahoo Finance (cotação atual + histórico de
dividendos).

Endpoint confirmado direto contra a API real (2026-07-10, recheckado
2026-07-12 pra cotação) — público, sem chave, sem cadastro, funciona pra
qualquer ticker da B3 (diferente da brapi, cuja cotação de graça só cobre 4
tickers de teste sem token — ver histórico em `PROJECT_STATE.md`, Sessão 4/6):
GET /v8/finance/chart/{ticker}.SA devolve um bloco `chart.result[0].meta`
com `regularMarketPrice` (usado por `fetch_quotes`) e, quando
`events=div` é passado, também `chart.result[0].events.dividends` — um
dict de {epoch_do_anuncio: {"amount": <R$/ação>, "date":
<epoch_do_pagamento>}} (usado por `fetch_dividends_avg`).

**Achado**: essa rota não é documentada oficialmente pelo Yahoo (é a mesma
usada por baixo dos panos pela lib `yfinance`) — sem contrato formal de
estabilidade, mas amplamente usada e sem exigir cadastro nenhum.

`fetch_dividends_avg` calcula o "dividendo médio por ação, últimos 5 anos
completos" que o Bazin pede (soma por ano, descarta o ano corrente parcial,
média dos últimos 5 anos completos). `fetch_quotes` substituiu totalmente
`acoes_brapi.py` (removido) — a brapi exigia token pago pra qualquer ticker
fora da demo, o que quebrava a busca ad hoc por ticker (Fase "fetch por
ticker" nos formulários de valuation).

`fetch_technicals` (tela "Stock Lookup") reusa o mesmo endpoint com
`range=10y&interval=1d` — confirmado direto contra a API real (2026-07-15):
`chart.result[0].timestamp` (epoch/dia) e
`chart.result[0].indicators.quote[0].close` (fechamento diário, `None` em
dias sem pregão) cobrem, numa única chamada, tanto a SMA200 (só precisa dos
últimos ~200 candles) quanto o CAGR de 10 anos (precisa do candle mais
antigo do range).
"""

from collections import defaultdict
from datetime import datetime, timezone

import requests

YAHOO_CHART_URL = "https://query1.finance.yahoo.com/v8/finance/chart"

# Cobre o suficiente pra sempre sobrar 5 anos completos depois de descartar
# o ano corrente (parcial), e também o range usado pelo `fetch_technicals`
# pro CAGR de 10 anos.
HISTORY_RANGE = "10y"
DIVIDENDS_YEARS_AVERAGED = 5

SMA_WINDOWS = (50, 100, 200)
CAGR_YEARS = (5, 10)
SECONDS_PER_YEAR = 365.25 * 86400
# Se o candle mais próximo da data-alvo (hoje - N anos) estiver mais longe
# que isso, o ticker não tem histórico suficiente pra esse CAGR (ex: IPO
# recente) — melhor `None` do que um CAGR calculado sobre um período errado.
CAGR_ANCHOR_TOLERANCE_DAYS = 30


def fetch_quotes(tickers: list[str]) -> list[dict]:
    """Busca a cotação atual de uma lista de tickers.

    Retorna uma lista de {"ticker": str, "price": float, "name": str | None,
    "exchange": str | None, "currency": str | None} — os 3 últimos vêm do
    mesmo bloco `meta` já lido pro preço (sem chamada extra), usados pro
    pré-preenchimento do cadastro de Ativo (Portfolio, Ação BR). Só o preço
    é obrigatório — os outros usam `.get()` pra não descartar o ticker se o
    Yahoo omitir algum deles. Tickers que falharem na API são ignorados —
    não derrubam o resto, mesmo padrão de `fetch_dividends_avg`.
    """
    results = []

    for ticker in tickers:
        try:
            response = requests.get(
                f"{YAHOO_CHART_URL}/{ticker}.SA",
                params={"range": "5d", "interval": "1d"},
                headers={"User-Agent": "Mozilla/5.0"},
                timeout=15,
            )
            response.raise_for_status()
            meta = response.json()["chart"]["result"][0]["meta"]
            price = meta["regularMarketPrice"]
        except (requests.RequestException, KeyError, TypeError, IndexError):
            continue

        results.append(
            {
                "ticker": ticker,
                "price": price,
                "name": meta.get("longName") or meta.get("shortName"),
                "exchange": meta.get("fullExchangeName") or meta.get("exchangeName"),
                "currency": meta.get("currency"),
            }
        )

    return results


def fetch_dividends_avg(tickers: list[str]) -> list[dict]:
    """Busca o dividendo médio por ação dos últimos 5 anos completos.

    Retorna uma lista de {"ticker": str, "avg_dividend_5y": float}. Tickers
    sem dividendo nenhum registrado (ex: growth stock, IPO recente sem ano
    completo) ou que falharem na API são ignorados — não derrubam o resto.
    """
    current_year = datetime.now(timezone.utc).year
    results = []

    for ticker in tickers:
        try:
            response = requests.get(
                f"{YAHOO_CHART_URL}/{ticker}.SA",
                params={"range": HISTORY_RANGE, "interval": "3mo", "events": "div"},
                headers={"User-Agent": "Mozilla/5.0"},
                timeout=15,
            )
            response.raise_for_status()
            chart_result = response.json()["chart"]["result"][0]
        except (requests.RequestException, KeyError, TypeError, IndexError):
            continue

        dividends = chart_result.get("events", {}).get("dividends", {})
        if not dividends:
            continue

        yearly_totals: dict[int, float] = defaultdict(float)
        for entry in dividends.values():
            year = datetime.fromtimestamp(entry["date"], tz=timezone.utc).year
            if year == current_year:
                continue
            yearly_totals[year] += entry["amount"]

        complete_years = sorted(yearly_totals, reverse=True)[:DIVIDENDS_YEARS_AVERAGED]
        if not complete_years:
            continue

        avg = sum(yearly_totals[year] for year in complete_years) / len(complete_years)
        results.append({"ticker": ticker, "avg_dividend_5y": avg})

    return results


def _closest_close(
    timestamps: list[int],
    closes: list[float | None],
    target_ts: float,
    tolerance_days: float = CAGR_ANCHOR_TOLERANCE_DAYS,
) -> float | None:
    """Fecho mais próximo de `target_ts`, ignorando candles sem pregão
    (`close=None`). Retorna `None` se o mais próximo estiver fora de
    `tolerance_days` (histórico insuficiente, ou nenhum candle perto o
    bastante da data pedida).
    """
    best_ts = None
    best_close = None
    for ts, close in zip(timestamps, closes):
        if close is None:
            continue
        if best_ts is None or abs(ts - target_ts) < abs(best_ts - target_ts):
            best_ts, best_close = ts, close

    if best_ts is None:
        return None
    if abs(best_ts - target_ts) > tolerance_days * 86400:
        return None
    return best_close


def fetch_technicals(tickers: list[str]) -> list[dict]:
    """Busca médias móveis (50/100/200 dias) e CAGR (5/10 anos) de preço.

    Retorna uma lista de dicts com `ticker`, `sma_50`, `sma_100`, `sma_200`,
    `cagr_5y`, `cagr_10y` (`float` em % ou `None` quando não há histórico
    suficiente pra aquele cálculo — ex: IPO recente sem 200 pregões ou sem
    5/10 anos completos). Tickers que falharem na API são ignorados, mesmo
    padrão de `fetch_quotes`/`fetch_dividends_avg`.
    """
    results = []

    for ticker in tickers:
        try:
            response = requests.get(
                f"{YAHOO_CHART_URL}/{ticker}.SA",
                params={"range": HISTORY_RANGE, "interval": "1d"},
                headers={"User-Agent": "Mozilla/5.0"},
                timeout=15,
            )
            response.raise_for_status()
            chart_result = response.json()["chart"]["result"][0]
            timestamps = chart_result["timestamp"]
            closes = chart_result["indicators"]["quote"][0]["close"]
        except (requests.RequestException, KeyError, TypeError, IndexError):
            continue

        valid_closes = [c for c in closes if c is not None]
        if not valid_closes:
            continue

        latest_close = valid_closes[-1]
        latest_ts = timestamps[-1]

        record = {"ticker": ticker}

        for window in SMA_WINDOWS:
            if len(valid_closes) >= window:
                record[f"sma_{window}"] = sum(valid_closes[-window:]) / window
            else:
                record[f"sma_{window}"] = None

        for years in CAGR_YEARS:
            anchor_close = _closest_close(
                timestamps, closes, latest_ts - years * SECONDS_PER_YEAR
            )
            if anchor_close is None or anchor_close <= 0:
                record[f"cagr_{years}y"] = None
            else:
                record[f"cagr_{years}y"] = (
                    (latest_close / anchor_close) ** (1 / years) - 1
                ) * 100

        results.append(record)

    return results


# Diferença de dia útil real entre o pagamento (calendário) e o candle mais
# próximo raramente passa de 1-2 dias (fim de semana/feriado) — confirmado
# contra BBAS3 real (2026-07-15), todo pagamento bateu num candle do mesmo
# dia. Bem mais apertado que `CAGR_ANCHOR_TOLERANCE_DAYS` (30d, usado pra
# achar o candle "de ~N anos atrás", não pra casar uma data exata).
DIVIDEND_PRICE_TOLERANCE_DAYS = 5


def fetch_dividend_payments(tickers: list[str]) -> list[dict]:
    """Busca o histórico completo de pagamentos de dividendo (10 anos) com o
    preço de fechamento do dia do pagamento, pro gráfico da tela Stock
    Lookup (Fase 9.3).

    Retorna uma lista de dicts com `ticker`, `payment_date` (`YYYY-MM-DD`),
    `amount` (R$/ação), `price_at_payment` (fechamento do dia, `None` se não
    achar um candle perto o bastante — ex.: pagamento no limite dos 10 anos
    de histórico) e `yield_pct` (`amount / price_at_payment * 100`, `None`
    junto quando o preço também é `None`). Um ticker sem nenhum dividendo no
    período é ignorado, mesmo padrão de `fetch_dividends_avg`.
    """
    results = []

    for ticker in tickers:
        try:
            response = requests.get(
                f"{YAHOO_CHART_URL}/{ticker}.SA",
                params={"range": HISTORY_RANGE, "interval": "1d", "events": "div"},
                headers={"User-Agent": "Mozilla/5.0"},
                timeout=15,
            )
            response.raise_for_status()
            chart_result = response.json()["chart"]["result"][0]
            timestamps = chart_result["timestamp"]
            closes = chart_result["indicators"]["quote"][0]["close"]
        except (requests.RequestException, KeyError, TypeError, IndexError):
            continue

        dividends = chart_result.get("events", {}).get("dividends", {})
        if not dividends:
            continue

        for entry in dividends.values():
            payment_ts = entry["date"]
            amount = entry["amount"]
            price = _closest_close(
                timestamps, closes, payment_ts, DIVIDEND_PRICE_TOLERANCE_DAYS
            )
            payment_date = datetime.fromtimestamp(payment_ts, tz=timezone.utc).date()

            results.append(
                {
                    "ticker": ticker,
                    "payment_date": payment_date.isoformat(),
                    "amount": amount,
                    "price_at_payment": price,
                    "yield_pct": (amount / price * 100) if price else None,
                }
            )

    return results

"""Cliente da API do CoinGecko (dado de mercado: preço, market cap, volume).

Endpoint confirmado direto contra a API real (2026-07-16) — público, sem
chave: GET /api/v3/coins/{id}/market_chart devolve séries diárias de
market cap e volume de trading (mesmo shape, [[timestamp_ms, valor], ...]).

Usado aqui só pro indicador `nvt_ratio` do score cripto (Fase 3). Ressalva
importante, decidida com o usuário: o "volume" usado aqui é volume de
EXCHANGE (trading), não volume liquidado on-chain (a definição original de
Willy Woo pro NVT). Blockchair tem o dado on-chain de verdade, mas só o
valor de hoje, sem histórico grátis — não dá pra calcular a média móvel de
90 dias que a regra de sinal pede sem esperar ~90 dias de coleta acumulada.
Essa proxy é menos "pura", mas funciona com uma chamada só, desde o
primeiro dia.
"""

from datetime import datetime, timezone

import requests

COINGECKO_BASE_URL = "https://api.coingecko.com/api/v3"

NVT_MA_WINDOW_DAYS = 90
PRICE_HISTORY_DAYS = 365


def fetch_nvt_ratio_vs_ma90(coin_id: str = "ethereum") -> float:
    """Retorna o NVT de hoje dividido pela média móvel dos últimos 90 dias.

    <1.0 = NVT de hoje abaixo da média (rede "barata" perto do volume) — bom.
    >1.0 = NVT de hoje acima da média (rede "cara" perto do volume) — alerta.
    """
    response = requests.get(
        f"{COINGECKO_BASE_URL}/coins/{coin_id}/market_chart",
        params={"vs_currency": "usd", "days": NVT_MA_WINDOW_DAYS, "interval": "daily"},
        timeout=15,
    )
    response.raise_for_status()
    data = response.json()

    market_caps = [point[1] for point in data["market_caps"]]
    volumes = [point[1] for point in data["total_volumes"]]
    daily_nvt = [cap / vol for cap, vol in zip(market_caps, volumes)]

    # O ponto mais recente é "hoje" (dia ainda em andamento) — a média dos
    # 90 dias fechados anteriores é o que compõe a MA de referência.
    nvt_today = daily_nvt[-1]
    nvt_ma90 = sum(daily_nvt[:-1]) / len(daily_nvt[:-1])

    return nvt_today / nvt_ma90


def resolve_coin_id(symbol: str) -> dict | None:
    """Resolves a free-text ticker (e.g. "ETH", "BTC") to a CoinGecko coin id.

    Confirmed direct against the real API (2026-08-02): GET /search?query=X
    returns a `coins` list, each with `symbol`/`id`/`name`/`market_cap_rank`.
    Several unrelated coins can share the same symbol (e.g. searching "eth"
    also returns tickers like "ETHFI"), so this only accepts an *exact*
    (case-insensitive) symbol match — never fuzzy, same "zero or ambiguous
    match returns None" rule already used by `cvm_fii.py`'s CNPJ resolution.
    Among exact matches (still possible — many low-cap coins reuse a
    well-known symbol), the lowest `market_cap_rank` wins: for real tickers
    like "ETH"/"BTC" that's unambiguously the canonical coin.
    """
    response = requests.get(
        f"{COINGECKO_BASE_URL}/search",
        params={"query": symbol},
        timeout=15,
    )
    response.raise_for_status()
    coins = response.json().get("coins", [])

    exact_matches = [c for c in coins if c.get("symbol", "").lower() == symbol.lower()]
    if not exact_matches:
        return None

    best = min(
        exact_matches,
        key=lambda c: c.get("market_cap_rank") or float("inf"),
    )
    return {"id": best["id"], "name": best["name"]}


def fetch_market_chart(coin_id: str, days: int = PRICE_HISTORY_DAYS) -> list[dict]:
    """Daily close price series for `coin_id`, most recent point last.

    Confirmed direct against the real API (2026-08-02): GET
    /coins/{id}/market_chart?days=365&interval=daily returns `prices`, a
    list of `[epoch_ms, price]` pairs, one per day. The last point doubles
    as "current price" (same source, one HTTP call covers both the quote
    used by Assets/Research and the backfill `stock_price_history` needs for
    TWR) — no separate `/simple/price` call needed.
    """
    response = requests.get(
        f"{COINGECKO_BASE_URL}/coins/{coin_id}/market_chart",
        params={"vs_currency": "usd", "days": days, "interval": "daily"},
        timeout=15,
    )
    response.raise_for_status()
    prices = response.json().get("prices", [])

    # A day can appear twice near the boundary (today's still-forming
    # candle) — keep the last point seen per date, then sort chronologically.
    by_date: dict[str, float] = {}
    for epoch_ms, price in prices:
        date = datetime.fromtimestamp(epoch_ms / 1000, tz=timezone.utc).date().isoformat()
        by_date[date] = price

    return [
        {"price_date": date, "price": price}
        for date, price in sorted(by_date.items())
    ]

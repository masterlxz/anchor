"""Cliente do Yahoo Finance pra metais preciosos (Fase 10, item 8, Sessão 55).

Diferente de Ação/FII/ETF/BDR (todos `{ticker}.SA`, listados na B3), metal
não é negociado na B3 — a fonte é o contrato futuro do COMEX, mesmo endpoint
`v8/finance/chart` mas **sem** o sufixo `.SA`. Confirmado ao vivo
(2026-08-02): `GC=F` (ouro) responde tanto pro preço atual (`meta.
regularMarketPrice`) quanto pro histórico diário (`range=10y&interval=1d`,
2517 pontos) — mesmo shape que `acoes_yahoo.py` já usa, só o ticker muda.

Cotação do COMEX vem em USD por **onça troy** (unidade padrão do contrato),
mas o cadastro deste app usa **grama** pra metal (pedido original, Sessão
30) — convertido aqui, uma vez, na fonte, pra que o resto do app
(`stock_quotes`/`stock_price_history`, TWR, posições) nunca precise saber
disso: grava sempre preço por grama, igual qualquer outro `close_price`/
`price` do projeto.

`TICKER_TO_YAHOO_SYMBOL` é o único lugar que precisa mudar pra adicionar
outro metal (prata, platina, etc.) — o dono do projeto escolheu começar só
com ouro (`XAU`, código internacional padrão de metal) nesta sessão.
"""

from datetime import datetime, timezone

import requests

YAHOO_CHART_URL = "https://query1.finance.yahoo.com/v8/finance/chart"
HISTORY_RANGE = "10y"
TROY_OUNCE_GRAMS = 31.1034768

TICKER_TO_YAHOO_SYMBOL = {
    "XAU": "GC=F",
}

METAL_NAMES = {
    "XAU": "Gold",
}


def fetch_quote_and_history(ticker: str) -> dict:
    """Cotação atual + histórico diário (10y) de um metal, preço já em USD/grama.

    Levanta `RuntimeError` se `ticker` não estiver em `TICKER_TO_YAHOO_SYMBOL`
    (nunca chuta um símbolo) ou se a chamada ao Yahoo falhar.
    """
    yahoo_symbol = TICKER_TO_YAHOO_SYMBOL.get(ticker)
    if yahoo_symbol is None:
        raise RuntimeError(f"Unsupported metal ticker '{ticker}'")

    response = requests.get(
        f"{YAHOO_CHART_URL}/{yahoo_symbol}",
        params={"range": HISTORY_RANGE, "interval": "1d"},
        headers={"User-Agent": "Mozilla/5.0"},
        timeout=15,
    )
    response.raise_for_status()
    chart_result = response.json()["chart"]["result"][0]

    meta = chart_result["meta"]
    price_per_gram = meta["regularMarketPrice"] / TROY_OUNCE_GRAMS

    timestamps = chart_result["timestamp"]
    closes = chart_result["indicators"]["quote"][0]["close"]
    history = []
    for ts, close in zip(timestamps, closes):
        if close is None:
            continue
        price_date = datetime.fromtimestamp(ts, tz=timezone.utc).date().isoformat()
        history.append({"price_date": price_date, "close_price": close / TROY_OUNCE_GRAMS})

    return {
        "ticker": ticker,
        "name": METAL_NAMES.get(ticker, ticker),
        "price": price_per_gram,
        "history": history,
    }

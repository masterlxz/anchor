"""Cliente do Yahoo Finance pra metais preciosos (Fase 10, item 8, Sessão 55,
unidade revisada + Prata/Platina/Paládio na Sessão 57).

Diferente de Ação/FII/ETF/BDR (todos `{ticker}.SA`, listados na B3), metal
não é negociado na B3 — a fonte é o contrato futuro do COMEX/NYMEX, mesmo
endpoint `v8/finance/chart` mas **sem** o sufixo `.SA`. Confirmado ao vivo
(2026-08-02): `GC=F` (ouro), `SI=F` (prata), `PL=F` (platina) e `PA=F`
(paládio) respondem todos no mesmo shape — preço atual (`meta.
regularMarketPrice`) e histórico diário (`range=10y&interval=1d`) — mesmo
formato que `acoes_yahoo.py` já usa, só o ticker muda. Cobre (`HG=F`) existe
também, mas cota em USD por **libra-peso**, não onça — fora de escopo por
decisão do dono do projeto (unidade diferente dos outros 4).

**Unidade — revisado na Sessão 57**: a Sessão 55 convertia o preço de onça
troy pra grama na fonte, achando que bateria com o pedido original da Fase
10 ("quantidade em gramas"). Testando, o dono do projeto pediu o oposto: o
preço/quantidade fica **sempre em onça troy** — a unidade real do contrato
COMEX/NYMEX, sem conversão nenhuma aqui. Um indicador auxiliar simples
"US$/g" existe só na tela de Research (`MetalLookupSection.tsx`), calculado
ali por exibição — nunca grava `close_price`/`price` convertido.

`TICKER_TO_YAHOO_SYMBOL`/`METAL_NAMES` são os únicos lugares que precisam
mudar pra adicionar outro metal — usam os códigos ISO 4217 padrão de metal
precioso (XAU/XAG/XPT/XPD), mesma convenção internacional.
"""

from datetime import datetime, timezone

import requests

YAHOO_CHART_URL = "https://query1.finance.yahoo.com/v8/finance/chart"
HISTORY_RANGE = "10y"

TICKER_TO_YAHOO_SYMBOL = {
    "XAU": "GC=F",
    "XAG": "SI=F",
    "XPT": "PL=F",
    "XPD": "PA=F",
}

METAL_NAMES = {
    "XAU": "Gold",
    "XAG": "Silver",
    "XPT": "Platinum",
    "XPD": "Palladium",
}


def fetch_quote_and_history(ticker: str) -> dict:
    """Cotação atual + histórico diário (10y) de um metal, preço em USD/onça
    troy — o valor cru que o COMEX/NYMEX já devolve, sem conversão nenhuma.

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
    price = meta["regularMarketPrice"]

    timestamps = chart_result["timestamp"]
    closes = chart_result["indicators"]["quote"][0]["close"]
    history = []
    for ts, close in zip(timestamps, closes):
        if close is None:
            continue
        price_date = datetime.fromtimestamp(ts, tz=timezone.utc).date().isoformat()
        history.append({"price_date": price_date, "close_price": close})

    return {
        "ticker": ticker,
        "name": METAL_NAMES.get(ticker, ticker),
        "price": price,
        "history": history,
    }

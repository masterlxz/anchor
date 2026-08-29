"""Cliente da API da bolsai — resto local após a Fase 1.7 do EasyBusiness.

Endpoint e formato de resposta confirmados em usebolsai.com/docs
(2026-07-10): base URL https://api.usebolsai.com/api/v1, autenticação via
header X-API-Key (chave gratuita obtida com login Google no dashboard deles
— sem tickers de teste sem chave, diferente da brapi).

**Fase 1.7 (Sessão 7)**: `fetch_fundamentals` foi removido — a Finance API
(`finance_api_client.fetch_bolsai_fundamentals`, `GET /v1/stocks/{ticker}/
bolsai-fundamentals`) cobre o mesmo dado agora. O que sobra aqui é só
`fetch_fii_summary`, usado por `cvm_fii.resolve_cnpj` — a resolução
ticker→CNPJ de FII nunca foi portada pra Finance API (decisão da Sessão 4).
"""

import os

import requests

BOLSAI_BASE_URL = "https://api.usebolsai.com/api/v1"


def _headers() -> dict:
    api_key = os.environ.get("BOLSAI_API_KEY")
    if not api_key:
        raise RuntimeError(
            "BOLSAI_API_KEY not set — get a free key at usebolsai.com/dashboard "
            "and add it to data-collector/.env"
        )
    return {"X-API-Key": api_key}


def fetch_fii_summary(ticker: str) -> dict | None:
    """Busca o resumo de um FII (`GET /fiis/{ticker}`, plano free — achado
    testando de verdade, 2026-08-02).

    Usado só como passo auxiliar de `cvm_fii.py::resolve_cnpj` — a bolsai não
    devolve o CNPJ do próprio fundo (só `administrator_cnpj`, do
    administrador, que gerencia dezenas de fundos), então isso sozinho não
    identifica o fundo. Combinado com `name` (nome oficial, batendo
    caractere a caractere com `Nome_Fundo_Classe` da CVM — confirmado contra
    o HGLG11 real) dá pra restringir e casar contra o cadastro público da
    CVM sem arriscar. Depois que o CNPJ é resolvido uma vez e salvo em
    `assets.cnpj`, a bolsai não é chamada de novo — toda leitura recorrente
    de indicador vem direto da CVM (decisão do dono do projeto).

    Retorna `{"ticker", "name", "administrator_cnpj"}` ou `None` se o ticker
    não existir/não for FII (404).
    """
    headers = _headers()
    response = requests.get(
        f"{BOLSAI_BASE_URL}/fiis/{ticker}",
        headers=headers,
        timeout=10,
    )
    if response.status_code == 404:
        return None
    response.raise_for_status()
    payload = response.json()

    return {
        "ticker": payload["ticker"],
        "name": payload["name"],
        "administrator_cnpj": payload["administrator_cnpj"],
    }

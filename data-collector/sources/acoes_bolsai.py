"""Cliente da API da bolsai (fundamentos de ações BR).

Endpoint e formato de resposta confirmados em usebolsai.com/docs
(2026-07-10): base URL https://api.usebolsai.com/api/v1, autenticação via
header X-API-Key (chave gratuita obtida com login Google no dashboard deles
— sem tickers de teste sem chave, diferente da brapi).

GET /fundamentals/{ticker} — snapshot atual com ~27 indicadores; usamos só
lpa, vpa (Graham) e roe (Bancos).

**Nota (Sessão 5)**: `GET /dividends/{ticker}` também existia aqui pro
dividendo médio do Bazin, mas retorna 403 no plano Free (exclusivo do plano
Pro) — removido nesta sessão (6) em favor de `acoes_yahoo.py`, que resolve o
mesmo dado de graça e sem exigir cadastro nenhum.
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


def fetch_fundamentals(tickers: list[str]) -> list[dict]:
    """Busca LPA, VPA, ROE, nº de ações e o código CVM atuais de uma lista de tickers.

    Retorna uma lista de {"ticker": str, "lpa": float, "vpa": float,
    "roe": float, "shares_outstanding": float, "cvm_code": str}. Um ticker
    que falhar (404, etc.) é ignorado — não derruba o restante. `cvm_code` é
    reaproveitado por `cvm_dfp.py` (Sessão 5) pra filtrar os dados abertos da
    CVM sem precisar de uma segunda chamada só pra resolver ticker → empresa.
    `shares_outstanding` também é usado pelo DCF — a CVM tem um campo
    equivalente (`composicao_capital`), mas testando contra a VALE3 real ele
    veio com um erro de escala (1000x menor) específico daquela companhia;
    o da bolsai já vem conferido, então é o que o DCF usa.
    """
    headers = _headers()
    results = []

    for ticker in tickers:
        response = requests.get(
            f"{BOLSAI_BASE_URL}/fundamentals/{ticker}",
            headers=headers,
            timeout=10,
        )
        if response.status_code == 404:
            continue
        response.raise_for_status()
        payload = response.json()

        results.append(
            {
                "ticker": payload["ticker"],
                "lpa": payload["lpa"],
                "vpa": payload["vpa"],
                "roe": payload["roe"],
                "shares_outstanding": payload["shares_outstanding"],
                "cvm_code": payload["cvm_code"],
            }
        )

    return results


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

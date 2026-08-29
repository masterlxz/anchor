"""Cliente HTTP da Finance API do EasyBusiness (Fase 1.7, Sessão 7).

Substitui, pra tudo que ela cobre, as fontes locais que `main.py` chamava
direto (Yahoo Finance, BCB SGS, CVM DFP/FII, cripto, metais, B3 index
stats) — a Finance API agora é a fonte única de verdade pra esses dados,
rodando em `docker compose up` no repo `easybusiness` (self-host).

Cada função abaixo replica a forma exata (mesmas chaves de dict) da função
antiga que substitui, pra minimizar a mudança no corpo das `collect_*` de
`main.py`. Onde a fonte antiga recebia uma lista de tickers/CNPJs mas a
Finance API só aceita **um identificador por chamada** (decisão de design
documentada em `easybusiness/project/ARCHITECTURE.md`: "quem chama decide o
batch"), o loop fica escondido aqui dentro, sequencial — sem paralelismo,
decisão consciente pra não sobrecarregar uma API self-hosted de single-user.

Não cobre tudo que `main.py` fazia antes — ver `README.md` pra a lista do
que continua local (cotação/técnicos/dividendos/preço de ticker sem sufixo
`.SA`, REIT fundamentals, IBOV, resolução de CNPJ de FII).
"""

import os

import requests

BASE_URL_ENV_VAR = "FINANCE_API_BASE_URL"
API_KEY_ENV_VAR = "FINANCE_API_KEY"
DEFAULT_BASE_URL = "http://localhost:8000"

TIMEOUT_SECONDS = 30


class FinanceApiError(RuntimeError):
    """Finance API fora do ar, chave ausente, ou erro 5xx — mesmo contrato
    `RuntimeError` que `acoes_bolsai`/`sec_edgar` já usam, pra qualquer
    `except RuntimeError` existente em `main.py` continuar funcionando."""


class FinanceApiNotFoundError(FinanceApiError):
    """404 — identificador desconhecido no catálogo, ou sem dado disponível
    (a Finance API não distingue os dois casos na maioria das rotas, ver
    `easybusiness/project/ARCHITECTURE.md`)."""


def _base_url() -> str:
    return os.environ.get(BASE_URL_ENV_VAR, DEFAULT_BASE_URL).rstrip("/")


def _headers() -> dict:
    api_key = os.environ.get(API_KEY_ENV_VAR)
    if not api_key:
        raise FinanceApiError(
            f"{API_KEY_ENV_VAR} not set — configure a mesma chave usada em "
            "easybusiness/api/.env (dev) e adicione em data-collector/.env"
        )
    return {"X-API-Key": api_key}


def _get(path: str) -> dict:
    try:
        response = requests.get(f"{_base_url()}{path}", headers=_headers(), timeout=TIMEOUT_SECONDS)
    except requests.RequestException as exc:
        raise FinanceApiError(f"Finance API unreachable ({path}): {exc}") from exc

    if response.status_code == 404:
        raise FinanceApiNotFoundError(f"Finance API 404: {path}")
    if response.status_code >= 400:
        raise FinanceApiError(f"Finance API {response.status_code} on {path}: {response.text}")
    return response.json()


def fetch_quotes(tickers: list[str]) -> list[dict]:
    """Espelha `acoes_yahoo.fetch_quotes` — resiliência por ticker só pra
    404 (ticker sem dado nesse ponto, mesmo espírito do skip antigo do
    Yahoo). Finance API fora do ar/erro 5xx propaga `FinanceApiError` — decisão
    deliberada, diferente do scraping antigo: um erro de infraestrutura na
    fonte única de verdade deve falhar alto, não virar silenciosamente
    "0 resultados, exit 0"."""
    results = []
    for ticker in tickers:
        try:
            data = _get(f"/v1/stocks/{ticker}/quote")
        except FinanceApiNotFoundError:
            continue
        results.append(
            {
                "ticker": ticker,
                "price": data["price"],
                "name": data["name"],
                "exchange": data["exchange"],
                "currency": data["currency"],
            }
        )
    return results


def fetch_price_history(tickers: list[str]) -> list[dict]:
    results = []
    for ticker in tickers:
        try:
            data = _get(f"/v1/stocks/{ticker}/price-history")
        except FinanceApiNotFoundError:
            continue
        for point in data["data"]:
            results.append(
                {
                    "ticker": ticker,
                    "price_date": point["price_date"],
                    "close_price": point["close_price"],
                }
            )
    return results


def fetch_dividends_avg(tickers: list[str]) -> list[dict]:
    results = []
    for ticker in tickers:
        try:
            data = _get(f"/v1/stocks/{ticker}/dividends-avg")
        except FinanceApiNotFoundError:
            continue
        results.append({"ticker": ticker, "avg_dividend_5y": data["avg_dividend_5y"]})
    return results


def fetch_technicals(tickers: list[str]) -> list[dict]:
    results = []
    for ticker in tickers:
        try:
            data = _get(f"/v1/stocks/{ticker}/technicals")
        except FinanceApiNotFoundError:
            continue
        results.append(
            {
                "ticker": ticker,
                "sma_50": data["sma_50"],
                "sma_100": data["sma_100"],
                "sma_200": data["sma_200"],
                "cagr_5y": data["cagr_5y"],
                "cagr_10y": data["cagr_10y"],
            }
        )
    return results


def fetch_dividend_payments(tickers: list[str]) -> list[dict]:
    results = []
    for ticker in tickers:
        try:
            data = _get(f"/v1/stocks/{ticker}/dividend-payments")
        except FinanceApiNotFoundError:
            continue
        for payment in data["data"]:
            results.append(
                {
                    "ticker": ticker,
                    "payment_date": payment["payment_date"],
                    "amount": payment["amount"],
                    "price_at_payment": payment["price_at_payment"],
                    "yield_pct": payment["yield_pct"],
                }
            )
    return results


def fetch_bolsai_fundamentals(tickers: list[str]) -> list[dict]:
    """Espelha `acoes_bolsai.fetch_fundamentals` (removida) — FII sempre
    404 aqui (a bolsai não cobre FII em `/fundamentals`), ignorado como
    antes, não quebra o resto do coletor."""
    results = []
    for ticker in tickers:
        try:
            data = _get(f"/v1/stocks/{ticker}/bolsai-fundamentals")
        except FinanceApiNotFoundError:
            continue
        results.append(
            {
                "ticker": ticker,
                "lpa": data["lpa"],
                "vpa": data["vpa"],
                "roe": data["roe"],
                "shares_outstanding": data["shares_outstanding"],
                "cvm_code": data["cvm_code"],
            }
        )
    return results


def fetch_company_roe(ticker_cvm_codes: dict[str, str]) -> list[dict]:
    """Espelha `cvm_dfp.fetch_roe` (removida). Ticker sem ROE extraível na
    CVM é descartado inteiro — mesma regra de antes."""
    results = []
    for ticker, cvm_code in ticker_cvm_codes.items():
        try:
            data = _get(f"/v1/companies/{int(cvm_code)}/roe")
        except FinanceApiNotFoundError:
            continue
        results.append({"ticker": ticker, "roe": data["roe"]})
    return results


def fetch_company_payout(ticker_cvm_codes: dict[str, str]) -> list[dict]:
    """Espelha `cvm_dfp.fetch_payout` (removida) — campo `payout_avg_5y` da
    Finance API renomeado pra `payout`, mesma chave que `main.py` já espera."""
    results = []
    for ticker, cvm_code in ticker_cvm_codes.items():
        try:
            data = _get(f"/v1/companies/{int(cvm_code)}/payout")
        except FinanceApiNotFoundError:
            continue
        results.append({"ticker": ticker, "payout": data["payout_avg_5y"]})
    return results


def fetch_company_dcf_fundamentals(ticker_cvm_codes: dict[str, str]) -> list[dict]:
    """Espelha `cvm_dfp.fetch_dcf_fundamentals` (removida) — mesmo shape de
    9 campos, `shares_outstanding` continua vindo de fora (bolsai), como
    antes."""
    results = []
    for ticker, cvm_code in ticker_cvm_codes.items():
        try:
            data = _get(f"/v1/companies/{int(cvm_code)}/dcf-fundamentals")
        except FinanceApiNotFoundError:
            continue
        results.append(
            {
                "ticker": ticker,
                "reference_year": data["reference_year"],
                "ebit": data["ebit"],
                "tax_rate": data["tax_rate"],
                "depreciation_amortization": data["depreciation_amortization"],
                "capex": data["capex"],
                "nwc_change": data["nwc_change"],
                "total_debt": data["total_debt"],
                "cash": data["cash"],
                "revenue": data["revenue"],
                "inventory": data["inventory"],
            }
        )
    return results


def fetch_benchmark_series(series_code: str) -> list[dict]:
    """Espelha `bcb_sgs.fetch_monthly_series` (removida) — `series_code` é
    o código da Finance API (`"cdi"`/`"ipca"`), não mais o código numérico do
    SGS. `reference_month` (data ISO, primeiro dia do mês) vira
    `year_month` (`"YYYY-MM"`), formato que `main.py` já grava."""
    data = _get(f"/v1/macro-series/{series_code}")
    return [
        {"year_month": point["reference_month"][:7], "value_pct": point["value_pct"]}
        for point in data["data"]
    ]


def fetch_b3_index_history(index_code: str) -> list[dict]:
    """Espelha `b3_index_stats.fetch_index_history` (removida) — sem
    `start_year`/`end_year`: a Finance API já devolve o histórico completo.
    Catálogo da Finance API usa código minúsculo (`ifix`), diferente do
    ticker maiúsculo que `main.py` usa como chave em `stock_price_history`."""
    data = _get(f"/v1/b3-indexes/{index_code.lower()}/history")
    return [
        {"price_date": point["price_date"], "close_price": point["close_price"]}
        for point in data["data"]
    ]


def fetch_fii_monthly_indicators(cnpjs: list[str]) -> list[dict]:
    """Espelha `cvm_fii.fetch_monthly_indicators` (removida) — CNPJ sem
    dado (404) é ignorado, mesma tolerância de antes."""
    results = []
    for cnpj in cnpjs:
        try:
            data = _get(f"/v1/fiis/{cnpj}/monthly-indicators")
        except FinanceApiNotFoundError:
            continue
        results.append(
            {
                "cnpj": cnpj,
                "reference_date": data["reference_date"],
                "patrimonio_liquido": data["patrimonio_liquido"],
                "valor_patrimonial_cota": data["valor_patrimonial_cota"],
                "numero_cotistas": data["numero_cotistas"],
                "dividend_yield_mes": data["dividend_yield_mes"],
                "rentabilidade_efetiva_mes": data["rentabilidade_efetiva_mes"],
            }
        )
    return results


def fetch_fii_properties(cnpjs: list[str]) -> list[dict]:
    """Espelha `cvm_fii.fetch_property_data` (removida). `/properties`
    devolve `data: []` tanto pra CNPJ desconhecido quanto pra FII sem
    imóvel (limitação documentada em `easybusiness/project/PENDING.md`,
    item P1) — lista vazia aqui é sempre "sem imóvel", nunca erro."""
    results = []
    for cnpj in cnpjs:
        try:
            data = _get(f"/v1/fiis/{cnpj}/properties")
        except FinanceApiNotFoundError:
            continue
        for item in data["data"]:
            results.append(
                {
                    "cnpj": cnpj,
                    "reference_date": item["reference_date"],
                    "nome_imovel": item["nome_imovel"],
                    "endereco": item["endereco"],
                    "area_m2": item["area_m2"],
                    "percentual_vacancia": item["percentual_vacancia"],
                    "percentual_inadimplencia": item["percentual_inadimplencia"],
                    "percentual_receitas_fii": item["percentual_receitas_fii"],
                    "percentual_locado": item["percentual_locado"],
                }
            )
    return results


def fetch_eth_indicator(indicator_code: str) -> float:
    """Espelha as 4 funções de indicador ETH removidas
    (`cripto_defillama.fetch_tvl_trend_mom`,
    `cripto_ultrasound.fetch_net_issuance_annualized_pct`/
    `fetch_fees_vs_emission_ratio`, `cripto_coingecko.fetch_nvt_ratio_vs_ma90`)
    — `indicator_code` é um dos 4 códigos do catálogo da Finance API (hífen,
    não underscore — diferente do nome interno que `main.py` usa em
    `indicator_thresholds`/`crypto_indicators`): `tvl-trend`, `net-issuance`,
    `fees-vs-emission`, `nvt-ratio`."""
    data = _get(f"/v1/crypto/eth-indicators/{indicator_code}")
    return data["raw_value"]


def fetch_fear_greed() -> dict:
    """Espelha `cripto_feargreed.fetch_latest` (removida)."""
    data = _get("/v1/crypto/fear-greed")
    return {
        "value": data["value"],
        "classification": data["classification"],
        "reading_date": data["reading_date"],
    }


def fetch_crypto_quote(symbol: str) -> dict:
    """Espelha a parte de cotação de `cripto_coingecko` — resolução de
    `coin_id` agora é server-side, `main.py` não precisa mais chamar
    `resolve_coin_id` antes."""
    data = _get(f"/v1/crypto/{symbol}/quote")
    return {"coin_id": data["coin_id"], "name": data["name"], "price": data["price"]}


def fetch_crypto_price_history(symbol: str) -> list[dict]:
    data = _get(f"/v1/crypto/{symbol}/price-history")
    return [{"price_date": p["price_date"], "price": p["price"]} for p in data["data"]]


def fetch_metal_quote(metal_code: str) -> dict:
    """Espelha `metais_yahoo.fetch_quote_and_history` (removida) — cotação
    e histórico viraram duas chamadas (`fetch_metal_quote`/
    `fetch_metal_price_history`), a Finance API já expõe assim. Catálogo da
    Finance API usa código minúsculo (`xau`), diferente do ticker maiúsculo
    que `main.py` usa como chave em `stock_quotes`/`stock_price_history`."""
    data = _get(f"/v1/metals/{metal_code.lower()}/quote")
    return {"name": data["name"], "price": data["price"]}


def fetch_metal_price_history(metal_code: str) -> list[dict]:
    data = _get(f"/v1/metals/{metal_code.lower()}/price-history")
    return [{"price_date": p["price_date"], "close_price": p["close_price"]} for p in data["data"]]


def fetch_us_stock_fundamentals(tickers: list[str]) -> list[dict]:
    """Espelha `sec_edgar.fetch_fundamentals` (removida) — a Finance API
    resolve CIK internamente, `main.py` não precisa mais de `resolve_ciks`
    pra esse caminho."""
    results = []
    for ticker in tickers:
        try:
            data = _get(f"/v1/us-stocks/{ticker}/fundamentals")
        except FinanceApiNotFoundError:
            continue
        results.append(
            {
                "ticker": ticker,
                "lpa": data["lpa"],
                "vpa": data["vpa"],
                "roe": data["roe"],
                "shares_outstanding": data["shares_outstanding"],
            }
        )
    return results


def fetch_us_stock_dcf_fundamentals(tickers: list[str]) -> list[dict]:
    """Espelha `sec_edgar.fetch_dcf_fundamentals` (removida) — recebe
    tickers direto (não `ticker_ciks`, a resolução é server-side)."""
    results = []
    for ticker in tickers:
        try:
            data = _get(f"/v1/us-stocks/{ticker}/dcf-fundamentals")
        except FinanceApiNotFoundError:
            continue
        results.append(
            {
                "ticker": ticker,
                "reference_year": data["reference_year"],
                "ebit": data["ebit"],
                "tax_rate": data["tax_rate"],
                "depreciation_amortization": data["depreciation_amortization"],
                "capex": data["capex"],
                "nwc_change": data["nwc_change"],
                "total_debt": data["total_debt"],
                "cash": data["cash"],
                "revenue": data["revenue"],
                "inventory": data["inventory"],
            }
        )
    return results


def fetch_us_stock_payout(tickers: list[str]) -> list[dict]:
    """Espelha `sec_edgar.fetch_payout` (removida)."""
    results = []
    for ticker in tickers:
        try:
            data = _get(f"/v1/us-stocks/{ticker}/payout")
        except FinanceApiNotFoundError:
            continue
        results.append({"ticker": ticker, "payout": data["payout_avg_5y"]})
    return results

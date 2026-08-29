"""Cliente da SEC EDGAR — resto local após a Fase 1.7 do EasyBusiness.

Gratuita, sem chave — só exige um header `User-Agent` com e-mail de contato
(política deles, não autenticação de verdade: https://www.sec.gov/os/webmaster-faq#developers)
e respeita um teto de 10 req/s. Confirmado ao vivo (Sessão 58/59) contra a
Apple (CIK 320193) e a JPMorgan (CIK 19617).

**Fase 1.7 (Sessão 7)**: `fetch_fundamentals`/`fetch_dcf_fundamentals`/
`fetch_payout` (ação americana comum) foram removidos — a Finance API
(`finance_api_client`, `GET /v1/us-stocks/{ticker}/{fundamentals,
dcf-fundamentals,payout}`) cobre os três agora, resolvendo CIK internamente.
O que sobra aqui é `resolve_ciks` e `fetch_reit_fundamentals` — REIT nunca
foi portado pra Finance API (fora do catálogo original de 12 fontes).

Diferente da CVM (um zip por ano, códigos de conta numéricos), a SEC expõe
XBRL por empresa: `GET /api/xbrl/companyconcept/CIK{cik}/us-gaap/{tag}.json`
devolve o **histórico completo** de uma tag (`Revenues`, `NetIncomeLoss`,
...) numa chamada só.

**Achado crítico, testando ao vivo contra a Apple**: filtrar só por
`form == "10-K"` e `fp == "FY"` não basta pra conceitos de fluxo (duration,
os que têm campo `start`). A Apple usa a mesma tag nas notas de rodapé "dados
trimestrais selecionados" dentro do próprio 10-K — `Revenues` e
`PaymentsOfDividendsCommonStock` vieram com a maioria das linhas de
curta duração (trimestre/9 meses) mesmo com `form`/`fp` corretos, enquanto
`OperatingIncomeLoss`/`DepreciationAndAmortization` vieram limpos. A correção
é exigir também que a janela `end - start` tenha entre 350 e 380 dias antes
de aceitar uma linha como anual. Conceitos instant (sem `start`, ex.:
`StockholdersEquity`, `CommonStockSharesOutstanding`) não precisam desse
filtro extra.

**Restatement (10-K/A)**: mais de uma linha pode compartilhar o mesmo `end`
(uma correção re-arquiva o mesmo período) — desempate pela linha com `filed`
mais recente, não a primeira vista.

**Escala**: todo campo monetário agregado (revenue, patrimônio, etc.) é
devolvido em **milhões de dólares** (`_to_millions`) — necessário pra bater
com os rótulos "(US$ millions)" que a UI já assume. LPA (por ação),
VPA/ROE (razões) não são escalados.
"""

import os
import time
from datetime import date
from pathlib import Path

import requests

EDGAR_TICKERS_URL = "https://www.sec.gov/files/company_tickers.json"
EDGAR_CONCEPT_URL_TEMPLATE = (
    "https://data.sec.gov/api/xbrl/companyconcept/CIK{cik:010d}/us-gaap/{tag}.json"
)
CACHE_DIR = Path(__file__).parent.parent / ".cache" / "sec_edgar"
TICKERS_CACHE_PATH = CACHE_DIR / "company_tickers.json"

# Diferente do zip da CVM (nome do arquivo já carrega o ano, nunca fica
# "velho" por engano), `company_tickers.json` não tem versão no nome — cache
# eterno deixaria um IPO novo irresolvível pra sempre. TTL curto é o desvio
# deliberado do idioma `_resolve_zip_path`.
TICKERS_CACHE_TTL_SECONDS = 7 * 24 * 60 * 60

REQUEST_INTERVAL_SECONDS = 0.11  # ~9 req/s, sob o teto de 10 req/s da SEC

_ANNUAL_DURATION_MIN_DAYS = 350
_ANNUAL_DURATION_MAX_DAYS = 380

_last_request_at = 0.0


def _contact_email() -> str:
    email = os.environ.get("SEC_EDGAR_CONTACT_EMAIL")
    if not email:
        raise RuntimeError(
            "SEC_EDGAR_CONTACT_EMAIL not set — a SEC EDGAR exige um e-mail de "
            "contato no header User-Agent (https://www.sec.gov/os/webmaster-faq#developers), "
            "adicione em data-collector/.env"
        )
    return email


def _headers() -> dict:
    return {"User-Agent": f"anchor ({_contact_email()})"}


def _get(url: str) -> requests.Response:
    """Wrapper único de requisição — todo o módulo passa por aqui, garante
    o rate limit num lugar só."""
    global _last_request_at
    elapsed = time.monotonic() - _last_request_at
    if elapsed < REQUEST_INTERVAL_SECONDS:
        time.sleep(REQUEST_INTERVAL_SECONDS - elapsed)
    response = requests.get(url, headers=_headers(), timeout=15)
    _last_request_at = time.monotonic()
    return response


def _download_tickers() -> Path:
    if (
        TICKERS_CACHE_PATH.exists()
        and time.time() - TICKERS_CACHE_PATH.stat().st_mtime < TICKERS_CACHE_TTL_SECONDS
    ):
        return TICKERS_CACHE_PATH

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    response = _get(EDGAR_TICKERS_URL)
    response.raise_for_status()
    TICKERS_CACHE_PATH.write_bytes(response.content)
    return TICKERS_CACHE_PATH


def resolve_ciks(tickers: list[str]) -> dict[str, int]:
    """{ticker: cik} pros tickers resolvidos — ticker não encontrado é só
    ausente do dict devolvido (mesmo contrato de "404 → pula" da bolsai)."""
    import json

    path = _download_tickers()
    entries = json.loads(path.read_text())

    by_ticker = {entry["ticker"].upper(): entry["cik_str"] for entry in entries.values()}
    wanted = {t.upper() for t in tickers}
    return {ticker: cik for ticker, cik in by_ticker.items() if ticker in wanted}


def _fetch_concept(cik: int, tag: str) -> list[dict] | None:
    """Devolve as linhas de `units` pra essa tag (`USD`, `USD/shares` ou
    `shares` — a chave varia por tag, tenta as três). `None` se a empresa
    não reporta essa tag (404 — esperado pra bancos em vários campos, ver
    docstring do módulo)."""
    response = _get(EDGAR_CONCEPT_URL_TEMPLATE.format(cik=cik, tag=tag))
    if response.status_code == 404:
        return None
    response.raise_for_status()
    payload = response.json()

    units = payload.get("units", {})
    for unit_key in ("USD", "USD/shares", "shares"):
        if unit_key in units:
            return units[unit_key]
    return None


def _is_annual_duration(row: dict) -> bool:
    """`True` pra conceito instant (sem `start`) ou pra linha de duração
    realmente anual (achado crítico: `form`/`fp` sozinhos não bastam, ver
    docstring do módulo)."""
    if "start" not in row:
        return True
    start = date.fromisoformat(row["start"])
    end = date.fromisoformat(row["end"])
    days = (end - start).days
    return _ANNUAL_DURATION_MIN_DAYS <= days <= _ANNUAL_DURATION_MAX_DAYS


def _annual_rows(rows: list[dict]) -> list[dict]:
    return [
        r for r in rows if r.get("form") == "10-K" and r.get("fp") == "FY" and _is_annual_duration(r)
    ]


def _latest_duration(rows: list[dict]) -> dict | None:
    candidates = _annual_rows(rows)
    if not candidates:
        return None
    max_end = max(r["end"] for r in candidates)
    same_end = [r for r in candidates if r["end"] == max_end]
    return max(same_end, key=lambda r: r["filed"])


def _latest_instant(rows: list[dict], rank: int = 0) -> dict | None:
    """`rank=0` é o exercício mais recente — distintos por `end`, desempate
    por `filed` mais recente dentro do mesmo `end`."""
    candidates = _annual_rows(rows)
    if not candidates:
        return None
    ends = sorted({r["end"] for r in candidates}, reverse=True)
    if rank >= len(ends):
        return None
    same_end = [r for r in candidates if r["end"] == ends[rank]]
    return max(same_end, key=lambda r: r["filed"])


def _try_tags(cik: int, tags: list[str], picker) -> dict | None:
    """Tenta cada tag candidata e devolve a linha mais recente **entre
    todas as que resolveram**, não a primeira tag que tiver qualquer
    dado. Achado ao vivo que exigiu isso: a tag `Revenues` da Apple tem
    dado real, mas só até o exercício de 2018 (a empresa migrou pra
    `RevenueFromContractWithCustomerExcludingAssessedTax` depois da nova
    norma contábil ASC 606) — "primeira tag com dado" pegaria 2018 em vez
    do ano corrente disponível na tag mais nova. Desempate por `end`
    (mais recente vence) e depois por `filed` (correção mais recente
    vence)."""
    candidates = []
    for tag in tags:
        rows = _fetch_concept(cik, tag)
        if rows is None:
            continue
        row = picker(rows)
        if row is not None:
            candidates.append(row)
    if not candidates:
        return None
    return max(candidates, key=lambda r: (r["end"], r["filed"]))


def _required(cik: int, tags: list[str], picker) -> float:
    row = _try_tags(cik, tags, picker)
    if row is None:
        raise LookupError(f"nenhuma das tags {tags!r} encontrável pro CIK {cik}")
    return row["val"]


def _optional(cik: int, tags: list[str], picker) -> float | None:
    row = _try_tags(cik, tags, picker)
    return None if row is None else row["val"]


def _to_millions(val: float) -> float:
    return val / 1_000_000


def _to_millions_or_none(val: float | None) -> float | None:
    return None if val is None else _to_millions(val)


def fetch_reit_fundamentals(ticker_ciks: dict[str, int]) -> list[dict]:
    """Indicadores imobiliários pra REIT (Fase 10, item 8) — DCF clássico e
    LPA/VPA/ROE não encaixam bem em imobiliário (mesmo espírito de FII não
    ter esses campos), por isso REIT usa um shape próprio, não o de
    ação americana comum (agora servido pela Finance API). **Achado ao vivo,
    confirmado contra Realty Income/Simon Property/Prologis/AvalonBay**:
    FFO/AFFO e taxa de ocupação — os indicadores "de verdade" de REIT — não
    existem como tag XBRL em nenhuma taxonomia (`us-gaap`/`srt`/`invest`),
    são métricas non-GAAP só em texto/tabela do 10-K. Os campos abaixo são o
    que sobra automatizável: receita, valor de imóveis, patrimônio líquido,
    LPA, lucro líquido.

    `NetIncomeLoss` é inconsistente entre REITs (Simon Property, um UPREIT,
    não reporta essa tag — usa `ProfitLoss`) — tratado como opcional com
    fallback de tag via `_try_tags`.

    Obrigatórios (ticker descartado se faltar): receita, patrimônio
    líquido, LPA — as 3 tags consistentes nas 4 empresas testadas.
    Opcionais: valor de imóveis (custo e líquido), lucro líquido.
    """
    if not ticker_ciks:
        return []

    results = []
    for ticker, cik in ticker_ciks.items():
        try:
            revenue_row = _try_tags(
                cik,
                ["Revenues", "RevenueFromContractWithCustomerExcludingAssessedTax"],
                _latest_duration,
            )
            if revenue_row is None:
                continue

            equity = _required(cik, ["StockholdersEquity"], lambda rows: _latest_instant(rows, 0))
            eps = _required(cik, ["EarningsPerShareDiluted"], _latest_duration)

            results.append(
                {
                    "ticker": ticker,
                    "reference_year": revenue_row["fy"],
                    "revenue": _to_millions(revenue_row["val"]),
                    "real_estate_property_net": _to_millions_or_none(
                        _optional(
                            cik,
                            ["RealEstateInvestmentPropertyNet"],
                            lambda rows: _latest_instant(rows, 0),
                        )
                    ),
                    "real_estate_property_at_cost": _to_millions_or_none(
                        _optional(
                            cik,
                            ["RealEstateInvestmentPropertyAtCost"],
                            lambda rows: _latest_instant(rows, 0),
                        )
                    ),
                    "stockholders_equity": _to_millions(equity),
                    "net_income": _to_millions_or_none(
                        _optional(cik, ["NetIncomeLoss", "ProfitLoss"], _latest_duration)
                    ),
                    "eps_diluted": eps,
                }
            )
        except LookupError:
            continue

    return results

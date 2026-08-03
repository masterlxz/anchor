"""Cliente da SEC EDGAR (fundamentos americanos, equivalente da CVM pro DCF/FCFF).

Gratuita, sem chave — só exige um header `User-Agent` com e-mail de contato
(política deles, não autenticação de verdade: https://www.sec.gov/os/webmaster-faq#developers)
e respeita um teto de 10 req/s. Confirmado ao vivo (Sessão 58/59) contra a
Apple (CIK 320193) e a JPMorgan (CIK 19617).

Diferente da CVM (um zip por ano, códigos de conta numéricos), a SEC expõe
XBRL por empresa: `GET /api/xbrl/companyconcept/CIK{cik}/us-gaap/{tag}.json`
devolve o **histórico completo** de uma tag (`Revenues`, `NetIncomeLoss`,
...) numa chamada só — inclusive exercício atual e anterior pro mesmo
conceito, o que elimina a necessidade de uma segunda requisição pra
`nwc_change` (a CVM precisa de ÚLTIMO/PENÚLTIMO do mesmo arquivo anual; aqui
já vêm juntos na mesma resposta).

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

**Gap de taxonomia de banco (achado ao vivo, espelha o mesmo achado da CVM
pra bancos com COSIF)**: testando a JPMorgan real, `OperatingIncomeLoss`,
`InventoryNet`, `AccountsReceivableNetCurrent`, `AccountsPayableCurrent`,
`PaymentsToAcquirePropertyPlantAndEquipment`, `LongTermDebtNoncurrent` e
`RevenueFromContractWithCustomerExcludingAssessedTax` devolvem 404 — banco
não reporta EBIT/estoque/contas a receber-pagar/capex/receita-de-contrato do
jeito que empresa não-financeira reporta. `fetch_dcf_fundamentals` descarta
esses tickers inteiros (mesmo espírito do `_find_exact`/`LookupError` da
CVM); `fetch_fundamentals` (LPA/VPA/ROE) continua funcionando normalmente
pra eles, já que não depende desses tags.

**Escala**: todo campo monetário agregado (EBIT, D&A, Capex, as 3 pernas do
ΔNWC, dívida, caixa, receita, estoque) é devolvido em **milhões de dólares**
(`_to_millions`), mesma convenção de `cvm_dfp._to_millions_brl` — necessário
pra bater com os rótulos "(R$ millions)"/"(US$ millions)" que a UI já
assume. LPA (por ação), VPA/ROE/alíquota (razões — numerador e denominador
já na mesma escala, a razão não muda) não são escalados.
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
PAYOUT_YEARS_AVERAGED = 5

# Mesma constante/racional de `cvm_dfp._MAX_PLAUSIBLE_TAX_RATE`: rede de
# segurança extra pra razões positivas-mas-instáveis (resultado pré-tributos
# perto de zero) que já passariam pelo guard de `pretax_income <= 0`.
_MAX_PLAUSIBLE_TAX_RATE = 100.0

_ANNUAL_DURATION_MIN_DAYS = 350
_ANNUAL_DURATION_MAX_DAYS = 380

DIVIDEND_TAGS = ["PaymentsOfDividendsCommonStock", "PaymentsOfDividends"]

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
    return {"User-Agent": f"practice-valuation ({_contact_email()})"}


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
    """`rank=0` é o exercício mais recente, `rank=1` o anterior (pro
    `nwc_change`) — distintos por `end`, desempate por `filed` mais
    recente dentro do mesmo `end`."""
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


def _required_instant_at(cik: int, tag: str, rank: int) -> float:
    rows = _fetch_concept(cik, tag)
    if rows is None:
        raise LookupError(f"tag {tag!r} não encontrável pro CIK {cik}")
    row = _latest_instant(rows, rank)
    if row is None:
        raise LookupError(f"tag {tag!r} sem exercício de rank {rank} pro CIK {cik}")
    return row["val"]


def _nwc_change(cik: int) -> float:
    """ΔNWC = (Contas a Receber + Estoques − Fornecedores) no exercício
    atual menos o mesmo cálculo no anterior — mesma fórmula de
    `cvm_dfp._nwc_change`, só que lendo os dois exercícios da mesma
    resposta por tag em vez de duas colunas de um arquivo anual."""

    def nwc_at(rank: int) -> float:
        receivables = _required_instant_at(cik, "AccountsReceivableNetCurrent", rank)
        inventory = _required_instant_at(cik, "InventoryNet", rank)
        payables = _required_instant_at(cik, "AccountsPayableCurrent", rank)
        return receivables + inventory - payables

    return _to_millions(nwc_at(0) - nwc_at(1))


def _effective_tax_rate(cik: int) -> float | None:
    """Espelha `cvm_dfp._effective_tax_rate`: `None` se o resultado antes
    dos tributos for zero/negativo ou a razão sair fora de uma faixa
    plausível — instabilidade matemática, não alíquota de verdade."""
    pretax_income = _optional(
        cik,
        ["IncomeLossFromContinuingOperationsBeforeIncomeTaxesExtraordinaryItemsNoncontrollingInterest"],
        _latest_duration,
    )
    tax_expense = _optional(cik, ["IncomeTaxExpenseBenefit"], _latest_duration)

    if pretax_income is None or tax_expense is None or pretax_income <= 0:
        return None

    tax_rate = tax_expense / pretax_income * 100
    if not (0.0 <= tax_rate <= _MAX_PLAUSIBLE_TAX_RATE):
        return None

    return tax_rate


def fetch_fundamentals(tickers: list[str]) -> list[dict]:
    """Busca LPA, VPA, ROE, nº de ações e o CIK atuais de uma lista de
    tickers — mesmo shape de `acoes_bolsai.fetch_fundamentals`, `cik` no
    lugar de `cvm_code` (mesmo papel: reaproveitado por
    `fetch_dcf_fundamentals`/`fetch_payout` sem resolver a empresa de
    novo). Diferente do fluxo BR, não precisa de um `fetch_roe` separado
    corrigindo uma fonte não confiável — aqui ROE já é calculado direto de
    NetIncomeLoss/StockholdersEquity. Ticker sem CIK resolvido ou sem os 4
    campos necessários é descartado inteiro.

    Retorna {"ticker", "lpa", "vpa", "roe", "shares_outstanding", "cik"}.
    """
    ciks = resolve_ciks(tickers)
    results = []

    for ticker, cik in ciks.items():
        try:
            eps = _required(cik, ["EarningsPerShareDiluted"], _latest_duration)
            equity = _required(cik, ["StockholdersEquity"], lambda rows: _latest_instant(rows, 0))
            net_income = _required(cik, ["NetIncomeLoss"], _latest_duration)
            shares = _required(
                cik, ["CommonStockSharesOutstanding"], lambda rows: _latest_instant(rows, 0)
            )
            if equity <= 0 or shares <= 0:
                continue

            results.append(
                {
                    "ticker": ticker,
                    "lpa": eps,
                    "vpa": equity / shares,
                    "roe": net_income / equity * 100,
                    "shares_outstanding": shares,
                    "cik": cik,
                }
            )
        except LookupError:
            continue

    return results


def fetch_dcf_fundamentals(ticker_ciks: dict[str, int]) -> list[dict]:
    """Busca os 9 campos do DCF/Stock Lookup/RNAV pra cada ticker via SEC
    EDGAR — mesmo shape de `cvm_dfp.fetch_dcf_fundamentals`.
    `shares_outstanding` não vem daqui (mesmo motivo do módulo CVM — ver
    `fetch_fundamentals`, reaproveitado por `main.py`). Ticker sem EBIT ou
    sem algum dos campos obrigatórios (dívida, caixa, receita, estoque,
    contas a receber/pagar) é descartado — esperado pra bancos, ver
    docstring do módulo (achado ao vivo contra a JPMorgan)."""
    if not ticker_ciks:
        return []

    results = []
    for ticker, cik in ticker_ciks.items():
        try:
            ebit_row = _try_tags(cik, ["OperatingIncomeLoss"], _latest_duration)
            if ebit_row is None:
                continue

            long_term_debt = _required(cik, ["LongTermDebtNoncurrent"], lambda rows: _latest_instant(rows, 0))
            current_debt = _optional(cik, ["LongTermDebtCurrent"], lambda rows: _latest_instant(rows, 0))

            results.append(
                {
                    "ticker": ticker,
                    "reference_year": ebit_row["fy"],
                    "ebit": _to_millions(ebit_row["val"]),
                    "tax_rate": _effective_tax_rate(cik),
                    "depreciation_amortization": _to_millions_or_none(
                        _optional(
                            cik,
                            [
                                "DepreciationDepletionAndAmortization",
                                "DepreciationAmortizationAndAccretionNet",
                                "DepreciationAndAmortization",
                                # Achado ao vivo (Microsoft): empresa que não usa
                                # nenhuma tag combinada reporta só a depreciação de
                                # imobilizado aqui, separada da amortização de
                                # intangíveis — sub-representa D&A total pra essas
                                # empresas, mesmo tipo de imprecisão que
                                # `cvm_dfp.py` já documenta pra essa mesma linha.
                                "Depreciation",
                            ],
                            _latest_duration,
                        )
                    ),
                    "capex": _to_millions_or_none(
                        _optional(
                            cik, ["PaymentsToAcquirePropertyPlantAndEquipment"], _latest_duration
                        )
                    ),
                    "nwc_change": _nwc_change(cik),
                    "total_debt": _to_millions(long_term_debt + (current_debt or 0.0)),
                    "cash": _to_millions(
                        _required(
                            cik,
                            ["CashAndCashEquivalentsAtCarryingValue"],
                            lambda rows: _latest_instant(rows, 0),
                        )
                    ),
                    "revenue": _to_millions(
                        _required(
                            cik,
                            ["Revenues", "RevenueFromContractWithCustomerExcludingAssessedTax"],
                            _latest_duration,
                        )
                    ),
                    "inventory": _to_millions(
                        _required(
                            cik, ["InventoryNet"], lambda rows: _latest_instant(rows, 0)
                        )
                    ),
                }
            )
        except LookupError:
            continue

    return results


def fetch_reit_fundamentals(ticker_ciks: dict[str, int]) -> list[dict]:
    """Indicadores imobiliários pra REIT (Fase 10, item 8) — diferente de
    `fetch_dcf_fundamentals`/`fetch_fundamentals`, que alimentam os 8
    modelos de valuation que REIT não usa (decisão explícita: DCF clássico
    não encaixa bem em imobiliário, mesmo espírito de FII não ter
    LPA/VPA/ROE). **Achado ao vivo, confirmado contra Realty Income/Simon
    Property/Prologis/AvalonBay**: FFO/AFFO e taxa de ocupação — os
    indicadores "de verdade" de REIT — não existem como tag XBRL em nenhuma
    taxonomia (`us-gaap`/`srt`/`invest`), são métricas non-GAAP só em texto/
    tabela do 10-K. Os campos abaixo são o que sobra automatizável: receita,
    valor de imóveis, patrimônio líquido, LPA, lucro líquido.

    `NetIncomeLoss` é inconsistente entre REITs (Simon Property, um UPREIT,
    não reporta essa tag — usa `ProfitLoss`) — tratado como opcional com
    fallback de tag via `_try_tags`, mesmo mecanismo que a D&A do DCF já usa
    pra empresas que só reportam depreciação separada de amortização.

    Obrigatórios (ticker descartado se faltar, mesmo padrão de
    `fetch_fundamentals`): receita, patrimônio líquido, LPA — as 3 tags
    consistentes nas 4 empresas testadas. Opcionais: valor de imóveis
    (custo e líquido), lucro líquido.
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


def fetch_payout(ticker_ciks: dict[str, int]) -> list[dict]:
    """Payout médio dos últimos `PAYOUT_YEARS_AVERAGED` anos fiscais
    disponíveis (soma de dividendos pagos ÷ soma de lucro líquido, ambos
    somados ano a ano antes de dividir uma vez só — mesmo método de
    `cvm_dfp.fetch_payout`). Diferente da CVM, não baixa um arquivo por
    ano: a SEC já devolve o histórico completo de uma tag numa chamada só,
    isto é só uma janela sobre dado já obtido.

    Tag primária `PaymentsOfDividendsCommonStock` (confirmada ao vivo
    contra a Apple: dado real, 200, linha anual limpa depois do filtro de
    duração), fallback `PaymentsOfDividends` (mais abrangente, inclui
    dividendo de ação preferencial). Ticker sem nenhum ano com os dois
    valores (lucro e dividendo) é descartado; ano isolado sem dado só
    perde aquele ano."""
    if not ticker_ciks:
        return []

    results = []
    for ticker, cik in ticker_ciks.items():
        net_income_rows = _fetch_concept(cik, "NetIncomeLoss")
        if not net_income_rows:
            continue
        # Ordena por `filed` ascendente antes de indexar por `fy` — se
        # houver retificação (mesmo `fy`, `filed` mais novo), o dict
        # comprehension sobrescreve com o valor corrigido por último,
        # nunca o original.
        net_income_by_year = {
            r["fy"]: r["val"]
            for r in sorted(_annual_rows(net_income_rows), key=lambda r: r["filed"])
        }

        # Funde os anos das duas tags em vez de "primeira tag com
        # qualquer dado vence" (mesmo achado que corrigiu `_try_tags`
        # pra receita — uma tag pode ter dado só até um ano antigo e
        # ainda assim "vencer" injustamente). Ordem em `DIVIDEND_TAGS`
        # (primária primeiro) só decide o desempate quando as duas têm
        # o mesmo ano: percorre em ordem reversa pra a tag primária
        # sobrescrever por último.
        dividends_by_year: dict[int, float] = {}
        for tag in reversed(DIVIDEND_TAGS):
            rows = _fetch_concept(cik, tag)
            annual = _annual_rows(rows) if rows else []
            for r in sorted(annual, key=lambda r: r["filed"]):
                dividends_by_year[r["fy"]] = r["val"]

        years = sorted(
            set(net_income_by_year) & set(dividends_by_year), reverse=True
        )[:PAYOUT_YEARS_AVERAGED]
        if not years:
            continue

        total_income = sum(net_income_by_year[y] for y in years if net_income_by_year[y] > 0)
        total_dividends = sum(dividends_by_year[y] for y in years if net_income_by_year[y] > 0)
        if total_income <= 0:
            continue

        results.append({"ticker": ticker, "payout": total_dividends / total_income * 100})

    return results

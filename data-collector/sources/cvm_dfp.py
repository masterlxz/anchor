"""Cliente dos Dados Abertos da CVM (fundamentos pro DCF/FCFF).

Não é uma API REST como brapi/bolsai — a CVM publica **um zip por ano** com
as demonstrações financeiras de todas as ~870 companhias abertas juntas:
https://dados.cvm.gov.br/dados/CIA_ABERTA/DOC/DFP/DADOS/dfp_cia_aberta_{ano}.zip

Confirmado contra o arquivo real (2026-07-10): dentro do zip, vários CSVs
`;`-delimitados, encoding `latin1`, um por demonstração — `DRE_con` (resultado),
`BPA_con`/`BPP_con` (balanço, ativo/passivo), `DFC_MI_con` (fluxo de caixa,
método indireto) — cada linha é uma conta (`CD_CONTA`, código fixo e igual
pra qualquer empresa) de uma companhia (`CNPJ_CIA`/`CD_CVM`) num período
(`ORDEM_EXERC` = 'ÚLTIMO'/'PENÚLTIMO' — o próprio arquivo anual já traz os
dois últimos exercícios).

**Achado testando de verdade**: o `cvm_code` que a bolsai devolve (ex.:
"9512") vem sem zero à esquerda; o `CD_CVM` da CVM vem com zero à esquerda
("009512"). Comparar como texto não bate nunca — os dois precisam virar
`int` antes de comparar.

**Nem toda conta é tão padronizada quanto parece.** EBIT, dívida, caixa e as
peças do ΔNWC usam o mesmo `CD_CONTA` pra ~850 das ~870 empresas — seguro
ler direto. D&A e Capex não: o código varia empresa a empresa (a WEG rotula
a linha de Capex só como "Imobilizado"/"Intangível", sem a palavra
"Aquisição" que a maioria usa). Pra essas duas, a extração busca por
palavra-chave no texto da conta (`DS_CONTA`) em vez de um código fixo — e
se não achar exatamente um grupo de linhas confiável, devolve `None` em vez
de arriscar um número errado.

**Nº de ações não vem daqui.** O arquivo `composicao_capital` da CVM tinha
esse dado, mas testando contra a VALE3 real ele veio 1000x menor que o
número de ações de verdade (4,5 milhões em vez de ~4,5 bilhões — um erro
nos próprios dados da CVM pra essa companhia especificamente, não um bug de
leitura). A bolsai já devolve `shares_outstanding` correto e conferido no
`/fundamentals` (mesma chamada que já busca LPA/VPA/ROE/cvm_code) — reaproveitado
em `main.py` em vez de extrair (e arriscar) esse campo da CVM.

**Alíquota efetiva (`tax_rate`) também é melhor-esforço, mas por um motivo
diferente de D&A/Capex**: o código da conta é estável (`3.07`/`3.08` da
DRE, mesma tabela do EBIT), o problema é matemático — resultado antes dos
tributos perto de zero faz a razão explodir (achado testando a MGLU3 real:
resultado quase zero e negativo, "deu" 263%). Devolve `None` quando o
resultado antes dos tributos é zero/negativo ou a razão sai acima de um
teto de sanidade generoso (100%) — mas **não** limita alíquotas altas e
plausíveis (a VALE3 real deu 55,75%, legítimo pra uma mineradora com
tributação de subsidiárias no exterior, e passa normal).
"""

import csv
import io
import zipfile
from pathlib import Path

import requests

CVM_ZIP_URL_TEMPLATE = (
    "https://dados.cvm.gov.br/dados/CIA_ABERTA/DOC/DFP/DADOS/"
    "dfp_cia_aberta_{year}.zip"
)
CACHE_DIR = Path(__file__).parent.parent / ".cache" / "cvm_dfp"

LATEST = "ÚLTIMO"
PRIOR = "PENÚLTIMO"

# ESCALA_MOEDA -> multiplicador pra converter VL_CONTA em reais.
_CURRENCY_SCALE = {"MIL": 1_000, "MILHAO": 1_000_000, "UNIDADE": 1}


def _zip_path(year: int) -> Path:
    return CACHE_DIR / f"dfp_cia_aberta_{year}.zip"


def _download_zip(year: int) -> Path:
    path = _zip_path(year)
    if path.exists():
        return path

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    response = requests.get(CVM_ZIP_URL_TEMPLATE.format(year=year), timeout=60)
    response.raise_for_status()
    path.write_bytes(response.content)
    return path


def _resolve_zip_path() -> Path:
    """CVM nomeia o zip pelo ano do exercício coberto, não pelo ano de
    publicação — em julho/2026 o zip mais recente é o de 2025 (referente ao
    exercício fiscal encerrado em 2025-12-31, arquivado no início de 2026),
    não um zip "2026" (que só existiria a partir de ~março/2027). Tenta o
    ano anterior ao atual primeiro, cai pro ano anterior a esse em caso de
    404 (ex.: início de ano, antes da CVM publicar o zip mais recente).
    """
    from datetime import datetime, timezone

    candidate_year = datetime.now(timezone.utc).year - 1
    try:
        return _download_zip(candidate_year)
    except requests.HTTPError:
        return _download_zip(candidate_year - 1)


def _read_csv_from_zip(zf: zipfile.ZipFile, filename: str) -> list[dict]:
    with zf.open(filename) as raw:
        text = io.TextIOWrapper(raw, encoding="latin1")
        return list(csv.DictReader(text, delimiter=";"))


def _index_by_cvm_code(rows: list[dict]) -> dict[int, list[dict]]:
    index: dict[int, list[dict]] = {}
    for row in rows:
        index.setdefault(int(row["CD_CVM"]), []).append(row)
    return index


def _latest_version_rows(rows: list[dict]) -> list[dict]:
    """Uma companhia pode ter mais de uma `VERSAO` (retificação) pro mesmo
    exercício — mantém só as linhas da versão mais recente, senão contas
    duplicadas de uma retificação inflariam qualquer soma."""
    if not rows:
        return []
    max_version = max(int(row["VERSAO"]) for row in rows)
    return [row for row in rows if int(row["VERSAO"]) == max_version]


def _to_millions_brl(row: dict) -> float:
    scale = _CURRENCY_SCALE[row["ESCALA_MOEDA"]]
    return float(row["VL_CONTA"]) * scale / 1_000_000


def _find_exact(rows: list[dict], cd_conta: str, orden_exerc: str = LATEST) -> float:
    """Lê uma conta de código estável (EBIT, dívida, caixa, ...). Levanta
    `LookupError` se a conta não existir pra essa empresa/período — sinal
    de dado incomum (ex.: setor financeiro, taxonomia diferente), tratado
    pelo chamador como "pula essa empresa", não como zero."""
    candidates = _latest_version_rows(
        [r for r in rows if r["CD_CONTA"] == cd_conta and r["ORDEM_EXERC"] == orden_exerc]
    )
    if not candidates:
        raise LookupError(f"conta {cd_conta!r} ({orden_exerc}) não encontrada")
    return _to_millions_brl(candidates[0])


def _find_by_keyword(
    rows: list[dict],
    code_prefix: str,
    keywords: list[str],
    orden_exerc: str = LATEST,
) -> float | None:
    """Busca linhas por prefixo de código + palavra-chave no texto da conta
    (D&A, Capex — códigos não padronizados entre empresas). Descarta linhas
    "pai" (cujo código é prefixo do código de outra linha já casada, ex.:
    total `6.02` quando `6.02.02`/`6.02.03` também casaram) pra não somar
    subtotal + detalhe. Devolve `None` se sobrar zero linhas — nunca chuta."""
    matched = _latest_version_rows(
        [
            r
            for r in rows
            if r["CD_CONTA"].startswith(code_prefix)
            and r["ORDEM_EXERC"] == orden_exerc
            and any(keyword in r["DS_CONTA"].lower() for keyword in keywords)
        ]
    )
    if not matched:
        return None

    codes = {r["CD_CONTA"] for r in matched}
    leaves = [
        r
        for r in matched
        if not any(other != r["CD_CONTA"] and other.startswith(r["CD_CONTA"] + ".") for other in codes)
    ]
    return abs(sum(_to_millions_brl(r) for r in leaves))


# Teto de sanidade pra alíquota efetiva. Testando contra empresas reais:
# a VALE3 deu 55,75% (alíquota alta, mas real — mineradoras pagam CFEM e
# retenção na fonte de subsidiárias no exterior, não é bug); já a MGLU3 deu
# 263% só porque o resultado pré-tributos tava perto de zero (instabilidade
# de divisão, não alíquota de verdade — mas esse caso já é pego pelo guard
# de `pretax_income <= 0` abaixo). O teto aqui é só uma rede de segurança
# extra pra positivos-mas-perto-de-zero que passariam por aquele guard.
_MAX_PLAUSIBLE_TAX_RATE = 100.0


def _effective_tax_rate(dre_rows: list[dict]) -> float | None:
    """Alíquota efetiva = IR e CSLL ÷ resultado antes dos tributos, ambos da
    DRE (`3.08`/`3.07` — mesma tabela do EBIT, ~97% estáveis entre empresas).
    Devolvido em % (16.83, não 0.1683), mesma convenção já usada nesta
    tabela de referência pro ROE.

    Devolve `None` quando o resultado antes dos tributos é zero/negativo ou
    quando a razão sai fora de uma faixa plausível — nesses casos o número
    é matematicamente instável (divisão por algo perto de zero), não uma
    alíquota de verdade."""
    pretax_income = _find_exact(dre_rows, "3.07")
    tax_expense = _find_exact(dre_rows, "3.08")

    if pretax_income <= 0:
        return None

    tax_rate = -tax_expense / pretax_income * 100
    if not (0.0 <= tax_rate <= _MAX_PLAUSIBLE_TAX_RATE):
        return None

    return tax_rate


def _nwc_change(bpa_rows: list[dict], bpp_rows: list[dict]) -> float:
    """ΔNWC = (Contas a Receber + Estoques − Fornecedores) no exercício
    atual menos o mesmo cálculo no exercício anterior — usa só os 3 códigos
    estáveis do balanço, evitando os códigos instáveis do fluxo de caixa
    (mesmo problema de instabilidade do Capex)."""

    def nwc_at(orden_exerc: str) -> float:
        receivables = _find_exact(bpa_rows, "1.01.03", orden_exerc)
        inventory = _find_exact(bpa_rows, "1.01.04", orden_exerc)
        payables = _find_exact(bpp_rows, "2.01.02", orden_exerc)
        return receivables + inventory - payables

    return nwc_at(LATEST) - nwc_at(PRIOR)


def fetch_roe(ticker_cvm_codes: dict[str, str]) -> list[dict]:
    """Calcula o ROE (Lucro Líquido Consolidado do Período ÷ Patrimônio
    Líquido Consolidado, ambos do exercício fiscal mais recente) direto da
    CVM, em vez de confiar no campo `roe` da bolsai.

    **Achado (Sessão 16)**: o `roe` da bolsai mistura lucro trimestral não
    anualizado com lucro TTM dependendo da empresa, sem indicar qual é qual
    — testando contra o BPAC11 real, ela devolveu ROE de 3,54% (só o
    trimestre mais recente ÷ patrimônio) quando o real reportado é 26,6%.

    O código da conta pra essas duas linhas **não é padronizado entre banco
    e empresa normal** (ex.: patrimônio líquido é a conta "2.03" pra VALE3
    mas "2.08" pra bancos, que usam uma taxonomia contábil diferente,
    COSIF) — por isso usa `_find_by_keyword` (busca por texto), não
    `_find_exact`. O texto em si já é estável nos dois casos: confirmado
    contra VALE3/ITUB4/BBAS3/BPAC11 reais que tanto "Lucro/Prejuízo
    Consolidado do Período" quanto a variante "Lucro ou Prejuízo Líquido
    Consolidado do Período" (BBAS3) contêm a frase "consolidado do
    período", e que "Patrimônio Líquido Consolidado" é idêntico nos 4.

    Retorna uma lista de {"ticker": str, "roe": float} — em **percentual**
    (23.0, não 0.23), mesma convenção já usada em `stock_fundamentals.roe`
    (ver nota em `_effective_tax_rate`). Ticker sem as duas contas
    encontráveis é ignorado — pula em vez de arriscar um número errado.

    Achado de performance (Sessão 46, 2026-08-02): `ticker_cvm_codes` vem
    vazio sempre que a bolsai não achou fundamentos pro ticker (garantido
    pra qualquer FII, já que ela só tem `/fundamentals` pra ação) — sem o
    guard abaixo, esta função baixava/abria/processava o zip inteiro da CVM
    (~870 empresas) só pra devolver `[]` de qualquer jeito, ~15s
    desperdiçados por busca de FII na tela de Pesquisa.
    """
    if not ticker_cvm_codes:
        return []

    zip_path = _resolve_zip_path()

    with zipfile.ZipFile(zip_path) as zf:
        year = int(zip_path.stem.rsplit("_", 1)[-1])
        dre_rows = _read_csv_from_zip(zf, f"dfp_cia_aberta_DRE_con_{year}.csv")
        bpp_rows = _read_csv_from_zip(zf, f"dfp_cia_aberta_BPP_con_{year}.csv")

    dre_by_cvm_code = _index_by_cvm_code(dre_rows)
    bpp_by_cvm_code = _index_by_cvm_code(bpp_rows)
    results = []

    for ticker, cvm_code in ticker_cvm_codes.items():
        try:
            net_income = _find_by_keyword(
                dre_by_cvm_code[int(cvm_code)], "3.", ["consolidado do período"]
            )
            equity = _find_by_keyword(
                bpp_by_cvm_code[int(cvm_code)], "2.", ["patrimônio líquido consolidado"]
            )
        except KeyError:
            continue

        if net_income is None or equity is None or equity <= 0:
            continue

        results.append({"ticker": ticker, "roe": net_income / equity * 100})

    return results


_PATRIMONIO_LIQUIDO_CONSOLIDADO_COLUMN = "Patrimônio Líquido Consolidado"
_EMPTY_COLUMN = ""
PAYOUT_YEARS_AVERAGED = 5


def _resolve_recent_years(count: int) -> list[int]:
    """Últimos `count` anos fiscais com zip publicado na CVM, mais recente
    primeiro. Reaproveita `_resolve_zip_path` só pra descobrir o ano mais
    recente (já resolve o caso "zip nomeado pelo ano do exercício, não o de
    publicação") — daqui pra trás não precisa checar 404, a CVM tem zip
    anual desde que existe."""
    zip_path = _resolve_zip_path()
    latest_year = int(zip_path.stem.rsplit("_", 1)[-1])
    return list(range(latest_year, latest_year - count, -1))


def _extract_distributions(dmpl_rows: list[dict]) -> float | None:
    """Dividendos + Juros sobre Capital Próprio (JCP — vantagem tributária,
    mais comum em bancos), buscados por palavra-chave (mesmo padrão do
    lucro em `fetch_roe`) na coluna "Patrimônio Líquido Consolidado" do
    DMPL (o total — o arquivo repete cada linha uma vez por componente do
    patrimônio, sem esse filtro a soma ficaria multiplicada).

    **Achado (Sessão 16, teste real do BBDC4)**: pra algumas empresas essa
    coluna vem zerada por inconsistência da própria CVM, mesmo com a
    distribuição real presente nos dados — nesse caso o valor verdadeiro
    aparece na coluna sem rótulo (`COLUNA_DF == ""`). Só cai nesse fallback
    quando a coluna principal soma exatamente `0.0` (não `None` — `None` é
    "nenhuma linha bateu", `0.0` é "bateu mas somou zero", só o segundo é
    suspeito); não troca a coluna padrão porque pelo menos uma empresa
    testada (BBAS3) tem as duas preenchidas com valores diferentes
    (provavelmente bruto vs líquido do efeito fiscal do JCP), e a coluna
    "Patrimônio Líquido Consolidado" está correta pra ela.
    """
    consolidado_rows = [
        r for r in dmpl_rows if r.get("COLUNA_DF") == _PATRIMONIO_LIQUIDO_CONSOLIDADO_COLUMN
    ]
    distributions = _find_by_keyword(
        consolidado_rows, "5.", ["dividendo", "juros sobre capital"]
    )
    if distributions != 0.0:
        return distributions

    empty_rows = [r for r in dmpl_rows if r.get("COLUNA_DF") == _EMPTY_COLUMN]
    return _find_by_keyword(empty_rows, "5.", ["dividendo", "juros sobre capital"])


def fetch_payout(ticker_cvm_codes: dict[str, str]) -> list[dict]:
    """Calcula o payout médio dos últimos `PAYOUT_YEARS_AVERAGED` anos
    fiscais (soma de Dividendos+JCP ÷ soma de Lucro Líquido Consolidado,
    ambos somados ano a ano antes de dividir uma vez só no final — não
    média simples dos payouts anuais) direto da CVM.

    **Por que média em vez de um ano só (achado do usuário, Sessão 16)**:
    um ano isolado pode distorcer a realidade — testando a VALE3 real, um
    ano de lucro baixo com uma distribuição grande vinda de reservas
    acumuladas deu payout de 275%. Somar 5 anos antes de dividir (em vez de
    tirar a média de 5 percentuais) evita que um ano de lucro anormalmente
    baixo puxe o resultado com o mesmo peso dos outros anos.

    Ticker sem dado num ano específico (ex.: IPO recente) só perde aquele
    ano — os outros anos já acumulados continuam valendo, mesmo espírito
    de `acoes_yahoo.fetch_dividends_avg`. Ticker sem nenhum ano com dado é
    descartado inteiro. Retorna uma lista de {"ticker": str, "payout":
    float} em percentual (28.5, não 0.285), mesma convenção de `fetch_roe`.

    Mesmo guard de performance de `fetch_roe` (Sessão 46) — aqui o
    desperdício seria ainda maior sem ele, já que este loop baixa/processa
    até `PAYOUT_YEARS_AVERAGED` zips (vários anos), não só 1.
    """
    if not ticker_cvm_codes:
        return []

    totals = {
        ticker: {"net_income": 0.0, "distributions": 0.0, "years_found": 0}
        for ticker in ticker_cvm_codes
    }

    for year in _resolve_recent_years(PAYOUT_YEARS_AVERAGED):
        zip_path = _download_zip(year)
        with zipfile.ZipFile(zip_path) as zf:
            dre_rows = _read_csv_from_zip(zf, f"dfp_cia_aberta_DRE_con_{year}.csv")
            dmpl_rows = _read_csv_from_zip(zf, f"dfp_cia_aberta_DMPL_con_{year}.csv")

        dre_by_cvm_code = _index_by_cvm_code(dre_rows)
        dmpl_by_cvm_code = _index_by_cvm_code(dmpl_rows)

        for ticker, cvm_code in ticker_cvm_codes.items():
            try:
                net_income = _find_by_keyword(
                    dre_by_cvm_code[int(cvm_code)], "3.", ["consolidado do período"]
                )
                distributions = _extract_distributions(dmpl_by_cvm_code[int(cvm_code)])
            except KeyError:
                continue

            if net_income is None or net_income <= 0 or distributions is None:
                continue

            entry = totals[ticker]
            entry["net_income"] += net_income
            entry["distributions"] += distributions
            entry["years_found"] += 1

    return [
        {"ticker": ticker, "payout": entry["distributions"] / entry["net_income"] * 100}
        for ticker, entry in totals.items()
        if entry["years_found"] > 0
    ]


def fetch_dcf_fundamentals(ticker_cvm_codes: dict[str, str]) -> list[dict]:
    """Busca os 9 campos do DCF/Stock Lookup/RNAV derivados de dados
    contábeis da CVM (EBIT, alíquota efetiva, D&A, Capex, ΔNWC, dívida
    total, caixa, receita líquida, estoque) pra cada ticker.
    `shares_outstanding` não vem daqui — ver nota no topo do arquivo.

    `ticker_cvm_codes` é {ticker: cvm_code} — já resolvido via
    `acoes_bolsai.fetch_fundamentals` (mesma chamada que já busca LPA/VPA/ROE,
    sem chamada extra só pra achar a empresa). Retorna uma lista de dicts
    com `ticker`, `reference_year`, `ebit`, `tax_rate`,
    `depreciation_amortization` (pode ser `None`), `capex` (pode ser
    `None`), `nwc_change`, `total_debt`, `cash`, `revenue`, `inventory`. Um
    ticker sem dado encontrável (ex.: banco — taxonomia de DRE diferente,
    ver domain/dcf.rs) é ignorado, não derruba o restante.

    `revenue` (Sessão 20, pra margem líquida do Stock Lookup) usa a conta
    "3.01" da DRE ("Receita de Venda de Bens e/ou Serviços" na maioria das
    empresas, rótulo muda pra banco/seguradora mas o código é igualmente
    estável) — confirmado contra o zip real: mesma cobertura exata da conta
    "3.05" do EBIT (436 de 436 empresas com EBIT também têm receita).

    `inventory` (Sessão 29, pra auto-fill do RNAV) usa a conta "1.01.04" do
    BPA — mesmo código já lido dentro de `_nwc_change` pra esse mesmo
    ticker (Estoques, Ativo Circulante), só que aqui devolvido isolado em
    vez de só entrar na conta do ΔNWC. Não é uma leitura nova de risco: se
    essa conta não existisse pra um ticker, `_nwc_change` já teria
    derrubado o ticker inteiro antes de chegar aqui.

    Mesmo guard de performance de `fetch_roe`/`fetch_payout` (Sessão 46) —
    hoje o chamador (`main.py`) só invoca esta função quando `fundamentals`
    não está vazio, então na prática `ticker_cvm_codes` nunca chega vazio
    aqui, mas o guard é barato e evita reintroduzir o mesmo desperdício se
    o chamador mudar no futuro.
    """
    if not ticker_cvm_codes:
        return []

    zip_path = _resolve_zip_path()
    results = []

    with zipfile.ZipFile(zip_path) as zf:
        year = int(zip_path.stem.rsplit("_", 1)[-1])
        dre_rows = _read_csv_from_zip(zf, f"dfp_cia_aberta_DRE_con_{year}.csv")
        bpa_rows = _read_csv_from_zip(zf, f"dfp_cia_aberta_BPA_con_{year}.csv")
        bpp_rows = _read_csv_from_zip(zf, f"dfp_cia_aberta_BPP_con_{year}.csv")
        dfc_rows = _read_csv_from_zip(zf, f"dfp_cia_aberta_DFC_MI_con_{year}.csv")

    dre_by_cvm_code = _index_by_cvm_code(dre_rows)
    bpa_by_cvm_code = _index_by_cvm_code(bpa_rows)
    bpp_by_cvm_code = _index_by_cvm_code(bpp_rows)
    dfc_by_cvm_code = _index_by_cvm_code(dfc_rows)

    for ticker, cvm_code in ticker_cvm_codes.items():
        try:
            company_dre = dre_by_cvm_code[int(cvm_code)]
            company_bpa = bpa_by_cvm_code[int(cvm_code)]
            company_bpp = bpp_by_cvm_code[int(cvm_code)]
            company_dfc = dfc_by_cvm_code[int(cvm_code)]

            results.append(
                {
                    "ticker": ticker,
                    "reference_year": year,
                    "ebit": _find_exact(company_dre, "3.05"),
                    "tax_rate": _effective_tax_rate(company_dre),
                    "depreciation_amortization": _find_by_keyword(
                        company_dfc,
                        "6.01.01",
                        ["depreciaç", "amortiza", "exaust"],
                    ),
                    "capex": _find_by_keyword(
                        company_dfc, "6.02", ["imobilizado", "intangív", "intangiv"]
                    ),
                    "nwc_change": _nwc_change(company_bpa, company_bpp),
                    "total_debt": (
                        _find_exact(company_bpp, "2.01.04")
                        + _find_exact(company_bpp, "2.02.01")
                    ),
                    "cash": _find_exact(company_bpa, "1.01.01"),
                    "revenue": _find_exact(company_dre, "3.01"),
                    "inventory": _find_exact(company_bpa, "1.01.04"),
                }
            )
        except (KeyError, LookupError, IndexError):
            continue

    return results

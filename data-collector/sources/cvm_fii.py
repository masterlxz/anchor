"""Cliente dos Dados Abertos da CVM, fatia FII (Fase 10, item 8 — Sessão 41).

Mesmo portal que `cvm_dfp.py` já usa pras ações (`dados.cvm.gov.br`), mas um
conjunto de arquivos totalmente separado, com schema próprio (nomes de campo
`Data_Referencia`/`Versao`, não `DT_REFER`/`VERSAO` da DFP) e uma convenção
de nome de arquivo diferente: o zip é nomeado pelo **ano corrente** dos
dados (`inf_mensal_fii_2026.zip` já contém os meses de 2026 publicados até
agora), não pelo ano do exercício fiscal encerrado como a DFP.

Três informes usados aqui, confirmados contra os arquivos reais
(2026-08-02):
- `INF_MENSAL` — patrimônio líquido, valor patrimonial da cota, dividend
  yield do mês, nº de cotistas (`inf_mensal_fii_geral_{ano}.csv` +
  `..._complemento_{ano}.csv`, mesmo `CNPJ_Fundo_Classe`/`Data_Referencia`).
  Também é a única fonte usada pra resolver ticker→CNPJ (via
  `resolve_cnpj`), já que só o `geral` tem `Nome_Fundo_Classe`.
- `INF_TRIMESTRAL` — só o arquivo `imovel` é usado por ora: **vacância e
  inadimplência por imóvel**, exatamente o que o dono do projeto pediu
  (`Percentual_Vacancia`/`Percentual_Inadimplencia`), mais endereço/área/%
  locado/% da receita do fundo. Os outros 15 CSVs do informe trimestral
  (inquilinos por contrato, resultado contábil-financeiro completo,
  aquisição/alienação de imóvel...) ficam pra uma fatia futura se o dono do
  projeto quiser mais detalhe.

**Preço de mercado e proventos não vêm daqui** — a CVM é reguladora, não a
bolsa; continuam vindo do Yahoo (`acoes_yahoo.py`, já estende pra FII desde
a fatia anterior desta mesma sessão).
"""

import csv
import io
import re
import zipfile
from pathlib import Path

import requests

from . import acoes_bolsai

CVM_FII_BASE_URL = "https://dados.cvm.gov.br/dados/FII/DOC"
CACHE_DIR = Path(__file__).parent.parent / ".cache" / "cvm_fii"


def _cnpj_digits(cnpj: str) -> str:
    return re.sub(r"\D", "", cnpj)


def _normalize_name(name: str) -> str:
    return re.sub(r"\s+", " ", name).strip().upper()


def _zip_path(kind: str, year: int) -> Path:
    return CACHE_DIR / f"inf_{kind}_fii_{year}.zip"


def _download_zip(kind: str, year: int) -> Path:
    path = _zip_path(kind, year)
    if path.exists():
        return path

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    url = f"{CVM_FII_BASE_URL}/INF_{kind.upper()}/DADOS/inf_{kind}_fii_{year}.zip"
    response = requests.get(url, timeout=60)
    response.raise_for_status()
    path.write_bytes(response.content)
    return path


def _resolve_zip(kind: str) -> zipfile.ZipFile:
    """Diferente da DFP (`cvm_dfp.py`), o zip da FII é nomeado pelo ano
    corrente dos dados publicados, não pelo ano do exercício encerrado —
    tenta o ano atual primeiro (já tem meses/trimestres publicados de 2026
    confirmado ao vivo), cai pro ano anterior só se ainda não existir
    (ex.: primeiros dias de janeiro, antes da CVM abrir o zip do ano novo)."""
    from datetime import datetime, timezone

    current_year = datetime.now(timezone.utc).year
    try:
        path = _download_zip(kind, current_year)
    except requests.HTTPError:
        path = _download_zip(kind, current_year - 1)
    return zipfile.ZipFile(path)


def _read_csv(zf: zipfile.ZipFile, filename: str) -> list[dict]:
    with zf.open(filename) as raw:
        text = io.TextIOWrapper(raw, encoding="latin1")
        return list(csv.DictReader(text, delimiter=";"))


def _latest_version_rows(rows: list[dict]) -> list[dict]:
    """Mesma disciplina de `cvm_dfp.py::_latest_version_rows` — uma
    retificação (`Versao` maior) pro mesmo `Data_Referencia` substitui a
    versão anterior, nunca soma/duplica."""
    if not rows:
        return []
    max_version = max(int(row["Versao"]) for row in rows)
    return [row for row in rows if int(row["Versao"]) == max_version]


def _latest_reference_rows(rows: list[dict]) -> list[dict]:
    """Mantém só as linhas do `Data_Referencia` mais recente (já filtradas
    pela versão mais nova daquele período) — usado tanto pro indicador
    mensal (1 linha por fundo) quanto pro imóvel trimestral (N linhas por
    fundo, um por imóvel, todas do mesmo trimestre)."""
    if not rows:
        return []
    latest_date = max(row["Data_Referencia"] for row in rows)
    return _latest_version_rows([r for r in rows if r["Data_Referencia"] == latest_date])


def resolve_cnpj(ticker: str) -> dict | None:
    """Resolve o CNPJ do fundo (não do administrador) a partir do ticker,
    combinando a bolsai (nome oficial do fundo + CNPJ do administrador) com
    o cadastro público `INF_MENSAL/.../geral` da CVM (que tem
    `Nome_Fundo_Classe` mas não o ticker).

    Match exigido: `CNPJ_Administrador` batendo (reduz o universo — um
    administrador comum gerencia dezenas de fundos, confirmado testando
    contra o Banco Genial real) **e** `Nome_Fundo_Classe` batendo exato
    (normalizado só por espaço/caixa, sem tirar acento — confirmado que o
    nome da bolsai bate caractere a caractere com o da CVM pro HGLG11 real).
    Zero ou mais de um match → `None`, nunca chuta (mesma disciplina de
    `cvm_dfp.py::_find_exact`) — o dono do projeto cola o CNPJ manualmente
    nesse caso.
    """
    summary = acoes_bolsai.fetch_fii_summary(ticker)
    if summary is None:
        return None

    admin_cnpj_digits = _cnpj_digits(summary["administrator_cnpj"])
    target_name = _normalize_name(summary["name"])

    zf = _resolve_zip("mensal")
    rows = _read_csv(zf, next(n for n in zf.namelist() if n.startswith("inf_mensal_fii_geral_")))

    candidates = {
        row["CNPJ_Fundo_Classe"]
        for row in rows
        if _cnpj_digits(row["CNPJ_Administrador"]) == admin_cnpj_digits
        and _normalize_name(row["Nome_Fundo_Classe"]) == target_name
    }

    if len(candidates) != 1:
        return None

    cnpj = next(iter(candidates))
    return {"cnpj": cnpj, "fund_name": summary["name"]}


def fetch_monthly_indicators(cnpjs: list[str]) -> list[dict]:
    """Indicadores do informe mensal mais recente disponível, um por fundo.

    Retorna `{"cnpj", "reference_date", "patrimonio_liquido",
    "valor_patrimonial_cota", "numero_cotistas", "dividend_yield_mes",
    "rentabilidade_efetiva_mes"}`. Um CNPJ sem nenhuma linha no ano corrente
    é ignorado (não derruba o resto) — mesmo padrão de `acoes_yahoo.py`.
    """
    if not cnpjs:
        return []

    zf = _resolve_zip("mensal")
    rows = _read_csv(
        zf, next(n for n in zf.namelist() if n.startswith("inf_mensal_fii_complemento_"))
    )

    cnpj_set = set(cnpjs)
    by_cnpj: dict[str, list[dict]] = {}
    for row in rows:
        if row["CNPJ_Fundo_Classe"] in cnpj_set:
            by_cnpj.setdefault(row["CNPJ_Fundo_Classe"], []).append(row)

    results = []
    for cnpj, fund_rows in by_cnpj.items():
        latest = _latest_reference_rows(fund_rows)
        if not latest:
            continue
        row = latest[0]
        results.append(
            {
                "cnpj": cnpj,
                "reference_date": row["Data_Referencia"],
                "patrimonio_liquido": float(row["Patrimonio_Liquido"]),
                "valor_patrimonial_cota": float(row["Valor_Patrimonial_Cotas"]),
                "numero_cotistas": (
                    int(row["Total_Numero_Cotistas"]) if row["Total_Numero_Cotistas"] else None
                ),
                "dividend_yield_mes": (
                    float(row["Percentual_Dividend_Yield_Mes"])
                    if row["Percentual_Dividend_Yield_Mes"]
                    else None
                ),
                "rentabilidade_efetiva_mes": (
                    float(row["Percentual_Rentabilidade_Efetiva_Mes"])
                    if row["Percentual_Rentabilidade_Efetiva_Mes"]
                    else None
                ),
            }
        )

    return results


def fetch_property_data(cnpjs: list[str]) -> list[dict]:
    """Imóveis do informe trimestral mais recente disponível, um item por
    imóvel (um fundo pode ter vários).

    Retorna `{"cnpj", "reference_date", "nome_imovel", "endereco",
    "area_m2", "percentual_vacancia", "percentual_inadimplencia",
    "percentual_receitas_fii", "percentual_locado"}`. Campos percentuais já
    vêm em fração (0-1) na CVM, mesma convenção do indicador mensal — o
    lado Rust/frontend decide a formatação em %.
    """
    if not cnpjs:
        return []

    zf = _resolve_zip("trimestral")
    rows = _read_csv(
        zf, next(n for n in zf.namelist() if n.startswith("inf_trimestral_fii_imovel_"))
    )

    cnpj_set = set(cnpjs)
    by_cnpj: dict[str, list[dict]] = {}
    for row in rows:
        if row["CNPJ_Fundo_Classe"] in cnpj_set:
            by_cnpj.setdefault(row["CNPJ_Fundo_Classe"], []).append(row)

    results = []
    for cnpj, fund_rows in by_cnpj.items():
        for row in _latest_reference_rows(fund_rows):
            results.append(
                {
                    "cnpj": cnpj,
                    "reference_date": row["Data_Referencia"],
                    "nome_imovel": row["Nome_Imovel"],
                    "endereco": row["Endereco"] or None,
                    "area_m2": float(row["Area"]) if row["Area"] else None,
                    "percentual_vacancia": (
                        float(row["Percentual_Vacancia"]) if row["Percentual_Vacancia"] else None
                    ),
                    "percentual_inadimplencia": (
                        float(row["Percentual_Inadimplencia"])
                        if row["Percentual_Inadimplencia"]
                        else None
                    ),
                    "percentual_receitas_fii": (
                        float(row["Percentual_Receitas_FII"])
                        if row["Percentual_Receitas_FII"]
                        else None
                    ),
                    "percentual_locado": (
                        float(row["Percentual_Locado"]) if row["Percentual_Locado"] else None
                    ),
                }
            )

    return results

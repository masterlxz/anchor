"""Cliente dos Dados Abertos da CVM, fatia FII — resto local após a Fase 1.7.

Mesmo portal que `cvm_dfp.py` (removido na Sessão 7) usava pras ações
(`dados.cvm.gov.br`), conjunto de arquivos `INF_MENSAL` com schema próprio
(`Data_Referencia`/`Versao`, `CNPJ_Fundo_Classe`), zip nomeado pelo **ano
corrente** dos dados publicados (não pelo exercício fiscal encerrado).

**Fase 1.7 (Sessão 7)**: `fetch_monthly_indicators`/`fetch_property_data`
foram removidos — a Finance API (`finance_api_client.fetch_fii_monthly_indicators`/
`fetch_fii_properties`, `GET /v1/fiis/{cnpj}/{monthly-indicators,properties}`)
cobre os dois agora. O que sobra aqui é só `resolve_cnpj` (ticker→CNPJ),
nunca portado pra Finance API (decisão da Sessão 4) — usa o arquivo `geral` do
informe mensal (único com `Nome_Fundo_Classe`) combinado com
`acoes_bolsai.fetch_fii_summary`.
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

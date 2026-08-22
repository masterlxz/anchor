"""Cliente da API interna de estatísticas de índices da B3.

Endpoint confirmado direto ao vivo (Fase 13.5, Sessão 83) — público, sem
chave, sem autenticação, cert válido:
GET https://sistemaswebb3-listados.b3.com.br/indexStatisticsProxy/IndexCall/GetPortfolioDay/{base64}
onde `{base64}` é o base64 de `{"language": "en-us", "index": "<CÓDIGO>",
"year": <ANO>}` (achado via o pacote R `rb3`/rOpenSci, que documenta como
consome dados da B3 — diferente da rota `indexPage`/`indexProxy` tentada sem
sucesso na Sessão 81.2, que só expõe composição de carteira, não série
histórica). Devolve, por ano, um grid fixo de 31 linhas (`day` 1..31) × 12
colunas `rateValue1`..`rateValue12` (jan..dez); `None`/string vazia em dia
sem pregão ou que não existe no mês (ex. dia 31 de abril). Valor vem como
`"3,314.09"` (separador de milhar vírgula, decimal ponto — por causa do
`language=en-us`, evita o parse de decimal-vírgula brasileiro).
"""

import base64
import json
from datetime import date

import requests

B3_INDEX_STATS_URL = (
    "https://sistemaswebb3-listados.b3.com.br/indexStatisticsProxy/IndexCall/GetPortfolioDay"
)


def fetch_index_history(index_code: str, start_year: int, end_year: int) -> list[dict]:
    """Busca o histórico diário de pontos de um índice B3 (IFIX/SMLL/IDIV)
    entre `start_year` e `end_year` (inclusive), um request por ano.

    Retorna `[{"price_date": "YYYY-MM-DD", "close_price": 3314.09}, ...]`.
    Anos anteriores à data-base do índice devolvem grid vazio (tudo `None`) —
    inofensivo, só um request desperdiçado, não é erro.
    """
    results = []
    for year in range(start_year, end_year + 1):
        params = json.dumps({"language": "en-us", "index": index_code, "year": year})
        b64 = base64.b64encode(params.encode()).decode()
        response = requests.get(
            f"{B3_INDEX_STATS_URL}/{b64}",
            headers={"User-Agent": "Mozilla/5.0"},
            timeout=15,
        )
        response.raise_for_status()

        for row in response.json().get("results", []):
            day = row["day"]
            for month in range(1, 13):
                raw = row.get(f"rateValue{month}")
                if not raw:
                    continue
                try:
                    price_date = date(year, month, day)
                except ValueError:
                    continue
                results.append(
                    {
                        "price_date": price_date.isoformat(),
                        "close_price": float(raw.replace(",", "")),
                    }
                )
    return results

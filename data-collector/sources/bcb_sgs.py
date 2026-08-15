"""Cliente da API SGS (Sistema Gerenciador de Séries Temporais) do Banco
Central do Brasil.

Endpoint confirmado direto contra a API real (Fase 13.5, Sessão 81) —
público, sem chave, sem cadastro:
GET https://api.bcb.gov.br/dados/serie/bcdata.sgs.{codigo}/dados?formato=json
devolve `[{"data": "dd/mm/yyyy", "valor": "0.47"}, ...]`. Séries mensais
(CDI acumulado no mês = 4391, IPCA variação mensal = 433) sempre trazem
`data` no dia 01 do mês, e `valor` já é o percentual mensal pronto — sem
nenhuma conta a fazer, diferente do CDI diário (série 12), que não é usada
aqui.
"""

import requests

BCB_SGS_URL = "https://api.bcb.gov.br/dados/serie/bcdata.sgs.{code}/dados"


def fetch_monthly_series(series_code: int) -> list[dict]:
    """Busca o histórico completo de uma série mensal do SGS (não
    `ultimos/N` — a carteira do usuário pode ter começado em qualquer mês
    passado, então o backfill precisa da série inteira, não só recente).

    Retorna `[{"year_month": "YYYY-MM", "value_pct": 0.47}, ...]`. Erro de
    rede/parse propaga (é uma série só por chamada, diferente de
    `acoes_yahoo`, que itera vários tickers e pula individualmente).
    """
    response = requests.get(
        BCB_SGS_URL.format(code=series_code),
        params={"formato": "json"},
        headers={"User-Agent": "Mozilla/5.0"},
        timeout=15,
    )
    response.raise_for_status()

    results = []
    for item in response.json():
        day, month, year = item["data"].split("/")
        results.append(
            {
                "year_month": f"{year}-{month}",
                "value_pct": float(item["valor"]),
            }
        )
    return results

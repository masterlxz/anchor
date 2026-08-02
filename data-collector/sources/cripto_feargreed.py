"""Cliente da API do alternative.me (Crypto Fear & Greed Index).

Endpoint confirmado direto contra a API real (2026-08-02) — público, sem
chave: GET /fng/?limit=1 devolve `data: [{value, value_classification,
timestamp, time_until_update}]`, sempre o valor mais recente quando
`limit=1`. Índice **global de mercado**, não por moeda — sem parâmetro de
coin, diferente de tudo em `cripto_coingecko.py`.
"""

from datetime import datetime, timezone

import requests

FEAR_GREED_URL = "https://api.alternative.me/fng/"


def fetch_latest() -> dict:
    """Retorna {"value": int, "classification": str, "reading_date": "YYYY-MM-DD"}."""
    response = requests.get(FEAR_GREED_URL, params={"limit": 1}, timeout=15)
    response.raise_for_status()
    entry = response.json()["data"][0]

    reading_date = (
        datetime.fromtimestamp(int(entry["timestamp"]), tz=timezone.utc).date().isoformat()
    )

    return {
        "value": int(entry["value"]),
        "classification": entry["value_classification"],
        "reading_date": reading_date,
    }

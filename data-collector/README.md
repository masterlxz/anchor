# data-collector

Coletores de dados em Python (ações BR + cripto) que escrevem direto no banco SQLite
compartilhado com o app (`../desktop`). Ver `../project/PHASE.md` — Fase 2
(desenho completo das fontes está lá, seção "Fase 2 — Coleta de Dados").

**Fase 1.7 (Sessão 7)**: a maior parte da coleta (ação BR, FII, cripto, metais, índices B3,
CDI/IPCA) agora depende da Finance API do EasyBusiness rodando (`docker compose up` no repo
`easybusiness`, porta 8000 por padrão) — sem ela, esses caminhos falham alto (propagam erro,
sem fallback silencioso pra fonte local antiga). O que **não** depende dela, continua 100%
local: cotação/técnicos/dividendos/histórico de preço de ticker sem sufixo `.SA` (ação US, ETF
US, REIT — `--us-ticker`/`--reit-ticker`/`--etf-us-ticker`), indicadores imobiliários de REIT,
IBOV no benchmark, e `--fii-resolve-cnpj`.

## Setup

```
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
cp .env.example .env  # preencher BOLSAI_API_KEY, SEC_EDGAR_CONTACT_EMAIL, FINANCE_API_KEY
```

`FINANCE_API_KEY` deve ser a mesma chave configurada em `easybusiness/api/.env` (`API_KEYS`).
`FINANCE_API_BASE_URL` tem default `http://localhost:8000`, só precisa mudar se a Finance API
rodar em outro host/porta.

## Rodar manualmente (sem passar pelo app)

```
.venv/bin/python3 main.py
```

Disparado pelo app via o comando Tauri `run_stock_collector`
(`../desktop/src-tauri/src/commands/collector.rs`), que roda esse mesmo
`main.py` como subprocess.

## Implementado

Ver `../project/PHASE.md` (seção "Fase 2 — Coleta de Dados")
para o estado atual e `../project/SESSIONS.md` para o log de sessões — este
arquivo não é atualizado a cada mudança de fonte de dado.

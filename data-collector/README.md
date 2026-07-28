# data-collector

Coletores de dados em Python (ações BR + cripto) que escrevem direto no banco SQLite
compartilhado com o app (`../desktop`). Ver `../project/PHASE.md` — Fase 2
(desenho completo das fontes está lá, seção "Fase 2 — Coleta de Dados").

## Setup

```
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
cp .env.example .env  # preencher BOLSAI_API_KEY
```

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

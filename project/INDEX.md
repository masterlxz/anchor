# Practice Valuation — Estado do Projeto

> Este projeto usa a mesma estrutura de arquivos do TruthID (outro projeto do mesmo dono): o estado
> foi dividido em arquivos menores dentro desta pasta (`project/`), em vez de um único
> `PROJECT_STATE.md` monolítico na raiz.
> Última atualização: 2026-08-02 (Sessão 47 — Fase 6.1, checklist de segurança final rodado item por item contra o repo real e todo o histórico do git: segredos/credenciais confirmados limpos (nenhum achado real, `.gitignore`/`.env.example`/keyring todos corretos). 2 achados de hardening/consistência registrados como pendência consciente no Roadmap por decisão do dono do projeto (não corrigidos agora): UI construída em português desde a Fase 10 inteira, contrariando a diretriz de código em inglês; CSP desabilitado (`security.csp: null`) no Tauri. Sessão 46, mesmo dia: performance + auto-fetch: `cvm_dfp.py::fetch_roe`/`fetch_payout` processavam o zip inteiro da CVM mesmo sem nenhum ticker pra buscar (sempre o caso pra FII) — ~15s desperdiçados por busca, corrigido com guard de saída antecipada; VGHF11 caiu de ~20s pra ~4,9s, PETR4 sem regressão. `fii_cvm_monthly` (yield/rentabilidade/PVP) ganhou auto-fetch (antes só botão manual). Sessão 45, mesmo dia: Pesquisa ganhou botão "Adicionar aos Ativos"; `assets.cnpj`/`fii_cnpj_cache` sincronizados (eram 2 fontes de verdade divergentes). Sessão 44: bugfix — `main.py` cortava proventos/técnicos de FII achando que dependiam da bolsai. Sessão 43: Fase 9 ("Stock Lookup") virou **"Pesquisa"** unificada (seletor de classe explícito, não aba por classe). Sessões 41-42: FII ganhou busca automática + TWR/Dietz + indicadores CVM (vacância/inadimplência/patrimônio), CNPJ resolvido via bolsai cruzado contra a CVM. Tudo testado no app real a partir da Sessão 43; `cargo test --lib` 102/102, `tsc --noEmit` limpo em todas. Sessões 33-40 (2026-07-29 a 08-02): Fase 10 completa até 10.6, Fase 8.3 confirmada bloqueada, 8.6 estendida pro `/pin`. Histórico completo, sessão a sessão, em `SESSIONS.md`.

---

## Como Usar Este Arquivo

Leia o arquivo relevante para o que você precisa:

| Para saber sobre | Leia |
|---|---|
| Diretrizes de código, segurança e ensino; ritmo do projeto | `GUIDELINES.md` |
| Visão geral, stack, status das fases, ambiente de desenvolvimento, arquitetura de código | `OVERVIEW.md` |
| **Todas as fases detalhadas (0 a 10)** | **`PHASE.md`** |
| Decisões de arquitetura em aberto | `ARCHITECTURE.md` |
| Roadmap, evoluções planejadas | `ROADMAP.md` |
| Log completo de sessões de trabalho | `SESSIONS.md` |

**Ao começar uma sessão**: diga ao Claude Code "leia os arquivos em `project/` e me ajude a continuar"
**Ao terminar uma sessão**: atualize o Log de Sessões em `SESSIONS.md` e marque etapas concluídas em `PHASE.md`
**Ao tomar uma decisão**: registre em `ARCHITECTURE.md`
**Ao mudar de máquina**: sincronize via git

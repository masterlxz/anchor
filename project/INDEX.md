# Practice Valuation — Estado do Projeto

> Este projeto usa a mesma estrutura de arquivos do TruthID (outro projeto do mesmo dono): o estado
> foi dividido em arquivos menores dentro desta pasta (`project/`), em vez de um único
> `PROJECT_STATE.md` monolítico na raiz.
> Última atualização: 2026-07-28 (Sessão 29 — estado do projeto migrado de `PROJECT_STATE.md` para `project/`, seguindo a estrutura do TruthID; Fase 10 corrigida (Django/Postgres era equívoco do Gemini), ampliada (carteira pertence ao Workspace, rentabilidade histórica via TWR/Dietz Modificado) e com ordem de implementação decidida — Workspace nasce single-user, multi-ativos é a primeira fatia; Fase 2.5 implementada — auto-fill de `net_cash`/`inventory_at_market_value` no RNAV a partir do balanço CVM, corrigindo códigos de conta errados de uma ideia anterior; Fase 2.4 redesenhada e implementada — extração de dado de PDF via IA, reaproveitando o chat multi-provider da Fase 7 em vez de um script isolado, nascendo genérica pra também servir a Fase 10.5 no futuro — código pronto, teste real contra API ainda pendente por falta de PDF à mão)

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

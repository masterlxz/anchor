# Practice Valuation — Estado do Projeto

> Este projeto usa a mesma estrutura de arquivos do TruthID (outro projeto do mesmo dono): o estado
> foi dividido em arquivos menores dentro desta pasta (`project/`), em vez de um único
> `PROJECT_STATE.md` monolítico na raiz.
> Última atualização: 2026-07-28 (Sessão 31 — Fase 8.1 deployada de verdade em Base Sepolia (`0x9e2fd1a4146e3c40ecfef9d35a09e75e00ac30e8`, via Ledger física) e Fase 8.2 (escrita via canal delegado do TruthID) implementada e validada ponta a ponta pelo usuário no app real; achado/corrigido no meio do caminho um bug pré-existente de regra de hooks do React no TruthID Desktop (`App.tsx`), fora do escopo original mas bloqueava o teste ao vivo. Sessão 30 anterior: dois rascunhos à parte trazidos pelo dono do projeto transcritos e removidos: `practice-valuation-carteira-multiativo (1).md` ampliou a Fase 10 com o modelo detalhado de multi-ativos — listagem/exposição/classe, FII/REIT/ETF/Metal/Imóvel/Empresa não listada, `AtivoManual`, anexos polimórficos, busca unificada (item 8); `practice-valuation-mobile-custodia-nome.md` virou Gestão de Custódias/Contas na Fase 10 (item 9) e uma nova Fase 11 — Rebranding pra **Anchor**, App Mobile em **Flutter** e Empacotamento do desktop (sidecar Python via PyInstaller/Nuitka + CI GitHub Actions). Nenhum código tocado, só registro/planejamento. Sessão 29 anterior: estado do projeto migrado de `PROJECT_STATE.md` para `project/`, seguindo a estrutura do TruthID; Fase 10 corrigida (Django/Postgres era equívoco do Gemini), ampliada (carteira pertence ao Workspace, rentabilidade histórica via TWR/Dietz Modificado) e com ordem de implementação decidida — Workspace nasce single-user, multi-ativos é a primeira fatia; Fase 2.5 implementada — auto-fill de `net_cash`/`inventory_at_market_value` no RNAV a partir do balanço CVM, corrigindo códigos de conta errados de uma ideia anterior; Fase 2.4 redesenhada e implementada — extração de dado de PDF via IA, reaproveitando o chat multi-provider da Fase 7 em vez de um script isolado, nascendo genérica pra também servir a Fase 10.5 no futuro — código pronto, teste real contra API ainda pendente por falta de PDF à mão)

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

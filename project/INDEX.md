# Practice Valuation — Estado do Projeto

> Este projeto usa a mesma estrutura de arquivos do TruthID (outro projeto do mesmo dono): o estado
> foi dividido em arquivos menores dentro desta pasta (`project/`), em vez de um único
> `PROJECT_STATE.md` monolítico na raiz.
> Última atualização: 2026-08-02 (Sessão 50 — ETF ganhou tela de análise própria em Research (`EtfLookupSection.tsx`: cotação/SMA/CAGR/proventos), sem herdar fundamentos/DCF/"preço teto" da ação — nenhum desses conceitos existe pra fundo de índice, removidos por pedido explícito ("tira a sessão de preço teto, não tem como fazer"). Confirmado de novo que composição de carteira/índice replicado/site do fundo não têm fonte automática gratuita (nem Yahoo, nem bolsai). Testado ao vivo com BOVA11/SMAL11. Sessão 49, mesmo dia: **ETF (B3)** virou classe nova de ativo, fatia de cotação automática — cadastro geral de fundos da CVM não tem nenhum registro de ETF (`cad_fi.csv` conferido, zero match pra "ISHARES"), diferente de FII. Sessão 48: Fase 6.1 fechada — CSP configurada no Tauri (testada sem quebrar o `tauri dev`) e UI inteira traduzida pra inglês (~20 arquivos). Sessão 47: checklist de segurança rodado contra o repo real e todo o histórico do git — limpo. Sessões 41-46: FII ganhou busca automática, indicadores da CVM, Fase 9 virou "Research" unificada, bugfix de proventos vazios, Pesquisa/Ativos convergentes, performance corrigida. `tsc --noEmit` limpo em todas as sessões (Rust sem mudança nas Sessões 48/50). Sessões 33-40: Fase 10 completa até 10.6, Fase 8.3 confirmada bloqueada. Histórico completo, sessão a sessão, em `SESSIONS.md`.

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

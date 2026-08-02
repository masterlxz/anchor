# Practice Valuation — Estado do Projeto

> Este projeto usa a mesma estrutura de arquivos do TruthID (outro projeto do mesmo dono): o estado
> foi dividido em arquivos menores dentro desta pasta (`project/`), em vez de um único
> `PROJECT_STATE.md` monolítico na raiz.
> Última atualização: 2026-08-02 (Sessão 49 — **ETF (B3)** virou classe nova de ativo, só na fatia de cotação automática (mesmo Yahoo `.SA` de ação/FII, TWR estendida de graça) — indicador de fundo específico não deu: cadastro geral de fundos da CVM não tem nenhum registro de ETF (`cad_fi.csv` conferido de verdade, zero match pra "ISHARES"), diferente de FII que tem categoria própria de dados abertos. Testado ao vivo com BOVA11. Sessão 48, mesmo dia: Fase 6.1 fechada — CSP configurada no Tauri (testada sem quebrar o HMR do `tauri dev`, único modo que o app roda hoje) e UI inteira traduzida pra inglês (~20 arquivos, escopo restrito a texto visível, siglas BR tipo B3/FII/CPF/CNPJ mantidas). Sessão 47: checklist de segurança rodado contra o repo real e todo o histórico do git — segredos/credenciais limpos. Sessões 41-46: FII ganhou busca automática, indicadores da CVM (vacância/inadimplência/patrimônio), Fase 9 virou "Research" unificada, bugfix de proventos vazios, Pesquisa/Ativos convergentes, performance da CVM corrigida. `cargo test --lib` 102/102, `tsc --noEmit` limpo em todas as sessões. Sessões 33-40 (2026-07-29 a 08-02): Fase 10 completa até 10.6, Fase 8.3 confirmada bloqueada, 8.6 estendida pro `/pin`. Histórico completo, sessão a sessão, em `SESSIONS.md`.

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

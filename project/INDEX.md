# Practice Valuation — Estado do Projeto

> Este projeto usa a mesma estrutura de arquivos do TruthID (outro projeto do mesmo dono): o estado
> foi dividido em arquivos menores dentro desta pasta (`project/`), em vez de um único
> `PROJECT_STATE.md` monolítico na raiz.
> Última atualização: 2026-08-03 (Sessão 60 — **"Sobre o ativo"** (EM DEBUG, não fechada): resumo qualitativo gerado por IA (Claude/OpenAI/Gemini, infra já existente da Fase 7) em todas as 7 telas de Research — cacheado por `(ticker, asset_class)` numa tabela nova (`company_ai_info`), deveria gerar sozinho na primeira visita, botão "Regenerate" abre popup pedindo o motivo. Sem key de IA cadastrada, a seção some por completo, sem erro. Implementação completa (schema + `commands/company_ai_info.rs` + `chat::generate_completion` + componente `AboutCompanySection.tsx` plugado nas 7 telas), `cargo test --lib` 102/102 e `tsc --noEmit` limpos, migration aplicada no banco real de dev (achado: app não migra sozinho no startup, sempre manual via `sea-orm-cli migrate up`). **Mas o dono do projeto testou ao vivo e a seção fica só com "—", sem gerar nem mostrar o botão** — `company_ai_info` seguiu com 0 linhas. Hipótese líder (não confirmada): `AboutCompanySection.tsx` só expõe erro de `generateMutation`, não de `infoQuery` — se a leitura falhar, fica invisível e a geração automática (que depende dela) nunca dispara. Debug continua na próxima sessão. Sessão 59 — **Ação americana, Fatia 2**: fundamentos + DCF via SEC EDGAR (`data.sec.gov/api/xbrl/companyconcept`), fechando paridade completa com Ação BR (os 8 modelos de valuation passam a funcionar também pra ticker americano) — escopo ampliado em relação ao planejado originalmente ("só DCF") depois de confirmado ao vivo que uma fonte só cobre os dois tipos de fundamento, decisão do dono do projeto via `AskUserQuestion`. 2 achados críticos só descobertos testando ao vivo: filtro de `form`/`fp` sozinho não basta pra tags de fluxo (Apple mistura dado trimestral na mesma tag dentro do 10-K, corrigido exigindo duração de 350-380 dias) e a tag `Revenues` da Apple só tem dado até 2018 (empresa migrou de tag depois da ASC 606 — "primeira tag com dado" pegava o ano errado, corrigido pra "mais recente entre todas as tags candidatas"). Confirmado ao vivo o mesmo gap de taxonomia de banco que a CVM já tinha com COSIF (JPMorgan sem EBIT/estoque/contas a receber-pagar — DCF descarta o ticker, fundamentos básicos continuam). Corrigido de caminho um bug real: `useTickerCollector.ts` sempre roteava pro Yahoo `.SA` (BR), quebrando o botão "Fetch" dos forms de valuation pra ticker americano — parametrizado com `assetClass`. Sessão 58 — Ação americana, Fatia 1: cotação/técnicos/dividendos/histórico via Yahoo sem sufixo `.SA`, sétima classe expandida da Fase 10 item 8. Sessão 57 — **Metal revisado**: unidade voltou pra onça troy, ganhou Prata/Platina/Paládio. Sessão 56 — **Valuation migrou pra dentro de Research** (botões "New Valuation"/"All Saved Valuations" dentro da análise de Ação BR), "Calculate"/"Save" separados. Sessão 55 — **Metal (ouro)** virou a sexta classe expandida. Sessão 54 — TruthID Sync virou seção em Configurações. Sessão 53 — **BDR (B3)** virou a quinta classe expandida. Sessão 52 — **Gráfico de preço** padrão em toda classe. Sessão 51 — **Cripto** virou classe de ativo de verdade (CoinGecko). Sessão 50/49 — **ETF (B3)** virou classe nova. Sessão 48: Fase 6.1 fechada. Sessão 47: checklist de segurança limpo. Sessões 41-46: FII completo. Sessões 33-40: Fase 10 completa até 10.6, Fase 8.3 confirmada bloqueada. Histórico completo, sessão a sessão, em `SESSIONS.md`.

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

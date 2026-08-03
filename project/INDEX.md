# Practice Valuation — Estado do Projeto

> Este projeto usa a mesma estrutura de arquivos do TruthID (outro projeto do mesmo dono): o estado
> foi dividido em arquivos menores dentro desta pasta (`project/`), em vez de um único
> `PROJECT_STATE.md` monolítico na raiz.
> Última atualização: 2026-08-03 (Sessão 59 — **Ação americana, Fatia 2**: fundamentos + DCF via SEC EDGAR (`data.sec.gov/api/xbrl/companyconcept`), fechando paridade completa com Ação BR (os 8 modelos de valuation passam a funcionar também pra ticker americano) — escopo ampliado em relação ao planejado originalmente ("só DCF") depois de confirmado ao vivo que uma fonte só cobre os dois tipos de fundamento, decisão do dono do projeto via `AskUserQuestion`. 2 achados críticos só descobertos testando ao vivo: filtro de `form`/`fp` sozinho não basta pra tags de fluxo (Apple mistura dado trimestral na mesma tag dentro do 10-K, corrigido exigindo duração de 350-380 dias) e a tag `Revenues` da Apple só tem dado até 2018 (empresa migrou de tag depois da ASC 606 — "primeira tag com dado" pegava o ano errado, corrigido pra "mais recente entre todas as tags candidatas"). Confirmado ao vivo o mesmo gap de taxonomia de banco que a CVM já tinha com COSIF (JPMorgan sem EBIT/estoque/contas a receber-pagar — DCF descarta o ticker, fundamentos básicos continuam). Corrigido de caminho um bug real: `useTickerCollector.ts` sempre roteava pro Yahoo `.SA` (BR), quebrando o botão "Fetch" dos forms de valuation pra ticker americano — parametrizado com `assetClass`. Novo `sources/sec_edgar.py`, `UsStockLookupSection.tsx` ganhou os blocos de Fundamentals/DCF/Saved valuation. `cargo test --lib` 102/102, `tsc --noEmit` limpo, testado ao vivo contra AAPL/MSFT/JPM reais. Sessão 58 — Ação americana, Fatia 1: cotação/técnicos/dividendos/histórico via Yahoo sem sufixo `.SA` (`acoes_yahoo.py` ganhou parâmetro `suffix`), sétima classe expandida da Fase 10 item 8, fundamentos deixados pra Fatia 2 de propósito (risco de mapeamento de tags XBRL isolado numa sessão dedicada). Sessão 57 — **Metal revisado**: unidade voltou pra onça troy (a real do contrato COMEX/NYMEX, sem conversão na fonte), com um indicador simples "US$/g" só de referência na tela — reverte a conversão onça→grama da Sessão 55, pedido explícito do dono do projeto depois de testar. Ganhou **Prata/Platina/Paládio** (mesmo mecanismo do ouro, confirmado ao vivo), cobre ficou de fora (unidade em libra-peso, diferente). Sessão 56 — **Valuation migrou pra dentro de Research** — a antiga aba própria no nav principal (seletor de modelo + 8 forms) virou botões "New Valuation"/"All Saved Valuations" dentro da análise de Ação BR (`StockAnalysisSection.tsx`), planejado via `/plan`. Junto, "Calculate" e "Save" viraram ações separadas (`calculate_<model>` preview puro, `save_<model>` novo persiste de verdade). Sessão 55 — **Metal (ouro)** virou a sexta classe expandida da Fase 10 item 8, contrato futuro do COMEX (`GC=F`), só ouro por ora. Sessão 54 — TruthID Sync saiu do nav principal e virou seção em Configurações. Sessão 53 — **BDR (B3)** virou a quinta classe expandida, zero código Python novo (mesmo endpoint Yahoo `.SA`). Sessão 52 — **Gráfico de preço** com seletor de período, padrão em toda classe com cotação automática. Sessão 51 — **Cripto** virou classe de ativo de verdade (CoinGecko), Fear & Greed Index global. Sessão 50/49 — **ETF (B3)** virou classe nova, tela de análise própria em Research. Sessão 48: Fase 6.1 fechada (CSP + inglês). Sessão 47: checklist de segurança limpo. Sessões 41-46: FII completo (busca automática, indicadores CVM, Research unificada). Sessões 33-40: Fase 10 completa até 10.6, Fase 8.3 confirmada bloqueada. Histórico completo, sessão a sessão, em `SESSIONS.md`.

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

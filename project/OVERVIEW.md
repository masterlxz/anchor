# O que é o Practice Valuation

App desktop pessoal para acompanhar teses de investimento em ações (B3) e criptoativos.
Substitui a ideia original de planilha (ver Fase 2, histórico) por um app com banco de dados local.

**O que ele precisa fazer (visão do usuário, ainda sendo refinada):**
- Puxar o máximo de dados possível de fontes externas (fundamentos de ações BR, dados on-chain/mercado de cripto), com espaço pra ajuste manual quando necessário
- Guardar **múltiplos preços-teto/cálculos de valuation por ativo**, cada um com seu próprio conjunto de premissas (ex: duas projeções do mesmo ativo com taxas de crescimento diferentes, ambas salvas e comparáveis lado a lado)
- Cadastrar premissas por ativo (incluindo cripto) e monitorar indicadores automaticamente
- Avisar o usuário quando um ativo entrar em "zona de compra" segundo as premissas cadastradas
- Banco de dados **local** por enquanto — sync entre máquinas/nuvem é ideia pra mais adiante (ver Roadmap)

**Decidido até agora** (ver "Decisões de Arquitetura em Aberto"):
- Stack híbrida: app em **Tauri + Rust + React/TypeScript** (reaproveitando o padrão do TruthID), coleta de dados em **Python** (ver Fase 2), os dois se comunicando só através de um banco **SQLite** local compartilhado — sem API/IPC entre eles

- UI: **Tailwind CSS + shadcn/ui (Radix) + TanStack Table**, visual **arejado tipo dashboard** (não denso tipo planilha, apesar da ideia original de "funcionar como planilha" — isso ficou pro comportamento/dado, não pra densidade visual)

**Ainda não decidido**:
- Biblioteca de gráfico (pra tela de cripto/indicadores, Fase 4.3) — avaliar quando chegar lá (candidatos: Recharts, ou lightweight-charts da TradingView, mais voltada pra preço/candlestick)
- ~~Lista exata de metodologias de preço-teto~~ — entregue na Sessão 1, ver Fase 3

---

# Status Geral

```
Fase 0 — Fundamentos & Decisões de Arquitetura   [~] Em andamento (0.1–0.5 ✓, falta 0.6)
Fase 1 — Modelo de Dados (schema do banco local)  [~] Em andamento (migrations rodando normalmente a cada modelo, falta só formalizar 1.3 como concluída)
Fase 2 — Coleta de Dados (ações BR + cripto)      [~] Em andamento (cotação e dividendo médio do Bazin via Yahoo Finance — brapi removida na Sessão 10 —, LPA/VPA via bolsai, ROE e Payout (Banks) via CVM (Sessão 16 — ROE corrigido, payout novo, ambos calculados direto do DFP/DMPL) e todas as entradas contábeis do DCF via CVM funcionando ponta a ponta — nenhuma pendência de dado de ações sobrando; 4 dos 9 indicadores cripto automatizados (TVL Trend, Net Issuance, Fees vs Emissão, NVT Ratio — os 2 últimos concluídos na Sessão 21) — ver Log de Sessões; os outros 5 (MVRV, Puell, Exchange Netflow, Endereços Ativos, Staking Yield) têm bloqueio documentado, sem fonte grátis conhecida)
Fase 3 — Motor de Cálculo (preço-teto/valuation)  [x] Completa — 7 modelos de ação + score cripto (9 indicadores), todos ponta a ponta
Fase 4 — Interface Desktop                        [~] Em andamento (shadcn/ui + TanStack Table instalados, tela de valuations salvos completa incl. detalhe fino de premissas, 7 formulários + painel cripto vestidos, identidade visual dark+verde definida; Sessão 10: painel cru de tabelas de teste removido, cada formulário ganhou botão de buscar dado por ticker que preenche os campos automaticamente)
Fase 5 — Monitoramento & Alertas                  [x] Completa — cadastro (5.1), verificação periódica (5.2) e notificação nativa do SO (5.3, validada na Sessão 12)
Fase 6 — Publicação (GitHub público)               [~] Em andamento (6.2 README e 6.3 LICENSE concluídos na Sessão 25, adiantados por pedido; 6.4 já estava satisfeita há tempos — o repo já tinha histórico público em `origin`/`gitea` bem antes desta sessão, checklist só nunca tinha sido marcado; falta só 6.1, checklist de segurança final)
Fase 7 — Chat de IA Integrado                      [x] Completa (7.1 a 7.10.6 — Sessão 27 (2026-07-21) fechou a fase inteira, entregando 7.10.3 (contador de tokens) e 7.10.4 (IA propõe criar valuation, com confirmação humana obrigatória) na mesma sessão; testado ponta a ponta pelo usuário no app real, incl. aprovar/descartar/reload; Claude/OpenAI seguem sem teste contra API real por falta de chave de teste — não bloqueia o fechamento da fase, é uma pendência à parte — ver Fases Detalhadas)
Fase 8 — Sync Multi-Dispositivo via TruthID+IPFS  [~] Em andamento — Sessão 23 (2026-07-16): transporte cross-device do `/sign-request` validado ponta a ponta; Sessão 24 (2026-07-21): fatia 8.1 (contrato próprio `SyncRegistry` + leitura via `eth_call`) implementada e testada — deploy real em Base Sepolia ainda pendente, aguardando revisão do dono do projeto; ver Fases Detalhadas
Fase 9 — Tela de Pesquisa de Ação (Stock Lookup)  [x] Completa — 9.1 a 9.5 concluídas, nenhuma pendência conhecida
Fase 10 — Remodelagem para Workspaces Colaborativos & Multi-Ativos [ ] Não iniciada — ideia grande registrada na Sessão 29 (workspaces/RBAC/multi-ativos/teses/watchlists + rentabilidade histórica via TWR); confirmado que fica na stack atual (Tauri+Rust+SQLite, sem servidor/Postgres/Django), carteira pertence ao Workspace (não a um membro), mas colaboração multiusuário de verdade ainda depende de um problema de permissão descentralizada não resolvido (ver Fases Detalhadas)
```

---

# Ambiente de Desenvolvimento

**Docker** — decidido na Sessão 1, mesmo padrão usado no TruthID: um único container com Node + Rust + WebKitGTK (pra abrir a janela do Tauri) e também **Python3 + venv** (pra rodar os coletores de dados chamados pelo próprio app). `docker compose up` sobe tudo, X11 do host repassado pro container pra a janela do app aparecer na tela — nada precisa ser instalado na máquina.

Diferente do TruthID (que precisava de acesso a USB pra Ledger), este projeto não mexe com hardware — o container fica mais simples e menos privilegiado (sem `privileged: true`, sem montar `/dev`).

Criado na Fase 0.5: `desktop/Dockerfile`, `desktop/docker-compose.yml`, `desktop/dev.sh` (`xhost +local:docker && docker compose up`).

**⚠️ Cuidado (achado na Sessão 1)**: a pasta do app se chama `desktop/`, igual a do TruthID — sem um `name:` explícito no topo do `docker-compose.yml`, o Compose usa o nome da pasta como nome do projeto e **colide** com as imagens/volumes do TruthID (`desktop-desktop`, `desktop_cargo-*`). Por isso o `docker-compose.yml` daqui tem `name: practice-valuation` logo na primeira linha — não remover.

**⚠️ Cuidado com espaço em disco**: a máquina roda os dois projetos (TruthID + Practice Valuation) e o disco de 32GB vive perto do limite por causa dos caches Docker do TruthID (imagens Flutter/Gradle/NDK, cache do cargo). Antes de builds Docker pesados, checar `df -h /` — na Sessão 1 o disco chegou a 100% (0 disponível) durante o setup inicial e isso **causou perda de arquivos** (os 3 arquivos de Docker recém-criados sumiram no meio de uma operação). `docker image prune -f` remove imagens órfãs com segurança (não mexe em nada usado pelo TruthID); ir além disso (limpar volumes/imagens nomeadas do TruthID) é decisão do usuário, não fazer sem perguntar.

---

# Arquitetura de Código

Decidido na Sessão 1: mesmo sendo um projeto pessoal, vale organizar bem desde o início — "fácil manutenção" não significa construir mais funcionalidade agora, significa estruturar bem o pouco que já existe.

**Camadas no lado Rust** (convenção adotada, aplica a partir da Fase 3):
- **`commands/`** — a "cola" exposta ao React via `invoke()`. Fina: só recebe o pedido e chama a camada de baixo, não tem regra de negócio
- **`domain/`** (ou `valuation/`) — as funções puras de cálculo (Bazin, DCF, etc.) e a lógica do score cripto. Não sabem nada de banco nem de Tauri — só recebem números/dados, devolvem números/resultado. É a "função pura" já mencionada na Fase 3, só que com um lugar físico definido
- **Repository/entities (SeaORM)** — só sabe conversar com o banco

Princípio: não misturar regra de negócio com acesso a banco — o mesmo motivo pelo qual qualquer linguagem separa "service layer" de "data layer".

**Tratamento de erro (Rust)**: **`thiserror`** — um enum de erro próprio (`AppError::AssetNotFound`, `AppError::InvalidGuard`, etc.) que serializa pro React como JSON estruturado (`{ code, message }`), em vez de string solta. Decidido na Sessão 1 em vez de `anyhow` (mais genérico/dinâmico, bom pra prototipagem rápida, mas não dá pra distinguir tipos de erro na UI depois).

**Busca de dados no React**: **TanStack Query** pra chamar os comandos Tauri (`invoke()`) — cuida de cache, loading, erro e refetch de forma consistente em toda tela, em vez de cada componente reinventar isso com `useState`/`useEffect`. Decidido na Sessão 1; mesma família do TanStack Table já escolhido pra grid (Fase 0.4).

**Testes**: funções de `domain/` são puras (input → output, sem I/O) — dá pra testar sem precisar de banco nem de mock, então a prática é escrever teste unitário junto de cada função de cálculo conforme ela é escrita (não é uma decisão de infraestrutura, é só disciplina a manter).


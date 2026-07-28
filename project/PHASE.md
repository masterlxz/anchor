## Fases Detalhadas

### Fase 0 — Fundamentos & Decisões de Arquitetura

**Objetivo**: decidir, com calma e com explicação de trade-offs, a stack do projeto antes de escrever código de verdade.

**Etapas**:
- [x] 0.1 — Nome do projeto → **Practice Valuation** (repo: `practice-valuation`), decidido na Sessão 1
- [x] 0.2 — Framework do app desktop → **Tauri + Rust + React/TypeScript** (mesmo padrão do TruthID), decidido na Sessão 1
- [x] 0.3 — Banco de dados local → **SQLite** (compartilhado entre o app Tauri/Rust e os coletores em Python), decidido na Sessão 1
- [x] 0.4 — Stack/lib de UI e direção visual → **Tailwind + shadcn/ui + TanStack Table**, visual **arejado tipo dashboard** (não denso), decidido na Sessão 1
- [x] 0.5 — Estrutura inicial do repositório, criada na Sessão 1:
  - `desktop/` — projeto Tauri + React + TS, gerado via `create-tauri-app` e renomeado (`practice-valuation`). Tailwind v4 já plugado (`@tailwindcss/vite`, `src/index.css`) — shadcn/ui e TanStack Table entram quando a Fase 4 começar a construir telas de verdade
  - `desktop/Dockerfile` + `docker-compose.yml` + `dev.sh` — ambiente de dev (ver "Ambiente de Desenvolvimento")
  - `data-collector/` — pasta reservada pro coletor Python (Fase 2), com só um `README.md` e `requirements.txt` vazio por enquanto — implementação real ainda não começou
  - Ainda falta: README.md na raiz do repo, LICENSE
- [ ] 0.6 — Checklist de segurança aplicado desde o primeiro commit (ver "Diretriz de segurança" acima)

---

### Fase 1 — Modelo de Dados

**Objetivo**: desenhar o schema do banco local que sustenta tudo — ativos, premissas, cálculos salvos, indicadores e alertas.

**Fonte da verdade pro schema**: as fórmulas completas de cada metodologia estão na Fase 3 (inputs, cálculo e guarda de erro, modelo por modelo).

**Entidades decididas** (revisado 2x depois da spec chegar — ver as duas notas de mudança de abordagem abaixo):
- `asset` — ativo acompanhado (ação BR ou cripto), com tipo, ticker/símbolo, nome
- **`valuation`** — tabela compartilhada por todos os modelos, com os campos comuns da "regra geral": `ticker`, `ano_ref`, `preco_atual`, `model` (qual dos 7 modelos), `preco_justo` (calculado, cacheado, nulo se a guarda de erro impediu o cálculo), `margem_seguranca`, `veredito`, `data_ultima_atualizacao`. Cada linha é um cálculo salvo — nada é sobrescrito, dá pra ter várias linhas do mesmo ticker com premissas diferentes (o "múltiplos preços-teto salvos" pedido desde o início). É a tabela que alimenta a tela de listagem (Fase 4.1) — uma consulta só, sem `UNION`
- **Uma tabela pequena de inputs por modelo**, ligada a `valuation` por FK (`valuation_id`), só com os campos específicos daquele modelo: `bazin_inputs`, `graham_inputs`, `gordon_ddm_inputs`, `dcf_fcff_inputs`, `bank_pb_roe_gordon_inputs`, `realty_rnav_inputs`, `projected_ceiling_price_inputs` (ver campos de cada um na Fase 3)
- `cripto_indicadores` — série temporal do score cripto: `moeda`, `data`, `indicador`, `valor_bruto`, `sinal` (verde/vermelho), `fonte` — permite plotar a evolução do score, não só o snapshot do dia
- `tracked_indicator` / `alert` — ainda a desenhar (Fase 5), quando entrarmos no motor de monitoramento/zona de compra

**Mudança de abordagem #1 (Sessão 1, depois da spec chegar)**: a ideia anterior de premissas genéricas em JSON (`assumption_set` flexível) foi **substituída** por tabelas rígidas por modelo, como o próprio spec funcional sugere — agora que os campos de cada metodologia são conhecidos e estáveis (não é mais "esperando a lista"), colunas tipadas por modelo são mais simples de validar (ex: as guardas `WACC−g <= 0`, `Ke <= g`) e mais fáceis de consultar do que um blob JSON.

**Mudança de abordagem #2 (Sessão 1, revisão pedida pelo usuário)**: a primeira versão dessa correção tinha virado "uma tabela por modelo" **auto-contida** (7 tabelas, cada uma repetindo os campos comuns tipo `ticker`/`ano_ref`/`preco_justo`). O usuário pediu uma revisão pensando em manutenção de longo prazo, e isso foi trocado por **`valuation` compartilhada + tabela de inputs por modelo** (acima) — evita repetir os campos comuns 7 vezes (mudar um campo comum = 1 migration, não 7) e deixa a tela de "listar tudo" trivial. Os inputs continuam tipados por modelo (não regrediu pra JSON) — só o que era comum foi extraído.

**Regra geral, comum a todos os modelos de ação** (ver spec): `margem_segurança = (preço_justo − preço_atual) / preço_justo`; `veredito` = BARATO se margem > 0, senão CARO. Todo modelo também carrega `ticker`, `ano_ref` (o app calcula `anos_desatualizado = ano_atual − ano_ref` e sinaliza: ≤0 em dia, ==1 atenção, ≥2 desatualizado) e `preço_atual` (API com fallback manual).

**Como Rust e Python acessam o mesmo banco**: os dois já rodam dentro do mesmo container (decisão da Fase 0 — stack híbrida), então não precisa de rede/API entre eles — só apontar os dois pro mesmo arquivo `.db`. Arquivo físico decidido: `data-collector/practice_valuation.db` (já coberto pelo `*.db` do `.gitignore` da raiz; a pasta já é bind-mount, então o arquivo sobrevive entre execuções do container).

**Etapas**:
- [x] 1.1 — Entidades validadas — desbloqueado pela chegada do spec funcional (Sessão 1). Ver "Mudança de abordagem" acima
- [x] 1.2 — Driver/ORM Rust: **SeaORM** — decidido na Sessão 1 (revisado depois de decidir `rusqlite` na mesma sessão). Motivo: o usuário já tem hábito de pensar em ORM (estilo Django/SQLAlchemy/ActiveRecord); `rusqlite` exigiria escrever SQL cru e mapear linha a linha na mão, atrito maior do que ganho de simplicidade pra quem tá aprendendo Rust e banco ao mesmo tempo. SeaORM imita bem esse modelo mental (`Entity::find().all(&db)`, migrations via `sea-orm-cli`, geração de entity a partir do schema). É assíncrono, mas isso não é custo extra real — o Tauri já roda sobre `tokio`. Trade-off aceito: SeaORM é mais novo/menos batalhado que Diesel (a alternativa "ORM maduro", descartada pela sintaxe de query mais macro-pesada e curva de compilador mais dura)
- [x] 1.2b — **Modo WAL** (Write-Ahead Log) do SQLite será ligado por padrão — Rust e Python são processos diferentes lendo/escrevendo o mesmo arquivo, e WAL deixa isso coexistir melhor (menos "database is locked")
- [x] 1.3 — Migrations iniciais (abordagem simples: arquivos SQL versionados aplicados em ordem, sem framework pesado). Rodando normalmente desde a Sessão 2 — cada modelo/indicador novo ganha sua própria migration (`sea-orm-cli migrate generate`), aplicada com `migrate up`. Marco final: 9 migrations aplicadas (`valuation`+`bazin_inputs`, uma por modelo de ação, `indicator_thresholds`+`crypto_indicators`)

---

### Fase 2 — Coleta de Dados

**Objetivo**: puxar o máximo de dado possível de fontes externas, com fallback manual quando a fonte automática não cobre.

**Histórico**: o levantamento abaixo foi desenhado pelo usuário antes deste projeto virar app desktop, pensando em escrever direto numa planilha do Google Sheets (via Service Account + `gspread`). Essa rota foi abandonada na Sessão 1 — o desenho de fontes/APIs e o pipeline de dados continuam válidos, só o destino final mudou de "planilha" pra "banco de dados local do app" (o módulo `sheets/writer.py` e a autenticação via Service Account descritos na ideia original não se aplicam mais).

**Fontes já mapeadas**:
| Categoria | Dado | Fonte primária | Fallback |
|---|---|---|---|
| Ações BR | Fundamentos (P/L, P/VP, ROE, ROIC, margens, EV/EBITDA — 27 indicadores TTM) | bolsai (200 req/dia grátis) | — |
| Ações BR | Cotação atual | Yahoo Finance (não-oficial, grátis pra qualquer ticker — trocou a brapi na Sessão 10, que exigia token pago fora de 4 tickers demo) | — |
| Ações BR | Balanço/DRE/DFC histórico (contas CVM brutas) | bolsai / CVM Dados Abertos (DFP/ITR) | — |
| Ações BR | Dividendos históricos | Yahoo Finance | — |
| Cripto | Preço, market cap, volume | CoinGecko | — |
| Cripto | TVL (DeFi) | DefiLlama | — |
| Cripto | Emissão líquida (issuance − burn, ETH) | ultrasound.money | — |
| Cripto | Endereços ativos/transações diárias | Etherscan (rate limit baixo) | — |
| Cripto | Exchange netflow, MVRV Z-Score, Puell Multiple | CryptoQuant/Glassnode (pago, sem alternativa gratuita boa) | manual, link pro dashboard |
| Cripto | Staking Yield líquido | stakingrewards.com (free tier limitado) | manual |
| PDF/release não estruturado | Campos qualitativos (landbank, comentários) | pdfplumber/PyMuPDF + API Claude (schema fixo) | preenchimento manual |

Cobre bem os indicadores de **triagem** (P/L, P/VP, ROE, DY, EV/EBITDA, CAGR receita) e 6 dos 8 indicadores de cripto de graça. Pros inputs finos do DCF completo (Capex de expansão vs manutenção, ΔNWC detalhado) o script deixa pré-preenchido com o dado bruto da CVM, mas ainda vale conferir contra o release nos casos historicamente problemáticos (banco, incorporadora).

**Estrutura de módulos planejada** (pasta `data-collector/`, ver Fase 0.5):
```
data-collector/
├── main.py                    # orquestrador — roda tudo ou um módulo específico
├── config.yaml                # lista de tickers/moedas a acompanhar, chaves de API
├── requirements.txt
└── sources/
    ├── acoes_bolsai.py         # cliente da API bolsai (fundamentos B3)
    ├── acoes_yahoo.py          # cliente do Yahoo Finance (cotação + dividendo médio)
    ├── cvm_dfp.py              # baixa o zip trimestral da CVM, mapeia conta → campo do modelo
    ├── pdf_extractor.py        # pdfplumber/PyMuPDF + chamada à API Claude com schema fixo
    ├── cripto_coingecko.py     # preço, market cap, volume
    ├── cripto_defillama.py     # TVL
    ├── cripto_ultrasound.py    # emissão líquida ETH (issuance − burn)
    ├── cripto_etherscan.py     # endereços ativos / transações
    └── cripto_stakingrewards.py # staking yield líquido
```
Chaves de API ficam em `.env`/`config.yaml` fora do controle de versão (ver "Diretriz de segurança").

**CVM Dados Abertos — como funciona na prática** (fonte principal pro DCF/RNAV/Bancos): não é uma API tipo REST (não dá pra chamar `/empresa/FIQE3`). É um **arquivo zip por ano**, com o balanço de todas as ~500 empresas abertas dentro:
```
https://dados.cvm.gov.br/dados/CIA_ABERTA/DOC/DFP/DADOS/dfp_cia_aberta_2025.zip
```
Dentro do zip, vários CSVs (um por demonstração: Balanço Ativo `BPA`, Balanço Passivo `BPP`, Resultado `DRE`, Fluxo de Caixa `DFC_MI` — sempre com versão `_con` = consolidado e `_ind` = individual). Cada linha: `CNPJ_CIA | DENOM_CIA | CD_CVM | DT_REFER | CD_CONTA | DS_CONTA | VL_CONTA`. `CD_CONTA` é o código fixo da conta (ex: `3.11` = Lucro Líquido, `2.03` = Patrimônio Líquido, `2.01.04` = Estoques — igual pra qualquer empresa aberta) e `VL_CONTA` o valor daquele período. É o mesmo dataset que bolsai/brapi consultam por trás — baixando direto, dá pra montar o próprio mapeamento conta → campo do DCF (Receita, EBIT, D&A, Capex, Dívida) sem depender de a API "empacotar" exatamente o campo necessário, e não quebra quando o layout de um PDF muda.

Fluxo do `cvm_dfp.py`:
1. `baixar_zip_ano(ano)` — baixa o zip do ano uma vez (todas as empresas vêm juntas)
2. Abre os CSVs com pandas, filtra pelas linhas da(s) empresa(s) de interesse (por `CNPJ_CIA` ou `DENOM_CIA`)
3. `ticker_para_cnpj(ticker)` — a CVM identifica empresa por CNPJ, não por ticker; resolve com uma chamada rápida à API bolsai/brapi só pra traduzir
4. `extrair_contas(cnpj, lista_codigos_conta)` — pivota só os `CD_CONTA` que interessam pro modelo (mapeamento fixo)

Pra maioria das empresas "normais" (o grosso da lista), esse caminho sozinho já cobre praticamente tudo — Capex, D&A, ΔNWC, dívida, tudo vem de contas padronizadas do DFP.

**Extração via PDF (fallback, só quando o dado não é estruturado)**: coisas como composição de landbank de uma incorporadora, ou comentário qualitativo do release, não vêm no DFP — só no PDF/apresentação. Pra esses casos: `pdfplumber`/`PyMuPDF` extrai texto e tabelas → vai pra API da Anthropic (Claude) com um prompt fixo pedindo só JSON com os campos que faltam (a mesma coisa que fazer manualmente mandando o PDF no chat, só que como script) → o script valida o JSON e grava no banco junto com a fonte ("Source: Release 4T25, pág. X") pra conferência rápida.

**Etapas**:
- [x] 2.1 — Decidir onde/como a coleta roda → **processo Python separado**, disparado **manualmente por um botão na UI** ("Run"/"Atualizar dados"), sem cron/scheduler (a ideia original de cron — 1x/ano ações, 1x/dia cripto — foi descartada, ver Sessão 1). Mecanismo:
  - Frontend: botão chama `invoke()` de um comando Tauri
  - Backend (Rust): comando assíncrono dispara o script Python como subprocesso (não trava a UI), espera terminar
  - Python: puxa os dados das fontes e grava direto no SQLite compartilhado
  - Frontend: enquanto roda, mostra spinner; ao terminar, mostra um resumo (quantos ativos, sucesso/erro) — sem log ao vivo linha a linha por enquanto (pode vir depois se sentir falta)
  - **Guarda contra clique duplo/spam**: desabilitar o botão no frontend enquanto roda **e** ter uma trava no lado Rust (ex: `Mutex`/flag no estado do app) que recusa uma segunda chamada concorrente mesmo se disparada rápido demais — evita dois processos Python escrevendo no mesmo SQLite ao mesmo tempo e evita estourar rate limit das APIs gratuitas
  - A Fase 5 (alertas) pode um dia precisar de checagem periódica dos indicadores **já salvos** — isso é diferente de "puxar dado novo" e fica pra quando chegarmos lá
- [x] 2.2 — Implementar clientes de fonte de dados de ações — **`acoes_brapi.py` (cotação) concluído na Sessão 4**; **`acoes_bolsai.py` (fundamentos LPA/VPA/ROE) e `cvm_dfp.py` (fundamentos do DCF, incluindo alíquota efetiva) concluídos na Sessão 5**; **`acoes_yahoo.py` (dividendo médio 5 anos, via API não-oficial do Yahoo Finance) concluído na Sessão 6**, substituindo a bolsai (bloqueada, 403 Pro-only) e resolvendo pra qualquer ticker real (não só demo, diferente da brapi). **Sessão 10**: `acoes_brapi.py` removido inteiramente — sua limitação de token pago fora dos 4 tickers demo quebrava a busca ad hoc por ticker recém-adicionada nos formulários; cotação migrou pra `acoes_yahoo.py::fetch_quotes` (mesmo endpoint não-oficial já usado pro dividendo médio, testado contra ticker real fora da demo — BBAS3 — antes de trocar). Todas as entradas contábeis do DCF automatizadas; só sobram as 5 premissas de mercado (Beta, Rf, prêmio de risco, Kd, g), que nunca vêm de balanço. **Nenhuma pendência de dado de ações sobrando**
- [~] 2.3 — Implementar clientes de fonte de dados de cripto — **`cripto_defillama.py` (TVL Trend) e `cripto_ultrasound.py` (Net Issuance) concluídos na Sessão 5**, ambos sem cadastro. Achado corrigido: a suposição inicial de que ultrasound.money não tinha API pública (só WebSocket) estava **errada** — o backend é open source (`eth-analysis-rs`, axum) e expõe rotas REST reais em `/api/v2/fees/*`, achadas lendo o código-fonte no GitHub em vez de só a doc/site. `cripto_etherscan.py` tentado (Sessão 5) e descartado — todo o módulo `stats` de séries diárias é Etherscan API Pro-only, sem workaround gratuito razoável pra `active_addresses_trend`. `staking_yield` (stakingrewards.com) investigado na Sessão 6 e descartado — sem free tier de verdade, só planos pagos. `mvrv_z_score`, `puell_multiple` e `exchange_netflow` seguem sem fonte gratuita conhecida — continuam manuais (ver mapeamento de fontes). **NVT Ratio e Fees de Rede vs Emissão concluídos na Sessão 21** (ver detalhe abaixo) — restam só os 5 indicadores sem fonte grátis conhecida, sem novo caminho a investigar por enquanto.
  - **Fees de Rede vs Emissão** (indicador 9): `cripto_ultrasound.py` ganhou `fetch_fees_vs_emission_ratio` — o cliente já usava `/api/v2/fees/supply-over-time` (variação líquida de supply); adicionado `/api/v2/fees/burn-sums` (confirmado real, também público/sem chave) pra pegar a queima (fees) no mesmo período de 30 dias. A API não devolve emissão bruta pronta — reconstruída como `variação_líquida + queima`. Ratio final = `queima / emissão_bruta`.
  - **NVT Ratio** (indicador 2, só "Parcial" na spec original): **decisão consultada com o usuário** entre duas fontes reais confirmadas — Blockchair (volume on-chain "de verdade", mas só valor de hoje, sem histórico grátis pra montar a MA de 90d que a regra de sinal pede) vs CoinGecko (`/coins/{id}/market_chart`, grátis/sem chave, dá market cap **e** volume dos últimos 90 dias numa chamada só, mas o volume é de exchange/trading, não liquidado on-chain — proxy mais fraca da definição original de Willy Woo). Usuário escolheu a proxy do CoinGecko por funcionar desde o primeiro dia, sem esperar ~90 dias de coleta acumulada. Novo `cripto_coingecko.py::fetch_nvt_ratio_vs_ma90` — NVT de hoje dividido pela média dos 90 dias fechados anteriores (exclui o dia de hoje, que ainda está em andamento, do cálculo da própria média).
  - Nenhuma migration nova — os thresholds de `nvt_ratio` (0.9/1.3) e `fees_vs_emission` (0.5/0.1) já estavam semeados desde a Sessão 1 (`m20260709_212958_create_crypto_score_tables.rs`), só nunca tinham fonte automatizada. `main.py::main_crypto` ganhou `collect_crypto_fees_vs_emission`/`collect_crypto_nvt_ratio`, mesmo padrão de `_record_crypto_indicator` dos outros 2. `CryptoScorePanel.tsx`: botão e comentário atualizados pra citar os 4 indicadores automatizados.
  - Testado ponta a ponta contra as APIs reais: `docker compose run ... .venv/bin/python3 main.py crypto` rodado de verdade (NVT Ratio 0,982 → NEUTRAL, Fees vs Emission 0,0139 → RED — ambos conferidos manualmente antes de codar), linhas reais confirmadas em `crypto_indicators` via `sqlite3` do módulo Python (o binário `sqlite3` da imagem Docker não está no PATH — achado: um shim de node-modules faz `node` tentar carregar `./sqlite3` como módulo, gerando `Cannot find module`; contornado usando `python3 -c "import sqlite3..."` em vez do binário CLI). `npx tsc --noEmit` limpo. **Confirmado pelo usuário no app real** ("deu boa, os dois indicadores apareceram")
- [~] 2.4 — Fallback de extração via PDF, **redesenhado na Sessão 29**: a ideia original (script Python isolado, `pdfplumber` + API da Anthropic direto) foi trocada por pedido do dono do projeto — reaproveitar a infraestrutura de IA que já existe no app (chat multi-provider da Fase 7: Gemini/Claude/OpenAI, chave via keyring, `list_api_keys`/`read_api_key_secret`) e nascer **genérica** ("IA lê um documento → devolve dado estruturado"), pensando também na ideia futura de teses a partir de releases (Fase 10.5, não iniciada). **Achado de pesquisa**: os 3 provedores já aceitam PDF nativo (sem `pdfplumber`/PyMuPDF nenhum) + JSON validado contra schema — Claude via bloco `document`+`output_config.format`, Gemini via `inline_data`+`generationConfig.response_schema` (**não confirmado contra chamada real nesta sessão** — só verificado contra doc, é o primeiro lugar a olhar se a extração via Gemini der 400), OpenAI Chat Completions via bloco `file`+`response_format.json_schema`. **Decisão de arquitetura**: comando avulso (`extract_document_data` em `commands/document_extraction.rs`, novo módulo — não mexe nos `ask_*_api` de `chat.rs`, que carregam histórico/tool-calling incompatíveis), sem tabela nova no banco e sem o mecanismo pesado de `ai_valuation_proposal` (aquele existe pra proteger uma escrita irreversível; aqui só preenche um `<Input>` já editável/revisado antes do submit, mesmo espírito do auto-fill de CVM da 2.5). Frontend: `DocumentExtractDialog.tsx` genérico (escolhe PDF via `tauri-plugin-dialog`, novo — nenhum file-picker existia no app antes —, escolhe chave/modelo com o mesmo padrão do chat, mostra preview com Usar/Descartar antes de gravar no formulário) — recebe `instructions`/`jsonSchema`/`parseResult`/`renderPreview`/`onAccept` de fora, então um futuro consumidor (Fase 10.5) não precisa tocar Rust nem esse componente de novo. Primeiro consumidor: botão "Extract from PDF" ao lado do campo `landbank` do RNAV (nunca teve fonte automática — só existe em texto/tabela de release, nunca no balanço formal da CVM). Refactor de passagem: `DEFAULT_MODEL_BY_PROVIDER` (duplicado em `ChatPanel.tsx`/`useConversationRuntime.ts`) extraído pra `desktop/src/ai/modelDefaults.ts`. `cargo check`/`cargo test --lib` (83/83, 6 novos) e `npx tsc --noEmit` limpos. **Pendência pra próxima sessão**: nenhum teste real contra API rodou ainda (usuário sem PDF à mão nesta sessão) — falta confirmar que o file-picker abre de verdade, que os 3 provedores aceitam o formato do body (principalmente o Gemini, sinalizado acima), e que o fluxo Extrair → preview → Usar este valor funciona ponta a ponta no app rodando.
- [x] 2.5 — Auto-fill de `net_cash` e `inventory_at_market_value` no RNAV a partir do balanço CVM já baixado. **Concluída na Sessão 29**: planejada com `/plan` (2 agentes `Explore` em paralelo levantaram o estado atual do RNAV nas 3 camadas e os códigos de conta reais já usados no `cvm_dfp.py`). **Achado importante**: os códigos anotados na ideia original da Sessão 28 (caixa `1.01`, dívida `2.01.04.01`/`2.02.01.04`, estoque `2.03`) estavam errados — `2.03`, por exemplo, já é Patrimônio Líquido no resto do próprio arquivo. Os códigos corretos (caixa `1.01.01`, dívida `2.01.04`+`2.02.01`) já são extraídos e testados desde a Sessão 5/20 como `cash`/`total_debt` de `fetch_dcf_fundamentals` — então `net_cash = cash − total_debt` não precisou de nenhuma mudança de backend, só conta no frontend. Só `inventory` (estoque, conta `1.01.04` do BPA, já uma dependência implícita do ΔNWC mas nunca devolvida isolada) era campo novo: adicionado ao retorno de `fetch_dcf_fundamentals`, nova coluna nullable em `stock_dcf_fundamentals` (migration `m20260728_103434_add_inventory_to_stock_dcf_fundamentals`, mesmo padrão de `payout`/`tax_rate`/`revenue`). `RnavForm.tsx::handleFetch` ganhou o cálculo de `net_cash` (sem guarda de nulidade — `cash`/`total_debt` são sempre não-nulos quando `dcfFundamentals` existe) e o prefill de `inventory_at_market_value` (guarda `!== null`, mesmo padrão de payout/tax_rate), com aviso de UI que o valor vem do custo contábil da CVM, não necessariamente o de mercado. `landbank` continua 100% manual, sem fonte automática. **Achado à parte, resolvido com aprovação do usuário**: o banco de dev tinha 2 migrations órfãs (`ai_alert_proposal` + coluna `target_valuation_id`), aplicadas em 2026-07-24 sem nenhum arquivo/commit correspondente no repo — revertidas do banco local antes de rodar a migration nova (sem perda de dado real: tabela vazia, coluna sempre `NULL`). `cargo check`/`cargo test --lib` (77/77) e `npx tsc --noEmit` limpos. Testado contra a CVM real pra 3 incorporadoras (CYRE3, EZTC3, MRVE3) — valores de estoque plausíveis frente à receita de cada uma, conferidos direto no banco. **Testado pelo usuário no app real** ("já testei e deu boa").

---

### Fase 3 — Motor de Cálculo (Preço-Teto/Valuation)

**Objetivo**: calcular e salvar preços-teto/valuation com premissas customizáveis, permitindo múltiplos cálculos por ativo. Metodologias entregues pelo usuário na Sessão 1 — esta seção é a fonte da verdade completa (não precisa consultar outro arquivo).

#### Regra geral (vale pra todos os modelos de ação)

Todo modelo carrega 3 campos fixos além dos específicos — **ticker** (texto, ex: `FIQE3`), **ano de referência** (`ano_ref`, ano-base dos dados usados) e **preço atual** (R$, de API com fallback manual) — e termina com a mesma "cauda final":
```
margem_seguranca = (preco_justo − preco_atual) / preco_justo
veredito         = "BARATO" se margem_seguranca > 0, senão "CARO"
```
O app também calcula `anos_desatualizado = ano_atual − ano_ref` e sinaliza: `<=0` em dia, `==1` atenção, `>=2` desatualizado — é o campo que avisa quando revisar aquela empresa.

**Bug conhecido, não corrigido (achado na Sessão 19, investigação a pedido do usuário)**: a fórmula da margem de segurança só dá o sinal certo quando `preco_justo > 0`. Nenhum dos 8 modelos garante isso antes de dividir (só o Graham é seguro por construção, já que exige `eps > 0` e `vpa > 0`). Se alguma entrada gerar `preco_justo <= 0` — ex. Bazin com dividendo médio negativo, Gordon/Projected Ceiling com dividendo não positivo, Banks com ROE muito baixo, RNAV com dívida líquida grande, DCF com alavancagem extrema — a divisão **inverte o sinal** (preço justo negativo classifica como "BARATO" com margem alta, quando deveria sempre ser "CARO") ou produz `NaN`/`Infinity` gravado silenciosamente no banco e exibido sem aviso na UI. Fix sugerido (não implementado): guarda `if preco_justo <= 0.0 { return Err(...) }` em cada `domain::<modelo>::calculate`, mesmo padrão dos guards `WACC−g`/`Ke vs g` já existentes, cada um coberto por um teste `rejects_non_positive_fair_price`.

#### 1. DCF / FCFF (empresas "normais")

**Quando usar**: empresa com capital de giro e capex previsíveis (varejo, indústria, tech, utilities). Não usar em banco ou incorporadora.

| Input | Unidade |
|---|---|
| Receita Líquida | R$ milhões |
| EBIT | R$ milhões |
| Alíquota Efetiva de IR | % |
| D&A (Depreciação/Amortização) | R$ milhões |
| Capex | R$ milhões |
| ΔNWC (variação capital de giro) | R$ milhões |
| Dívida Total | R$ milhões |
| Caixa | R$ milhões |
| Nº de Ações | milhões |
| Beta | número |
| Rf (taxa livre de risco) | % |
| Prêmio de Risco de Mercado | % |
| Kd (custo da dívida) | % |
| g (crescimento na perpetuidade) | % |

```
FCFF         = EBIT × (1 − IR) + D&A − Capex − ΔNWC
Ke (CAPM)    = Rf + Beta × Prêmio_Risco_Mercado
E (equity)   = Preço_Atual × Nº_Ações
WACC         = [E / (E + Dívida)] × Ke + [Dívida / (E + Dívida)] × Kd × (1 − IR)
Valor_Firma  = FCFF × (1 + g) / (WACC − g)
Valor_Equity = Valor_Firma − Dívida_Total + Caixa
Preco_Justo  = Valor_Equity / Nº_Ações
```
**Guarda**: se `WACC − g <= 0`, não calcular (modelo quebra matematicamente) — mostrar aviso em vez de número.

#### 2. Gordon / DDM (Dividend Discount Model)

**Quando usar**: boa pagadora de dividendo, crescimento previsível.

| Input | Unidade |
|---|---|
| Dividendo Atual (D0) | R$/ação |
| Crescimento Esperado dos Dividendos (g) | % |
| Ke (retorno exigido) | % |

```
D1          = D0 × (1 + g)
Preco_Justo = D1 / (Ke − g)
```
**Guarda**: `Ke > g`, senão inválido.

#### 3. Bazin

**Quando usar**: "vaca leiteira" (bancão, elétrica, saneamento), foco em yield de dividendo.

| Input | Unidade |
|---|---|
| Dividendo Médio por Ação (últimos 5 anos) | R$/ação |
| Yield Desejado | % (default sugerido: 6%) |

```
Preco_Teto = Dividendo_Médio / Yield_Desejado
```

#### 4. Graham (Graham Number)

**Quando usar**: filtro rápido de margem de segurança, qualquer empresa com lucro e patrimônio positivos.

| Input | Unidade |
|---|---|
| LPA (Lucro por Ação) | R$/ação |
| VPA (Valor Patrimonial por Ação) | R$/ação |

```
Graham_Number = RAIZ(22.5 × LPA × VPA)
```
**Guarda**: se LPA ≤ 0 ou VPA ≤ 0, não calcular (empresa com prejuízo ou PL negativo não se encaixa nesse método).

#### 5. Bancos (P/B via ROE-Gordon)

**Quando usar**: bancos e instituições financeiras — FCFF não serve porque dívida é matéria-prima do negócio, não uma alavancagem a evitar.

| Input | Unidade |
|---|---|
| VPA (Valor Patrimonial por Ação) | R$/ação |
| ROE | % |
| Payout | % |
| Ke (retorno exigido) | % |

```
g_sustentável = ROE × (1 − Payout)
P/B_Justo     = (ROE − g_sustentável) / (Ke − g_sustentável)
Preco_Justo   = P/B_Justo × VPA
```
**Guarda**: `Ke > g_sustentável`.

#### 6. RIM — Lucro Residual (Bancos)

**Quando usar**: bancos e instituições financeiras, como alternativa mais robusta ao modelo Bancos (P/B via ROE-Gordon, seção 5) — em vez de assumir ROE constante pra sempre, projeta o ROE convergindo (fade linear) do patamar atual até o próprio Ke (custo de capital) ao longo de N anos explícitos, sob a leitura de que a vantagem competitiva de um banco erode até ele criar exatamente zero lucro econômico no limite. Não precisa de valor terminal separado: em t=N o ROE já é Ke, então o lucro residual dali em diante é zero por construção. Quando ROE atual = Ke, o preço justo bate exatamente no valor patrimonial — mesmo caso particular do modelo Bancos (P/B_Justo = 1 quando ROE = Ke).

| Input | Unidade |
|---|---|
| VPA (Valor Patrimonial por Ação) | R$/ação |
| ROE Atual | % |
| Payout | % |
| Ke (retorno exigido, também o ROE de convergência) | % |
| N — Anos de Fade | inteiro |

```
Para t = 1..N:
  ROE_t          = ROE_atual + (Ke − ROE_atual) × (t/N)
  LucroResid_t   = (ROE_t − Ke) × VPA_(t-1)
  VPA_t          = VPA_(t-1) × (1 + ROE_t × (1 − Payout))
Preco_Justo       = VPA0 + Σ VP(LucroResid_t)
```
**Guarda**: `N >= 1`.

#### 7. Incorporadoras (RNAV)

**Quando usar**: construtoras/incorporadoras — o "estoque" é imóvel, não dá pra projetar FCFF de forma suave trimestre a trimestre.

| Input | Unidade |
|---|---|
| Landbank a Valor de Mercado | R$ milhões |
| Estoque a Valor de Mercado | R$ milhões |
| Caixa Líquido (caixa − dívida, pode ser negativo) | R$ milhões |
| Nº de Ações | milhões |

```
RNAV_Total = Landbank + Estoque + Caixa_Líquido
RNAV/Ação  = RNAV_Total / Nº_Ações
```
(`RNAV/Ação` entra no lugar de `preco_justo` na regra geral.)

#### 8. Preço Teto Projetivo

**Quando usar**: mesma lógica do Bazin, mas trazendo N anos de crescimento esperado pra frente e descontando a valor presente — útil quando se quer o teto "olhando pra frente", não só o dividendo de hoje.

| Input | Unidade |
|---|---|
| Dividendo Atual (D0) | R$/ação |
| Crescimento Esperado (g) | % |
| Anos de Projeção (N) | inteiro (default sugerido: 5) |
| Yield Desejado (alvo, estilo Bazin) | % (default sugerido: 6%) |
| Ke (taxa de desconto) | % |

```
Dividendo_Projetado_N = D0 × (1 + g)^N
Preco_Teto_Futuro_N   = Dividendo_Projetado_N / Yield_Desejado
Preco_Teto_Projetivo  = Preco_Teto_Futuro_N / (1 + Ke)^N
```
(`Preco_Teto_Projetivo` entra como `preco_justo` na regra geral.)

#### Persistência (ver Fase 1)

Tabela `valuation` compartilhada (`ticker`, `ano_ref`, `preco_atual`, `model`, `preco_justo` cacheado, `margem_seguranca`, `veredito`, `data_ultima_atualizacao`) + uma tabela pequena de inputs por modelo (só os campos específicos, ligada por FK). Permite mostrar histórico ("como essa margem evoluiu ano a ano") sem recalcular tudo toda vez, salvar quantos cálculos o usuário quiser por ativo (premissas diferentes = linhas diferentes, nada sobrescreve), e listar tudo com uma consulta só na `valuation`.

#### Score de Cripto (Ethereum) — score contínuo

Diferente de ação (1x/ano), aqui é um **score contínuo**: cada indicador vira verde (bom pra compra/manter) ou vermelho (sinal de reduzir risco), e o app soma quantos estão verdes de um total de 9 — contagem objetiva, não "vibe".

| # | Indicador | O que mede | Fonte | Automatizável? | Regra de sinal (ponto de partida — ajustável) |
|---|---|---|---|---|---|
| 1 | MVRV Z-Score | Preço vs custo-base médio da rede | Glassnode (pago) | Não (fallback manual) | Verde se < 0 · Vermelho se > 7 |
| 2 | NVT Ratio | "P/L" da rede (valor de mercado / volume transacionado) | Calculável com dado on-chain | Parcial | Verde se abaixo da média móvel de 90d · Vermelho se muito acima |
| 3 | Puell Multiple | Emissão diária (USD) vs média histórica | Glassnode (pago) | Não (fallback manual) | Verde se < 0.5 · Vermelho se > 4 |
| 4 | Emissão Líquida (issuance − burn) | ETH é deflacionário ou não no período | ultrasound.money | Sim | Verde se negativa (deflacionário) · Vermelho se fortemente positiva |
| 5 | Staking Yield Líquido | Retorno real do staking, descontada a diluição | stakingrewards.com | Sim (free tier) | Verde se yield real > 2% · Vermelho se perto de 0 ou negativo |
| 6 | TVL DeFi (Ethereum) | Uso real da rede em DeFi | DefiLlama | Sim | Verde se em tendência de alta (MoM) · Vermelho se queda consistente |
| 7 | Endereços Ativos / Transações Diárias | Adoção/atividade da rede | Etherscan | Sim | Verde se crescendo (MoM/YoY) · Vermelho se caindo |
| 8 | Exchange Netflow | Saída (acumulação) ou entrada (venda) líquida das corretoras | CryptoQuant/Glassnode (pago) | Não (fallback manual) | Verde se saída líquida (negativo) · Vermelho se entrada líquida forte |
| 9 | Fees de Rede vs Emissão | "Receita líquida" real do protocolo pós EIP-1559 | ultrasound.money / Etherscan | Sim | Verde se fees líquidas cobrindo bem a emissão · Vermelho se dependente de emissão alta |

**Score final** = verdes / 9. Leitura sugerida (ajustável depois de ver rodando um tempo): **7-9 verdes** → tese intacta, manter/aportar · **4-6 verdes** → neutro, observar de perto · **0-3 verdes** → considerar reduzir risco/posição.

**Persistência**: tabela `cripto_indicadores` com `moeda`, `data`, `indicador`, `valor_bruto`, `sinal` (verde/vermelho), `fonte` — dá histórico de série temporal, dá pra plotar a evolução do score ao longo do tempo, não só o snapshot do dia.

**⚠️ Importante**: os thresholds acima (`< 0`, `> 7`, `> 2%`, etc.) são ponto de partida razoável baseado em uso histórico de mercado, **não são regra imutável** — o app precisa deixar esses números configuráveis (não hardcoded), porque o usuário provavelmente vai querer calibrar depois de ver como cada indicador se comporta na prática.

**Etapas**:
- [x] 3.1 — Lista de metodologias entregue (Sessão 1) — ver esta seção completa
- [x] 3.2 — Modelar cada metodologia (dos 7 modelos acima) como função pura Rust: inputs (tabela específica do modelo) → resultado (`preco_justo`, `margem_seguranca`, `veredito`), aplicando as guardas de erro — **concluído na Sessão 3**: os 7 modelos (Bazin, Graham, Gordon/DDM, DCF/FCFF, Bancos, RNAV, Preço Teto Projetivo) fechados ponta a ponta
- [x] 3.3 — Motor do score cripto — **concluído na Sessão 4**: sinal verde/neutro/vermelho por indicador com threshold configurável (tabela `indicator_thresholds`), leituras salvas em série temporal (`crypto_indicators`), score somado no front (verdes/9). Ver Log de Sessões pra detalhes de schema/domínio
- [ ] 3.4 — Permitir salvar quantos cálculos o usuário quiser por ativo (já é a natureza do schema — cada linha é um cálculo, nada sobrescreve), todos comparáveis lado a lado na UI — a parte de schema já está resolvida, falta só a tela de comparação (Fase 4)
- [x] 3.5 — 8º modelo, RIM — Lucro Residual (Bancos) — **concluído na Sessão 17**: generaliza o modelo Bancos (seção 5) permitindo o ROE convergir (fade) até o próprio Ke ao longo de N anos, em vez de assumir ROE constante pra sempre. Ver seção 6 acima pra fórmula/guarda e Log de Sessões pra detalhes de schema/domínio

---

### Fase 4 — Interface Desktop

**Objetivo**: telas simples, "planilha-like", que dão espaço pra edição manual quando preciso.

**⚠️ Nota (Sessão 3)**: as telas dos modelos já implementados (Bazin, Graham) são propositalmente cruas — `<input>` HTML puro com classes utilitárias do Tailwind, sem os componentes do shadcn/ui instalados ainda. É rascunho funcional pra provar a fatia vertical (cálculo → banco → tela) de cada modelo, não a interface final. Decisão: terminar a Fase 3 (os 7 modelos + cripto) com esse padrão cru primeiro, e só então entrar na Fase 4 de verdade — instalar shadcn/ui, desenhar a navegação real (lista de ativos, histórico de cálculos salvos) e vestir os formulários de uma vez, em vez de estilizar um por um sem ainda saber todos os inputs que a navegação final vai precisar acomodar.

**Etapas**:
- [x] 4.1 — Tela: lista de ativos acompanhados — **concluída na Sessão 4** como "Saved Valuations": tickers distintos derivados da tabela `valuation` (sem tabela `asset` própria — ver Log de Sessões)
- [x] 4.2 — Tela: detalhe do ativo (histórico de cálculos salvos) — **concluída na Sessão 4**: comparação lado a lado dos campos comuns, detalhe fino (linha expansível "Assumptions" por cálculo) e **edição/exclusão em lugar** (coluna "Actions": View/Edit/Delete) — corrige um cálculo salvo sem virar linha nova, e remove um cálculo (cascade limpa a tabela de inputs específica sozinho)
- [x] 4.3 — Tela: cripto/indicadores — **vestida com shadcn/ui na Sessão 4** (mesmo painel de registro/placar, agora com Card/Select/Table/Badge em vez de HTML cru)
- [ ] 4.4 — Tela: alertas/zona de compra
- [x] 4.5 — Direção visual → **arejado, tipo dashboard** (Tailwind + shadcn/ui + TanStack Table), decidido na Sessão 1; **identidade de cor definida na Sessão 4** — dark navy + verde claro, inspirada no TruthID (ver Log de Sessões)

---

### Fase 5 — Monitoramento & Alertas

**Objetivo**: cadastrar premissas de compra por ativo e avisar o usuário quando o indicador entrar na zona configurada.

**Etapas**:
- [x] 5.1 — Cadastro de regra de alerta (tabela `alert_rule` polimórfica: `target_type` "stock_price" reusa o `fair_price` já calculado numa valuation salva, "crypto_indicator" reusa o signal GREEN/RED já calculado via `indicator_thresholds`; sem checagem periódica nem notificação ainda — só o CRUD)
- [x] 5.2 — Verificação periódica (background task no próprio app — `tauri::async_runtime::spawn` + `tokio::time::interval` a cada 5 min, reavalia contra dado já coletado, sem disparar o coletor; estado rastreado via tabela append-only `alert_event`)
- [x] 5.3 — Notificação nativa do SO, disparada só ao entrar em triggered. Validada de verdade na Sessão 12 (ver histórico da sessão) — precisou trocar `tauri-plugin-notification` por `notify-rust` direto com urgência crítica, já que KDE Plasma 6/Wayland suprime o popup de notificações normais vindas de apps sem `.desktop` registrado

---

### Fase 6 — Publicação (GitHub Público)

**Etapas**:
- [ ] 6.1 — Checklist de segurança final (ver "Diretriz de segurança") antes do primeiro push público
- [x] 6.2 — README explicando o projeto (em inglês, já que o repo é público). **Concluído na Sessão 25** — adiantado por pedido do dono do projeto ("por mais que eu ache que é cedo"), mesmo o resto da Fase 6 não tendo começado; README pode ser revisado/expandido mais pra frente. Seções: o quê/por quê, features, tabela de arquitetura (desktop/data-collector/contracts), build from source (Docker pro app, Foundry pros contratos), status (aponta pro `PROJECT_STATE.md` pra detalhe), segurança (keyring, DB gitignored, não auditado), license
- [x] 6.3 — LICENSE. **Concluído na Sessão 25** junto com o README — MIT, mesmo texto/copyright (`masterlxz`, 2026) já usado no TruthID, pra consistência entre os dois projetos do mesmo dono
- [x] 6.4 — `git init` + primeiro commit. **Já estava satisfeita, corrigido na Sessão 26**: o checklist nunca tinha sido marcado, mas o repo já tinha `git init`/commits desde a Fase 0/Sessão 1 e histórico publicado em `origin` (GitHub) e `gitea` bem antes desta sessão — confirmado ao rodar `git push && git push gitea` (o range enviado foi só os commits novos, `81d77ac..7cc07d6`, não o histórico inteiro, prova de que o remoto já tinha tudo até `81d77ac`)
- [ ] 6.5 — **Ideia registrada na Sessão 10** (não planejada em detalhe ainda, só anotada): aviso de atualização disponível — um indicador discreto num cantinho da tela quando existir uma versão nova do app. Candidato natural: `tauri-plugin-updater` (checa contra um manifesto de release, ex. GitHub Releases) — só faz sentido depois que 6.1–6.4 existirem (precisa ter release/versionamento publicado pra ter contra o que comparar)

---

### Fase 7 — Chat de IA Integrado (ideia levantada na Sessão 10, em andamento)

**Objetivo**: um chat de IA dentro do próprio app, num painel lateral flutuante (sobrepõe a tela atual, não é uma aba nova), onde o usuário usa a própria chave de API (Gemini/Claude/ChatGPT) pra tirar dúvidas sobre suas valuations e alertas salvos ("cara, essa valuation deu estranho, por quê?"). O chat tem acesso **só de leitura** ao banco (sem criar/editar nada) e a um "repertório" fixo de como os modelos do sistema funcionam. Histórico da conversa vive em memória enquanto o app está aberto — reseta só quando o app fecha, não quando o painel é fechado/reaberto.

Planejado com `/plan` na Sessão 10 (pesquisa real contra a doc do Gemini via `WebFetch`, e contra a doc/discussão do GitHub do Tauri sobre storage seguro — Stronghold, a opção mais óbvia, está sendo **descontinuada** na v3, então não é a escolha certa pra começar algo novo agora). Escopo combinado com o usuário: implementar em fatias, começando só pelo Gemini funcionando de ponta a ponta; Claude e OpenAI ficam desenhados na abstração mas com erro claro de "ainda não implementado" até uma sessão futura continuar.

**Decisões já tomadas com o usuário** (Sessão 10, antes de qualquer código):
- Painel lateral flutuante (shadcn `Sheet`, ainda não instalado no projeto — hoje só tem `badge`/`button`/`card`/`input`/`label`/`select`/`table`/`tabs`), não uma aba nova na `Tabs` existente
- Acesso ao banco **só leitura** por enquanto (sem criar valuation/disparar coletor via chat)
- Chave de API guardada no keyring do SO (não em texto puro em lugar nenhum, nunca vai pro SQLite nem pro git) — **risco em aberto**: o Linux usa Secret Service via D-Bus (mesmo barramento que a notificação nativa da Fase 5.3 precisou), mas ter o barramento montado não garante que exista um keyring daemon do outro lado dentro do container Docker de dev. Tratamento combinado: tentar de verdade, e se não achar um keyring disponível, mostrar erro claro na hora de salvar a chave em vez de travar silenciosamente — só descobrimos testando de verdade
- Histórico reseta ao fechar o **app**, não o painel — state React vive acima do `Sheet`, sobrevive abrir/fechar

**Etapas** (cada uma pode ser uma sessão/fatia separada):
- [x] 7.1 — Storage seguro da chave: crate `keyring` (v4, não `tauri-plugin-stronghold`, que está sendo descontinuado) chamada direto de comandos Tauri novos (`store_api_key`/`get_api_key_status`/`delete_api_key` — a chave em si nunca volta pro frontend, só um booleano "tem chave guardada"; já cientes de provedor, `SUPPORTED_PROVIDERS = ["gemini"]` por enquanto). Testado de verdade contra o container de dev (roundtrip + persistência entre processos separados via KWallet real do host, pelo mesmo D-Bus da Fase 5.3) **antes** de qualquer UI (Sessão 12) — maior risco técnico da feature inteira, resolvido
- [x] 7.2 — Cliente HTTP do Gemini em Rust: `reqwest` novo no `Cargo.toml` (nenhum outro comando fazia chamada HTTP de fora até aqui, só subprocess Python — primeira vez que o Rust fala com uma API de terceiro direto), testado de ponta a ponta contra a API real do Gemini (Sessão 13)
- [x] 7.3 — Contexto de leitura do banco: `ask_gemini` agora busca `valuation` (todas) + `alert_event` (últimos 20), formata como texto compacto e concatena com um repertório fixo (escrito à mão) explicando os 7 modelos de valuation e o score cripto — vira o `system_instruction` de cada chamada ao Gemini. Testado de ponta a ponta contra o banco real e a API real do Gemini (Sessão 14)
- [x] 7.4 — UI do painel: shadcn `Sheet` instalado via `npx shadcn add sheet` (gerou só `src/components/ui/sheet.tsx`, nenhuma dependência npm nova — o Radix Dialog por baixo já vinha junto do pacote unificado `radix-ui`), pasta nova `desktop/src/chat/` (`types.ts`, `ApiKeyGate.tsx`, `ChatToggleButton.tsx`, `ChatPanel.tsx`), estado do histórico e do aberto/fechado levantado pra `App.tsx` via `useState` simples (sem Context — só dois consumidores diretos, prop-drilling de um nível só) pra sobreviver abrir/fechar o painel, resetando só quando o app reinicia. Planejado com `/plan` (dois agentes `Explore` levantaram convenções exatas do frontend — imports relativos `../types` pros arquivos de feature, padrão `useMutation`/`useQuery` + `AppError`, estilo shadcn "radix-nova" — antes de escrever código) e testado de ponta a ponta pelo próprio usuário no app real rodando via `dev.sh`: abriu o painel sem chave salva (gate apareceu), salvou uma chave real do Gemini (gate sumiu, chat apareceu), mandou uma pergunta real e recebeu resposta usando o contexto do banco (Sessão 15)
- [x] 7.5 — Abstração multi-provider: novo `domain::chat_provider::Provider` (enum `Gemini`/`Claude`/`OpenAi`, `parse`/`as_str`, testado com 3 testes unitários) substitui a lista solta `SUPPORTED_PROVIDERS` que só aceitava "gemini". `commands/gemini.rs` renomeado pra `commands/chat.rs`; o comando público virou `ask_ai(provider, db, history)` (era `ask_gemini(db, history)`), com um `match` que chama o Gemini de verdade pro braço `Provider::Gemini` e devolve o novo erro tipado `AppError::ProviderNotImplemented` pros braços `Claude`/`OpenAi` — distinto de `UnknownProvider` (que continua existindo pra string que não é nenhum dos 3 IDs). `store_api_key`/`get_api_key_status`/`delete_api_key` passaram a aceitar os 3 IDs também (decisão consultada e aceita: liberar o storage pros 3 desde já, mesmo sem o HTTP client de Claude/OpenAI existir ainda). Planejado com `/plan` antes de codar (leitura direta de `gemini.rs`/`api_key.rs`/`error.rs`/`ChatPanel.tsx` em vez de agentes `Explore`, já que os arquivos são pequenos e já tinham sido lidos na sessão). `cargo test --lib` (48/48, incl. os 3 novos de `chat_provider`) e `npx tsc --noEmit` limpos; teste temporário (`#[test]`, escrito e removido) confirmou roundtrip real no keyring do host pra `claude`/`openai`. **Testado de ponta a ponta pelo usuário no app real** (`./desktop/dev.sh`): abriu o painel (chave Gemini já salva de sessão anterior), mandou uma pergunta real, resposta veio certa via `ask_ai` — confirma que o dispatcher genérico não quebrou o caminho do Gemini (Sessão 16)
- [x] 7.6/7.7 — **Claude e OpenAI implementados na Sessão 21**, os dois de uma vez (mesmo formato de request, fez sentido fazer junto). `ask_claude_api` (`api.anthropic.com/v1/messages`, header `x-api-key` + `anthropic-version: 2023-06-01`, `system` como campo próprio) e `ask_openai_api` (`api.openai.com/v1/chat/completions`, header `Authorization: Bearer`, sem campo `system` separado — vira a primeira mensagem do array) — a pergunta em aberto da 7.7 sobre migrar pra Responses API não foi revisitada, ficou em Chat Completions mesmo (mais simples, suficiente pra um chat de pergunta e resposta sem tool-calling ainda). As duas convertem o histórico compartilhado `GeminiContent` (role `"user"`/`"model"`) pro formato de cada API (`"user"`/`"assistant"`) via uma função `gemini_role_to_assistant_style` reaproveitada pelas duas. Erros novos no `error.rs`: `ClaudeApi`/`OpenAiApi`, mesmo molde do `GeminiApi` já existente. `AppError::ProviderNotImplemented` **removido** (não sobrou nenhum branch que o usasse, os 3 providers têm cliente real agora).
  - **Escolha de modelo virou parâmetro, não constante fixa**: `ask_ai(provider, model, db, history)` ganhou o campo `model` (o Gemini também passou a receber o model por parâmetro em vez do `GEMINI_MODEL` fixo, pra ficar simétrico aos outros 2). Decisão consultada com o usuário: em vez de eu escolher um modelo default pra cada provider e travar, o `ChatPanel.tsx` ganhou um seletor de provider (`Select` do shadcn, já usado noutras telas) + um campo de texto livre pro modelo (mesmo padrão do campo "Coin" do Crypto Score), com um valor sugerido por provider (`gemini-3.1-flash-lite`, `claude-haiku-4-5`, `gpt-5-mini` — todos tier barato/rápido, mesmo raciocínio do Gemini já usado: é um widget de pergunta rápida sobre dado já salvo, não um agente de código). Trocar de provider já reseta o campo de modelo pro sugerido daquele provider. `ApiKeyGate.tsx` também parou de ter "gemini" fixo — recebe `provider` como prop e mostra o rótulo/placeholder certo (Claude usa chave `sk-ant-...`, OpenAI `sk-...`).
  - `cargo check`/`cargo test --lib` (52/52) e `npx tsc --noEmit` limpos. **Teste contra API real ainda pendente** — usuário não tinha chave de teste de Claude/OpenAI à mão nesta sessão (o padrão de teste seguro via arquivo no scratchpad + `--env-from-file`, já usado no Gemini na Sessão 13, está pronto pra usar assim que houver uma chave real disponível). Até lá, o código está verificado por `cargo check`/tipos e revisão manual da forma exata dos dois requests (conferida contra a doc oficial), mas não confirmado ponta a ponta contra a API de verdade.
- [ ] 7.8 (ideia levantada na Sessão 15) — Conversas salvas, múltiplas, nomeadas. **Absorvida pela ideia maior da 7.10 (Sessão 20, ver abaixo)** — vira a etapa 7.10.2, dentro da tela cheia nova, em vez de um item isolado do widget flutuante atual
- [ ] 7.9 (ideia levantada na Sessão 16, logo depois do smoke test da 7.5) — Página de configurações de verdade com múltiplas chaves nomeadas por provider + redesenho do painel de chat. Duas perguntas de escopo já esclarecidas com o usuário na Sessão 16: (1) "quantas APIs eu quiser, com nome de cada uma" significa **várias chaves por provider** (ex: 2+ chaves de Gemini, cada uma com um apelido) — não é só um apelido pro provider inteiro, então o storage deixa de ser "1 chave = 1 provider" (`keyring::Entry::new(service, provider.as_str())`) e vira uma lista de entradas nomeadas; (2) a tela mora numa **rota/página de Configurações nova**, fora das `Tabs` que já existem na tela principal, não uma aba a mais. Quebrada em 5 etapas (cada uma pode ser uma sessão/fatia separada, mesmo padrão da Fase 7 original — a ordem prioriza o ganho visual rápido e de baixo risco primeiro, depois a mudança de storage de maior risco técnico, só then a UI que depende dela):
  - [x] 7.9.1 — Redesenho do painel de chat — **concluído na Sessão 17**: `ChatPanel.tsx` trocou o `Sheet` (Radix Dialog, ocupava a tela inteira de cima a baixo) por uma `div` flutuante de tamanho fixo (360×672px, proporção ~9:16 ajustada com o usuário testando ao vivo), ancorada perto do `ChatToggleButton` (canto inferior direito), sem overlay/backdrop — mais parecido com um widget de chat (Intercom-style) do que um modal. Fechar com Escape precisou de um listener manual (`document.addEventListener`), já que deixou de vir de graça do Radix Dialog. Só CSS/layout, nenhuma mudança de backend
  - [x] 7.9.2 — Storage de múltiplas chaves nomeadas. **Concluída na Sessão 21**: tabela nova `ai_api_key` (`id`, `provider`, `name`, `created_at`) via migration SeaORM (`m20260716_005530_create_ai_api_key`) — o segredo continua só no keyring, agora sob username `"{provider}:{id}"` (`api_key.rs::keyring_username`) em vez do provider puro, então renomear só toca a coluna `name`, nunca o keyring. `store_api_key`/`get_api_key_status`/`delete_api_key` substituídos por `create_api_key(provider, name, key) -> id`, `list_api_keys() -> Vec<ApiKeySummary>` (nunca devolve o segredo), `rename_api_key(id, name)`, `delete_api_key(id)`; novo `read_api_key_secret(db, key_id)` (não é comando Tauri, só usado pelo `ask_ai`). **Migração automática das chaves antigas** (decisão consultada com o usuário): `list_api_keys` detecta, na primeira chamada, uma chave ainda no esquema antigo (`username = provider` puro) sem nenhuma linha nova ainda pra aquele provider, cria uma linha `"Default"` e migra o segredo pro novo username — idempotente, sem perder a chave do Gemini já configurada desde a Sessão 15
  - [x] 7.9.3 — `ask_ai` concluída na Sessão 21 junto com a 7.9.2: passou a receber `key_id: i32` em vez de `provider: String` — busca a linha em `ai_api_key` via `read_api_key_secret` pra descobrir o provider e o segredo, despacha pro mesmo `match` de `Provider` que a 7.5/7.6/7.7 já criaram
  - [x] 7.9.4 — Página de Configurações, concluída na Sessão 21. **Decisão de arquitetura consultada com o usuário**: sem lib de rotas — app desktop de janela única não ganha nada com URL/histórico de navegador. `App.tsx` ganhou um `useState<"main" | "settings">` que troca a tela inteira (mesmo padrão do toggle form/saved já existente dentro da aba Valuation, só que no nível do App), acionado por um ícone de engrenagem ao lado da lista de abas. `settings/SettingsPage.tsx` novo: barra lateral com só a seção "IA" por enquanto (sem abstração de registro de seções — seria over-engineering pra 1 item só), conteúdo lista chaves agrupadas por provider, formulário de nova chave, rename inline (clique no nome vira input) e delete com o mesmo padrão de confirmação dupla já usado no `SavedValuationsPanel` (`confirmingDeleteId`, botão vira "Confirm?" destructive no primeiro clique)
  - [x] 7.9.5 — Seletor de chave no chat, concluído na Sessão 21: `ChatPanel.tsx` trocou o seletor de provider (da 7.6/7.7) por um seletor de chave nomeada (`Select` populado por `list_api_keys`, rótulo "{nome} ({provider})"), troca a qualquer momento sem resetar o histórico da conversa (requisito já dado antes: "que seja possível alternar na conversa"). Campo de modelo (texto livre) mantido, só a sugestão de modelo padrão passou a vir do provider da chave escolhida em vez de um seletor de provider próprio. `ApiKeyGate.tsx` (gate de "nenhuma chave ainda") também virou provider-agnóstico — pede pra escolher o provider inline e cria a primeira chave chamada "Default" via `create_api_key`
  - **Planejada com `/plan` (EnterPlanMode)** antes de codar, dado o tamanho (migration+entity, reescrita de 4 comandos, troca de esquema do keyring, `ask_ai` mudando de assinatura, página nova). Um agente `Explore` levantou a estrutura de navegação real do `App.tsx`, confirmou que não existe lib de rotas instalada, e trouxe o padrão exato de migration/entity/registro de comando a seguir antes de qualquer código ser escrito. `cargo check`/`cargo test --lib` (52/52) e `npx tsc --noEmit` limpos a cada etapa; `sea-orm-cli migrate up` + `generate entity` + `chown` rodados de verdade contra o banco real. **Testado de ponta a ponta pelo usuário no app real** rodando via `dev.sh` (o `tauri dev` recompilou e reabriu a janela sozinho ao detectar as mudanças no Rust): confirmou a chave do Gemini migrando sozinha pra "Default", criação de chave nova, rename, delete com confirmação dupla, e envio de mensagem real pelo chat usando o seletor de chave — "deu boa, testei tudo e funcionou"

- [x] 7.10 (ideia grande levantada na Sessão 20) — **Chat em tela cheia, estilo ChatGPT**, como uma segunda experiência de chat dentro do app, separada do widget flutuante atual (que **continua existindo do jeito que está hoje**, pra perguntas rápidas — não é substituído). **Completa**: alicerce (7.10.1/7.10.2/7.10.5) na Sessão 26, contador de tokens (7.10.3) e proposta de valuation com aprovação humana (7.10.4) na Sessão 27 — ver detalhe completo logo abaixo dos itens. Requisitos trazidos pelo usuário, em ordem de menção:
  - [x] 7.10.1 — Tela/rota dedicada em tela cheia. **Concluída na Sessão 26**: sem lib de rotas (mesma decisão da 7.9.4) — `AppView` ganhou `"chat"`, mesmo padrão `useState`+branch condicional de `"settings"`, novo ícone (`MessageSquareIcon`) ao lado da engrenagem
  - [x] 7.10.2 — Conversas salvas, múltiplas, nomeadas — absorve a ideia da antiga 7.8 (ver acima). **Concluída na Sessão 26**: tabelas `ai_conversation`/`ai_message`, primeira escrita no banco feita pelo chat (reverte a restrição só-leitura da Sessão 10 só pra essa parte — salvar a conversa em si, não criar/editar valuations, isso continua sendo só a 7.10.4)
  - [x] 7.10.3 — Contador de tokens visível pro usuário, pra ter noção de custo/uso. **Concluída na Sessão 27**: `generate_reply` (e os 3 `ask_*_api`) passam a devolver `(texto, TokenUsage)` em vez de só o texto, lendo `usageMetadata` (Gemini)/`usage` (Claude/OpenAI) reais de cada provedor — nenhuma chamada extra, nenhuma estimativa local. Persistido em `ai_message.input_tokens`/`output_tokens` (nova migration, só na mensagem do modelo, não na do usuário). Frontend mostra os dois níveis pedidos pelo usuário: total acumulado da conversa no cabeçalho do `ChatScreen.tsx`, e custo por mensagem no rodapé de cada resposta (`MessageTokenUsage` novo em `thread.tsx`, lendo `metadata.custom` que o `assistant-ui` já suporta nativamente). Widget flutuante não mudou — fora do escopo pedido
  - [x] 7.10.4 — **A IA passa a poder propor criar uma valuation nova** (não editar/apagar — escopo reduzido de propósito, perguntado direto ao usuário). **Concluída na Sessão 27**, planejada com `/plan` dedicado (3 agentes `Explore` + 1 `Plan` em paralelo, mais verificação real via `WebFetch` contra a doc de tool-calling dos 3 provedores). Nova tool genérica `propose_valuation` declarada nos 3 provedores (`chat.rs` ganhou `AiOutcome`/`ToolCall`, shape de tool diferente por provedor — Gemini `functionDeclarations`, Claude `input_schema`, OpenAI `tools[].function` com `arguments` como string). A IA nunca escreve sozinha: a proposta vira uma linha `ai_valuation_proposal` (nova tabela, `pending`/`approved`/`rejected`) presa à mensagem que a exibe, renderizada no chat como um card próprio (`ValuationProposalCard.tsx`, no estilo do `ValuationResult.tsx` manual) com botões Criar/Descartar — só o clique explícito do usuário roda a escrita real (`respond_to_valuation_proposal`, reaproveitando o despacho por-string-de-modelo já usado em `update_valuation`). Mecanismo de aprovação humana do `assistant-ui` (`onRespondToToolApproval`/`approval`) confirmado funcionando com `useExternalStoreRuntime` (não é exclusivo do `useLocalRuntime`). Decisões de simplicidade deliberadas: sem round-trip de volta pro modelo depois da aprovação (mensagem sintética "✅ Valuation criada..." em vez disso) e proposta inválida é beco sem saída sem retry automático. Achado real no meio da implementação: `GeminiPart.text` precisou virar `Option<String>` (resposta de function-call do Gemini não traz esse campo). Testado ponta a ponta pelo usuário no app real: proposta aprovada (valuation real criada no banco, conferido via `sqlite3`), proposta descartada (nada escrito), e reload da conversa confirmando que o card aprovado não volta a mostrar os botões (estado vem do banco). `cargo test --lib`: 77/77
  - [x] 7.10.5 — Troca de API/provider/modelo dentro da tela cheia — reusa a mesma ideia da 7.9.5. **Concluída na Sessão 26**, mesmo seletor de chave/campo de modelo livre, agora no cabeçalho da tela nova
  - [~] 7.10.6 — **Pesquisa real feita na Sessão 21** (via `WebSearch`/`WebFetch`, não por memória) — decisão de usar ou não ainda em aberto, **não decidir sozinho** (usuário pediu só a pesquisa nesta sessão, não a execução). Achados:
    - **`assistant-ui`** ([github.com/assistant-ui/assistant-ui](https://github.com/assistant-ui/assistant-ui)) — candidato forte. Lib React/TypeScript, MIT, ativa de verdade (11k stars, push no dia da pesquisa, 1094 forks, mantida por empresa YC — conferido via `gh api repos/assistant-ui/assistant-ui`). Suporta Gemini/Claude/OpenAI nativamente. **`useLocalRuntime`**: o adapter que fala com o "modelo" é só uma função async arbitrária, sem exigir servidor HTTP — encaixa direto no `invoke()` do Tauri, sem precisar de ponte/sidecar Node. Resolve de graça os dois pedaços mais arriscados da 7.10: tool-calling com pausa-pra-aprovação já é conceito de primeira classe (`status: "requires-action"`, mapeia direto no requisito da 7.10.4) e múltiplas conversas ("threads") com persistência 100% customizável (não depende do "Assistant Cloud" pago deles — a gente escreveria nosso próprio adapter salvando no SQLite, mesmo padrão de sempre). Tem hook de contador de token (`useThreadTokenUsage`), mas só funciona se `ask_claude_api`/`ask_openai_api`/`ask_gemini_api` passarem a extrair e devolver o uso de tokens da resposta (hoje só devolvem o texto) — isso continuaria sendo trabalho nosso independente da lib. Sem servidor de terceiro envolvido: é só mais um pacote no `package.json`, roda dentro do bundle do próprio app, sem SaaS no meio — só precisaria de trabalho de tema pra combinar com o visual atual (vem sem estilo pronto, mas tem variante shadcn/ui, que o projeto já usa)
    - **`prompt-kit`** ([prompt-kit.com/chat-ui](https://www.prompt-kit.com/chat-ui)) — alternativa mais enxuta: só componentes de UI (input, lista de mensagem, markdown, streaming) construídos em shadcn/ui + Tailwind, sem resolver conversas múltiplas nem aprovação de tool-call sozinho — mais "peças soltas" que framework completo
    - **LibreChat/Chatbot UI** (clones completos de ChatGPT) — descartados: são aplicações full-stack que esperam rodar servidor Node próprio, contra a arquitetura atual (tudo via Rust/Tauri, sem servidor Node)
    - Claude recomendou `assistant-ui` quando perguntado diretamente (a aprovação de ação é a parte mais arriscada da 7.10 e a lib já resolve isso testado por terceiros, sem risco de arquitetura) — usuário ainda não decidiu, só registrou a pesquisa
    - **Decisão tomada na Sessão 26**: perguntado diretamente, o usuário escolheu **`assistant-ui`** (framework) e escopar a sessão só no **alicerce** (7.10.1/7.10.2/7.10.5), deixando 7.10.3 (contador de token) e 7.10.4 (IA escrevendo valuations) pra sessões futuras dedicadas — 7.10.4 principalmente por já estar marcada acima como o maior risco técnico da fase, merecendo `/plan` próprio

**Sessão 26 (2026-07-21) — alicerce da 7.10 implementado**: planejado com `/plan` (3 agentes `Explore` em paralelo levantaram o widget flutuante existente incl. `ask_ai`/contexto de leitura do banco, o padrão de navegação `AppView`/`SettingsPage` e libs já instaladas, e os comandos de criação/edição de valuation como referência pro desenho futuro da 7.10.4). Pesquisa real contra a doc do `assistant-ui` (`www.assistant-ui.com/docs`, via `WebFetch`) antes de codar, não só memória da Sessão 21 — confirmou a API exata de `useExternalStoreRuntime`/`ChatModelAdapter`/`RemoteThreadListAdapter`.

- **Decisão de arquitetura**: usar `useExternalStoreRuntime` do `assistant-ui` (runtime dono só da conversa atual) em vez do `RemoteThreadListAdapter` (API maior, pensada pra multi-thread 100% no molde do framework). A lista de conversas (criar/selecionar/renomear/apagar) foi construída do jeito de sempre do projeto — CRUD fino em Rust + SQLite, mesmo padrão de `SettingsPage.tsx`/`SavedValuationsPanel.tsx` — deixando o `assistant-ui` responsável só pela parte que economiza trabalho de verdade (lista de mensagens, markdown, composer, auto-scroll, via o componente `Thread` escafoldado).
- **Backend**: migration nova (`ai_conversation`: título, `key_id` nullable com `ON DELETE SET NULL`, `model`, timestamps; `ai_message`: `conversation_id` com `ON DELETE CASCADE`, `role`/`content`/`created_at`) — `.schema` real conferido, sem o bug de coluna com dígito (nenhuma aqui termina em dígito). `ask_ai` (`commands/chat.rs`) refatorado: a lógica de resolver chave+montar contexto+chamar o provider virou `generate_reply` (função solta, não comando), reaproveitada pelos comandos novos — evita duplicar o `SYSTEM_REPERTOIRE`/contexto entre os dois pontos de entrada. `commands/conversation.rs` novo: `list_conversations`/`create_conversation`/`rename_conversation`/`delete_conversation`/`get_conversation_messages`/`send_conversation_message` (este último insere a mensagem do usuário, chama `generate_reply`, insere a resposta, e atualiza `key_id`/`model`/`updated_at` da conversa). `cargo check`/`cargo test --lib`: **68/68**, sem regressão.
- **Frontend**: `npm install @assistant-ui/react @assistant-ui/react-markdown remark-gfm zustand` + `npx shadcn@latest add @assistant-ui/thread` (escafoldou `thread.tsx` e componentes de apoio — `tooltip`/`avatar`/`collapsible`/`dialog`, `tw-shimmer` como dependência nova, tudo aditivo no `index.css`, tema dark+verde intacto). **Achado**: o `thread.tsx` escafoldado usa `Object.hasOwn` (ES2022) — `tsconfig.json` precisou subir de `target"ES2020"` pra `"ES2022"` (`lib` junto), sem impacto prático no WebKitGTK do Tauri. Pasta nova `desktop/src/chat-full/`: `ConversationSidebar.tsx` (lista+rename inline+delete com confirmação dupla, mesmo idioma de `SettingsPage`), `useConversationRuntime.ts` (monta o `useExternalStoreRuntime`, com atualização otimista da mensagem do usuário no cache do TanStack Query antes da resposta chegar — mesmo padrão do próprio exemplo mínimo da doc do `assistant-ui`), `ChatScreen.tsx` (layout sidebar+thread, seletor de chave/modelo igual ao `ChatPanel.tsx`). `App.tsx` ganhou a view `"chat"`. `npx tsc --noEmit` limpo.
- **Testado ponta a ponta pelo usuário no app real** (`./desktop/dev.sh`, rebuild do zero com as crates novas — `alloy-*`, `ark-*`, `secp256k1` etc. — baixadas e compiladas dentro do container): confirmado funcionando ("tá funcionando perfeitamente, mt massa"). Observação do próprio usuário: tem espaço pra melhorar UX, deixado pra depois, sem detalhar ainda.

---

### Fase 8 — Sync Multi-Dispositivo via TruthID + IPFS (ideia levantada na Sessão 11, desenho revisado na Sessão 22, não iniciada)

**Objetivo**: sincronizar valuations/alertas salvos entre dispositivos (celular, outro PC) de forma descentralizada — sem servidor próprio do Practice Valuation — reaproveitando a identidade e a infraestrutura já existentes no TruthID (outro projeto do usuário, `~/Documents/workspace/truthid`).

**Sessão 18 (2026-07-14/15) — prova de conceito do canal de assinatura delegada, feita a partir de
uma sessão do TruthID**: `desktop/src-tauri/src/commands/truthid.rs` (descobre o TruthID Desktop
rodando na mesma máquina, faz handshake, manda 1 pedido de assinatura de teste sem efeito
econômico) + aba "TruthID Sync" (`desktop/src/truthid/TruthIdPanel.tsx`). Prova que o canal HTTP
local do TruthID (`local_signer_server.rs`/`sign_request.rs`, portas 47950-47954) funciona — não é
a Fase 8 em si, é a base que ela usa.

**Sessão 22 (2026-07-15) — três suposições do brainstorm original corrigidas, desenho revisado
registrado (só documentação, nenhum código tocado)**: as ideias abaixo (Sessões 11-96, preservadas
por histórico logo depois) assumiam (1) um Paymaster no TruthID cobrindo o gas de apps terceiros —
não existe, descartado deliberadamente lá desde a Sessão 52; (2) generalizar o `VaultRegistry`
(`identityId → CID`) pra caber múltiplos apps — rejeitado duas vezes pelo dono do TruthID (Sessões
95 e 102): o vault de senhas não muda, cada app terceiro traz e mantém o próprio contrato; (3) uma
session key escopada pra aprovação de assinatura — também descartada (Sessão 102) em favor de algo
mais simples que **já foi construído e testado**: o canal de sign-request acima, aprovação humana
por clique único, sem escopo/sessão. Ver `PROJECT_STATE.md` do TruthID, seção "Vault genérico
multi-app...", pra o histórico completo da correção.

**Desenho revisado (Sessão 22)**:
1. **Registro de CID — contrato próprio do Practice Valuation**, não o `VaultRegistry` do TruthID. Mesmo formato de dado (`{cid, contentHash, version, updatedAt}`), mas gated por `msg.sender` (a smart account que chamou) em vez de `identityId` — como todo device do usuário assina pela mesma smart account, qualquer um escreve no mesmo slot sem precisar de conceito de identidade do TruthID no contrato em si.
2. **Escrita do CID — reaproveita o canal de assinatura delegada como já está** (`discover` → `handshake` → `POST /truthid/v1/sign-request`, `commands/truthid.rs` sem mudança de desenho). Cada dispositivo roda sua própria instância do TruthID local, assinando com a autoridade daquele device — não precisa de pareamento novo nem canal P2P entre devices.
3. **Leitura do CID — não precisa de assinatura**, é `eth_call` público via qualquer RPC.
4. **Chave de criptografia — depende de uma rota nova no TruthID, `/truthid/v1/sign-message`, ainda NÃO implementada** (pendência cross-repo, registrada no `PROJECT_STATE.md` do TruthID, Sessão 106). Mesmo padrão do `/sign-request`: app terceiro manda `{appName, purpose}`, TruthID monta uma mensagem padronizada com domain separation, assina via `personal_sign` com aprovação humana, devolve a assinatura — o app deriva a chave simétrica localmente (HKDF) sem o TruthID nunca saber pra que serve. **Decisão explícita do dono do projeto**: essa rota tem que ser genérica pra qualquer app terceiro, não um privilégio hardcoded pro Practice Valuation — o Practice Valuation é só o primeiro caso de uso real que força o desenho a existir.
5. **Pinning no IPFS — também pensado como capacidade genérica nova do TruthID, `/truthid/v1/pin`, ainda NÃO implementada** (ideia trazida pelo dono do projeto nesta sessão, também registrada no `PROJECT_STATE.md` do TruthID, Sessão 106): já que o TruthID é a porta única de qualquer app descentralizado construído sobre ele, ele pode oferecer os providers de pinning que o usuário já configurou (`ipfs.rs`/`pin_vault`, sem mudar a lógica) como serviço — app terceiro manda o blob já cifrado, TruthID faz o upload com a própria config e devolve só `{cid, contentHash}`, sem nunca expor a API key. Estritamente opcional (em aberto o modelo de consentimento — aprovação por chamada vs. única por app, já que pinning pode ser bem mais frequente que assinar transação). **Fallback caso essa rota não exista ainda ou o app prefira não depender dela**: módulo próprio no Practice Valuation reaproveitando a mesma abstração `PinningProvider{name, kind: "kubo"|"psa", endpoint_url, api_key}` + `pin_vault()` do TruthID (cópia adaptada do padrão, crates separadas), Pinata (PSA, free tier) como default, Kubo local como opção.
6. **Correção feita ainda na Sessão 22, depois de uma pergunta do dono do projeto — o desenho acima (itens 1-5) só funciona quando o Practice Valuation e o TruthID rodam na MESMA máquina**, porque `local_signer_server.rs` escuta estritamente em `127.0.0.1` (nunca a rede), por desenho de segurança do TruthID. Cenário real que quebra isso: Practice Valuation só no computador, TruthID só instalado no celular — hoje **não existe canal nenhum** entre os dois nesse caso. **Não é mais tratado como resolvido** (uma versão anterior desta seção, ainda na Sessão 22, chegou a marcar essa questão como "não é mais necessária" — estava errada, corrigida ainda na mesma sessão). Fica **em aberto**, com um caminho claro pra resolver: o TruthID já tem esse problema resolvido pra outro caso de uso (extensão de navegador, Fase 13.9 dele) — dois transportes, tentados em paralelo:
   - **Descoberta na mesma rede local (LAN)**: `mobile/lib/services/vault_lan_server_service.dart` sobe um servidor HTTP efêmero (portas 47850-47854) que serve exatamente 1 request e fecha.
   - **Dead-drop assíncrono via IPFS/IPNS**: funciona em redes diferentes, mas com propagação mais lenta (~1-2min) — o dispositivo com o TruthID publica um blob cifrado sob um nome IPNS derivado deterministicamente do QR; o outro lado recalcula o mesmo nome e resolve.
   - **Segurança confirmada no código real, não é "qualquer dispositivo na rede consegue"**: o QR carrega um `sessionId` de 128 bits (imprevisível, não adivinhável varrendo a rede), o servidor LAN devolve 404 uniforme pra qualquer path errado (não vaza sinal de "quase certo"), e o payload em si é cifrado via ECIES pra uma chave pública efêmera que só existe no QR — quem não escaneou o QR de verdade (proximidade física/visual) não consegue nem achar nem decifrar o blob, mesmo estando na mesma rede. TTL curto (3min) e resposta única (o servidor atende 1 request e morre) limitam a janela de exposição.
   - **Ressalva honesta**: essa mesma peça de criptografia (ECIES no pareamento) teve um bug real que passou batido por várias sessões do TruthID antes de ser achado testando contra hardware de verdade (Sessão 99 de lá) — não é motivo pra não usar o padrão, mas reforça que validar isso em hardware real antes de confiar é obrigatório, não opcional.
   - **Ainda não decidido**: estender as rotas genéricas (`/sign-message`, `/pin`, e talvez até `/sign-request`) pra também aceitar esses dois transportes (hoje só funcionam loopback) é trabalho novo do lado do TruthID, não desenhado em detalhe — fica registrado como pendência em aberto, junto com as outras duas rotas (Sessão 106 do `PROJECT_STATE.md` do TruthID).

**Fatias propostas (cada uma uma sessão futura, mesmo padrão das outras fases)**:
- [~] 8.1 — contrato próprio (deploy + leitura pública, sem escrita ainda). **Código pronto desde a Sessão 24** (`contracts/src/SyncRegistry.sol` + `sync_registry.rs`, 10/10 + 68/68 testes) — falta só o deploy real em Base Sepolia, pendente de revisão do dono do projeto
- [ ] 8.2 — escrita via canal de assinatura delegada já existente (reaproveita `truthid.rs`, só cobre o caso "TruthID na mesma máquina"). Não iniciada, mas não bloqueada — só depende do deploy da 8.1 acontecer primeiro (precisa do endereço real do contrato)
- [ ] 8.3 — pinning: começa pelo fallback próprio (Pinata default + Kubo opcional) pra não ficar bloqueado; migra pra `/truthid/v1/pin` quando essa rota existir do lado do TruthID. **Deixou de estar bloqueada** — `/truthid/v1/pin` já foi implementado do lado do TruthID (Sessões 119-121, 2026-07-17); ainda não consumida aqui
- [ ] 8.4 — criptografia real via `/truthid/v1/sign-message`. **Deixou de estar bloqueada** — a rota e os dois transportes (LAN+dead-drop) já existem do lado do TruthID (Sessões 108-109 de lá); ainda não consumida aqui
- [ ] 8.5 — integração fim a fim: publish/pull automático, merge por replay causal (`valuation`/`alert_event` já são append-only, formato natural pra isso sem precisar desenhar CRDT do zero). Não iniciada
- [x] 8.6 (parcial) — suporte a "TruthID num dispositivo diferente" via LAN/dead-drop, no molde da 13.9 do TruthID. **Validado ponta a ponta pro par `/sign-request`** na Sessão 23 (celular físico real). O mesmo transporte já existe do lado do TruthID pra `/sign-message` também (Sessões 108-109), mas ainda não foi exercitado por nenhum app requisitante de verdade — só quando a 8.4 consumir `/sign-message` isso fica provado igual

**Nenhuma fatia restante depende mais de pendência cross-repo** — as duas que bloqueavam (rota `/sign-message` com transporte cross-device, e `/truthid/v1/pin`) foram resolvidas do lado do TruthID entre 2026-07-16 e 07-17. O que falta agora é só trabalho neste repo: revisar e deployar o contrato da 8.1, depois seguir em ordem (8.2 → 8.3/8.4 → 8.5).

**Sessão 23 (2026-07-16) — Practice Valuation vira o app requisitante de referência do transporte cross-device do `/sign-request`, fecha parte da 8.6**: até aqui, o canal de assinatura delegada (`commands/truthid.rs`) só tinha sido provado com TruthID Desktop na mesma máquina (Sessão 18) — nenhum app terceiro real tinha gerado o QR nem consumido o transporte LAN/dead-drop que o TruthID lado Mobile já expunha (Sessões 108-111 de lá, pro par `/sign-message`+`/sign-request`). Três commits, mesmo dia:
- `2db1eab` — Practice Valuation passa a gerar o QR (par de chaves ECIES efêmero + `sessionId` de 128 bits) e varrer a LAN nas mesmas portas que `RemoteSignerLanServer` do TruthID Mobile usa. Novos `ecies.rs` (porta do decrypt ECIES já testado em Rust no TruthID Desktop) e `lan_sweep.rs` (porta da varredura já em TS na extensão do TruthID).
- `2e81c52` — segundo transporte em paralelo: dead-drop assíncrono via IPFS/IPNS (`ipns_key.rs` recalcula o nome IPNS determinístico a partir do `sessionId`, HKDF+Ed25519+protobuf+CID+base36 — mesmo algoritmo já usado em Dart/TS do lado do TruthID; `dead_drop.rs` consulta o gateway público `ipfs.io`). LAN tenta a cada 2s, dead-drop a cada ~20s (bater num gateway público a cada 2s seria agressivo, e a propagação de IPNS já leva 1-2min) — o primeiro que responder decide.
- `81d77ac` — bugfix achado testando com celular físico de verdade (Samsung SM_S731B, TruthID Sessão 114): `rename_all = "camelCase"` em `TruthIdSignResult` resolvia a leitura do JSON que chega de fora mas quebrava a serialização de volta pro frontend Tauri/JS (que espera snake_case, mesmo padrão de `TruthIdHandshakeResult`). Corrigido separando em `TruthIdWireResult` (Deserialize, camelCase, só parsing interno) convertido via `From` pra `TruthIdSignResult` (Serialize, sem rename_all). Sem teste automatizado cobrindo isso — os campos são `Option<T>`, então uma chave ausente nunca dá erro, só vira `None` em silêncio.

Isso fecha, do lado do TruthID (Sessão 114 de lá), **a pendência "nenhuma troca ponta a ponta real foi observada"** que estava aberta desde a Sessão 108 — a primeira troca real com um app terceiro de verdade e um celular físico aconteceu nesta sessão. **O que ainda não mudou neste repo**: nenhum contrato de CID foi deployado (8.1), a chave de criptografia derivada de `/sign-message` ainda não é consumida por nenhum fluxo real de sync (8.4), e não há pinning (8.3) — só a camada de transporte/canal foi validada. Vale notar que, **do lado do TruthID** (fora deste repo, sessões 108-121, todas em 2026-07-16/17), tanto `/sign-message` quanto `/sign-request` já têm os dois transportes (LAN + dead-drop) prontos, e `/truthid/v1/pin` também já foi implementado nas 3 fatias (núcleo Rust, rota HTTP + comandos Tauri, tela de Settings) — ou seja, **as duas pendências cross-repo que bloqueavam 8.3/8.4/8.6 não existem mais**; falta só este repo de fato começar a consumi-las (8.1 contrato próprio, 8.3 integração real com `/pin`, 8.4 integração real com `/sign-message`).

**Sessão 24 (2026-07-21) — fatia 8.1 implementada (contrato + leitura), planejada com `/plan` (3 agentes `Explore` em paralelo levantaram a arquitetura Rust existente, as crates web3/crypto já presentes — nenhuma — e o `VaultRegistry` do TruthID como referência)**:
- **Debate antes de codar**: o dono do projeto perguntou se fazia mais sentido o TruthID hospedar um contrato genérico pra qualquer app terceiro, em vez de cada um trazer o próprio. Argumento contra centralizar, aceito: `/sign-message`/`/pin` precisam ser genéricos no TruthID porque dependem de um recurso que só ele tem (chave de device, credenciais de pinning); um contrato de registro não depende de nenhum segredo do TruthID — qualquer smart account chama qualquer contrato deployado, não importa quem fez o deploy. Além disso, um registro único-por-endereço compartilhado por vários apps colidiria (o mesmo endereço usado em apps diferentes sobrescrevendo o CID um do outro) sem uma dimensão extra de `appId` — complexidade que contratos por-app não têm, já que cada contrato já é o próprio namespace. Mesmo raciocínio, no fundo, da rejeição de generalizar o `VaultRegistry` (Sessões 95/102 do TruthID). Mantido o desenho original: contrato próprio, simples.
- **`contracts/` novo na raiz do repo** (Foundry, `forge-std` instalado como arquivos normais via `forge install --no-git`, não submodule — mesmo padrão do TruthID): `src/SyncRegistry.sol` (adaptado do `VaultRegistry.sol` do TruthID, mas **sem indireção de identidade** — gated direto por `msg.sender`, já que este contrato não depende de um `IdentityRegistry` próprio nem do TruthID). Struct `CidRecord{cid, contentHash, updatedAt, version, exists}`, `updateRecord`/`getRecord`/`hasRecord`, evento `RecordUpdated`. **Decisão deliberada**: `updateRecord` (escrita) já entra no contrato nesta fatia, mesmo a 8.1 sendo "só leitura" do lado do app — contratos não são upgradeable aqui, e redeployar só pra acrescentar a escrita na 8.2 seria desperdício de endereço/histórico. `test/SyncRegistry.t.sol` (10 testes, mesmo estilo `vm.prank`/`vm.expectRevert`/`vm.expectEmit` do `VaultRegistry.t.sol`) — `forge test`: **10/10 passando**. `script/DeploySyncRegistry.s.sol` pronto, mas **deploy real ainda não rodado** — passo manual combinado com o dono do projeto, que quis olhar o contrato de novo antes de rodar o `--broadcast` (gasta ETH de testnet). Foundry já estava instalado no host (mesma versão do TruthID, v1.7.1) — nenhuma mudança no Docker do `desktop/` foi necessária.
- **Decisão de ABI encoding, perguntada ao dono do projeto**: adicionar `alloy-primitives`+`alloy-sol-types` (v1.6.1) em vez de codar o encode/decode ABI à mão (como `ipns_key.rs` fez pra CID/protobuf) — projeto ainda não tinha nenhuma lib Ethereum no Rust. Escolhido `alloy` (moderna, sucessora do `ethers-rs` que está em manutenção) porque a macro `sol!` gera o encode/decode a partir da própria assinatura Solidity, cobrindo também a Fase 8.2 (escrita) de graça.
- **`desktop/src-tauri/src/sync_registry.rs`** (novo módulo, mesmo nível de `ecies.rs`/`lan_sweep.rs`/`ipns_key.rs`): `sol! { struct CidRecord {...} function getRecord(address) ...; error RecordNotFound(address); }` gera o encode/decode; `get_record(who)` monta o `eth_call` via `reqwest` (já dependência do projeto) contra `RPC_URL = "https://sepolia.base.org"` (pública, sem chave — mesmo endpoint que o ecossistema TruthID já usa como fallback) e um endereço de contrato ainda placeholder (`0x000...0`, preenchido só depois do deploy real). Revert `RecordNotFound` vira `Ok(None)`, não erro — "ainda não tem registro" é resultado esperado. 4 testes novos (`cargo test`): calldata tem seletor+endereço no formato certo, round-trip de encode/decode de um `CidRecord` completo, `parse_address` aceita/rejeita o esperado — **68/68 no total, sem regressão**.
- **`commands/sync_registry.rs`**: comando fino `get_sync_record(address) -> Option<CidRecordResponse>`, seguindo o padrão de `commands/bazin.rs`. `CidRecordResponse` é um tipo próprio (não o `CidRecord` gerado pela macro, que não implementa `Serialize` e usa tipos ABI como `U256`/`FixedBytes<32>`) — mesma razão do split `TruthIdWireResult`/`TruthIdSignResult` já usado em `commands/truthid.rs`.
- **Frontend**: `TruthIdPanel.tsx` ganhou uma seção nova (campo de endereço + botão "Read sync record"), mesmo padrão `useMutation`/`invoke` do resto do painel — só o suficiente pra provar a leitura visualmente quando o contrato existir de verdade; uma aba "Sync" própria fica pra quando 8.2/8.5 também existirem.
- **Validado sem o deploy**: `cargo check`/`cargo test --lib` (68/68) e `npx tsc --noEmit` limpos; `curl` direto contra `https://sepolia.base.org` confirmou o RPC respondendo (chain id `0x14a34` = 84532, Base Sepolia) e que um `eth_call` pro endereço placeholder (`0x0`) retorna `"0x"` (sem contrato, sem revert) — exatamente o caminho que o código já trata como erro de decodificação (`AppError::Rpc`), não um crash.
- **Pendência pra próxima sessão**: dono do projeto vai revisar o `SyncRegistry.sol` de novo antes do deploy real em Base Sepolia (`forge script script/DeploySyncRegistry.s.sol --broadcast`); depois disso, atualizar `SYNC_REGISTRY_ADDRESS` em `sync_registry.rs` e validar a leitura ponta a ponta (escrever 1 registro via `cast send`, ler pelo botão novo no app rodando).

<details>
<summary>Brainstorm original (Sessões 11-96), preservado por histórico — suposições corrigidas acima, não usar como referência de desenho</summary>

- **P2P direto entre devices foi descartado** — usuário não gosta da abordagem (conexão ao vivo entre dispositivos, NAT, complexidade). Transporte vira **assíncrono via IPFS**: cada device publica quando muda algo, os outros puxam quando abrem — sem exigir dois devices online ao mesmo tempo. (Isso continua valendo no desenho revisado.)
- Ideia original considerava generalizar o `VaultRegistry` do TruthID (`identityId + vaultKind/appId → VaultRef`) — **rejeitada pelo TruthID, ver desenho revisado acima**: Practice Valuation traz o próprio contrato.
- Ideia original assumia session key escopada + Paymaster pra assinatura delegada — **substituída pelo canal de sign-request genérico já implementado**, ver desenho revisado acima.
- Login/pareamento de device: discutido reaproveitar o fluxo de QR do TruthID (servidor HTTP local na mesma rede vs. polling on-chain) — a questão em si continua válida (ver item 6 do desenho revisado acima), só o mecanismo concreto mudou: não é mais "login" propriamente, é o mesmo transporte LAN/dead-drop que a Fase 13.9 do TruthID já usa pra extensão de navegador.

</details>

---

### Fase 9 — Tela de Pesquisa de Ação (Stock Lookup, ideia levantada na Sessão 20)

**Objetivo**: tela nova e separada (aba "Stock Lookup", ao lado de "Valuation" na navegação principal) pra pesquisar um ticker de ação e ver várias informações reunidas de uma vez — cotação, fundamentos (LPA/VPA/ROE/payout), dividendo médio 5 anos, DCF fundamentals, SMA 50/100/200 dias, CAGR de preço 5/10 anos, P/L, P/VP, a valuation já salva pra esse ticker (se existir) e um espaço de anotações livres. Só ações por enquanto — cripto (que já tem seu próprio dashboard, Fase 4.3) fica pra depois.

**Etapas**:
- [x] 9.1 — Concluída na Sessão 20:
  - `fetch_technicals` novo em `data-collector/sources/acoes_yahoo.py` (reusa o mesmo endpoint do Yahoo já usado por `fetch_quotes`/`fetch_dividends_avg`, `range=10y&interval=1d`) calcula SMA 50/100/200 dias e CAGR de preço 5/10 anos (`None` quando o ticker não tem histórico suficiente pra aquele cálculo — mesmo padrão de `payout`/`tax_rate`). Testado contra a API real (PETR4, MGLU3, WEGE3, RAIZ4, ticker inválido) antes de integrar no resto
  - Tabelas novas `stock_technicals` (SMA/CAGR, campos nullable) e `stock_notes` (anotação livre por ticker, upsert sem constraint de unicidade — app single-user local) via migration SeaORM. **Achado real**: o mesmo bug de conversão de nome de coluna da Sessão 5/6 (`avg_dividend5y` sem underscore, corrigido por `m20260710_220000_rename_avg_dividend5y_column`) se repetiu aqui — `DeriveIden` converteu `Sma50`/`Cagr5y` pra `sma50`/`cagr5y` (sem underscore antes do dígito). Corrigido **antes** de qualquer dado real ser gravado: `Alias::new("sma_50")` etc. explícito em vez das variantes do enum, `migrate down -n 2` + edição + `migrate up` de novo
  - `collect_stock_technicals` novo no `main.py`, chamado dentro do mesmo pipeline unificado que os formulários de valuation já usam (`run_stock_collector` continua servindo os dois — decisão consultada com o usuário: unificar a busca de histórico no Python, mas **não** disparar sempre — ver próximo item)
  - Aba nova `StockLookupPanel.tsx` (`desktop/src/stock-lookup/`), com busca "cache-aware": a primeira pesquisa de um ticker sem nada salvo no banco dispara o collector sozinha (`useEffect` com guarda por `useRef`, evita loop infinito se o ticker realmente não existir/não tiver dado); pesquisas seguintes só leem o banco; botão "Refresh data" força atualização a qualquer momento (mesmo padrão visual do botão "Run crypto collector" do Crypto Score)
  - P/L e P/VP calculados no frontend, não persistidos — derivados de `price`/`lpa`/`vpa` já existentes
  - Planejada com `/plan` (dois agentes `Explore` em paralelo levantaram os padrões existentes de navegação/painéis no frontend e de migrations/entities/commands/collector no backend antes de qualquer código) e testada de ponta a ponta: `cargo check` (app principal + crate `migration`) e `npx tsc --noEmit` limpos, `python main.py --ticker PETR4` rodado contra a API real depois da migration corrigida (linha real conferida em `stock_technicals`), app rodando via `dev.sh` — **testado pelo próprio usuário no app real, aprovado** ("mt boa, mt legal")

**Pendências levantadas pelo usuário na Sessão 20, pra uma sessão futura** (só registradas, nada planejado em detalhe ainda):
- [x] 9.2 — Mais indicadores na tela: dívida líquida/EBITDA, EV/EBIT, margem líquida. **Concluída na Sessão 20**: dos 3, 2 são conta pura no frontend (`StockLookupPanel.tsx`) em cima de campos que já existiam — `net_debt_ebitda = (total_debt − cash) / (ebit + depreciation_amortization)` e `ev_ebit = (preço × ações_em_circulação + total_debt − cash) / ebit`, tudo já em R$ milhões vindo da CVM, sem precisar converter unidade. O 3º (margem líquida) precisou de dado novo: `revenue` (receita líquida, conta "3.01" da DRE) adicionado em `cvm_dfp.py::fetch_dcf_fundamentals` — **verificado contra o zip real da CVM antes de codar**: conta "3.01" tem exatamente a mesma cobertura da "3.05" (EBIT), 436 de 436 empresas. Coluna nova `revenue` (nullable) em `stock_dcf_fundamentals` via migration (`m20260715_230504_add_revenue_to_stock_dcf_fundamentals`). Lucro líquido não ganhou extração própria — deriva de ROE × VPA × ações em circulação (dado que já existe), evitando puxar mais uma conta da CVM só pra isso. Testado ponta a ponta: extração isolada contra PETR4/WEGE3/MGLU3/ITUB4 reais (ITUB4/banco corretamente descartado, taxonomia diferente), `cargo check` nas duas crates, `npx tsc --noEmit`, `python main.py --ticker PETR4` real com a migration aplicada, conferência manual das 3 contas com os números reais da PETR4 (Net Debt/EBITDA ≈1,51x, EV/EBIT ≈6,0x, Net Margin ≈23,7%) — **confirmado pelo usuário no app real** ("deu boa, os 3 indicadores aparecendo certinho")
- [x] 9.3 — Gráfico de histórico de dividendos por mês e por ano. **Concluída na Sessão 20**: nova função `fetch_dividend_payments` em `acoes_yahoo.py` (`range=10y&interval=1d&events=div`, mesmo endpoint de `fetch_technicals`) casa cada pagamento com o fechamento do dia via `_closest_close` (generalizada com um parâmetro de tolerância — 5 dias aqui, 30 pro CAGR) pra calcular o yield daquele pagamento específico; verificado contra BBAS3 real (71 pagamentos, todos casados, `diff_days=0.0`). Tabela nova `stock_dividend_payments` (histórico real, diferente do resto do coletor: usa `INSERT OR IGNORE` num índice único `(ticker, payment_date)`, já que é fato histórico que não muda — rodar de novo/"Refresh data" não duplica). **Primeira biblioteca de gráfico do projeto**: escolhida `Recharts` com o usuário (opções apresentadas: Recharts, lightweight-charts, SVG sem dependência). Seguida a skill de dataviz: **nunca dois eixos Y** — em vez de um gráfico combinado, viraram 2 mini gráficos de barra empilhados (valor pago e yield%) com toggle mensal/anual, reusando a rampa de verde já definida em `index.css` (`--chart-1`.._5`). Testado ponta a ponta: extração real, `cargo check`/`tsc` limpos, dedupe confirmado rodando o coletor 2x pro mesmo ticker (71→71, "0 new" na segunda vez), conferido visualmente pelo usuário no app real — **"mt bonitinho, exatamente como imaginei"**
- [x] 9.4 — Ícone/logo da empresa na tela. **Concluída na Sessão 21**: verificado contra a API real antes de codar (`curl` direto, sem token) — `brapi.dev/api/quote/{ticker}` devolve um campo `logourl` apontando pra `https://icons.brapi.dev/icons/{TICKER}.svg`, e esse padrão de URL funciona sozinho, sem precisar chamar a API: CDN pública (GitHub Pages + Fastly, `access-control-allow-origin: *`, cache configurado), 200 com SVG real pra ticker existente (testado PETR4/VALE3/WEGE3/MGLU3/ITUB4/RAIZ4), 404 limpo pra ticker inválido. Como é só uma URL previsível, a implementação ficou **inteiramente no frontend** — nenhuma mudança em `data-collector`, nenhum comando Rust novo, nenhuma tabela/migration: `CompanyLogo` novo em `StockLookupPanel.tsx`, um `<img>` com `onError` escondendo o elemento (sem placeholder/ícone genérico) quando o logo não existe pra aquele ticker. Todos os SVGs do CDN já vêm com fundo colorido próprio (confirmado inspecionando o conteúdo), então não precisou de wrapper branco. `npx tsc --noEmit` limpo, testado no app real rodando via `./desktop/dev.sh` — **confirmado pelo usuário** ("deu boa dms")
- [x] 9.5 — Reorganizar a navegação: **concluída na Sessão 20** — "Saved Valuations" saiu de `SECTIONS` (não é mais aba própria); a aba "Valuation" ganhou um estado local (`valuationView: "form" | "saved"`) que troca entre o formulário de cálculo e o `SavedValuationsPanel`, com um botão "Saved Valuations" no canto (ao lado do seletor de modelo) e um botão "← Back" pra voltar. `SavedValuationsPanel` desmonta/remonta ao trocar de view (reseta pra lista sempre que volta), decisão simples aceita sem precisar de estado persistente. `npx tsc --noEmit` limpo, aplicado via HMR no app rodando e confirmado pelo usuário ("o botão do saved valuations funcionou")

---

### Fase 10 — Remodelagem para Workspaces Colaborativos & Multi-Ativos (ideia levantada na Sessão 29, decisão de stack confirmada — mesma stack do projeto, não iniciada)

**Objetivo**: evoluir o Practice Valuation de app desktop single-user pra uma plataforma colaborativa de Intelligence & Portfolio Management (estilo Status Invest / Investidor10 + Notion), com múltiplos usuários por Workspace, autenticação via TruthID, e suporte a mais classes de ativos além de ações BR.

**Decisão de arquitetura (Sessão 29, resolvida)**: o rascunho original mencionava "App Django" e PostgreSQL — o dono do projeto confirmou que isso veio de uma sugestão equivocada do Gemini ao ajudar a escrever o rascunho, não uma intenção real de trocar de stack. **Fica mantido o padrão já decidido no resto do projeto**: Tauri + Rust + React/TS, SQLite local, sem servidor próprio operado pelo Practice Valuation (ver "Decisões de Arquitetura em Aberto"). Os conceitos abaixo (workspaces, RBAC, multi-ativos, teses, watchlists) precisam ser adaptados pra rodar sobre esse modelo.

**⚠️ Ressalva honesta que continua em aberto**: isso não é só trocar "Postgres" por "SQLite" no papel. Um Workspace com Owner/Admin/Editor/Viewer de verdade — várias pessoas diferentes, em máquinas diferentes, escrevendo no mesmo Workspace — pressupõe **alguém arbitrando quem pode escrever o quê**. Num servidor central isso é trivial (checar `role` numa query antes de aceitar o `UPDATE`). De forma descentralizada, sem servidor do Practice Valuation, isso é um problema bem mais difícil: teria que apoiar em cima do sync da Fase 8 (TruthID + IPFS) e resolver permissão via algo tipo capability/assinatura em vez de uma checagem de banco — nada disso está desenhado ainda. Vale separar, quando a hora chegar, o que é "só mais schema no SQLite de sempre" (multi-ativos, watchlists, teses+anexos — não dependem de multiusuário nenhum) do que é de fato "colaboração entre pessoas diferentes" (Workspaces com convite/RBAC — esse sim exige resolver o problema acima antes de começar a codar).

**1. Arquitetura Multi-Tenant & Colaboração (Workspaces)**
- App passa a ser centrado em **Workspaces**: carteiras, teses, listas e anexos pertencem a um Workspace, não a um usuário direto.
- Fluxo: login via TruthID → tela inicial lista Workspaces próprios (Owner) e Workspaces em que o usuário foi convidado (Membro/Convidado) → botão "Criar Novo Workspace".
- RBAC granular por membro, definido pelo Owner/Admin: papéis padrão `Owner`/`Admin`/`Editor`/`Viewer`, mais flags específicas — `can_add_transactions`, `can_delete_transactions`, `can_create_theses`, `can_manage_members`.
- **Decidido (continuação da Sessão 29)**: pra destravar o início da Fase 10 sem depender do problema de permissão descentralizada (ressalva acima), o Workspace **nasce single-user** — convite/membro externo/RBAC ativo entre pessoas diferentes fica pra depois, ver "Ordem de Implementação" no fim desta fase.

**2. Consolidação de Carteiras & Multi-Ativos**
- Um Workspace pode conter múltiplas carteiras de investimento pra consolidação global.
- **Confirmado explicitamente pelo dono do projeto (continuação da Sessão 29)**: a carteira (`Portfolio`) pertence ao **Workspace**, não a um membro individual — mesmo modelo já refletido no rascunho de banco abaixo (`Portfolios(id, workspace_id, ...)`, sem `user_id`). Um Workspace com 3 membros compartilha as mesmas carteiras; não existe "minha carteira dentro do workspace" separada da carteira de outro membro — é a carteira do Workspace, e quem pode mexer nela é resolvido pelo RBAC (item 1), não por dono individual da carteira.
- Classes de ativos propostas: Ações (Brasil, já suportado hoje), Stocks internacionais (moeda base USD, novo), Criptomoedas (paridades principais, fração de ativos), Tesouro Direto detalhado (nome do título, taxa contratada, indexador, data de aplicação/vencimento), Renda Fixa geral (emissor, taxa % CDI/Pré/Pós, vencimento, liquidez). **Lista ampliada e detalhada na Sessão 30** — ver item 8 abaixo (FII, REIT, ETF, Metal, Imóvel, Empresa não listada, mais o modelo de listagem/exposição/classe).

**3. Pesquisa Centralizada & Tela de Análise**
- Barra de busca global (command bar / spotlight) centralizando a pesquisa de qualquer ativo (ações/stocks/cripto/renda fixa) ou tese.
- Tela unificada do ativo: dados de mercado (cotação, variação, gráficos, valuation), posição consolidada no Workspace (preço médio, total investido, P&L), e teses/notas do Workspace vinculadas àquele ativo.

**4. Teses de Investimento, Documentos e Anexos**
- `Thesis`: vinculável a um ativo específico (ex.: tese em WEGE3) ou global/macro (ex.: "Cenário de Juros 2027"), editor Markdown/Rich Text.
- `ThesisAttachment`: anexar PDFs (relatórios de RI, fatos relevantes, lâminas), planilhas e imagens — armazenamento em bucket (S3/MinIO/Cloudflare R2) com metadados no banco.

**5. Listas de Ativos (Watchlists) & Favoritos**
- Favoritos: marcação rápida de ativos pra destaque no dashboard do Workspace.
- Watchlists nomeadas (ex.: "Ações de Dividendos", "Turnarounds pra Acompanhar") com anotações e preço-alvo por ativo.

**6. Rascunho de modelagem de banco trazido junto** (formato ilustrativo do brainstorm original; ao planejar de verdade, vira migration SeaORM/SQLite como todo o resto do projeto — ver Fase 1):
```sql
-- Workspaces & Membros
Workspaces (id, name, owner_truthid, created_at)
WorkspaceMembers (id, workspace_id, user_truthid, role, permissions_json)

-- Carteiras e Transações
Portfolios (id, workspace_id, name, description)
Assets (id, ticker, name, asset_class, currency)
Transactions (id, portfolio_id, asset_id, type, quantity, price, date, fees, fixed_income_metadata_json)

-- Teses, Documentos e Watchlists
Theses (id, workspace_id, asset_id NULL, title, content_markdown, created_by)
ThesisAttachments (id, thesis_id, file_url, file_name, file_size)
Watchlists (id, workspace_id, name)
WatchlistItems (id, watchlist_id, asset_id, target_price, notes)
```

**7. Rentabilidade Histórica (Backfill) da Carteira** (ideia trazida num rascunho à parte na continuação da Sessão 29, `spec_rentabilidade_historica_carteira.md`, conteúdo transcrito aqui e o arquivo removido)

**Problema**: migrar uma carteira que já existe há 2-3 anos pro app sem perder o histórico de rentabilidade — lançar retroativamente as movimentações (compra/venda/provento/aporte/retirada) desde a origem da carteira e calcular a rentabilidade mês a mês, consolidando mês → ano → período total, com granularidade **mensal** (não diária).

**Por que não é `(valor final − valor inicial) / valor inicial`**: essa conta ingênua quebra na presença de aporte/retirada no meio do mês (um aporte de R$5.000 no meio do mês infla o "retorno" sem ser rendimento de verdade). Existem duas famílias de métrica:
- **Money-Weighted Return (TIR/XIRR)**: mede o retorno em R$ do dinheiro do investidor, mas não encadeia mês a mês de forma simples.
- **Time-Weighted Return (TWR)**: mede o retorno da carteira em si, neutralizando o efeito de quando o dinheiro entrou/saiu — é o padrão usado por fundos/CVM/B3, e **é geometricamente componível** (jan +1%, fev −2%, ... → total = produto de `(1+r)` − 1). É essa a métrica certa pro que foi pedido (rendimento mês a mês se somando geometricamente até o total).

**Como calcular TWR mensal sem dado diário — Método de Dietz Modificado** (aproxima o TWR "de livro", que exigiria reavaliar a carteira a cada fluxo de caixa, usando só o valor no início/fim do mês + os fluxos do mês):
```
R_mes = (EMV - BMV - CF_total) / (BMV + Σ(CF_i × W_i))
```
onde `BMV`/`EMV` são o valor da carteira no início/fim do mês, e `W_i = (dias_no_mes - dias_desde_inicio_ate_o_fluxo) / dias_no_mes` é o peso de cada fluxo (um aporte no dia 1 "trabalhou" o mês inteiro, peso ≈ 1; um aporte no dia 28 quase não influenciou o mês, peso ≈ 0). Proventos entram como fluxo de caixa positivo dentro do mês (mesma lógica de aporte) se sacados, ou só aumentam o `EMV` se reinvestidos/mantidos em caixa — decisão de modelagem em aberto, ver abaixo.

**Consolidação mês → ano → total** (TWR é geometricamente componível, sem mistério):
```
R_ano   = Π(1 + R_mes_i) - 1   para os N meses do ano
R_total = Π(1 + R_mes_i) - 1   para todos os meses do período filtrado
```
Filtro de período (ex.: "desde que comecei", "últimos 2 anos", "2024") é só decidir o intervalo de meses no produtório — nenhuma mudança de metodologia.

**O que precisa ser modelado**: pra calcular `BMV`/`EMV` de cada mês retroativamente, o sistema precisa, pra cada fim de mês no passado, saber (1) quais ativos estavam na carteira e em que quantidade — derivado do histórico de lançamentos até aquela data, (2) o preço de fechamento de cada ativo naquele fim de mês — precisa de série histórica de preços, não só o preço atual (pra ativos B3 o projeto já tem infra de dado de mercado; pra ativos vendidos antes de hoje, precisa do preço histórico até a data da venda), e (3) todos os fluxos de caixa do mês (aportes/retiradas/proventos em dinheiro) com data e valor. Rascunho de estrutura (ilustrativo, viraria migration SeaORM/SQLite como o resto do projeto):
```
Lancamento { data, tipo: compra|venda|aporte|retirada|provento, ativo (null p/ aporte/retirada puro), quantidade, preco_unitario, valor_total }
SnapshotMensal { ano_mes, valor_carteira_fim_mes (calculado), rentabilidade_mes (calculado via Dietz Modificado) }
```
No desenho da Fase 10, `Lancamento` mapeia naturalmente pra `Transactions` (já no rascunho de banco acima, escopada por `portfolio_id` — e por consequência por Workspace, ver item 2); `SnapshotMensal` seria tabela nova, por `portfolio_id` + `ano_mes`.

**Decisões em aberto (o dono do projeto ainda precisa bater o martelo)**:
- Proventos contam como retorno mesmo sem reinvestir, ou só quando reinvestidos? (Convenção de mercado: contam no mês do pagamento, entram no `EMV` via caixa, mesmo sem reinvestir.)
- Meses sem lançamento nenhum ainda têm `R_mes` (variação de preço dos ativos parados) — não dá pra simplesmente pular o mês no produtório.
- Carteira zerada em algum mês (vendeu tudo) quebra o encadeamento de TWR ali — tratar como "novo período" a partir da próxima entrada, ou explicitar na consolidação total.

**Exemplo numérico** (do rascunho original, confirma a fórmula): janeiro começa com R$10.000, aporte de R$2.000 no dia 10 (peso ≈ 21/31 ≈ 0,68), fecha com R$12.500 → `R_jan = (12.500 − 10.000 − 2.000) / (10.000 + 2.000×0,68) = 500/11.360 ≈ 4,4%`. Fevereiro começa com R$12.500, sem fluxo, fecha com R$12.250 → `R_fev = (12.250 − 12.500)/12.500 = −2,0%`. Consolidado jan+fev: `(1,044 × 0,98) − 1 ≈ 2,3%`.

**8. Modelo Detalhado de Multi-Ativos: Exposição, Novas Classes e Anexos** (rascunho à parte, `practice-valuation-carteira-multiativo (1).md`, conteúdo transcrito aqui e arquivo removido, Sessão 30). Detalha e substitui a lista simples de classes do item 2 acima — é o desenho de como a fatia 10.2 (multi-ativos) deve funcionar de fato.

**Classes de ativo suportadas**: Ação (B3 e bolsas americanas), FII, REIT (equivalente americano do FII — FII usa P/VP e dividend yield em BRL, REIT usa FFO/AFFO e cap rate em USD, mas convivem na mesma "família" de fundo imobiliário na UI), ETF (B3 e bolsa americana, expansível a outros países no futuro), Cripto, Metal (ouro, prata, etc.), Imóvel (ativo físico, cadastro manual) e Empresa não listada (equity privado, cadastro manual).

**Modelo central — listagem ≠ exposição ≠ classe**: hoje cada ativo tende a ter implicitamente um único país, herdado da fonte de dados (bolsa de negociação). O modelo separa três conceitos:

| Conceito | O que representa | Origem |
|---|---|---|
| Listagem/exchange | Onde o ativo é negociado (ex.: NASDAQ, B3) | Puxado automático da fonte de dados, editável |
| Exposição | A que país(es) ou categoria especial o ativo realmente expõe o investidor | Padrão = país da listagem, editável pelo usuário |
| Classe do ativo | Ação, FII, REIT, ETF, cripto, metal, imóvel, empresa não listada | Cadastro do ativo |

Exposição é **um ou mais países com peso % somando 100%, OU uma categoria especial** (cripto, metal_ouro, metal_prata, etc.) — mutuamente exclusivo por ativo. Campo sugerido `tipo_exposicao = pais | categoria_especial`, com tabela `ativo_exposicao_pais (ativo_id, pais, peso_%)` pro caso de país. Exemplos: Banco Inter negociado nos EUA → listagem EUA (auto), exposição 100% Brasil (editado manualmente), classe Ação; IVVB11 → listagem B3 (auto), exposição 100% EUA (editado), classe ETF; ETF brasileiro de cripto → listagem B3, exposição categoria_especial (cripto) — fora do cálculo de alocação por país; ETF de ouro → mesma lógica, categoria_especial (metal_ouro).

**Automatização da exposição**: puxar a exposição real de um ETF via fact sheet tem custo alto de implementação com pouco ganho no estágio atual — abordagem pro MVP é override manual por ativo (cadastra uma vez, fica salvo); evolução futura possível seria uma base curada própria de ETFs conhecidos, community-maintained.

**Metais como classe**: preço puxado automaticamente (cotação ao vivo, como cripto, sem valor manual); quantidade cadastrada em **gramas**, não "ações/cotas" — pede um campo `unidade_medida` no ativo (`acao`, `cota`, `grama`, `unidade`), generalizando o cálculo de posição pra `quantidade × preço_unitário` independente da classe. Fonte de cotação provavelmente é uma API diferente das já usadas — mais um provider seguindo o padrão de "cada classe pode ter sua fonte própria".

**Imóvel** (ativo físico, cadastro manual, sem fonte de dados externa): fotos/documentos anexos (escritura, ITBI, impostos pagos), endereço, valor de compra (fixo, histórico), histórico de avaliações `(data, valor, origem: manual | reajuste_automatico)`. Reajuste automático por % anual — mecanismo de cálculo (ao vivo vs. snapshot periódico) **ainda não decidido**, adiado pra implementação.

**Empresa não listada** (equity privado, mesmo padrão do imóvel): percentual de participação, quantidade de ações, valor de compra, valor atual (editável manualmente, mesmo mecanismo de histórico do imóvel), valuation total da empresa, fotos/foto de perfil da empresa. **Ponto em aberto**: percentual, quantidade de ações e valuation total são matematicamente relacionados (`percentual = suas_ações / total_de_ações_da_empresa`; `sua_participação_R$ = percentual × valuation_total`), mas isso só fecha se o total de ações emitidas pela empresa for conhecido — nem sempre disponível em empresa não listada. Decidir se o percentual é validado contra esses campos ou se é um valor digitado independente.

**Generalização `AtivoManual`**: Imóvel e Empresa não listada compartilham o mesmo esqueleto — cadastro manual, sem fonte de dados externa, histórico de valor, anexos. Faz sentido um tipo base `AtivoManual` (nome, valor de compra, histórico de valor, anexos), com cada subtipo adicionando só os campos específicos (endereço vs. participação societária).

**Anexos (sistema polimórfico único)**: em vez de um sistema de anexo por feature (imóvel, empresa, lançamento), um único subsistema reutilizável: `anexo (id, entidade_tipo, entidade_id, arquivo, tipo_documento, data)`, onde `entidade_tipo` pode ser `imovel`, `empresa` ou `lancamento` — todos passam pelo mesmo código de upload/storage, só muda o rótulo do tipo de documento. (Mesma necessidade de storage de arquivo já levantada em `ThesisAttachment`, item 4 acima — decidir na implementação se vira o mesmo mecanismo ou fica separado.)

**Busca unificada de ativos**: uma única busca cobrindo todas as classes negociadas em bolsa (não inclui Imóvel/Empresa não listada, que são cadastro manual direto, sem ticker). Arquitetura híbrida: catálogo local pré-indexado (`ativo_catalogo`: ticker, nome, classe, exchange, fonte_dados) populado por job de sync periódico, busca instantânea pro caso comum; atualização sob demanda — se a busca não encontra o ativo no catálogo local, cai pra fan-out ao vivo nas APIs externas, e se encontrar, indexa no catálogo pra próxima busca. UX: cada resultado exibe classe + exchange + exposição (ex.: "BIDI3 — Ação — B3 — 🇧🇷" vs. "INTR — Stock — NASDAQ — 🇧🇷"), pra resolver o caso de ativos do mesmo emissor negociados em mercados diferentes (ex.: Banco Inter).

**Decisões em aberto trazidas por este rascunho**: mecanismo de reajuste automático de imóvel (ao vivo vs. snapshot); validação cruzada percentual/quantidade/valuation de empresa não listada; automação de exposição de ETF via fact sheet (adiado — override manual no MVP).

**9. Gestão de Custódias/Contas** (rascunho à parte, `practice-valuation-mobile-custodia-nome.md`, seção 4, conteúdo transcrito aqui e arquivo removido, Sessão 30). Nova dimensão no modelo, ortogonal à classe do ativo: **onde** cada investimento está guardado (corretora, exchange, hardware wallet, banco), independente do que o ativo é.

Entidade `Custodia`: cadastro livre pelo usuário, sem nada pré-pronto (ex.: "Rico", "Nomad", "Bipa", "Ledger"). Campo `instituicao` (nome da corretora/exchange/wallet) e campo `titular` (CPF/CNPJ ou identificação do titular da conta) — dois campos separados na mesma entidade, sem precisar de hierarquia adicional. Permite agrupar por qualquer uma das duas dimensões: "tudo que está na Rico" (soma titulares) ou "tudo que é meu CNPJ" (soma instituições).

Posição quebrada por custódia: `posicao (ativo_id, custodia_id, quantidade)` — um mesmo ativo pode estar dividido entre múltiplas custódias (ex.: parte do BTC na Binance, parte na Ledger). Total do ativo na carteira = soma de todas as linhas de posição. Resumo consolidado da carteira soma tudo, ignorando titular/custódia por padrão — o detalhamento é uma visão adicional, não a visão default.

Tela de gerenciamento de contas: seção própria na navegação, separada do cadastro de ativo. Lista de custódias cadastradas, cada uma mostrando total consolidado ali dentro + breakdown por classe de ativo. Duas visões possíveis a partir da mesma tabela `posicao` — por custódia (o que está em cada lugar) e por ativo (onde cada ativo está distribuído) — não duplica dado, só agrupa diferente.

Lançamentos com custódia e taxa: vínculo à custódia acontece no momento do lançamento (compra/transferência já pede em qual custódia está entrando), não como passo manual separado. Novo tipo de lançamento **transferência** entre custódias, com `custodia_origem`, `custodia_destino`, `quantidade`. Campo `taxa` genérico, aplicável a qualquer tipo de lançamento (compra, venda, transferência) — reaproveita a estrutura de lançamento já existente (`Lancamento`/`Transactions`, ver item 7 acima), sem tabela separada.

**Decisão em aberto**: taxa de transferência sempre no próprio ativo transferido, ou pode ser em ativo/moeda diferente (ex.: taxa de rede em cripto vs. corretagem em BRL) — não decidido.

**Ordem de implementação decidida com o dono do projeto (continuação da Sessão 29, via `AskUserQuestion`)**:

**Workspace nasce single-user** — um único Workspace implícito (o próprio dono), sem convite/membro externo, sem RBAC ativo ainda. Essa escolha destrava começar a Fase 10 sem precisar resolver a permissão descentralizada primeiro (ver ressalva acima) — convite/multiusuário real vira uma fatia à parte, bloqueada até esse problema estar resolvido (provavelmente amarrado à Fase 8). **Primeira fatia concreta escolhida pra quando começarmos: multi-ativos.**

- [ ] 10.1 — Fundação: schema `Workspace`/`Portfolio` (SQLite/SeaORM) — Workspace single-user, sem `WorkspaceMember`/convite ainda. Pré-requisito de tudo abaixo.
- [ ] 10.2 — **Primeira fatia, decidida com o dono do projeto**: Multi-ativos — Stocks internacionais (moeda USD), Tesouro Direto detalhado, Renda Fixa geral no schema de `Transactions` (substitui o antigo item "tabela unificada de `Transactions`"); escopo ampliado na Sessão 30 com FII/REIT/ETF/Cripto/Metal/Imóvel/Empresa não listada e o modelo de listagem/exposição/classe (ver item 8) e com Gestão de Custódias/Contas (ver item 9)
- [ ] 10.3 — Rentabilidade histórica (TWR/Dietz Modificado, ver seção 7 acima) — tecnicamente não depende do Workspace/multi-ativos prontos; ordem relativa às fatias 10.4/10.5 ainda não decidida, pode furar a fila se fizer mais sentido na hora
- [ ] 10.4 — Watchlists + favoritos — a mais simples das três restantes, boa pra validar o modelo de Workspace na prática; ordem relativa a 10.3/10.5 ainda não decidida
- [ ] 10.5 — Teses + anexos — precisa decidir armazenamento de arquivo (S3/MinIO/R2) antes de começar (infra nova que as outras fatias não exigem); ordem relativa a 10.3/10.4 ainda não decidida
- [ ] 10.6 — Seletor de múltiplos Workspaces na tela inicial (ex.: "Pessoal" vs "Família") — só faz sentido se o dono do projeto quiser mais de um Workspace próprio; não depende de multiusuário
- [ ] 10.7 — **Bloqueada, não planejada**: convite + RBAC ativo entre pessoas diferentes + login TruthID multiusuário (diferente do uso atual do TruthID no projeto, que é só assinatura delegada pra sync — Fase 8) — depende de resolver a permissão descentralizada primeiro (ver ressalva acima)

---

### Fase 11 — Rebranding (Anchor), App Mobile & Empacotamento Desktop (ideia levantada na Sessão 30, não iniciada)

**Objetivo**: renomear o app, ganhar um cliente mobile completo e empacotar o desktop como instalador standalone (sem exigir Python instalado na máquina do usuário) — três decisões trazidas juntas num segundo rascunho à parte (`practice-valuation-mobile-custodia-nome.md`, conteúdo transcrito aqui e arquivo removido; a 4ª ideia do mesmo rascunho, Gestão de Custódias, foi pra Fase 10 item 9 por pertencer ao modelo de carteira, não a essa fase).

**1. Nome do app → Anchor**: critérios usados na escolha — em inglês, remetendo a segurança/controle do próprio patrimônio (mesma linha conceitual do TruthID — self-custody, sem intermediário), curto e fácil de pronunciar em qualquer idioma. Como é software open-source, não há preocupação de conflito de marca/domínio. **Decisão de nome registrada — execução do rename (repo, pastas, README, referências internas) ainda não feita.**

**2. App mobile**: completo, replicando todas as funções que o desktop tem e terá. Framework escolhido: **Flutter** — base de código separada do desktop, resolve de saída o problema de o sidecar Python não rodar em mobile. (Atualiza o item "Companion mobile" do Roadmap, que antes era só uma ideia solta sem framework decidido.)

**3. Empacotamento do desktop (Tauri + Python)**: sidecar Python compilado pra binário standalone via **PyInstaller** ou **Nuitka**, sem exigir Python instalado na máquina do usuário. Binário registrado em `tauri.conf.json` (`bundle.externalBin`), empacotado junto no instalador final do Tauri (NSIS/MSI, `.dmg`, `.deb`/AppImage). CI: **GitHub Actions**, com matrix build (`windows-latest`, `macos-latest`, `ubuntu-latest`) compilando o sidecar nativamente em cada runner antes do step do `tauri-action`, que gera os instaladores e publica na release. Atenção: nome do binário do sidecar precisa incluir o target triple da plataforma (ex.: `sidecar-x86_64-pc-windows-msvc.exe`) pro Tauri localizar corretamente.

**Etapas**:
- [ ] 11.1 — Executar o rename pra Anchor (repo, pastas, README, referências internas)
- [ ] 11.2 — App mobile em Flutter, replicando as funções do desktop
- [ ] 11.3 — Empacotamento do sidecar Python (PyInstaller/Nuitka) + CI GitHub Actions (matrix build) gerando instaladores standalone


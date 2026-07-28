## Roadmap de Evoluções Planejadas

- **Sync entre máquinas/nuvem**: hoje o banco é 100% local; desenho descentralizado via TruthID + IPFS discutido na Sessão 11, ver Fase 8 em "Fases Detalhadas"
- **Mais indicadores de cripto pagos** (Glassnode/CryptoQuant — MVRV, Puell, Netflow) se o usuário decidir assinar
- **Companion mobile** — só se fizer sentido depois do desktop estar redondo
- **Mais metodologias de valuation** conforme o usuário for trazendo (Bazin/preço-teto, Graham, DCF, EV/EBITDA setorial, etc.)
- **Remodelagem pra plataforma colaborativa multi-tenant** (Workspaces, RBAC, multi-ativos, teses/anexos, watchlists, carteira pertence ao Workspace e não a um membro, rentabilidade histórica via TWR/Dietz Modificado) — ideia grande registrada na Sessão 29, ver Fase 10 em "Fases Detalhadas"; fica na stack atual (sem servidor/Postgres/Django, isso era um equívoco do Gemini no rascunho original). Workspace nasce single-user pra destravar o início sem depender do problema de permissão descentralizada (colaboração multiusuário real fica bloqueada até resolver isso); ordem de implementação já decidida: fundação → multi-ativos (primeira fatia) → rentabilidade/watchlists/teses (ordem entre elas em aberto)


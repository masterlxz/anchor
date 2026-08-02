export type Workspace = {
  id: number;
  name: string;
  created_at: string;
};

export type Portfolio = {
  id: number;
  workspace_id: number;
  name: string;
  description: string | null;
  created_at: string;
};

export type Custodia = {
  id: number;
  workspace_id: number;
  instituicao: string;
  titular: string;
  created_at: string;
};

// Fase 10.2, escopo Sessão 29 — REIT/cripto/metal/imóvel/empresa não listada
// (Sessão 30) ficam pra uma fatia futura. `fii` entrou na Sessão 41, `etf_br`
// na Sessão 49 — só a fatia de cotação automática por ora, sem indicador de
// fundo específico (CVM não tem categoria de dados abertos pra fundo de
// índice, diferente de FII, achado pesquisando de verdade — ver PHASE.md
// item 8). Ver commands/asset.rs::ASSET_CLASSES.
export type AssetClass =
  | "acao_br"
  | "fii"
  | "etf_br"
  | "acao_internacional"
  | "tesouro_direto"
  | "renda_fixa";

export const ASSET_CLASSES: AssetClass[] = [
  "acao_br",
  "fii",
  "etf_br",
  "acao_internacional",
  "tesouro_direto",
  "renda_fixa",
];

export const ASSET_CLASS_LABELS: Record<AssetClass, string> = {
  acao_br: "Stock (B3)",
  fii: "FII (B3)",
  etf_br: "ETF (B3)",
  acao_internacional: "International stock",
  tesouro_direto: "Tesouro Direto",
  renda_fixa: "Fixed income",
};

// Classes que usam os campos fi_* (renda fixa detalhada) na transação de compra.
export const FIXED_INCOME_CLASSES: AssetClass[] = ["tesouro_direto", "renda_fixa"];

// Classes negociadas na B3 com busca automática de cotação por ticker (mesmo
// endpoint Yahoo `{ticker}.SA`, sem coletor dedicado por classe) — usada por
// AssetSection.tsx (form vira busca) e ProfitabilitySection.tsx (TWR).
export const ASSET_CLASSES_WITH_AUTO_QUOTE: AssetClass[] = ["acao_br", "fii", "etf_br"];

export type ExposureType = "pais" | "categoria_especial";

export type Asset = {
  id: number;
  ticker: string;
  name: string;
  asset_class: string;
  currency: string;
  exchange: string | null;
  exposure_type: string;
  exposure_value: string;
  created_at: string;
  cnpj: string | null;
};

// Fase 10, item 8, Sessão 41 — indicadores de FII direto da CVM (dados
// abertos), priorizados sobre a bolsai por pedido explícito do dono do
// projeto. `cnpj` é a chave de junção com `assets.cnpj` (não um ID interno
// dessas tabelas). Ver commands/fii.rs.
export type FiiCnpjSuggestion = {
  cnpj: string;
  fund_name: string;
};

export type FiiCvmMonthly = {
  id: number;
  cnpj: string;
  reference_date: string;
  patrimonio_liquido: number;
  valor_patrimonial_cota: number;
  numero_cotistas: number | null;
  dividend_yield_mes: number | null;
  rentabilidade_efetiva_mes: number | null;
  source: string;
  fetched_at: string;
};

export type FiiCvmProperty = {
  id: number;
  cnpj: string;
  reference_date: string;
  nome_imovel: string;
  endereco: string | null;
  area_m2: number | null;
  percentual_vacancia: number | null;
  percentual_inadimplencia: number | null;
  percentual_receitas_fii: number | null;
  percentual_locado: number | null;
  source: string;
  fetched_at: string;
};

export type TransactionType =
  | "compra"
  | "venda"
  | "aporte"
  | "retirada"
  | "provento"
  | "transferencia";

export const TRANSACTION_TYPES: TransactionType[] = [
  "compra",
  "venda",
  "aporte",
  "retirada",
  "provento",
  "transferencia",
];

export const TRANSACTION_TYPE_LABELS: Record<TransactionType, string> = {
  compra: "Buy",
  venda: "Sell",
  aporte: "Contribution",
  retirada: "Withdrawal",
  provento: "Dividend",
  transferencia: "Transfer between custodies",
};

export const FI_INDEXADORES = ["CDI", "IPCA", "SELIC", "PREFIXADO", "OUTRO"] as const;
export const FI_LIQUIDEZ_OPTIONS = ["diaria", "no_vencimento", "outro"] as const;

export type TransactionView = {
  id: number;
  asset_id: number | null;
  ticker: string | null;
  custodia_id: number | null;
  custodia_label: string | null;
  transfer_to_custodia_id: number | null;
  transfer_to_custodia_label: string | null;
  transaction_type: string;
  quantity: number | null;
  unit_price: number | null;
  total_value: number;
  fee: number | null;
  transaction_date: string;
  notes: string | null;
  fi_emissor: string | null;
  fi_indexador: string | null;
  fi_taxa_percentual: number | null;
  fi_data_vencimento: string | null;
  fi_liquidez: string | null;
  created_at: string;
};

export type CustodiaBreakdown = {
  custodia_id: number | null;
  custodia_label: string | null;
  quantity: number;
};

// Fase 10.3, fatia das classes com preço histórico automatizado (Ação
// BR/FII) — aporte/retirada e as demais classes ainda não entram nesse
// cálculo (ver ProfitabilitySection.tsx).
export type MonthlyReturn = {
  year_month: string;
  bmv: number;
  emv: number;
  cf_total: number;
  r_month_pct: number;
  r_cumulative_pct: number;
};

export type PositionView = {
  asset_id: number;
  ticker: string;
  name: string;
  asset_class: string;
  currency: string;
  quantity: number;
  average_buy_price: number | null;
  by_custodia: CustodiaBreakdown[];
};

// Fase 10.4 — listas nomeadas de ativos (preço-alvo/notas) e favoritos
// rápidos (estrela), dois mecanismos separados por pedido explícito do dono
// do projeto.
export type Watchlist = {
  id: number;
  workspace_id: number;
  name: string;
  created_at: string;
};

export type WatchlistItem = {
  id: number;
  watchlist_id: number;
  asset_id: number;
  target_price: number | null;
  notes: string | null;
  created_at: string;
};

export type AssetFavorite = {
  id: number;
  workspace_id: number;
  asset_id: number;
  created_at: string;
};

// Fase 10.5 — tese de investimento, vinculável a um ativo específico
// (asset_id preenchido) ou global/macro (asset_id null). Anexos vivem em
// disco (app_data_dir), só metadados ficam no banco.
export type Thesis = {
  id: number;
  workspace_id: number;
  asset_id: number | null;
  title: string;
  content_markdown: string;
  created_at: string;
  updated_at: string;
};

export type ThesisAttachment = {
  id: number;
  thesis_id: number;
  original_file_name: string;
  stored_relative_path: string;
  file_size_bytes: number;
  content_type: string | null;
  created_at: string;
};

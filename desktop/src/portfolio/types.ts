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

// Fase 10.2, escopo Sessão 29 — só estas 4 classes por ora (ver
// commands/asset.rs::ASSET_CLASSES); FII/REIT/ETF/cripto/metal/imóvel/
// empresa não listada (Sessão 30) ficam pra uma fatia futura.
export type AssetClass =
  | "acao_br"
  | "acao_internacional"
  | "tesouro_direto"
  | "renda_fixa";

export const ASSET_CLASSES: AssetClass[] = [
  "acao_br",
  "acao_internacional",
  "tesouro_direto",
  "renda_fixa",
];

export const ASSET_CLASS_LABELS: Record<AssetClass, string> = {
  acao_br: "Ação (B3)",
  acao_internacional: "Stock internacional",
  tesouro_direto: "Tesouro Direto",
  renda_fixa: "Renda Fixa",
};

// Classes que usam os campos fi_* (renda fixa detalhada) na transação de compra.
export const FIXED_INCOME_CLASSES: AssetClass[] = ["tesouro_direto", "renda_fixa"];

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
  compra: "Compra",
  venda: "Venda",
  aporte: "Aporte",
  retirada: "Retirada",
  provento: "Provento",
  transferencia: "Transferência entre custódias",
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

// Fase 10.3, fatia Ação BR — aporte/retirada e as demais classes ainda não
// entram nesse cálculo (ver ProfitabilitySection.tsx).
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

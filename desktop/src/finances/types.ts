export type BankAccountView = {
  id: number;
  workspace_id: number;
  nome: string;
  titular: string;
  created_at: string;
  balance: number;
};

export type GeneralTransactionCategory = {
  id: number;
  workspace_id: number;
  nome: string;
  created_at: string;
};

export type GeneralTransaction = {
  id: number;
  workspace_id: number;
  bank_account_id: number;
  transaction_type: string;
  category_id: number | null;
  valor: number;
  transaction_date: string;
  notes: string | null;
  created_at: string;
};

// Fase 12, núcleo de Finanças Gerais — só `receita`/`despesa` são
// selecionáveis no formulário manual. `parcela_divida` existe como tipo de
// transação (gerado só por `create_liability`, nunca digitado à mão) —
// `compra_ativo`/`venda_ativo` do rascunho original seguem fora de escopo
// (ver PHASE.md, Fase 12).
export const GENERAL_TRANSACTION_TYPES = ["receita", "despesa"] as const;
export type GeneralTransactionType = (typeof GENERAL_TRANSACTION_TYPES)[number];

// Registro solto (não `Record<GeneralTransactionType, string>`) porque
// precisa rotular também `parcela_divida`, que não é selecionável no form.
export const GENERAL_TRANSACTION_TYPE_LABELS: Record<string, string> = {
  receita: "Income",
  despesa: "Expense",
  parcela_divida: "Loan installment",
};

export type Liability = {
  id: number;
  workspace_id: number;
  bank_account_id: number;
  linked_asset_id: number | null;
  nome: string;
  titular: string;
  principal: number;
  taxa_percentual: number;
  indexador: string;
  prazo_meses: number;
  sistema_amortizacao: string;
  data_inicio: string;
  created_at: string;
};

export type LiabilityView = Liability & {
  saldo_devedor_atual: number;
};

// Fase 12 — dívida/passivo. Vocabulário duplicado de
// `portfolio/types.ts::FI_INDEXADORES` de propósito: Finanças Gerais é
// módulo irmão de Portfolio, não depende dele (ver PHASE.md, Fase 12).
export const LIABILITY_INDEXADORES = ["CDI", "IPCA", "SELIC", "PREFIXADO", "OUTRO"] as const;

export const LIABILITY_AMORTIZATION_SYSTEMS = ["price", "sac"] as const;
export type LiabilityAmortizationSystem = (typeof LIABILITY_AMORTIZATION_SYSTEMS)[number];

export const LIABILITY_AMORTIZATION_SYSTEM_LABELS: Record<LiabilityAmortizationSystem, string> = {
  price: "Price (fixed installment)",
  sac: "SAC (fixed amortization)",
};

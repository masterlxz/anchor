export type BankAccountView = {
  id: number;
  workspace_id: number;
  nome: string;
  titular: string;
  created_at: string;
  balance: number;
};

export type GeneralTransaction = {
  id: number;
  workspace_id: number;
  bank_account_id: number;
  transaction_type: string;
  categoria: string | null;
  valor: number;
  transaction_date: string;
  notes: string | null;
  created_at: string;
};

// Fase 12, núcleo de Finanças Gerais — só `receita`/`despesa` nesta fatia.
// `parcela_divida`/`compra_ativo`/`venda_ativo` do rascunho original ficam
// para as fatias de `Liability`/link com Portfolio (ver PHASE.md, Fase 12).
export const GENERAL_TRANSACTION_TYPES = ["receita", "despesa"] as const;
export type GeneralTransactionType = (typeof GENERAL_TRANSACTION_TYPES)[number];

export const GENERAL_TRANSACTION_TYPE_LABELS: Record<GeneralTransactionType, string> = {
  receita: "Income",
  despesa: "Expense",
};

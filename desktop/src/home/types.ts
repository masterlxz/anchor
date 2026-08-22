// Home do Workspace, fatia 1 (layout fixo) — espelha
// `commands/workspace_summary.rs`. `ClassAllocation`/`MonthlyReturn` são os
// mesmos tipos já usados por `portfolio/SummarySection.tsx`/`ProfitabilitySection.tsx`,
// só que agregados no Workspace inteiro em vez de 1 portfolio só.
import type { ClassAllocation, MonthlyReturn } from "../portfolio/types";

export type PortfolioBreakdown = {
  portfolio_id: number;
  name: string;
  market_value: number;
};

export type WorkspaceSummary = {
  total_market_value: number;
  total_cost_basis: number;
  unrealized_pl: number;
  unrealized_pl_pct: number | null;
  dividends_received_12m: number;
  allocation_by_class: ClassAllocation[];
  positions_missing_price: number;
  portfolios: PortfolioBreakdown[];
};

export type { MonthlyReturn };

export type StockQuote = {
  id: number;
  ticker: string;
  price: number;
  source: string;
  fetched_at: string;
  name: string | null;
  exchange: string | null;
  currency: string | null;
};

export type StockFundamentals = {
  id: number;
  ticker: string;
  lpa: number;
  vpa: number;
  roe: number;
  source: string;
  fetched_at: string;
  payout: number | null;
};

export type StockDividendsAvg = {
  id: number;
  ticker: string;
  avg_dividend_5y: number;
  source: string;
  fetched_at: string;
};

export type StockDcfFundamentals = {
  id: number;
  ticker: string;
  reference_year: number;
  ebit: number;
  tax_rate: number | null;
  depreciation_amortization: number | null;
  capex: number | null;
  nwc_change: number;
  total_debt: number;
  cash: number;
  shares_outstanding: number;
  source: string;
  fetched_at: string;
  revenue: number | null;
  inventory: number | null;
};

export type StockDividendPayment = {
  id: number;
  ticker: string;
  payment_date: string;
  amount: number;
  price_at_payment: number | null;
  yield_pct: number | null;
  source: string;
  fetched_at: string;
};

export type StockTechnicals = {
  id: number;
  ticker: string;
  sma_50: number | null;
  sma_100: number | null;
  sma_200: number | null;
  cagr_5y: number | null;
  cagr_10y: number | null;
  source: string;
  fetched_at: string;
};

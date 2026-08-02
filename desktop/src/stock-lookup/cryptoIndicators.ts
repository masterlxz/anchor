// The 9 Ethereum score indicators from Fase 3 — keys match the `indicator`
// column seeded in `indicator_thresholds` by the migration.
export const INDICATORS = {
  mvrv_z_score: "MVRV Z-Score",
  nvt_ratio: "NVT Ratio (vs 90d moving average)",
  puell_multiple: "Puell Multiple",
  net_issuance: "Net Issuance (issuance - burn)",
  staking_yield: "Staking Yield (real)",
  tvl_trend: "TVL Trend (MoM %)",
  active_addresses_trend: "Active Addresses Trend (%)",
  exchange_netflow: "Exchange Netflow",
  fees_vs_emission: "Fees vs Emission",
} as const;

export type IndicatorKey = keyof typeof INDICATORS;

export const INDICATOR_KEYS = Object.keys(INDICATORS) as IndicatorKey[];

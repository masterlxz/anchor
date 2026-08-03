import { invoke } from "@tauri-apps/api/core";
import { useMutation } from "@tanstack/react-query";
import type { AppError } from "../types";
import { latestForTicker } from "./latestForTicker";
import type {
  StockDcfFundamentals,
  StockDividendsAvg,
  StockFundamentals,
  StockQuote,
} from "./types";

type CollectorSummary = {
  success: boolean;
  output: string;
};

export type TickerData = {
  quote: StockQuote | null;
  fundamentals: StockFundamentals | null;
  dividendsAvg: StockDividendsAvg | null;
  dcfFundamentals: StockDcfFundamentals | null;
};

// Runs the Python collector for a single ticker (quotes + fundamentals +
// dividends avg + DCF fundamentals, always all four — see the plan's
// "uniform pipeline" decision), then reads back whatever landed in the DB
// for that ticker so a valuation form can autofill its own fields.
//
// `assetClass` decides which collector path `run_stock_collector` picks
// (Rust `commands/collector.rs::run_stock_collector`) — without it, every
// caller fell through to the generic `.SA`-suffixed `--ticker` path, which
// silently fetched the wrong data for a US ticker like AAPL (bug found
// during Fatia 2, "Ação americana" fundamentals).
export function useTickerCollector(assetClass?: string) {
  return useMutation<TickerData, AppError, string>({
    mutationFn: async (ticker) => {
      const normalized = ticker.trim().toUpperCase();
      await invoke<CollectorSummary>("run_stock_collector", {
        ticker: normalized,
        asset_class: assetClass ?? null,
      });

      const [quotes, fundamentals, dividendsAvg, dcfFundamentals] =
        await Promise.all([
          invoke<StockQuote[]>("list_stock_quotes"),
          invoke<StockFundamentals[]>("list_stock_fundamentals"),
          invoke<StockDividendsAvg[]>("list_stock_dividends_avg"),
          invoke<StockDcfFundamentals[]>("list_stock_dcf_fundamentals"),
        ]);

      return {
        quote: latestForTicker(quotes, normalized),
        fundamentals: latestForTicker(fundamentals, normalized),
        dividendsAvg: latestForTicker(dividendsAvg, normalized),
        dcfFundamentals: latestForTicker(dcfFundamentals, normalized),
      };
    },
  });
}

import { useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import { Cell, Legend, Pie, PieChart, ResponsiveContainer, Tooltip } from "recharts";
import type { AppError } from "../types";
import type { Asset, AssetFavorite } from "../portfolio/types";
import { ASSET_CLASS_LABELS, type AssetClass } from "../portfolio/types";
import type { StockQuote } from "../collector/types";
import { latestForTicker } from "../collector/latestForTicker";
import type { WorkspaceSummary, MonthlyReturn } from "./types";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

// Mesma paleta/convenção de `portfolio/SummarySection.tsx` — duplicada
// localmente em vez de extraída pra um módulo compartilhado, seguindo a
// convenção já estabelecida no projeto (ver `finances/LiabilitySection.tsx`,
// Sessão 82).
const CLASS_COLORS: Partial<Record<AssetClass, string>> = {
  acao_br: "#3987e5",
  fii: "#d95926",
  etf_br: "#199e70",
  cripto: "#c98500",
  bdr: "#d55181",
  metal: "#008300",
  acao_internacional: "#9085e9",
  reit: "#e66767",
};
const OTHER_COLOR = "#9fb1c2"; // --muted-foreground

function formatCurrency(value: number): string {
  return value.toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

function formatPct(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

function plColorClass(value: number | null): string {
  if (value == null || value === 0) return "text-muted-foreground";
  return value > 0 ? "text-primary" : "text-destructive";
}

function KpiCard({
  title,
  value,
  valueClassName,
  subtext,
}: {
  title: string;
  value: string;
  valueClassName?: string;
  subtext?: string;
}) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <p className="text-sm text-muted-foreground">{title}</p>
      <p className={`mt-1 text-2xl font-semibold ${valueClassName ?? ""}`}>{value}</p>
      {subtext && <p className="mt-1 text-xs text-muted-foreground">{subtext}</p>}
    </div>
  );
}

// Mesmo rebaseamento de `SummarySection.tsx::trailingTwelveMonthPct` — a
// série de `get_workspace_profitability` já vem encadeada
// (`r_cumulative_pct`, `domain::twr::chain`), então isolar "últimos 12
// meses" é só dividir o índice do mês mais recente pelo de 12 meses atrás.
function trailingTwelveMonthPct(monthly: MonthlyReturn[]): number | null {
  if (monthly.length === 0) return null;
  if (monthly.length <= 12) return monthly[monthly.length - 1].r_cumulative_pct;
  const end = monthly[monthly.length - 1];
  const baseline = monthly[monthly.length - 13];
  const endIndex = 1 + end.r_cumulative_pct / 100;
  const baseIndex = 1 + baseline.r_cumulative_pct / 100;
  return (endIndex / baseIndex - 1) * 100;
}

function AllocationTooltip({
  active,
  payload,
}: {
  active?: boolean;
  payload?: { payload: { asset_class: string; market_value: number } }[];
}) {
  if (!active || !payload?.length) return null;
  const row = payload[0].payload;
  return (
    <div
      className="rounded-lg border border-border px-3 py-2 text-sm"
      style={{ background: "#111820" }}
    >
      <p className="font-medium">{ASSET_CLASS_LABELS[row.asset_class as AssetClass] ?? row.asset_class}</p>
      <p className="text-muted-foreground">{formatCurrency(row.market_value)}</p>
    </div>
  );
}

// Home do Workspace, fatia 1 (layout fixo, ver PHASE.md item 10) — agrega
// todas as carteiras do Workspace, separada do "Resumo" de cada Carteira
// (`portfolio/SummarySection.tsx`, que continua existindo do jeito que
// está). Painel de mercado reaproveita o mecanismo de Favoritos (Fase 10.4)
// como está — sem coluna de ações aqui, gerenciar continua em Watchlists.
function HomeSection({ workspaceId }: { workspaceId: number }) {
  const summaryQuery = useQuery<WorkspaceSummary, AppError>({
    queryKey: ["workspace-summary", workspaceId],
    queryFn: () => invoke("get_workspace_summary", { workspaceId }),
  });
  const profitabilityQuery = useQuery<MonthlyReturn[], AppError>({
    queryKey: ["workspace-profitability", workspaceId],
    queryFn: () => invoke("get_workspace_profitability", { workspaceId }),
  });
  const assetsQuery = useQuery<Asset[], AppError>({
    queryKey: ["assets"],
    queryFn: () => invoke("list_assets"),
  });
  const quotesQuery = useQuery<StockQuote[], AppError>({
    queryKey: ["stock-quotes"],
    queryFn: () => invoke("list_stock_quotes"),
  });
  const favoritesQuery = useQuery<AssetFavorite[], AppError>({
    queryKey: ["asset-favorites", workspaceId],
    queryFn: () => invoke("list_favorite_assets", { workspaceId }),
  });

  const summary = summaryQuery.data;
  const monthly = profitabilityQuery.data ?? [];
  const assets = assetsQuery.data ?? [];
  const quotes = quotesQuery.data ?? [];
  const favorites = favoritesQuery.data ?? [];

  const totalReturnPct = monthly.length > 0 ? monthly[monthly.length - 1].r_cumulative_pct : null;
  const twelveMonthReturnPct = useMemo(() => trailingTwelveMonthPct(monthly), [monthly]);

  function assetFor(assetId: number): Asset | undefined {
    return assets.find((a) => a.id === assetId);
  }

  function currentPriceFor(assetId: number): number | null {
    const asset = assetFor(assetId);
    if (!asset) return null;
    return latestForTicker(quotes, asset.ticker)?.price ?? null;
  }

  if (summaryQuery.isError) {
    return <p className="text-red-600">{summaryQuery.error.message}</p>;
  }
  if (summaryQuery.isLoading || !summary) {
    return <p className="text-muted-foreground">Loading...</p>;
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <KpiCard
          title="Total portfolio value"
          value={formatCurrency(summary.total_market_value)}
          subtext={`Invested: ${formatCurrency(summary.total_cost_basis)}`}
        />
        <KpiCard
          title="Profit"
          value={formatCurrency(summary.unrealized_pl)}
          valueClassName={plColorClass(summary.unrealized_pl)}
          subtext={
            summary.unrealized_pl_pct != null
              ? formatPct(summary.unrealized_pl_pct * 100)
              : undefined
          }
        />
        <KpiCard
          title="Dividends received (12M)"
          value={formatCurrency(summary.dividends_received_12m)}
        />
        <KpiCard
          title="Return"
          value={twelveMonthReturnPct != null ? formatPct(twelveMonthReturnPct) : "—"}
          valueClassName={plColorClass(twelveMonthReturnPct)}
          subtext={
            totalReturnPct != null
              ? `Total: ${formatPct(totalReturnPct)}`
              : "No return history yet"
          }
        />
      </div>

      {summary.positions_missing_price > 0 && (
        <p className="text-sm text-muted-foreground">
          {summary.positions_missing_price} position{summary.positions_missing_price === 1 ? "" : "s"} without a
          registered price/valuation — the totals above don't include {summary.positions_missing_price === 1 ? "it" : "them"}.
        </p>
      )}

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Allocation by class</CardTitle>
          </CardHeader>
          <CardContent>
            {summary.allocation_by_class.length === 0 ? (
              <p className="text-muted-foreground">
                No priced positions yet in this Workspace.
              </p>
            ) : (
              <ResponsiveContainer width="100%" height={280}>
                <PieChart>
                  <Pie
                    data={summary.allocation_by_class}
                    dataKey="market_value"
                    nameKey="asset_class"
                    innerRadius={64}
                    outerRadius={104}
                    paddingAngle={2}
                    label={(props: { percent?: number }) =>
                      `${((props.percent ?? 0) * 100).toFixed(0)}%`
                    }
                  >
                    {summary.allocation_by_class.map((entry) => (
                      <Cell
                        key={entry.asset_class}
                        fill={CLASS_COLORS[entry.asset_class as AssetClass] ?? OTHER_COLOR}
                        stroke="#111820"
                        strokeWidth={2}
                      />
                    ))}
                  </Pie>
                  <Tooltip content={<AllocationTooltip />} />
                  <Legend
                    formatter={(_value, entry) => {
                      const payload = (entry as { payload?: { asset_class: string } }).payload;
                      const assetClass = payload?.asset_class ?? "";
                      return ASSET_CLASS_LABELS[assetClass as AssetClass] ?? assetClass;
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Portfolios</CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Portfolio</TableHead>
                  <TableHead>Market value</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {summary.portfolios.length === 0 && (
                  <TableRow>
                    <TableCell colSpan={2} className="text-center text-muted-foreground">
                      No portfolios in this Workspace yet.
                    </TableCell>
                  </TableRow>
                )}
                {summary.portfolios.map((p) => (
                  <TableRow key={p.portfolio_id}>
                    <TableCell>{p.name}</TableCell>
                    <TableCell>{formatCurrency(p.market_value)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>★ Market panel</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="mb-4 text-sm text-muted-foreground">
            Favorited assets (manage the star on the "Assets"/"Watchlists" tabs) — useful to keep
            an eye on something without owning it, e.g. a market index.
          </p>
          {favoritesQuery.isError && (
            <p className="mb-3 text-red-600">{favoritesQuery.error.message}</p>
          )}
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Ticker</TableHead>
                <TableHead>Name</TableHead>
                <TableHead>Current price</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {favorites.length === 0 && (
                <TableRow>
                  <TableCell colSpan={3} className="text-center text-muted-foreground">
                    No favorited assets yet.
                  </TableCell>
                </TableRow>
              )}
              {favorites.map((favorite) => {
                const asset = assetFor(favorite.asset_id);
                const price = currentPriceFor(favorite.asset_id);
                return (
                  <TableRow key={favorite.id}>
                    <TableCell>{asset?.ticker ?? `#${favorite.asset_id}`}</TableCell>
                    <TableCell>{asset?.name ?? "—"}</TableCell>
                    <TableCell>{price !== null ? price.toFixed(2) : "—"}</TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
}

export default HomeSection;

import { useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Line,
  LineChart,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { AppError } from "../types";
import { ASSET_CLASSES_WITH_AUTO_QUOTE, type MonthlyReturn, type PositionView } from "./types";
import { Button } from "@/components/ui/button";
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

type CollectorSummary = { success: boolean; output: string };

// Mesma paleta/convenção de DividendHistoryChart.tsx (rampa de verde do
// tema, nunca dois eixos Y na mesma barra — ver skill dataviz). A polaridade
// (mês positivo/negativo) já é lida pela posição da barra em torno da linha
// de referência zero, não pela cor — evita o par verde/vermelho clássico
// (falha de separação pra daltonismo, confirmado pelo validador da skill).
const BAR_COLOR = "#4ade80"; // --chart-1
const LINE_COLOR = "#22c55e"; // --chart-3
const GRID_COLOR = "#1f2630"; // --border
const AXIS_COLOR = "#9fb1c2"; // --muted-foreground
const TOOLTIP_BG = "#111820"; // --card
const REFERENCE_COLOR = "#3a4553";

const MONTH_LABELS = [
  "Jan", "Fev", "Mar", "Abr", "Mai", "Jun",
  "Jul", "Ago", "Set", "Out", "Nov", "Dez",
];

function monthLabel(yearMonth: string): string {
  const [year, month] = yearMonth.split("-");
  return `${MONTH_LABELS[Number(month) - 1]}/${year.slice(2)}`;
}

function formatPct(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}%`;
}

function formatCurrency(value: number): string {
  return value.toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

function MonthTooltip({
  active,
  payload,
  field,
}: {
  active?: boolean;
  payload?: { payload: MonthlyReturn }[];
  field: "r_month_pct" | "r_cumulative_pct";
}) {
  if (!active || !payload?.length) return null;
  const row = payload[0].payload;
  return (
    <div
      className="rounded-lg border border-border px-3 py-2 text-sm"
      style={{ background: TOOLTIP_BG }}
    >
      <p className="font-medium">{monthLabel(row.year_month)}</p>
      <p className="text-muted-foreground">{formatPct(row[field])}</p>
    </div>
  );
}

function ProfitabilitySection({ portfolioId }: { portfolioId: number }) {
  const queryClient = useQueryClient();

  const positionsQuery = useQuery<PositionView[], AppError>({
    queryKey: ["positions", portfolioId],
    queryFn: () => invoke("get_portfolio_positions", { portfolioId }),
  });

  const profitabilityQuery = useQuery<MonthlyReturn[], AppError>({
    queryKey: ["profitability", portfolioId],
    queryFn: () => invoke("get_portfolio_profitability", { portfolioId }),
  });

  // Ativos zerados (já vendidos por completo) continuam em
  // get_portfolio_positions — precisam do backfill também, pra precificar
  // os meses passados em que ainda eram carregados.
  const autoQuoteTickers = useMemo(() => {
    const positions = positionsQuery.data ?? [];
    const autoQuoteClasses = ASSET_CLASSES_WITH_AUTO_QUOTE as string[];
    const tickers = positions
      .filter((p) => autoQuoteClasses.includes(p.asset_class))
      .map((p) => p.ticker);
    return [...new Set(tickers)];
  }, [positionsQuery.data]);

  const backfillMutation = useMutation<CollectorSummary, AppError, string[]>({
    mutationFn: (tickers) => invoke("run_price_history_backfill", { tickers }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["profitability", portfolioId] });
    },
  });

  const monthlyReturns = profitabilityQuery.data ?? [];
  const chartData = monthlyReturns.map((row) => ({ ...row, label: monthLabel(row.year_month) }));

  return (
    <div className="flex flex-col gap-6">
      <Card>
        <CardHeader>
          <CardTitle>Rentabilidade (TWR / Dietz Modificado)</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <p className="text-sm text-muted-foreground">
            Cobre Ação BR e FII nesta fatia — aporte/retirada e as demais classes (Tesouro
            Direto, Renda Fixa, ações internacionais) ainda não entram nesse cálculo, por falta
            de preço histórico automatizado pra elas.
          </p>

          <div>
            <Button
              type="button"
              variant="outline"
              onClick={() => backfillMutation.mutate(autoQuoteTickers)}
              disabled={backfillMutation.isPending || autoQuoteTickers.length === 0}
            >
              {backfillMutation.isPending ? "Atualizando..." : "Atualizar histórico de preços"}
            </Button>
            {autoQuoteTickers.length === 0 && (
              <p className="mt-2 text-sm text-muted-foreground">
                Nenhum ativo Ação BR ou FII na carteira ainda.
              </p>
            )}
          </div>

          {backfillMutation.isError && (
            <p className="text-red-600">{backfillMutation.error.message}</p>
          )}
          {profitabilityQuery.isError && (
            <p className="text-red-600">
              {profitabilityQuery.error.message} — se for preço faltando, rode "Atualizar
              histórico de preços" acima.
            </p>
          )}

          {chartData.length > 0 && (
            <>
              <div>
                <p className="mb-1 text-sm text-muted-foreground">Rentabilidade mensal</p>
                <ResponsiveContainer width="100%" height={200}>
                  <BarChart data={chartData} margin={{ top: 4, right: 8, left: 0, bottom: 0 }}>
                    <CartesianGrid stroke={GRID_COLOR} vertical={false} />
                    <XAxis dataKey="label" stroke={AXIS_COLOR} fontSize={12} tickLine={false} />
                    <YAxis
                      stroke={AXIS_COLOR}
                      fontSize={12}
                      tickLine={false}
                      axisLine={false}
                      width={48}
                      tickFormatter={(value: number) => `${value}%`}
                    />
                    <ReferenceLine y={0} stroke={REFERENCE_COLOR} />
                    <Tooltip
                      content={<MonthTooltip field="r_month_pct" />}
                      cursor={{ fill: GRID_COLOR }}
                    />
                    <Bar dataKey="r_month_pct" fill={BAR_COLOR} radius={[4, 4, 4, 4]} />
                  </BarChart>
                </ResponsiveContainer>
              </div>

              <div>
                <p className="mb-1 text-sm text-muted-foreground">Rentabilidade acumulada</p>
                <ResponsiveContainer width="100%" height={200}>
                  <LineChart data={chartData} margin={{ top: 4, right: 8, left: 0, bottom: 0 }}>
                    <CartesianGrid stroke={GRID_COLOR} vertical={false} />
                    <XAxis dataKey="label" stroke={AXIS_COLOR} fontSize={12} tickLine={false} />
                    <YAxis
                      stroke={AXIS_COLOR}
                      fontSize={12}
                      tickLine={false}
                      axisLine={false}
                      width={48}
                      tickFormatter={(value: number) => `${value}%`}
                    />
                    <ReferenceLine y={0} stroke={REFERENCE_COLOR} />
                    <Tooltip
                      content={<MonthTooltip field="r_cumulative_pct" />}
                      cursor={{ stroke: GRID_COLOR }}
                    />
                    <Line
                      type="monotone"
                      dataKey="r_cumulative_pct"
                      stroke={LINE_COLOR}
                      strokeWidth={2}
                      dot={false}
                    />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            </>
          )}

          {!profitabilityQuery.isError && chartData.length === 0 && (
            <p className="text-muted-foreground">
              Nenhum lançamento de Ação BR ou FII ainda — lance uma compra na aba "Lançamentos &
              Posições" pra começar a acompanhar a rentabilidade.
            </p>
          )}

          {chartData.length > 0 && (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Mês</TableHead>
                  <TableHead>BMV</TableHead>
                  <TableHead>Fluxo de caixa</TableHead>
                  <TableHead>EMV</TableHead>
                  <TableHead>R mês</TableHead>
                  <TableHead>R acumulado</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {monthlyReturns.map((row) => (
                  <TableRow key={row.year_month}>
                    <TableCell>{monthLabel(row.year_month)}</TableCell>
                    <TableCell>{formatCurrency(row.bmv)}</TableCell>
                    <TableCell>{formatCurrency(row.cf_total)}</TableCell>
                    <TableCell>{formatCurrency(row.emv)}</TableCell>
                    <TableCell>{formatPct(row.r_month_pct)}</TableCell>
                    <TableCell>{formatPct(row.r_cumulative_pct)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default ProfitabilitySection;

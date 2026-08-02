import { useMemo, useState } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { Button } from "@/components/ui/button";
import type { StockDividendPayment } from "../collector/types";

type Granularity = "monthly" | "yearly";

type Bucket = {
  period: string;
  label: string;
  amount: number;
  avgYield: number | null;
};

// Mesma rampa de verde já definida em index.css (--chart-1.._5) pro tema —
// um gráfico de barra por medida (nunca dois eixos Y na mesma barra, ver
// skill dataviz) e cada um usa uma parada diferente da mesma família,
// já que os dois painéis são sobre o mesmo assunto (dividendo).
const AMOUNT_COLOR = "#4ade80"; // --chart-1
const YIELD_COLOR = "#22c55e"; // --chart-3
const GRID_COLOR = "#1f2630"; // --border
const AXIS_COLOR = "#9fb1c2"; // --muted-foreground
const TOOLTIP_BG = "#111820"; // --card

const MONTH_LABELS = [
  "Jan", "Feb", "Mar", "Apr", "May", "Jun",
  "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

function bucketKey(paymentDate: string, granularity: Granularity): string {
  return granularity === "monthly" ? paymentDate.slice(0, 7) : paymentDate.slice(0, 4);
}

function bucketLabel(key: string, granularity: Granularity): string {
  if (granularity === "yearly") return key;
  const [year, month] = key.split("-");
  return `${MONTH_LABELS[Number(month) - 1]}/${year.slice(2)}`;
}

// Agrupa os pagamentos individuais por mês ou ano: soma o valor pago, e
// tira a média do yield de cada pagamento (não soma % — não faz sentido
// somar percentuais de pagamentos diferentes no mesmo período).
function groupPayments(
  payments: StockDividendPayment[],
  granularity: Granularity,
): Bucket[] {
  const buckets = new Map<string, { amount: number; yieldSum: number; yieldCount: number }>();

  for (const payment of payments) {
    const key = bucketKey(payment.payment_date, granularity);
    const bucket = buckets.get(key) ?? { amount: 0, yieldSum: 0, yieldCount: 0 };
    bucket.amount += payment.amount;
    if (payment.yield_pct != null) {
      bucket.yieldSum += payment.yield_pct;
      bucket.yieldCount += 1;
    }
    buckets.set(key, bucket);
  }

  return [...buckets.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([key, bucket]) => ({
      period: key,
      label: bucketLabel(key, granularity),
      amount: bucket.amount,
      avgYield: bucket.yieldCount > 0 ? bucket.yieldSum / bucket.yieldCount : null,
    }));
}

function AmountTooltip({ active, payload }: { active?: boolean; payload?: { payload: Bucket }[] }) {
  if (!active || !payload?.length) return null;
  const bucket = payload[0].payload;
  return (
    <div
      className="rounded-lg border border-border px-3 py-2 text-sm"
      style={{ background: TOOLTIP_BG }}
    >
      <p className="font-medium">{bucket.label}</p>
      <p className="text-muted-foreground">R$ {bucket.amount.toFixed(2)}/share</p>
    </div>
  );
}

function YieldTooltip({ active, payload }: { active?: boolean; payload?: { payload: Bucket }[] }) {
  if (!active || !payload?.length) return null;
  const bucket = payload[0].payload;
  if (bucket.avgYield == null) return null;
  return (
    <div
      className="rounded-lg border border-border px-3 py-2 text-sm"
      style={{ background: TOOLTIP_BG }}
    >
      <p className="font-medium">{bucket.label}</p>
      <p className="text-muted-foreground">{bucket.avgYield.toFixed(2)}% at the payment-date price</p>
    </div>
  );
}

function DividendHistoryChart({ payments }: { payments: StockDividendPayment[] }) {
  const [granularity, setGranularity] = useState<Granularity>("monthly");
  const data = useMemo(() => groupPayments(payments, granularity), [payments, granularity]);

  if (payments.length === 0) {
    return <p className="text-muted-foreground">No dividend payments found.</p>;
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex gap-2">
        <Button
          type="button"
          size="sm"
          variant={granularity === "monthly" ? "default" : "outline"}
          onClick={() => setGranularity("monthly")}
        >
          Monthly
        </Button>
        <Button
          type="button"
          size="sm"
          variant={granularity === "yearly" ? "default" : "outline"}
          onClick={() => setGranularity("yearly")}
        >
          Yearly
        </Button>
      </div>

      <div>
        <p className="mb-1 text-sm text-muted-foreground">Amount paid (R$/share)</p>
        <ResponsiveContainer width="100%" height={180}>
          <BarChart data={data} margin={{ top: 4, right: 8, left: 0, bottom: 0 }}>
            <CartesianGrid stroke={GRID_COLOR} vertical={false} />
            <XAxis dataKey="label" stroke={AXIS_COLOR} fontSize={12} tickLine={false} />
            <YAxis stroke={AXIS_COLOR} fontSize={12} tickLine={false} axisLine={false} width={48} />
            <Tooltip content={<AmountTooltip />} cursor={{ fill: GRID_COLOR }} />
            <Bar dataKey="amount" fill={AMOUNT_COLOR} radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>

      <div>
        <p className="mb-1 text-sm text-muted-foreground">
          Yield at payment (% of the price at that time)
        </p>
        <ResponsiveContainer width="100%" height={180}>
          <BarChart data={data} margin={{ top: 4, right: 8, left: 0, bottom: 0 }}>
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
            <Tooltip content={<YieldTooltip />} cursor={{ fill: GRID_COLOR }} />
            <Bar dataKey="avgYield" fill={YIELD_COLOR} radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

export default DividendHistoryChart;

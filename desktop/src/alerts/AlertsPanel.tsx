import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError, ValuationModel } from "../types";
import { INDICATORS, INDICATOR_KEYS, type IndicatorKey } from "../stock-lookup/cryptoIndicators";
import Field from "../components/Field";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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

const MODEL_LABELS: Record<string, string> = {
  bazin: "Bazin",
  graham: "Graham",
  gordon: "Gordon / DDM",
  dcf: "DCF / FCFF",
  banks: "Banks (P/B)",
  rnav: "RNAV",
  projected_ceiling: "Projected Ceiling",
};

type TargetType = "stock_price" | "crypto_indicator";

const STOCK_CONDITIONS = ["BELOW_FAIR_PRICE", "ABOVE_FAIR_PRICE"] as const;
const CRYPTO_CONDITIONS = ["SIGNAL_GREEN", "SIGNAL_RED"] as const;

const CONDITION_LABELS: Record<string, string> = {
  BELOW_FAIR_PRICE: "Price drops below fair price",
  ABOVE_FAIR_PRICE: "Price rises above fair price",
  SIGNAL_GREEN: "Signal turns GREEN",
  SIGNAL_RED: "Signal turns RED",
};

type AlertRuleView = {
  id: number;
  target_type: TargetType;
  condition: string;
  is_active: boolean;
  created_at: string;
  valuation_id: number | null;
  ticker: string | null;
  fair_price: number | null;
  coin: string | null;
  indicator: string | null;
  is_triggered: boolean;
  last_message: string | null;
};

type CreateAlertRuleRequest = {
  target_type: TargetType;
  condition: string;
  valuation_id: number | null;
  coin: string | null;
  indicator: string | null;
};

// Same "way more precision than a table needs" timestamp shape as
// SavedValuationsPanel — reused here rather than extracted, this is only
// the second place that needs it.
function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function targetLabel(rule: AlertRuleView): string {
  if (rule.target_type === "stock_price") {
    const price = rule.fair_price !== null ? `R$ ${rule.fair_price.toFixed(2)}` : "—";
    return `${rule.ticker ?? "?"} (fair price ${price})`;
  }
  const label = rule.indicator ? INDICATORS[rule.indicator as IndicatorKey] ?? rule.indicator : "?";
  return `${rule.coin ?? "?"} — ${label}`;
}

function AlertsPanel() {
  const [targetType, setTargetType] = useState<TargetType>("stock_price");
  const [condition, setCondition] = useState<string>(STOCK_CONDITIONS[0]);
  const [selectedValuationId, setSelectedValuationId] = useState("");
  const [coin, setCoin] = useState("ETH");
  const [indicator, setIndicator] = useState<IndicatorKey>(INDICATOR_KEYS[0]);
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<number | null>(null);

  const queryClient = useQueryClient();

  const alertRulesQuery = useQuery<AlertRuleView[], AppError>({
    queryKey: ["alert-rules"],
    queryFn: () => invoke("list_alert_rules"),
    // Picks up the background checker's (alert_checker.rs, Fase 5.2) state
    // without a push-event system — good enough for a 5-minute check cycle.
    refetchInterval: 30_000,
  });

  // Reuses the same queryKey as SavedValuationsPanel, so TanStack Query
  // dedupes/caches it instead of firing a second `list_valuations` call.
  const valuationsQuery = useQuery<ValuationModel[], AppError>({
    queryKey: ["valuations"],
    queryFn: () => invoke("list_valuations"),
  });

  const valuationOptions = (valuationsQuery.data ?? []).filter(
    (v) => v.fair_price !== null,
  );

  const createMutation = useMutation<AlertRuleView, AppError, CreateAlertRuleRequest>({
    mutationFn: (request) => invoke("create_alert_rule", { request }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["alert-rules"] });
      setSelectedValuationId("");
    },
  });

  const toggleActiveMutation = useMutation<
    AlertRuleView,
    AppError,
    { alertRuleId: number; isActive: boolean }
  >({
    mutationFn: ({ alertRuleId, isActive }) =>
      invoke("set_alert_rule_active", {
        request: { alert_rule_id: alertRuleId, is_active: isActive },
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["alert-rules"] }),
  });

  const deleteMutation = useMutation<void, AppError, number>({
    mutationFn: (alertRuleId) => invoke("delete_alert_rule", { alertRuleId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["alert-rules"] });
      setConfirmingDeleteId(null);
    },
  });

  function handleDeleteClick(id: number) {
    if (confirmingDeleteId === id) {
      deleteMutation.mutate(id);
    } else {
      setConfirmingDeleteId(id);
    }
  }

  function handleTargetTypeChange(value: TargetType) {
    setTargetType(value);
    setCondition(value === "stock_price" ? STOCK_CONDITIONS[0] : CRYPTO_CONDITIONS[0]);
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    createMutation.mutate({
      target_type: targetType,
      condition,
      valuation_id: targetType === "stock_price" ? Number(selectedValuationId) : null,
      coin: targetType === "crypto_indicator" ? coin.toUpperCase() : null,
      indicator: targetType === "crypto_indicator" ? indicator : null,
    });
  }

  const canSubmit =
    targetType === "stock_price" ? selectedValuationId !== "" : coin.trim() !== "";

  const rules = alertRulesQuery.data ?? [];

  return (
    <Card>
      <CardHeader>
        <CardTitle>Alerts</CardTitle>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="mb-8 flex flex-col gap-4">
          <Field label="Alert type">
            <Select value={targetType} onValueChange={handleTargetTypeChange}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="stock_price">Stock price</SelectItem>
                <SelectItem value="crypto_indicator">Crypto indicator</SelectItem>
              </SelectContent>
            </Select>
          </Field>

          {targetType === "stock_price" ? (
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label="Saved valuation">
                <Select value={selectedValuationId} onValueChange={setSelectedValuationId}>
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select a valuation" />
                  </SelectTrigger>
                  <SelectContent>
                    {valuationOptions.map((v) => (
                      <SelectItem key={v.id} value={String(v.id)}>
                        {v.ticker} — {MODEL_LABELS[v.model] ?? v.model} — fair price R${" "}
                        {v.fair_price!.toFixed(2)}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>

              <Field label="Condition">
                <Select value={condition} onValueChange={setCondition}>
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {STOCK_CONDITIONS.map((key) => (
                      <SelectItem key={key} value={key}>
                        {CONDITION_LABELS[key]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>
            </div>
          ) : (
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
              <Field label="Coin">
                <Input value={coin} onChange={(e) => setCoin(e.currentTarget.value)} />
              </Field>

              <Field label="Indicator">
                <Select
                  value={indicator}
                  onValueChange={(value) => setIndicator(value as IndicatorKey)}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {INDICATOR_KEYS.map((key) => (
                      <SelectItem key={key} value={key}>
                        {INDICATORS[key]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>

              <Field label="Condition">
                <Select value={condition} onValueChange={setCondition}>
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {CRYPTO_CONDITIONS.map((key) => (
                      <SelectItem key={key} value={key}>
                        {CONDITION_LABELS[key]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>
            </div>
          )}

          {createMutation.isError && (
            <p className="text-red-600">{createMutation.error.message}</p>
          )}

          <Button type="submit" disabled={!canSubmit || createMutation.isPending} className="w-fit">
            {createMutation.isPending ? "Creating..." : "Create alert rule"}
          </Button>
        </form>

        {alertRulesQuery.isError && (
          <p className="mb-3 text-red-600">{alertRulesQuery.error.message}</p>
        )}

        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Type</TableHead>
              <TableHead>Target</TableHead>
              <TableHead>Condition</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Created at</TableHead>
              <TableHead>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rules.length === 0 && (
              <TableRow>
                <TableCell colSpan={6} className="text-center text-muted-foreground">
                  No alert rules yet.
                </TableCell>
              </TableRow>
            )}
            {rules.map((rule) => {
              const isConfirming = confirmingDeleteId === rule.id;
              return (
                <TableRow key={rule.id}>
                  <TableCell>
                    {rule.target_type === "stock_price" ? "Stock price" : "Crypto indicator"}
                  </TableCell>
                  <TableCell>{targetLabel(rule)}</TableCell>
                  <TableCell>{CONDITION_LABELS[rule.condition] ?? rule.condition}</TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <Badge
                        className={
                          rule.is_active
                            ? "bg-green-100 text-green-800 dark:bg-green-950 dark:text-green-300"
                            : "bg-muted text-muted-foreground"
                        }
                      >
                        {rule.is_active ? "Active" : "Paused"}
                      </Badge>
                      {rule.is_triggered && (
                        <Badge
                          className="bg-red-100 text-red-800 dark:bg-red-950 dark:text-red-300"
                          title={rule.last_message ?? undefined}
                        >
                          Triggered
                        </Badge>
                      )}
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() =>
                          toggleActiveMutation.mutate({
                            alertRuleId: rule.id,
                            isActive: !rule.is_active,
                          })
                        }
                      >
                        {rule.is_active ? "Pause" : "Resume"}
                      </Button>
                    </div>
                  </TableCell>
                  <TableCell>{formatDateTime(rule.created_at)}</TableCell>
                  <TableCell>
                    <Button
                      variant={isConfirming ? "destructive" : "outline"}
                      size="sm"
                      onClick={() => handleDeleteClick(rule.id)}
                    >
                      {isConfirming ? "Confirm?" : "Delete"}
                    </Button>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}

export default AlertsPanel;

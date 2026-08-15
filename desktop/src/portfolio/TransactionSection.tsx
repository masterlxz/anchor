import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
import {
  FIXED_INCOME_CLASSES,
  FI_INDEXADORES,
  FI_LIQUIDEZ_OPTIONS,
  TRANSACTION_TYPES,
  TRANSACTION_TYPE_LABELS,
  type Asset,
  type Custodia,
  type PositionView,
  type TransactionType,
  type TransactionView,
} from "./types";
import Field from "../components/Field";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
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

type CreateTransactionRequest = {
  portfolio_id: number;
  asset_id: number | null;
  custodia_id: number | null;
  transfer_to_custodia_id: number | null;
  transaction_type: string;
  quantity: number | null;
  unit_price: number | null;
  total_value: number;
  fee: number | null;
  transaction_date: string;
  notes: string | null;
  fi_emissor: string | null;
  fi_indexador: string | null;
  fi_taxa_percentual: number | null;
  fi_data_vencimento: string | null;
  fi_liquidez: string | null;
};

function todayIso(): string {
  return new Date().toISOString().slice(0, 10);
}

function needsAsset(type: TransactionType): boolean {
  return type !== "aporte" && type !== "retirada";
}

function needsQuantity(type: TransactionType): boolean {
  return type === "compra" || type === "venda" || type === "transferencia";
}

function needsUnitPrice(type: TransactionType): boolean {
  return type === "compra" || type === "venda";
}

function needsTransferDestination(type: TransactionType): boolean {
  return type === "transferencia";
}

function TransactionSection({
  workspaceId,
  portfolioId,
}: {
  workspaceId: number;
  portfolioId: number;
}) {
  const [transactionType, setTransactionType] = useState<TransactionType>("compra");
  const [assetId, setAssetId] = useState("");
  const [custodiaId, setCustodiaId] = useState("");
  const [transferToCustodiaId, setTransferToCustodiaId] = useState("");
  const [quantity, setQuantity] = useState("");
  const [unitPrice, setUnitPrice] = useState("");
  const [totalValue, setTotalValue] = useState("");
  const [fee, setFee] = useState("");
  const [transactionDate, setTransactionDate] = useState(todayIso());
  const [notes, setNotes] = useState("");
  const [fiEmissor, setFiEmissor] = useState("");
  const [fiIndexador, setFiIndexador] = useState<string>(FI_INDEXADORES[0]);
  const [fiTaxaPercentual, setFiTaxaPercentual] = useState("");
  const [fiDataVencimento, setFiDataVencimento] = useState("");
  const [fiLiquidez, setFiLiquidez] = useState<string>(FI_LIQUIDEZ_OPTIONS[0]);

  const queryClient = useQueryClient();

  const assetsQuery = useQuery<Asset[], AppError>({
    queryKey: ["assets"],
    queryFn: () => invoke("list_assets"),
  });
  const custodiasQuery = useQuery<Custodia[], AppError>({
    queryKey: ["custodias", workspaceId],
    queryFn: () => invoke("list_custodias", { workspaceId }),
  });
  const positionsQuery = useQuery<PositionView[], AppError>({
    queryKey: ["positions", portfolioId],
    queryFn: () => invoke("get_portfolio_positions", { portfolioId }),
  });

  const assets = assetsQuery.data ?? [];
  const custodias = custodiasQuery.data ?? [];
  const selectedAsset = assets.find((a) => a.id === Number(assetId));
  const showFixedIncomeFields =
    transactionType === "compra" &&
    selectedAsset !== undefined &&
    (FIXED_INCOME_CLASSES as string[]).includes(selectedAsset.asset_class);

  function invalidateAfterWrite() {
    queryClient.invalidateQueries({ queryKey: ["transactions", portfolioId] });
    queryClient.invalidateQueries({ queryKey: ["positions", portfolioId] });
  }

  const createMutation = useMutation<TransactionView, AppError, CreateTransactionRequest>({
    mutationFn: (request) => invoke("create_transaction", { request }),
    onSuccess: () => {
      invalidateAfterWrite();
      setQuantity("");
      setUnitPrice("");
      setTotalValue("");
      setFee("");
      setNotes("");
      setFiTaxaPercentual("");
      setFiDataVencimento("");
    },
  });

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    createMutation.mutate({
      portfolio_id: portfolioId,
      asset_id: needsAsset(transactionType) && assetId !== "" ? Number(assetId) : null,
      custodia_id: custodiaId !== "" ? Number(custodiaId) : null,
      transfer_to_custodia_id:
        needsTransferDestination(transactionType) && transferToCustodiaId !== ""
          ? Number(transferToCustodiaId)
          : null,
      transaction_type: transactionType,
      quantity: needsQuantity(transactionType) && quantity !== "" ? Number(quantity) : null,
      unit_price: needsUnitPrice(transactionType) && unitPrice !== "" ? Number(unitPrice) : null,
      total_value: Number(totalValue),
      fee: fee !== "" ? Number(fee) : null,
      transaction_date: transactionDate,
      notes: notes.trim() === "" ? null : notes,
      fi_emissor: showFixedIncomeFields && fiEmissor.trim() !== "" ? fiEmissor : null,
      fi_indexador: showFixedIncomeFields ? fiIndexador : null,
      fi_taxa_percentual: showFixedIncomeFields && fiTaxaPercentual !== "" ? Number(fiTaxaPercentual) : null,
      fi_data_vencimento: showFixedIncomeFields && fiDataVencimento !== "" ? fiDataVencimento : null,
      fi_liquidez: showFixedIncomeFields ? fiLiquidez : null,
    });
  }

  const canSubmit =
    totalValue !== "" &&
    (!needsAsset(transactionType) || assetId !== "") &&
    (!needsQuantity(transactionType) || quantity !== "") &&
    (!needsUnitPrice(transactionType) || unitPrice !== "") &&
    (!needsTransferDestination(transactionType) ||
      (custodiaId !== "" && transferToCustodiaId !== "" && custodiaId !== transferToCustodiaId));

  const positions = positionsQuery.data ?? [];

  return (
    <div className="flex flex-col gap-6">
      <Card>
        <CardHeader>
          <CardTitle>New transaction</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
              <Field label="Type">
                <Select
                  value={transactionType}
                  onValueChange={(value) => setTransactionType(value as TransactionType)}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {TRANSACTION_TYPES.map((key) => (
                      <SelectItem key={key} value={key}>
                        {TRANSACTION_TYPE_LABELS[key]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>

              <Field label="Date">
                <Input
                  required
                  type="date"
                  value={transactionDate}
                  onChange={(e) => setTransactionDate(e.currentTarget.value)}
                />
              </Field>

              {needsAsset(transactionType) && (
                <Field label="Asset">
                  <Select value={assetId} onValueChange={setAssetId}>
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="Select an asset" />
                    </SelectTrigger>
                    <SelectContent>
                      {assets.map((asset) => (
                        <SelectItem key={asset.id} value={String(asset.id)}>
                          {asset.ticker} — {asset.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
              )}
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-4">
              {needsQuantity(transactionType) && (
                <Field label="Quantity">
                  <Input
                    required
                    type="number"
                    step="any"
                    value={quantity}
                    onChange={(e) => setQuantity(e.currentTarget.value)}
                  />
                </Field>
              )}
              {needsUnitPrice(transactionType) && (
                <Field label="Unit price">
                  <Input
                    required
                    type="number"
                    step="any"
                    value={unitPrice}
                    onChange={(e) => setUnitPrice(e.currentTarget.value)}
                  />
                </Field>
              )}
              <Field label="Total value">
                <Input
                  required
                  type="number"
                  step="any"
                  value={totalValue}
                  onChange={(e) => setTotalValue(e.currentTarget.value)}
                />
              </Field>
              <Field label="Fee (optional)">
                <Input
                  type="number"
                  step="any"
                  value={fee}
                  onChange={(e) => setFee(e.currentTarget.value)}
                />
              </Field>
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label={needsTransferDestination(transactionType) ? "Custody (source)" : "Custody (optional)"}>
                <Select value={custodiaId} onValueChange={setCustodiaId}>
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="No custody set" />
                  </SelectTrigger>
                  <SelectContent>
                    {custodias.map((custodia) => (
                      <SelectItem key={custodia.id} value={String(custodia.id)}>
                        {custodia.instituicao} ({custodia.titular})
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>

              {needsTransferDestination(transactionType) && (
                <Field label="Custody (destination)">
                  <Select value={transferToCustodiaId} onValueChange={setTransferToCustodiaId}>
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="Select the destination" />
                    </SelectTrigger>
                    <SelectContent>
                      {custodias.map((custodia) => (
                        <SelectItem key={custodia.id} value={String(custodia.id)}>
                          {custodia.instituicao} ({custodia.titular})
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
              )}
            </div>

            {showFixedIncomeFields && (
              <div className="grid grid-cols-1 gap-4 rounded-md border p-4 sm:grid-cols-3">
                <Field label="Issuer (optional)">
                  <Input value={fiEmissor} onChange={(e) => setFiEmissor(e.currentTarget.value)} />
                </Field>
                <Field label="Index">
                  <Select value={fiIndexador} onValueChange={setFiIndexador}>
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {FI_INDEXADORES.map((key) => (
                        <SelectItem key={key} value={key}>
                          {key}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
                <Field label="Contracted rate (%)">
                  <Input
                    type="number"
                    step="any"
                    value={fiTaxaPercentual}
                    onChange={(e) => setFiTaxaPercentual(e.currentTarget.value)}
                  />
                </Field>
                <Field label="Maturity date">
                  <Input
                    type="date"
                    value={fiDataVencimento}
                    onChange={(e) => setFiDataVencimento(e.currentTarget.value)}
                  />
                </Field>
                <Field label="Liquidity">
                  <Select value={fiLiquidez} onValueChange={setFiLiquidez}>
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {FI_LIQUIDEZ_OPTIONS.map((key) => (
                        <SelectItem key={key} value={key}>
                          {key}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
              </div>
            )}

            <Field label="Notes (optional)">
              <Input value={notes} onChange={(e) => setNotes(e.currentTarget.value)} />
            </Field>

            {createMutation.isError && (
              <p className="text-red-600">{createMutation.error.message}</p>
            )}

            <Button type="submit" disabled={!canSubmit || createMutation.isPending} className="w-fit">
              {createMutation.isPending ? "Logging..." : "Log transaction"}
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Consolidated positions</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="mb-4 text-sm text-muted-foreground">
            Net quantity and average buy price (simple average of purchases, not reduced by
            sells — real average-cost/FIFO is left for Fase 10.3).
          </p>
          {positionsQuery.isError && (
            <p className="mb-3 text-red-600">{positionsQuery.error.message}</p>
          )}
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Ticker</TableHead>
                <TableHead>Name</TableHead>
                <TableHead>Quantity</TableHead>
                <TableHead>Avg. price (buy)</TableHead>
                <TableHead>By custody</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {positions.length === 0 && (
                <TableRow>
                  <TableCell colSpan={5} className="text-center text-muted-foreground">
                    No positions yet — log a purchase above.
                  </TableCell>
                </TableRow>
              )}
              {positions.map((position) => (
                <TableRow key={position.asset_id}>
                  <TableCell>{position.ticker}</TableCell>
                  <TableCell>{position.name}</TableCell>
                  <TableCell>{position.quantity}</TableCell>
                  <TableCell>
                    {position.average_buy_price !== null
                      ? `${position.currency} ${position.average_buy_price.toFixed(2)}`
                      : "—"}
                  </TableCell>
                  <TableCell>
                    {position.by_custodia
                      .map((c) => `${c.custodia_label ?? "no custody"}: ${c.quantity}`)
                      .join(" · ")}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
}

export default TransactionSection;

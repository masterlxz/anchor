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

function formatQuantity(value: number | null): string {
  return value === null ? "—" : value.toString();
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
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<number | null>(null);

  const queryClient = useQueryClient();

  const assetsQuery = useQuery<Asset[], AppError>({
    queryKey: ["assets"],
    queryFn: () => invoke("list_assets"),
  });
  const custodiasQuery = useQuery<Custodia[], AppError>({
    queryKey: ["custodias", workspaceId],
    queryFn: () => invoke("list_custodias", { workspaceId }),
  });
  const transactionsQuery = useQuery<TransactionView[], AppError>({
    queryKey: ["transactions", portfolioId],
    queryFn: () => invoke("list_transactions", { portfolioId }),
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

  const deleteMutation = useMutation<void, AppError, number>({
    mutationFn: (transactionId) => invoke("delete_transaction", { transactionId }),
    onSuccess: () => {
      invalidateAfterWrite();
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
  const transactions = transactionsQuery.data ?? [];

  return (
    <div className="flex flex-col gap-6">
      <Card>
        <CardHeader>
          <CardTitle>Novo lançamento</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
              <Field label="Tipo">
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

              <Field label="Data">
                <Input
                  required
                  type="date"
                  value={transactionDate}
                  onChange={(e) => setTransactionDate(e.currentTarget.value)}
                />
              </Field>

              {needsAsset(transactionType) && (
                <Field label="Ativo">
                  <Select value={assetId} onValueChange={setAssetId}>
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="Selecione um ativo" />
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
                <Field label="Quantidade">
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
                <Field label="Preço unitário">
                  <Input
                    required
                    type="number"
                    step="any"
                    value={unitPrice}
                    onChange={(e) => setUnitPrice(e.currentTarget.value)}
                  />
                </Field>
              )}
              <Field label="Valor total">
                <Input
                  required
                  type="number"
                  step="any"
                  value={totalValue}
                  onChange={(e) => setTotalValue(e.currentTarget.value)}
                />
              </Field>
              <Field label="Taxa (opcional)">
                <Input
                  type="number"
                  step="any"
                  value={fee}
                  onChange={(e) => setFee(e.currentTarget.value)}
                />
              </Field>
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label={needsTransferDestination(transactionType) ? "Custódia (origem)" : "Custódia (opcional)"}>
                <Select value={custodiaId} onValueChange={setCustodiaId}>
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Sem custódia definida" />
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
                <Field label="Custódia (destino)">
                  <Select value={transferToCustodiaId} onValueChange={setTransferToCustodiaId}>
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="Selecione o destino" />
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
                <Field label="Emissor (opcional)">
                  <Input value={fiEmissor} onChange={(e) => setFiEmissor(e.currentTarget.value)} />
                </Field>
                <Field label="Indexador">
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
                <Field label="Taxa contratada (%)">
                  <Input
                    type="number"
                    step="any"
                    value={fiTaxaPercentual}
                    onChange={(e) => setFiTaxaPercentual(e.currentTarget.value)}
                  />
                </Field>
                <Field label="Data de vencimento">
                  <Input
                    type="date"
                    value={fiDataVencimento}
                    onChange={(e) => setFiDataVencimento(e.currentTarget.value)}
                  />
                </Field>
                <Field label="Liquidez">
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

            <Field label="Notas (opcional)">
              <Input value={notes} onChange={(e) => setNotes(e.currentTarget.value)} />
            </Field>

            {createMutation.isError && (
              <p className="text-red-600">{createMutation.error.message}</p>
            )}

            <Button type="submit" disabled={!canSubmit || createMutation.isPending} className="w-fit">
              {createMutation.isPending ? "Lançando..." : "Lançar"}
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Posições consolidadas</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="mb-4 text-sm text-muted-foreground">
            Quantidade líquida e preço médio de compra (média simples das compras, sem redução por
            venda — custo médio/FIFO de verdade fica pra Fase 10.3).
          </p>
          {positionsQuery.isError && (
            <p className="mb-3 text-red-600">{positionsQuery.error.message}</p>
          )}
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Ticker</TableHead>
                <TableHead>Nome</TableHead>
                <TableHead>Quantidade</TableHead>
                <TableHead>Preço médio (compra)</TableHead>
                <TableHead>Por custódia</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {positions.length === 0 && (
                <TableRow>
                  <TableCell colSpan={5} className="text-center text-muted-foreground">
                    Nenhuma posição ainda — lance uma compra acima.
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
                      .map((c) => `${c.custodia_label ?? "sem custódia"}: ${c.quantity}`)
                      .join(" · ")}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Histórico de lançamentos</CardTitle>
        </CardHeader>
        <CardContent>
          {transactionsQuery.isError && (
            <p className="mb-3 text-red-600">{transactionsQuery.error.message}</p>
          )}
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Data</TableHead>
                <TableHead>Tipo</TableHead>
                <TableHead>Ativo</TableHead>
                <TableHead>Quantidade</TableHead>
                <TableHead>Preço unit.</TableHead>
                <TableHead>Valor total</TableHead>
                <TableHead>Custódia</TableHead>
                <TableHead>Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {transactions.length === 0 && (
                <TableRow>
                  <TableCell colSpan={8} className="text-center text-muted-foreground">
                    Nenhum lançamento ainda.
                  </TableCell>
                </TableRow>
              )}
              {transactions.map((tx) => {
                const isConfirming = confirmingDeleteId === tx.id;
                return (
                  <TableRow key={tx.id}>
                    <TableCell>{tx.transaction_date}</TableCell>
                    <TableCell>
                      {TRANSACTION_TYPE_LABELS[tx.transaction_type as TransactionType] ??
                        tx.transaction_type}
                    </TableCell>
                    <TableCell>{tx.ticker ?? "—"}</TableCell>
                    <TableCell>{formatQuantity(tx.quantity)}</TableCell>
                    <TableCell>{tx.unit_price?.toFixed(2) ?? "—"}</TableCell>
                    <TableCell>{tx.total_value.toFixed(2)}</TableCell>
                    <TableCell>
                      {tx.custodia_label ?? "—"}
                      {tx.transfer_to_custodia_label ? ` → ${tx.transfer_to_custodia_label}` : ""}
                    </TableCell>
                    <TableCell>
                      <Button
                        variant={isConfirming ? "destructive" : "outline"}
                        size="sm"
                        onClick={() => handleDeleteClick(tx.id)}
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
    </div>
  );
}

export default TransactionSection;

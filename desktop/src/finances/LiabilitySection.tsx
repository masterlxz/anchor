import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
import type { Asset } from "../portfolio/types";
import type { BankAccountView } from "./types";
import {
  LIABILITY_AMORTIZATION_SYSTEMS,
  LIABILITY_AMORTIZATION_SYSTEM_LABELS,
  LIABILITY_INDEXADORES,
  type Liability,
  type LiabilityAmortizationSystem,
  type LiabilityView,
} from "./types";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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

type CreateLiabilityRequest = {
  workspace_id: number;
  bank_account_id: number;
  linked_asset_id: number | null;
  nome: string;
  titular: string;
  principal: number;
  taxa_percentual: number;
  indexador: string;
  prazo_meses: number;
  sistema_amortizacao: string;
  data_inicio: string;
};

function todayIso(): string {
  return new Date().toISOString().slice(0, 10);
}

function accountLabel(accounts: BankAccountView[], id: number): string {
  return accounts.find((a) => a.id === id)?.nome ?? "—";
}

// Fase 12 — dívida/passivo com geração automática de parcelas (Price ou
// SAC), Sessão 71. As parcelas em si (`parcela_divida`) aparecem na aba
// Transactions, geradas de uma vez na criação — esta tela só cadastra a
// dívida e mostra o saldo devedor atual (derivado, mesmo espírito do saldo
// de `BankAccountSection`).
function LiabilitySection({ workspaceId }: { workspaceId: number }) {
  const [isAddDialogOpen, setIsAddDialogOpen] = useState(false);
  const [nome, setNome] = useState("");
  const [titular, setTitular] = useState("");
  const [bankAccountId, setBankAccountId] = useState("");
  const [linkedAssetId, setLinkedAssetId] = useState("");
  const [principal, setPrincipal] = useState("");
  const [taxaPercentual, setTaxaPercentual] = useState("");
  const [indexador, setIndexador] = useState<string>(LIABILITY_INDEXADORES[0]);
  const [prazoMeses, setPrazoMeses] = useState("");
  const [sistemaAmortizacao, setSistemaAmortizacao] =
    useState<LiabilityAmortizationSystem>("price");
  const [dataInicio, setDataInicio] = useState(todayIso());
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<number | null>(null);

  const queryClient = useQueryClient();

  const liabilitiesQuery = useQuery<LiabilityView[], AppError>({
    queryKey: ["liabilities", workspaceId],
    queryFn: () => invoke("list_liabilities", { workspaceId }),
  });
  const accountsQuery = useQuery<BankAccountView[], AppError>({
    queryKey: ["bank-accounts", workspaceId],
    queryFn: () => invoke("list_bank_accounts", { workspaceId }),
  });
  const assetsQuery = useQuery<Asset[], AppError>({
    queryKey: ["assets"],
    queryFn: () => invoke("list_assets"),
  });

  const liabilities = liabilitiesQuery.data ?? [];
  const accounts = accountsQuery.data ?? [];
  const assets = assetsQuery.data ?? [];
  const totalDebt = liabilities.reduce((sum, l) => sum + l.saldo_devedor_atual, 0);

  function resetForm() {
    setNome("");
    setTitular("");
    setBankAccountId("");
    setLinkedAssetId("");
    setPrincipal("");
    setTaxaPercentual("");
    setIndexador(LIABILITY_INDEXADORES[0]);
    setPrazoMeses("");
    setSistemaAmortizacao("price");
    setDataInicio(todayIso());
  }

  const createMutation = useMutation<Liability, AppError, CreateLiabilityRequest>({
    mutationFn: (request) => invoke("create_liability", { request }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["liabilities", workspaceId] });
      queryClient.invalidateQueries({ queryKey: ["bank-accounts", workspaceId] });
      queryClient.invalidateQueries({ queryKey: ["general-transactions", workspaceId] });
      resetForm();
      setIsAddDialogOpen(false);
    },
  });

  const deleteMutation = useMutation<void, AppError, number>({
    mutationFn: (liabilityId) => invoke("delete_liability", { liabilityId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["liabilities", workspaceId] });
      queryClient.invalidateQueries({ queryKey: ["bank-accounts", workspaceId] });
      queryClient.invalidateQueries({ queryKey: ["general-transactions", workspaceId] });
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
      workspace_id: workspaceId,
      bank_account_id: Number(bankAccountId),
      linked_asset_id: linkedAssetId !== "" ? Number(linkedAssetId) : null,
      nome,
      titular,
      principal: Number(principal),
      taxa_percentual: Number(taxaPercentual),
      indexador,
      prazo_meses: Number(prazoMeses),
      sistema_amortizacao: sistemaAmortizacao,
      data_inicio: dataInicio,
    });
  }

  const canSubmit =
    nome.trim() !== "" &&
    titular.trim() !== "" &&
    bankAccountId !== "" &&
    principal !== "" &&
    taxaPercentual !== "" &&
    prazoMeses !== "";

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Liabilities</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="mb-4 text-sm text-muted-foreground">
            Debts with automatically generated installments (Price or SAC) — each installment
            shows up as a "Loan installment" transaction, debited from the linked account on its
            due date. Outstanding balance is always derived, never entered directly.
          </p>

          <div className="mb-4 flex items-center justify-between">
            <p className="text-sm">
              <span className="text-muted-foreground">Total debt: </span>
              <span className="font-medium">{totalDebt.toFixed(2)}</span>
            </p>
            <Button type="button" onClick={() => setIsAddDialogOpen(true)}>
              New liability
            </Button>
          </div>

          {liabilitiesQuery.isError && (
            <p className="mb-3 text-red-600">{liabilitiesQuery.error.message}</p>
          )}

          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Holder</TableHead>
                <TableHead>Account</TableHead>
                <TableHead>Principal</TableHead>
                <TableHead>Outstanding</TableHead>
                <TableHead>Term</TableHead>
                <TableHead>System</TableHead>
                <TableHead>Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {liabilities.length === 0 && (
                <TableRow>
                  <TableCell colSpan={8} className="text-center text-muted-foreground">
                    No liability registered yet.
                  </TableCell>
                </TableRow>
              )}
              {liabilities.map((liability) => {
                const isConfirming = confirmingDeleteId === liability.id;
                return (
                  <TableRow key={liability.id}>
                    <TableCell>{liability.nome}</TableCell>
                    <TableCell>{liability.titular}</TableCell>
                    <TableCell>{accountLabel(accounts, liability.bank_account_id)}</TableCell>
                    <TableCell>{liability.principal.toFixed(2)}</TableCell>
                    <TableCell>{liability.saldo_devedor_atual.toFixed(2)}</TableCell>
                    <TableCell>{liability.prazo_meses}mo</TableCell>
                    <TableCell>
                      {LIABILITY_AMORTIZATION_SYSTEM_LABELS[
                        liability.sistema_amortizacao as LiabilityAmortizationSystem
                      ] ?? liability.sistema_amortizacao}
                    </TableCell>
                    <TableCell>
                      <Button
                        variant={isConfirming ? "destructive" : "outline"}
                        size="sm"
                        onClick={() => handleDeleteClick(liability.id)}
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

      <Dialog open={isAddDialogOpen} onOpenChange={setIsAddDialogOpen}>
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>New liability</DialogTitle>
          </DialogHeader>

          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label="Name">
                <Input
                  required
                  placeholder="e.g.: Financiamento Apto"
                  value={nome}
                  onChange={(e) => setNome(e.currentTarget.value)}
                />
              </Field>
              <Field label="Holder">
                <Input
                  required
                  placeholder="e.g.: CPF/CNPJ"
                  value={titular}
                  onChange={(e) => setTitular(e.currentTarget.value)}
                />
              </Field>
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label="Paid from account">
                <Select value={bankAccountId} onValueChange={setBankAccountId}>
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select an account" />
                  </SelectTrigger>
                  <SelectContent>
                    {accounts.map((account) => (
                      <SelectItem key={account.id} value={String(account.id)}>
                        {account.nome} ({account.titular})
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>
              <Field label="Linked asset (optional)">
                <Select value={linkedAssetId} onValueChange={setLinkedAssetId}>
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="None" />
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
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label="Principal">
                <Input
                  required
                  type="number"
                  step="any"
                  value={principal}
                  onChange={(e) => setPrincipal(e.currentTarget.value)}
                />
              </Field>
              <Field label="Monthly rate (%)">
                <Input
                  required
                  type="number"
                  step="any"
                  value={taxaPercentual}
                  onChange={(e) => setTaxaPercentual(e.currentTarget.value)}
                />
              </Field>
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
              <Field label="Index">
                <Select value={indexador} onValueChange={setIndexador}>
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {LIABILITY_INDEXADORES.map((key) => (
                      <SelectItem key={key} value={key}>
                        {key}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>
              <Field label="Term (months)">
                <Input
                  required
                  type="number"
                  step="1"
                  min="1"
                  value={prazoMeses}
                  onChange={(e) => setPrazoMeses(e.currentTarget.value)}
                />
              </Field>
              <Field label="Amortization">
                <Select
                  value={sistemaAmortizacao}
                  onValueChange={(value) =>
                    setSistemaAmortizacao(value as LiabilityAmortizationSystem)
                  }
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {LIABILITY_AMORTIZATION_SYSTEMS.map((key) => (
                      <SelectItem key={key} value={key}>
                        {LIABILITY_AMORTIZATION_SYSTEM_LABELS[key]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Field>
            </div>

            <Field label="Start date">
              <Input
                required
                type="date"
                value={dataInicio}
                onChange={(e) => setDataInicio(e.currentTarget.value)}
              />
            </Field>

            {createMutation.isError && (
              <p className="text-red-600">{createMutation.error.message}</p>
            )}

            <Button type="submit" disabled={!canSubmit || createMutation.isPending} className="w-fit">
              {createMutation.isPending ? "Creating..." : "Create liability"}
            </Button>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default LiabilitySection;

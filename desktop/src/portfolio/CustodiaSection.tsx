import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
import type { Custodia } from "./types";
import Field from "../components/Field";
import { Input } from "@/components/ui/input";
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

type CreateCustodiaRequest = {
  workspace_id: number;
  instituicao: string;
  titular: string;
};

function CustodiaSection({ workspaceId }: { workspaceId: number }) {
  const [instituicao, setInstituicao] = useState("");
  const [titular, setTitular] = useState("");
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<number | null>(null);

  const queryClient = useQueryClient();

  const custodiasQuery = useQuery<Custodia[], AppError>({
    queryKey: ["custodias", workspaceId],
    queryFn: () => invoke("list_custodias", { workspaceId }),
  });

  const createMutation = useMutation<Custodia, AppError, CreateCustodiaRequest>({
    mutationFn: (request) => invoke("create_custodia", { request }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["custodias", workspaceId] });
      setInstituicao("");
      setTitular("");
    },
  });

  const deleteMutation = useMutation<void, AppError, number>({
    mutationFn: (custodiaId) => invoke("delete_custodia", { custodiaId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["custodias", workspaceId] });
      setConfirmingDeleteId(null);
    },
  });

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    createMutation.mutate({ workspace_id: workspaceId, instituicao, titular });
  }

  function handleDeleteClick(id: number) {
    if (confirmingDeleteId === id) {
      deleteMutation.mutate(id);
    } else {
      setConfirmingDeleteId(id);
    }
  }

  const custodias = custodiasQuery.data ?? [];

  return (
    <Card>
      <CardHeader>
        <CardTitle>Custódias</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="mb-4 text-sm text-muted-foreground">
          Onde cada investimento está guardado (corretora, exchange, wallet, banco) — ortogonal à
          classe do ativo. Cadastro livre, sem lista pré-pronta.
        </p>
        <form onSubmit={handleSubmit} className="mb-8 grid grid-cols-1 gap-4 sm:grid-cols-3">
          <Field label="Instituição" className="sm:col-span-1">
            <Input
              required
              placeholder="ex.: Rico, Ledger"
              value={instituicao}
              onChange={(e) => setInstituicao(e.currentTarget.value)}
            />
          </Field>
          <Field label="Titular" className="sm:col-span-1">
            <Input
              required
              placeholder="ex.: CPF/CNPJ"
              value={titular}
              onChange={(e) => setTitular(e.currentTarget.value)}
            />
          </Field>
          <div className="flex items-end">
            <Button type="submit" disabled={createMutation.isPending} className="w-fit">
              {createMutation.isPending ? "Adicionando..." : "Adicionar custódia"}
            </Button>
          </div>
        </form>

        {createMutation.isError && (
          <p className="mb-3 text-red-600">{createMutation.error.message}</p>
        )}
        {custodiasQuery.isError && (
          <p className="mb-3 text-red-600">{custodiasQuery.error.message}</p>
        )}

        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Instituição</TableHead>
              <TableHead>Titular</TableHead>
              <TableHead>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {custodias.length === 0 && (
              <TableRow>
                <TableCell colSpan={3} className="text-center text-muted-foreground">
                  Nenhuma custódia cadastrada ainda.
                </TableCell>
              </TableRow>
            )}
            {custodias.map((custodia) => {
              const isConfirming = confirmingDeleteId === custodia.id;
              return (
                <TableRow key={custodia.id}>
                  <TableCell>{custodia.instituicao}</TableCell>
                  <TableCell>{custodia.titular}</TableCell>
                  <TableCell>
                    <Button
                      variant={isConfirming ? "destructive" : "outline"}
                      size="sm"
                      onClick={() => handleDeleteClick(custodia.id)}
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

export default CustodiaSection;

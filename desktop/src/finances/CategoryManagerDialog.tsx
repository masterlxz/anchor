import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
import type { GeneralTransactionCategory } from "./types";
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
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

type CreateCategoryRequest = {
  workspace_id: number;
  nome: string;
};

// Fase 12 — cadastro de categorias, Sessão 71 (feedback do dono do projeto
// depois do teste ao vivo do núcleo: escolher numa lista em vez de digitar
// texto livre a cada lançamento). Popup em vez de aba própria — pedido
// explícito do dono do projeto, gerenciar a partir da tela de Transactions
// mesmo. Sem limite de orçamento mensal ainda, sem edição (mesmo estágio
// que `Custodia` tinha antes de ganhar update).
function CategoryManagerDialog({
  workspaceId,
  open,
  onOpenChange,
}: {
  workspaceId: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [nome, setNome] = useState("");
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<number | null>(null);

  const queryClient = useQueryClient();

  const categoriesQuery = useQuery<GeneralTransactionCategory[], AppError>({
    queryKey: ["general-transaction-categories", workspaceId],
    queryFn: () => invoke("list_general_transaction_categories", { workspaceId }),
  });

  function invalidate() {
    queryClient.invalidateQueries({
      queryKey: ["general-transaction-categories", workspaceId],
    });
  }

  const createMutation = useMutation<GeneralTransactionCategory, AppError, CreateCategoryRequest>({
    mutationFn: (request) => invoke("create_general_transaction_category", { request }),
    onSuccess: () => {
      invalidate();
      setNome("");
    },
  });

  const deleteMutation = useMutation<void, AppError, number>({
    mutationFn: (generalTransactionCategoryId) =>
      invoke("delete_general_transaction_category", { generalTransactionCategoryId }),
    onSuccess: () => {
      invalidate();
      setConfirmingDeleteId(null);
    },
  });

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    createMutation.mutate({ workspace_id: workspaceId, nome });
  }

  function handleDeleteClick(id: number) {
    if (confirmingDeleteId === id) {
      deleteMutation.mutate(id);
    } else {
      setConfirmingDeleteId(id);
    }
  }

  const categories = categoriesQuery.data ?? [];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Manage categories</DialogTitle>
        </DialogHeader>

        <p className="text-sm text-muted-foreground">
          Cadastradas uma vez, escolhidas na hora de lançar receita/despesa — sem limite de
          orçamento mensal por ora.
        </p>

        <form onSubmit={handleSubmit} className="flex items-end gap-4">
          <Field label="Name" className="flex-1">
            <Input
              required
              placeholder="e.g.: Rent, Salary"
              value={nome}
              onChange={(e) => setNome(e.currentTarget.value)}
            />
          </Field>
          <Button type="submit" disabled={createMutation.isPending} className="w-fit">
            {createMutation.isPending ? "Adding..." : "Add"}
          </Button>
        </form>

        {createMutation.isError && (
          <p className="text-red-600">{createMutation.error.message}</p>
        )}
        {categoriesQuery.isError && (
          <p className="text-red-600">{categoriesQuery.error.message}</p>
        )}

        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {categories.length === 0 && (
              <TableRow>
                <TableCell colSpan={2} className="text-center text-muted-foreground">
                  No category registered yet.
                </TableCell>
              </TableRow>
            )}
            {categories.map((category) => {
              const isConfirming = confirmingDeleteId === category.id;
              return (
                <TableRow key={category.id}>
                  <TableCell>{category.nome}</TableCell>
                  <TableCell>
                    <Button
                      variant={isConfirming ? "destructive" : "outline"}
                      size="sm"
                      onClick={() => handleDeleteClick(category.id)}
                    >
                      {isConfirming ? "Confirm?" : "Delete"}
                    </Button>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </DialogContent>
    </Dialog>
  );
}

export default CategoryManagerDialog;

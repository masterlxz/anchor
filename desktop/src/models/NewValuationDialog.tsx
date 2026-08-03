import { useState } from "react";
import { MODELS, type ModelKey } from "./registry";
import Field from "../components/Field";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

/// Fase 3/10, item 8, Sessão 56 — a antiga aba própria "Valuation" (seletor
/// de modelo + form, `App.tsx`) migrou pra dentro de Research/análise de
/// ação, virando este popup — pedido explícito do dono do projeto: os
/// modelos servem pra "empresas" (hoje só Ação BR tem análise em Research,
/// mas Stocks internacionais também vão servir quando ganharem a própria,
/// Fase 10 item 8). `ticker` vem pré-preenchido de quem abriu o popup (a
/// tela de análise já sabe qual ticker está sendo olhado) — cada form
/// ainda deixa o campo editável, só muda o valor inicial.
function NewValuationDialog({
  open,
  onOpenChange,
  ticker,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  ticker: string;
}) {
  const [selectedModel, setSelectedModel] = useState<ModelKey>("bazin");
  const SelectedForm = MODELS[selectedModel].component;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>New Valuation — {ticker}</DialogTitle>
        </DialogHeader>

        <div className="mb-4 max-w-xs">
          <Field label="Valuation model">
            <Select
              value={selectedModel}
              onValueChange={(value) => setSelectedModel(value as ModelKey)}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {Object.entries(MODELS).map(([key, { label }]) => (
                  <SelectItem key={key} value={key}>
                    {label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
        </div>

        <SelectedForm key={selectedModel} ticker={ticker} />
      </DialogContent>
    </Dialog>
  );
}

export default NewValuationDialog;

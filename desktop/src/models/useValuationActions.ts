import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { AppError, ValuationModel } from "../types";
import type { ModelKey } from "./registry";

export type ValuationOutcome = {
  fair_price: number;
  safety_margin: number;
  verdict: string;
};

export type SavedValuationResponse<TInputs> = {
  valuation: ValuationModel;
  inputs: TInputs;
};

/// Sessão 56 — as 8 telas de modelo faziam a mesma coisa (uma `useMutation`
/// só, calcula-e-salva junto) antes de virar "Calculate" (preview, sem
/// gravar) + "Save" (persiste) como dois botões separados, pedido explícito
/// do dono do projeto. Diferente da duplicação de UI entre as seções de
/// Research (onde o projeto não abstrai por decisão consciente — conteúdo
/// diferente por classe), aqui o wiring é mecanicamente idêntico nos 8
/// modelos, só o nome do comando muda — hook único parametrizado por
/// `model`, reaproveitado pelos 8 forms em vez de repetir a mesma
/// `useMutation` 8 vezes.
export function useValuationActions<TRequest, TInputs>(model: ModelKey) {
  const queryClient = useQueryClient();

  const calculateMutation = useMutation<ValuationOutcome, AppError, TRequest>({
    mutationFn: (request) => invoke(`calculate_${model}`, { request }),
  });

  const saveMutation = useMutation<SavedValuationResponse<TInputs>, AppError, TRequest>({
    mutationFn: (request) => invoke(`save_${model}`, { request }),
    onSuccess: () => {
      // Chave global compartilhada com `list_valuations` (StockAnalysisSection
      // "Saved valuation", SavedValuationsPanel) — invalidar aqui, uma vez,
      // já propaga pra qualquer tela aberta que dependa dela, mesmo que o
      // form esteja rodando dentro de um popup separado.
      queryClient.invalidateQueries({ queryKey: ["valuations"] });
    },
  });

  return { calculateMutation, saveMutation };
}

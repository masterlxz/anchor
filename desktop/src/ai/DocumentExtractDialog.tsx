import { useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useMutation, useQuery } from "@tanstack/react-query";
import type { AppError } from "../types";
import type { ApiKeySummary } from "../settings/SettingsPage";
import { PROVIDER_LABELS } from "../settings/SettingsPage";
import { DEFAULT_MODEL_BY_PROVIDER } from "./modelDefaults";
import ApiKeyGate from "../chat/ApiKeyGate";
import Field from "../components/Field";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
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
import { FileText } from "lucide-react";

// Bloco genérico reutilizável: "escolha um PDF, a IA extrai um dado
// estruturado dele, você confere e confirma". Não sabe nada sobre o
// consumidor específico (RNAV, ou futuramente teses de investimento —
// Fase 10.5) — só recebe instructions/jsonSchema/parseResult/renderPreview/
// onAccept de fora. O núcleo genérico de verdade é o comando Tauri
// `extract_document_data`; este componente só empresta a mesma UI de
// escolha de chave/modelo já usada no chat (Fase 7.9.5).
type DocumentExtractDialogProps<T> = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  instructions: string;
  jsonSchema: object;
  parseResult: (raw: unknown) => T;
  renderPreview: (result: T) => ReactNode;
  onAccept: (result: T) => void;
};

function DocumentExtractDialog<T>({
  open: isOpen,
  onOpenChange,
  title,
  instructions,
  jsonSchema,
  parseResult,
  renderPreview,
  onAccept,
}: DocumentExtractDialogProps<T>) {
  const [filePath, setFilePath] = useState<string | null>(null);
  const [selectedKeyId, setSelectedKeyId] = useState<number | null>(null);
  const [model, setModel] = useState("");
  const [result, setResult] = useState<T | null>(null);
  const [parseError, setParseError] = useState<string | null>(null);

  const keysQuery = useQuery<ApiKeySummary[], AppError>({
    queryKey: ["api-keys"],
    queryFn: () => invoke("list_api_keys"),
    enabled: isOpen,
  });
  const keys = keysQuery.data ?? [];

  const extractMutation = useMutation<unknown, AppError, void>({
    mutationFn: () =>
      invoke("extract_document_data", {
        keyId: selectedKeyId,
        model,
        filePath,
        instructions,
        jsonSchema,
      }),
    onSuccess: (raw) => {
      try {
        setResult(parseResult(raw));
        setParseError(null);
      } catch (err) {
        setParseError(
          err instanceof Error ? err.message : "Resposta inesperada da IA.",
        );
      }
    },
  });

  function handleKeyChange(nextKeyId: string) {
    const id = Number(nextKeyId);
    setSelectedKeyId(id);
    const key = keys.find((k) => k.id === id);
    if (key) setModel(DEFAULT_MODEL_BY_PROVIDER[key.provider] ?? "");
  }

  async function handleChooseFile() {
    const picked = await open({
      multiple: false,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (typeof picked === "string") {
      setFilePath(picked);
      setResult(null);
      setParseError(null);
    }
  }

  function handleClose(nextOpen: boolean) {
    if (!nextOpen) {
      setFilePath(null);
      setResult(null);
      setParseError(null);
      extractMutation.reset();
    }
    onOpenChange(nextOpen);
  }

  function handleAccept() {
    if (result === null) return;
    onAccept(result);
    handleClose(false);
  }

  const hasKey = keys.length > 0;

  return (
    <Dialog open={isOpen} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>

        {keysQuery.isPending ? null : !hasKey ? (
          <ApiKeyGate />
        ) : (
          <div className="flex flex-col gap-3">
            <Button type="button" variant="outline" onClick={handleChooseFile}>
              <FileText />
              {filePath ? filePath.split("/").pop() : "Escolher PDF..."}
            </Button>

            <Field label="Chave de API">
              <div className="flex gap-2">
                <Select
                  value={selectedKeyId !== null ? String(selectedKeyId) : undefined}
                  onValueChange={handleKeyChange}
                >
                  <SelectTrigger className="w-36 shrink-0">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {keys.map((k) => (
                      <SelectItem key={k.id} value={String(k.id)}>
                        {k.name} ({PROVIDER_LABELS[k.provider] ?? k.provider})
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Input
                  value={model}
                  onChange={(e) => setModel(e.currentTarget.value)}
                  placeholder="modelo"
                  className="text-xs"
                />
              </div>
            </Field>

            {(extractMutation.isError || parseError) && (
              <p className="text-red-600">
                {parseError ?? extractMutation.error?.message}
              </p>
            )}

            {result !== null && (
              <div className="rounded-md border p-3 text-sm">
                {renderPreview(result)}
              </div>
            )}

            {result === null ? (
              <Button
                type="button"
                onClick={() => extractMutation.mutate()}
                disabled={!filePath || selectedKeyId === null || extractMutation.isPending}
              >
                {extractMutation.isPending ? "Extraindo..." : "Extrair"}
              </Button>
            ) : (
              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => setResult(null)}>
                  Descartar
                </Button>
                <Button type="button" onClick={handleAccept}>
                  Usar este valor
                </Button>
              </DialogFooter>
            )}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}

export default DocumentExtractDialog;

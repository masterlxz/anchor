import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { AppError } from "../types";
import type { ApiKeySummary } from "../settings/SettingsPage";
import { PROVIDER_LABELS } from "../settings/SettingsPage";
import { DEFAULT_MODEL_BY_PROVIDER } from "../ai/modelDefaults";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

// Estilos mínimos pro Markdown curto que a IA devolve (### subtítulos, listas,
// negrito) — não usa o mapa de componentes do assistant-ui porque aquele é
// acoplado ao `MarkdownTextPrimitive` do chat, não ao `ReactMarkdown` genérico.
const aboutMarkdownComponents = {
  h3: ({ className, ...props }: React.ComponentPropsWithoutRef<"h3">) => (
    <h3
      className={`mt-5 mb-1.5 text-sm font-semibold first:mt-0 ${className ?? ""}`}
      {...props}
    />
  ),
  p: ({ className, ...props }: React.ComponentPropsWithoutRef<"p">) => (
    <p className={`mb-1.5 text-sm leading-relaxed last:mb-0 ${className ?? ""}`} {...props} />
  ),
  ul: ({ className, ...props }: React.ComponentPropsWithoutRef<"ul">) => (
    <ul
      className={`my-1.5 ms-5 list-disc text-sm leading-relaxed last:mb-0 [&>li]:mt-1.5 ${className ?? ""}`}
      {...props}
    />
  ),
  li: ({ className, ...props }: React.ComponentPropsWithoutRef<"li">) => (
    <li className={`leading-relaxed ${className ?? ""}`} {...props} />
  ),
  strong: ({ className, ...props }: React.ComponentPropsWithoutRef<"strong">) => (
    <strong className={`font-semibold ${className ?? ""}`} {...props} />
  ),
};

type CompanyAiInfo = {
  id: number;
  ticker: string;
  asset_class: string;
  content: string;
  provider: string;
  model: string;
  generated_at: string;
  regenerate_reason: string | null;
};

/// Fase 10, item 8, Sessão 60 — "Sobre o ativo": resumo qualitativo gerado
/// por IA, cacheado por `(ticker, asset_class)`. Pedido explícito do dono do
/// projeto: preencher isso na mão não escala (7 classes de ativo, texto fica
/// desatualizado), então gera automaticamente na primeira visita e some por
/// completo se o usuário não tiver nenhuma API de IA cadastrada em
/// Configurações — "sem BO", sem estado de erro visível nesse caso. Usa o
/// mesmo padrão de auto-seleção de key/modelo do `ChatPanel.tsx` (primeira
/// key cadastrada + modelo barato default do provider) — sem seletor de
/// provider aqui, mantém a seção simples.
function AboutCompanySection({
  ticker,
  assetClass,
}: {
  ticker: string;
  assetClass: string;
}) {
  const queryClient = useQueryClient();
  const [reasonDialogOpen, setReasonDialogOpen] = useState(false);
  const [reason, setReason] = useState("");

  const keysQuery = useQuery<ApiKeySummary[], AppError>({
    queryKey: ["api-keys"],
    queryFn: () => invoke("list_api_keys"),
  });
  const keys = keysQuery.data ?? [];

  const infoQueryKey = ["company-ai-info", assetClass, ticker];
  const infoQuery = useQuery<CompanyAiInfo | null, AppError>({
    queryKey: infoQueryKey,
    queryFn: () => invoke("get_company_ai_info", { ticker, assetClass }),
    enabled: keys.length > 0,
  });

  const generateMutation = useMutation<CompanyAiInfo, AppError, string | undefined>({
    mutationFn: (mutationReason) => {
      const first = keys[0];
      return invoke("generate_company_ai_info", {
        request: {
          key_id: first.id,
          model: DEFAULT_MODEL_BY_PROVIDER[first.provider] ?? "",
          ticker,
          asset_class: assetClass,
          reason: mutationReason,
        },
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: infoQueryKey });
      setReasonDialogOpen(false);
      setReason("");
    },
  });

  // Auto-gera silenciosamente na primeira visita (nada salvo ainda, ao menos
  // uma key configurada) — sem popup de motivo aqui, não tem o que corrigir
  // ainda. `generateMutation.isIdle` evita disparar de novo em cada
  // re-render enquanto a geração já está em andamento ou já falhou.
  useEffect(() => {
    if (
      keys.length > 0 &&
      infoQuery.isSuccess &&
      infoQuery.data == null &&
      generateMutation.isIdle
    ) {
      generateMutation.mutate(undefined);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [keys.length, infoQuery.isSuccess, infoQuery.data]);

  if (keys.length === 0) return null;

  const info = infoQuery.data;
  const isGenerating = generateMutation.isPending;

  return (
    <div className="flex flex-col gap-2">
      <h3 className="text-lg font-semibold">About the asset</h3>
      <div className="rounded-lg border border-border bg-card p-4">
        {isGenerating ? (
          <p className="text-sm text-muted-foreground">Gerando com IA…</p>
        ) : info ? (
          <>
            <ReactMarkdown remarkPlugins={[remarkGfm]} components={aboutMarkdownComponents}>
              {info.content}
            </ReactMarkdown>
            <p className="mt-3 text-xs text-muted-foreground">
              Gerado via {PROVIDER_LABELS[info.provider] ?? info.provider} em{" "}
              {new Date(info.generated_at).toLocaleString()}
            </p>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="mt-2"
              onClick={() => setReasonDialogOpen(true)}
            >
              Regenerate
            </Button>
          </>
        ) : (
          <p className="text-sm text-muted-foreground">
            {generateMutation.isError ? generateMutation.error.message : "—"}
          </p>
        )}
      </div>

      <Dialog open={reasonDialogOpen} onOpenChange={setReasonDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Regenerate — {ticker}</DialogTitle>
          </DialogHeader>
          <Textarea
            value={reason}
            onChange={(e) => setReason(e.currentTarget.value)}
            placeholder="Why do you want to regenerate it? (optional, helps the AI correct it)"
            rows={3}
          />
          {generateMutation.isError && (
            <p className="text-sm text-red-600">{generateMutation.error.message}</p>
          )}
          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => setReasonDialogOpen(false)}
              disabled={isGenerating}
            >
              Cancel
            </Button>
            <Button
              type="button"
              onClick={() => generateMutation.mutate(reason.trim() || undefined)}
              disabled={isGenerating}
            >
              {isGenerating ? "Regenerating..." : "Regenerate"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default AboutCompanySection;

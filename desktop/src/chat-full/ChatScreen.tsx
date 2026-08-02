import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/react-query";
import { AssistantRuntimeProvider } from "@assistant-ui/react";
import type { AppError } from "../types";
import ApiKeyGate from "../chat/ApiKeyGate";
import { PROVIDER_LABELS } from "../settings/SettingsPage";
import ConversationSidebar, { type ConversationSummary } from "./ConversationSidebar";
import { useConversationRuntime } from "./useConversationRuntime";
import { Thread } from "@/components/assistant-ui/thread";
import ValuationProposalCard from "./ValuationProposalCard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

// Fase 7.10.1/7.10.2/7.10.5 — alicerce do chat em tela cheia: conversas
// salvas/múltiplas/nomeadas + troca de chave/modelo, reaproveitando
// `assistant-ui` (useExternalStoreRuntime) só pra lista de mensagens/composer.
// O widget flutuante (`chat/ChatPanel.tsx`) continua existindo à parte.
function ChatScreen({ onBack }: { onBack: () => void }) {
  const [selectedConversationId, setSelectedConversationId] = useState<number | null>(null);

  const conversationsQuery = useQuery<ConversationSummary[], AppError>({
    queryKey: ["conversations"],
    queryFn: () => invoke("list_conversations"),
  });

  // Auto-seleciona a conversa mais recente na primeira carga, mesmo espírito
  // do auto-select de chave em `ChatPanel.tsx`.
  useEffect(() => {
    if (selectedConversationId !== null) return;
    const first = conversationsQuery.data?.[0];
    if (first) setSelectedConversationId(first.id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [conversationsQuery.data]);

  const {
    runtime,
    keys,
    selectedKeyId,
    model,
    setModel,
    handleKeyChange,
    hasKey,
    error,
    totalTokens,
  } = useConversationRuntime(selectedConversationId);

  return (
    <Card className="flex h-[85vh] flex-col">
      <CardHeader>
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <Button variant="outline" size="sm" onClick={onBack}>
              ← Back
            </Button>
            <CardTitle>AI Chat</CardTitle>
          </div>
          {hasKey && (
            <div className="flex items-center gap-2">
              {totalTokens > 0 && (
                <span className="text-muted-foreground text-xs whitespace-nowrap">
                  {totalTokens.toLocaleString("pt-BR")} tokens
                </span>
              )}
              <Select
                value={selectedKeyId !== null ? String(selectedKeyId) : undefined}
                onValueChange={handleKeyChange}
              >
                <SelectTrigger className="w-40">
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
                placeholder="model"
                className="w-36 text-xs"
              />
            </div>
          )}
        </div>
      </CardHeader>
      <CardContent className="flex flex-1 gap-4 overflow-hidden">
        <ConversationSidebar
          selectedId={selectedConversationId}
          onSelect={setSelectedConversationId}
        />
        <div className="flex flex-1 flex-col overflow-hidden">
          {!hasKey ? (
            <ApiKeyGate />
          ) : selectedConversationId === null ? (
            <p className="text-muted-foreground">
              Create or select a conversation to get started.
            </p>
          ) : (
            <>
              {error && <p className="text-red-600">{error}</p>}
              <AssistantRuntimeProvider runtime={runtime}>
                <Thread components={{ ToolFallback: ValuationProposalCard }} />
              </AssistantRuntimeProvider>
            </>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

export default ChatScreen;

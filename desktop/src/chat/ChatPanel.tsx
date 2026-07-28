import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery } from "@tanstack/react-query";
import type { AppError } from "../types";
import type { GeminiContent } from "./types";
import type { ApiKeySummary } from "../settings/SettingsPage";
import { PROVIDER_LABELS } from "../settings/SettingsPage";
import ApiKeyGate from "./ApiKeyGate";
import { DEFAULT_MODEL_BY_PROVIDER } from "../ai/modelDefaults";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { SendIcon, XIcon } from "lucide-react";

function MessageBubble({ message }: { message: GeminiContent }) {
  const isUser = message.role === "user";
  return (
    <div className={`flex ${isUser ? "justify-end" : "justify-start"}`}>
      <div
        className={`max-w-[85%] rounded-lg px-3 py-2 text-sm whitespace-pre-wrap ${
          isUser
            ? "bg-primary text-primary-foreground"
            : "bg-muted text-foreground"
        }`}
      >
        {message.parts.map((part) => part.text).join("")}
      </div>
    </div>
  );
}

function ChatPanel({
  open,
  onOpenChange,
  history,
  onHistoryChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  history: GeminiContent[];
  onHistoryChange: (history: GeminiContent[]) => void;
}) {
  const [input, setInput] = useState("");
  const [selectedKeyId, setSelectedKeyId] = useState<number | null>(null);
  const [model, setModel] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  const keysQuery = useQuery<ApiKeySummary[], AppError>({
    queryKey: ["api-keys"],
    queryFn: () => invoke("list_api_keys"),
    enabled: open,
  });

  const keys = keysQuery.data ?? [];

  // Auto-select the first available key once the list loads (or after the
  // selected one gets deleted in Settings while the panel is open) — the
  // model field follows along, defaulted from that key's provider.
  useEffect(() => {
    if (keys.length === 0) {
      setSelectedKeyId(null);
      return;
    }
    if (!keys.some((k) => k.id === selectedKeyId)) {
      const first = keys[0];
      setSelectedKeyId(first.id);
      setModel(DEFAULT_MODEL_BY_PROVIDER[first.provider] ?? "");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [keys]);

  const sendMutation = useMutation<string, AppError, GeminiContent[]>({
    mutationFn: (nextHistory) =>
      invoke("ask_ai", { keyId: selectedKeyId, model, history: nextHistory }),
  });

  function handleKeyChange(nextKeyId: string) {
    const id = Number(nextKeyId);
    setSelectedKeyId(id);
    const key = keys.find((k) => k.id === id);
    if (key) setModel(DEFAULT_MODEL_BY_PROVIDER[key.provider] ?? "");
  }

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [history, sendMutation.isPending]);

  // The floating panel has no modal overlay to click outside of anymore, so
  // Escape-to-close (free before via Radix Dialog) needs a manual listener.
  useEffect(() => {
    if (!open) return;
    function handleEscape(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") onOpenChange(false);
    }
    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [open, onOpenChange]);

  function handleSend() {
    const text = input.trim();
    if (!text || selectedKeyId === null || sendMutation.isPending) return;

    const nextHistory: GeminiContent[] = [
      ...history,
      { role: "user", parts: [{ text }] },
    ];
    onHistoryChange(nextHistory);
    setInput("");

    sendMutation.mutate(nextHistory, {
      onSuccess: (reply) => {
        onHistoryChange([...nextHistory, { role: "model", parts: [{ text: reply }] }]);
      },
    });
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      handleSend();
    }
  }

  const hasKey = keys.length > 0;

  if (!open) return null;

  return (
    <div
      role="dialog"
      aria-label="Chat de IA"
      className="fixed right-6 bottom-24 z-50 flex h-[42rem] max-h-[80vh] w-[22.5rem] max-w-[calc(100vw-3rem)] flex-col overflow-hidden rounded-xl border bg-popover text-popover-foreground shadow-2xl duration-200 animate-in fade-in-0 slide-in-from-bottom-4"
    >
      <div className="flex items-center justify-between gap-2 border-b p-4">
        <p className="font-heading text-base font-medium text-foreground">
          Chat de IA
        </p>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          onClick={() => onOpenChange(false)}
        >
          <XIcon />
          <span className="sr-only">Fechar chat</span>
        </Button>
      </div>

      {hasKey && (
        <div className="flex gap-2 border-b p-3">
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
      )}

      {keysQuery.isPending ? null : !hasKey ? (
        <ApiKeyGate />
      ) : (
        <>
          <div
            ref={scrollRef}
            className="flex flex-1 flex-col gap-3 overflow-y-auto px-4 py-3"
          >
            {history.length === 0 && (
              <p className="text-sm text-muted-foreground">
                Pergunte sobre suas valuations salvas ou alertas.
              </p>
            )}
            {history.map((message, index) => (
              <MessageBubble key={index} message={message} />
            ))}
            {sendMutation.isPending && (
              <div className="flex justify-start">
                <div className="max-w-[85%] rounded-lg bg-muted px-3 py-2 text-sm text-muted-foreground">
                  Pensando...
                </div>
              </div>
            )}
            {sendMutation.isError && (
              <p className="text-red-600">{sendMutation.error.message}</p>
            )}
          </div>

          <div className="flex gap-2 border-t p-4">
            <Input
              value={input}
              onChange={(e) => setInput(e.currentTarget.value)}
              onKeyDown={handleKeyDown}
              disabled={sendMutation.isPending}
              placeholder="Digite sua pergunta..."
            />
            <Button
              type="button"
              size="icon"
              onClick={handleSend}
              disabled={!input.trim() || sendMutation.isPending}
            >
              <SendIcon />
            </Button>
          </div>
        </>
      )}
    </div>
  );
}

export default ChatPanel;

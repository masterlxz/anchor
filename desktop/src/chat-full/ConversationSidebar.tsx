import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export type ConversationSummary = {
  id: number;
  title: string;
  key_id: number | null;
  model: string;
  updated_at: string;
};

// Mesmo padrão de lista+rename inline+delete com confirmação dupla já usado
// em `SettingsPage.tsx` (seção IA) e `SavedValuationsPanel.tsx` — não é o
// `RemoteThreadListAdapter` do assistant-ui (ver decisão no plano da 7.10).
function ConversationSidebar({
  selectedId,
  onSelect,
}: {
  selectedId: number | null;
  onSelect: (id: number) => void;
}) {
  const queryClient = useQueryClient();
  const [renamingId, setRenamingId] = useState<number | null>(null);
  const [renameDraft, setRenameDraft] = useState("");
  const [confirmingDeleteId, setConfirmingDeleteId] = useState<number | null>(null);

  const conversationsQuery = useQuery<ConversationSummary[], AppError>({
    queryKey: ["conversations"],
    queryFn: () => invoke("list_conversations"),
  });

  const createMutation = useMutation<ConversationSummary, AppError, void>({
    mutationFn: () => invoke("create_conversation", { title: null }),
    onSuccess: (conversation) => {
      queryClient.invalidateQueries({ queryKey: ["conversations"] });
      onSelect(conversation.id);
    },
  });

  const renameMutation = useMutation<void, AppError, { id: number; title: string }>({
    mutationFn: ({ id, title }) => invoke("rename_conversation", { id, title }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["conversations"] });
      setRenamingId(null);
    },
  });

  const deleteMutation = useMutation<void, AppError, number>({
    mutationFn: (id) => invoke("delete_conversation", { id }),
    onSuccess: (_data, id) => {
      queryClient.invalidateQueries({ queryKey: ["conversations"] });
      setConfirmingDeleteId(null);
      if (selectedId === id) {
        const remaining = conversations.filter((c) => c.id !== id);
        if (remaining[0]) onSelect(remaining[0].id);
      }
    },
  });

  function handleDeleteClick(id: number) {
    if (confirmingDeleteId === id) {
      deleteMutation.mutate(id);
    } else {
      setConfirmingDeleteId(id);
    }
  }

  const conversations = conversationsQuery.data ?? [];

  return (
    <div className="flex w-64 shrink-0 flex-col gap-2 border-r pr-4">
      <Button
        type="button"
        variant="outline"
        className="w-full"
        onClick={() => createMutation.mutate()}
        disabled={createMutation.isPending}
      >
        + New conversation
      </Button>

      {conversationsQuery.isError && (
        <p className="text-red-600">{conversationsQuery.error.message}</p>
      )}
      {conversations.length === 0 && !conversationsQuery.isPending && (
        <p className="text-sm text-muted-foreground">No conversations yet.</p>
      )}

      <div className="flex flex-col gap-1 overflow-y-auto">
        {conversations.map((c) => (
          <div
            key={c.id}
            className={`flex items-center gap-1 rounded-md px-2 py-1.5 ${
              selectedId === c.id ? "bg-muted" : "hover:bg-muted/50"
            }`}
          >
            {renamingId === c.id ? (
              <Input
                autoFocus
                value={renameDraft}
                onChange={(e) => setRenameDraft(e.currentTarget.value)}
                onBlur={() =>
                  renameMutation.mutate({ id: c.id, title: renameDraft.trim() || c.title })
                }
                onKeyDown={(e) => {
                  if (e.key === "Enter") e.currentTarget.blur();
                }}
                className="h-7 flex-1 text-sm"
              />
            ) : (
              <button
                type="button"
                className="flex-1 truncate text-left text-sm"
                onClick={() => onSelect(c.id)}
                onDoubleClick={() => {
                  setRenamingId(c.id);
                  setRenameDraft(c.title);
                }}
              >
                {c.title}
              </button>
            )}
            <Button
              type="button"
              variant={confirmingDeleteId === c.id ? "destructive" : "ghost"}
              size="sm"
              className="h-7 shrink-0 px-2 text-xs"
              onClick={() => handleDeleteClick(c.id)}
            >
              {confirmingDeleteId === c.id ? "Confirm?" : "×"}
            </Button>
          </div>
        ))}
      </div>
    </div>
  );
}

export default ConversationSidebar;

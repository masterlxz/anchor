import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
import { PROVIDER_LABELS } from "../settings/SettingsPage";
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

const KEY_PLACEHOLDERS: Record<string, string> = {
  gemini: "AIza...",
  claude: "sk-ant-...",
  openai: "sk-...",
};

// Fase 7.9.2: this is now just a quick way to create the *first* named key
// (called "Default") without a trip to Settings — provider choice moved
// here since the chat no longer has a fixed provider, only a chosen key.
function ApiKeyGate() {
  const [provider, setProvider] = useState("gemini");
  const [key, setKey] = useState("");
  const queryClient = useQueryClient();

  const createMutation = useMutation<number, AppError, void>({
    mutationFn: () =>
      invoke("create_api_key", { provider, name: "Default", key: key.trim() }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["api-keys"] });
      setKey("");
    },
  });

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!key.trim() || createMutation.isPending) return;
    createMutation.mutate();
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4 p-4">
      <p className="text-sm text-muted-foreground">
        Paste an API key to enable the chat. It's stored only in your
        system's keyring, never in the database or in git — you can add
        more keys and name them later in Settings.
      </p>

      <Field label="Provider">
        <Select value={provider} onValueChange={setProvider}>
          <SelectTrigger className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {Object.entries(PROVIDER_LABELS).map(([id, label]) => (
              <SelectItem key={id} value={id}>
                {label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>

      <Field label={`${PROVIDER_LABELS[provider] ?? provider} API key`}>
        <Input
          type="password"
          value={key}
          onChange={(e) => setKey(e.currentTarget.value)}
          placeholder={KEY_PLACEHOLDERS[provider] ?? ""}
        />
      </Field>

      {createMutation.isError && (
        <p className="text-red-600">{createMutation.error.message}</p>
      )}

      <Button
        type="submit"
        disabled={!key.trim() || createMutation.isPending}
        className="w-fit"
      >
        {createMutation.isPending ? "Saving..." : "Save key"}
      </Button>
    </form>
  );
}

export default ApiKeyGate;

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
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

type FinanceApiSettingsView = {
  mode: "local" | "remote";
  remote_url: string | null;
  has_remote_key: boolean;
};

// Fase 14.3 — só armazenamento + leitura da escolha Local/Remote (schema+UI, decisão explícita
// de escopo). Nada aqui é consultado pelo sidecar ainda — "Local" continua sendo o único modo
// com efeito real no app hoje; "Remote" fica salvo pra uma fatia futura ligar de verdade.
function FinanceApiSettingsSection() {
  const queryClient = useQueryClient();

  const settingsQuery = useQuery<FinanceApiSettingsView, AppError>({
    queryKey: ["finance-api-settings"],
    queryFn: () => invoke("get_finance_api_settings"),
  });

  const [mode, setMode] = useState<"local" | "remote">("local");
  const [remoteUrl, setRemoteUrl] = useState("");
  const [remoteKey, setRemoteKey] = useState("");

  useEffect(() => {
    if (!settingsQuery.data) return;
    setMode(settingsQuery.data.mode);
    setRemoteUrl(settingsQuery.data.remote_url ?? "");
  }, [settingsQuery.data]);

  const saveMutation = useMutation<void, AppError, void>({
    mutationFn: () =>
      invoke("set_finance_api_settings", {
        mode,
        remoteUrl: mode === "remote" ? remoteUrl.trim() : null,
        remoteKey: remoteKey.trim() || null,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["finance-api-settings"] });
      setRemoteKey("");
    },
  });

  const canSave = mode === "local" || remoteUrl.trim().length > 0;

  return (
    <div className="flex flex-col gap-4">
      <h3 className="text-sm font-semibold text-muted-foreground">
        Finance API
      </h3>
      <p className="text-sm text-muted-foreground">
        Where market data comes from — a sidecar bundled with the app
        (Local) or a future paid EasyBusiness Cloud instance (Remote).
      </p>
      {settingsQuery.isError && (
        <p className="text-red-600">{settingsQuery.error.message}</p>
      )}

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <Field label="Mode">
          <Select
            value={mode}
            onValueChange={(value) => setMode(value as "local" | "remote")}
          >
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="local">Local</SelectItem>
              <SelectItem value="remote">Remote</SelectItem>
            </SelectContent>
          </Select>
        </Field>
        {mode === "remote" && (
          <Field label="URL">
            <Input
              value={remoteUrl}
              onChange={(e) => setRemoteUrl(e.currentTarget.value)}
              placeholder="https://api.easybusiness.example.com"
            />
          </Field>
        )}
      </div>

      {mode === "remote" && (
        <Field label="API key">
          <Input
            type="password"
            value={remoteKey}
            onChange={(e) => setRemoteKey(e.currentTarget.value)}
            placeholder={
              settingsQuery.data?.has_remote_key
                ? "Remote key configured — leave blank to keep it"
                : "No key set"
            }
          />
        </Field>
      )}

      {saveMutation.isError && (
        <p className="text-red-600">{saveMutation.error.message}</p>
      )}
      <Button
        type="button"
        className="w-fit"
        disabled={!canSave || saveMutation.isPending}
        onClick={() => saveMutation.mutate()}
      >
        {saveMutation.isPending ? "Saving..." : "Save"}
      </Button>
    </div>
  );
}

export default FinanceApiSettingsSection;

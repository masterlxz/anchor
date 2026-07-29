import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation } from "@tanstack/react-query";
import type { AppError } from "../types";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { renderQrToCanvas } from "./renderQr";

type TruthIdHandshakeResult = {
  port: number;
  desktop_version: string;
  accepted: boolean;
};

type TruthIdSignResult = {
  status: string;
  user_op_hash: string | null;
  transaction_hash: string | null;
  error: string | null;
};

type CrossDeviceSession = {
  session_id: string;
  ephemeral_priv_key_hex: string;
  expires_at_ms: number;
  qr_payload_json: string;
};

type CidRecordResponse = {
  cid: string;
  content_hash: string;
  updated_at: number;
  version: number;
  exists: boolean;
};

type PinResult = {
  status: string;
  cid: string | null;
  content_hash: string | null;
  providers_ok: string[] | null;
  providers_failed: string[] | null;
  error: string | null;
};

/**
 * Prova de conceito mínima da fatia 3: descobre um TruthID Desktop rodando
 * na mesma máquina, faz handshake, e manda 1 pedido de assinatura de teste
 * (transferência de valor zero pro endereço de burn — sem efeito econômico,
 * mas passa pelo pipeline real de UserOperation). Não é a Fase 8 (sync via
 * IPFS) — só prova que o mecanismo de assinatura delegada funciona.
 */
function TruthIdPanel() {
  const connectionMutation = useMutation<TruthIdHandshakeResult, AppError, void>({
    mutationFn: () => invoke("test_truthid_connection"),
  });

  const signRequestMutation = useMutation<TruthIdSignResult, AppError, void>({
    mutationFn: () => invoke("send_test_sign_request"),
  });

  // Fatia cross-device: em vez de falar loopback com um TruthID Desktop na
  // mesma máquina, gera um QR e varre a LAN esperando um celular pareado
  // responder — mesmo protocolo `/sign-request` cross-device que o TruthID
  // Mobile já implementa desde a Sessão 110 (LAN, portas 48050-54).
  const crossDeviceSessionMutation = useMutation<CrossDeviceSession, AppError, void>({
    mutationFn: () => invoke("create_cross_device_sign_request"),
  });

  const crossDeviceResultMutation = useMutation<TruthIdSignResult, AppError, CrossDeviceSession>({
    mutationFn: (session) =>
      invoke("await_cross_device_sign_request_response", {
        sessionId: session.session_id,
        ephemeralPrivKeyHex: session.ephemeral_priv_key_hex,
        expiresAtMs: session.expires_at_ms,
      }),
  });

  // Fase 8.1: leitura pública (eth_call) do SyncRegistry — prova de conceito.
  const [syncAddress, setSyncAddress] = useState("");
  const syncRecordMutation = useMutation<CidRecordResponse | null, AppError, string>({
    mutationFn: (address) => invoke("get_sync_record", { address }),
  });

  // Fase 8.2: escrita do CID via o canal delegado do TruthID (mesma máquina).
  // CID/content hash ainda inseridos à mão — cifrar+publicar de verdade é
  // Fase 8.3/8.4, fora do escopo desta fatia.
  const [updateCid, setUpdateCid] = useState("");
  const [updateContentHash, setUpdateContentHash] = useState("");
  const updateSyncRecordMutation = useMutation<
    TruthIdSignResult,
    AppError,
    { cid: string; contentHash: string }
  >({
    mutationFn: ({ cid, contentHash }) =>
      invoke("update_sync_record", { cid, contentHash }),
  });

  // Fase 8.3: pina os bytes reais do arquivo SQLite atual via o proxy de
  // pinning do TruthID — o resultado (cid/content_hash) alimenta direto a
  // seção 8.2 acima, sem precisar digitar nada à mão.
  const pinDatabaseMutation = useMutation<PinResult, AppError, void>({
    mutationFn: () => invoke("pin_database_snapshot"),
  });

  function useThisPinResult() {
    if (!pinDatabaseMutation.data?.cid || !pinDatabaseMutation.data?.content_hash) return;
    setUpdateCid(pinDatabaseMutation.data.cid);
    setUpdateContentHash(pinDatabaseMutation.data.content_hash);
  }

  const qrCanvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const session = crossDeviceSessionMutation.data;
    if (!session || !qrCanvasRef.current) return;
    void renderQrToCanvas(qrCanvasRef.current, session.qr_payload_json);
    // O celular só começa a servir depois de aprovar (e, no sign-request,
    // depois da UserOp terminar de executar, até ~60s) — dispara a
    // varredura assim que o QR aparece, sem esperar clique, mesma filosofia
    // de "já começa a servir" do lado Mobile.
    crossDeviceResultMutation.mutate(session);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [crossDeviceSessionMutation.data]);

  function startCrossDeviceRequest() {
    crossDeviceResultMutation.reset();
    crossDeviceSessionMutation.mutate();
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>TruthID Sync (proof of concept)</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-6">
        <div className="flex flex-col gap-2">
          <Button
            onClick={() => connectionMutation.mutate()}
            disabled={connectionMutation.isPending}
          >
            {connectionMutation.isPending ? "Looking for TruthID..." : "Test connection"}
          </Button>
          {connectionMutation.isError && (
            <p className="text-red-600">{connectionMutation.error.message}</p>
          )}
          {connectionMutation.isSuccess && (
            <p className="text-green-700">
              Found TruthID Desktop {connectionMutation.data.desktop_version} on port{" "}
              {connectionMutation.data.port}.
            </p>
          )}
        </div>

        <div className="flex flex-col gap-2">
          <Button
            variant="outline"
            onClick={() => signRequestMutation.mutate()}
            disabled={signRequestMutation.isPending}
          >
            {signRequestMutation.isPending
              ? "Waiting for approval in TruthID..."
              : "Send test sign-request"}
          </Button>
          {signRequestMutation.isError && (
            <p className="text-red-600">{signRequestMutation.error.message}</p>
          )}
          {signRequestMutation.isSuccess && (
            <div>
              <p>Status: {signRequestMutation.data.status}</p>
              {signRequestMutation.data.user_op_hash && (
                <p className="break-all">userOpHash: {signRequestMutation.data.user_op_hash}</p>
              )}
              {signRequestMutation.data.transaction_hash && (
                <p className="break-all">
                  transactionHash: {signRequestMutation.data.transaction_hash}
                </p>
              )}
              {signRequestMutation.data.error && (
                <p className="text-red-600">{signRequestMutation.data.error}</p>
              )}
            </div>
          )}
        </div>

        <div className="flex flex-col gap-2 border-t pt-6">
          <p className="text-sm text-muted-foreground">
            Cross-device: scan with your paired TruthID phone instead of a TruthID Desktop on this
            same machine.
          </p>
          <Button
            variant="outline"
            onClick={startCrossDeviceRequest}
            disabled={crossDeviceSessionMutation.isPending || crossDeviceResultMutation.isPending}
          >
            {crossDeviceResultMutation.isPending
              ? "Waiting for your phone (LAN + IPFS backup)..."
              : "Start cross-device request"}
          </Button>
          {crossDeviceSessionMutation.isError && (
            <p className="text-red-600">{crossDeviceSessionMutation.error.message}</p>
          )}
          {crossDeviceSessionMutation.isSuccess && !crossDeviceResultMutation.isSuccess && (
            <div className="flex flex-col items-center gap-2">
              <canvas ref={qrCanvasRef} />
              <p className="text-sm text-muted-foreground">
                Scan this QR with your paired TruthID phone.
              </p>
            </div>
          )}
          {crossDeviceResultMutation.isError && (
            <p className="text-red-600">{crossDeviceResultMutation.error.message}</p>
          )}
          {crossDeviceResultMutation.isSuccess && (
            <div>
              <p>Status: {crossDeviceResultMutation.data.status}</p>
              {crossDeviceResultMutation.data.user_op_hash && (
                <p className="break-all">
                  userOpHash: {crossDeviceResultMutation.data.user_op_hash}
                </p>
              )}
              {crossDeviceResultMutation.data.transaction_hash && (
                <p className="break-all">
                  transactionHash: {crossDeviceResultMutation.data.transaction_hash}
                </p>
              )}
              {crossDeviceResultMutation.data.error && (
                <p className="text-red-600">{crossDeviceResultMutation.data.error}</p>
              )}
            </div>
          )}
        </div>

        <div className="flex flex-col gap-2 border-t pt-6">
          <p className="text-sm text-muted-foreground">
            Fase 8.1 — SyncRegistry (Base Sepolia): read-only proof of concept.
          </p>
          <div className="flex gap-2">
            <Input
              placeholder="0x..."
              value={syncAddress}
              onChange={(e) => setSyncAddress(e.target.value)}
            />
            <Button
              variant="outline"
              onClick={() => syncRecordMutation.mutate(syncAddress)}
              disabled={syncRecordMutation.isPending || !syncAddress}
            >
              {syncRecordMutation.isPending ? "Reading..." : "Read sync record"}
            </Button>
          </div>
          {syncRecordMutation.isError && (
            <p className="text-red-600">{syncRecordMutation.error.message}</p>
          )}
          {syncRecordMutation.isSuccess && syncRecordMutation.data === null && (
            <p className="text-muted-foreground">No record found for this address.</p>
          )}
          {syncRecordMutation.isSuccess && syncRecordMutation.data && (
            <div>
              <p>CID: {syncRecordMutation.data.cid}</p>
              <p className="break-all">Content hash: {syncRecordMutation.data.content_hash}</p>
              <p>Version: {syncRecordMutation.data.version}</p>
              <p>Updated at: {new Date(syncRecordMutation.data.updated_at * 1000).toLocaleString()}</p>
            </div>
          )}
        </div>

        <div className="flex flex-col gap-2 border-t pt-6">
          <p className="text-sm text-muted-foreground">
            Fase 8.3 — Pin database snapshot: pins the current SQLite file's real bytes via
            TruthID's pinning proxy. No real encryption yet (Fase 8.4) — this is the raw file.
          </p>
          <Button
            variant="outline"
            onClick={() => pinDatabaseMutation.mutate()}
            disabled={pinDatabaseMutation.isPending}
          >
            {pinDatabaseMutation.isPending
              ? "Waiting for approval in TruthID (up to 5 min)..."
              : "Pin database snapshot"}
          </Button>
          {pinDatabaseMutation.isError && (
            <p className="text-red-600">{pinDatabaseMutation.error.message}</p>
          )}
          {pinDatabaseMutation.isSuccess && (
            <div>
              <p>Status: {pinDatabaseMutation.data.status}</p>
              {pinDatabaseMutation.data.cid && <p className="break-all">CID: {pinDatabaseMutation.data.cid}</p>}
              {pinDatabaseMutation.data.content_hash && (
                <p className="break-all">Content hash: {pinDatabaseMutation.data.content_hash}</p>
              )}
              {pinDatabaseMutation.data.providers_ok && pinDatabaseMutation.data.providers_ok.length > 0 && (
                <p className="text-green-700">Pinned via: {pinDatabaseMutation.data.providers_ok.join(", ")}</p>
              )}
              {pinDatabaseMutation.data.providers_failed &&
                pinDatabaseMutation.data.providers_failed.length > 0 && (
                  <p className="text-red-600">
                    Failed providers: {pinDatabaseMutation.data.providers_failed.join(", ")}
                  </p>
                )}
              {pinDatabaseMutation.data.error && (
                <p className="text-red-600">{pinDatabaseMutation.data.error}</p>
              )}
              {pinDatabaseMutation.data.cid && pinDatabaseMutation.data.content_hash && (
                <Button variant="outline" onClick={useThisPinResult}>
                  Use this result in Update sync record
                </Button>
              )}
            </div>
          )}
        </div>

        <div className="flex flex-col gap-2 border-t pt-6">
          <p className="text-sm text-muted-foreground">
            Fase 8.2 — Update sync record (same-machine only): CID/content hash can be pasted from
            the pin result above, or typed manually. No real encryption yet (Fase 8.4).
          </p>
          <Input placeholder="CID (bafy...)" value={updateCid} onChange={(e) => setUpdateCid(e.target.value)} />
          <Input
            placeholder="Content hash (0x + 64 hex chars)"
            value={updateContentHash}
            onChange={(e) => setUpdateContentHash(e.target.value)}
          />
          <Button
            variant="outline"
            onClick={() =>
              updateSyncRecordMutation.mutate({ cid: updateCid, contentHash: updateContentHash })
            }
            disabled={updateSyncRecordMutation.isPending || !updateCid || !updateContentHash}
          >
            {updateSyncRecordMutation.isPending ? "Waiting for approval..." : "Update sync record"}
          </Button>
          {updateSyncRecordMutation.isError && (
            <p className="text-red-600">{updateSyncRecordMutation.error.message}</p>
          )}
          {updateSyncRecordMutation.isSuccess && (
            <div>
              <p>Status: {updateSyncRecordMutation.data.status}</p>
              {updateSyncRecordMutation.data.user_op_hash && (
                <p className="break-all">userOpHash: {updateSyncRecordMutation.data.user_op_hash}</p>
              )}
              {updateSyncRecordMutation.data.transaction_hash && (
                <p className="break-all">
                  transactionHash: {updateSyncRecordMutation.data.transaction_hash}
                </p>
              )}
              {updateSyncRecordMutation.data.error && (
                <p className="text-red-600">{updateSyncRecordMutation.data.error}</p>
              )}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

export default TruthIdPanel;

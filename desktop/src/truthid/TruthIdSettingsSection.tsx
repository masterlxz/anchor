import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation } from "@tanstack/react-query";
import type { AppError } from "../types";
import { Button } from "@/components/ui/button";
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

type SignMessageResult = {
  status: string;
  message: string | null;
  signature: string | null;
  error: string | null;
};

/** Bloco de exibição de um `PinResult`, reusado pelo pin loopback (Fase 8.3)
 * e pelo pin cross-device (Fase 8.4) — mesmos campos, mesma origem de tipo. */
function PinResultDisplay({
  result,
  onUseResult,
}: {
  result: PinResult;
  onUseResult: () => void;
}) {
  return (
    <div>
      <p>Status: {result.status}</p>
      {result.cid && <p className="break-all">CID: {result.cid}</p>}
      {result.content_hash && (
        <p className="break-all">Content hash: {result.content_hash}</p>
      )}
      {result.providers_ok && result.providers_ok.length > 0 && (
        <p className="text-green-700">Pinned via: {result.providers_ok.join(", ")}</p>
      )}
      {result.providers_failed && result.providers_failed.length > 0 && (
        <p className="text-red-600">
          Failed providers: {result.providers_failed.join(", ")}
        </p>
      )}
      {result.error && <p className="text-red-600">{result.error}</p>}
      {result.cid && result.content_hash && (
        <Button variant="outline" onClick={onUseResult}>
          Use this result in Update sync record
        </Button>
      )}
    </div>
  );
}

/**
 * Prova de conceito mínima da fatia 3 (Fase 8): descobre um TruthID Desktop
 * rodando na mesma máquina, faz handshake, e manda 1 pedido de assinatura de
 * teste (transferência de valor zero pro endereço de burn — sem efeito
 * econômico, mas passa pelo pipeline real de UserOperation). Não é a Fase 8
 * completa (sync via IPFS) — só prova que o mecanismo de assinatura delegada
 * funciona.
 *
 * Migrado de aba própria no nav principal (`TruthIdPanel.tsx`, era o único
 * conteúdo da antiga aba "TruthID Sync") pra dentro de Settings (Sessão 54,
 * pedido explícito do dono do projeto) — mesmo lugar que já abriga a seção
 * "IA" (`IaSettingsSection`, `SettingsPage.tsx`). Sem `Card` própria (o
 * `Card` já vem de fora, de `SettingsPage.tsx` — evita cartão dentro de
 * cartão, mesmo padrão que `IaSettingsSection` já segue).
 */
function TruthIdSettingsSection() {
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

  function useThisPinResult(result: PinResult) {
    if (!result.cid || !result.content_hash) return;
    setUpdateCid(result.cid);
    setUpdateContentHash(result.content_hash);
  }

  // Cross-device pin: pede pro celular pinar o snapshot em vez de falar
  // loopback com um TruthID Desktop na mesma máquina — mesmo protocolo que o
  // TruthID Mobile já implementa desde a sessão pós-127 (`PinApprovalScreen`,
  // LAN nas portas 48050-54). Desde a Fase 8.4 (Sessão 87) isso é um fluxo de
  // **2 QRs em sequência**: 1º pede a assinatura via `/sign-message`
  // (deriva a chave de cifra do snapshot — ver `crossDeviceSignMessage*`
  // abaixo), só depois o 2º empurra o conteúdo já cifrado em duas camadas
  // (`snapshotSignatureRef`, lido dentro do `push_pin_content` mutationFn,
  // nunca precisa virar estado React porque só é escrito e lido dentro desta
  // mesma sequência, sem re-render no meio).
  const snapshotSignatureRef = useRef("");

  const crossDeviceSignMessageSessionMutation = useMutation<CrossDeviceSession, AppError, void>({
    mutationFn: () => invoke("create_cross_device_sign_message_request"),
  });

  const crossDeviceSignMessageResultMutation = useMutation<
    SignMessageResult,
    AppError,
    CrossDeviceSession
  >({
    mutationFn: (session) =>
      invoke("await_cross_device_sign_message_response", {
        sessionId: session.session_id,
        ephemeralPrivKeyHex: session.ephemeral_priv_key_hex,
        expiresAtMs: session.expires_at_ms,
      }),
  });

  const crossDevicePinSessionMutation = useMutation<CrossDeviceSession, AppError, void>({
    mutationFn: () => invoke("create_cross_device_pin_request"),
  });

  const crossDevicePinPushMutation = useMutation<void, AppError, CrossDeviceSession>({
    mutationFn: (session) =>
      invoke("push_pin_content", {
        sessionId: session.session_id,
        expiresAtMs: session.expires_at_ms,
        snapshotSignatureHex: snapshotSignatureRef.current,
      }),
  });

  const crossDevicePinResultMutation = useMutation<PinResult, AppError, CrossDeviceSession>({
    mutationFn: (session) =>
      invoke("await_cross_device_pin_response", {
        sessionId: session.session_id,
        ephemeralPrivKeyHex: session.ephemeral_priv_key_hex,
        expiresAtMs: session.expires_at_ms,
      }),
  });

  const qrCanvasRef = useRef<HTMLCanvasElement>(null);
  const signMessageQrCanvasRef = useRef<HTMLCanvasElement>(null);
  const pinQrCanvasRef = useRef<HTMLCanvasElement>(null);

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

  // 1º QR do fluxo de pin cross-device: assim que a sessão de sign-message
  // aparece, renderiza o QR e já dispara a espera pela resposta do celular —
  // mesma filosofia de "já começa a servir" das outras sessões cross-device.
  useEffect(() => {
    const session = crossDeviceSignMessageSessionMutation.data;
    if (!session || !signMessageQrCanvasRef.current) return;
    void renderQrToCanvas(signMessageQrCanvasRef.current, session.qr_payload_json);
    crossDeviceSignMessageResultMutation.mutate(session);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [crossDeviceSignMessageSessionMutation.data]);

  // Assim que o celular assina (status "signed"), guarda a assinatura pro
  // `push_pin_content` ler depois e dispara o 2º QR (pin) — se rejeitado ou
  // com erro, para aqui, sem avançar pro pin (já fica exposto via
  // `crossDeviceSignMessageResultMutation.data.error`/`.isError`).
  useEffect(() => {
    const result = crossDeviceSignMessageResultMutation.data;
    if (!result || result.status !== "signed" || !result.signature) return;
    snapshotSignatureRef.current = result.signature;
    crossDevicePinSessionMutation.mutate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [crossDeviceSignMessageResultMutation.data]);

  useEffect(() => {
    const session = crossDevicePinSessionMutation.data;
    if (!session || !pinQrCanvasRef.current) return;
    void renderQrToCanvas(pinQrCanvasRef.current, session.qr_payload_json);
    // Sequencial, não em paralelo: o celular só decifra o conteúdo (fase 1)
    // antes de mostrar a tela de aprovação — só faz sentido esperar o
    // resultado (fase 2) depois que o push realmente for aceito.
    void (async () => {
      try {
        await crossDevicePinPushMutation.mutateAsync(session);
        await crossDevicePinResultMutation.mutateAsync(session);
      } catch {
        // Erro já fica exposto via *.isError/*.error de cada mutation —
        // nada a fazer aqui além de não deixar a rejection subir sem tratar.
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [crossDevicePinSessionMutation.data]);

  function startCrossDevicePinFlow() {
    snapshotSignatureRef.current = "";
    crossDeviceSignMessageResultMutation.reset();
    crossDevicePinSessionMutation.reset();
    crossDevicePinPushMutation.reset();
    crossDevicePinResultMutation.reset();
    crossDeviceSignMessageSessionMutation.mutate();
  }

  return (
    <div className="flex flex-col gap-6">
      <p className="text-sm text-muted-foreground">
        TruthID Sync (proof of concept) — delegated signing + IPFS pinning integration, Fase 8.
      </p>

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
          Fase 8.1 — SyncRegistry (Base Mainnet): read-only proof of concept.
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
          Fase 8.3 — Pin database snapshot: pins the current SQLite file via TruthID's pinning
          proxy. Encrypted since Fase 8.4 — asks TruthID for a sign-message signature first (a
          second approval prompt) to derive the encryption key, then encrypts the bytes before
          pinning.
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
          <PinResultDisplay
            result={pinDatabaseMutation.data}
            onUseResult={() => useThisPinResult(pinDatabaseMutation.data)}
          />
        )}
      </div>

      <div className="flex flex-col gap-2 border-t pt-6">
        <p className="text-sm text-muted-foreground">
          Cross-device pin: scan with your paired TruthID phone instead of a TruthID Desktop on
          this same machine. Two QR codes in sequence — the first asks your phone to sign a
          message (derives the encryption key), the second pushes the already-encrypted snapshot
          for your phone to pin.
        </p>
        <Button
          variant="outline"
          onClick={startCrossDevicePinFlow}
          disabled={
            crossDeviceSignMessageSessionMutation.isPending ||
            crossDeviceSignMessageResultMutation.isPending ||
            crossDevicePinSessionMutation.isPending ||
            crossDevicePinPushMutation.isPending ||
            crossDevicePinResultMutation.isPending
          }
        >
          {crossDevicePinResultMutation.isPending
            ? "Waiting for approval on your phone..."
            : crossDevicePinPushMutation.isPending
              ? "Sending content to your phone..."
              : crossDeviceSignMessageResultMutation.isPending
                ? "Waiting for signature on your phone..."
                : "Start cross-device pin"}
        </Button>
        {crossDeviceSignMessageSessionMutation.isError && (
          <p className="text-red-600">{crossDeviceSignMessageSessionMutation.error.message}</p>
        )}
        {crossDeviceSignMessageSessionMutation.isSuccess &&
          !crossDeviceSignMessageResultMutation.isSuccess && (
            <div className="flex flex-col items-center gap-2">
              <canvas ref={signMessageQrCanvasRef} />
              <p className="text-sm text-muted-foreground">
                1/2 — Scan this QR with your paired TruthID phone to sign the encryption key.
              </p>
            </div>
          )}
        {crossDeviceSignMessageResultMutation.isError && (
          <p className="text-red-600">{crossDeviceSignMessageResultMutation.error.message}</p>
        )}
        {crossDeviceSignMessageResultMutation.isSuccess &&
          crossDeviceSignMessageResultMutation.data.status !== "signed" && (
            <p className="text-red-600">
              Status: {crossDeviceSignMessageResultMutation.data.status}
              {crossDeviceSignMessageResultMutation.data.error &&
                ` — ${crossDeviceSignMessageResultMutation.data.error}`}
            </p>
          )}
        {crossDevicePinSessionMutation.isError && (
          <p className="text-red-600">{crossDevicePinSessionMutation.error.message}</p>
        )}
        {crossDevicePinSessionMutation.isSuccess && !crossDevicePinResultMutation.isSuccess && (
          <div className="flex flex-col items-center gap-2">
            <canvas ref={pinQrCanvasRef} />
            <p className="text-sm text-muted-foreground">
              2/2 — Scan this QR with your paired TruthID phone to pin the encrypted content.
            </p>
          </div>
        )}
        {crossDevicePinPushMutation.isError && (
          <p className="text-red-600">{crossDevicePinPushMutation.error.message}</p>
        )}
        {crossDevicePinResultMutation.isError && (
          <p className="text-red-600">{crossDevicePinResultMutation.error.message}</p>
        )}
        {crossDevicePinResultMutation.isSuccess && (
          <PinResultDisplay
            result={crossDevicePinResultMutation.data}
            onUseResult={() => useThisPinResult(crossDevicePinResultMutation.data)}
          />
        )}
      </div>

      <div className="flex flex-col gap-2 border-t pt-6">
        <p className="text-sm text-muted-foreground">
          Fase 8.2 — Update sync record (same-machine only): CID/content hash can be pasted from
          the pin result above, or typed manually.
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
    </div>
  );
}

export default TruthIdSettingsSection;

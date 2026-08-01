import { useState, type FormEvent } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import * as XLSX from "xlsx";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { AppError } from "../types";
import type { Asset, Thesis, ThesisAttachment } from "./types";
import Field from "../components/Field";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

type CreateThesisRequest = {
  workspace_id: number;
  asset_id: number | null;
  title: string;
  content_markdown: string;
};
type UpdateThesisRequest = {
  thesis_id: number;
  asset_id: number | null;
  title: string;
  content_markdown: string;
};
type AddThesisAttachmentRequest = { thesis_id: number; source_path: string };

const ATTACHMENT_EXTENSIONS = ["pdf", "png", "jpg", "jpeg", "gif", "webp", "xlsx", "xls", "csv"];

function assetLabel(assets: Asset[], assetId: number | null): string {
  if (assetId === null) return "Global (sem ativo)";
  const asset = assets.find((a) => a.id === assetId);
  return asset ? `${asset.ticker} — ${asset.name}` : `#${assetId}`;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function attachmentKind(attachment: ThesisAttachment): "image" | "pdf" | "spreadsheet" | "other" {
  const type = attachment.content_type ?? "";
  if (type.startsWith("image/")) return "image";
  if (type === "application/pdf") return "pdf";
  if (
    type === "text/csv" ||
    type === "application/vnd.ms-excel" ||
    type === "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
  ) {
    return "spreadsheet";
  }
  return "other";
}

// Fase 10.5 — teses de investimento (vinculáveis a um ativo ou globais/macro)
// com anexos guardados em disco (não bucket, ver PHASE.md item 10.5).
// Visualização é sempre dentro do próprio app, sem abrir programa externo
// (pedido explícito do dono do projeto): imagem/PDF via `convertFileSrc` +
// `<img>`/`<iframe>` (o WebView já sabe renderizar os dois nativamente),
// planilha via `fetch` sobre a mesma URL + SheetJS, desenhada como tabela.
function ThesisSection({ workspaceId }: { workspaceId: number }) {
  const [openThesisId, setOpenThesisId] = useState<number | "new" | null>(null);
  const [formTitle, setFormTitle] = useState("");
  const [formAssetId, setFormAssetId] = useState("none");
  const [formContent, setFormContent] = useState("");
  const [formMode, setFormMode] = useState<"edit" | "preview">("edit");
  const [confirmingDeleteThesisId, setConfirmingDeleteThesisId] = useState<number | null>(null);

  const [uploadError, setUploadError] = useState<string | null>(null);
  const [confirmingDeleteAttachmentId, setConfirmingDeleteAttachmentId] = useState<number | null>(
    null,
  );
  const [viewingAttachmentId, setViewingAttachmentId] = useState<number | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [spreadsheetRows, setSpreadsheetRows] = useState<unknown[][] | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);

  const queryClient = useQueryClient();

  const assetsQuery = useQuery<Asset[], AppError>({
    queryKey: ["assets"],
    queryFn: () => invoke("list_assets"),
  });
  const assets = assetsQuery.data ?? [];

  const thesesQuery = useQuery<Thesis[], AppError>({
    queryKey: ["theses", workspaceId],
    queryFn: () => invoke("list_theses", { workspaceId }),
  });
  const theses = thesesQuery.data ?? [];

  function closePreview() {
    setViewingAttachmentId(null);
    setPreviewUrl(null);
    setSpreadsheetRows(null);
    setPreviewError(null);
  }

  function openForCreating() {
    setOpenThesisId("new");
    setFormTitle("");
    setFormAssetId("none");
    setFormContent("");
    setFormMode("edit");
    setConfirmingDeleteThesisId(null);
    closePreview();
  }

  function openForEditing(thesis: Thesis) {
    setOpenThesisId(thesis.id);
    setFormTitle(thesis.title);
    setFormAssetId(thesis.asset_id !== null ? String(thesis.asset_id) : "none");
    setFormContent(thesis.content_markdown);
    setFormMode("edit");
    setConfirmingDeleteThesisId(null);
    closePreview();
  }

  const createMutation = useMutation<Thesis, AppError, CreateThesisRequest>({
    mutationFn: (request) => invoke("create_thesis", { request }),
    onSuccess: (created) => {
      queryClient.invalidateQueries({ queryKey: ["theses", workspaceId] });
      openForEditing(created);
    },
  });

  const updateMutation = useMutation<Thesis, AppError, UpdateThesisRequest>({
    mutationFn: (request) => invoke("update_thesis", { request }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["theses", workspaceId] });
    },
  });

  const deleteMutation = useMutation<void, AppError, number>({
    mutationFn: (thesisId) => invoke("delete_thesis", { thesisId }),
    onSuccess: (_data, thesisId) => {
      queryClient.invalidateQueries({ queryKey: ["theses", workspaceId] });
      setConfirmingDeleteThesisId(null);
      if (openThesisId === thesisId) {
        setOpenThesisId(null);
      }
    },
  });

  const attachmentsQuery = useQuery<ThesisAttachment[], AppError>({
    queryKey: ["thesis-attachments", openThesisId],
    queryFn: () => invoke("list_thesis_attachments", { thesisId: openThesisId }),
    enabled: typeof openThesisId === "number",
  });
  const attachments = attachmentsQuery.data ?? [];

  const addAttachmentMutation = useMutation<
    ThesisAttachment,
    AppError,
    AddThesisAttachmentRequest
  >({
    mutationFn: (request) => invoke("add_thesis_attachment", { request }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["thesis-attachments", openThesisId] });
    },
  });

  const deleteAttachmentMutation = useMutation<void, AppError, number>({
    mutationFn: (attachmentId) => invoke("delete_thesis_attachment", { attachmentId }),
    onSuccess: (_data, attachmentId) => {
      queryClient.invalidateQueries({ queryKey: ["thesis-attachments", openThesisId] });
      setConfirmingDeleteAttachmentId(null);
      if (viewingAttachmentId === attachmentId) closePreview();
    },
  });

  function handleSubmitForm(event: FormEvent) {
    event.preventDefault();
    const assetId = formAssetId === "none" ? null : Number(formAssetId);
    if (openThesisId === "new") {
      createMutation.mutate({
        workspace_id: workspaceId,
        asset_id: assetId,
        title: formTitle,
        content_markdown: formContent,
      });
    } else if (typeof openThesisId === "number") {
      updateMutation.mutate({
        thesis_id: openThesisId,
        asset_id: assetId,
        title: formTitle,
        content_markdown: formContent,
      });
    }
  }

  function handleDeleteThesisClick(thesisId: number) {
    if (confirmingDeleteThesisId === thesisId) {
      deleteMutation.mutate(thesisId);
    } else {
      setConfirmingDeleteThesisId(thesisId);
    }
  }

  async function handleUploadAttachments() {
    if (typeof openThesisId !== "number") return;
    const picked = await open({
      multiple: true,
      filters: [{ name: "Anexos", extensions: ATTACHMENT_EXTENSIONS }],
    });
    if (!picked) return;
    setUploadError(null);
    for (const sourcePath of picked) {
      try {
        await addAttachmentMutation.mutateAsync({
          thesis_id: openThesisId,
          source_path: sourcePath,
        });
      } catch (err) {
        setUploadError(err instanceof Error ? err.message : `Erro ao anexar ${sourcePath}.`);
      }
    }
  }

  function handleDeleteAttachmentClick(attachmentId: number) {
    if (confirmingDeleteAttachmentId === attachmentId) {
      deleteAttachmentMutation.mutate(attachmentId);
    } else {
      setConfirmingDeleteAttachmentId(attachmentId);
    }
  }

  async function handleViewAttachment(attachment: ThesisAttachment) {
    setViewingAttachmentId(attachment.id);
    setPreviewError(null);
    setSpreadsheetRows(null);
    setPreviewUrl(null);
    try {
      const path = await invoke<string>("get_thesis_attachment_path", {
        attachmentId: attachment.id,
      });
      const url = convertFileSrc(path);
      if (attachmentKind(attachment) === "spreadsheet") {
        const buffer = await fetch(url).then((r) => r.arrayBuffer());
        const workbook = XLSX.read(buffer, { type: "array" });
        const sheet = workbook.Sheets[workbook.SheetNames[0]];
        setSpreadsheetRows(XLSX.utils.sheet_to_json<unknown[]>(sheet, { header: 1 }));
      } else {
        setPreviewUrl(url);
      }
    } catch (err) {
      setPreviewError(err instanceof Error ? err.message : "Erro ao carregar preview.");
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <Card>
        <CardHeader>
          <CardTitle>Teses de Investimento</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="mb-4 flex justify-end">
            <Button type="button" onClick={openForCreating}>
              Nova tese
            </Button>
          </div>

          {thesesQuery.isError && <p className="mb-3 text-red-600">{thesesQuery.error.message}</p>}

          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Título</TableHead>
                <TableHead>Vinculada a</TableHead>
                <TableHead>Atualizada em</TableHead>
                <TableHead>Ações</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {theses.length === 0 && (
                <TableRow>
                  <TableCell colSpan={4} className="text-center text-muted-foreground">
                    Nenhuma tese cadastrada ainda.
                  </TableCell>
                </TableRow>
              )}
              {theses.map((thesis) => {
                const isConfirmingDelete = confirmingDeleteThesisId === thesis.id;
                return (
                  <TableRow key={thesis.id}>
                    <TableCell>{thesis.title}</TableCell>
                    <TableCell>{assetLabel(assets, thesis.asset_id)}</TableCell>
                    <TableCell>{new Date(thesis.updated_at).toLocaleString()}</TableCell>
                    <TableCell className="flex gap-2">
                      <Button size="sm" variant="outline" onClick={() => openForEditing(thesis)}>
                        Ver/Editar
                      </Button>
                      <Button
                        size="sm"
                        variant={isConfirmingDelete ? "destructive" : "outline"}
                        onClick={() => handleDeleteThesisClick(thesis.id)}
                      >
                        {isConfirmingDelete ? "Confirm?" : "Delete"}
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {openThesisId !== null && (
        <Card>
          <CardHeader>
            <CardTitle>{openThesisId === "new" ? "Nova tese" : "Editar tese"}</CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleSubmitForm} className="mb-6 flex flex-col gap-4">
              <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                <Field label="Título">
                  <Input
                    required
                    value={formTitle}
                    onChange={(e) => setFormTitle(e.currentTarget.value)}
                  />
                </Field>
                <Field label="Ativo vinculado">
                  <Select value={formAssetId} onValueChange={setFormAssetId}>
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="none">Global (sem ativo)</SelectItem>
                      {assets.map((asset) => (
                        <SelectItem key={asset.id} value={String(asset.id)}>
                          {asset.ticker} — {asset.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
              </div>

              <Field label="Conteúdo (Markdown)">
                <div className="mb-2 flex gap-2">
                  <Button
                    type="button"
                    size="sm"
                    variant={formMode === "edit" ? "default" : "outline"}
                    onClick={() => setFormMode("edit")}
                  >
                    Editar
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant={formMode === "preview" ? "default" : "outline"}
                    onClick={() => setFormMode("preview")}
                  >
                    Preview
                  </Button>
                </div>
                {formMode === "edit" ? (
                  <Textarea
                    className="min-h-48 font-mono text-sm"
                    value={formContent}
                    onChange={(e) => setFormContent(e.currentTarget.value)}
                  />
                ) : (
                  <div className="min-h-48 rounded-lg border border-input px-3 py-2">
                    <ReactMarkdown remarkPlugins={[remarkGfm]}>
                      {formContent || "*Nada pra mostrar ainda.*"}
                    </ReactMarkdown>
                  </div>
                )}
              </Field>

              {(createMutation.isError || updateMutation.isError) && (
                <p className="text-red-600">
                  {(createMutation.error ?? updateMutation.error)?.message}
                </p>
              )}

              <div>
                <Button
                  type="submit"
                  disabled={createMutation.isPending || updateMutation.isPending}
                >
                  {createMutation.isPending || updateMutation.isPending ? "Salvando..." : "Salvar"}
                </Button>
              </div>
            </form>

            {typeof openThesisId === "number" && (
              <div className="border-t pt-6">
                <div className="mb-4 flex items-center justify-between">
                  <h3 className="text-sm font-semibold">Anexos</h3>
                  <Button type="button" variant="outline" size="sm" onClick={handleUploadAttachments}>
                    Adicionar anexo
                  </Button>
                </div>

                {uploadError && <p className="mb-3 text-red-600">{uploadError}</p>}
                {attachmentsQuery.isError && (
                  <p className="mb-3 text-red-600">{attachmentsQuery.error.message}</p>
                )}

                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Nome</TableHead>
                      <TableHead>Tamanho</TableHead>
                      <TableHead>Ações</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {attachments.length === 0 && (
                      <TableRow>
                        <TableCell colSpan={3} className="text-center text-muted-foreground">
                          Nenhum anexo ainda.
                        </TableCell>
                      </TableRow>
                    )}
                    {attachments.map((attachment) => {
                      const isConfirmingDelete = confirmingDeleteAttachmentId === attachment.id;
                      return (
                        <TableRow key={attachment.id}>
                          <TableCell>{attachment.original_file_name}</TableCell>
                          <TableCell>{formatBytes(attachment.file_size_bytes)}</TableCell>
                          <TableCell className="flex gap-2">
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => handleViewAttachment(attachment)}
                            >
                              Ver
                            </Button>
                            <Button
                              size="sm"
                              variant={isConfirmingDelete ? "destructive" : "outline"}
                              onClick={() => handleDeleteAttachmentClick(attachment.id)}
                            >
                              {isConfirmingDelete ? "Confirm?" : "Delete"}
                            </Button>
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>

                {viewingAttachmentId !== null && (
                  <div className="mt-4">
                    <div className="mb-2 flex justify-end">
                      <Button size="sm" variant="outline" onClick={closePreview}>
                        Fechar preview
                      </Button>
                    </div>
                    {previewError && <p className="text-red-600">{previewError}</p>}
                    {!previewError && spreadsheetRows && (
                      <div className="max-h-[500px] overflow-auto rounded border">
                        <table className="w-full text-sm">
                          <tbody>
                            {spreadsheetRows.map((row, rowIndex) => (
                              <tr key={rowIndex}>
                                {row.map((cell, cellIndex) => (
                                  <td key={cellIndex} className="border px-2 py-1">
                                    {String(cell ?? "")}
                                  </td>
                                ))}
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    )}
                    {!previewError && !spreadsheetRows && previewUrl && (
                      (() => {
                        const attachment = attachments.find((a) => a.id === viewingAttachmentId);
                        const kind = attachment ? attachmentKind(attachment) : "other";
                        if (kind === "image") {
                          return (
                            <img
                              src={previewUrl}
                              alt={attachment?.original_file_name ?? "anexo"}
                              className="max-h-[500px] w-full object-contain"
                            />
                          );
                        }
                        if (kind === "pdf") {
                          return (
                            <iframe
                              src={previewUrl}
                              title={attachment?.original_file_name ?? "anexo"}
                              className="h-[500px] w-full rounded border"
                            />
                          );
                        }
                        return (
                          <p className="text-muted-foreground">
                            Sem preview disponível para este tipo de arquivo.
                          </p>
                        );
                      })()
                    )}
                  </div>
                )}
              </div>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}

export default ThesisSection;

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
import type { Asset, FiiCvmMonthly, FiiCvmProperty } from "./types";
import Field from "../components/Field";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

type CollectorSummary = { success: boolean; output: string };
type UpdateAssetCnpjRequest = { asset_id: number; cnpj: string };

function formatPct(value: number | null): string {
  return value === null ? "—" : `${(value * 100).toFixed(2)}%`;
}

function formatCurrency(value: number): string {
  return value.toLocaleString("pt-BR", { style: "currency", currency: "BRL" });
}

function formatArea(value: number | null): string {
  return value === null ? "—" : `${value.toLocaleString("pt-BR")} m²`;
}

/// Fase 10, item 8, Sessão 41 — bloco expandido por FII na tabela de Ativos
/// (`AssetSection.tsx`). Sem CNPJ salvo, só oferece o campo pra colar/salvar
/// manualmente (`update_asset_cnpj` — a sugestão automática já roda na hora
/// do cadastro, ver `resolveCnpjMutation` em `AssetSection.tsx`; isto aqui é
/// o caminho de recuperação quando aquela sugestão falhou ou o ativo já
/// existia de antes). Com CNPJ, mostra o indicador mensal mais recente e os
/// imóveis do trimestre mais recente, ambos direto da CVM — nenhuma chamada
/// à bolsai acontece a partir daqui.
function FiiCvmDetails({ asset }: { asset: Asset }) {
  const [cnpjInput, setCnpjInput] = useState("");
  const queryClient = useQueryClient();

  const saveCnpjMutation = useMutation<Asset, AppError, string>({
    mutationFn: (cnpj) =>
      invoke("update_asset_cnpj", {
        request: { asset_id: asset.id, cnpj } satisfies UpdateAssetCnpjRequest,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["assets"] });
    },
  });

  const monthlyQuery = useQuery<FiiCvmMonthly[], AppError>({
    queryKey: ["fii-cvm-monthly", asset.cnpj],
    queryFn: () => invoke("list_fii_cvm_monthly", { cnpj: asset.cnpj }),
    enabled: asset.cnpj !== null,
  });

  const propertiesQuery = useQuery<FiiCvmProperty[], AppError>({
    queryKey: ["fii-cvm-properties", asset.cnpj],
    queryFn: () => invoke("list_fii_cvm_properties", { cnpj: asset.cnpj }),
    enabled: asset.cnpj !== null,
  });

  const collectorMutation = useMutation<CollectorSummary, AppError, string>({
    mutationFn: (cnpj) => invoke("run_fii_cvm_collector", { cnpjs: [cnpj] }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["fii-cvm-monthly", asset.cnpj] });
      queryClient.invalidateQueries({ queryKey: ["fii-cvm-properties", asset.cnpj] });
    },
  });

  if (asset.cnpj === null) {
    return (
      <div className="flex items-end gap-3 py-2">
        <Field label="CNPJ do fundo (não resolvido automaticamente no cadastro)" className="flex-1 max-w-sm">
          <Input
            placeholder="00.000.000/0001-00"
            value={cnpjInput}
            onChange={(e) => setCnpjInput(e.currentTarget.value)}
          />
        </Field>
        <Button
          type="button"
          disabled={cnpjInput.trim() === "" || saveCnpjMutation.isPending}
          onClick={() => saveCnpjMutation.mutate(cnpjInput.trim())}
        >
          Salvar CNPJ
        </Button>
        {saveCnpjMutation.isError && (
          <p className="text-red-600">{saveCnpjMutation.error.message}</p>
        )}
      </div>
    );
  }

  const monthly = monthlyQuery.data ?? [];
  const latestMonthly = monthly[monthly.length - 1] ?? null;
  const properties = propertiesQuery.data ?? [];
  const hasAnyData = latestMonthly !== null || properties.length > 0;

  return (
    <div className="flex flex-col gap-4 py-2">
      <div className="flex items-center gap-3">
        <p className="text-sm text-muted-foreground">CNPJ: {asset.cnpj} — fonte: CVM</p>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={collectorMutation.isPending}
          onClick={() => collectorMutation.mutate(asset.cnpj as string)}
        >
          {collectorMutation.isPending ? "Atualizando..." : "Atualizar dados CVM"}
        </Button>
      </div>

      {collectorMutation.isError && (
        <p className="text-red-600">{collectorMutation.error.message}</p>
      )}
      {(monthlyQuery.isError || propertiesQuery.isError) && (
        <p className="text-red-600">
          {monthlyQuery.error?.message ?? propertiesQuery.error?.message}
        </p>
      )}

      {!hasAnyData && !monthlyQuery.isLoading && !propertiesQuery.isLoading && (
        <p className="text-muted-foreground">
          Nenhum dado da CVM ainda — clique em "Atualizar dados CVM" acima.
        </p>
      )}

      {latestMonthly && (
        <div className="grid grid-cols-2 gap-x-6 gap-y-1 text-sm sm:grid-cols-3">
          <p>
            <span className="text-muted-foreground">Referência: </span>
            {latestMonthly.reference_date}
          </p>
          <p>
            <span className="text-muted-foreground">Patrimônio líquido: </span>
            {formatCurrency(latestMonthly.patrimonio_liquido)}
          </p>
          <p>
            <span className="text-muted-foreground">Valor patrimonial/cota: </span>
            {formatCurrency(latestMonthly.valor_patrimonial_cota)}
          </p>
          <p>
            <span className="text-muted-foreground">Cotistas: </span>
            {latestMonthly.numero_cotistas ?? "—"}
          </p>
          <p>
            <span className="text-muted-foreground">Dividend yield (mês): </span>
            {formatPct(latestMonthly.dividend_yield_mes)}
          </p>
          <p>
            <span className="text-muted-foreground">Rentabilidade efetiva (mês): </span>
            {formatPct(latestMonthly.rentabilidade_efetiva_mes)}
          </p>
        </div>
      )}

      {properties.length > 0 && (
        <div>
          <p className="mb-1 text-sm text-muted-foreground">
            Imóveis (referência: {properties[0].reference_date})
          </p>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Imóvel</TableHead>
                <TableHead>Endereço</TableHead>
                <TableHead>Área</TableHead>
                <TableHead>Vacância</TableHead>
                <TableHead>Inadimplência</TableHead>
                <TableHead>% Receita</TableHead>
                <TableHead>% Locado</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {properties.map((property) => (
                <TableRow key={property.id}>
                  <TableCell>{property.nome_imovel}</TableCell>
                  <TableCell>{property.endereco ?? "—"}</TableCell>
                  <TableCell>{formatArea(property.area_m2)}</TableCell>
                  <TableCell>{formatPct(property.percentual_vacancia)}</TableCell>
                  <TableCell>{formatPct(property.percentual_inadimplencia)}</TableCell>
                  <TableCell>{formatPct(property.percentual_receitas_fii)}</TableCell>
                  <TableCell>{formatPct(property.percentual_locado)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  );
}

export default FiiCvmDetails;

import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
import {
  ASSET_CLASSES,
  ASSET_CLASS_LABELS,
  type Asset,
  type AssetClass,
  type ExposureType,
} from "./types";
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

type CreateAssetRequest = {
  ticker: string;
  name: string;
  asset_class: string;
  currency: string;
  exchange: string | null;
  exposure_type: string;
  exposure_value: string;
};

function AssetSection() {
  const [ticker, setTicker] = useState("");
  const [name, setName] = useState("");
  const [assetClass, setAssetClass] = useState<AssetClass>("acao_br");
  const [currency, setCurrency] = useState("BRL");
  const [exchange, setExchange] = useState("");
  const [exposureType, setExposureType] = useState<ExposureType>("pais");
  const [exposureValue, setExposureValue] = useState("BR");

  const queryClient = useQueryClient();

  const assetsQuery = useQuery<Asset[], AppError>({
    queryKey: ["assets"],
    queryFn: () => invoke("list_assets"),
  });

  const createMutation = useMutation<Asset, AppError, CreateAssetRequest>({
    mutationFn: (request) => invoke("create_asset", { request }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["assets"] });
      setTicker("");
      setName("");
      setExchange("");
    },
  });

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    createMutation.mutate({
      ticker: ticker.toUpperCase(),
      name,
      asset_class: assetClass,
      currency: currency.toUpperCase(),
      exchange: exchange.trim() === "" ? null : exchange,
      exposure_type: exposureType,
      exposure_value: exposureValue,
    });
  }

  const assets = assetsQuery.data ?? [];

  return (
    <Card>
      <CardHeader>
        <CardTitle>Ativos</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="mb-4 text-sm text-muted-foreground">
          Catálogo de ativos negociáveis/registráveis — compartilhado entre todos os Portfolios do
          Workspace. Escopo desta fatia: Ação (B3), Stocks internacionais, Tesouro Direto e Renda
          Fixa (as classes expandidas na Sessão 30 ficam pra depois).
        </p>
        <form onSubmit={handleSubmit} className="mb-8 flex flex-col gap-4">
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
            <Field label="Ticker / identificador">
              <Input
                required
                placeholder="ex.: PETR4, Tesouro IPCA+ 2035"
                value={ticker}
                onChange={(e) => setTicker(e.currentTarget.value)}
              />
            </Field>
            <Field label="Nome" className="sm:col-span-2">
              <Input
                required
                placeholder="ex.: Petrobras PN"
                value={name}
                onChange={(e) => setName(e.currentTarget.value)}
              />
            </Field>
          </div>

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
            <Field label="Classe do ativo">
              <Select
                value={assetClass}
                onValueChange={(value) => setAssetClass(value as AssetClass)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {ASSET_CLASSES.map((key) => (
                    <SelectItem key={key} value={key}>
                      {ASSET_CLASS_LABELS[key]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field label="Moeda">
              <Input
                required
                placeholder="BRL, USD..."
                value={currency}
                onChange={(e) => setCurrency(e.currentTarget.value)}
              />
            </Field>
            <Field label="Bolsa/listagem (opcional)">
              <Input
                placeholder="ex.: B3, NASDAQ"
                value={exchange}
                onChange={(e) => setExchange(e.currentTarget.value)}
              />
            </Field>
          </div>

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <Field label="Tipo de exposição">
              <Select
                value={exposureType}
                onValueChange={(value) => setExposureType(value as ExposureType)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="pais">País</SelectItem>
                  <SelectItem value="categoria_especial">Categoria especial</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            <Field
              label={
                exposureType === "pais"
                  ? "País de exposição (ex.: BR, US)"
                  : "Categoria (ex.: cripto, metal_ouro)"
              }
            >
              <Input
                required
                value={exposureValue}
                onChange={(e) => setExposureValue(e.currentTarget.value)}
              />
            </Field>
          </div>

          {createMutation.isError && (
            <p className="text-red-600">{createMutation.error.message}</p>
          )}

          <Button type="submit" disabled={createMutation.isPending} className="w-fit">
            {createMutation.isPending ? "Adicionando..." : "Adicionar ativo"}
          </Button>
        </form>

        {assetsQuery.isError && (
          <p className="mb-3 text-red-600">{assetsQuery.error.message}</p>
        )}

        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Ticker</TableHead>
              <TableHead>Nome</TableHead>
              <TableHead>Classe</TableHead>
              <TableHead>Moeda</TableHead>
              <TableHead>Bolsa</TableHead>
              <TableHead>Exposição</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {assets.length === 0 && (
              <TableRow>
                <TableCell colSpan={6} className="text-center text-muted-foreground">
                  Nenhum ativo cadastrado ainda.
                </TableCell>
              </TableRow>
            )}
            {assets.map((asset) => (
              <TableRow key={asset.id}>
                <TableCell>{asset.ticker}</TableCell>
                <TableCell>{asset.name}</TableCell>
                <TableCell>
                  {ASSET_CLASS_LABELS[asset.asset_class as AssetClass] ?? asset.asset_class}
                </TableCell>
                <TableCell>{asset.currency}</TableCell>
                <TableCell>{asset.exchange ?? "—"}</TableCell>
                <TableCell>
                  {asset.exposure_type === "pais" ? "🌍 " : "🏷️ "}
                  {asset.exposure_value}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}

export default AssetSection;

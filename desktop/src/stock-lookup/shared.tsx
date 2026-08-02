import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
import type { Asset, AssetClass } from "../portfolio/types";
import { Button } from "@/components/ui/button";

// Fase 10, item 8, Sessão 43 — extraído da antiga `StockLookupPanel.tsx`
// monolítica (só Ação BR) pra ser reaproveitado também por
// `FiiLookupSection.tsx`, já que a tela de Pesquisa virou unificada (um
// seletor de classe, não uma aba por classe — pedido explícito do dono do
// projeto: "queria deixar o app completo mas enxuto").

export function formatCurrency(value: number | null | undefined): string {
  return value == null ? "—" : `R$ ${value.toFixed(2)}`;
}

export function formatPercent(value: number | null | undefined): string {
  return value == null ? "—" : `${value.toFixed(2)}%`;
}

// A CVM devolve percentuais como fração (0-1, ex.: 0.0323 = 3,23%) —
// convenção diferente de `formatPercent` acima (Yahoo/CVM-DFP já devolvem
// em escala de %, ex.: `cagr_5y: 12.34`). Distinguir os dois evita o erro
// clássico de multiplicar/dividir por 100 no lugar errado.
export function formatFractionAsPercent(value: number | null | undefined): string {
  return value == null ? "—" : `${(value * 100).toFixed(2)}%`;
}

export function formatRatio(value: number | null): string {
  return value == null ? "—" : value.toFixed(2);
}

// Free public icon CDN (`icons.brapi.dev`, part of the brapi.dev project,
// CORS-enabled) keyed by ticker — no API call needed, just a predictable
// image URL. 404s for tickers without a logo, so `onError` hides it instead
// of showing a broken-image icon. Works for FII tickers too (same B3
// listing), confirmed the CDN doesn't distinguish by asset class.
export function CompanyLogo({ ticker }: { ticker: string }) {
  const [failed, setFailed] = useState(false);
  if (failed) return null;
  return (
    <img
      src={`https://icons.brapi.dev/icons/${ticker}.svg`}
      alt=""
      className="h-10 w-10 rounded-full"
      onError={() => setFailed(true)}
    />
  );
}

// Same "label / value" tile shape as CryptoScorePanel's IndicatorTile, reused
// here for quote/technicals/fundamentals/DCF/CVM stats instead of signals.
export function StatTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <p className="text-sm text-muted-foreground">{label}</p>
      <p className="mt-1 text-2xl font-semibold">{value}</p>
    </div>
  );
}

export type StockNote = {
  id: number;
  ticker: string;
  note: string;
  updated_at: string;
};

type CreateAssetRequest = {
  ticker: string;
  name: string;
  asset_class: string;
  currency: string;
  exchange: string | null;
  exposure_type: string;
  exposure_value: string;
  cnpj: string | null;
};

/// Fase 10, item 8, Sessão 45 — "quero que dê pra adicionar aos ativos pela
/// tela de análise" (pedido explícito do dono do projeto). Registra no
/// catálogo global de `assets` (não escopado por Workspace/Portfolio — não
/// precisa saber em qual Workspace o usuário está) com os defaults já
/// estabelecidos em `AssetSection.tsx` (exposição `pais`/`BR`, mesma
/// convenção de qualquer ativo listado na B3). Consulta `["assets"]`
/// primeiro pra não oferecer cadastrar de novo um ticker já cadastrado —
/// essa é a mesma cache key que `AssetSection.tsx` usa, então cadastrar por
/// aqui atualiza a tabela de Ativos também sem precisar de nada especial.
export function AddToAssetsButton({
  ticker,
  assetClass,
  name,
  currency,
  exchange,
  cnpj,
}: {
  ticker: string;
  assetClass: AssetClass;
  name: string;
  currency: string;
  exchange: string | null;
  cnpj: string | null;
}) {
  const queryClient = useQueryClient();

  const assetsQuery = useQuery<Asset[], AppError>({
    queryKey: ["assets"],
    queryFn: () => invoke("list_assets"),
  });
  const alreadyRegistered = (assetsQuery.data ?? []).some((a) => a.ticker === ticker);

  const createMutation = useMutation<Asset, AppError, void>({
    mutationFn: () =>
      invoke("create_asset", {
        request: {
          ticker,
          name,
          asset_class: assetClass,
          currency,
          exchange,
          exposure_type: "pais",
          exposure_value: "BR",
          cnpj,
        } satisfies CreateAssetRequest,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["assets"] });
    },
  });

  if (assetsQuery.isLoading) return null;

  if (alreadyRegistered) {
    return <p className="text-sm text-muted-foreground">✓ Já cadastrado em Ativos</p>;
  }

  return (
    <div className="flex items-center gap-2">
      <Button
        type="button"
        variant="outline"
        size="sm"
        disabled={createMutation.isPending}
        onClick={() => createMutation.mutate()}
      >
        {createMutation.isPending ? "Adicionando..." : "Adicionar aos Ativos"}
      </Button>
      {createMutation.isError && (
        <p className="text-sm text-red-600">{createMutation.error.message}</p>
      )}
    </div>
  );
}

import { Fragment, useEffect, useRef, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
import {
  ASSET_CLASSES,
  ASSET_CLASSES_WITH_AUTO_QUOTE,
  ASSET_CLASS_LABELS,
  type Asset,
  type AssetClass,
  type AssetFavorite,
  type ExposureType,
  type FiiCnpjSuggestion,
} from "./types";
import { latestForTicker } from "../collector/latestForTicker";
import type { StockQuote } from "../collector/types";
import FiiCvmDetails from "./FiiCvmDetails";
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
  cnpj: string | null;
};

type CollectorSummary = { success: boolean; output: string };
type ToggleFavoriteRequest = { workspace_id: number; asset_id: number };

function AssetSection({ workspaceId }: { workspaceId: number }) {
  const [ticker, setTicker] = useState("");
  const [name, setName] = useState("");
  const [assetClass, setAssetClass] = useState<AssetClass>("acao_br");
  const [currency, setCurrency] = useState("BRL");
  const [exchange, setExchange] = useState("");
  const [exposureType, setExposureType] = useState<ExposureType>("pais");
  const [exposureValue, setExposureValue] = useState("BR");
  const [cnpj, setCnpj] = useState("");
  const [expandedAssetId, setExpandedAssetId] = useState<number | null>(null);

  const [tickerQuery, setTickerQuery] = useState("");
  const [activeTicker, setActiveTicker] = useState<string | null>(null);
  const autoFetchedTickerRef = useRef<string | null>(null);
  const prefilledTickerRef = useRef<string | null>(null);
  const cnpjResolvedTickerRef = useRef<string | null>(null);

  const isAutoQuoteClass = (ASSET_CLASSES_WITH_AUTO_QUOTE as string[]).includes(assetClass);
  const isFii = assetClass === "fii";

  const queryClient = useQueryClient();

  const assetsQuery = useQuery<Asset[], AppError>({
    queryKey: ["assets"],
    queryFn: () => invoke("list_assets"),
  });

  // Fase 10.4 — favorito rápido (estrela), separado das watchlists nomeadas
  // (aba própria "Watchlists"): mesma query cacheada sob a chave
  // ["asset-favorites", workspaceId] usada lá, pra ficar em sincronia.
  const favoritesQuery = useQuery<AssetFavorite[], AppError>({
    queryKey: ["asset-favorites", workspaceId],
    queryFn: () => invoke("list_favorite_assets", { workspaceId }),
  });
  const favoriteAssetIds = new Set((favoritesQuery.data ?? []).map((f) => f.asset_id));

  const toggleFavoriteMutation = useMutation<boolean, AppError, number>({
    mutationFn: (assetId) =>
      invoke("toggle_favorite", {
        request: { workspace_id: workspaceId, asset_id: assetId } satisfies ToggleFavoriteRequest,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["asset-favorites", workspaceId] });
    },
  });

  // Só busca cotação pras classes de ASSET_CLASSES_WITH_AUTO_QUOTE (Ação
  // BR/FII, mesmo endpoint Yahoo) — as outras ainda não têm coletor
  // integrado (ver Fase 10, item 8), então continuam com cadastro manual.
  const lookupQuery = useQuery<StockQuote | null, AppError>({
    queryKey: ["asset-section-stock-quote", activeTicker],
    enabled: isAutoQuoteClass && activeTicker !== null,
    queryFn: async () => {
      const quotes = await invoke<StockQuote[]>("list_stock_quotes");
      return latestForTicker(quotes, activeTicker as string);
    },
  });

  const collectorMutation = useMutation<CollectorSummary, AppError, string>({
    mutationFn: (t) => invoke<CollectorSummary>("run_stock_collector", { ticker: t }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["asset-section-stock-quote", activeTicker] });
    },
  });

  // Fase 10, item 8, Sessão 41 — só pra FII: tenta sugerir o CNPJ do fundo
  // assim que a cotação chega (mesmo gatilho do prefill abaixo). A bolsai
  // só é chamada aqui, uma vez por busca — depois de confirmado e salvo,
  // toda leitura recorrente (FiiCvmDetails) usa só a CVM.
  const resolveCnpjMutation = useMutation<FiiCnpjSuggestion | null, AppError, string>({
    mutationFn: (t) => invoke("resolve_fii_cnpj", { ticker: t }),
  });

  // Mesmo padrão do Stock Lookup (Fase 9): um ticker sem dado no banco
  // dispara o coletor no máximo uma vez por busca, guardado pelo ref.
  useEffect(() => {
    if (
      isAutoQuoteClass &&
      activeTicker &&
      lookupQuery.isSuccess &&
      lookupQuery.data === null &&
      autoFetchedTickerRef.current !== activeTicker
    ) {
      autoFetchedTickerRef.current = activeTicker;
      collectorMutation.mutate(activeTicker);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAutoQuoteClass, activeTicker, lookupQuery.isSuccess, lookupQuery.data]);

  // Pré-preenche o preview editável assim que a cotação chega — só na
  // primeira vez por ticker, pra não sobrescrever edições manuais do
  // usuário em buscas repetidas (ex: "Refresh" implícito de reabrir a tela).
  useEffect(() => {
    const quote = lookupQuery.data;
    if (isAutoQuoteClass && activeTicker && quote && prefilledTickerRef.current !== activeTicker) {
      prefilledTickerRef.current = activeTicker;
      setTicker(activeTicker);
      setName(quote.name ?? "");
      setCurrency(quote.currency ?? "BRL");
      setExchange(quote.exchange ?? "");
      setExposureType("pais");
      setExposureValue("BR");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAutoQuoteClass, activeTicker, lookupQuery.data]);

  // Sugestão de CNPJ (só FII) — dispara junto com o prefill acima, uma vez
  // por ticker. `cnpj` fica vazio se não achar sugestão confiável (o
  // usuário cola manualmente, ou deixa em branco e resolve depois pelo
  // painel de detalhes CVM via `update_asset_cnpj`).
  useEffect(() => {
    const quote = lookupQuery.data;
    if (isFii && activeTicker && quote && cnpjResolvedTickerRef.current !== activeTicker) {
      cnpjResolvedTickerRef.current = activeTicker;
      resolveCnpjMutation.mutate(activeTicker, {
        onSuccess: (suggestion) => setCnpj(suggestion?.cnpj ?? ""),
      });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isFii, activeTicker, lookupQuery.data]);

  // Trocar de classe pra fora da busca automática limpa a busca em
  // andamento, pra não deixar um preview de ticker antigo aparecendo se
  // voltar depois.
  useEffect(() => {
    if (!isAutoQuoteClass) {
      setActiveTicker(null);
      setTickerQuery("");
      autoFetchedTickerRef.current = null;
      prefilledTickerRef.current = null;
    }
    if (!isFii) {
      setCnpj("");
      cnpjResolvedTickerRef.current = null;
    }
  }, [isAutoQuoteClass, isFii]);

  const createMutation = useMutation<Asset, AppError, CreateAssetRequest>({
    mutationFn: (request) => invoke("create_asset", { request }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["assets"] });
      setTicker("");
      setName("");
      setExchange("");
      setCnpj("");
      setTickerQuery("");
      setActiveTicker(null);
      autoFetchedTickerRef.current = null;
      prefilledTickerRef.current = null;
      cnpjResolvedTickerRef.current = null;
    },
  });

  function handleTickerSearch(event: FormEvent) {
    event.preventDefault();
    const normalized = tickerQuery.trim().toUpperCase();
    if (!normalized) return;
    setActiveTicker(normalized);
  }

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
      cnpj: isFii && cnpj.trim() !== "" ? cnpj.trim() : null,
    });
  }

  const assets = assetsQuery.data ?? [];
  const showCreateForm = !isAutoQuoteClass || (activeTicker !== null && lookupQuery.data != null);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Assets</CardTitle>
      </CardHeader>
      <CardContent>
        <p className="mb-4 text-sm text-muted-foreground">
          Catalog of tradeable/registrable assets — shared across every Portfolio in the
          Workspace. Stock (B3), FII (B3) and ETF (B3) fetch data automatically by ticker; the
          other classes (International stocks, Tesouro Direto, Fixed income) still use manual
          registration for now.
        </p>

        <div className="mb-4 max-w-xs">
          <Field label="Asset class">
            <Select value={assetClass} onValueChange={(value) => setAssetClass(value as AssetClass)}>
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
        </div>

        {isAutoQuoteClass && (
          <form onSubmit={handleTickerSearch} className="mb-4 flex items-end gap-3">
            <Field label="Search ticker (B3)" className="flex-1">
              <Input
                required
                placeholder={
                  assetClass === "fii" ? "HGLG11" : assetClass === "etf_br" ? "BOVA11" : "PETR4"
                }
                value={tickerQuery}
                onChange={(e) => setTickerQuery(e.currentTarget.value)}
              />
            </Field>
            <Button type="submit">Search</Button>
          </form>
        )}

        {isAutoQuoteClass && lookupQuery.isError && (
          <p className="mb-3 text-red-600">{lookupQuery.error.message}</p>
        )}
        {isAutoQuoteClass && collectorMutation.isError && (
          <p className="mb-3 text-red-600">{collectorMutation.error.message}</p>
        )}
        {isAutoQuoteClass && activeTicker && lookupQuery.isLoading && (
          <p className="mb-3 text-muted-foreground">Loading {activeTicker}...</p>
        )}
        {isAutoQuoteClass && activeTicker && lookupQuery.data === null && collectorMutation.isPending && (
          <p className="mb-3 text-muted-foreground">Fetching {activeTicker} for the first time...</p>
        )}
        {isAutoQuoteClass &&
          activeTicker &&
          lookupQuery.isSuccess &&
          lookupQuery.data === null &&
          !collectorMutation.isPending && (
            <p className="mb-3 text-muted-foreground">No data found for {activeTicker}.</p>
          )}

        {showCreateForm && (
          <form onSubmit={handleSubmit} className="mb-8 flex flex-col gap-4">
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
              <Field label="Ticker / identifier">
                <Input
                  required
                  disabled={isAutoQuoteClass}
                  placeholder="e.g.: PETR4, Tesouro IPCA+ 2035"
                  value={ticker}
                  onChange={(e) => setTicker(e.currentTarget.value)}
                />
              </Field>
              <Field label="Name" className="sm:col-span-2">
                <Input
                  required
                  placeholder="e.g.: Petrobras PN"
                  value={name}
                  onChange={(e) => setName(e.currentTarget.value)}
                />
              </Field>
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label="Currency">
                <Input
                  required
                  placeholder="BRL, USD..."
                  value={currency}
                  onChange={(e) => setCurrency(e.currentTarget.value)}
                />
              </Field>
              <Field label="Exchange/listing (optional)">
                <Input
                  placeholder="e.g.: B3, NASDAQ"
                  value={exchange}
                  onChange={(e) => setExchange(e.currentTarget.value)}
                />
              </Field>
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label="Exposure type">
                <Select
                  value={exposureType}
                  onValueChange={(value) => setExposureType(value as ExposureType)}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="pais">Country</SelectItem>
                    <SelectItem value="categoria_especial">Special category</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              <Field
                label={
                  exposureType === "pais"
                    ? "Exposure country (e.g.: BR, US)"
                    : "Category (e.g.: crypto, gold_metal)"
                }
              >
                <Input
                  required
                  value={exposureValue}
                  onChange={(e) => setExposureValue(e.currentTarget.value)}
                />
              </Field>
            </div>

            {isFii && (
              <Field label="Fund CNPJ (to pull indicators from CVM — vacancy, delinquency, net assets)">
                <Input
                  placeholder="00.000.000/0001-00"
                  value={cnpj}
                  onChange={(e) => setCnpj(e.currentTarget.value)}
                />
                {resolveCnpjMutation.isPending && (
                  <p className="mt-1 text-sm text-muted-foreground">Looking up CNPJ...</p>
                )}
                {resolveCnpjMutation.isSuccess && resolveCnpjMutation.data === null && (
                  <p className="mt-1 text-sm text-muted-foreground">
                    Couldn't find the CNPJ automatically — paste it manually (fund's website or
                    CVM) or leave it blank and resolve it later from the CVM details panel.
                  </p>
                )}
              </Field>
            )}

            {createMutation.isError && (
              <p className="text-red-600">{createMutation.error.message}</p>
            )}

            <Button type="submit" disabled={createMutation.isPending} className="w-fit">
              {createMutation.isPending ? "Adding..." : "Add asset"}
            </Button>
          </form>
        )}

        {assetsQuery.isError && (
          <p className="mb-3 text-red-600">{assetsQuery.error.message}</p>
        )}

        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Ticker</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Class</TableHead>
              <TableHead>Currency</TableHead>
              <TableHead>Exchange</TableHead>
              <TableHead>Exposure</TableHead>
              <TableHead>★</TableHead>
              <TableHead></TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {assets.length === 0 && (
              <TableRow>
                <TableCell colSpan={8} className="text-center text-muted-foreground">
                  No assets registered yet.
                </TableCell>
              </TableRow>
            )}
            {assets.map((asset) => {
              const isFavorite = favoriteAssetIds.has(asset.id);
              const isExpanded = expandedAssetId === asset.id;
              return (
                <Fragment key={asset.id}>
                  <TableRow>
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
                    <TableCell>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        disabled={toggleFavoriteMutation.isPending}
                        onClick={() => toggleFavoriteMutation.mutate(asset.id)}
                        aria-label={isFavorite ? "Remove from favorites" : "Add to favorites"}
                      >
                        {isFavorite ? "★" : "☆"}
                      </Button>
                    </TableCell>
                    <TableCell>
                      {asset.asset_class === "fii" && (
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          onClick={() => setExpandedAssetId(isExpanded ? null : asset.id)}
                        >
                          {isExpanded ? "Hide CVM" : "CVM data"}
                        </Button>
                      )}
                    </TableCell>
                  </TableRow>
                  {isExpanded && (
                    <TableRow>
                      <TableCell colSpan={8} className="bg-muted/30">
                        <FiiCvmDetails asset={asset} />
                      </TableCell>
                    </TableRow>
                  )}
                </Fragment>
              );
            })}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}

export default AssetSection;

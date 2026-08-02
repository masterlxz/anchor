import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AppError } from "../types";
import type { Asset, AssetFavorite, Watchlist, WatchlistItem } from "./types";
import type { StockQuote } from "../collector/types";
import { latestForTicker } from "../collector/latestForTicker";
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

type CreateWatchlistRequest = { workspace_id: number; name: string };
type AddWatchlistItemRequest = {
  watchlist_id: number;
  asset_id: number;
  target_price: number | null;
  notes: string | null;
};
type UpdateWatchlistItemRequest = {
  item_id: number;
  target_price: number | null;
  notes: string | null;
};
type ToggleFavoriteRequest = { workspace_id: number; asset_id: number };

// Fase 10.4 — dois mecanismos separados por pedido explícito do dono do
// projeto: watchlists nomeadas (preço-alvo/notas) e favoritos rápidos
// (estrela, sem lista/anotação nenhuma). O preço atual em ambos os blocos é
// derivado client-side de `list_stock_quotes` + `latestForTicker`, mesmo
// padrão já usado em `AssetSection.tsx` — sem lógica de comparação/veredito
// nesta fatia, só exibição lado a lado.
function WatchlistSection({ workspaceId }: { workspaceId: number }) {
  const [selectedWatchlistId, setSelectedWatchlistId] = useState<number | null>(null);
  const [newWatchlistName, setNewWatchlistName] = useState("");
  const [confirmingDeleteWatchlist, setConfirmingDeleteWatchlist] = useState(false);

  const [newItemAssetId, setNewItemAssetId] = useState("");
  const [newItemTargetPrice, setNewItemTargetPrice] = useState("");
  const [newItemNotes, setNewItemNotes] = useState("");

  const [editingItemId, setEditingItemId] = useState<number | null>(null);
  const [editTargetPrice, setEditTargetPrice] = useState("");
  const [editNotes, setEditNotes] = useState("");
  const [confirmingDeleteItemId, setConfirmingDeleteItemId] = useState<number | null>(null);

  const queryClient = useQueryClient();

  const assetsQuery = useQuery<Asset[], AppError>({
    queryKey: ["assets"],
    queryFn: () => invoke("list_assets"),
  });
  const assets = assetsQuery.data ?? [];

  const quotesQuery = useQuery<StockQuote[], AppError>({
    queryKey: ["stock-quotes"],
    queryFn: () => invoke("list_stock_quotes"),
  });
  const quotes = quotesQuery.data ?? [];

  const favoritesQuery = useQuery<AssetFavorite[], AppError>({
    queryKey: ["asset-favorites", workspaceId],
    queryFn: () => invoke("list_favorite_assets", { workspaceId }),
  });
  const favorites = favoritesQuery.data ?? [];

  const toggleFavoriteMutation = useMutation<boolean, AppError, number>({
    mutationFn: (assetId) =>
      invoke("toggle_favorite", {
        request: { workspace_id: workspaceId, asset_id: assetId } satisfies ToggleFavoriteRequest,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["asset-favorites", workspaceId] });
    },
  });

  const watchlistsQuery = useQuery<Watchlist[], AppError>({
    queryKey: ["watchlists", workspaceId],
    queryFn: () => invoke("list_watchlists", { workspaceId }),
  });
  const watchlists = watchlistsQuery.data ?? [];

  const createWatchlistMutation = useMutation<Watchlist, AppError, CreateWatchlistRequest>({
    mutationFn: (request) => invoke("create_watchlist", { request }),
    onSuccess: (created) => {
      queryClient.invalidateQueries({ queryKey: ["watchlists", workspaceId] });
      setSelectedWatchlistId(created.id);
      setNewWatchlistName("");
    },
  });

  const deleteWatchlistMutation = useMutation<void, AppError, number>({
    mutationFn: (watchlistId) => invoke("delete_watchlist", { watchlistId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["watchlists", workspaceId] });
      setSelectedWatchlistId(null);
      setConfirmingDeleteWatchlist(false);
    },
  });

  const itemsQuery = useQuery<WatchlistItem[], AppError>({
    queryKey: ["watchlist-items", selectedWatchlistId],
    queryFn: () => invoke("list_watchlist_items", { watchlistId: selectedWatchlistId }),
    enabled: selectedWatchlistId !== null,
  });
  const items = itemsQuery.data ?? [];

  const addItemMutation = useMutation<WatchlistItem, AppError, AddWatchlistItemRequest>({
    mutationFn: (request) => invoke("add_watchlist_item", { request }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["watchlist-items", selectedWatchlistId] });
      setNewItemAssetId("");
      setNewItemTargetPrice("");
      setNewItemNotes("");
    },
  });

  const updateItemMutation = useMutation<WatchlistItem, AppError, UpdateWatchlistItemRequest>({
    mutationFn: (request) => invoke("update_watchlist_item", { request }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["watchlist-items", selectedWatchlistId] });
      setEditingItemId(null);
    },
  });

  const removeItemMutation = useMutation<void, AppError, number>({
    mutationFn: (itemId) => invoke("remove_watchlist_item", { itemId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["watchlist-items", selectedWatchlistId] });
      setConfirmingDeleteItemId(null);
    },
  });

  function assetFor(assetId: number): Asset | undefined {
    return assets.find((a) => a.id === assetId);
  }

  function currentPriceFor(assetId: number): number | null {
    const asset = assetFor(assetId);
    if (!asset) return null;
    return latestForTicker(quotes, asset.ticker)?.price ?? null;
  }

  function handleCreateWatchlist(event: FormEvent) {
    event.preventDefault();
    createWatchlistMutation.mutate({ workspace_id: workspaceId, name: newWatchlistName });
  }

  function handleDeleteWatchlistClick() {
    if (selectedWatchlistId === null) return;
    if (confirmingDeleteWatchlist) {
      deleteWatchlistMutation.mutate(selectedWatchlistId);
    } else {
      setConfirmingDeleteWatchlist(true);
    }
  }

  function handleAddItem(event: FormEvent) {
    event.preventDefault();
    if (selectedWatchlistId === null || !newItemAssetId) return;
    addItemMutation.mutate({
      watchlist_id: selectedWatchlistId,
      asset_id: Number(newItemAssetId),
      target_price: newItemTargetPrice.trim() === "" ? null : Number(newItemTargetPrice),
      notes: newItemNotes.trim() === "" ? null : newItemNotes,
    });
  }

  function startEditing(item: WatchlistItem) {
    setEditingItemId(item.id);
    setEditTargetPrice(item.target_price !== null ? String(item.target_price) : "");
    setEditNotes(item.notes ?? "");
  }

  function handleSaveEdit() {
    if (editingItemId === null) return;
    updateItemMutation.mutate({
      item_id: editingItemId,
      target_price: editTargetPrice.trim() === "" ? null : Number(editTargetPrice),
      notes: editNotes.trim() === "" ? null : editNotes,
    });
  }

  function handleDeleteItemClick(itemId: number) {
    if (confirmingDeleteItemId === itemId) {
      removeItemMutation.mutate(itemId);
    } else {
      setConfirmingDeleteItemId(itemId);
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <Card>
        <CardHeader>
          <CardTitle>★ Favorites</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="mb-4 text-sm text-muted-foreground">
            Quick asset marking, separate from the named watchlists below — favorite/unfavorite
            via the star on the "Assets" tab.
          </p>
          {favoritesQuery.isError && (
            <p className="mb-3 text-red-600">{favoritesQuery.error.message}</p>
          )}
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Ticker</TableHead>
                <TableHead>Name</TableHead>
                <TableHead>Current price</TableHead>
                <TableHead>Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {favorites.length === 0 && (
                <TableRow>
                  <TableCell colSpan={4} className="text-center text-muted-foreground">
                    No favorited assets yet.
                  </TableCell>
                </TableRow>
              )}
              {favorites.map((favorite) => {
                const asset = assetFor(favorite.asset_id);
                const price = currentPriceFor(favorite.asset_id);
                return (
                  <TableRow key={favorite.id}>
                    <TableCell>{asset?.ticker ?? `#${favorite.asset_id}`}</TableCell>
                    <TableCell>{asset?.name ?? "—"}</TableCell>
                    <TableCell>{price !== null ? price.toFixed(2) : "—"}</TableCell>
                    <TableCell>
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={toggleFavoriteMutation.isPending}
                        onClick={() => toggleFavoriteMutation.mutate(favorite.asset_id)}
                      >
                        ★ Remove
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Watchlists</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="mb-4 text-sm text-muted-foreground">
            Named lists of assets with target price and notes (e.g.: "Dividend Stocks",
            "Turnarounds to Watch").
          </p>

          <div className="mb-4 flex items-end gap-4">
            <Field label="Watchlist" className="flex-1">
              <Select
                value={selectedWatchlistId !== null ? String(selectedWatchlistId) : ""}
                onValueChange={(value) => {
                  setSelectedWatchlistId(Number(value));
                  setConfirmingDeleteWatchlist(false);
                }}
              >
                <SelectTrigger className="w-full">
                  <SelectValue placeholder="Select a watchlist" />
                </SelectTrigger>
                <SelectContent>
                  {watchlists.map((watchlist) => (
                    <SelectItem key={watchlist.id} value={String(watchlist.id)}>
                      {watchlist.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Button
              type="button"
              variant={confirmingDeleteWatchlist ? "destructive" : "outline"}
              disabled={selectedWatchlistId === null}
              onClick={handleDeleteWatchlistClick}
            >
              {confirmingDeleteWatchlist ? "Confirm?" : "Delete"}
            </Button>
          </div>

          <form onSubmit={handleCreateWatchlist} className="mb-8 flex items-end gap-4">
            <Field label="New watchlist" className="flex-1">
              <Input
                required
                placeholder="e.g.: Dividend Stocks"
                value={newWatchlistName}
                onChange={(e) => setNewWatchlistName(e.currentTarget.value)}
              />
            </Field>
            <Button type="submit" disabled={createWatchlistMutation.isPending}>
              {createWatchlistMutation.isPending ? "Creating..." : "Create watchlist"}
            </Button>
          </form>

          {watchlistsQuery.isError && (
            <p className="mb-3 text-red-600">{watchlistsQuery.error.message}</p>
          )}

          {selectedWatchlistId !== null && (
            <>
              <form
                onSubmit={handleAddItem}
                className="mb-6 grid grid-cols-1 gap-4 sm:grid-cols-4"
              >
                <Field label="Asset" className="sm:col-span-2">
                  <Select value={newItemAssetId} onValueChange={setNewItemAssetId}>
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="Select an asset" />
                    </SelectTrigger>
                    <SelectContent>
                      {assets.map((asset) => (
                        <SelectItem key={asset.id} value={String(asset.id)}>
                          {asset.ticker} — {asset.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
                <Field label="Target price (optional)">
                  <Input
                    type="number"
                    step="any"
                    value={newItemTargetPrice}
                    onChange={(e) => setNewItemTargetPrice(e.currentTarget.value)}
                  />
                </Field>
                <Field label="Notes (optional)">
                  <Input value={newItemNotes} onChange={(e) => setNewItemNotes(e.currentTarget.value)} />
                </Field>
                <div className="sm:col-span-4">
                  <Button type="submit" disabled={addItemMutation.isPending || !newItemAssetId}>
                    {addItemMutation.isPending ? "Adding..." : "Add item"}
                  </Button>
                </div>
              </form>

              {addItemMutation.isError && (
                <p className="mb-3 text-red-600">{addItemMutation.error.message}</p>
              )}
              {itemsQuery.isError && <p className="mb-3 text-red-600">{itemsQuery.error.message}</p>}

              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Asset</TableHead>
                    <TableHead>Target price</TableHead>
                    <TableHead>Current price</TableHead>
                    <TableHead>Notes</TableHead>
                    <TableHead>Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {items.length === 0 && (
                    <TableRow>
                      <TableCell colSpan={5} className="text-center text-muted-foreground">
                        No items in this watchlist yet.
                      </TableCell>
                    </TableRow>
                  )}
                  {items.map((item) => {
                    const isEditing = editingItemId === item.id;
                    const isConfirmingDelete = confirmingDeleteItemId === item.id;
                    const price = currentPriceFor(item.asset_id);
                    return (
                      <TableRow key={item.id}>
                        <TableCell>{assetFor(item.asset_id)?.ticker ?? `#${item.asset_id}`} — {assetFor(item.asset_id)?.name ?? ""}</TableCell>
                        {isEditing ? (
                          <>
                            <TableCell>
                              <Input
                                type="number"
                                step="any"
                                value={editTargetPrice}
                                onChange={(e) => setEditTargetPrice(e.currentTarget.value)}
                              />
                            </TableCell>
                            <TableCell>{price !== null ? price.toFixed(2) : "—"}</TableCell>
                            <TableCell>
                              <Input value={editNotes} onChange={(e) => setEditNotes(e.currentTarget.value)} />
                            </TableCell>
                            <TableCell className="flex gap-2">
                              <Button
                                size="sm"
                                onClick={handleSaveEdit}
                                disabled={updateItemMutation.isPending}
                              >
                                Save
                              </Button>
                              <Button size="sm" variant="outline" onClick={() => setEditingItemId(null)}>
                                Cancel
                              </Button>
                            </TableCell>
                          </>
                        ) : (
                          <>
                            <TableCell>
                              {item.target_price !== null ? item.target_price.toFixed(2) : "—"}
                            </TableCell>
                            <TableCell>{price !== null ? price.toFixed(2) : "—"}</TableCell>
                            <TableCell>{item.notes ?? "—"}</TableCell>
                            <TableCell className="flex gap-2">
                              <Button size="sm" variant="outline" onClick={() => startEditing(item)}>
                                Edit
                              </Button>
                              <Button
                                size="sm"
                                variant={isConfirmingDelete ? "destructive" : "outline"}
                                onClick={() => handleDeleteItemClick(item.id)}
                              >
                                {isConfirmingDelete ? "Confirm?" : "Delete"}
                              </Button>
                            </TableCell>
                          </>
                        )}
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default WatchlistSection;

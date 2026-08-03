import { useState, type FormEvent } from "react";
import {
  ASSET_CLASSES_WITH_AUTO_QUOTE,
  ASSET_CLASS_LABELS,
  type AssetClass,
} from "../portfolio/types";
import Field from "../components/Field";
import StockAnalysisSection from "./StockAnalysisSection";
import FiiLookupSection from "./FiiLookupSection";
import EtfLookupSection from "./EtfLookupSection";
import CryptoLookupSection from "./CryptoLookupSection";
import BdrLookupSection from "./BdrLookupSection";
import MetalLookupSection from "./MetalLookupSection";
import UsStockLookupSection from "./UsStockLookupSection";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

/// Fase 10, item 8, Sessão 43 — "Pesquisa" virou a tela unificada de
/// análise pra qualquer classe de ativo com dado automático, em vez de uma
/// aba por classe. Pedido explícito do dono do projeto depois de eu sugerir
/// telas separadas: "queria deixar o app completo mas enxuto, pq se toda
/// feature for adicionar uma aba nova...". Também rejeitou a ideia de
/// detectar a classe tentando um fetch atrás do outro — prefere escolher a
/// classe explicitamente antes de buscar. Este componente é só a moldura
/// (busca + seletor de classe); todo o conteúdo específico de cada classe
/// mora em `StockAnalysisSection.tsx`/`FiiLookupSection.tsx`/
/// `EtfLookupSection.tsx` (este último desde a Sessão 50, sem fundamentos/
/// DCF/valuation — nenhum desses conceitos existe pra um fundo de índice)/
/// `CryptoLookupSection.tsx` (Sessão 51 — absorveu a antiga aba solta
/// "Crypto Score", removida do nav principal)/`BdrLookupSection.tsx`
/// (Sessão 53 — irmã quase idêntica de `EtfLookupSection.tsx`, mesma
/// ausência de fundamentos/DCF, bolsai não tem pra BDR)/`MetalLookupSection.tsx`
/// (Sessão 55 — só ouro por ora, fonte Yahoo sem `.SA`, sem
/// fundamentos/dividendos, que não fazem sentido pra metal)/
/// `UsStockLookupSection.tsx` (Fatia 1 — ação americana, fonte Yahoo sem
/// `.SA`, sem fundamentos/DCF nesta fatia — SEC EDGAR fica pra depois), que
/// só recebem o ticker já commitado — cada uma cuida do próprio fetch/estado,
/// sem nada compartilhado entre elas além do visual (`shared.tsx`).
function StockLookupPanel() {
  const [tickerInput, setTickerInput] = useState("");
  const [activeTicker, setActiveTicker] = useState<string | null>(null);
  const [assetClass, setAssetClass] = useState<AssetClass>("acao_br");

  function handleSearch(event: FormEvent) {
    event.preventDefault();
    const normalized = tickerInput.trim().toUpperCase();
    if (!normalized) return;
    setActiveTicker(normalized);
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Research</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-6">
        <form onSubmit={handleSearch} className="flex items-end gap-3">
          <div className="w-40">
            <Field label="Class">
              <Select
                value={assetClass}
                onValueChange={(value) => setAssetClass(value as AssetClass)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {ASSET_CLASSES_WITH_AUTO_QUOTE.map((key) => (
                    <SelectItem key={key} value={key}>
                      {ASSET_CLASS_LABELS[key]}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
          </div>
          <Field label="Ticker" className="flex-1">
            <Input
              required
              value={tickerInput}
              onChange={(e) => setTickerInput(e.currentTarget.value)}
              placeholder={
                assetClass === "fii"
                  ? "HGLG11"
                  : assetClass === "etf_br"
                    ? "BOVA11"
                    : assetClass === "cripto"
                      ? "ETH"
                      : assetClass === "bdr"
                        ? "AAPL34"
                        : assetClass === "metal"
                          ? "XAU"
                          : assetClass === "acao_internacional"
                            ? "AAPL"
                            : "PETR4"
              }
            />
          </Field>
          <Button type="submit">Search</Button>
        </form>

        {activeTicker &&
          (assetClass === "fii" ? (
            <FiiLookupSection key={activeTicker} ticker={activeTicker} />
          ) : assetClass === "etf_br" ? (
            <EtfLookupSection key={activeTicker} ticker={activeTicker} />
          ) : assetClass === "cripto" ? (
            <CryptoLookupSection key={activeTicker} ticker={activeTicker} />
          ) : assetClass === "bdr" ? (
            <BdrLookupSection key={activeTicker} ticker={activeTicker} />
          ) : assetClass === "metal" ? (
            <MetalLookupSection key={activeTicker} ticker={activeTicker} />
          ) : assetClass === "acao_internacional" ? (
            <UsStockLookupSection key={activeTicker} ticker={activeTicker} />
          ) : (
            <StockAnalysisSection key={activeTicker} ticker={activeTicker} />
          ))}
      </CardContent>
    </Card>
  );
}

export default StockLookupPanel;

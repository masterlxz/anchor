import { useState, type FormEvent } from "react";
import { RefreshCw } from "lucide-react";
import { useTickerCollector } from "../collector/useTickerCollector";
import { useValuationActions } from "./useValuationActions";
import ValuationResult from "../components/ValuationResult";
import Field from "../components/Field";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

type CalculateGrahamRequest = {
  ticker: string;
  reference_year: number;
  current_price: number;
  eps: number;
  book_value_per_share: number;
};

type GrahamInputsModel = {
  id: number;
  valuation_id: number;
  eps: number;
  book_value_per_share: number;
};

function GrahamForm({ ticker: initialTicker }: { ticker?: string } = {}) {
  const [ticker, setTicker] = useState(initialTicker ?? "");
  const [referenceYear, setReferenceYear] = useState(
    String(new Date().getFullYear()),
  );
  const [currentPrice, setCurrentPrice] = useState("");
  const [eps, setEps] = useState("");
  const [bookValuePerShare, setBookValuePerShare] = useState("");

  const tickerCollector = useTickerCollector();
  const [tickerError, setTickerError] = useState<string | null>(null);

  const { calculateMutation, saveMutation } = useValuationActions<
    CalculateGrahamRequest,
    GrahamInputsModel
  >("graham");
  const [lastAction, setLastAction] = useState<"calculate" | "save" | null>(null);

  async function handleFetch() {
    if (!ticker.trim()) {
      setTickerError("Ticker is required to fetch data.");
      return;
    }
    setTickerError(null);
    const data = await tickerCollector.mutateAsync(ticker).catch(() => null);
    if (!data) return;
    let filled = 0;
    if (data.quote) {
      setCurrentPrice(String(data.quote.price));
      filled++;
    }
    if (data.fundamentals) {
      setEps(String(data.fundamentals.lpa));
      setBookValuePerShare(String(data.fundamentals.vpa));
      filled++;
    }
    if (filled === 0) {
      setTickerError(`No data found for ${ticker.toUpperCase()}.`);
    }
  }

  function buildRequest(): CalculateGrahamRequest {
    return {
      ticker: ticker.toUpperCase(),
      reference_year: Number(referenceYear),
      current_price: Number(currentPrice),
      eps: Number(eps),
      book_value_per_share: Number(bookValuePerShare),
    };
  }

  function handleCalculate(event: FormEvent) {
    event.preventDefault();
    setLastAction("calculate");
    calculateMutation.mutate(buildRequest());
  }

  function handleSave() {
    setLastAction("save");
    saveMutation.mutate(buildRequest());
  }

  const activeMutation = lastAction === "save" ? saveMutation : calculateMutation;
  const resultValuation =
    lastAction === "save" ? (saveMutation.data?.valuation ?? null) : (calculateMutation.data ?? null);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Fair Price (Graham)</CardTitle>
      </CardHeader>
      <CardContent>
        <form
          onSubmit={handleCalculate}
          className="grid grid-cols-1 gap-4 sm:grid-cols-2"
        >
          <Field label="Ticker">
            <div className="flex gap-2">
              <Input
                required
                value={ticker}
                onChange={(e) => setTicker(e.currentTarget.value)}
                placeholder="ITSA4"
                className="flex-1"
              />
              <Button
                type="button"
                variant="outline"
                size="icon"
                onClick={handleFetch}
                disabled={tickerCollector.isPending}
                aria-label="Fetch data for this ticker"
                title="Fetch data for this ticker"
              >
                <RefreshCw
                  className={tickerCollector.isPending ? "animate-spin" : ""}
                />
              </Button>
            </div>
            {tickerError && <p className="text-red-600">{tickerError}</p>}
            {tickerCollector.isError && (
              <p className="text-red-600">{tickerCollector.error.message}</p>
            )}
          </Field>

          <Field label="Reference year">
            <Input
              required
              type="number"
              value={referenceYear}
              onChange={(e) => setReferenceYear(e.currentTarget.value)}
            />
          </Field>

          <Field label="Current price (R$)">
            <Input
              required
              type="number"
              step="0.01"
              value={currentPrice}
              onChange={(e) => setCurrentPrice(e.currentTarget.value)}
            />
          </Field>

          <Field label="EPS — earnings per share (R$)">
            <Input
              required
              type="number"
              step="0.01"
              value={eps}
              onChange={(e) => setEps(e.currentTarget.value)}
            />
          </Field>

          <Field label="Book value per share (R$)">
            <Input
              required
              type="number"
              step="0.01"
              value={bookValuePerShare}
              onChange={(e) => setBookValuePerShare(e.currentTarget.value)}
            />
          </Field>

          <div className="flex gap-3 sm:col-span-2">
            <Button type="submit" disabled={calculateMutation.isPending} className="flex-1">
              {calculateMutation.isPending ? "Calculating..." : "Calculate"}
            </Button>
            <Button
              type="button"
              variant="outline"
              onClick={handleSave}
              disabled={saveMutation.isPending}
              className="flex-1"
            >
              {saveMutation.isPending ? "Saving..." : "Save"}
            </Button>
          </div>
        </form>

        <ValuationResult
          isError={activeMutation.isError}
          error={activeMutation.error ?? null}
          isSuccess={activeMutation.isSuccess}
          valuation={resultValuation}
        />
        {lastAction === "save" && saveMutation.isSuccess && (
          <p className="mt-2 text-sm text-green-700">Saved.</p>
        )}
      </CardContent>
    </Card>
  );
}

export default GrahamForm;

import { useState } from "react";
import BazinForm from "./models/BazinForm";
import GrahamForm from "./models/GrahamForm";
import GordonForm from "./models/GordonForm";
import DcfForm from "./models/DcfForm";
import BanksForm from "./models/BanksForm";
import RimForm from "./models/RimForm";
import RnavForm from "./models/RnavForm";
import ProjectedCeilingForm from "./models/ProjectedCeilingForm";
import SavedValuationsPanel from "./valuations/SavedValuationsPanel";
import StockLookupPanel from "./stock-lookup/StockLookupPanel";
import AlertsPanel from "./alerts/AlertsPanel";
import PortfolioPanel from "./portfolio/PortfolioPanel";
import WorkspaceGate from "./workspace/WorkspaceGate";
import TruthIdPanel from "./truthid/TruthIdPanel";
import ChatPanel from "./chat/ChatPanel";
import ChatToggleButton from "./chat/ChatToggleButton";
import type { GeminiContent } from "./chat/types";
import SettingsPage from "./settings/SettingsPage";
import ChatScreen from "./chat-full/ChatScreen";
import Field from "./components/Field";
import { Button } from "@/components/ui/button";
import { MessageSquareIcon, SettingsIcon, LogOutIcon } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

const MODELS = {
  bazin: { label: "Bazin", component: BazinForm },
  graham: { label: "Graham", component: GrahamForm },
  gordon: { label: "Gordon / DDM", component: GordonForm },
  dcf: { label: "DCF / FCFF", component: DcfForm },
  banks: { label: "Banks (P/B)", component: BanksForm },
  rim: { label: "RIM (Bancos)", component: RimForm },
  rnav: { label: "RNAV", component: RnavForm },
  projected_ceiling: {
    label: "Projected Ceiling",
    component: ProjectedCeilingForm,
  },
} as const;

type ModelKey = keyof typeof MODELS;

// Fase 10, item 8, Sessão 51 — a antiga aba solta "Crypto Score" foi
// absorvida por Research (agora um seletor de classe ali, junto com Ação/
// FII/ETF) — pedido explícito do dono do projeto pra unificar as duas
// telas. `run_crypto_collector`/`crypto_indicators` no backend não mudaram
// de nome, só o lugar de onde a UI os chama (StockLookupPanel.tsx →
// CryptoLookupSection.tsx).
const SECTIONS = {
  valuation: "Valuation",
  lookup: "Research",
  portfolio: "Portfolio",
  alerts: "Alerts",
  truthid: "TruthID Sync",
} as const;

type ValuationView = "form" | "saved";

type SectionKey = keyof typeof SECTIONS;

// Sem lib de rotas — app desktop de janela única não tem barra de endereço
// pra uma URL de verdade ganhar alguma coisa. Esse estado troca a tela
// inteira (Tabs vs Configurações), mesmo padrão já usado dentro da aba
// Valuation pro toggle form/saved, só que no nível do App inteiro.
type AppView = "main" | "settings" | "chat";

function App() {
  const [workspaceId, setWorkspaceId] = useState<number | null>(null);
  const [view, setView] = useState<AppView>("main");
  const [section, setSection] = useState<SectionKey>("valuation");
  const [valuationView, setValuationView] = useState<ValuationView>("form");
  const [selectedModel, setSelectedModel] = useState<ModelKey>("bazin");
  const SelectedForm = MODELS[selectedModel].component;
  const [chatOpen, setChatOpen] = useState(false);
  const [chatHistory, setChatHistory] = useState<GeminiContent[]>([]);

  if (workspaceId === null) {
    return (
      <main className="mx-auto max-w-6xl p-8">
        <WorkspaceGate onEnter={setWorkspaceId} />
      </main>
    );
  }

  if (view === "settings") {
    return (
      <main className="mx-auto max-w-6xl p-8">
        <SettingsPage onBack={() => setView("main")} />
      </main>
    );
  }

  if (view === "chat") {
    return (
      <main className="mx-auto max-w-6xl p-8">
        <ChatScreen onBack={() => setView("main")} />
      </main>
    );
  }

  return (
    <>
      <main className="mx-auto max-w-6xl p-8">
        <Tabs
          value={section}
          onValueChange={(value) => setSection(value as SectionKey)}
        >
          <div className="mb-6 flex items-center justify-between">
            <TabsList>
              {Object.entries(SECTIONS).map(([key, label]) => (
                <TabsTrigger key={key} value={key}>
                  {label}
                </TabsTrigger>
              ))}
            </TabsList>
            <div className="flex items-center gap-1">
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                onClick={() => setView("chat")}
              >
                <MessageSquareIcon />
                <span className="sr-only">Full-screen chat</span>
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                onClick={() => setView("settings")}
              >
                <SettingsIcon />
                <span className="sr-only">Settings</span>
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                onClick={() => setWorkspaceId(null)}
              >
                <LogOutIcon />
                <span className="sr-only">Switch Workspace</span>
              </Button>
            </div>
          </div>

          <TabsContent value="valuation" className="flex flex-col gap-6">
            {valuationView === "form" ? (
              <>
                <div className="flex items-end justify-between gap-4">
                  <Field label="Valuation model" className="flex-1">
                    <Select
                      value={selectedModel}
                      onValueChange={(value) => setSelectedModel(value as ModelKey)}
                    >
                      <SelectTrigger className="w-full">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {Object.entries(MODELS).map(([key, { label }]) => (
                          <SelectItem key={key} value={key}>
                            {label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </Field>
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => setValuationView("saved")}
                  >
                    Saved Valuations
                  </Button>
                </div>

                <SelectedForm />
              </>
            ) : (
              <>
                <Button
                  type="button"
                  variant="outline"
                  className="w-fit"
                  onClick={() => setValuationView("form")}
                >
                  ← Back
                </Button>
                <SavedValuationsPanel />
              </>
            )}
          </TabsContent>

          <TabsContent value="lookup">
            <StockLookupPanel />
          </TabsContent>

          <TabsContent value="portfolio">
            <PortfolioPanel workspaceId={workspaceId} />
          </TabsContent>

          <TabsContent value="alerts">
            <AlertsPanel />
          </TabsContent>

          <TabsContent value="truthid">
            <TruthIdPanel />
          </TabsContent>
        </Tabs>
      </main>
      <ChatToggleButton open={chatOpen} onToggle={() => setChatOpen((o) => !o)} />
      <ChatPanel
        open={chatOpen}
        onOpenChange={setChatOpen}
        history={chatHistory}
        onHistoryChange={setChatHistory}
      />
    </>
  );
}

export default App;

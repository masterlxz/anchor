import { useState } from "react";
import StockLookupPanel from "./stock-lookup/StockLookupPanel";
import AlertsPanel from "./alerts/AlertsPanel";
import PortfolioPanel from "./portfolio/PortfolioPanel";
import WorkspaceGate from "./workspace/WorkspaceGate";
import ChatPanel from "./chat/ChatPanel";
import ChatToggleButton from "./chat/ChatToggleButton";
import type { GeminiContent } from "./chat/types";
import SettingsPage from "./settings/SettingsPage";
import ChatScreen from "./chat-full/ChatScreen";
import { Button } from "@/components/ui/button";
import { MessageSquareIcon, SettingsIcon, LogOutIcon } from "lucide-react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

// Fase 10, item 8, Sessão 51 — a antiga aba solta "Crypto Score" foi
// absorvida por Research (agora um seletor de classe ali, junto com Ação/
// FII/ETF) — pedido explícito do dono do projeto pra unificar as duas
// telas. `run_crypto_collector`/`crypto_indicators` no backend não mudaram
// de nome, só o lugar de onde a UI os chama (StockLookupPanel.tsx →
// CryptoLookupSection.tsx). Sessão 54: mesma lógica pra "TruthID Sync" —
// virou uma seção dentro de Configurações (`SettingsPage.tsx` →
// `TruthIdSettingsSection.tsx`), não mais aba própria aqui. Sessão 56: a
// aba "Valuation" (seletor de modelo + 8 forms) migrou pra dentro de
// Research/análise de ação (`StockAnalysisSection.tsx` → botão "New
// Valuation" → `models/NewValuationDialog.tsx`) — os modelos servem pra
// "empresas", não só uma tela solta, e a lista "Saved Valuations" foi
// junto (mesmo componente `valuations/SavedValuationsPanel.tsx`, agora
// aberto via popup em vez de toggle de aba).
const SECTIONS = {
  lookup: "Research",
  portfolio: "Portfolio",
  alerts: "Alerts",
} as const;

type SectionKey = keyof typeof SECTIONS;

// Sem lib de rotas — app desktop de janela única não tem barra de endereço
// pra uma URL de verdade ganhar alguma coisa. Esse estado troca a tela
// inteira (Tabs vs Configurações/chat).
type AppView = "main" | "settings" | "chat";

function App() {
  const [workspaceId, setWorkspaceId] = useState<number | null>(null);
  const [view, setView] = useState<AppView>("main");
  const [section, setSection] = useState<SectionKey>("lookup");
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

          <TabsContent value="lookup">
            <StockLookupPanel />
          </TabsContent>

          <TabsContent value="portfolio">
            <PortfolioPanel workspaceId={workspaceId} />
          </TabsContent>

          <TabsContent value="alerts">
            <AlertsPanel />
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

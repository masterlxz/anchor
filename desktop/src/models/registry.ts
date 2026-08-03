import type { ComponentType } from "react";
import BazinForm from "./BazinForm";
import GrahamForm from "./GrahamForm";
import GordonForm from "./GordonForm";
import DcfForm from "./DcfForm";
import BanksForm from "./BanksForm";
import RimForm from "./RimForm";
import RnavForm from "./RnavForm";
import ProjectedCeilingForm from "./ProjectedCeilingForm";

// Sessão 56 — movido de App.tsx: a aba própria "Valuation" foi migrada pra
// dentro de Research/análise de ação (`NewValuationDialog.tsx`), então esse
// registro deixou de ser algo que só o App inteiro usava.
export const MODELS = {
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
} as const satisfies Record<
  string,
  { label: string; component: ComponentType<{ ticker?: string; assetClass?: string }> }
>;

export type ModelKey = keyof typeof MODELS;

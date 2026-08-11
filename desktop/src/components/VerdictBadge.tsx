import { Badge } from "@/components/ui/badge";

function VerdictBadge({ verdict }: { verdict: string | null }) {
  if (!verdict) return <span className="text-muted-foreground">—</span>;

  return (
    <Badge
      className={
        verdict === "CHEAP"
          ? "bg-green-100 text-green-800 dark:bg-green-950 dark:text-green-300"
          : "bg-red-100 text-red-800 dark:bg-red-950 dark:text-red-300"
      }
    >
      {verdict}
    </Badge>
  );
}

export default VerdictBadge;

// Tier barato/rápido por provider — usado por qualquer entry point que
// sugere um modelo default assim que uma key é escolhida (chat, conversas
// salvas, extração de documento). O campo de modelo em cada UI continua
// livre pra editar depois.
export const DEFAULT_MODEL_BY_PROVIDER: Record<string, string> = {
  gemini: "gemini-3.1-flash-lite",
  claude: "claude-haiku-4-5",
  openai: "gpt-5-mini",
};

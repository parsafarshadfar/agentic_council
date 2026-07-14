import type { ModelInfo, ProviderSummary } from "../types";

const providerOrder = [
  "openrouter",
  "openai",
  "anthropic",
  "gemini",
  "huggingface",
  "xai",
  "groq",
  "deepseek",
  "together",
  "fireworks",
  "siliconflow",
  "perplexity",
  "dashscope",
  "moonshot",
  "zai",
  "inference_net",
] as const;

const providerRank = new Map<string, number>(providerOrder.map((id, index) => [id, index]));

export const sortProvidersForSelection = (providers: ProviderSummary[]) =>
  providers
    .filter((provider) => provider.id !== "demo")
    .sort((left, right) => {
      if (left.id === "custom") return right.id === "custom" ? 0 : 1;
      if (right.id === "custom") return -1;
      const leftRank = providerRank.get(left.id);
      const rightRank = providerRank.get(right.id);
      if (leftRank != null || rightRank != null) {
        return (leftRank ?? Number.MAX_SAFE_INTEGER) - (rightRank ?? Number.MAX_SAFE_INTEGER);
      }
      return left.name.localeCompare(right.name);
    });

export const providerVerification = (provider: ProviderSummary) => {
  if (provider.credential_status === "valid" || provider.credential_status === "not_required") {
    return { kind: "verified" as const, label: "Verified", mark: "✓" };
  }
  if (provider.credential_status === "configured") {
    return { kind: "imported" as const, label: "Imported", mark: "✓" };
  }
  return { kind: "other" as const, label: "", mark: "" };
};

export const providerOptionLabel = (provider: ProviderSummary) => {
  const status = providerVerification(provider);
  return status.kind === "other"
    ? provider.name
    : `${status.mark} ${provider.name} — ${status.label}`;
};

export const modelMatchesSearch = (model: ModelInfo, query: string) => {
  const term = query.trim().toLocaleLowerCase();
  return !term
    || model.name.toLocaleLowerCase().includes(term)
    || model.id.toLocaleLowerCase().includes(term);
};

import { describe, expect, it } from "vitest";
import type { ProviderSummary } from "../types";
import {
  modelMatchesSearch,
  providerOptionLabel,
  sortProvidersForSelection,
} from "./providerPresentation";

const provider = (
  id: string,
  credential_status: ProviderSummary["credential_status"],
): ProviderSummary => ({
  id,
  name: id.toUpperCase(),
  base_url: "https://example.com/v1",
  protocol: "open_ai",
  credential_status,
  supports_discovery: true,
  configurable_endpoint: false,
  timeout: {
    connect_secs: 13,
    first_token_secs: 38,
    idle_stream_secs: 19,
    total_secs: 375,
    max_attempts: 3,
  },
});

describe("model word search", () => {
  const model = {
    id: "vendor/model:free",
    provider_id: "openrouter",
    name: "Useful Model (Free)",
    context_window: 128_000,
    input_per_million: 0,
    output_per_million: 0,
    supports_vision: false,
    supports_documents: true,
    supports_streaming: true,
    reasoning: false,
  };

  it("matches case-insensitively against both model names and IDs", () => {
    expect(modelMatchesSearch(model, "free")).toBe(true);
    expect(modelMatchesSearch(model, "VENDOR/MODEL")).toBe(true);
    expect(modelMatchesSearch(model, "paid-only")).toBe(false);
  });
});

describe("provider selection presentation", () => {
  it("uses the requested provider order and keeps custom last", () => {
    const sorted = sortProvidersForSelection([
      provider("custom", "valid"),
      provider("deepseek", "valid"),
      provider("groq", "untested"),
      provider("xai", "configured"),
      provider("huggingface", "invalid"),
      provider("gemini", "untested"),
      provider("anthropic", "untested"),
      provider("openai", "untested"),
      provider("openrouter", "untested"),
      provider("another", "valid"),
      provider("demo", "not_required"),
    ]);
    expect(sorted.map((item) => item.id)).toEqual([
      "openrouter",
      "openai",
      "anthropic",
      "gemini",
      "huggingface",
      "xai",
      "groq",
      "deepseek",
      "another",
      "custom",
    ]);
    expect(providerOptionLabel(sorted[5]!)).toContain("✓ XAI — Imported");
    expect(providerOptionLabel(sorted[7]!)).toContain("✓ DEEPSEEK — Verified");
    expect(providerOptionLabel(sorted[4]!)).toBe("HUGGINGFACE");
  });
});

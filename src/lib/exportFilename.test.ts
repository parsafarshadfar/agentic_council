import { describe, expect, it } from "vitest";
import type { SessionState } from "../types";
import { fileSafePhrase, sessionExportBasename } from "./exportFilename";

describe("session export filenames", () => {
  it("uses date, orchestrator phrase, and latest round", () => {
    const session = {
      main_phrase: "Choosing a Privacy-First Model!",
      objective: "A longer objective that should not be used",
      rounds: [{ index: 3 }],
    } as SessionState;
    expect(sessionExportBasename(session, new Date(2026, 6, 13))).toBe(
      "2026-07-13__choosing-a-privacy-first-model__round_3",
    );
  });

  it("falls back safely for legacy or punctuation-only phrases", () => {
    expect(fileSafePhrase("Évidence & cost / risk")).toBe("evidence-cost-risk");
    expect(fileSafePhrase("!!!")).toBe("council-session");
  });
});

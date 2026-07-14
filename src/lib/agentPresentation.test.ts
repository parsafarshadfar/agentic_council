import { describe, expect, it } from "vitest";
import { councilorLabel } from "./agentPresentation";

describe("councilorLabel", () => {
  it("renames numbered legacy member labels", () => {
    expect(councilorLabel("Member 1")).toBe("Councilor 1");
    expect(councilorLabel("member 8")).toBe("Councilor 8");
    expect(councilorLabel("Member 4 · gpt-example")).toBe("Councilor 4 · gpt-example");
  });

  it("preserves custom display names", () => {
    expect(councilorLabel("Security reviewer")).toBe("Security reviewer");
  });
});

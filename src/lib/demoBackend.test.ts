import { describe, expect, it } from "vitest";
import { createDemoBootstrap, DemoBackend } from "./demoBackend";
import type { SessionState } from "../types";

describe("browser demo council lifecycle", () => {
  it("runs the gated round, scoring, and final synthesis flow", async () => {
    const demo = new DemoBackend(createDemoBootstrap());
    const initial = await demo.command<ReturnType<typeof createDemoBootstrap>>("bootstrap", {});
    const evaluated = await demo.command<SessionState>("start_preflight", {
      input: {
        objective: "Design a secure local-first desktop system with measurable reliability targets and a four-week validation plan.",
        agents: initial.session.agents,
        attachment_paths: [],
      },
    });
    const clarified = evaluated.phase === "clarification"
      ? await demo.command<SessionState>("submit_clarification", {
          answers: Object.fromEntries(evaluated.clarification_questions.map((question) => [question.id, "Produce an implementation-ready recommendation within the stated security and timeline constraints."])),
        })
      : evaluated;
    expect(clarified.phase).toBe("aspect_gate");
    expect(clarified.aspects.map((aspect) => aspect.name)).toEqual(expect.arrayContaining([
      "Technical feasibility",
      "Timeline & deliverability",
    ]));
    const approved = await demo.command<SessionState>("approve_aspects", { aspects: clarified.aspects });
    expect(approved.phase).toBe("post_round");

    const chunks: string[] = [];
    const unlisten = demo.listen<{ delta: string }>("agent://chunk", (chunk) => chunks.push(chunk.delta));
    const result = await demo.command<SessionState>("start_round", { userArgument: null });
    unlisten();
    expect(result.phase).toBe("post_round");
    expect(chunks.length).toBeGreaterThan(1);
    expect(result.rounds[0]?.responses.every((response) => response.status === "complete")).toBe(true);
    expect(result.rounds[0]?.friction.length).toBeGreaterThan(0);
    expect(result.rounds[0]?.scores.length).toBeGreaterThan(0);

    const finalized = await demo.command<SessionState>("finalize_session", {});
    expect(finalized.phase).toBe("finalized");
    expect(finalized.final_synthesis).toMatch(/evidence-led/i);
  });
});

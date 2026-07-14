import type {
  AgentAssignment,
  AppNotice,
  Aspect,
  BootstrapPayload,
  FrictionItem,
  ModelInfo,
  Persona,
  PreflightInput,
  ProviderSummary,
  RoundRecord,
  ScoreCell,
  SessionState,
  StreamChunk,
  TelemetrySnapshot,
} from "../types";

type DemoEvent = "session://snapshot" | "agent://chunk" | "telemetry://updated" | "app://notice";
type Callback = (payload: never) => void;

const now = () => new Date().toISOString();
const uid = () => crypto.randomUUID();
const clone = <T,>(value: T): T => structuredClone(value);

const timeout = {
  connect_secs: 13,
  first_token_secs: 38,
  idle_stream_secs: 19,
  total_secs: 375,
  max_attempts: 3,
};

const providerSeeds: Array<[string, string, ProviderSummary["protocol"], string, boolean]> = [
  ["demo", "Local Demo", "demo", "local://demo", false],
  ["openai", "OpenAI", "open_ai", "https://api.openai.com/v1", true],
  ["anthropic", "Anthropic", "anthropic", "https://api.anthropic.com/v1", true],
  ["gemini", "Google Gemini", "gemini", "https://generativelanguage.googleapis.com/v1beta", true],
  ["deepseek", "DeepSeek", "open_ai", "https://api.deepseek.com", false],
  ["xai", "xAI", "open_ai", "https://api.x.ai/v1", true],
  ["openrouter", "OpenRouter", "open_ai", "https://openrouter.ai/api/v1", true],
  ["groq", "Groq", "open_ai", "https://api.groq.com/openai/v1", true],
  ["together", "Together AI", "open_ai", "https://api.together.xyz/v1", true],
  ["fireworks", "Fireworks AI", "open_ai", "https://api.fireworks.ai/inference/v1", true],
  ["siliconflow", "SiliconFlow", "open_ai", "https://api.siliconflow.com/v1", true],
  ["huggingface", "Hugging Face", "open_ai", "https://router.huggingface.co/v1", true],
  ["perplexity", "Perplexity", "open_ai", "https://api.perplexity.ai", false],
  ["dashscope", "Alibaba DashScope", "open_ai", "https://dashscope-intl.aliyuncs.com/compatible-mode/v1", false],
  ["moonshot", "Moonshot AI", "open_ai", "https://api.moonshot.ai/v1", false],
  ["zai", "Z.AI", "open_ai", "https://api.z.ai/api/paas/v4", false],
  ["inference_net", "Inference.net", "open_ai", "https://api.inference.net/v1", false],
  ["custom", "Custom OpenAI-compatible", "open_ai", "https://example.invalid/v1", false],
];

export const defaultProviders = (): ProviderSummary[] =>
  providerSeeds.map(([id, name, protocol, base_url, discovery]) => ({
    id,
    name,
    protocol,
    base_url,
    supports_discovery: discovery,
    configurable_endpoint: id === "custom" || id === "openai",
    credential_status: id === "demo" ? "not_required" : "untested",
    timeout: { ...timeout, first_token_secs: id === "deepseek" ? 113 : 38 },
  }));

export const defaultModels = (): ModelInfo[] => [
  {
    id: "council-demo",
    provider_id: "demo",
    name: "Council Demo (offline)",
    context_window: 64_000,
    input_per_million: 0,
    output_per_million: 0,
    supports_vision: true,
    supports_documents: true,
    supports_streaming: true,
    reasoning: false,
  },
  { id: "gpt-4.1", provider_id: "openai", name: "GPT-4.1", context_window: 1_047_576, input_per_million: 2, output_per_million: 8, supports_vision: true, supports_documents: true, supports_streaming: true, reasoning: false },
  { id: "o3", provider_id: "openai", name: "o3", context_window: 200_000, input_per_million: 2, output_per_million: 8, supports_vision: true, supports_documents: true, supports_streaming: true, reasoning: true },
  { id: "claude-sonnet-4", provider_id: "anthropic", name: "Claude Sonnet 4", context_window: 200_000, input_per_million: 3, output_per_million: 15, supports_vision: true, supports_documents: true, supports_streaming: true, reasoning: true },
  { id: "gemini-2.5-pro", provider_id: "gemini", name: "Gemini 2.5 Pro", context_window: 1_048_576, input_per_million: 1.25, output_per_million: 10, supports_vision: true, supports_documents: true, supports_streaming: true, reasoning: true },
  { id: "deepseek-v4-flash", provider_id: "deepseek", name: "DeepSeek V4 Flash", context_window: 128_000, input_per_million: null, output_per_million: null, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: true },
  { id: "deepseek-v4-pro", provider_id: "deepseek", name: "DeepSeek V4 Pro", context_window: 128_000, input_per_million: null, output_per_million: null, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: true },
  { id: "grok-3", provider_id: "xai", name: "Grok 3", context_window: 131_072, input_per_million: 3, output_per_million: 15, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: true },
  { id: "openai/gpt-4.1", provider_id: "openrouter", name: "GPT-4.1 via OpenRouter", context_window: 1_047_576, input_per_million: 2, output_per_million: 8, supports_vision: true, supports_documents: true, supports_streaming: true, reasoning: false },
  { id: "llama-3.3-70b-versatile", provider_id: "groq", name: "Llama 3.3 70B", context_window: 128_000, input_per_million: 0.59, output_per_million: 0.79, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: false },
  { id: "meta-llama/Llama-3.3-70B-Instruct-Turbo", provider_id: "together", name: "Llama 3.3 70B Turbo", context_window: 131_072, input_per_million: 0.88, output_per_million: 0.88, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: false },
  { id: "accounts/fireworks/models/llama-v3p3-70b-instruct", provider_id: "fireworks", name: "Llama 3.3 70B", context_window: 131_072, input_per_million: 0.9, output_per_million: 0.9, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: false },
  { id: "Qwen/Qwen2.5-72B-Instruct", provider_id: "siliconflow", name: "Qwen 2.5 72B", context_window: 32_768, input_per_million: 0.56, output_per_million: 0.56, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: false },
  { id: "openai/gpt-oss-120b:fastest", provider_id: "huggingface", name: "GPT OSS 120B (fastest route)", context_window: 128_000, input_per_million: null, output_per_million: null, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: false },
  { id: "sonar-pro", provider_id: "perplexity", name: "Sonar Pro", context_window: 200_000, input_per_million: 3, output_per_million: 15, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: false },
  { id: "qwen-plus", provider_id: "dashscope", name: "Qwen Plus", context_window: 131_072, input_per_million: null, output_per_million: null, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: false },
  { id: "moonshot-v1-128k", provider_id: "moonshot", name: "Moonshot 128K", context_window: 128_000, input_per_million: null, output_per_million: null, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: false },
  { id: "glm-5.1", provider_id: "zai", name: "GLM 5.1", context_window: 202_752, input_per_million: null, output_per_million: null, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: true },
  { id: "meta-llama/llama-3.3-70b-instruct", provider_id: "inference_net", name: "Llama 3.3 70B", context_window: 128_000, input_per_million: null, output_per_million: null, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: false },
  { id: "custom-model", provider_id: "custom", name: "Custom model", context_window: 32_768, input_per_million: null, output_per_million: null, supports_vision: false, supports_documents: true, supports_streaming: true, reasoning: false },
];

export const defaultPersonas = (): Persona[] => [
  ["devils-advocate", "Devil's Advocate", "Challenges assumptions and exposes omissions.", "Question assumptions, probe logic, expose omissions, and challenge premature consensus.", ["Name the strongest counterargument", "Separate evidence from assertion"]],
  ["visionary", "Visionary Product Innovator", "Optimizes for a delightful, disruptive product.", "Prioritize user experience, simplicity, aesthetic detail, and disruptive product thinking.", ["Start from the user outcome", "Look for a 10x simplification"]],
  ["first-principles", "First-Principles Simplifier", "Reduces the problem to fundamental truths.", "Decompose the problem into fundamentals, remove jargon, and identify unnecessary assumptions.", ["Define irreducible constraints", "Prefer the simplest sufficient model"]],
  ["strategist", "Pragmatic Strategist", "Tests viability, incentives, and competitive position.", "Analyze incentives, power dynamics, competitive advantage, hidden risk, and pragmatic viability.", ["Identify the binding constraint", "Make trade-offs explicit"]],
  ["architect", "Technical Architect", "Evaluates design quality and operational behavior.", "Evaluate architecture, complexity, performance, reliability, security, and maintainability.", ["Quantify scale assumptions", "Trace failure modes"]],
  ["ethical-guardian", "Ethical Guardian", "Assesses consequences, fairness, and resilience.", "Assess long-term consequences, stakeholder impact, ethical risk, balance, and resilience.", ["Include affected non-users", "Test misuse and reversibility"]],
].map(([id, name, description, system_prompt, directives]) => ({
  id: id as string,
  name: name as string,
  description: description as string,
  system_prompt: system_prompt as string,
  directives: directives as string[],
  builtin: true,
}));

const demoAgent = (id: string, role: AgentAssignment["role"], name: string, persona: string | null): AgentAssignment => ({
  id,
  role,
  display_name: name,
  provider_id: "demo",
  model_id: "council-demo",
  persona_id: persona,
  status: "idle",
});

export const emptySession = (): SessionState => ({
  schema_version: 1,
  id: uid(),
  phase: "pre_session",
  objective: "",
  main_phrase: "",
  ambiguity_score: null,
  clarification_questions: [],
  clarification_answers: {},
  aspects: [],
  agents: [
    demoAgent("orchestrator", "orchestrator", "Moderator", "strategist"),
    demoAgent("member-1", "member", "Councilor 1", "devils-advocate"),
    demoAgent("member-2", "member", "Councilor 2", "architect"),
  ],
  attachments: [],
  rounds: [],
  compacted_history: [],
  final_synthesis: null,
  created_at: now(),
  updated_at: now(),
});

const emptyTelemetry = (sessionId: string): TelemetrySnapshot => ({
  session_id: sessionId,
  by_model: [],
  total_input_tokens: 0,
  total_output_tokens: 0,
  total_cost_usd: 0,
});

export const createDemoBootstrap = (): BootstrapPayload => {
  const session = emptySession();
  return {
    session,
    telemetry: emptyTelemetry(session.id),
    providers: defaultProviders(),
    models: defaultModels(),
    personas: defaultPersonas(),
    notices: [],
    recoverable_checkpoint: false,
    app_version: "0.1.0-browser-demo",
  };
};

const defaultAspects = (objective: string): Aspect[] => {
  const lower = objective.toLowerCase();
  const candidates: Array<[RegExp, string, string]> = [
    [/\b(?:privacy|secure|security|confidential|confidentiality|data)\b/, "Privacy & security", "Evaluate privacy boundaries, threat exposure, data handling, and concrete safeguards."],
    [/\b(?:cost|costs|budget|price|affordable|affordability)\b/, "Cost & resource trade-offs", "Compare direct and hidden costs, resource demands, and value under the stated constraints."],
    [/\b(?:deadline|timeline|week|weeks|month|months|delivery)\b/, "Timeline & deliverability", "Test whether the proposal can be delivered and validated within the stated time horizon."],
    [/\b(?:software|app|application|system|platform|api|architecture|technical)\b/, "Technical feasibility", "Evaluate implementation complexity, integration constraints, reliability, and maintainability."],
    [/\b(?:research|evidence|study|studies|source|sources|claim|claims)\b/, "Evidence quality", "Assess source quality, uncertainty, competing interpretations, and what evidence could change the conclusion."],
    [/\b(?:law|legal|regulation|regulatory|policy|compliance)\b/, "Legal & policy constraints", "Identify relevant legal or policy constraints, jurisdictional uncertainty, and compliance risk."],
    [/\b(?:health|medical|clinical|patient|patients|safety)\b/, "Safety & outcome evidence", "Weigh safety, outcome evidence, uncertainty, and the cost of a wrong conclusion."],
    [/\b(?:people|user|users|student|students|team|community|ethical|society)\b/, "Stakeholder impact", "Compare effects on the people who use, operate, or are affected by the decision."],
    [/\b(?:compare|comparison|choose|choice|versus|vs|option|options|alternative|alternatives)\b/, "Comparative advantage", "Make the decisive differences between the available options explicit and testable."],
  ];
  const topic = objective.trim().replaceAll(/\s+/g, " ").slice(0, 120);
  const selected: Array<[string, string]> = [["Decision fit", `Judge how directly the recommendation resolves: “${topic}${objective.length > 120 ? "…" : ""}”.`]];
  for (const [pattern, name, description] of candidates) {
    if (pattern.test(lower) && !selected.some(([item]) => item === name)) selected.push([name, description]);
    if (selected.length === 5) break;
  }
  const fallbacks: Array<[string, string]> = [
    ["Evidence & assumptions", "Separate supported facts from assumptions and identify the evidence most likely to change the recommendation."],
    ["Risks & reversibility", "Compare failure modes, downside severity, reversibility, and practical mitigation options."],
    ["Actionability", "Require a concrete next step, decision threshold, accountable owner, and review condition."],
  ];
  for (const fallback of fallbacks) {
    if (selected.length < 4) selected.push(fallback);
  }
  return selected.slice(0, 5).map(([name, description]) => ({
    id: name.toLowerCase().replaceAll(/[^a-z]+/g, "-").replaceAll(/(^-|-$)/g, ""),
    name,
    description,
    weight: 1,
  }));
};

const clarificationFor = (objective: string) => {
  const lower = objective.toLowerCase();
  const topic = objective.trim().replaceAll(/\s+/g, " ").slice(0, 90);
  const questions: SessionState["clarification_questions"] = [];
  if (!/compare|choose|decide|design|plan|explain|evaluate|recommend/.test(lower)) {
    questions.push({ id: uid(), prompt: `What exact decision or deliverable should the council produce for “${topic}”?`, rationale: "The requested output is not explicit enough to determine what a useful conclusion looks like." });
  }
  if (!/must|without|within|budget|deadline|constraint|cannot/.test(lower)) {
    questions.push({ id: uid(), prompt: `Which constraints or unacceptable outcomes must shape the answer about “${topic}”?`, rationale: "The objective does not identify the boundaries that would rule an otherwise attractive option out." });
  }
  if (!/success|measure|criteria|threshold|outcome/.test(lower)) {
    questions.push({ id: uid(), prompt: "What evidence or observable outcome should distinguish a strong answer from a weak one?", rationale: "A decision criterion lets the council compare recommendations against the same standard." });
  }
  return questions.slice(0, 3);
};

const ambiguity = (objective: string) => {
  let score = Math.min(90, 35 + objective.trim().split(/\s+/).length * 1.5);
  if (/\b(must|should|without|within|budget|deadline|because)\b/i.test(objective)) score += 8;
  if (objective.length < 60) score -= 20;
  return Math.max(10, Math.min(98, Math.round(score)));
};

const mainPhraseFor = (objective: string) => {
  const stopWords = new Set(["a", "an", "and", "are", "for", "how", "i", "in", "is", "of", "on", "the", "to", "what", "with"]);
  const words = objective
    .split(/[^\p{L}\p{N}]+/u)
    .filter((word) => word && !stopWords.has(word.toLocaleLowerCase()))
    .slice(0, 8);
  return words.join(" ") || "Council session";
};

const responseFor = (agent: AgentAssignment, objective: string, aspects: Aspect[], round: number) => {
  const persona = agent.persona_id?.replaceAll("-", " ") ?? "independent analyst";
  const lead = round === 1
    ? `My recommendation is to turn the objective into a falsifiable decision, then test the riskiest assumption before committing to the full solution.`
    : `Building on the prior friction, I would narrow the disagreement to the assumptions that could actually change the decision.`;
  const bullets = aspects.map((aspect, index) =>
    `**${aspect.name}.** ${index % 2 === 0 ? "Define a measurable threshold and an owner" : "Run a bounded experiment and record the failure condition"}; this keeps the council's conclusion actionable rather than rhetorical.`,
  );
  return `### ${agent.display_name}\n\nAs the ${persona}, I interpret the objective as: “${objective}”\n\n${lead}\n\n${bullets.map((item) => `- ${item}`).join("\n")}\n\n**Decision test:** proceed only if the evidence clears the agreed thresholds; otherwise preserve reversibility and revisit the cheapest disputed assumption.`;
};

const frictionFor = (agents: AgentAssignment[], aspects: Aspect[]): FrictionItem[] => [
  {
    id: uid(),
    kind: "contradiction",
    agent_ids: agents.slice(0, 2).map((agent) => agent.id),
    aspect_id: aspects[1]?.id ?? null,
    explanation: "The responses favor different levels of upfront validation, but neither quantifies the cost of waiting.",
    challenge: "What evidence would justify acting now, and what is the explicit cost of one more validation cycle?",
  },
  {
    id: uid(),
    kind: "omission",
    agent_ids: [],
    aspect_id: aspects.at(-1)?.id ?? null,
    explanation: "No response names a single accountable owner or rollback trigger.",
    challenge: "Assign an owner, a review date, and a reversible exit condition in the next round.",
  },
];

const scoresFor = (members: AgentAssignment[], aspects: Aspect[], round: number): ScoreCell[] =>
  members.flatMap((agent, agentIndex) =>
    aspects.map((aspect, aspectIndex) => {
      const votes = members
        .filter((peer) => peer.id !== agent.id)
        .map((peer, voterIndex) => ({
          voter_alias: `${peer.display_name} · ${peer.model_id}`,
          score: Math.min(10, 6.8 + ((agentIndex + aspectIndex + voterIndex + round) % 4) * 0.6),
          outlier: false,
        }));
      const sorted = votes.map((vote) => vote.score).sort((a, b) => a - b);
      const median = sorted.length === 0
        ? 0
        : sorted.length % 2
          ? sorted[Math.floor(sorted.length / 2)]!
          : (sorted[sorted.length / 2 - 1]! + sorted[sorted.length / 2]!) / 2;
      return { agent_id: agent.id, aspect_id: aspect.id, median, votes };
    }),
  );

export class DemoBackend {
  private payload: BootstrapPayload;
  private listeners = new Map<DemoEvent, Set<Callback>>();
  private cancelled = false;

  constructor(payload: BootstrapPayload) {
    this.payload = payload;
  }

  listen<T>(event: DemoEvent, callback: (payload: T) => void) {
    const callbacks = this.listeners.get(event) ?? new Set<Callback>();
    callbacks.add(callback as Callback);
    this.listeners.set(event, callbacks);
    return () => callbacks.delete(callback as Callback);
  }

  private emit<T>(event: DemoEvent, payload: T) {
    this.listeners.get(event)?.forEach((callback) => callback(payload as never));
  }

  private snapshot() {
    this.payload.session.updated_at = now();
    const value = clone(this.payload.session);
    this.emit("session://snapshot", value);
    return value;
  }

  private notice(severity: AppNotice["severity"], title: string, message: string) {
    const item: AppNotice = { id: uid(), severity, title, message, details: null, timestamp: now() };
    this.payload.notices.unshift(item);
    this.emit("app://notice", clone(item));
  }

  async command<T>(name: string, args: Record<string, unknown>): Promise<T> {
    switch (name) {
      case "bootstrap": return clone(this.payload) as T;
      case "round_poll": return clone({
        session: this.payload.session,
        telemetry: this.payload.telemetry,
        notices: this.payload.notices,
      }) as T;
      case "start_preflight": return this.startPreflight(args.input as PreflightInput) as T;
      case "submit_clarification": return this.submitClarification(args.answers as Record<string, string>) as T;
      case "approve_aspects": return this.approveAspects(args.aspects as Aspect[]) as T;
      case "reject_aspects": {
        this.payload.session.phase = "pre_session";
        return this.snapshot() as T;
      }
      case "start_round": {
        return await this.beginRound((args.userArgument as string | null) ?? null) as T;
      }
      case "stop_round": {
        this.cancelled = true;
        this.payload.session.agents.forEach((agent) => {
          if (agent.status === "streaming") agent.status = "cancelled";
        });
        this.payload.session.phase = this.payload.session.rounds.length ? "post_round" : "pre_session";
        this.notice("warning", "Round stopped", "Partial responses were retained and excluded from scoring.");
        return this.snapshot() as T;
      }
      case "retry_agent": return this.retryAgent(args.agentId as string) as T;
      case "finalize_session": {
        this.payload.session.final_synthesis = "The council converged on a reversible, evidence-led decision path: define measurable thresholds, test the riskiest assumption first, and assign a named owner and rollback trigger before scaling the commitment.";
        this.payload.session.phase = "finalized";
        return this.snapshot() as T;
      }
      case "new_session": return this.reset((args.agents as AgentAssignment[] | null) ?? this.payload.session.agents) as T;
      case "save_credential": {
        const provider = this.payload.providers.find((item) => item.id === args.providerId);
        if (provider) provider.credential_status = "configured";
        return "Saved securely and imported the current model catalog. Connection is not verified yet." as T;
      }
      case "delete_credential": {
        const provider = this.payload.providers.find((item) => item.id === args.providerId);
        if (provider && provider.id !== "demo") provider.credential_status = "untested";
        return undefined as T;
      }
      case "test_connection": {
        const provider = this.payload.providers.find((item) => item.id === args.providerId);
        if (provider) provider.credential_status = "valid";
        return "Connection verified in browser demo mode." as T;
      }
      case "refresh_models": return "Current generation models retrieved in browser demo mode." as T;
      case "save_persona": {
        const persona = args.persona as Persona;
        this.payload.personas = [...this.payload.personas.filter((item) => item.id !== persona.id), persona];
        return clone(this.payload.personas) as T;
      }
      case "delete_persona": {
        this.payload.personas = this.payload.personas.filter((item) => item.id !== args.personaId || item.builtin);
        return clone(this.payload.personas) as T;
      }
      case "restore_checkpoint":
      case "discard_checkpoint": return clone(this.payload) as T;
      case "hard_clear": return this.reset(null) as T;
      case "ingest_files": {
        this.notice("info", "Desktop feature", "File extraction is available in the Tauri desktop build.");
        return this.snapshot() as T;
      }
      case "import_session":
      case "export_markdown":
      case "export_pdf": throw new Error("Native file operations require the Tauri desktop build.");
      default: throw new Error(`Unknown backend command: ${name}`);
    }
  }

  private startPreflight(input: PreflightInput) {
    const score = ambiguity(input.objective);
    const questions = score < 62 ? clarificationFor(input.objective) : [];
    this.payload.session.objective = input.objective.trim();
    this.payload.session.main_phrase = mainPhraseFor(input.objective);
    this.payload.session.agents = clone(input.agents);
    this.payload.session.ambiguity_score = score;
    this.payload.session.aspects = defaultAspects(input.objective);
    this.payload.session.clarification_questions = questions;
    this.payload.session.phase = questions.length > 0 ? "clarification" : "aspect_gate";
    return this.snapshot();
  }

  private submitClarification(answers: Record<string, string>) {
    this.payload.session.clarification_answers = { ...this.payload.session.clarification_answers, ...answers };
    const complete = this.payload.session.clarification_questions.every((question) => answers[question.id]?.trim());
    if (!complete) throw new Error("Please answer each clarification question.");
    this.payload.session.objective += `\n\nClarifications:\n${Object.values(answers).map((value) => `- ${value}`).join("\n")}`;
    this.payload.session.main_phrase = mainPhraseFor(this.payload.session.objective);
    this.payload.session.ambiguity_score = Math.min(98, (this.payload.session.ambiguity_score ?? 50) + 25);
    this.payload.session.aspects = defaultAspects(this.payload.session.objective);
    this.payload.session.phase = "aspect_gate";
    return this.snapshot();
  }

  private approveAspects(aspects: Aspect[]) {
    if (aspects.length < 3 || aspects.length > 5) throw new Error("Use between 3 and 5 discussion aspects.");
    this.payload.session.aspects = clone(aspects);
    this.payload.session.phase = "post_round";
    return this.snapshot();
  }

  private async beginRound(userArgument: string | null) {
    this.cancelled = false;
    const index = this.payload.session.rounds.length + 1;
    const members = this.payload.session.agents.filter((agent) => agent.role === "member");
    const round: RoundRecord = {
      index,
      started_at: now(),
      completed_at: null,
      responses: members.map((agent) => ({ agent_id: agent.id, content: "", status: "streaming", error: null, input_tokens: null, output_tokens: null, latency_ms: 0 })),
      friction: [],
      scores: [],
      user_argument: userArgument,
      semantic_similarity: null,
      consensus: null,
    };
    this.payload.session.rounds.push(round);
    this.payload.session.phase = "round_running";
    this.payload.session.agents.forEach((agent) => { agent.status = agent.role === "member" ? "streaming" : "idle"; });
    this.snapshot();
    // The browser preview completes its local simulation within the command so
    // hidden-tab scheduling cannot strand the UI. Native desktop rounds still
    // stream and remain cancellable in the Rust engine.
    await this.runRound(round, members);
    return clone(this.payload.session);
  }

  private async runRound(round: RoundRecord, members: AgentAssignment[]) {
    const started = performance.now();
    const jobs = members.map(async (agent) => {
      const content = responseFor(agent, this.payload.session.objective, this.payload.session.aspects, round.index);
      // Keep the demo stream bounded so background browser timer throttling cannot
      // make an otherwise-local preview appear stalled. Real providers emit their
      // native token cadence through the Rust event stream.
      const chunkSize = Math.max(1, Math.ceil(content.length / 6));
      const chunks = content.match(new RegExp(`[\\s\\S]{1,${chunkSize}}`, "g")) ?? [content];
      for (const delta of chunks) {
        if (this.cancelled) return;
        // Yield between chunks without relying on timers. Browsers can suspend
        // timer and MessageChannel tasks for a hidden preview tab, while promise
        // continuations remain deterministic for this local-only simulation.
        await Promise.resolve();
        const response = round.responses.find((item) => item.agent_id === agent.id);
        if (response) response.content += delta;
        const event: StreamChunk = { session_id: this.payload.session.id, round_index: round.index, agent_id: agent.id, delta };
        this.emit("agent://chunk", event);
      }
      const response = round.responses.find((item) => item.agent_id === agent.id);
      if (response) {
        response.status = "complete";
        response.input_tokens = Math.round(this.payload.session.objective.length / 4);
        response.output_tokens = Math.round(response.content.length / 4);
        response.latency_ms = Math.round(performance.now() - started);
      }
      agent.status = "complete";
    });
    await Promise.all(jobs);
    if (this.cancelled) return;
    round.friction = frictionFor(members, this.payload.session.aspects);
    round.scores = scoresFor(members, this.payload.session.aspects, round.index);
    round.semantic_similarity = Math.min(94, 42 + round.index * 7);
    round.consensus = Math.min(96, 68 + round.index * 5);
    round.completed_at = now();
    this.payload.session.phase = "post_round";
    const input = round.responses.reduce((sum, item) => sum + (item.input_tokens ?? 0), 0);
    const output = round.responses.reduce((sum, item) => sum + (item.output_tokens ?? 0), 0);
    this.payload.telemetry = {
      session_id: this.payload.session.id,
      by_model: [{ provider_id: "demo", model_id: "council-demo", input_tokens: input, output_tokens: output, input_cost_usd: 0, output_cost_usd: 0, total_cost_usd: 0 }],
      total_input_tokens: (this.payload.telemetry.total_input_tokens ?? 0) + input,
      total_output_tokens: (this.payload.telemetry.total_output_tokens ?? 0) + output,
      total_cost_usd: 0,
    };
    this.emit("telemetry://updated", clone(this.payload.telemetry));
    this.snapshot();
  }

  private retryAgent(agentId: string) {
    const round = this.payload.session.rounds.at(-1);
    const agent = this.payload.session.agents.find((item) => item.id === agentId);
    if (!round || !agent) throw new Error("Agent or round not found.");
    round.responses = round.responses.filter((item) => item.agent_id !== agentId);
    round.responses.push({ agent_id: agentId, content: responseFor(agent, this.payload.session.objective, this.payload.session.aspects, round.index), status: "complete", error: null, input_tokens: null, output_tokens: null, latency_ms: 0 });
    agent.status = "complete";
    return this.snapshot();
  }

  private reset(agents: AgentAssignment[] | null = this.payload.session.agents) {
    const session = emptySession();
    if (agents?.length) {
      session.agents = clone(agents).map((agent) => ({ ...agent, status: "idle" }));
    }
    this.payload = { ...this.payload, session, telemetry: emptyTelemetry(session.id), notices: [], recoverable_checkpoint: false };
    return clone(this.payload);
  }
}

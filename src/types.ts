export type LifecyclePhase =
  | "pre_session"
  | "clarification"
  | "aspect_gate"
  | "round_running"
  | "post_round"
  | "finalized";

export type AgentRole = "orchestrator" | "member";
export type AgentStatus = "idle" | "streaming" | "complete" | "failed" | "cancelled";
export type NoticeSeverity = "info" | "warning" | "critical";

export interface TimeoutPolicy {
  connect_secs: number;
  first_token_secs: number;
  idle_stream_secs: number;
  total_secs: number;
  max_attempts: number;
}

export interface ProviderSummary {
  id: string;
  name: string;
  base_url: string;
  protocol: "open_ai" | "anthropic" | "gemini" | "demo";
  credential_status: "configured" | "untested" | "valid" | "invalid" | "not_required";
  supports_discovery: boolean;
  configurable_endpoint: boolean;
  timeout: TimeoutPolicy;
}

export interface ModelInfo {
  id: string;
  provider_id: string;
  name: string;
  context_window: number;
  input_per_million: number | null;
  output_per_million: number | null;
  supports_vision: boolean;
  supports_documents: boolean;
  supports_streaming: boolean;
  reasoning: boolean;
}

export interface Persona {
  id: string;
  name: string;
  description: string;
  system_prompt: string;
  directives: string[];
  builtin: boolean;
}

export interface AgentAssignment {
  id: string;
  role: AgentRole;
  display_name: string;
  provider_id: string;
  model_id: string;
  persona_id: string | null;
  status: AgentStatus;
}

export interface Aspect {
  id: string;
  name: string;
  description: string;
  weight: number;
}

export interface Attachment {
  id: string;
  name: string;
  path: string;
  media_type: string;
  bytes: number;
  extracted_chars: number;
  status: "ready" | "warning" | "failed";
  warning: string | null;
}

export interface ClarificationQuestion {
  id: string;
  prompt: string;
  rationale: string;
}

export interface AgentResponse {
  agent_id: string;
  content: string;
  status: AgentStatus;
  error: string | null;
  input_tokens: number | null;
  output_tokens: number | null;
  latency_ms: number;
}

export interface FrictionItem {
  id: string;
  kind: "contradiction" | "omission" | "unsupported_claim" | "consensus";
  agent_ids: string[];
  aspect_id: string | null;
  explanation: string;
  challenge: string;
}

export interface VoteDetail {
  voter_alias: string;
  score: number;
  outlier: boolean;
}

export interface ScoreCell {
  agent_id: string;
  aspect_id: string;
  median: number;
  votes: VoteDetail[];
}

export interface RoundRecord {
  index: number;
  started_at: string;
  completed_at: string | null;
  responses: AgentResponse[];
  friction: FrictionItem[];
  scores: ScoreCell[];
  user_argument: string | null;
  semantic_similarity: number | null;
  consensus: number | null;
}

export interface SessionState {
  schema_version: number;
  id: string;
  phase: LifecyclePhase;
  objective: string;
  main_phrase: string;
  ambiguity_score: number | null;
  clarification_questions: ClarificationQuestion[];
  clarification_answers: Record<string, string>;
  aspects: Aspect[];
  agents: AgentAssignment[];
  attachments: Attachment[];
  rounds: RoundRecord[];
  compacted_history: string[];
  final_synthesis: string | null;
  created_at: string;
  updated_at: string;
}

export interface ModelUsage {
  provider_id: string;
  model_id: string;
  input_tokens: number | null;
  output_tokens: number | null;
  input_cost_usd: number | null;
  output_cost_usd: number | null;
  total_cost_usd: number | null;
}

export interface TelemetrySnapshot {
  session_id: string;
  by_model: ModelUsage[];
  total_input_tokens: number | null;
  total_output_tokens: number | null;
  total_cost_usd: number | null;
}

export interface AppNotice {
  id: string;
  severity: NoticeSeverity;
  title: string;
  message: string;
  details: string | null;
  timestamp: string;
}

export interface BootstrapPayload {
  session: SessionState;
  telemetry: TelemetrySnapshot;
  providers: ProviderSummary[];
  models: ModelInfo[];
  personas: Persona[];
  notices: AppNotice[];
  recoverable_checkpoint: boolean;
  app_version: string;
}

export interface RoundPollPayload {
  session: SessionState;
  telemetry: TelemetrySnapshot;
  notices: AppNotice[];
}

export interface PreflightInput {
  objective: string;
  agents: AgentAssignment[];
  attachment_paths: string[];
}

export interface StreamChunk {
  session_id: string;
  round_index: number;
  agent_id: string;
  delta: string;
}

export interface ExportResult {
  path: string;
  bytes: number;
}

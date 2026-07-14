import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AgentAssignment,
  AppNotice,
  Aspect,
  BootstrapPayload,
  ExportResult,
  Persona,
  PreflightInput,
  RoundPollPayload,
  SessionState,
  StreamChunk,
  TelemetrySnapshot,
} from "../types";
import { createDemoBootstrap, DemoBackend } from "./demoBackend";

const isTauri = () => "__TAURI_INTERNALS__" in window;
const demo = new DemoBackend(createDemoBootstrap());

async function command<T>(name: string, args: Record<string, unknown> = {}): Promise<T> {
  if (isTauri()) return invoke<T>(name, args);
  return demo.command<T>(name, args);
}

export const backend = {
  isDesktop: isTauri,
  bootstrap: () => command<BootstrapPayload>("bootstrap"),
  roundPoll: () => command<RoundPollPayload>("round_poll"),
  startPreflight: (input: PreflightInput) => command<SessionState>("start_preflight", { input }),
  submitClarification: (answers: Record<string, string>) =>
    command<SessionState>("submit_clarification", { answers }),
  approveAspects: (aspects: Aspect[]) => command<SessionState>("approve_aspects", { aspects }),
  rejectAspects: () => command<SessionState>("reject_aspects"),
  startRound: (userArgument?: string) =>
    command<SessionState>("start_round", { userArgument: userArgument || null }),
  stopRound: () => command<SessionState>("stop_round"),
  retryAgent: (agentId: string) => command<SessionState>("retry_agent", { agentId }),
  finalizeSession: () => command<SessionState>("finalize_session"),
  newSession: (agents?: AgentAssignment[]) => command<BootstrapPayload>("new_session", { agents: agents ?? null }),
  saveCredential: (providerId: string, secret: string) =>
    command<string>("save_credential", { providerId, secret }),
  deleteCredential: (providerId: string) => command<void>("delete_credential", { providerId }),
  testConnection: (providerId: string) => command<string>("test_connection", { providerId }),
  refreshModels: (providerId: string) => command<string>("refresh_models", { providerId }),
  updateProvider: (providerId: string, baseUrl: string, timeout: unknown) =>
    command<BootstrapPayload>("update_provider", { providerId, baseUrl, timeout }),
  savePersona: (persona: Persona) => command<Persona[]>("save_persona", { persona }),
  deletePersona: (personaId: string) => command<Persona[]>("delete_persona", { personaId }),
  ingestFiles: (paths: string[]) => command<SessionState>("ingest_files", { paths }),
  importSession: (path: string) => command<SessionState>("import_session", { path }),
  exportMarkdown: (path: string) => command<ExportResult>("export_markdown", { path }),
  exportPdf: (path: string) => command<ExportResult>("export_pdf", { path }),
  restoreCheckpoint: () => command<BootstrapPayload>("restore_checkpoint"),
  discardCheckpoint: () => command<BootstrapPayload>("discard_checkpoint"),
  hardClear: () => command<BootstrapPayload>("hard_clear"),
  async onSession(callback: (value: SessionState) => void): Promise<UnlistenFn> {
    if (isTauri()) return listen<SessionState>("session://snapshot", (event) => callback(event.payload));
    return demo.listen("session://snapshot", callback);
  },
  async onChunk(callback: (value: StreamChunk) => void): Promise<UnlistenFn> {
    if (isTauri()) return listen<StreamChunk>("agent://chunk", (event) => callback(event.payload));
    return demo.listen("agent://chunk", callback);
  },
  async onTelemetry(callback: (value: TelemetrySnapshot) => void): Promise<UnlistenFn> {
    if (isTauri()) return listen<TelemetrySnapshot>("telemetry://updated", (event) => callback(event.payload));
    return demo.listen("telemetry://updated", callback);
  },
  async onNotice(callback: (value: AppNotice) => void): Promise<UnlistenFn> {
    if (isTauri()) return listen<AppNotice>("app://notice", (event) => callback(event.payload));
    return demo.listen("app://notice", callback);
  },
};

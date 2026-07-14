import { create } from "zustand";
import { backend } from "../lib/backend";
import type {
  AppNotice,
  BootstrapPayload,
  Persona,
  ProviderSummary,
  ModelInfo,
  RoundPollPayload,
  SessionState,
  StreamChunk,
  TelemetrySnapshot,
} from "../types";

interface CouncilStore {
  ready: boolean;
  busy: boolean;
  appVersion: string;
  session: SessionState | null;
  telemetry: TelemetrySnapshot | null;
  providers: ProviderSummary[];
  models: ModelInfo[];
  personas: Persona[];
  notices: AppNotice[];
  recoverableCheckpoint: boolean;
  initialize: () => Promise<void>;
  applyBootstrap: (payload: BootstrapPayload) => void;
  setSession: (session: SessionState) => void;
  appendChunk: (chunk: StreamChunk) => void;
  setTelemetry: (telemetry: TelemetrySnapshot) => void;
  applyRoundPoll: (payload: RoundPollPayload) => void;
  addNotice: (notice: AppNotice) => void;
  dismissNotice: (id: string) => void;
  setBusy: (busy: boolean) => void;
  setPersonas: (personas: Persona[]) => void;
}

export const useCouncilStore = create<CouncilStore>((set, get) => ({
  ready: false,
  busy: false,
  appVersion: "0.1.0",
  session: null,
  telemetry: null,
  providers: [],
  models: [],
  personas: [],
  notices: [],
  recoverableCheckpoint: false,
  initialize: async () => {
    const payload = await backend.bootstrap();
    get().applyBootstrap(payload);
  },
  applyBootstrap: (payload) =>
    set({
      ready: true,
      busy: false,
      appVersion: payload.app_version,
      session: payload.session,
      telemetry: payload.telemetry,
      providers: payload.providers.filter((provider) => provider.id !== "demo"),
      models: payload.models.filter((model) => model.provider_id !== "demo"),
      personas: payload.personas,
      notices: payload.notices,
      recoverableCheckpoint: payload.recoverable_checkpoint,
    }),
  setSession: (session) => set({ session, busy: session.phase === "round_running" }),
  appendChunk: (chunk) =>
    set((state) => {
      if (!state.session || state.session.id !== chunk.session_id) return state;
      const rounds = state.session.rounds.map((round) => {
        if (round.index !== chunk.round_index) return round;
        return {
          ...round,
          responses: round.responses.map((response) =>
            response.agent_id === chunk.agent_id
              ? { ...response, content: response.content + chunk.delta, status: "streaming" as const }
              : response,
          ),
        };
      });
      return { session: { ...state.session, rounds } };
    }),
  setTelemetry: (telemetry) => set({ telemetry }),
  applyRoundPoll: (payload) => set({
    session: payload.session,
    telemetry: payload.telemetry,
    notices: payload.notices,
    busy: payload.session.phase === "round_running",
  }),
  addNotice: (notice) => set((state) => ({ notices: [notice, ...state.notices].slice(0, 100) })),
  dismissNotice: (id) => set((state) => ({ notices: state.notices.filter((item) => item.id !== id) })),
  setBusy: (busy) => set({ busy }),
  setPersonas: (personas) => set({ personas }),
}));

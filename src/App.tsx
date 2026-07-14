import {
  AlertTriangle,
  ArchiveRestore,
  BarChart3,
  BookOpenText,
  BrainCircuit,
  ChevronRight,
  CircleDollarSign,
  FileDown,
  FilePlus2,
  Gauge,
  KeyRound,
  Menu,
  MessageSquareWarning,
  RotateCcw,
  Settings,
  ShieldCheck,
  Sparkles,
  Square,
  UsersRound,
} from "lucide-react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useCallback, useEffect, useMemo, useState } from "react";
import { backend } from "./lib/backend";
import { useCouncilStore } from "./store/councilStore";
import type { AppNotice, BootstrapPayload, SessionState } from "./types";
import { AspectGate } from "./components/AspectGate";
import { ClarificationPanel } from "./components/ClarificationPanel";
import { Modal } from "./components/Modal";
import { NoticeToast } from "./components/NoticeToast";
import { Roundtable } from "./components/Roundtable";
import { SessionSetup } from "./components/SessionSetup";
import { SettingsPanel } from "./components/SettingsPanel";
import { TelemetryPanel } from "./components/TelemetryPanel";

const phaseLabels: Record<SessionState["phase"], string> = {
  pre_session: "Compose",
  clarification: "Clarify",
  aspect_gate: "Approve aspects",
  round_running: "Live round",
  post_round: "Review",
  finalized: "Finalized",
};

function App() {
  const {
    ready,
    session,
    notices,
    recoverableCheckpoint,
    initialize,
    applyBootstrap,
    applyRoundPoll,
    setSession,
    appendChunk,
    setTelemetry,
    addNotice,
    dismissNotice,
  } = useCouncilStore();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [telemetryOpen, setTelemetryOpen] = useState(false);
  const [errorsOpen, setErrorsOpen] = useState(false);
  const [recoveryOpen, setRecoveryOpen] = useState(false);
  const [zoom, setZoom] = useState(() => {
    const saved = Number(window.localStorage.getItem("agentic-council-zoom"));
    return [80, 90, 100, 110, 125, 150].includes(saved) ? saved : 100;
  });

  const localNotice = useCallback((message: string, details?: string) => {
    const notice: AppNotice = {
      id: crypto.randomUUID(),
      severity: "critical",
      title: "Action failed",
      message,
      details: details ?? null,
      timestamp: new Date().toISOString(),
    };
    addNotice(notice);
  }, [addNotice]);

  useEffect(() => {
    let active = true;
    const unlisteners: Array<() => void> = [];
    void initialize()
      .then(() => {
        if (active && useCouncilStore.getState().recoverableCheckpoint) setRecoveryOpen(true);
      })
      .catch((error: unknown) => localNotice("The application could not initialize.", String(error)));
    void Promise.all([
      backend.onSession(setSession),
      backend.onChunk(appendChunk),
      backend.onTelemetry(setTelemetry),
      backend.onNotice(addNotice),
    ]).then((items) => active ? unlisteners.push(...items) : items.forEach((item) => item()));
    return () => {
      active = false;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [addNotice, appendChunk, initialize, localNotice, setSession, setTelemetry]);

  useEffect(() => {
    if (recoverableCheckpoint) setRecoveryOpen(true);
  }, [recoverableCheckpoint]);

  useEffect(() => {
    const scale = zoom / 100;
    window.localStorage.setItem("agentic-council-zoom", String(zoom));
    document.documentElement.dataset.appZoom = String(zoom);
    if (backend.isDesktop()) {
      document.documentElement.style.removeProperty("zoom");
      void getCurrentWebview().setZoom(scale).catch(() => {
        document.documentElement.style.setProperty("zoom", String(scale));
      });
    } else {
      document.documentElement.style.setProperty("zoom", String(scale));
    }
  }, [zoom]);

  useEffect(() => {
    if (!backend.isDesktop() || session?.phase !== "round_running") return;
    let active = true;
    let inFlight = false;
    let reportedFailure = false;
    const poll = async () => {
      if (!active || inFlight) return;
      inFlight = true;
      try {
        const payload = await backend.roundPoll();
        if (active) applyRoundPoll(payload);
      } catch (error) {
        if (active && !reportedFailure) {
          reportedFailure = true;
          localNotice("The live round display could not be refreshed.", String(error));
        }
      } finally {
        inFlight = false;
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 300);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [applyRoundPoll, localNotice, session?.phase]);

  const criticalCount = useMemo(() => notices.filter((notice) => notice.severity !== "info").length, [notices]);
  const latestNotice = notices[0];

  const startNewSession = async () => {
    try {
      applyBootstrap(await backend.newSession(session?.agents));
    } catch (error) {
      localNotice("A new session could not be created.", String(error));
    }
  };

  const applyRecovery = async (action: "restore" | "discard") => {
    try {
      const payload = action === "restore" ? await backend.restoreCheckpoint() : await backend.discardCheckpoint();
      applyBootstrap(payload);
      setRecoveryOpen(false);
    } catch (error) {
      localNotice("Checkpoint recovery failed.", String(error));
    }
  };

  if (!ready || !session) {
    return (
      <main className="loading-screen">
        <div className="loading-orbit"><BrainCircuit size={32} /></div>
        <h1>Preparing the council</h1>
        <p>Loading local providers, personas, and session state…</p>
      </main>
    );
  }

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand-lockup">
          <div className="brand-mark"><UsersRound size={20} /></div>
          <div>
            <strong>Agentic Council</strong>
            <span className="eyebrow">developed by <b>Parsa Farshadfar</b></span>
          </div>
        </div>
        <div className="phase-rail" aria-label="Session stage">
          <span className="phase-pulse" />
          <span>{phaseLabels[session.phase]}</span>
          {session.rounds.length > 0 && <small>Round {session.rounds.length}</small>}
        </div>
        <nav className="top-actions" aria-label="Application controls">
          <button type="button" onClick={() => void startNewSession()} title="Start a new session with the same models">
            <RotateCcw size={17} /><span>New session</span>
          </button>
          <button type="button" onClick={() => setTelemetryOpen(true)} title="Session usage and cost">
            <CircleDollarSign size={17} /><span>Usage</span>
          </button>
          <button type="button" onClick={() => setErrorsOpen(true)} title="Diagnostic notices">
            <MessageSquareWarning size={17} /><span>Log</span>
            {criticalCount > 0 && <b>{criticalCount}</b>}
          </button>
          <button type="button" onClick={() => setSettingsOpen(true)} title="Settings">
            <Settings size={17} /><span>Settings</span>
          </button>
        </nav>
      </header>

      <main className="workspace">
        {session.phase === "pre_session" && <SessionSetup onError={localNotice} />}
        {session.phase === "clarification" && <ClarificationPanel onError={localNotice} />}
        {session.phase === "aspect_gate" && <AspectGate onError={localNotice} />}
        {(session.phase === "round_running" || session.phase === "post_round" || session.phase === "finalized") && (
          <Roundtable onError={localNotice} />
        )}
      </main>

      {latestNotice && (
        <div className="toast-stack">
          <NoticeToast notice={latestNotice} onClose={() => dismissNotice(latestNotice.id)} />
        </div>
      )}

      <Modal open={telemetryOpen} onClose={() => setTelemetryOpen(false)} title="Session usage" description="Token and estimated cost totals for this session only." wide>
        <TelemetryPanel />
      </Modal>

      <Modal open={settingsOpen} onClose={() => setSettingsOpen(false)} title="Settings & credential ledger" description="Secrets are sent directly to the OS credential manager and never returned to this window." wide>
        <SettingsPanel zoom={zoom} onZoomChange={setZoom} onError={localNotice} onApplied={(payload?: BootstrapPayload) => payload && applyBootstrap(payload)} />
      </Modal>

      <Modal open={errorsOpen} onClose={() => setErrorsOpen(false)} title="Diagnostic log" description="Sanitized status events; prompt bodies and full responses are not logged." wide>
        <div className="diagnostic-list">
          {notices.length === 0 && <div className="empty-state"><ShieldCheck size={24} /><p>No notices in this session.</p></div>}
          {notices.map((notice) => (
            <article key={notice.id} className={`diagnostic-row severity-${notice.severity}`}>
              <AlertTriangle size={17} />
              <div><strong>{notice.title}</strong><p>{notice.message}</p>{notice.details && <pre>{notice.details}</pre>}</div>
              <time>{new Date(notice.timestamp).toLocaleTimeString()}</time>
            </article>
          ))}
        </div>
      </Modal>

      <Modal open={recoveryOpen} onClose={() => undefined} title="Interrupted session found" description="The last atomic checkpoint can restore the council to its most recent completed boundary.">
        <div className="recovery-card">
          <ArchiveRestore size={38} />
          <p>Any provider stream that had not completed at the time of interruption will be discarded. You can safely re-run that round.</p>
          <div className="button-row">
            <button className="button secondary" type="button" onClick={() => void applyRecovery("discard")}><RotateCcw size={16} />Discard & start fresh</button>
            <button className="button primary" type="button" onClick={() => void applyRecovery("restore")}><ChevronRight size={16} />Resume session</button>
          </div>
        </div>
      </Modal>
    </div>
  );
}

export default App;

// Keep these imports visible to the bundler's icon tree-shaking while the command palette is staged.
void [BarChart3, BookOpenText, FileDown, FilePlus2, Gauge, KeyRound, Menu, Sparkles, Square];

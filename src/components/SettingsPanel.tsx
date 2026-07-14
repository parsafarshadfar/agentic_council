import {
  CheckCircle2,
  ChevronRight,
  Eraser,
  Eye,
  EyeOff,
  KeyRound,
  MonitorUp,
  Plus,
  RefreshCw,
  Search,
  ShieldCheck,
  Trash2,
  UserRoundCog,
  XCircle,
} from "lucide-react";
import { useMemo, useState } from "react";
import { backend } from "../lib/backend";
import { sortProvidersForSelection } from "../lib/providerPresentation";
import { useCouncilStore } from "../store/councilStore";
import type { BootstrapPayload, Persona, ProviderSummary } from "../types";
import { InfoTip } from "./InfoTip";

interface SettingsPanelProps {
  zoom: number;
  onZoomChange: (zoom: number) => void;
  onError: (message: string, details?: string) => void;
  onApplied: (payload?: BootstrapPayload) => void;
}

const statusLabel: Record<ProviderSummary["credential_status"], string> = {
  configured: "Imported",
  untested: "Untested",
  valid: "Verified",
  invalid: "Invalid",
  not_required: "No key needed",
};

const zoomLevels = [80, 90, 100, 110, 125, 150];

export function SettingsPanel({ zoom, onZoomChange, onError, onApplied }: SettingsPanelProps) {
  const { providers, personas, setPersonas } = useCouncilStore();
  const [tab, setTab] = useState<"providers" | "personas" | "appearance" | "privacy">("providers");
  const [query, setQuery] = useState("");
  const [expanded, setExpanded] = useState<string | null>("openrouter");
  const [secrets, setSecrets] = useState<Record<string, string>>({});
  const [visible, setVisible] = useState<Record<string, boolean>>({});
  const [working, setWorking] = useState<string | null>(null);
  const [connectionMessage, setConnectionMessage] = useState<Record<string, string>>({});
  const [confirmWipe, setConfirmWipe] = useState(false);
  const [draftPersona, setDraftPersona] = useState<Persona | null>(null);

  const filtered = useMemo(() => sortProvidersForSelection(providers).filter((provider) => provider.name.toLowerCase().includes(query.toLowerCase()) || provider.id.includes(query.toLowerCase())), [providers, query]);

  const refresh = async () => {
    const payload = await backend.bootstrap();
    onApplied(payload);
  };

  const saveKey = async (providerId: string) => {
    const secret = secrets[providerId]?.trim();
    if (!secret) return;
    try {
      setWorking(providerId);
      const message = await backend.saveCredential(providerId, secret);
      setSecrets((current) => ({ ...current, [providerId]: "" }));
      setConnectionMessage((current) => ({ ...current, [providerId]: message }));
      await refresh();
    } catch (error) {
      onError("The credential could not be saved.", String(error));
    } finally {
      setWorking(null);
    }
  };

  const refreshModels = async (providerId: string) => {
    try {
      setWorking(providerId);
      const message = await backend.refreshModels(providerId);
      setConnectionMessage((current) => ({ ...current, [providerId]: message }));
      await refresh();
    } catch (error) {
      setConnectionMessage((current) => ({ ...current, [providerId]: String(error) }));
      onError("The provider model list could not be refreshed.", String(error));
    } finally {
      setWorking(null);
    }
  };

  const testKey = async (providerId: string) => {
    try {
      setWorking(providerId);
      const message = await backend.testConnection(providerId);
      setConnectionMessage((current) => ({ ...current, [providerId]: message }));
      await refresh();
    } catch (error) {
      setConnectionMessage((current) => ({ ...current, [providerId]: String(error) }));
      onError("Connection test failed.", String(error));
    } finally {
      setWorking(null);
    }
  };

  const deleteKey = async (providerId: string) => {
    try {
      setWorking(providerId);
      await backend.deleteCredential(providerId);
      setConnectionMessage((current) => ({ ...current, [providerId]: "Local credential removed. Revoke it separately at the provider if needed." }));
      await refresh();
    } catch (error) {
      onError("The credential could not be removed.", String(error));
    } finally {
      setWorking(null);
    }
  };

  const createPersona = () => setDraftPersona({ id: crypto.randomUUID(), name: "", description: "", system_prompt: "", directives: [""], builtin: false });
  const savePersona = async () => {
    if (!draftPersona?.name.trim() || !draftPersona.system_prompt.trim()) return;
    try {
      setPersonas(await backend.savePersona({ ...draftPersona, directives: draftPersona.directives.filter((item) => item.trim()) }));
      setDraftPersona(null);
    } catch (error) {
      onError("The persona could not be saved.", String(error));
    }
  };

  const wipe = async () => {
    try {
      const payload = await backend.hardClear();
      setSecrets({});
      setConfirmWipe(false);
      onApplied(payload);
    } catch (error) {
      onError("Local data could not be wiped completely.", String(error));
    }
  };

  return (
    <div className="settings-layout">
      <nav className="settings-tabs" aria-label="Settings sections">
        <button type="button" className={tab === "providers" ? "active" : ""} onClick={() => setTab("providers")}><KeyRound size={16} />Providers</button>
        <button type="button" className={tab === "personas" ? "active" : ""} onClick={() => setTab("personas")}><UserRoundCog size={16} />Personas</button>
        <button type="button" className={tab === "appearance" ? "active" : ""} onClick={() => setTab("appearance")}><MonitorUp size={16} />Appearance</button>
        <button type="button" className={tab === "privacy" ? "active" : ""} onClick={() => setTab("privacy")}><ShieldCheck size={16} />Privacy & reset</button>
      </nav>

      <div className="settings-content">
        {tab === "providers" && (
          <section>
            <div className="settings-section-head">
              <div><h3>Provider credentials</h3><p>Keys are write-only. Saving retrieves the provider’s non-billable model catalog where supported; verification remains a separate action.</p></div>
              <label className="search-box"><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Find provider" /></label>
            </div>
            <div className="provider-list">
              {filtered.map((provider) => {
                const isExpanded = expanded === provider.id;
                const status = provider.credential_status;
                return (
                  <article key={provider.id} className={`provider-row ${isExpanded ? "expanded" : ""}`}>
                    <button className="provider-summary" type="button" onClick={() => setExpanded(isExpanded ? null : provider.id)}>
                      <span className="provider-monogram">{provider.name.slice(0, 2).toUpperCase()}</span>
                      <span><strong>{provider.name}</strong><small>{provider.protocol.replaceAll("_", " ")} • {provider.base_url}</small></span>
                      <em className={`credential-status status-${status}`}>{status === "valid" || status === "configured" ? <CheckCircle2 size={13} /> : status === "invalid" ? <XCircle size={13} /> : null}{statusLabel[status]}</em>
                      <ChevronRight size={16} />
                    </button>
                    {isExpanded && (
                      <div className="provider-details">
                        <>
                            <label>API key <InfoTip label="Secure key storage">The plaintext value is passed directly to the Rust backend, written to Windows Credential Manager or macOS Keychain, zeroed from the request buffer, and never returned.</InfoTip>
                              <div className="secret-input">
                                <KeyRound size={15} />
                                <input type={visible[provider.id] ? "text" : "password"} value={secrets[provider.id] ?? ""} onChange={(event) => setSecrets((current) => ({ ...current, [provider.id]: event.target.value }))} placeholder="Paste a new key (existing values cannot be read)" autoComplete="off" spellCheck={false} />
                                <button type="button" onClick={() => setVisible((current) => ({ ...current, [provider.id]: !current[provider.id] }))} aria-label={visible[provider.id] ? "Hide API key" : "Show API key"}>{visible[provider.id] ? <EyeOff size={15} /> : <Eye size={15} />}</button>
                              </div>
                            </label>
                            <div className="timeout-summary"><span>Connect {provider.timeout.connect_secs}s</span><span>First token {provider.timeout.first_token_secs}s</span><span>Idle {provider.timeout.idle_stream_secs}s</span><span>Total {provider.timeout.total_secs}s</span><span>{provider.timeout.max_attempts} attempts</span></div>
                            {connectionMessage[provider.id] && <p className="connection-message">{connectionMessage[provider.id]}</p>}
                            <div className="button-row align-right">
                              <button className="button ghost danger-text" type="button" disabled={working === provider.id} onClick={() => void deleteKey(provider.id)}><Trash2 size={14} />Remove local key</button>
                              {provider.supports_discovery && <button className="button secondary" type="button" disabled={working === provider.id || status === "untested"} onClick={() => void refreshModels(provider.id)}><RefreshCw size={14} className={working === provider.id ? "spin" : ""} />Refresh models</button>}
                              <button className="button secondary" type="button" disabled={working === provider.id} onClick={() => void testKey(provider.id)}><RefreshCw size={14} className={working === provider.id ? "spin" : ""} />Test connection</button>
                              <button className="button primary" type="button" disabled={!secrets[provider.id]?.trim() || working === provider.id} onClick={() => void saveKey(provider.id)}>Save key</button>
                            </div>
                        </>
                      </div>
                    )}
                  </article>
                );
              })}
            </div>
          </section>
        )}

        {tab === "personas" && (
          <section>
            <div className="settings-section-head"><div><h3>Thinking archetypes</h3><p>Archetypes shape reasoning style without imitating a real person.</p></div><button className="button primary compact" type="button" onClick={createPersona}><Plus size={15} />New archetype</button></div>
            <div className="persona-grid">
              {personas.map((persona) => (
                <article key={persona.id}>
                  <span>{persona.builtin ? "BUILT-IN" : "CUSTOM"}</span><h4>{persona.name}</h4><p>{persona.description}</p>
                  <ul>{persona.directives.map((directive) => <li key={directive}>{directive}</li>)}</ul>
                  {!persona.builtin && <div><button className="text-button" type="button" onClick={() => setDraftPersona(persona)}>Edit</button><button className="text-button danger-text" type="button" onClick={() => void backend.deletePersona(persona.id).then(setPersonas).catch((error) => onError("Persona deletion failed.", String(error)))}>Delete</button></div>}
                </article>
              ))}
            </div>
            {draftPersona && (
              <div className="persona-editor">
                <h4>{personas.some((item) => item.id === draftPersona.id) ? "Edit archetype" : "Create archetype"}</h4>
                <div className="field-grid">
                  <label>Name<input value={draftPersona.name} onChange={(event) => setDraftPersona({ ...draftPersona, name: event.target.value })} placeholder="e.g. Evidence Auditor" /></label>
                  <label>Description<input value={draftPersona.description} onChange={(event) => setDraftPersona({ ...draftPersona, description: event.target.value })} placeholder="One-line summary" /></label>
                  <label className="full-width">System prompt<textarea rows={4} value={draftPersona.system_prompt} onChange={(event) => setDraftPersona({ ...draftPersona, system_prompt: event.target.value })} placeholder="Describe the cognitive lens, priorities, and boundaries…" /></label>
                  <label className="full-width">Behavioral directives<textarea rows={3} value={draftPersona.directives.join("\n")} onChange={(event) => setDraftPersona({ ...draftPersona, directives: event.target.value.split("\n") })} placeholder="One directive per line" /></label>
                </div>
                <div className="button-row align-right"><button className="button secondary" type="button" onClick={() => setDraftPersona(null)}>Cancel</button><button className="button primary" type="button" disabled={!draftPersona.name.trim() || !draftPersona.system_prompt.trim()} onClick={() => void savePersona()}>Save archetype</button></div>
              </div>
            )}
          </section>
        )}

        {tab === "appearance" && (
          <section>
            <div className="settings-section-head"><div><h3>Interface zoom</h3><p>Scale the complete app like browser zoom. Layout breakpoints and overlays adapt to the effective viewport.</p></div></div>
            <div className="zoom-settings">
              <div><MonitorUp size={25} /><span><strong>{zoom}%</strong><small>Current zoom</small></span></div>
              <div className="zoom-options" role="group" aria-label="Interface zoom level">
                {zoomLevels.map((level) => <button type="button" className={zoom === level ? "active" : ""} key={level} onClick={() => onZoomChange(level)}>{level}%</button>)}
              </div>
              <p>Your selection is saved on this device and applied through the native webview zoom engine.</p>
            </div>
          </section>
        )}

        {tab === "privacy" && (
          <section className="privacy-settings">
            <div className="privacy-hero"><ShieldCheck size={28} /><div><h3>Local processing boundary</h3><p>There is no analytics, telemetry service, or application cloud. Content leaves this device only in requests to providers you assign to council seats.</p></div></div>
            <div className="privacy-grid">
              <article><strong>Credentials</strong><p>OS-native credential manager only; never stored in webview storage, sessions, checkpoints, or logs.</p></article>
              <article><strong>Session files</strong><p>Local application data, unencrypted, protected by your OS account and full-disk encryption.</p></article>
              <article><strong>Diagnostics</strong><p>Keys are masked, prompts are truncated after 50 characters, and model response text is never logged.</p></article>
              <article><strong>Network</strong><p>TLS requests only to configured provider endpoints and optional provider model-roster endpoints.</p></article>
            </div>
            <div className="danger-zone">
              <div><span><Eraser size={18} />DESTRUCTIVE LOCAL ACTION</span><h4>Wipe credentials & clear cache</h4><p>This does not revoke provider-side keys. Revoke those separately from each provider dashboard.</p></div>
              {!confirmWipe ? <button className="button danger" type="button" onClick={() => setConfirmWipe(true)}>Review wipe scope</button> : (
                <div className="wipe-confirm">
                  <strong>The following local data will be permanently deleted:</strong>
                  <ul><li>All OS-keychain API key entries managed by Agentic Council</li><li>Cached model rosters and pricing tables</li><li>Document extraction and OCR temporary files</li><li>Session state, checkpoints, transcripts, and compacted history</li><li>Export caches and Typst intermediates</li><li>Sanitized diagnostic logs and in-memory credential references</li></ul>
                  <div className="button-row align-right"><button className="button secondary" type="button" onClick={() => setConfirmWipe(false)}>Cancel</button><button className="button danger" type="button" onClick={() => void wipe()}><Eraser size={15} />Wipe local data</button></div>
                </div>
              )}
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

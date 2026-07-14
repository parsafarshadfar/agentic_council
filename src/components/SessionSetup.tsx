import { open } from "@tauri-apps/plugin-dialog";
import {
  BrainCircuit,
  Check,
  ChevronDown,
  FileInput,
  FilePlus2,
  ImagePlus,
  Import,
  Plus,
  Search,
  ShieldCheck,
  Trash2,
  UsersRound,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { backend } from "../lib/backend";
import { councilorLabel } from "../lib/agentPresentation";
import {
  modelMatchesSearch,
  providerOptionLabel,
  providerVerification,
  sortProvidersForSelection,
} from "../lib/providerPresentation";
import { useCouncilStore } from "../store/councilStore";
import type { AgentAssignment } from "../types";
import { InfoTip } from "./InfoTip";

const supportedExtensions = ["pdf", "txt", "md", "csv", "json", "docx", "png", "jpg", "jpeg", "webp"];

export function SessionSetup({ onError }: { onError: (message: string, details?: string) => void }) {
  const { session, providers, models, personas, busy, setSession, setBusy } = useCouncilStore();
  const [objective, setObjective] = useState(session?.objective ?? "");
  const [attachmentPaths, setAttachmentPaths] = useState<string[]>([]);
  const [dragging, setDragging] = useState(false);
  const [modelQueries, setModelQueries] = useState<Record<string, string>>({});
  const [openModelPicker, setOpenModelPicker] = useState<string | null>(null);
  const [pendingSeatScroll, setPendingSeatScroll] = useState<string | null>(null);
  const agentListRef = useRef<HTMLDivElement>(null);

  const agents = session?.agents ?? [];
  const members = agents.filter((agent) => agent.role === "member");
  const orchestrators = agents.filter((agent) => agent.role === "orchestrator");
  const quorum = orchestrators.length === 1 && members.length >= 2;
  const canStart = objective.trim().length >= 12 && quorum;
  const configuredProviders = useMemo(() => new Map(providers.map((provider) => [provider.id, provider])), [providers]);
  const sortedProviders = useMemo(() => sortProvidersForSelection(providers), [providers]);

  useEffect(() => {
    const fallbackProvider = sortedProviders[0];
    if (!fallbackProvider) return;
    if (!session) return;
    let changed = false;
    const next = agents.map((agent) => {
        const providerId = sortedProviders.some((provider) => provider.id === agent.provider_id)
          ? agent.provider_id
          : fallbackProvider.id;
        const available = models.filter((model) => model.provider_id === providerId);
        const modelId = available.some((model) => model.id === agent.model_id)
          ? agent.model_id
          : (available[0]?.id ?? "");
        if (providerId === agent.provider_id && modelId === agent.model_id) return agent;
        changed = true;
        return { ...agent, provider_id: providerId, model_id: modelId };
    });
    if (changed) setSession({ ...session, agents: next });
  }, [agents, models, session, setSession, sortedProviders]);

  useEffect(() => {
    if (!pendingSeatScroll) return;
    const frame = window.requestAnimationFrame(() => {
      const list = agentListRef.current;
      if (!list?.querySelector(`[data-agent-id="${pendingSeatScroll}"]`)) return;
      list.scrollTo({ top: list.scrollHeight, behavior: "smooth" });
      setPendingSeatScroll(null);
    });
    return () => window.cancelAnimationFrame(frame);
  }, [agents, pendingSeatScroll]);

  const updateAgent = (id: string, patch: Partial<AgentAssignment>) => {
    if (patch.provider_id) {
      setModelQueries((queries) => ({ ...queries, [id]: "" }));
    }
    if (!session) return;
    const nextAgents = agents.map((agent) => {
      if (agent.id !== id) return agent;
      const next = { ...agent, ...patch };
      if (patch.provider_id) {
        next.model_id = models.find((model) => model.provider_id === patch.provider_id)?.id ?? "";
      }
      return next;
    });
    setSession({ ...session, agents: nextAgents });
  };

  const addMember = () => {
    const index = members.length + 1;
    const providerId = sortedProviders[0]?.id ?? "";
    if (!session) return;
    const id = crypto.randomUUID();
    setSession({ ...session, agents: [...agents, {
      id,
      role: "member",
      display_name: `Councilor ${index}`,
      provider_id: providerId,
      model_id: models.find((model) => model.provider_id === providerId)?.id ?? "",
      persona_id: null,
      status: "idle",
    }] });
    setPendingSeatScroll(id);
  };

  const chooseFiles = async () => {
    if (!backend.isDesktop()) {
      onError("Attachments require the desktop build.", "The browser preview cannot access native file paths.");
      return;
    }
    try {
      const selected = await open({
        multiple: true,
        title: "Add context files",
        filters: [{ name: "Council context", extensions: supportedExtensions }],
      });
      if (selected) setAttachmentPaths((paths) => [...new Set([...paths, ...(Array.isArray(selected) ? selected : [selected])])]);
    } catch (error) {
      onError("The file picker could not be opened.", String(error));
    }
  };

  const importSession = async () => {
    if (!backend.isDesktop()) {
      onError("Session import requires the desktop build.");
      return;
    }
    try {
      const selected = await open({ multiple: false, filters: [{ name: "Agentic Council session", extensions: ["md"] }] });
      if (typeof selected === "string") setSession(await backend.importSession(selected));
    } catch (error) {
      onError("The session could not be imported.", String(error));
    }
  };

  const start = async () => {
    if (!canStart) return;
    setBusy(true);
    try {
      const next = await backend.startPreflight({ objective: objective.trim(), agents, attachment_paths: attachmentPaths });
      setSession(next);
    } catch (error) {
      setBusy(false);
      onError("Pre-flight analysis failed.", String(error));
    }
  };

  const acceptDroppedFiles = (event: React.DragEvent) => {
    event.preventDefault();
    setDragging(false);
    onError("Use “Add files” in the desktop build so the secure backend receives native paths.");
  };

  return (
    <div className="setup-layout">
      <section className="hero-column">
        <div className="hero-copy">
          <h1><strong>Your APIs. Your brainstorming rules!</strong><span>The Open-Source Alternative to Multi-Model Brainstorming Feature of Perplexity.</span></h1>
          <p>Put your question on the table. Your models independently generate, critique, and challenge each other's reasoning in real-time.</p>
        </div>

        <div className="prompt-card">
          <div className="prompt-label">
            <label htmlFor="objective">Council objective</label>
            <span>{objective.length.toLocaleString()} chars</span>
          </div>
          <textarea
            id="objective"
            value={objective}
            onChange={(event) => setObjective(event.target.value)}
            placeholder="Describe the decision, constraints, desired outcome, and what a useful answer must include…"
            rows={8}
            autoFocus
          />
          <div
            className={`drop-zone ${dragging ? "dragging" : ""}`}
            onDragOver={(event) => { event.preventDefault(); setDragging(true); }}
            onDragLeave={() => setDragging(false)}
            onDrop={acceptDroppedFiles}
          >
            <div><FilePlus2 size={17} /><span>{attachmentPaths.length ? `${attachmentPaths.length} file${attachmentPaths.length === 1 ? "" : "s"} selected` : "Add evidence and context"}</span></div>
            <button type="button" className="text-button" onClick={() => void chooseFiles()}>Add files</button>
          </div>
          {attachmentPaths.length > 0 && (
            <ul className="attachment-list">
              {attachmentPaths.map((path) => (
                <li key={path}><FileInput size={14} /><span>{path.split(/[\\/]/).at(-1)}</span><button type="button" onClick={() => setAttachmentPaths((items) => items.filter((item) => item !== path))} aria-label="Remove attachment"><Trash2 size={13} /></button></li>
              ))}
            </ul>
          )}
          <div className="prompt-footer">
            <span><ShieldCheck size={14} /> Files are extracted locally</span>
            <button type="button" className="text-button" onClick={() => void importSession()}><Import size={14} />Import MD session</button>
          </div>
        </div>
      </section>

      <aside className="council-builder">
        <div className="builder-head">
          <div>
            <span className="section-kicker"><UsersRound size={14} /> COUNCIL ROSTER</span>
            <h2>Seat the council</h2>
          </div>
          <span className={`quorum-badge ${quorum ? "valid" : ""}`}>{members.length} / 2 minimum</span>
        </div>
        <p className="builder-intro">Assign a model and optional thinking archetype to each seat. Model clones are welcome.</p>

        <div className="agent-config-list" ref={agentListRef}>
          {agents.map((agent) => {
            const provider = configuredProviders.get(agent.provider_id);
            const verification = provider ? providerVerification(provider) : null;
            const modelQuery = modelQueries[agent.id] ?? "";
            const availableModels = models.filter((model) => model.provider_id === agent.provider_id);
            const matchingModels = availableModels.filter((model) => modelMatchesSearch(model, modelQuery));
            const selectedModel = availableModels.find((model) => model.id === agent.model_id);
            const modelPickerOpen = openModelPicker === agent.id;
            return (
              <article className={`agent-config ${agent.role}`} key={agent.id} data-agent-id={agent.id}>
                <header>
                  <span className="agent-index">{agent.role === "orchestrator" ? "M" : members.findIndex((item) => item.id === agent.id) + 1}</span>
                  <div><strong>{agent.role === "orchestrator" ? "Orchestrator" : councilorLabel(agent.display_name)}</strong><small>{agent.role === "orchestrator" ? "Moderator & synthesis" : "Independent generation & peer vote"}</small></div>
                  {agent.role === "member" && members.length > 2 && <button className="icon-button small" type="button" onClick={() => session && setSession({ ...session, agents: agents.filter((item) => item.id !== agent.id) })} aria-label={`Remove ${councilorLabel(agent.display_name)}`}><Trash2 size={14} /></button>}
                </header>
                <div className="field-grid">
                  <label><span className="field-label-line">Provider
                    {verification && verification.kind !== "other" && <em className={`provider-choice-status ${verification.kind}`}>{verification.mark} {verification.label}</em>}
                  </span>
                    <select className={`provider-picker provider-${verification?.kind ?? "other"}`} value={agent.provider_id} onChange={(event) => updateAgent(agent.id, { provider_id: event.target.value })}>
                      {sortedProviders.map((item) => {
                        const status = providerVerification(item);
                        return <option className={`provider-option ${status.kind}`} key={item.id} value={item.id}>{providerOptionLabel(item)}</option>;
                      })}
                    </select>
                  </label>
                  <label>Model
                    <div
                      className={`model-combobox ${modelPickerOpen ? "open" : ""}`}
                      onBlur={(event) => {
                        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
                          setOpenModelPicker((current) => current === agent.id ? null : current);
                          setModelQueries((queries) => ({ ...queries, [agent.id]: "" }));
                        }
                      }}
                    >
                      <div className="model-combobox-input">
                        <Search size={13} />
                        <input
                          type="search"
                          role="combobox"
                          aria-expanded={modelPickerOpen}
                          aria-controls={`model-options-${agent.id}`}
                          aria-autocomplete="list"
                          value={modelPickerOpen ? modelQuery : (selectedModel?.name ?? "")}
                          onFocus={() => {
                            setOpenModelPicker(agent.id);
                            setModelQueries((queries) => ({ ...queries, [agent.id]: "" }));
                          }}
                          onChange={(event) => {
                            setOpenModelPicker(agent.id);
                            setModelQueries((queries) => ({ ...queries, [agent.id]: event.target.value }));
                          }}
                          onKeyDown={(event) => {
                            if (event.key === "Escape") {
                              setOpenModelPicker(null);
                              setModelQueries((queries) => ({ ...queries, [agent.id]: "" }));
                              event.currentTarget.blur();
                            } else if (event.key === "Enter" && matchingModels[0]) {
                              event.preventDefault();
                              updateAgent(agent.id, { model_id: matchingModels[0].id });
                              setOpenModelPicker(null);
                              setModelQueries((queries) => ({ ...queries, [agent.id]: "" }));
                            }
                          }}
                          placeholder="Search or choose a model"
                          aria-label={`Search and choose a model for ${councilorLabel(agent.display_name)}`}
                        />
                        <ChevronDown size={14} />
                      </div>
                      {modelPickerOpen && (
                        <div className="model-options" id={`model-options-${agent.id}`} role="listbox">
                          {matchingModels.length === 0 && <span className="model-option-empty">No models match this search</span>}
                          {matchingModels.map((model) => (
                            <button
                              type="button"
                              role="option"
                              aria-selected={model.id === agent.model_id}
                              className={`model-option ${model.id === agent.model_id ? "selected" : ""}`}
                              key={model.id}
                              onMouseDown={(event) => event.preventDefault()}
                              onClick={() => {
                                updateAgent(agent.id, { model_id: model.id });
                                setOpenModelPicker(null);
                                setModelQueries((queries) => ({ ...queries, [agent.id]: "" }));
                              }}
                            >
                              <span><strong>{model.name}</strong><small>{model.id}</small></span>
                              {model.id === agent.model_id && <Check size={14} />}
                            </button>
                          ))}
                        </div>
                      )}
                    </div>
                  </label>
                  <label className="full-width">Persona <InfoTip label="Model clones">Use the same model in multiple seats with different archetypes to elicit independent cognitive lenses.</InfoTip>
                    <select value={agent.persona_id ?? ""} onChange={(event) => updateAgent(agent.id, { persona_id: event.target.value || null })}>
                      <option value="">No persona — model default</option>
                      {personas.map((persona) => <option key={persona.id} value={persona.id}>{persona.name}</option>)}
                    </select>
                  </label>
                </div>
                {provider && provider.credential_status !== "valid" && provider.credential_status !== "not_required" && (
                  <span className="credential-note">Key {provider.credential_status}. Configure it in Settings before a live round.</span>
                )}
              </article>
            );
          })}
        </div>

        {members.length < 8 && <button type="button" className="add-seat" onClick={addMember}><Plus size={16} />Add council seat</button>}
        <div className="builder-actions">
          <div className="quorum-help">
            <InfoTip label="Minimum quorum">A valid session needs exactly one Orchestrator and at least two councilors so independent comparison and peer scoring are meaningful.</InfoTip>
            <span>{quorum ? "Quorum ready" : "Add at least two councilors"}</span>
          </div>
          <button className="button primary start-button" type="button" disabled={!canStart || busy} onClick={() => void start()}>
            {busy ? "Orchestrator analyzing…" : "Convene council"} <BrainCircuit size={17} />
          </button>
        </div>
      </aside>
    </div>
  );
}

void ImagePlus;

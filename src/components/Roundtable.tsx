import { save } from "@tauri-apps/plugin-dialog";
import {
  AlertTriangle,
  ArrowRight,
  BrainCircuit,
  CheckCircle2,
  Download,
  FileText,
  Flag,
  Gauge,
  GitCompareArrows,
  LoaderCircle,
  MessageSquarePlus,
  RotateCcw,
  Sparkles,
  Square,
  Target,
  UsersRound,
  Zap,
} from "lucide-react";
import { useMemo, useState } from "react";
import {
  PolarAngleAxis,
  PolarGrid,
  PolarRadiusAxis,
  Radar,
  RadarChart,
  ResponsiveContainer,
  Tooltip,
} from "recharts";
import { backend } from "../lib/backend";
import { councilorLabel } from "../lib/agentPresentation";
import { sessionExportBasename } from "../lib/exportFilename";
import { useCouncilStore } from "../store/councilStore";
import type { AgentAssignment, ExportResult, FrictionItem, RoundRecord, ScoreCell } from "../types";
import { InfoTip } from "./InfoTip";
import { MarkdownContent } from "./MarkdownContent";

const frictionMeta: Record<FrictionItem["kind"], { label: string; icon: typeof Zap }> = {
  contradiction: { label: "Contradiction", icon: Zap },
  omission: { label: "Gap detected", icon: Target },
  unsupported_claim: { label: "Unsubstantiated", icon: AlertTriangle },
  consensus: { label: "Consensus", icon: CheckCircle2 },
};

const colors = ["#8b7cf8", "#3cd9bd", "#f1ad5b", "#ef718b", "#65a9ff", "#bc79f5"];

export function Roundtable({ onError }: { onError: (message: string, details?: string) => void }) {
  const { session, personas, models, setSession, applyBootstrap } = useCouncilStore();
  const [argument, setArgument] = useState("");
  const [exporting, setExporting] = useState(false);
  const [selectedScore, setSelectedScore] = useState<ScoreCell | null>(null);
  if (!session) return null;
  const members = session.agents.filter((agent) => agent.role === "member");
  const latestRound = session.rounds.at(-1);

  const runRound = async () => {
    try {
      setSession(await backend.startRound(argument.trim() || undefined));
      setArgument("");
    } catch (error) {
      onError("The round could not be started.", String(error));
    }
  };

  const stop = async () => {
    try {
      setSession(await backend.stopRound());
    } catch (error) {
      onError("The round could not be stopped cleanly.", String(error));
    }
  };

  const finalize = async () => {
    try {
      setSession(await backend.finalizeSession());
    } catch (error) {
      onError("Final synthesis failed.", String(error));
    }
  };

  const exportSession = async (format: "md" | "pdf") => {
    if (!backend.isDesktop()) {
      onError("Native exports require the Tauri desktop build.");
      return;
    }
    try {
      setExporting(true);
      const basename = sessionExportBasename(session);
      const path = await save({
        title: `Export Agentic Council ${format.toUpperCase()}`,
        defaultPath: `${basename}.${format}`,
        filters: [{ name: format === "md" ? "Markdown" : "PDF", extensions: [format] }],
      });
      if (!path) return;
      const result: ExportResult = format === "md" ? await backend.exportMarkdown(path) : await backend.exportPdf(path);
      void result;
    } catch (error) {
      onError(`The ${format.toUpperCase()} report could not be exported.`, String(error));
    } finally {
      setExporting(false);
    }
  };

  const newSession = async () => {
    try {
      applyBootstrap(await backend.newSession(session.agents));
    } catch (error) {
      onError("A new session could not be created.", String(error));
    }
  };

  const modelName = (agent: AgentAssignment) => models.find((model) => model.id === agent.model_id && model.provider_id === agent.provider_id)?.name ?? agent.model_id;
  const personaName = (agent: AgentAssignment) => personas.find((persona) => persona.id === agent.persona_id)?.name;
  const agentName = (agent: AgentAssignment) => councilorLabel(agent.display_name);

  return (
    <div className="roundtable-shell">
      <section className="session-banner">
        <div>
          <span className="section-kicker"><Flag size={13} /> COUNCIL OBJECTIVE</span>
          <h1>{session.objective.split("\n")[0]}</h1>
        </div>
        <div className="banner-actions">
          <button className="button secondary compact" type="button" disabled={exporting} onClick={() => void exportSession("md")}><FileText size={15} />MD</button>
          <button className="button secondary compact" type="button" disabled={exporting} onClick={() => void exportSession("pdf")}><Download size={15} />PDF</button>
          {session.phase === "round_running" && <button className="button danger compact" type="button" onClick={() => void stop()}><Square size={14} fill="currentColor" />Stop round</button>}
        </div>
      </section>

      {!latestRound && (
        <section className="ready-table">
          <div className="table-visual">
            <div className="seat moderator-seat"><BrainCircuit size={22} /><span>Moderator</span></div>
            <div className="table-core"><UsersRound size={42} /><strong>{members.length} perspectives</strong><span>{session.aspects.length} approved aspects</span></div>
            {members.map((agent, index) => <div key={agent.id} className={`seat member-seat seat-${index + 1}`}><span>{index + 1}</span><strong>{personaName(agent) ?? agentName(agent)}</strong><small>{modelName(agent)}</small></div>)}
          </div>
          <div className="ready-copy">
            <span className="section-kicker"><Sparkles size={13} /> BRIEF APPROVED</span>
            <h2>The table is set.</h2>
            <p>Councilors generate independently and in parallel. After all complete, the moderator identifies friction and each councilor scores every other response from 0 to 10. You can inspect who cast every vote.</p>
            <button className="button primary" type="button" onClick={() => void runRound()}>Begin round one<ArrowRight size={17} /></button>
          </div>
        </section>
      )}

      {latestRound && (
        <>
          <section className="live-grid">
            <div className="moderator-strip">
              <div className="moderator-avatar"><BrainCircuit size={22} /></div>
              <div><span>ORCHESTRATOR</span><strong>{session.phase === "round_running" ? "Independent generation in progress" : `Round ${latestRound.index} review`}</strong></div>
              <div className="stage-chips">
                <span className={session.phase === "round_running" ? "active" : "done"}>1 Generate</span>
                <span className={latestRound.friction.length ? "done" : ""}>2 Analyze</span>
                <span className={latestRound.scores.length ? "done" : ""}>3 Score</span>
              </div>
            </div>
            {members.map((agent, index) => {
              const response = latestRound.responses.find((item) => item.agent_id === agent.id);
              return (
                <article className="response-panel" key={agent.id} id={`agent-${agent.id}`} style={{ "--agent-color": colors[index % colors.length] } as React.CSSProperties}>
                  <header>
                    <span className="response-avatar">{index + 1}</span>
                    <div><strong>{agentName(agent)}</strong><small>{modelName(agent)}{personaName(agent) ? ` • ${personaName(agent)}` : ""}</small></div>
                    <span className={`status-pill status-${response?.status ?? agent.status}`}>
                      {response?.status === "streaming" && <LoaderCircle size={12} className="spin" />}{response?.status ?? agent.status}
                    </span>
                  </header>
                  <div className="response-content">
                    {response?.content ? <MarkdownContent content={response.content} /> : <span className="waiting-text">Waiting for first token…</span>}
                  </div>
                  <footer>
                    <span>{response?.output_tokens == null ? "Tokens N/A" : `${response.output_tokens.toLocaleString()} output tokens`}</span>
                    <span>{response?.latency_ms ? `${(response.latency_ms / 1000).toFixed(1)}s` : "Live"}</span>
                    {(response?.status === "failed" || response?.status === "cancelled") && <button type="button" onClick={() => void backend.retryAgent(agent.id).then(setSession).catch((error) => onError("Agent retry failed.", String(error)))}><RotateCcw size={13} />Retry</button>}
                  </footer>
                </article>
              );
            })}
          </section>

          {session.phase !== "round_running" && latestRound.friction.length > 0 && (
            <FrictionBoard round={latestRound} agents={members} />
          )}

          {session.phase !== "round_running" && latestRound.scores.length > 0 && (
            <ScoreBoard round={latestRound} agents={members} selected={selectedScore} onSelect={setSelectedScore} />
          )}

          {session.phase === "post_round" && (
            <section className="command-center">
              <div className="command-title">
                <span className="section-kicker"><MessageSquarePlus size={13} /> POST-ROUND COMMAND CENTER</span>
                <h2>Direct the next move.</h2>
              </div>
              <label className="argument-box">
                <span>Inject an argument or constraint into round {latestRound.index + 1}</span>
                <textarea rows={2} value={argument} onChange={(event) => setArgument(event.target.value)} placeholder="Optional: challenge an assumption, add evidence, or change the emphasis…" />
              </label>
              <div className="command-actions">
                <button type="button" className="button secondary" onClick={() => void finalize()}><Sparkles size={16} />Finalize council</button>
                <button type="button" className="button primary" onClick={() => void runRound()}>Run round {latestRound.index + 1}<ArrowRight size={16} /></button>
              </div>
            </section>
          )}

          {session.phase === "finalized" && (
            <section className="final-card">
              <div className="final-icon"><CheckCircle2 size={28} /></div>
              <div><span className="section-kicker">COUNCIL FROZEN</span><h2>Deliberation complete.</h2><MarkdownContent className="final-synthesis" content={session.final_synthesis || "The complete transcript, friction record, and scoring provenance are ready to export."} /></div>
              <div className="command-actions"><button className="button secondary" type="button" onClick={() => void exportSession("pdf")}><Download size={16} />Export PDF</button><button className="button primary" type="button" onClick={() => void newSession()}><RotateCcw size={16} />New session</button></div>
            </section>
          )}
        </>
      )}
    </div>
  );
}

function FrictionBoard({ round, agents }: { round: RoundRecord; agents: AgentAssignment[] }) {
  return (
    <section className="friction-board">
      <header><div><span className="section-kicker"><GitCompareArrows size={13} /> MODERATOR ANALYSIS</span><h2>Friction worth resolving</h2></div><span>{round.friction.length} items</span></header>
      <div className="friction-grid">
        {round.friction.map((item) => {
          const meta = frictionMeta[item.kind];
          const Icon = meta.icon;
          return (
            <article key={item.id} className={`friction-card friction-${item.kind}`}>
              <div className="friction-badge"><Icon size={14} />{meta.label}</div>
              <p>{item.explanation}</p>
              {item.agent_ids.length > 0 && <div className="agent-links">{item.agent_ids.map((id) => <a href={`#agent-${id}`} key={id}>{councilorLabel(agents.find((agent) => agent.id === id)?.display_name ?? "Response")}</a>)}</div>}
              <blockquote>{item.challenge}</blockquote>
            </article>
          );
        })}
      </div>
    </section>
  );
}

function ScoreBoard({ round, agents, selected, onSelect }: { round: RoundRecord; agents: AgentAssignment[]; selected: ScoreCell | null; onSelect: (score: ScoreCell | null) => void }) {
  const { session } = useCouncilStore();
  const chartData = useMemo(() => session?.aspects.map((aspect) => ({
    aspect: aspect.name.length > 18 ? `${aspect.name.slice(0, 17)}…` : aspect.name,
    fullAspect: aspect.name,
    ...Object.fromEntries(agents.map((agent) => [agent.id, round.scores.find((score) => score.agent_id === agent.id && score.aspect_id === aspect.id)?.median ?? 0])),
  })) ?? [], [agents, round.scores, session?.aspects]);
  const breakdown = selected ?? round.scores[0] ?? null;
  const selectedAgent = agents.find((agent) => agent.id === breakdown?.agent_id);
  const selectedAspect = session?.aspects.find((aspect) => aspect.id === breakdown?.aspect_id);

  return (
    <section className="score-board">
      <header>
        <div><span className="section-kicker"><Gauge size={13} /> BLIND PEER SCORING</span><h2>Where the arguments landed <InfoTip label="How median scoring works">Each answer's score is the median of the scores given by all other council models. Models review anonymized answers and never score their own response.</InfoTip></h2></div>
        <div className="metric-pills">
          <span><b>{round.semantic_similarity?.toFixed(0)}%</b> semantic overlap <InfoTip label="Semantic duplicate detection">Estimates how much responses repeat one another. High overlap can signal that more diverse models or personas are needed.</InfoTip></span>
          <span><b>{round.consensus?.toFixed(0)}%</b> consensus <InfoTip label="Consensus level">Derived from score variance. High consensus indicates alignment; low consensus identifies contested dimensions.</InfoTip></span>
        </div>
      </header>
      <div className="score-content">
        <div className="radar-wrap">
          <ResponsiveContainer width="100%" height={380}>
            <RadarChart data={chartData} outerRadius="72%">
              <PolarGrid stroke="rgba(160,174,210,.2)" />
              <PolarAngleAxis dataKey="aspect" tick={{ fill: "#9aa6c4", fontSize: 11 }} />
              <PolarRadiusAxis angle={25} domain={[0, 10]} tick={false} axisLine={false} />
              {agents.map((agent, index) => <Radar key={agent.id} name={councilorLabel(agent.display_name)} dataKey={agent.id} stroke={colors[index % colors.length]} fill={colors[index % colors.length]} fillOpacity={0.1} strokeWidth={2} />)}
              <Tooltip contentStyle={{ background: "#111628", border: "1px solid rgba(255,255,255,.12)", borderRadius: 12 }} />
            </RadarChart>
          </ResponsiveContainer>
          <div className="chart-legend">{agents.map((agent, index) => <span key={agent.id}><i style={{ background: colors[index % colors.length] }} />{councilorLabel(agent.display_name)}</span>)}</div>
        </div>
        <div className="score-matrix-wrap">
          <table className="score-matrix">
            <thead><tr><th>Aspect (median / 10)</th>{agents.map((agent) => <th key={agent.id}>{councilorLabel(agent.display_name)}</th>)}</tr></thead>
            <tbody>{session?.aspects.map((aspect) => <tr key={aspect.id}><th>{aspect.name}</th>{agents.map((agent) => { const score = round.scores.find((item) => item.aspect_id === aspect.id && item.agent_id === agent.id); return <td key={agent.id}><button type="button" className={breakdown === score ? "selected" : ""} onClick={() => score && onSelect(score)}>{score?.median.toFixed(1) ?? "—"}</button></td>; })}</tr>)}</tbody>
          </table>
          {breakdown && (
            <div className="vote-breakdown">
              <header><div><span>VOTE BREAKDOWN</span><strong>{selectedAgent ? councilorLabel(selectedAgent.display_name) : "Unknown councilor"} • {selectedAspect?.name}</strong></div><b>{breakdown.median.toFixed(1)}<small>/ 10</small></b></header>
              {breakdown.votes.length === 0 ? <p>No eligible peer votes.</p> : breakdown.votes.map((vote, index) => { const voterLabel = councilorLabel(vote.voter_alias); return <div key={`${vote.voter_alias}-${index}`}><span title={voterLabel}>{voterLabel}</span><div><i style={{ width: `${vote.score * 10}%` }} /></div><b>{vote.score.toFixed(1)} / 10</b>{vote.outlier && <em>outlier</em>}</div>; })}
              <small>Models see anonymous responses and cannot score themselves. You can see which model cast every vote.</small>
            </div>
          )}
        </div>
      </div>
    </section>
  );
}

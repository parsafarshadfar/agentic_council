import { ArrowLeft, ArrowRight, HelpCircle, Sparkles } from "lucide-react";
import { useState } from "react";
import { backend } from "../lib/backend";
import { useCouncilStore } from "../store/councilStore";
import { InfoTip } from "./InfoTip";

export function ClarificationPanel({ onError }: { onError: (message: string, details?: string) => void }) {
  const { session, setSession } = useCouncilStore();
  const [answers, setAnswers] = useState<Record<string, string>>(session?.clarification_answers ?? {});
  const [submitting, setSubmitting] = useState(false);
  if (!session) return null;
  const complete = session.clarification_questions.every((question) => answers[question.id]?.trim());

  const submit = async () => {
    setSubmitting(true);
    try {
      setSession(await backend.submitClarification(answers));
    } catch (error) {
      onError("Clarifications could not be submitted.", String(error));
    } finally {
      setSubmitting(false);
    }
  };

  const back = async () => {
    try {
      setSession(await backend.rejectAspects());
    } catch (error) {
      onError("Could not return to the objective.", String(error));
    }
  };

  return (
    <section className="gate-shell narrow-gate">
      <header className="gate-header">
        <span className="gate-icon"><HelpCircle /></span>
        <div>
          <span className="section-kicker">PREFLIGHT • CLARITY CHECK</span>
          <h1>A sharper brief will save a noisy round.</h1>
          <p>The moderator found a few gaps that could produce generic or incompatible answers.</p>
        </div>
        <div className="score-orb">
          <strong>{session.ambiguity_score ?? 0}</strong><span>/ 100 clarity</span>
          <InfoTip label="Ambiguity score">The Orchestrator's initial assessment of prompt clarity. Lower clarity triggers questions before agent tokens are spent.</InfoTip>
        </div>
      </header>

      <div className="objective-recap"><Sparkles size={16} /><p>{session.objective}</p></div>
      <div className="question-stack">
        {session.clarification_questions.map((question, index) => (
          <label key={question.id} className="question-card">
            <span><b>{String(index + 1).padStart(2, "0")}</b><span><strong>{question.prompt}</strong><small>{question.rationale}</small></span></span>
            <textarea rows={3} value={answers[question.id] ?? ""} onChange={(event) => setAnswers((current) => ({ ...current, [question.id]: event.target.value }))} placeholder="Give a concrete, decision-relevant answer…" />
          </label>
        ))}
      </div>
      <footer className="gate-actions">
        <button className="button secondary" type="button" onClick={() => void back()}><ArrowLeft size={16} />Revise objective</button>
        <button className="button primary" type="button" disabled={!complete || submitting} onClick={() => void submit()}>{submitting ? "Orchestrator refining…" : "Generate aspects"}<ArrowRight size={16} /></button>
      </footer>
    </section>
  );
}

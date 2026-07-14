import { ArrowLeft, Check, Plus, SlidersHorizontal, Trash2 } from "lucide-react";
import { useState } from "react";
import { backend } from "../lib/backend";
import { useCouncilStore } from "../store/councilStore";
import type { Aspect } from "../types";
import { InfoTip } from "./InfoTip";

export function AspectGate({ onError }: { onError: (message: string, details?: string) => void }) {
  const { session, setSession } = useCouncilStore();
  const [aspects, setAspects] = useState<Aspect[]>(session?.aspects ?? []);
  if (!session) return null;

  const update = (id: string, patch: Partial<Aspect>) => setAspects((items) => items.map((item) => item.id === id ? { ...item, ...patch } : item));
  const add = () => setAspects((items) => [...items, { id: crypto.randomUUID(), name: "New aspect", description: "Describe what agents should evaluate.", weight: 1 }]);
  const approve = async () => {
    try {
      setSession(await backend.approveAspects(aspects));
    } catch (error) {
      onError("The aspect matrix could not be approved.", String(error));
    }
  };
  const reject = async () => {
    try {
      setSession(await backend.rejectAspects());
    } catch (error) {
      onError("Could not return to the objective.", String(error));
    }
  };

  return (
    <section className="gate-shell">
      <header className="gate-header">
        <span className="gate-icon"><SlidersHorizontal /></span>
        <div>
          <span className="section-kicker">USER GATE • NO AGENT TOKENS YET</span>
          <h1>Approve the evaluation lens.</h1>
          <p>Every member receives these same dimensions. Rename, rebalance, add, or remove them before the live round.</p>
        </div>
        <InfoTip label="Discussion aspects">The dimensions the council uses to structure responses and peer scores. Editing them changes what “good” means for this session.</InfoTip>
      </header>

      <div className="aspect-editor">
        {aspects.map((aspect, index) => (
          <article className="aspect-row" key={aspect.id}>
            <b>{String(index + 1).padStart(2, "0")}</b>
            <div>
              <input aria-label={`Aspect ${index + 1} name`} value={aspect.name} onChange={(event) => update(aspect.id, { name: event.target.value })} />
              <input aria-label={`Aspect ${index + 1} description`} value={aspect.description} onChange={(event) => update(aspect.id, { description: event.target.value })} />
            </div>
            <label>Weight
              <input type="number" min="0.25" max="3" step="0.25" value={aspect.weight} onChange={(event) => update(aspect.id, { weight: Number(event.target.value) })} />
            </label>
            <button type="button" className="icon-button" disabled={aspects.length <= 2} onClick={() => setAspects((items) => items.filter((item) => item.id !== aspect.id))} aria-label={`Delete ${aspect.name}`}><Trash2 size={16} /></button>
          </article>
        ))}
        {aspects.length < 8 && <button type="button" className="add-aspect" onClick={add}><Plus size={16} />Add evaluation aspect</button>}
      </div>

      <footer className="gate-actions">
        <button className="button secondary" type="button" onClick={() => void reject()}><ArrowLeft size={16} />Stop & revise</button>
        <span>{aspects.length} aspects • weighted scoring</span>
        <button className="button primary" type="button" disabled={aspects.some((aspect) => !aspect.name.trim())} onClick={() => void approve()}><Check size={16} />Approve council brief</button>
      </footer>
    </section>
  );
}


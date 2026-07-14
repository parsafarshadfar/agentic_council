import { Activity, CircleDollarSign, Database, Gauge } from "lucide-react";
import { useCouncilStore } from "../store/councilStore";
import { InfoTip } from "./InfoTip";

const usd = (value: number | null) => value == null ? "N/A" : new Intl.NumberFormat("en-US", { style: "currency", currency: "USD", minimumFractionDigits: value < 0.01 ? 4 : 2 }).format(value);
const tokens = (value: number | null) => value == null ? "N/A" : value.toLocaleString();

export function TelemetryPanel() {
  const telemetry = useCouncilStore((state) => state.telemetry);
  if (!telemetry) return <div className="empty-state">No usage has been recorded.</div>;
  return (
    <div className="telemetry-panel">
      <div className="stat-grid">
        <article><span><Database size={16} />Input tokens</span><strong>{tokens(telemetry.total_input_tokens)}</strong></article>
        <article><span><Activity size={16} />Output tokens</span><strong>{tokens(telemetry.total_output_tokens)}</strong></article>
        <article><span><CircleDollarSign size={16} />Estimated cost <InfoTip label="Approximate cost">Calculated from cached provider rates. Final billed cost can differ because provider pricing and accounting rules vary.</InfoTip></span><strong>{usd(telemetry.total_cost_usd)}</strong></article>
      </div>
      <div className="table-scroll">
        <table className="telemetry-table">
          <thead><tr><th>Provider / model</th><th>Input tokens</th><th>Input cost</th><th>Output tokens</th><th>Output cost</th><th>Total</th></tr></thead>
          <tbody>
            {telemetry.by_model.length === 0 && <tr><td colSpan={6}><div className="empty-state"><Gauge size={20} />Usage appears after the first provider response.</div></td></tr>}
            {telemetry.by_model.map((item) => <tr key={`${item.provider_id}:${item.model_id}`}><th><span>{item.provider_id}</span><strong>{item.model_id}</strong></th><td>{tokens(item.input_tokens)}</td><td>{usd(item.input_cost_usd)}</td><td>{tokens(item.output_tokens)}</td><td>{usd(item.output_cost_usd)}</td><td><b>{usd(item.total_cost_usd)}</b></td></tr>)}
          </tbody>
        </table>
      </div>
      <p className="privacy-note">Telemetry is session-scoped, stays on this device, and resets when a new session begins. “N/A” means the provider did not return trustworthy usage metadata.</p>
    </div>
  );
}


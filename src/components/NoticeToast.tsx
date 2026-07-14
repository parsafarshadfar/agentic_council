import { AlertTriangle, CheckCircle2, Info, XCircle } from "lucide-react";
import type { AppNotice } from "../types";

const icons = {
  info: Info,
  warning: AlertTriangle,
  critical: XCircle,
};

export function NoticeToast({ notice, onClose }: { notice: AppNotice; onClose: () => void }) {
  const Icon = icons[notice.severity] ?? CheckCircle2;
  return (
    <article className={`notice notice-${notice.severity}`} role={notice.severity === "critical" ? "alert" : "status"}>
      <Icon size={18} />
      <div>
        <strong>{notice.title}</strong>
        <p>{notice.message}</p>
      </div>
      <button type="button" onClick={onClose} aria-label="Dismiss notification">×</button>
    </article>
  );
}


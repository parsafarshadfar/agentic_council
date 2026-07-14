import { Info, X } from "lucide-react";
import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

const CARD_WIDTH = 280;
const VIEWPORT_MARGIN = 10;
const TRIGGER_GAP = 8;

interface CardPosition {
  left: number;
  top: number;
  ready: boolean;
}

export function InfoTip({ label, children }: { label: string; children: React.ReactNode }) {
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<CardPosition>({ left: 0, top: 0, ready: false });
  const id = useId();
  const root = useRef<HTMLSpanElement>(null);
  const card = useRef<HTMLDivElement>(null);

  const placeCard = useCallback(() => {
    const trigger = root.current?.getBoundingClientRect();
    const tooltip = card.current;
    if (!trigger || !tooltip) return;

    const width = Math.min(CARD_WIDTH, Math.max(180, window.innerWidth - VIEWPORT_MARGIN * 2));
    tooltip.style.width = `${width}px`;
    const height = Math.min(tooltip.scrollHeight, window.innerHeight - VIEWPORT_MARGIN * 2);
    const centeredLeft = trigger.left + trigger.width / 2 - width / 2;
    const left = Math.min(
      Math.max(VIEWPORT_MARGIN, centeredLeft),
      Math.max(VIEWPORT_MARGIN, window.innerWidth - width - VIEWPORT_MARGIN),
    );
    const above = trigger.top - height - TRIGGER_GAP;
    const below = trigger.bottom + TRIGGER_GAP;
    const preferredTop = above >= VIEWPORT_MARGIN && above + height <= window.innerHeight - VIEWPORT_MARGIN
      ? above
      : below >= VIEWPORT_MARGIN && below + height <= window.innerHeight - VIEWPORT_MARGIN
        ? below
        : above;
    const top = Math.min(
      Math.max(VIEWPORT_MARGIN, preferredTop),
      Math.max(VIEWPORT_MARGIN, window.innerHeight - height - VIEWPORT_MARGIN),
    );
    setPosition({ left, top, ready: true });
  }, []);

  useLayoutEffect(() => {
    if (!open) return;
    setPosition((value) => ({ ...value, ready: false }));
    placeCard();
  }, [open, placeCard]);

  useEffect(() => {
    if (!open) return;
    const close = (event: MouseEvent) => {
      const target = event.target as Node;
      if (!root.current?.contains(target) && !card.current?.contains(target)) setOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", escape);
    window.addEventListener("resize", placeCard);
    window.addEventListener("scroll", placeCard, true);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", escape);
      window.removeEventListener("resize", placeCard);
      window.removeEventListener("scroll", placeCard, true);
    };
  }, [open, placeCard]);

  return (
    <span className="info-tip" ref={root}>
      <button
        type="button"
        className="info-trigger"
        aria-label={`About ${label}`}
        aria-expanded={open}
        aria-controls={id}
        onClick={() => setOpen((value) => !value)}
      >
        <Info size={13} />
      </button>
      {open && createPortal(
        <div
          id={id}
          role="tooltip"
          className="info-card"
          ref={card}
          style={{ left: position.left, top: position.top, visibility: position.ready ? "visible" : "hidden" }}
        >
          <span className="info-card-head">
            <strong>{label}</strong>
            <button type="button" aria-label="Close explanation" onClick={() => setOpen(false)}>
              <X size={13} />
            </button>
          </span>
          <span>{children}</span>
        </div>,
        document.body,
      )}
    </span>
  );
}

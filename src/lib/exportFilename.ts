import type { SessionState } from "../types";

const pad = (value: number) => String(value).padStart(2, "0");

export const fileSafePhrase = (value: string) => {
  const phrase = value
    .normalize("NFKD")
    .replace(/\p{M}/gu, "")
    .toLocaleLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 64)
    .replace(/-+$/g, "");
  return phrase || "council-session";
};

export const sessionExportBasename = (session: SessionState, date = new Date()) => {
  const day = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
  const sourcePhrase = session.main_phrase || session.objective.split(/\s+/).slice(0, 8).join(" ");
  const round = session.rounds.at(-1)?.index ?? 0;
  return `${day}__${fileSafePhrase(sourcePhrase)}__round_${round}`;
};

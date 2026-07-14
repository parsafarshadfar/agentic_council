export const councilorLabel = (label: string) =>
  label.replace(/^member\s+(\d+)(?=\s*(?:$|·))/i, "Councilor $1");

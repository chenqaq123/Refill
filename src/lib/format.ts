export function relativeTime(value?: string | number | null) {
  if (!value) return "从未";
  const date = typeof value === "number" ? new Date(value * 1000) : new Date(value);
  const diff = date.getTime() - Date.now();
  const abs = Math.abs(diff);
  const formatter = new Intl.RelativeTimeFormat("zh-CN", { numeric: "auto", style: "short" });
  const units: Array<[Intl.RelativeTimeFormatUnit, number]> = [
    ["day", 86_400_000],
    ["hour", 3_600_000],
    ["minute", 60_000],
    ["second", 1000],
  ];
  const [unit, ms] = units.find(([, size]) => abs >= size) ?? ["second", 1000];
  return formatter.format(Math.round(diff / ms), unit);
}

export function hostFromUrl(value: string) {
  try {
    return new URL(value).host;
  } catch {
    return value.replace(/^https?:\/\//, "");
  }
}

export function percent(value: number) {
  return `${Math.round(Math.max(0, Math.min(100, value)))}%`;
}

export function shortPath(path: string) {
  return path.replace(/^\/Users\/[^/]+/, "~");
}

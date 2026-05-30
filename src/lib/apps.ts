// Registry of the desktop apps Refill can manage. Today only Codex is wired
// end-to-end; others are placeholders so the UI (app switcher) and future
// backend work have a single source of truth to extend.

export type AppId = "codex" | "claude" | "gemini";

export type ManagedApp = {
  id: AppId;
  name: string;
  /** One-letter avatar / mark. */
  mark: string;
  /** Tailwind background class for the app's accent. */
  accent: string;
  /** Short tagline shown under the name in the app switcher. */
  tagline: string;
  /** Whether switching/usage is implemented for this app yet. */
  available: boolean;
};

export const APPS: ManagedApp[] = [
  {
    id: "codex",
    name: "Codex",
    mark: "C",
    accent: "bg-blue",
    tagline: "账号 · API · 额度",
    available: true,
  },
  {
    id: "claude",
    name: "Claude Code",
    mark: "✶",
    accent: "bg-[#d97757]",
    tagline: "即将支持",
    available: false,
  },
  {
    id: "gemini",
    name: "Gemini CLI",
    mark: "G",
    accent: "bg-[#1a73e8]",
    tagline: "即将支持",
    available: false,
  },
];

export function appById(id: AppId): ManagedApp {
  return APPS.find((app) => app.id === id) ?? APPS[0];
}

import { motion } from "framer-motion";
import { BarChart3, ChevronDown, Settings2, Users } from "lucide-react";
import { useState } from "react";
import { APPS, appById, type AppId } from "../../lib/apps";
import { cn } from "../../lib/cn";

export type PageId = "accounts" | "usage" | "settings";

const NAV: { id: PageId; label: string; icon: typeof Users }[] = [
  { id: "accounts", label: "账号", icon: Users },
  { id: "usage", label: "用量", icon: BarChart3 },
  { id: "settings", label: "设置", icon: Settings2 },
];

type SidebarProps = {
  appId: AppId;
  onSelectApp: (id: AppId) => void;
  page: PageId;
  onNavigate: (page: PageId) => void;
  footer?: React.ReactNode;
};

export function Sidebar({ appId, onSelectApp, page, onNavigate, footer }: SidebarProps) {
  const [appMenuOpen, setAppMenuOpen] = useState(false);
  const app = appById(appId);

  return (
    <aside className="flex h-screen w-[212px] shrink-0 flex-col border-r border-line bg-panel/60 px-3 py-4">
      <div className="px-1">
        <div className="flex items-center gap-2 px-1">
          <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-blue text-white shadow-[0_8px_18px_rgba(35,120,238,0.2)]">
            <span className="text-base font-black">R</span>
          </div>
          <span className="text-lg font-black tracking-tight text-ink">Refill</span>
        </div>
      </div>

      {/* App switcher — extensible: add entries in lib/apps.ts */}
      <div className="relative mt-4 px-1">
        <button
          type="button"
          onClick={() => setAppMenuOpen((open) => !open)}
          className="pressable flex w-full items-center gap-2.5 rounded-2xl border border-line bg-panel px-2.5 py-2 text-left hover:border-blue/30"
        >
          <div className={cn("flex h-8 w-8 items-center justify-center rounded-lg text-sm font-black text-white", app.accent)}>
            {app.mark}
          </div>
          <div className="min-w-0 flex-1">
            <div className="truncate text-sm font-black text-ink">{app.name}</div>
            <div className="truncate text-[11px] font-semibold text-sub">{app.tagline}</div>
          </div>
          <ChevronDown size={15} className={cn("text-sub transition-transform", appMenuOpen && "rotate-180")} />
        </button>

        {appMenuOpen ? (
          <motion.div
            initial={{ opacity: 0, y: -6 }}
            animate={{ opacity: 1, y: 0 }}
            className="absolute left-1 right-1 top-full z-30 mt-1.5 overflow-hidden rounded-2xl border border-line bg-panel p-1 shadow-soft"
          >
            {APPS.map((entry) => (
              <button
                key={entry.id}
                type="button"
                disabled={!entry.available}
                onClick={() => {
                  if (!entry.available) return;
                  onSelectApp(entry.id);
                  setAppMenuOpen(false);
                }}
                className={cn(
                  "flex w-full items-center gap-2.5 rounded-xl px-2 py-1.5 text-left",
                  entry.available ? "hover:bg-black/5" : "cursor-not-allowed opacity-45",
                  entry.id === appId && "bg-blue/8",
                )}
              >
                <div className={cn("flex h-7 w-7 items-center justify-center rounded-lg text-xs font-black text-white", entry.accent)}>
                  {entry.mark}
                </div>
                <div className="min-w-0">
                  <div className="truncate text-sm font-bold text-ink">{entry.name}</div>
                  <div className="truncate text-[11px] font-semibold text-sub">{entry.tagline}</div>
                </div>
              </button>
            ))}
          </motion.div>
        ) : null}
      </div>

      <nav className="mt-5 flex flex-col gap-1 px-0.5">
        {NAV.map((item) => {
          const active = page === item.id;
          const Icon = item.icon;
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => onNavigate(item.id)}
              className={cn(
                "pressable relative flex items-center gap-2.5 rounded-xl px-3 py-2 text-sm font-bold transition-colors",
                active ? "text-blue" : "text-sub hover:text-ink",
              )}
            >
              {active ? (
                <motion.div
                  layoutId="nav-active"
                  className="absolute inset-0 rounded-xl bg-blue/10"
                  transition={{ type: "spring", stiffness: 500, damping: 38 }}
                />
              ) : null}
              <Icon size={17} className="relative z-10" />
              <span className="relative z-10">{item.label}</span>
            </button>
          );
        })}
      </nav>

      <div className="mt-auto px-1">{footer}</div>
    </aside>
  );
}

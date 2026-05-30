import { motion } from "framer-motion";
import { BarChart3, Settings2, Users } from "lucide-react";
import { APPS, appById, type AppId } from "../../lib/apps";
import { cn } from "../../lib/cn";
import appIcon from "../../assets/app-icon.png";

export type PageId = "accounts" | "usage" | "settings";

const NAV: { id: PageId; label: string; icon: typeof Users }[] = [
  { id: "accounts", label: "账号", icon: Users },
  { id: "usage", label: "用量", icon: BarChart3 },
];

type SidebarProps = {
  appId: AppId;
  onSelectApp: (id: AppId) => void;
  page: PageId;
  onNavigate: (page: PageId) => void;
  syncLabel: string;
  notice: string;
};

export function Sidebar({ appId, onSelectApp, page, onNavigate, syncLabel, notice }: SidebarProps) {
  const app = appById(appId);

  return (
    <div className="flex h-screen shrink-0">
      {/* Tool rail — switch between managed apps (extensible via lib/apps.ts) */}
      <div className="flex w-[60px] flex-col items-center gap-2 border-r border-line bg-muted/40 py-3">
        <img src={appIcon} alt="Refill" className="mb-1 h-9 w-9 rounded-[11px] shadow-[0_6px_16px_rgba(28,35,45,0.18)]" />
        <div className="my-1 h-px w-7 bg-line" />
        {APPS.map((entry) => {
          const active = entry.id === appId;
          return (
            <button
              key={entry.id}
              type="button"
              disabled={!entry.available}
              title={entry.available ? entry.name : `${entry.name} · 即将支持`}
              onClick={() => entry.available && onSelectApp(entry.id)}
              className={cn(
                "relative flex h-10 w-10 items-center justify-center rounded-xl text-sm font-black transition-all",
                active
                  ? cn(entry.accent, "text-white shadow-[0_8px_18px_rgba(28,35,45,0.18)]")
                  : entry.available
                    ? "bg-panel text-sub hover:text-ink hover:shadow-card"
                    : "bg-panel/60 text-sub/35 cursor-not-allowed",
              )}
            >
              {active ? (
                <motion.span
                  layoutId="rail-active"
                  className="absolute -left-[11px] h-5 w-[3px] rounded-full bg-ink"
                  transition={{ type: "spring", stiffness: 500, damping: 36 }}
                />
              ) : null}
              {entry.mark}
            </button>
          );
        })}
      </div>

      {/* App sidebar — pages for the selected tool */}
      <aside className="flex w-[196px] flex-col bg-panel/60 px-3 py-4">
        <div className="px-1.5">
          <div className="text-lg font-black tracking-tight text-ink">{app.name}</div>
          <div className="mt-0.5 text-[11px] font-semibold text-sub">{app.tagline}</div>
        </div>

        <nav className="mt-5 flex flex-col gap-1">
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

        <div className="mt-auto space-y-2">
          <button
            type="button"
            onClick={() => onNavigate("settings")}
            className={cn(
              "pressable flex w-full items-center gap-2.5 rounded-xl px-3 py-2 text-sm font-bold transition-colors",
              page === "settings" ? "bg-blue/10 text-blue" : "text-sub hover:text-ink hover:bg-black/5",
            )}
          >
            <Settings2 size={17} />
            设置
          </button>
          <div className="rounded-xl bg-muted/60 px-2.5 py-2 text-[11px] font-bold text-sub/70">
            <div className="truncate">同步 {syncLabel}</div>
            <div className="mt-0.5 truncate text-sub/55">{notice}</div>
          </div>
        </div>
      </aside>
    </div>
  );
}

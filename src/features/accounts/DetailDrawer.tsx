import { AnimatePresence, motion } from "framer-motion";
import { Copy, FolderGit2, KeyRound, Link2, X } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { Chip } from "../../components/ui/Chip";
import type { DashboardState, Profile } from "../../lib/types";
import { shortPath } from "../../lib/format";

type DetailDrawerProps = {
  profile: Profile | null;
  dashboard?: DashboardState | null;
  onClose: () => void;
  onCopyDiagnostics: () => void;
};

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[92px_minmax(0,1fr)] items-center gap-3 border-b border-line/70 py-2.5 last:border-0">
      <span className="text-[11px] font-black uppercase tracking-wide text-sub/65">{label}</span>
      <span className="select-text truncate text-right text-sm font-semibold text-ink" title={value}>
        {value}
      </span>
    </div>
  );
}

export function DetailDrawer({ profile, dashboard, onClose, onCopyDiagnostics }: DetailDrawerProps) {
  return (
    <AnimatePresence>
      {profile ? (
        <>
          <motion.div
            className="fixed inset-0 z-40 bg-black/16 backdrop-blur-[2px]"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.18 }}
            onClick={onClose}
          />
          <motion.aside
            className="fixed bottom-3 right-3 top-3 z-50 flex w-[340px] flex-col overflow-hidden rounded-3xl border border-line bg-panel shadow-soft"
            initial={{ x: 380, opacity: 0.6 }}
            animate={{ x: 0, opacity: 1 }}
            exit={{ x: 380, opacity: 0.4 }}
            transition={{ type: "spring", stiffness: 420, damping: 38 }}
          >
            <div className="flex items-start justify-between border-b border-line/70 px-4 py-4">
              <div className="min-w-0">
                <h3 className="line-clamp-2 break-all text-lg font-black leading-tight text-ink" title={profile.title}>
                  {profile.title}
                </h3>
                <p className="mt-1 truncate text-sm font-semibold text-sub" title={profile.subtitle}>
                  {profile.subtitle}
                </p>
              </div>
              <Button variant="ghost" className="h-9 w-9 shrink-0 px-0" onClick={onClose} aria-label="关闭">
                <X size={17} />
              </Button>
            </div>

            <div className="flex-1 space-y-4 overflow-y-auto px-4 py-4">
              <div className="flex flex-wrap gap-1.5">
                <Chip tone={profile.isActive ? "green" : "gray"}>{profile.isActive ? "当前" : "未启动"}</Chip>
                <Chip tone={profile.isReady ? "teal" : "amber"}>{profile.isReady ? "可启动" : "需检查"}</Chip>
                <Chip tone={profile.kind === "api" ? "teal" : "blue"}>{profile.kind === "api" ? "API" : "官方账号"}</Chip>
              </div>
              <div className="rounded-2xl bg-muted/70 px-3 py-1">
                <Row label="Profile" value={shortPath(profile.diagnostics.profilePath)} />
                <Row label="Codex Home" value={shortPath(profile.diagnostics.codexHomePath)} />
                <Row label="Config" value={profile.diagnostics.configExists ? "存在" : "缺失"} />
                <Row label="Auth" value={profile.diagnostics.authExists ? "存在" : profile.kind === "api" ? "API 不需要" : "缺失"} />
                <Row label="Keychain" value={profile.diagnostics.keychainReady ? "Ready" : profile.kind === "api" ? "Missing" : "不适用"} />
              </div>
              <div className="grid grid-cols-2 gap-1.5">
                <Chip tone={profile.diagnostics.sessionsShared ? "green" : "amber"} icon={<Link2 size={13} />}>sessions</Chip>
                <Chip tone={profile.diagnostics.sessionIndexShared ? "green" : "amber"} icon={<Link2 size={13} />}>index</Chip>
                <Chip tone={profile.diagnostics.desktopStateShared ? "green" : "amber"} icon={<FolderGit2 size={13} />}>desktop</Chip>
                <Chip tone={profile.diagnostics.keychainReady ? "green" : "gray"} icon={<KeyRound size={13} />}>keychain</Chip>
              </div>
              {dashboard ? (
                <div className="rounded-2xl bg-muted/70 px-3 py-1">
                  <Row label="Profiles" value={shortPath(dashboard.profileRoot)} />
                  <Row label="Shared" value={shortPath(dashboard.sharedHistoryRoot)} />
                </div>
              ) : null}
            </div>

            <div className="border-t border-line/70 p-4">
              <Button variant="soft" className="w-full" onClick={onCopyDiagnostics} icon={<Copy size={16} />}>
                复制诊断信息
              </Button>
            </div>
          </motion.aside>
        </>
      ) : null}
    </AnimatePresence>
  );
}

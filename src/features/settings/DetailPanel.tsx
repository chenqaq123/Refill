import { Copy, FolderGit2, KeyRound, Link2, Settings } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { Chip } from "../../components/ui/Chip";
import type { DashboardState, Profile } from "../../lib/types";
import { shortPath } from "../../lib/format";

type DetailPanelProps = {
  dashboard?: DashboardState | null;
  profile?: Profile | null;
  onCopyDiagnostics: () => void;
};

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-4 border-b border-line/70 py-3 last:border-0">
      <span className="text-xs font-bold uppercase tracking-wide text-sub/70">{label}</span>
      <span className="max-w-[250px] select-text break-all text-right text-sm font-semibold text-ink">{value}</span>
    </div>
  );
}

export function DetailPanel({ dashboard, profile, onCopyDiagnostics }: DetailPanelProps) {
  return (
    <aside className="sticky top-5 h-[calc(100vh-40px)] w-[340px] shrink-0 rounded-[28px] border border-line bg-panel p-5 shadow-card">
      <div className="flex items-center justify-between">
        <div>
          <div className="text-sm font-black text-ink">详情</div>
          <div className="mt-1 text-xs font-semibold text-sub">诊断、共享状态与设置</div>
        </div>
        <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-black/5 text-sub">
          <Settings size={18} />
        </div>
      </div>

      <div className="mt-5 space-y-3">
        {profile ? (
          <>
            <div>
              <h3 className="truncate text-xl font-black text-ink">{profile.title}</h3>
              <p className="mt-1 truncate text-sm font-semibold text-sub">{profile.subtitle}</p>
            </div>
            <div className="flex flex-wrap gap-2">
              <Chip tone={profile.isActive ? "green" : "gray"}>{profile.isActive ? "当前" : "未启动"}</Chip>
              <Chip tone={profile.isReady ? "teal" : "amber"}>{profile.isReady ? "可启动" : "需检查"}</Chip>
              <Chip tone={profile.kind === "api" ? "teal" : "blue"}>{profile.kind === "api" ? "API" : "官方账号"}</Chip>
            </div>
            <div className="rounded-2xl bg-muted/60 p-3">
              <Row label="Profile" value={shortPath(profile.diagnostics.profilePath)} />
              <Row label="Codex Home" value={shortPath(profile.diagnostics.codexHomePath)} />
              <Row label="Config" value={profile.diagnostics.configExists ? "存在" : "缺失"} />
              <Row label="Auth" value={profile.diagnostics.authExists ? "存在" : profile.kind === "api" ? "API 不需要" : "缺失"} />
              <Row label="Keychain" value={profile.diagnostics.keychainReady ? "Ready" : profile.kind === "api" ? "Missing" : "不适用"} />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <Chip tone={profile.diagnostics.sessionsShared ? "green" : "amber"} icon={<Link2 size={13} />}>
                sessions
              </Chip>
              <Chip tone={profile.diagnostics.sessionIndexShared ? "green" : "amber"} icon={<Link2 size={13} />}>
                index
              </Chip>
              <Chip tone={profile.diagnostics.desktopStateShared ? "green" : "amber"} icon={<FolderGit2 size={13} />}>
                desktop
              </Chip>
              <Chip tone={profile.diagnostics.keychainReady ? "green" : "gray"} icon={<KeyRound size={13} />}>
                keychain
              </Chip>
            </div>
            <Button variant="soft" className="w-full" onClick={onCopyDiagnostics} icon={<Copy size={16} />}>
              复制诊断信息
            </Button>
          </>
        ) : (
          <div className="rounded-2xl bg-muted/60 p-4 text-sm font-semibold leading-6 text-sub">
            选择一个账号或 API provider，可以查看它的配置、共享历史、Keychain 和启动状态。
          </div>
        )}
      </div>

      {dashboard ? (
        <div className="absolute bottom-5 left-5 right-5 rounded-2xl bg-muted/70 p-3">
          <Row label="Profiles" value={shortPath(dashboard.profileRoot)} />
          <Row label="Shared" value={shortPath(dashboard.sharedHistoryRoot)} />
        </div>
      ) : null}
    </aside>
  );
}

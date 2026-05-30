import { useEffect, useMemo, useRef, useState } from "react";
import { CheckCircle2, KeyRound, Loader2, Plus, RefreshCw, Sparkles, UserRoundPlus, UsersRound, XCircle } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { Chip } from "../../components/ui/Chip";
import { api } from "../../lib/tauri";
import type { DashboardState, Profile, ProviderInput, ProviderUpdateInput, SwitchProgress } from "../../lib/types";
import { relativeTime } from "../../lib/format";
import { ProfileCard } from "../profiles/ProfileCard";
import { ProviderDialog } from "../providers/ProviderDialog";
import { DetailPanel } from "../settings/DetailPanel";

export function Dashboard() {
  const [dashboard, setDashboard] = useState<DashboardState | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [busyProfileId, setBusyProfileId] = useState<string | null>(null);
  const [progress, setProgress] = useState<SwitchProgress | null>(null);
  const [notice, setNotice] = useState("正在加载");
  const [providerDialogOpen, setProviderDialogOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Profile | null>(null);
  const [toast, setToast] = useState<{ kind: "ok" | "error"; text: string } | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  function showToast(kind: "ok" | "error", text: string) {
    window.clearTimeout(toastTimer.current);
    setToast({ kind, text });
    toastTimer.current = window.setTimeout(() => setToast(null), kind === "ok" ? 2600 : 5000);
  }

  const profiles = dashboard?.profiles ?? [];
  const officialProfiles = profiles.filter((profile) => profile.kind === "official");
  const apiProfiles = profiles.filter((profile) => profile.kind === "api");
  const selectedProfile = profiles.find((profile) => profile.id === selectedId) ?? profiles.find((profile) => profile.isActive) ?? null;

  const lastSynced = useMemo(() => {
    if (!dashboard?.lastSyncedAt) return "从未";
    return relativeTime(dashboard.lastSyncedAt);
  }, [dashboard?.lastSyncedAt]);

  async function refresh() {
    const state = await api.refreshProfiles();
    setDashboard(state);
    setSelectedId((current) => current ?? state.profiles.find((profile) => profile.isActive)?.id ?? state.profiles[0]?.id ?? null);
    setNotice("已同步");
  }

  useEffect(() => {
    refresh().catch((error) => setNotice(String(error)));
    const timer = window.setInterval(() => {
      refresh().catch((error) => setNotice(String(error)));
    }, 60_000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    api.onSwitchProgress((event) => setProgress(event)).then((dispose) => {
      unlisten = dispose;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  async function launch(profile: Profile) {
    setBusyProfileId(profile.id);
    setNotice(`正在切换 ${profile.title}`);
    setProgress({ profileId: profile.id, stage: "quitting_codex", message: "准备切换", percent: 4 });
    try {
      await api.switchProfile(profile.id);
      await refresh();
      setNotice(`已启动 ${profile.title}`);
      showToast("ok", `已切换到 ${profile.title}`);
    } catch (error) {
      setNotice(String(error));
      showToast("error", `切换失败：${String(error)}`);
      setProgress({ profileId: profile.id, stage: "failed", message: String(error), percent: 100 });
    } finally {
      setBusyProfileId(null);
      window.setTimeout(() => setProgress(null), 1800);
    }
  }

  async function saveCurrent() {
    setBusyProfileId("__current__");
    setNotice("正在保存当前账号");
    try {
      await api.saveCurrentProfile();
      await refresh();
      setNotice("当前账号已保存");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusyProfileId(null);
    }
  }

  async function submitProvider(input: ProviderInput | ProviderUpdateInput) {
    try {
      if (editingProvider) {
        await api.updateProvider(editingProvider.id, input as ProviderUpdateInput);
        setNotice("API provider 已更新");
        showToast("ok", "API provider 已更新");
      } else {
        await api.createProvider(input as ProviderInput);
        setNotice("API provider 已创建");
        showToast("ok", "API provider 已创建");
      }
      setProviderDialogOpen(false);
      setEditingProvider(null);
      await refresh();
    } catch (error) {
      setNotice(String(error));
      showToast("error", `保存失败：${String(error)}`);
    }
  }

  async function deleteProvider(profile: Profile) {
    setNotice(`正在删除 ${profile.title}`);
    try {
      await api.deleteProvider(profile.id);
      await refresh();
      setNotice("API provider 已删除");
    } catch (error) {
      setNotice(String(error));
    }
  }

  async function copyDiagnostics() {
    const payload = JSON.stringify({ selectedProfile, dashboard }, null, 2);
    await navigator.clipboard.writeText(payload);
    setNotice("诊断信息已复制");
  }

  return (
    <div className="flex h-screen bg-canvas text-ink">
      <main className="flex-1 overflow-y-auto px-6 py-5">
        <div className="mx-auto max-w-[1180px] space-y-4">
          <section className="glass-panel rounded-3xl px-5 py-4">
            <div className="flex items-center justify-between gap-5">
              <div className="flex min-w-0 items-center gap-3">
                <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-blue text-white shadow-[0_10px_24px_rgba(35,120,238,0.18)]">
                  <Sparkles size={22} />
                </div>
                <div className="min-w-0">
                  <h1 className="text-[26px] font-black leading-tight">Refill</h1>
                  <p className="mt-0.5 max-w-[700px] truncate text-sm font-semibold text-sub">
                    {dashboard?.activeLabel ?? "未连接"} · Codex 账号、API provider、额度与共享会话
                  </p>
                </div>
              </div>
              <div className="flex shrink-0 gap-2">
                <Button variant="soft" icon={<UserRoundPlus size={17} />} onClick={() => api.openLoginTerminal().catch((error) => setNotice(String(error)))}>
                  登录
                </Button>
                <Button
                  variant="soft"
                  className="bg-teal/10 text-teal hover:bg-teal/15"
                  icon={<KeyRound size={17} />}
                  onClick={() => {
                    setEditingProvider(null);
                    setProviderDialogOpen(true);
                  }}
                >
                  API
                </Button>
                <Button variant="soft" icon={<RefreshCw size={17} />} onClick={() => refresh().catch((error) => setNotice(String(error)))}>
                  同步
                </Button>
              </div>
            </div>
            <div className="mt-4 flex flex-wrap items-center gap-2">
              <Chip tone="green" solid>
                当前账号
              </Chip>
              <Chip tone="teal">共享会话</Chip>
              <Chip tone="blue" icon={<UsersRound size={14} />}>
                {officialProfiles.length} 个账号
              </Chip>
              <Chip tone="teal" icon={<KeyRound size={14} />}>
                {apiProfiles.length} 个 API
              </Chip>
              <Chip tone="gray">1 分钟同步</Chip>
              <span className="ml-auto min-w-0 truncate text-xs font-bold text-sub/60">同步 {lastSynced} · {notice}</span>
            </div>
          </section>

          {dashboard?.unmanagedCurrent ? (
            <section className="space-y-2.5">
              <SectionHeading title="待保存" count={1} />
              <ProfileCard
                profile={dashboard.unmanagedCurrent}
                busy={busyProfileId === "__current__"}
                selected={selectedProfile?.id === dashboard.unmanagedCurrent.id}
                onSelect={() => setSelectedId(dashboard.unmanagedCurrent?.id ?? null)}
                onLaunch={saveCurrent}
              />
            </section>
          ) : null}

          <section className="space-y-2.5">
            <SectionHeading title="官方账号" count={officialProfiles.length} />
            <div className="space-y-2.5">
              {officialProfiles.map((profile) => (
                <ProfileCard
                  key={profile.id}
                  profile={profile}
                  busy={busyProfileId === profile.id}
                  progress={progress?.profileId === profile.id ? progress : null}
                  selected={selectedProfile?.id === profile.id}
                  onSelect={() => setSelectedId(profile.id)}
                  onLaunch={() => launch(profile)}
                />
              ))}
            </div>
          </section>

          <section className="space-y-2.5">
            <SectionHeading title="API Providers" count={apiProfiles.length} />
            <div className="space-y-2.5">
              {apiProfiles.map((profile) => (
                <ProfileCard
                  key={profile.id}
                  profile={profile}
                  busy={busyProfileId === profile.id}
                  progress={progress?.profileId === profile.id ? progress : null}
                  selected={selectedProfile?.id === profile.id}
                  onSelect={() => setSelectedId(profile.id)}
                  onLaunch={() => launch(profile)}
                  onEdit={() => {
                    setEditingProvider(profile);
                    setProviderDialogOpen(true);
                  }}
                  onDelete={!profile.isActive ? () => deleteProvider(profile) : undefined}
                />
              ))}
              {apiProfiles.length === 0 ? (
                <button
                  className="pressable flex w-full items-center justify-center gap-3 rounded-2xl border border-dashed border-line bg-panel/70 p-6 text-sm font-bold text-sub hover:border-teal/35 hover:bg-teal/5"
                  onClick={() => setProviderDialogOpen(true)}
                >
                  <Plus size={18} />
                  添加第一个 API provider（DeepSeek / OpenRouter / Kimi …）
                </button>
              ) : null}
            </div>
          </section>
        </div>
      </main>

      <div className="border-l border-line/80 bg-[#f2f3f3] p-4">
        <DetailPanel dashboard={dashboard} profile={selectedProfile} onCopyDiagnostics={copyDiagnostics} />
      </div>

      {!dashboard ? (
        <div className="fixed inset-0 flex items-center justify-center bg-canvas">
          <div className="flex items-center gap-3 rounded-2xl bg-panel px-5 py-4 text-sm font-bold text-sub shadow-card">
            <Loader2 className="animate-spin" size={18} />
            正在载入 Refill
          </div>
        </div>
      ) : null}

      {toast ? (
        <div className="fixed bottom-5 right-5 z-50 animate-[fadeIn_0.15s_ease-out]">
          <div
            className={`flex max-w-[380px] items-start gap-2.5 rounded-2xl px-4 py-3 text-sm font-bold shadow-card ${
              toast.kind === "ok" ? "bg-[#0c7a4d] text-white" : "bg-red text-white"
            }`}
          >
            {toast.kind === "ok" ? <CheckCircle2 size={18} className="mt-0.5 shrink-0" /> : <XCircle size={18} className="mt-0.5 shrink-0" />}
            <span className="min-w-0 break-words">{toast.text}</span>
          </div>
        </div>
      ) : null}

      <ProviderDialog
        open={providerDialogOpen}
        profile={editingProvider}
        onClose={() => {
          setProviderDialogOpen(false);
          setEditingProvider(null);
        }}
        onSubmit={submitProvider}
      />
    </div>
  );
}

function SectionHeading({ title, count }: { title: string; count: number }) {
  return (
    <div className="flex items-center gap-2 px-1 pt-1">
      <h2 className="text-[13px] font-black text-sub">{title}</h2>
      <span className="rounded-full bg-black/5 px-2 py-0.5 text-[11px] font-black text-sub/55">{count}</span>
    </div>
  );
}

import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./tauri";
import type { DashboardState, Profile, ProviderInput, ProviderUpdateInput, SwitchProgress } from "./types";

export type Toast = { kind: "ok" | "error"; text: string };

// Central data + actions for Refill, shared across pages.
export function useRefill() {
  const [dashboard, setDashboard] = useState<DashboardState | null>(null);
  const [busyProfileId, setBusyProfileId] = useState<string | null>(null);
  const [progress, setProgress] = useState<SwitchProgress | null>(null);
  const [notice, setNotice] = useState("正在加载");
  const [toast, setToast] = useState<Toast | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  const showToast = useCallback((kind: "ok" | "error", text: string) => {
    window.clearTimeout(toastTimer.current);
    setToast({ kind, text });
    toastTimer.current = window.setTimeout(() => setToast(null), kind === "ok" ? 2600 : 5000);
  }, []);

  const refresh = useCallback(async () => {
    const state = await api.refreshProfiles();
    setDashboard(state);
    setNotice("已同步");
    return state;
  }, []);

  useEffect(() => {
    refresh().catch((error) => setNotice(String(error)));
    const timer = window.setInterval(() => {
      refresh().catch((error) => setNotice(String(error)));
    }, 60_000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    api.onSwitchProgress((event) => setProgress(event)).then((dispose) => {
      unlisten = dispose;
    });
    return () => unlisten?.();
  }, []);

  const launch = useCallback(
    async (profile: Profile) => {
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
    },
    [refresh, showToast],
  );

  const saveCurrent = useCallback(async () => {
    setBusyProfileId("__current__");
    setNotice("正在保存当前账号");
    try {
      await api.saveCurrentProfile();
      await refresh();
      setNotice("当前账号已保存");
      showToast("ok", "当前账号已保存");
    } catch (error) {
      setNotice(String(error));
      showToast("error", String(error));
    } finally {
      setBusyProfileId(null);
    }
  }, [refresh, showToast]);

  const submitProvider = useCallback(
    async (editingId: string | null, input: ProviderInput | ProviderUpdateInput) => {
      try {
        if (editingId) {
          await api.updateProvider(editingId, input as ProviderUpdateInput);
          showToast("ok", "API provider 已更新");
        } else {
          await api.createProvider(input as ProviderInput);
          showToast("ok", "API provider 已创建");
        }
        await refresh();
        return true;
      } catch (error) {
        showToast("error", `保存失败：${String(error)}`);
        return false;
      }
    },
    [refresh, showToast],
  );

  const deleteProvider = useCallback(
    async (profile: Profile) => {
      try {
        await api.deleteProvider(profile.id);
        await refresh();
        showToast("ok", "API provider 已删除");
      } catch (error) {
        showToast("error", String(error));
      }
    },
    [refresh, showToast],
  );

  const login = useCallback(() => {
    api.openLoginTerminal().catch((error) => showToast("error", String(error)));
  }, [showToast]);

  return {
    dashboard,
    busyProfileId,
    progress,
    notice,
    toast,
    refresh,
    launch,
    saveCurrent,
    submitProvider,
    deleteProvider,
    login,
    showToast,
  };
}

export type RefillStore = ReturnType<typeof useRefill>;

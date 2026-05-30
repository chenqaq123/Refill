import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { Check, Copy } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { TextField } from "../../components/ui/TextField";
import { api } from "../../lib/tauri";
import type { AppSettings } from "../../lib/types";
import type { RefillStore } from "../../lib/useRefill";
import { shortPath } from "../../lib/format";

export function SettingsPage({ store }: { store: RefillStore }) {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [saved, setSaved] = useState(false);
  const dashboard = store.dashboard;

  useEffect(() => {
    api.getSettings().then(setSettings).catch(() => undefined);
  }, []);

  async function save(next: AppSettings) {
    setSettings(next);
    try {
      await api.updateSettings(next);
      setSaved(true);
      window.setTimeout(() => setSaved(false), 1500);
    } catch (error) {
      store.showToast("error", String(error));
    }
  }

  async function copyDiagnostics() {
    await navigator.clipboard.writeText(JSON.stringify({ dashboard }, null, 2));
    store.showToast("ok", "诊断信息已复制");
  }

  return (
    <div className="mx-auto max-w-[760px] space-y-5">
      <header>
        <h1 className="text-2xl font-black tracking-tight">设置</h1>
        <p className="mt-0.5 text-sm font-semibold text-sub">同步、共享历史与诊断</p>
      </header>

      {settings ? (
        <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} className="space-y-4 rounded-2xl border border-line bg-panel p-5">
          <TextField
            label="同步间隔（秒）"
            type="number"
            min="10"
            value={String(settings.refreshIntervalSeconds)}
            onChange={(e) => save({ ...settings, refreshIntervalSeconds: parseInt(e.target.value) || 60 })}
            hint="自动刷新账号状态与额度的频率。"
          />
          <TextField
            label="Codex 应用名"
            value={settings.codexAppName}
            onChange={(e) => save({ ...settings, codexAppName: e.target.value })}
            hint="切换时退出/启动的应用名（默认 Codex）。"
          />
          <label className="flex items-center justify-between rounded-xl bg-muted/60 px-3 py-2.5">
            <span className="text-sm font-bold text-ink">跨账号共享历史</span>
            <input
              type="checkbox"
              checked={settings.shareHistory}
              onChange={(e) => save({ ...settings, shareHistory: e.target.checked })}
              className="h-5 w-5 accent-blue"
            />
          </label>
          <div className="flex h-5 items-center text-xs font-bold text-green">
            {saved ? (
              <motion.span initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="inline-flex items-center gap-1">
                <Check size={14} /> 已保存
              </motion.span>
            ) : null}
          </div>
        </motion.div>
      ) : null}

      <div className="space-y-1 rounded-2xl border border-line bg-panel p-5">
        <h2 className="text-sm font-black text-ink">路径</h2>
        <Row label="Profiles" value={shortPath(dashboard?.profileRoot ?? "—")} />
        <Row label="共享历史" value={shortPath(dashboard?.sharedHistoryRoot ?? "—")} />
        <Row label="Codex Home" value={shortPath(dashboard?.codexHome ?? "—")} />
        <div className="pt-3">
          <Button variant="soft" icon={<Copy size={16} />} onClick={copyDiagnostics}>
            复制诊断信息
          </Button>
        </div>
      </div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[110px_minmax(0,1fr)] items-center gap-3 border-b border-line/60 py-2 last:border-0">
      <span className="text-[11px] font-black uppercase tracking-wide text-sub/65">{label}</span>
      <span className="select-text truncate text-right text-sm font-semibold text-ink" title={value}>{value}</span>
    </div>
  );
}

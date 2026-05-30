import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { Modal } from "../../components/ui/Modal";
import { api } from "../../lib/tauri";
import type { UsageWindowRecord } from "../../lib/types";

type AccountUsageDialogProps = {
  open: boolean;
  profileId: string | null;
  title: string;
  onClose: () => void;
};

function windowTitle(record: UsageWindowRecord): string {
  const m = record.windowMinutes;
  if (m >= 10080) return "周窗口（coding plan）";
  if (m >= 1440) return `${Math.round(m / 1440)} 天窗口`;
  if (m >= 60) return `${Math.round(m / 60)} 小时窗口`;
  return `${m} 分钟窗口`;
}

function fmtReset(resetsAt?: number | null): string {
  if (!resetsAt) return "—";
  const d = new Date(resetsAt * 1000);
  return d.toLocaleString([], { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

function barColor(used: number): string {
  if (used >= 90) return "bg-red";
  if (used >= 70) return "bg-amber";
  return "bg-blue";
}

function UsageRow({ record }: { record: UsageWindowRecord }) {
  const used = Math.round(record.usedPercent);
  return (
    <div className={`rounded-xl border px-3 py-2 ${record.isCurrent ? "border-blue/40 bg-blue/5" : "border-neutral-200"}`}>
      <div className="flex items-center justify-between text-xs">
        <span className="font-semibold text-neutral-600">
          {record.isCurrent ? "本周期 · " : ""}
          {record.isCurrent ? `重置于 ${fmtReset(record.resetsAt)}` : `周期至 ${fmtReset(record.resetsAt)}`}
        </span>
        <span className="font-black tabular-nums text-neutral-900">{used}%</span>
      </div>
      <div className="mt-1.5 h-2 overflow-hidden rounded-full bg-neutral-100">
        <div className={`h-full rounded-full ${barColor(used)}`} style={{ width: `${Math.max(2, Math.min(100, used))}%` }} />
      </div>
    </div>
  );
}

export function AccountUsageDialog({ open, profileId, title, onClose }: AccountUsageDialogProps) {
  const [records, setRecords] = useState<UsageWindowRecord[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!open || !profileId) return;
    setLoading(true);
    api
      .accountUsageHistory(profileId)
      .then(setRecords)
      .catch(() => setRecords([]))
      .finally(() => setLoading(false));
  }, [open, profileId]);

  // Show the longer window (weekly / coding plan) first, regardless of whether
  // the upstream labels it primary or secondary.
  const groups = (["primary", "secondary"] as const)
    .filter((kind) => records.some((r) => r.kind === kind))
    .sort((a, b) => {
      const ma = records.find((r) => r.kind === a)?.windowMinutes ?? 0;
      const mb = records.find((r) => r.kind === b)?.windowMinutes ?? 0;
      return mb - ma;
    });

  return (
    <Modal open={open} onClose={onClose} title={`${title} · 用量历史`} description="按 rate-limit 窗口周期统计的已用百分比（峰值）。每次刷新自动累积。">
      {loading ? (
        <div className="flex items-center justify-center gap-2 py-10 text-sm text-neutral-500">
          <Loader2 size={16} className="animate-spin" /> 加载中…
        </div>
      ) : records.length === 0 ? (
        <p className="rounded-xl border border-neutral-200 bg-neutral-50 px-3 py-6 text-center text-sm text-neutral-500">
          还没有用量记录。用这个账号跑一会儿 Codex，刷新后这里会按周期显示你消耗了多少。
        </p>
      ) : (
        <div className="space-y-4">
          {groups.map((kind) => {
            const rows = records.filter((r) => r.kind === kind);
            if (rows.length === 0) return null;
            return (
              <div key={kind} className="space-y-2">
                <h4 className="text-sm font-bold text-neutral-700">{windowTitle(rows[0])}</h4>
                <div className="space-y-1.5">
                  {rows.slice(0, 12).map((r) => (
                    <UsageRow key={`${r.kind}-${r.resetsAt}`} record={r} />
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </Modal>
  );
}

import { Clock3 } from "lucide-react";
import type { UsageSnapshot, UsageWindow } from "../../lib/types";
import { percent, relativeTime } from "../../lib/format";
import { cn } from "../../lib/cn";

function UsageLine({ window }: { window: UsageWindow }) {
  const value = Math.round(window.remainingPercent);
  const tone =
    window.isEstimatedRecovered || value >= 55
      ? "text-blue"
      : value >= 25
        ? "text-amber"
        : "text-red";

  return (
    <div className="min-w-[112px] rounded-2xl bg-blue/9 px-3 py-2">
      <div className="mb-2 flex items-center justify-between gap-3">
        <span className="text-sm font-black text-sub">{window.label}</span>
        <span className={cn("text-lg font-black tabular-nums", tone)}>{percent(value)}</span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-ink/9">
        <div
          className={cn(
            "h-full rounded-full",
            window.isEstimatedRecovered || value >= 55 ? "bg-blue" : value >= 25 ? "bg-amber" : "bg-red",
          )}
          style={{ width: `${Math.max(4, Math.min(100, value))}%` }}
        />
      </div>
      <div className="mt-2 truncate text-xs font-bold text-sub/70">{window.resetText}</div>
    </div>
  );
}

export function UsageBadge({ usage }: { usage?: UsageSnapshot | null }) {
  if (!usage) {
    return (
      <div className="flex min-w-[240px] items-center justify-end gap-3 text-right">
        <div>
          <div className="inline-flex items-center gap-2 rounded-full bg-black/6 px-3 py-2 text-sm font-bold text-sub">
            <Clock3 size={15} />
            待刷新
          </div>
          <div className="mt-2 text-xs font-semibold text-sub/55">运行一次后显示额度</div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-w-[260px] flex-col items-end gap-2">
      <div className="flex gap-2">
        {usage.primary ? <UsageLine window={usage.primary} /> : null}
        {usage.secondary ? <UsageLine window={usage.secondary} /> : null}
      </div>
      <div className="text-xs font-bold text-sub/55">
        {usage.hasEstimatedRecovery ? "含本地估算 · " : ""}
        同步 {relativeTime(usage.timestamp)}
      </div>
    </div>
  );
}

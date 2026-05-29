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
    <div className="w-[92px] rounded-xl bg-muted/70 px-2.5 py-2">
      <div className="mb-1.5 flex items-center justify-between gap-2">
        <span className="text-xs font-black text-sub">{window.label}</span>
        <span className={cn("text-base font-black tabular-nums", tone)}>{percent(value)}</span>
      </div>
      <div className="h-1.5 overflow-hidden rounded-full bg-ink/10">
        <div
          className={cn(
            "h-full rounded-full",
            window.isEstimatedRecovered || value >= 55 ? "bg-blue" : value >= 25 ? "bg-amber" : "bg-red",
          )}
          style={{ width: `${Math.max(4, Math.min(100, value))}%` }}
        />
      </div>
      <div className="mt-1.5 truncate text-[11px] font-bold text-sub/70">{window.resetText}</div>
    </div>
  );
}

export function UsageBadge({ usage }: { usage?: UsageSnapshot | null }) {
  if (!usage) {
    return (
      <div className="flex min-w-[190px] items-center justify-end gap-3 text-right">
        <div>
          <div className="inline-flex items-center gap-2 rounded-full bg-black/5 px-3 py-1.5 text-xs font-bold text-sub">
            <Clock3 size={15} />
            待刷新
          </div>
          <div className="mt-1.5 text-[11px] font-semibold text-sub/55">运行一次后显示额度</div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-w-[205px] flex-col items-end gap-1.5">
      <div className="flex gap-2">
        {usage.primary ? <UsageLine window={usage.primary} /> : null}
        {usage.secondary ? <UsageLine window={usage.secondary} /> : null}
      </div>
      <div className="text-[11px] font-bold text-sub/50">
        {usage.hasEstimatedRecovery ? "含本地估算 · " : ""}
        同步 {relativeTime(usage.timestamp)}
      </div>
    </div>
  );
}

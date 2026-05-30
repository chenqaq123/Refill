import { CheckCircle2, Edit3, KeyRound, Play, RefreshCw, Trash2 } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { Chip } from "../../components/ui/Chip";
import type { Profile, SwitchProgress } from "../../lib/types";
import { cn } from "../../lib/cn";
import { hostFromUrl, shortPath } from "../../lib/format";
import { UsageBadge } from "../usage/UsageBadge";

type ProfileCardProps = {
  profile: Profile;
  busy: boolean;
  progress?: SwitchProgress | null;
  selected: boolean;
  onSelect: () => void;
  onLaunch: () => void;
  onEdit?: () => void;
  onDelete?: () => void;
  onUsage?: () => void;
};

function avatarText(profile: Profile) {
  return profile.title.trim().charAt(0).toUpperCase() || (profile.kind === "api" ? "A" : "C");
}

export function ProfileCard({
  profile,
  busy,
  progress,
  selected,
  onSelect,
  onLaunch,
  onEdit,
  onDelete,
  onUsage,
}: ProfileCardProps) {
  const isApi = profile.kind === "api";
  const actionText = profile.isActive ? "重启" : "启动";

  return (
    <article
      className={cn(
        "card-hover grid grid-cols-[auto_minmax(260px,1fr)_auto_auto] items-center gap-4 rounded-2xl border bg-panel px-4 py-3.5",
        selected ? "border-blue/35 shadow-card ring-1 ring-blue/15" : "border-line",
        profile.isActive && "bg-blue/5",
      )}
      onClick={onSelect}
    >
      <div
        className={cn(
          "flex h-12 w-12 items-center justify-center rounded-xl text-lg font-black text-white shadow-[0_8px_20px_rgba(28,35,45,0.08)]",
          isApi ? "bg-teal" : "bg-blue",
        )}
      >
        {avatarText(profile)}
      </div>

      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <h3 className="min-w-0 truncate text-base font-black text-ink" title={profile.title}>
            {profile.title}
          </h3>
          {profile.isActive ? <CheckCircle2 className="shrink-0 text-green" size={18} /> : null}
        </div>
        <div className="mt-0.5 truncate text-sm font-semibold text-sub" title={profile.subtitle}>
          {profile.subtitle}
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-1.5">
          {profile.isActive ? (
            <Chip tone="green" solid icon={<CheckCircle2 size={14} />}>
              当前
            </Chip>
          ) : null}
          <Chip tone={isApi ? "teal" : "blue"}>{profile.primaryPill}</Chip>
          <Chip tone={profile.isReady ? "teal" : "amber"}>{profile.isReady ? "可启动" : "需检查"}</Chip>
          {isApi && profile.provider ? (
            <Chip tone={profile.provider.keyStatus === "exists" ? "green" : "red"} icon={<KeyRound size={13} />}>
              {profile.provider.keyStatus === "exists" ? "Key ready" : "缺少 Key"}
            </Chip>
          ) : null}
          {isApi && profile.provider ? <Chip tone="gray">{hostFromUrl(profile.provider.baseUrl)}</Chip> : null}
        </div>
      </div>

      {isApi ? (
        <div className="w-[220px] text-right">
          <div className="truncate text-sm font-black text-ink" title={profile.provider?.model}>
            {profile.provider?.model}
          </div>
          <div className="mt-0.5 truncate text-xs font-semibold text-sub" title={profile.provider?.baseUrl}>
            {profile.provider?.baseUrl}
          </div>
          <div className="mt-1 truncate text-[11px] font-semibold text-sub/55">{shortPath(profile.diagnostics.profilePath)}</div>
        </div>
      ) : onUsage ? (
        <button
          type="button"
          title="查看用量历史"
          className="rounded-2xl px-1 transition hover:bg-black/[0.03]"
          onClick={(event) => {
            event.stopPropagation();
            onUsage();
          }}
        >
          <UsageBadge usage={profile.usage} />
        </button>
      ) : (
        <UsageBadge usage={profile.usage} />
      )}

      <div className="flex items-center gap-1.5">
        {onEdit ? (
          <Button
            variant="ghost"
            className="h-9 w-9 px-0"
            onClick={(event) => {
              event.stopPropagation();
              onEdit();
            }}
            aria-label="编辑"
          >
            <Edit3 size={16} />
          </Button>
        ) : null}
        {onDelete && !profile.isActive ? (
          <Button
            variant="ghost"
            className="h-9 w-9 px-0 text-red hover:bg-red/10"
            onClick={(event) => {
              event.stopPropagation();
              onDelete();
            }}
            aria-label="删除"
          >
            <Trash2 size={16} />
          </Button>
        ) : null}
        <Button
          variant={profile.isActive ? "soft" : "primary"}
          className={cn("min-w-[96px]", profile.isActive && "text-teal bg-teal/10 hover:bg-teal/15")}
          disabled={busy || !profile.isReady}
          onClick={(event) => {
            event.stopPropagation();
            onLaunch();
          }}
          icon={busy ? <RefreshCw className="animate-spin" size={17} /> : profile.isActive ? <RefreshCw size={17} /> : <Play size={17} />}
        >
          {busy ? "切换中" : actionText}
        </Button>
      </div>

      {busy && progress ? (
        <div className="col-span-4 -mb-1 mt-1">
          <div className="mb-1.5 flex items-center justify-between text-xs font-bold text-sub">
            <span>{progress.message}</span>
            <span>{Math.round(progress.percent ?? 0)}%</span>
          </div>
          <div className="h-1.5 overflow-hidden rounded-full bg-blue/10">
            <div className="h-full rounded-full bg-blue transition-all" style={{ width: `${progress.percent ?? 8}%` }} />
          </div>
        </div>
      ) : null}
    </article>
  );
}

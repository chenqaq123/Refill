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
}: ProfileCardProps) {
  const isApi = profile.kind === "api";
  const actionText = profile.isActive ? "重启" : "启动";

  return (
    <article
      className={cn(
        "card-hover grid grid-cols-[auto_1fr_auto_auto] items-center gap-5 rounded-[24px] border bg-panel p-5",
        selected ? "border-blue/45 shadow-card" : "border-line",
        profile.isActive && "bg-blue/4",
      )}
      onClick={onSelect}
    >
      <div
        className={cn(
          "flex h-14 w-14 items-center justify-center rounded-2xl text-xl font-black text-white shadow-card",
          isApi ? "bg-teal" : "bg-blue",
        )}
      >
        {avatarText(profile)}
      </div>

      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <h3 className="truncate text-lg font-black text-ink">{profile.title}</h3>
          {profile.isActive ? <CheckCircle2 className="shrink-0 text-green" size={18} /> : null}
        </div>
        <div className="mt-1 truncate text-sm font-semibold text-sub">{profile.subtitle}</div>
        <div className="mt-3 flex flex-wrap items-center gap-2">
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
        <div className="min-w-[250px] text-right">
          <div className="text-sm font-black text-ink">{profile.provider?.model}</div>
          <div className="mt-1 truncate text-xs font-semibold text-sub">{profile.provider?.baseUrl}</div>
          <div className="mt-2 text-xs font-semibold text-sub/60">{shortPath(profile.diagnostics.profilePath)}</div>
        </div>
      ) : (
        <UsageBadge usage={profile.usage} />
      )}

      <div className="flex items-center gap-2">
        {onEdit ? (
          <Button
            variant="ghost"
            className="h-10 w-10 px-0"
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
            className="h-10 w-10 px-0 text-red hover:bg-red/10"
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
          className={cn("min-w-[108px]", profile.isActive && "text-teal bg-teal/10 hover:bg-teal/14")}
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
          <div className="mb-2 flex items-center justify-between text-xs font-bold text-sub">
            <span>{progress.message}</span>
            <span>{Math.round(progress.percent ?? 0)}%</span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-blue/10">
            <div className="h-full rounded-full bg-blue transition-all" style={{ width: `${progress.percent ?? 8}%` }} />
          </div>
        </div>
      ) : null}
    </article>
  );
}

import { AnimatePresence, motion } from "framer-motion";
import { KeyRound, Plus, RefreshCw, Search, Sparkles, UserRoundPlus } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { ProfileCard } from "../profiles/ProfileCard";
import type { Profile } from "../../lib/types";
import type { RefillStore } from "../../lib/useRefill";

type AccountsPageProps = {
  store: RefillStore;
  query: string;
  onQuery: (value: string) => void;
  searchRef: React.RefObject<HTMLInputElement>;
  onAddProvider: () => void;
  onEditProvider: (profile: Profile) => void;
  onSelectProfile: (profile: Profile) => void;
  onAccountUsage: (profile: Profile) => void;
};

const listVariants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.04 } },
};
const itemVariants = {
  hidden: { opacity: 0, y: 8 },
  show: { opacity: 1, y: 0, transition: { type: "spring" as const, stiffness: 480, damping: 36 } },
};

export function AccountsPage({
  store,
  query,
  onQuery,
  searchRef,
  onAddProvider,
  onEditProvider,
  onSelectProfile,
  onAccountUsage,
}: AccountsPageProps) {
  const { dashboard, busyProfileId, progress, launch, saveCurrent, deleteProvider, login } = store;
  const profiles = dashboard?.profiles ?? [];
  const matches = (profile: Profile) => {
    const q = query.trim().toLowerCase();
    if (!q) return true;
    return `${profile.title} ${profile.subtitle} ${profile.primaryPill}`.toLowerCase().includes(q);
  };
  const official = profiles.filter((p) => p.kind === "official" && matches(p));
  const apis = profiles.filter((p) => p.kind === "api" && matches(p));
  const hasAny = profiles.length > 0 || Boolean(dashboard?.unmanagedCurrent);

  return (
    <div className="mx-auto max-w-[1100px] space-y-5">
      <header className="flex items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-black tracking-tight">账号</h1>
          <p className="mt-0.5 text-sm font-semibold text-sub">
            {dashboard?.activeLabel ?? "未连接"} · 切换账号与 API，保留同一条历史
          </p>
        </div>
        <div className="flex shrink-0 gap-2">
          <Button variant="soft" icon={<UserRoundPlus size={17} />} onClick={login}>
            登录
          </Button>
          <Button variant="soft" className="bg-teal/10 text-teal hover:bg-teal/15" icon={<KeyRound size={17} />} onClick={onAddProvider}>
            API
          </Button>
          <Button variant="soft" icon={<RefreshCw size={17} />} onClick={() => store.refresh()}>
            同步
          </Button>
        </div>
      </header>

      {hasAny ? (
        <div className="relative">
          <Search size={16} className="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-sub/55" />
          <input
            ref={searchRef}
            value={query}
            onChange={(e) => onQuery(e.target.value)}
            placeholder="搜索账号 / provider…"
            className="h-10 w-full rounded-2xl border border-line bg-panel/70 pl-10 pr-16 text-sm font-semibold text-ink outline-none placeholder:text-sub/55 focus:border-blue/55 focus:ring-4 focus:ring-blue/10"
          />
          <kbd className="pointer-events-none absolute right-3.5 top-1/2 -translate-y-1/2 rounded-md bg-black/5 px-1.5 py-0.5 text-[11px] font-bold text-sub/55">/</kbd>
        </div>
      ) : null}

      {!hasAny ? (
        <motion.section
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="glass-panel rounded-3xl px-6 py-10 text-center"
        >
          <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-2xl bg-blue/10 text-blue">
            <Sparkles size={24} />
          </div>
          <h2 className="mt-4 text-lg font-black">欢迎使用 Refill</h2>
          <p className="mx-auto mt-1 max-w-[440px] text-sm font-semibold text-sub">
            在多个 Codex 账号与第三方 API 之间切换，并保留同一条对话历史。先连接一个账号开始：
          </p>
          <div className="mt-4 flex justify-center gap-2">
            <Button variant="primary" icon={<UserRoundPlus size={17} />} onClick={login}>
              登录官方账号
            </Button>
            <Button variant="soft" className="bg-teal/10 text-teal hover:bg-teal/15" icon={<KeyRound size={17} />} onClick={onAddProvider}>
              添加 API Provider
            </Button>
          </div>
        </motion.section>
      ) : null}

      {dashboard?.unmanagedCurrent ? (
        <Section title="待保存" count={1}>
          <ProfileCard
            profile={dashboard.unmanagedCurrent}
            busy={busyProfileId === "__current__"}
            selected={false}
            onSelect={() => onSelectProfile(dashboard.unmanagedCurrent!)}
            onLaunch={saveCurrent}
          />
        </Section>
      ) : null}

      {hasAny ? (
        <Section title="官方账号" count={official.length}>
          <motion.div variants={listVariants} initial="hidden" animate="show" className="space-y-2.5">
            <AnimatePresence>
              {official.map((profile) => (
                <motion.div key={profile.id} variants={itemVariants} layout exit={{ opacity: 0, y: -6 }}>
                  <ProfileCard
                    profile={profile}
                    busy={busyProfileId === profile.id}
                    progress={progress?.profileId === profile.id ? progress : null}
                    selected={false}
                    onSelect={() => onSelectProfile(profile)}
                    onLaunch={() => launch(profile)}
                    onUsage={() => onAccountUsage(profile)}
                  />
                </motion.div>
              ))}
            </AnimatePresence>
          </motion.div>
        </Section>
      ) : null}

      {hasAny ? (
        <Section title="API Providers" count={apis.length}>
          <motion.div variants={listVariants} initial="hidden" animate="show" className="space-y-2.5">
            <AnimatePresence>
              {apis.map((profile) => (
                <motion.div key={profile.id} variants={itemVariants} layout exit={{ opacity: 0, y: -6 }}>
                  <ProfileCard
                    profile={profile}
                    busy={busyProfileId === profile.id}
                    progress={progress?.profileId === profile.id ? progress : null}
                    selected={false}
                    onSelect={() => onSelectProfile(profile)}
                    onLaunch={() => launch(profile)}
                    onEdit={() => onEditProvider(profile)}
                    onDelete={!profile.isActive ? () => deleteProvider(profile) : undefined}
                  />
                </motion.div>
              ))}
            </AnimatePresence>
            {apis.length === 0 ? (
              <button
                className="pressable flex w-full items-center justify-center gap-3 rounded-2xl border border-dashed border-line bg-panel/70 p-6 text-sm font-bold text-sub hover:border-teal/35 hover:bg-teal/5"
                onClick={onAddProvider}
              >
                <Plus size={18} />
                添加第一个 API provider（DeepSeek / OpenRouter / Kimi …）
              </button>
            ) : null}
          </motion.div>
        </Section>
      ) : null}
    </div>
  );
}

function Section({ title, count, children }: { title: string; count: number; children: React.ReactNode }) {
  return (
    <section className="space-y-2.5">
      <div className="flex items-center gap-2 px-1">
        <h2 className="text-[13px] font-black text-sub">{title}</h2>
        <span className="rounded-full bg-black/5 px-2 py-0.5 text-[11px] font-black text-sub/55">{count}</span>
      </div>
      {children}
    </section>
  );
}

import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { CheckCircle2, Loader2, XCircle } from "lucide-react";
import { Sidebar, type PageId } from "../components/shell/Sidebar";
import { AccountsPage } from "../features/accounts/AccountsPage";
import { DetailDrawer } from "../features/accounts/DetailDrawer";
import { UsagePage } from "../features/usage/UsagePage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { ProviderDialog } from "../features/providers/ProviderDialog";
import { useRefill } from "../lib/useRefill";
import { relativeTime } from "../lib/format";
import type { AppId } from "../lib/apps";
import type { Profile, ProviderInput, ProviderUpdateInput } from "../lib/types";

export function App() {
  const store = useRefill();
  const [appId, setAppId] = useState<AppId>("codex");
  const [page, setPage] = useState<PageId>("accounts");
  const [query, setQuery] = useState("");
  const searchRef = useRef<HTMLInputElement>(null);

  const [providerDialogOpen, setProviderDialogOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Profile | null>(null);
  const [drawerProfile, setDrawerProfile] = useState<Profile | null>(null);
  const [usageTab, setUsageTab] = useState<"official" | "cost" | "log">("official");

  // Keep the drawer's data fresh after a refresh.
  const profiles = store.dashboard?.profiles ?? [];
  useEffect(() => {
    if (drawerProfile) {
      const next = profiles.find((p) => p.id === drawerProfile.id);
      if (next && next !== drawerProfile) setDrawerProfile(next);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [store.dashboard]);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      const typing = event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement;
      const mod = event.metaKey || event.ctrlKey;
      if (mod && event.key.toLowerCase() === "r") {
        event.preventDefault();
        store.refresh();
      } else if (mod && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setEditingProvider(null);
        setProviderDialogOpen(true);
      } else if (event.key === "/" && !typing && page === "accounts") {
        event.preventDefault();
        searchRef.current?.focus();
      } else if (event.key === "Escape" && event.target === searchRef.current) {
        setQuery("");
        searchRef.current?.blur();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [store, page]);

  async function submitProvider(input: ProviderInput | ProviderUpdateInput) {
    const ok = await store.submitProvider(editingProvider?.id ?? null, input);
    if (ok) {
      setProviderDialogOpen(false);
      setEditingProvider(null);
    }
  }

  async function copyDiagnostics() {
    await navigator.clipboard.writeText(JSON.stringify({ profile: drawerProfile, dashboard: store.dashboard }, null, 2));
    store.showToast("ok", "诊断信息已复制");
  }

  const lastSynced = store.dashboard?.lastSyncedAt ? relativeTime(store.dashboard.lastSyncedAt) : "从未";

  return (
    <div className="flex h-screen bg-canvas text-ink">
      <Sidebar
        appId={appId}
        onSelectApp={setAppId}
        page={page}
        onNavigate={setPage}
        footer={
          <div className="rounded-xl bg-muted/60 px-2.5 py-2 text-[11px] font-bold text-sub/70">
            <div className="truncate">同步 {lastSynced}</div>
            <div className="mt-0.5 truncate text-sub/55">{store.notice}</div>
          </div>
        }
      />

      <main className="flex-1 overflow-y-auto px-7 py-6">
        <AnimatePresence mode="wait">
          <motion.div
            key={page}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.2 }}
          >
            {page === "accounts" ? (
              <AccountsPage
                store={store}
                query={query}
                onQuery={setQuery}
                searchRef={searchRef}
                onAddProvider={() => {
                  setEditingProvider(null);
                  setProviderDialogOpen(true);
                }}
                onEditProvider={(profile) => {
                  setEditingProvider(profile);
                  setProviderDialogOpen(true);
                }}
                onSelectProfile={setDrawerProfile}
                onAccountUsage={() => {
                  setUsageTab("official");
                  setPage("usage");
                }}
              />
            ) : null}
            {page === "usage" ? <UsagePage store={store} initialTab={usageTab} /> : null}
            {page === "settings" ? <SettingsPage store={store} /> : null}
          </motion.div>
        </AnimatePresence>
      </main>

      <DetailDrawer
        profile={drawerProfile}
        dashboard={store.dashboard}
        onClose={() => setDrawerProfile(null)}
        onCopyDiagnostics={copyDiagnostics}
      />

      <ProviderDialog
        open={providerDialogOpen}
        profile={editingProvider}
        onClose={() => {
          setProviderDialogOpen(false);
          setEditingProvider(null);
        }}
        onSubmit={submitProvider}
      />

      <AnimatePresence>
        {store.toast ? (
          <motion.div
            className="fixed bottom-5 right-5 z-[60]"
            initial={{ opacity: 0, y: 12, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 12, scale: 0.96 }}
            transition={{ type: "spring", stiffness: 460, damping: 32 }}
          >
            <div
              className={`flex max-w-[380px] items-start gap-2.5 rounded-2xl px-4 py-3 text-sm font-bold text-white shadow-card ${
                store.toast.kind === "ok" ? "bg-[#0c7a4d]" : "bg-red"
              }`}
            >
              {store.toast.kind === "ok" ? <CheckCircle2 size={18} className="mt-0.5 shrink-0" /> : <XCircle size={18} className="mt-0.5 shrink-0" />}
              <span className="min-w-0 break-words">{store.toast.text}</span>
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>

      {!store.dashboard ? (
        <div className="fixed inset-0 flex items-center justify-center bg-canvas">
          <div className="flex items-center gap-3 rounded-2xl bg-panel px-5 py-4 text-sm font-bold text-sub shadow-card">
            <Loader2 className="animate-spin" size={18} /> 正在载入 Refill
          </div>
        </div>
      ) : null}
    </div>
  );
}

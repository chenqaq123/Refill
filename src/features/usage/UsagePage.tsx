import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import { Loader2, RefreshCw } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { api } from "../../lib/tauri";
import type { Profile, UsageSummary, UsageWindowRecord } from "../../lib/types";
import type { RefillStore } from "../../lib/useRefill";

type Tab = "official" | "cost" | "log";

type Price = { input: number; output: number };
type PriceMap = Record<string, Price>;
const PRICE_KEY = "refill.pricing.v1";
const DEFAULT_PRICES: PriceMap = {
  "deepseek-chat": { input: 0.27, output: 1.1 },
  "deepseek-reasoner": { input: 0.55, output: 2.19 },
};
function loadPrices(): PriceMap {
  try {
    return { ...DEFAULT_PRICES, ...JSON.parse(localStorage.getItem(PRICE_KEY) ?? "{}") };
  } catch {
    return { ...DEFAULT_PRICES };
  }
}
function fmtTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}
function cost(model: string, input: number, output: number, prices: PriceMap): number | null {
  const p = prices[model];
  if (!p) return null;
  return (input / 1_000_000) * p.input + (output / 1_000_000) * p.output;
}
function windowTitle(r: UsageWindowRecord): string {
  const m = r.windowMinutes;
  if (m >= 10080) return "周窗口（coding plan）";
  if (m >= 1440) return `${Math.round(m / 1440)} 天窗口`;
  if (m >= 60) return `${Math.round(m / 60)} 小时窗口`;
  return `${m} 分钟窗口`;
}
function fmtReset(resetsAt?: number | null): string {
  if (!resetsAt) return "—";
  return new Date(resetsAt * 1000).toLocaleString([], { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" });
}
export function UsagePage({ store, initialTab }: { store: RefillStore; initialTab?: Tab }) {
  const [tab, setTab] = useState<Tab>(initialTab ?? "official");
  const [summary, setSummary] = useState<UsageSummary | null>(null);
  const [log, setLog] = useState<string[]>([]);
  const [history, setHistory] = useState<Record<string, UsageWindowRecord[]>>({});
  const [loading, setLoading] = useState(false);
  const [prices, setPrices] = useState<PriceMap>(loadPrices);

  const official = (store.dashboard?.profiles ?? []).filter((p) => p.kind === "official");

  async function load() {
    setLoading(true);
    try {
      const [s, l] = await Promise.all([api.usageSummary(), api.readProxyLog()]);
      setSummary(s);
      setLog(l);
      const entries = await Promise.all(
        official.map(async (p) => [p.id, await api.accountUsageHistory(p.id).catch(() => [])] as const),
      );
      setHistory(Object.fromEntries(entries));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (initialTab) setTab(initialTab);
  }, [initialTab]);

  function setPrice(model: string, field: keyof Price, value: number) {
    setPrices((cur) => {
      const next = { ...cur, [model]: { ...(cur[model] ?? { input: 0, output: 0 }), [field]: value } };
      localStorage.setItem(PRICE_KEY, JSON.stringify(next));
      return next;
    });
  }

  const grandCost = useMemo(() => {
    let total = 0;
    let complete = true;
    summary?.providers.forEach((p) =>
      p.models.forEach((m) => {
        const c = cost(m.model, m.inputTokens, m.outputTokens, prices);
        if (c === null) complete = false;
        else total += c;
      }),
    );
    return { total, complete };
  }, [summary, prices]);

  const tabs: { id: Tab; label: string }[] = [
    { id: "official", label: "官方额度" },
    { id: "cost", label: "API 成本" },
    { id: "log", label: "请求日志" },
  ];

  return (
    <div className="mx-auto max-w-[1100px] space-y-5">
      <header className="flex items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-black tracking-tight">用量</h1>
          <p className="mt-0.5 text-sm font-semibold text-sub">官方账号额度窗口 · API token 成本 · 代理请求日志</p>
        </div>
        <Button variant="soft" icon={loading ? <Loader2 size={16} className="animate-spin" /> : <RefreshCw size={16} />} onClick={load}>
          刷新
        </Button>
      </header>

      <div className="flex w-fit rounded-2xl bg-muted/70 p-1">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`relative rounded-xl px-4 py-1.5 text-sm font-bold transition-colors ${tab === t.id ? "text-ink" : "text-sub hover:text-ink"}`}
          >
            {tab === t.id ? <motion.div layoutId="usage-tab" className="absolute inset-0 rounded-xl bg-panel shadow-card" /> : null}
            <span className="relative z-10">{t.label}</span>
          </button>
        ))}
      </div>

      <motion.div key={tab} initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.18 }}>
        {tab === "official" ? <OfficialTab official={official} history={history} /> : null}
        {tab === "cost" ? <CostTab summary={summary} prices={prices} setPrice={setPrice} grandCost={grandCost} /> : null}
        {tab === "log" ? <LogTab log={log} /> : null}
      </motion.div>
    </div>
  );
}

function fmtResetShort(resetsAt?: number | null): string {
  if (!resetsAt) return "";
  return new Date(resetsAt * 1000).toLocaleString([], { month: "numeric", day: "numeric" });
}

// Time-series bar chart for one rate-limit window kind. Oldest → newest L→R,
// bar height = peak used %, colored by threshold, current period highlighted.
function WindowChart({ records }: { records: UsageWindowRecord[] }) {
  if (records.length === 0) return null;
  const data = [...records].slice(0, 28).reverse(); // recent periods, time order
  const sparse = data.length > 8;
  const current = records.find((r) => r.isCurrent) ?? records[0];
  const currentUsed = Math.round(current.usedPercent);

  return (
    <div className="space-y-2">
      <div className="flex items-baseline justify-between">
        <span className="text-xs font-bold text-neutral-600">{windowTitle(records[0])}</span>
        <span className="text-xs font-semibold text-sub">
          本周期 <span className={`text-sm font-black ${currentUsed >= 90 ? "text-red" : currentUsed >= 70 ? "text-amber" : "text-blue"}`}>{currentUsed}%</span>
        </span>
      </div>

      <div className="relative flex h-[128px] items-end gap-[3px] rounded-xl border border-line/70 bg-muted/30 px-2 pb-0 pt-5">
        {/* 100% / 50% gridlines */}
        <div className="pointer-events-none absolute inset-x-2 top-5 border-t border-dashed border-line" />
        <div className="pointer-events-none absolute inset-x-2 top-[calc(50%+10px)] border-t border-dashed border-line/50" />
        <span className="pointer-events-none absolute right-2 top-0.5 text-[9px] font-bold text-sub/50">100%</span>

        {data.map((r) => {
          const used = Math.round(r.usedPercent);
          const color = used >= 90 ? "bg-red" : used >= 70 ? "bg-amber" : "bg-blue";
          return (
            <div
              key={`${r.kind}-${r.resetsAt}`}
              className="group relative flex h-full flex-1 items-end justify-center"
              title={`${fmtReset(r.resetsAt)} · ${used}%`}
            >
              {!sparse ? (
                <span className="absolute -top-4 text-[10px] font-bold text-sub">{used}%</span>
              ) : (
                <span className="absolute -top-4 text-[10px] font-bold text-ink opacity-0 transition-opacity group-hover:opacity-100">{used}%</span>
              )}
              <motion.div
                className={`w-full max-w-[30px] rounded-md ${color} ${r.isCurrent ? "ring-2 ring-blue/50 ring-offset-1" : ""}`}
                initial={{ height: 0 }}
                animate={{ height: `${Math.max(2, Math.min(100, used))}%` }}
                transition={{ duration: 0.5, ease: "easeOut" }}
              />
            </div>
          );
        })}
      </div>

      <div className="flex gap-[3px] px-2">
        {data.map((r, i) => (
          <div key={`${r.kind}-${r.resetsAt}-x`} className="flex-1 truncate text-center text-[9px] font-semibold text-sub/55">
            {!sparse || i === 0 || i === data.length - 1 ? fmtResetShort(r.resetsAt) : ""}
          </div>
        ))}
      </div>
    </div>
  );
}

function OfficialTab({ official, history }: { official: Profile[]; history: Record<string, UsageWindowRecord[]> }) {
  if (official.length === 0) {
    return <Empty>还没有官方账号。登录后这里会显示每个账号每个周期的额度消耗。</Empty>;
  }
  return (
    <div className="space-y-3">
      {official.map((p) => {
        const records = history[p.id] ?? [];
        const kinds = (["primary", "secondary"] as const)
          .filter((k) => records.some((r) => r.kind === k))
          .sort((a, b) => (records.find((r) => r.kind === b)?.windowMinutes ?? 0) - (records.find((r) => r.kind === a)?.windowMinutes ?? 0));
        return (
          <div key={p.id} className="rounded-2xl border border-line bg-panel p-4">
            <div className="flex items-center justify-between">
              <div className="min-w-0">
                <div className="truncate text-base font-black text-ink">{p.title}</div>
                <div className="truncate text-xs font-semibold text-sub">{p.subtitle}</div>
              </div>
              {p.isActive ? <span className="rounded-full bg-green/15 px-2 py-0.5 text-[11px] font-black text-green">当前</span> : null}
            </div>
            {records.length === 0 ? (
              <p className="mt-3 text-xs font-semibold text-sub/70">暂无记录，用这个账号跑一会儿后刷新。</p>
            ) : (
              <div className="mt-4 grid grid-cols-1 gap-5 md:grid-cols-2">
                {kinds.map((kind) => (
                  <WindowChart key={kind} records={records.filter((r) => r.kind === kind)} />
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

function CostTab({
  summary,
  prices,
  setPrice,
  grandCost,
}: {
  summary: UsageSummary | null;
  prices: PriceMap;
  setPrice: (model: string, field: keyof Price, value: number) => void;
  grandCost: { total: number; complete: boolean };
}) {
  return (
    <div className="space-y-4">
      <div className="grid grid-cols-3 gap-2">
        <Stat label="请求数" value={String(summary?.totalRequests ?? 0)} />
        <Stat label="Token（入/出）" value={`${fmtTokens(summary?.totalInputTokens ?? 0)} / ${fmtTokens(summary?.totalOutputTokens ?? 0)}`} />
        <Stat label="预估花费" value={`$${grandCost.total.toFixed(4)}${grandCost.complete ? "" : "+"}`} />
      </div>
      {!summary || summary.providers.length === 0 ? (
        <Empty>还没有 API 用量记录。用 API provider 发几条消息后再回来看。</Empty>
      ) : (
        <div className="space-y-3">
          {summary.providers.map((p) => (
            <div key={p.providerId} className="overflow-hidden rounded-2xl border border-line">
              <div className="flex items-center justify-between px-3 py-2">
                <span className="text-sm font-bold">{p.name}</span>
                <span className="text-xs text-sub">{p.requests} 次 · {fmtTokens(p.inputTokens)}→{fmtTokens(p.outputTokens)}</span>
              </div>
              <table className="w-full text-xs">
                <thead className="text-sub/60">
                  <tr className="border-t border-line/70">
                    <th className="px-3 py-1 text-left font-semibold">模型</th>
                    <th className="px-2 py-1 text-right font-semibold">入</th>
                    <th className="px-2 py-1 text-right font-semibold">出</th>
                    <th className="px-2 py-1 text-right font-semibold">$/1M 入</th>
                    <th className="px-2 py-1 text-right font-semibold">$/1M 出</th>
                    <th className="px-3 py-1 text-right font-semibold">花费</th>
                  </tr>
                </thead>
                <tbody>
                  {p.models.map((m) => {
                    const c = cost(m.model, m.inputTokens, m.outputTokens, prices);
                    return (
                      <tr key={m.model} className="border-t border-line/60">
                        <td className="px-3 py-1 font-mono">{m.model}</td>
                        <td className="px-2 py-1 text-right">{fmtTokens(m.inputTokens)}</td>
                        <td className="px-2 py-1 text-right">{fmtTokens(m.outputTokens)}</td>
                        <td className="px-2 py-1 text-right"><PriceInput value={prices[m.model]?.input} onChange={(v) => setPrice(m.model, "input", v)} /></td>
                        <td className="px-2 py-1 text-right"><PriceInput value={prices[m.model]?.output} onChange={(v) => setPrice(m.model, "output", v)} /></td>
                        <td className="px-3 py-1 text-right font-semibold">{c === null ? "—" : `$${c.toFixed(4)}`}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          ))}
          <p className="text-xs text-sub/55">提示：填好每个模型的单价（美元 / 100 万 token）即可看到花费。修改即时保存到本机。</p>
        </div>
      )}
    </div>
  );
}

function LogTab({ log }: { log: string[] }) {
  return (
    <div className="max-h-[520px] overflow-auto rounded-2xl border border-line bg-neutral-900 p-3">
      {log.length === 0 ? (
        <p className="text-sm text-neutral-400">暂无日志。</p>
      ) : (
        <pre className="whitespace-pre-wrap break-all font-mono text-[11px] leading-relaxed text-neutral-200">{log.join("\n")}</pre>
      )}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-line bg-panel px-3 py-2.5">
      <div className="text-[11px] font-semibold text-sub">{label}</div>
      <div className="mt-0.5 text-lg font-black text-ink">{value}</div>
    </div>
  );
}
function Empty({ children }: { children: React.ReactNode }) {
  return <p className="rounded-2xl border border-line bg-muted/60 px-3 py-8 text-center text-sm font-semibold text-sub">{children}</p>;
}
function PriceInput({ value, onChange }: { value?: number; onChange: (v: number) => void }) {
  return (
    <input
      type="number"
      step="0.01"
      min="0"
      value={value ?? ""}
      placeholder="—"
      onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
      className="w-16 rounded-md border border-line px-1.5 py-0.5 text-right text-xs outline-none focus:border-blue/50"
    />
  );
}

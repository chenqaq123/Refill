import { useEffect, useMemo, useState } from "react";
import { Loader2, RefreshCw } from "lucide-react";
import { Modal } from "../../components/ui/Modal";
import { Button } from "../../components/ui/Button";
import { api } from "../../lib/tauri";
import type { UsageSummary } from "../../lib/types";

type UsageDialogProps = {
  open: boolean;
  onClose: () => void;
};

// USD per 1M tokens. Editable by the user; persisted in localStorage.
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

export function UsageDialog({ open, onClose }: UsageDialogProps) {
  const [summary, setSummary] = useState<UsageSummary | null>(null);
  const [log, setLog] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [prices, setPrices] = useState<PriceMap>(loadPrices);
  const [tab, setTab] = useState<"cost" | "log">("cost");

  async function load() {
    setLoading(true);
    try {
      const [s, l] = await Promise.all([api.usageSummary(), api.readProxyLog()]);
      setSummary(s);
      setLog(l);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (open) load();
  }, [open]);

  const models = useMemo(() => {
    const set = new Set<string>();
    summary?.providers.forEach((p) => p.models.forEach((m) => set.add(m.model)));
    return Array.from(set);
  }, [summary]);

  function setPrice(model: string, field: keyof Price, value: number) {
    setPrices((current) => {
      const next = { ...current, [model]: { ...(current[model] ?? { input: 0, output: 0 }), [field]: value } };
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

  return (
    <Modal open={open} onClose={onClose} title="用量 & 成本" description="基于本地代理转发的真实 token 用量统计。单价可编辑，仅保存在本机。">
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <div className="flex rounded-xl bg-neutral-100 p-1">
            <button
              type="button"
              onClick={() => setTab("cost")}
              className={`rounded-lg px-3 py-1 text-sm font-semibold ${tab === "cost" ? "bg-white shadow-sm" : "text-neutral-500"}`}
            >
              成本
            </button>
            <button
              type="button"
              onClick={() => setTab("log")}
              className={`rounded-lg px-3 py-1 text-sm font-semibold ${tab === "log" ? "bg-white shadow-sm" : "text-neutral-500"}`}
            >
              请求日志
            </button>
          </div>
          <Button type="button" variant="ghost" icon={loading ? <Loader2 size={15} className="animate-spin" /> : <RefreshCw size={15} />} onClick={load}>
            刷新
          </Button>
        </div>

        {tab === "cost" ? (
          <div className="space-y-4">
            <div className="grid grid-cols-3 gap-2">
              <Stat label="请求数" value={String(summary?.totalRequests ?? 0)} />
              <Stat label="Token（入/出）" value={`${fmtTokens(summary?.totalInputTokens ?? 0)} / ${fmtTokens(summary?.totalOutputTokens ?? 0)}`} />
              <Stat label="预估花费" value={`$${grandCost.total.toFixed(4)}${grandCost.complete ? "" : "+"}`} />
            </div>

            {!summary || summary.providers.length === 0 ? (
              <p className="rounded-xl border border-neutral-200 bg-neutral-50 px-3 py-6 text-center text-sm text-neutral-500">
                还没有用量记录。用 API provider 发几条消息后再回来看。
              </p>
            ) : (
              <div className="space-y-3">
                {summary.providers.map((p) => (
                  <div key={p.providerId} className="rounded-xl border border-neutral-200">
                    <div className="flex items-center justify-between px-3 py-2">
                      <span className="text-sm font-bold">{p.name}</span>
                      <span className="text-xs text-neutral-500">{p.requests} 次 · {fmtTokens(p.inputTokens)}→{fmtTokens(p.outputTokens)}</span>
                    </div>
                    <table className="w-full text-xs">
                      <thead className="text-neutral-400">
                        <tr className="border-t border-neutral-100">
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
                            <tr key={m.model} className="border-t border-neutral-100">
                              <td className="px-3 py-1 font-mono">{m.model}</td>
                              <td className="px-2 py-1 text-right">{fmtTokens(m.inputTokens)}</td>
                              <td className="px-2 py-1 text-right">{fmtTokens(m.outputTokens)}</td>
                              <td className="px-2 py-1 text-right">
                                <PriceInput value={prices[m.model]?.input} onChange={(v) => setPrice(m.model, "input", v)} />
                              </td>
                              <td className="px-2 py-1 text-right">
                                <PriceInput value={prices[m.model]?.output} onChange={(v) => setPrice(m.model, "output", v)} />
                              </td>
                              <td className="px-3 py-1 text-right font-semibold">{c === null ? "—" : `$${c.toFixed(4)}`}</td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  </div>
                ))}
                {models.length > 0 && (
                  <p className="text-xs text-neutral-400">提示：填好每个模型的单价（美元 / 100 万 token）即可看到花费。修改即时保存。</p>
                )}
              </div>
            )}
          </div>
        ) : (
          <div className="max-h-[420px] overflow-auto rounded-xl border border-neutral-200 bg-neutral-900 p-3">
            {log.length === 0 ? (
              <p className="text-sm text-neutral-400">暂无日志。</p>
            ) : (
              <pre className="whitespace-pre-wrap break-all font-mono text-[11px] leading-relaxed text-neutral-200">
                {log.join("\n")}
              </pre>
            )}
          </div>
        )}
      </div>
    </Modal>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-neutral-200 bg-neutral-50 px-3 py-2">
      <div className="text-[11px] font-semibold text-neutral-500">{label}</div>
      <div className="mt-0.5 text-base font-black text-neutral-900">{value}</div>
    </div>
  );
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
      className="w-16 rounded-md border border-neutral-200 px-1.5 py-0.5 text-right text-xs outline-none focus:border-neutral-400"
    />
  );
}

import { FormEvent, useEffect, useState } from "react";
import { CheckCircle2, Loader2, XCircle, Zap } from "lucide-react";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/ui/Modal";
import { TextField } from "../../components/ui/TextField";
import { api } from "../../lib/tauri";
import type {
  Profile,
  ProviderInput,
  ProviderTestResult,
  ProviderUpdateInput,
  WireApi,
} from "../../lib/types";

type ProviderDialogProps = {
  open: boolean;
  profile?: Profile | null;
  onClose: () => void;
  onSubmit: (input: ProviderInput | ProviderUpdateInput) => void;
};

const PROTOCOLS: { value: WireApi; label: string; hint: string }[] = [
  {
    value: "responses",
    label: "Responses API（原生）",
    hint: "服务本身支持 OpenAI 的 /responses 接口（如 OpenAI 官方）。Codex 直接连接。",
  },
  {
    value: "chat",
    label: "Chat Completions（需本地翻译）",
    hint: "服务只支持 /chat/completions（如 DeepSeek）。Switcher 会通过内置代理把 Codex 的 Responses 请求翻译过去。",
  },
];

type Preset = { name: string; baseUrl: string; wireApi: WireApi; modelHint: string };

const PRESETS: Preset[] = [
  { name: "DeepSeek", baseUrl: "https://api.deepseek.com", wireApi: "chat", modelHint: "deepseek-chat" },
  { name: "OpenRouter", baseUrl: "https://openrouter.ai/api/v1", wireApi: "chat", modelHint: "openai/gpt-4.1" },
  { name: "Kimi", baseUrl: "https://api.moonshot.cn/v1", wireApi: "chat", modelHint: "kimi-k2" },
  { name: "Ollama", baseUrl: "http://127.0.0.1:11434/v1", wireApi: "chat", modelHint: "qwen2.5-coder" },
];

export function ProviderDialog({ open, profile, onClose, onSubmit }: ProviderDialogProps) {
  const provider = profile?.provider;
  const editing = Boolean(provider);
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [wireApi, setWireApi] = useState<WireApi>("responses");
  const [modelHint, setModelHint] = useState("deepseek-chat");
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<ProviderTestResult | null>(null);

  useEffect(() => {
    if (!open) return;
    setName(provider?.name ?? "");
    setBaseUrl(provider?.baseUrl ?? "");
    setModel(provider?.model ?? "");
    setApiKey("");
    setWireApi(provider?.wireApi ?? "responses");
    setModelHint(provider?.model ? provider.model : "deepseek-chat");
    setTesting(false);
    setTestResult(null);
  }, [open, provider]);

  function applyPreset(preset: Preset) {
    setName((current) => (current.trim() === "" ? preset.name : current));
    setBaseUrl(preset.baseUrl);
    setWireApi(preset.wireApi);
    setModelHint(preset.modelHint);
    setTestResult(null);
  }

  async function runTest() {
    setTesting(true);
    setTestResult(null);
    try {
      const result = await api.testProviderConnection({
        baseUrl,
        model,
        wireApi,
        apiKey: apiKey.trim() === "" ? null : apiKey,
        profileId: editing ? profile?.id ?? null : null,
      });
      setTestResult(result);
    } catch (error) {
      setTestResult({ ok: false, status: 0, latencyMs: 0, message: String(error), suggestChat: false });
    } finally {
      setTesting(false);
    }
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    onSubmit({
      name,
      baseUrl,
      model,
      apiKey: editing && apiKey.trim() === "" ? null : apiKey,
      wireApi,
    });
  }

  const isChat = wireApi === "chat";

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={editing ? "编辑 API Provider" : "添加 API Provider"}
      description="API Key 写入 macOS Keychain，不进入 config.toml。"
    >
      <form className="space-y-4" onSubmit={submit}>
        {!editing && (
          <div className="space-y-2">
            <span className="text-sm font-medium text-neutral-700">快速预设</span>
            <div className="flex flex-wrap gap-2">
              {PRESETS.map((preset) => (
                <button
                  key={preset.name}
                  type="button"
                  onClick={() => applyPreset(preset)}
                  className="rounded-full border border-neutral-200 bg-white px-3 py-1 text-xs font-semibold text-neutral-700 transition hover:border-neutral-900 hover:text-neutral-900"
                >
                  {preset.name}
                </button>
              ))}
            </div>
          </div>
        )}

        <TextField label="名称" value={name} onChange={(event) => setName(event.target.value)} placeholder="DeepSeek" required />

        <div className="space-y-2">
          <span className="text-sm font-medium text-neutral-700">API 协议</span>
          <div className="grid grid-cols-2 gap-2">
            {PROTOCOLS.map((option) => {
              const active = wireApi === option.value;
              return (
                <button
                  key={option.value}
                  type="button"
                  onClick={() => {
                    setWireApi(option.value);
                    setTestResult(null);
                  }}
                  className={`rounded-xl border px-3 py-2 text-left text-sm transition ${
                    active
                      ? "border-neutral-900 bg-neutral-900 text-white"
                      : "border-neutral-200 bg-white text-neutral-700 hover:border-neutral-300"
                  }`}
                >
                  {option.label}
                </button>
              );
            })}
          </div>
          <p className="text-xs text-neutral-500">{PROTOCOLS.find((option) => option.value === wireApi)?.hint}</p>
        </div>

        {isChat && (
          <div className="space-y-1 rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800">
            <p className="font-medium">本地翻译模式</p>
            <p>
              Codex 只会说 Responses API，而 DeepSeek 等仅支持 Chat Completions。已为该 provider 启用 Switcher 内置代理做协议转换。
            </p>
            <p>⚠️ 使用此账号期间请保持 Switcher 运行，否则 Codex 将无法连接。</p>
          </div>
        )}

        <TextField
          label="Base URL"
          value={baseUrl}
          onChange={(event) => {
            setBaseUrl(event.target.value);
            setTestResult(null);
          }}
          placeholder={isChat ? "https://api.deepseek.com" : "https://openrouter.ai/api/v1"}
          required
        />
        <TextField
          label="Model"
          value={model}
          onChange={(event) => setModel(event.target.value)}
          placeholder={modelHint}
          hint="模型名以你的输入为准；代理会用这里填的模型请求上游。"
          required
        />
        <TextField
          label="API Key"
          type="password"
          value={apiKey}
          onChange={(event) => setApiKey(event.target.value)}
          placeholder={editing ? "留空则不修改" : "sk-..."}
          required={!editing}
        />

        <div className="space-y-2">
          <Button
            type="button"
            variant="soft"
            icon={testing ? <Loader2 size={15} className="animate-spin" /> : <Zap size={15} />}
            onClick={runTest}
            disabled={testing || baseUrl.trim() === "" || model.trim() === ""}
          >
            {testing ? "测试中…" : "测试连接"}
          </Button>
          {testResult && (
            <div
              className={`flex items-start gap-2 rounded-xl border px-3 py-2 text-xs ${
                testResult.ok
                  ? "border-emerald-200 bg-emerald-50 text-emerald-800"
                  : "border-red-200 bg-red-50 text-red-700"
              }`}
            >
              {testResult.ok ? <CheckCircle2 size={15} className="mt-0.5 shrink-0" /> : <XCircle size={15} className="mt-0.5 shrink-0" />}
              <div className="space-y-1">
                <p className="break-all">{testResult.message}</p>
                {testResult.suggestChat && (
                  <button
                    type="button"
                    onClick={() => {
                      setWireApi("chat");
                      setTestResult(null);
                    }}
                    className="font-semibold underline"
                  >
                    切换到 Chat Completions →
                  </button>
                )}
              </div>
            </div>
          )}
        </div>

        <div className="flex justify-end gap-2 pt-2">
          <Button type="button" variant="ghost" onClick={onClose}>
            取消
          </Button>
          <Button type="submit" variant="primary">
            {editing ? "保存" : "创建"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

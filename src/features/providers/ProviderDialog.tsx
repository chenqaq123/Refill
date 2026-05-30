import { FormEvent, useEffect, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/ui/Modal";
import { TextField } from "../../components/ui/TextField";
import type { Profile, ProviderInput, ProviderUpdateInput, WireApi } from "../../lib/types";

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

export function ProviderDialog({ open, profile, onClose, onSubmit }: ProviderDialogProps) {
  const provider = profile?.provider;
  const editing = Boolean(provider);
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [wireApi, setWireApi] = useState<WireApi>("responses");

  useEffect(() => {
    if (!open) return;
    setName(provider?.name ?? "");
    setBaseUrl(provider?.baseUrl ?? "");
    setModel(provider?.model ?? "gpt-4.1");
    setApiKey("");
    setWireApi(provider?.wireApi ?? "responses");
  }, [open, provider]);

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
                  onClick={() => setWireApi(option.value)}
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
              Codex 只会说 Responses API，而 DeepSeek 等仅支持 Chat Completions。已为该 provider 启用 Switcher 内置代理
              （127.0.0.1:8765）做协议转换。
            </p>
            <p>⚠️ 使用此账号期间请保持 Switcher 运行，否则 Codex 将无法连接。</p>
            <p className="text-amber-700">
              示例：Base URL <code className="font-mono">https://api.deepseek.com</code>，Model{" "}
              <code className="font-mono">deepseek-chat</code> 或 <code className="font-mono">deepseek-reasoner</code>。
            </p>
          </div>
        )}

        <TextField
          label="Base URL"
          value={baseUrl}
          onChange={(event) => setBaseUrl(event.target.value)}
          placeholder={isChat ? "https://api.deepseek.com" : "https://openrouter.ai/api/v1"}
          required
        />
        <TextField
          label="Model"
          value={model}
          onChange={(event) => setModel(event.target.value)}
          placeholder={isChat ? "deepseek-chat" : "openai/gpt-4.1"}
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

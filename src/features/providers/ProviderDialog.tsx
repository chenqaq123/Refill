import { FormEvent, useEffect, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Modal } from "../../components/ui/Modal";
import { TextField } from "../../components/ui/TextField";
import type { Profile, ProviderInput, ProviderUpdateInput } from "../../lib/types";

type ProviderDialogProps = {
  open: boolean;
  profile?: Profile | null;
  onClose: () => void;
  onSubmit: (input: ProviderInput | ProviderUpdateInput) => void;
};

export function ProviderDialog({ open, profile, onClose, onSubmit }: ProviderDialogProps) {
  const provider = profile?.provider;
  const editing = Boolean(provider);
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");

  useEffect(() => {
    if (!open) return;
    setName(provider?.name ?? "");
    setBaseUrl(provider?.baseUrl ?? "");
    setModel(provider?.model ?? "gpt-4.1");
    setApiKey("");
  }, [open, provider]);

  function submit(event: FormEvent) {
    event.preventDefault();
    onSubmit({
      name,
      baseUrl,
      model,
      apiKey: editing && apiKey.trim() === "" ? null : apiKey,
    });
  }

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={editing ? "编辑 API Provider" : "添加 API Provider"}
      description="仅支持 Responses-compatible API；Key 会写入 macOS Keychain，不进入 config.toml。"
    >
      <form className="space-y-4" onSubmit={submit}>
        <TextField label="名称" value={name} onChange={(event) => setName(event.target.value)} placeholder="OpenRouter" required />
        <TextField
          label="Base URL"
          value={baseUrl}
          onChange={(event) => setBaseUrl(event.target.value)}
          placeholder="https://openrouter.ai/api/v1"
          required
        />
        <TextField label="Model" value={model} onChange={(event) => setModel(event.target.value)} placeholder="openai/gpt-4.1" required />
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

import { useState } from "react";
import { api } from "@/lib/ipc";
import type { Provider } from "@/lib/types";
import { Button, Field, Modal, Select, TextInput } from "./ui";

const TYPE_PRESETS = [
  { value: "openai", label: "openai（兼容 OpenAI 格式）" },
  { value: "anthropic", label: "anthropic" },
  { value: "kimi", label: "kimi" },
  { value: "gemini", label: "gemini" },
];

export function ProviderEditor({
  initial,
  onClose,
  onSaved,
}: {
  initial?: Provider;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [name, setName] = useState(initial?.name ?? "");
  const [baseUrl, setBaseUrl] = useState(initial?.baseUrl ?? "");
  const [apiKey, setApiKey] = useState("");
  const [providerType, setProviderType] = useState(
    initial?.providerType ?? "openai",
  );
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await api.providers.upsert({
        name,
        originalName: initial?.name ?? null,
        baseUrl,
        apiKey: apiKey.trim() || null,
        providerType,
      });
      onSaved();
      onClose();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      title={initial ? `编辑 Provider · ${initial.name}` : "添加 Provider"}
      subtitle={
        initial
          ? "改名会同步更新其下所有模型的 provider 字段"
          : "填好端点与密钥后，再到该 provider 下添加模型"
      }
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            取消
          </Button>
          <Button variant="primary" onClick={save} disabled={saving}>
            {saving ? "保存中…" : "保存"}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-3">
        {error && (
          <div className="rounded-md border border-danger-bd bg-danger-soft px-3 py-2 text-xs text-danger">
            {error}
          </div>
        )}
        <Field label="名称" required hint="仅用于本工具与配置里的标识，可自定义">
          <TextInput
            value={name}
            onChange={setName}
            placeholder="我的 GLM 中转"
          />
        </Field>
        <Field
          label="Base URL"
          required
          hint="如 https://open.bigmodel.cn/api/paas/v4"
        >
          <TextInput mono value={baseUrl} onChange={setBaseUrl} placeholder="https://..." />
        </Field>
        <Field
          label="API Key"
          hint={
            initial?.apiKeyMasked
              ? `当前：${initial.apiKeyMasked}。留空表示不修改`
              : "留空表示该 provider 不需要密钥"
          }
        >
          <TextInput
            type="password"
            mono
            value={apiKey}
            onChange={setApiKey}
            placeholder="sk-..."
          />
        </Field>
        <Field label="接口类型" hint="决定请求格式，绝大多数中转用 openai">
          <Select
            value={providerType}
            onChange={setProviderType}
            options={TYPE_PRESETS}
          />
        </Field>
      </div>
    </Modal>
  );
}

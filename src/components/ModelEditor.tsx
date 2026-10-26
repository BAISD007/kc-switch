import { useEffect, useMemo, useState } from "react";
import { api } from "@/lib/ipc";
import {
  CAPABILITY_LABELS,
  EFFORT_LABELS,
  type MetaInfo,
  type ModelEntry,
  type ModelIssue,
} from "@/lib/types";
import {
  Button,
  Checkbox,
  Field,
  Modal,
  SectionLabel,
  Select,
  TextInput,
} from "./ui";

export function emptyModel(provider: string): ModelEntry {
  return {
    id: "",
    provider,
    model: "",
    displayName: "",
    maxContextSize: null,
    maxOutputSize: null,
    capabilities: [],
    supportEfforts: [],
    defaultEffort: null,
    adaptiveThinking: null,
    reasoningKey: null,
  };
}

const CONTEXT_PRESETS = [
  { label: "128K", value: 131072 },
  { label: "200K", value: 204800 },
  { label: "256K", value: 262144 },
  { label: "1M", value: 1048576 },
];

export function ModelEditor({
  initial,
  originalId,
  providers,
  meta,
  onClose,
  onSaved,
}: {
  initial: ModelEntry;
  originalId?: string;
  providers: string[];
  meta: MetaInfo | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [entry, setEntry] = useState<ModelEntry>(initial);
  const [issues, setIssues] = useState<ModelIssue[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [idTouched, setIdTouched] = useState(Boolean(originalId));

  const patch = (p: Partial<ModelEntry>) => setEntry((e) => ({ ...e, ...p }));

  // 自动补全模型 ID：provider/model，除非用户手动改过
  useEffect(() => {
    if (idTouched) return;
    if (entry.provider && entry.model && !entry.model.includes("/")) {
      patch({ id: `${entry.provider}/${entry.model}` });
    }
  }, [entry.provider, entry.model, idTouched]);

  // 实时诊断
  useEffect(() => {
    const t = setTimeout(() => {
      api.providers
        .diagnose(entry)
        .then(setIssues)
        .catch(() => setIssues([]));
    }, 200);
    return () => clearTimeout(t);
  }, [entry]);

  const caps = meta?.knownCapabilities ?? Object.keys(CAPABILITY_LABELS);
  const efforts = meta?.knownEfforts ?? ["low", "medium", "high", "max"];

  const toggleIn = (list: string[], value: string) =>
    list.includes(value) ? list.filter((v) => v !== value) : [...list, value];

  const effectiveCapLabel = (key: string) =>
    CAPABILITY_LABELS[key]?.label ?? key;
  const effectiveCapHint = (key: string) =>
    CAPABILITY_LABELS[key]?.hint ?? "";
  const effortLabel = (key: string) => EFFORT_LABELS[key] ?? key;

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await api.providers.upsertModel(entry, originalId ?? null);
      onSaved();
      onClose();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setSaving(false);
    }
  };

  const byLevel = useMemo(() => {
    const rank = { error: 0, warn: 1, info: 2 } as const;
    return [...issues].sort((a, b) => rank[a.level] - rank[b.level]);
  }, [issues]);

  return (
    <Modal
      wide
      title={originalId ? "编辑模型" : "添加模型"}
      subtitle="这些字段决定模型能否识图、思考强度是否生效、上下文上限等"
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
      <div className="flex flex-col gap-4">
        {error && (
          <div className="rounded-md border border-danger-bd bg-danger-soft px-3 py-2 text-xs text-danger">
            {error}
          </div>
        )}

        <div className="grid grid-cols-2 gap-3">
          <Field label="所属 Provider" required hint="决定走哪个 API 端点与密钥">
            <Select
              value={entry.provider}
              onChange={(v) => patch({ provider: v })}
              options={providers.map((p) => ({ value: p, label: p }))}
            />
          </Field>
          <Field
            label="模型名（发给 API 的 model）"
            required
            hint="如 glm-4.6、deepseek-v4-pro"
          >
            <TextInput
              mono
              value={entry.model}
              onChange={(v) => patch({ model: v })}
              placeholder="glm-4.6"
            />
          </Field>
        </div>

        <Field
          label="显示名称"
          hint="选模型时在列表里看到的名字"
        >
          <TextInput
            value={entry.displayName ?? ""}
            onChange={(v) => patch({ displayName: v })}
            placeholder="GLM-4.6"
          />
        </Field>

        <Field
          label="模型 ID（配置里的 key）"
          hint="形如 provider/model；留空会自动生成"
        >
          <TextInput
            mono
            value={entry.id}
            onChange={(v) => {
              setIdTouched(true);
              patch({ id: v });
            }}
            placeholder={`${entry.provider || "provider"}/${entry.model || "model"}`}
          />
        </Field>

        <div
          className="rounded-lg bg-surface-raised p-3"
          style={{ border: "var(--p-hairline) solid var(--color-line)" }}
        >
          <SectionLabel hint="图片识别不生效，多半是这里没勾 image_in">
            能力（capabilities）
          </SectionLabel>
          <div className="grid grid-cols-2 gap-2">
            {caps.map((c) => (
              <Checkbox
                key={c}
                label={effectiveCapLabel(c)}
                hint={effectiveCapHint(c)}
                checked={entry.capabilities.includes(c)}
                onChange={() =>
                  patch({ capabilities: toggleIn(entry.capabilities, c) })
                }
              />
            ))}
          </div>
          {entry.capabilities.some((c) => !caps.includes(c)) && (
            <div className="mt-2 text-[10px] text-ink-faint">
              文件中还有未知能力：
              {entry.capabilities.filter((c) => !caps.includes(c)).join(", ")}
            </div>
          )}
        </div>

        <div
          className="rounded-lg bg-surface-raised p-3"
          style={{ border: "var(--p-hairline) solid var(--color-line)" }}
        >
          <SectionLabel hint="决定可选的低/中/高/最高档">
            思考强度档位（support_efforts）
          </SectionLabel>
          <div className="flex flex-wrap gap-2">
            {efforts.map((e) => {
              const on = entry.supportEfforts.includes(e);
              return (
                <button
                  key={e}
                  onClick={() =>
                    patch({ supportEfforts: toggleIn(entry.supportEfforts, e) })
                  }
                  className={`h-7 rounded-md px-3 text-[11px] font-medium transition-colors duration-[var(--duration-fast)] ${
                    on
                      ? "bg-accent-soft text-accent ring-1 ring-inset ring-accent-bd"
                      : "bg-surface-sunken text-ink-faint hover:bg-hover hover:text-ink"
                  }`}
                >
                  {effortLabel(e)}
                </button>
              );
            })}
          </div>
          <div className="mt-3 grid grid-cols-2 gap-3">
            <Field label="默认档位" hint="不填则用 Kimi Code 的默认行为">
              <Select
                allowEmpty
                value={entry.defaultEffort ?? ""}
                onChange={(v) => patch({ defaultEffort: v || null })}
                options={(entry.supportEfforts.length
                  ? entry.supportEfforts
                  : efforts
                ).map((e) => ({ value: e, label: effortLabel(e) }))}
              />
            </Field>
            <Field label="自适应思考" hint="adaptive_thinking">
              <Select
                allowEmpty
                value={
                  entry.adaptiveThinking === null ||
                  entry.adaptiveThinking === undefined
                    ? ""
                    : entry.adaptiveThinking
                      ? "true"
                      : "false"
                }
                onChange={(v) =>
                  patch({
                    adaptiveThinking:
                      v === "" ? null : v === "true",
                  })
                }
                options={[
                  { value: "true", label: "开启" },
                  { value: "false", label: "关闭" },
                ]}
              />
            </Field>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <Field label="上下文长度" hint="max_context_size，单位 token">
            <div className="flex gap-1.5">
              <TextInput
                type="number"
                value={entry.maxContextSize?.toString() ?? ""}
                onChange={(v) =>
                  patch({ maxContextSize: v ? Number(v) : null })
                }
                placeholder="1048576"
              />
              <div className="flex gap-1">
                {CONTEXT_PRESETS.map((p) => (
                  <button
                    key={p.value}
                    onClick={() => patch({ maxContextSize: p.value })}
                    className="h-7 shrink-0 rounded-md px-1.5 text-[10px] text-ink-faint transition-colors hover:bg-hover hover:text-accent"
                  >
                    {p.label}
                  </button>
                ))}
              </div>
            </div>
          </Field>
          <Field label="最大输出长度" hint="max_output_size，可留空">
            <TextInput
              type="number"
              value={entry.maxOutputSize?.toString() ?? ""}
              onChange={(v) => patch({ maxOutputSize: v ? Number(v) : null })}
              placeholder="可选"
            />
          </Field>
        </div>

        <Field label="推理字段名" hint="reasoning_key，少数供应商需要，可留空">
          <TextInput
            mono
            value={entry.reasoningKey ?? ""}
            onChange={(v) => patch({ reasoningKey: v })}
            placeholder="可选"
          />
        </Field>

        {byLevel.length > 0 && (
          <div className="rounded-lg border border-attention/30 bg-attention-soft/60 p-3">
            <div className="mb-1.5 text-[11px] font-medium text-attention">
              配置检查（{byLevel.length} 条提示，不影响保存）
            </div>
            <ul className="flex flex-col gap-1">
              {byLevel.map((i, idx) => (
                <li
                  key={idx}
                  className={`text-[10px] leading-relaxed ${
                    i.level === "error"
                      ? "text-danger"
                      : i.level === "warn"
                        ? "text-attention"
                        : "text-ink-faint"
                  }`}
                >
                  · {i.message}
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </Modal>
  );
}

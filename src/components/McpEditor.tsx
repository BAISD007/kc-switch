import { useState } from "react";
import { api } from "@/lib/ipc";
import type { McpServer } from "@/lib/types";
import {
  Button,
  Checkbox,
  Field,
  Modal,
  Select,
  TextArea,
  TextInput,
} from "./ui";

export function McpEditor({
  initial,
  onClose,
  onSaved,
}: {
  initial?: McpServer;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [name, setName] = useState(initial?.name ?? "");
  const [transport, setTransport] = useState<"stdio" | "http">(
    initial?.transport ?? "stdio",
  );
  const [command, setCommand] = useState(initial?.command ?? "");
  const [args, setArgs] = useState((initial?.args ?? []).join("\n"));
  const [url, setUrl] = useState(initial?.url ?? "");
  const [bearerVar, setBearerVar] = useState(initial?.bearerTokenEnvVar ?? "");
  const [envText, setEnvText] = useState("");
  const [deferred, setDeferred] = useState(initial?.deferred ?? false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const parseEnv = (): [string, string][] =>
    envText
      .split("\n")
      .map((l) => l.trim())
      .filter((l) => l && l.includes("="))
      .map((l) => {
        const i = l.indexOf("=");
        return [l.slice(0, i).trim(), l.slice(i + 1).trim()] as [string, string];
      });

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await api.mcp.upsert({
        name,
        transport,
        command: transport === "stdio" ? command : null,
        args:
          transport === "stdio"
            ? args
                .split("\n")
                .map((a) => a.trim())
                .filter(Boolean)
            : [],
        url: transport === "http" ? url : null,
        bearerTokenEnvVar:
          transport === "http" ? bearerVar.trim() || null : null,
        env: parseEnv(),
        deferred,
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
      title={initial ? `编辑 MCP · ${initial.name}` : "添加 MCP 服务器"}
      subtitle="stdio 用命令启动，http 连远程端点"
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
        <div className="grid grid-cols-2 gap-3">
          <Field label="名称" required hint="同时作为配置里的 key">
            <TextInput value={name} onChange={setName} placeholder="my-mcp" />
          </Field>
          <Field label="传输方式">
            <Select
              value={transport}
              onChange={(v) => setTransport(v as "stdio" | "http")}
              options={[
                { value: "stdio", label: "stdio（本地进程）" },
                { value: "http", label: "http（远程服务）" },
              ]}
            />
          </Field>
        </div>

        {transport === "stdio" ? (
          <>
            <Field label="命令" required hint="如 npx、uvx 或可执行文件路径">
              <TextInput mono value={command} onChange={setCommand} placeholder="npx" />
            </Field>
            <Field label="参数" hint="每行一个参数">
              <TextArea
                value={args}
                onChange={setArgs}
                rows={3}
                placeholder={"-y\n@upstash/context7-mcp@latest"}
              />
            </Field>
          </>
        ) : (
          <>
            <Field label="URL" required hint="MCP 端点地址">
              <TextInput mono value={url} onChange={setUrl} placeholder="https://..." />
            </Field>
            <Field
              label="Bearer 令牌环境变量名"
              hint="只存变量名，不存令牌本身，如 ANYSEARCH_API_KEY"
            >
              <TextInput
                mono
                value={bearerVar}
                onChange={setBearerVar}
                placeholder="MY_API_KEY"
              />
            </Field>
          </>
        )}

        <Field
          label={transport === "stdio" ? "环境变量" : "额外请求头"}
          hint={
            transport === "stdio"
              ? "每行 KEY=VALUE，可留空"
              : "每行 Header: VALUE"
          }
        >
          <TextArea
            value={envText}
            onChange={setEnvText}
            rows={2}
            placeholder={
              transport === "stdio" ? "API_KEY=xxx" : "X-Custom: value"
            }
          />
        </Field>

        <Checkbox
          checked={deferred}
          onChange={setDeferred}
          label="延迟加载（deferred）"
          hint="不在启动时拉起，按需加载，减少启动开销"
        />
      </div>
    </Modal>
  );
}

import { useEffect, useState } from "react";
import { api } from "@/lib/ipc";
import type { DiscoverResult, DiscoveredModel, ModelEntry } from "@/lib/types";
import { Button, Empty, Modal, Spinner } from "./ui";

export function DiscoverDialog({
  providerName,
  onClose,
  onImported,
  onEdit,
}: {
  providerName: string;
  onClose: () => void;
  onImported: () => void;
  onEdit: (entry: ModelEntry, existingId?: string) => void;
}) {
  const [result, setResult] = useState<DiscoverResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [importing, setImporting] = useState(false);
  const [hideExisting, setHideExisting] = useState(true);

  const run = async (refresh: boolean) => {
    setLoading(true);
    setError(null);
    try {
      const r = await api.providers.discoverModels(providerName, refresh);
      setResult(r);
      // 默认全选「还没导入过」的模型
      setSelected(
        new Set(r.models.filter((m) => !m.exists).map((m) => m.remoteId)),
      );
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  };

  // 首次打开自动拉取
  useEffect(() => {
    void run(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const shown = (result?.models ?? []).filter((m) =>
    hideExisting ? !m.exists : true,
  );

  const toggle = (id: string) => {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelected(next);
  };

  const importSelected = async () => {
    if (!result) return;
    const entries = result.models
      .filter((m) => selected.has(m.remoteId))
      .map((m) => m.entry);
    if (entries.length === 0) return;
    setImporting(true);
    try {
      await api.providers.importModels(entries);
      onImported();
      onClose();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setImporting(false);
    }
  };

  return (
    <Modal
      wide
      title={`获取 ${providerName} 的模型列表`}
      subtitle="自动补全上下文长度、图片识别与思考档位；可先编辑再导入"
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={() => run(true)} disabled={loading}>
            刷新目录
          </Button>
          <div className="flex-1" />
          <Button variant="ghost" onClick={onClose}>
            关闭
          </Button>
          <Button
            variant="primary"
            onClick={importSelected}
            disabled={importing || selected.size === 0}
          >
            {importing ? "导入中…" : `导入选中的 ${selected.size} 个`}
          </Button>
        </>
      }
    >
      {loading && <Spinner label="正在拉取模型列表并匹配元数据…" />}

      {error && (
        <div className="rounded-md border border-danger-bd bg-danger-soft px-3 py-3">
          <div className="text-xs leading-relaxed text-danger">{error}</div>
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <Button size="sm" variant="secondary" onClick={() => run(false)}>
              重试
            </Button>
            <Button size="sm" variant="ghost" onClick={() => run(true)}>
              刷新目录并重试
            </Button>
            <span className="text-[10px] leading-relaxed text-ink-faint">
              自动读取系统代理；若该域名不该走代理，请在代理软件里设为直连
            </span>
          </div>
        </div>
      )}

      {!loading && result && (
        <div className="flex flex-col gap-3">
          {result.notes.length > 0 && (
            <details className="rounded-md bg-surface-sunken px-3 py-2">
              <summary className="cursor-pointer select-none text-[10px] text-ink-faint hover:text-ink-muted">
                网络与数据来源（点击展开）
              </summary>
              <ul className="mt-1.5 flex flex-col gap-0.5">
                {result.notes.map((n, i) => (
                  <li
                    key={i}
                    className={`text-[10px] leading-relaxed ${
                      n.includes("成功")
                        ? "text-success"
                        : n.includes("失败") || n.includes("错误")
                          ? "text-attention"
                          : "text-ink-faint"
                    }`}
                  >
                    · {n}
                  </li>
                ))}
              </ul>
            </details>
          )}

          <div className="flex items-center justify-between">
            <label className="flex cursor-pointer items-center gap-2 text-[11px] text-ink-muted">
              <input
                type="checkbox"
                checked={hideExisting}
                onChange={(e) => setHideExisting(e.target.checked)}
                className="h-3.5 w-3.5 cursor-pointer accent-accent"
              />
              隐藏已导入的模型
            </label>
            <div className="flex items-center gap-3 text-[10px] text-ink-faint">
              <span>共 {result.models.length} 个</span>
              <button
                onClick={() => setSelected(new Set(shown.map((m) => m.remoteId)))}
                className="hover:text-accent"
              >
                全选
              </button>
              <button
                onClick={() => setSelected(new Set())}
                className="hover:text-accent"
              >
                清空
              </button>
            </div>
          </div>

          {shown.length === 0 ? (
            <Empty text="没有可导入的模型（可能都已存在）" />
          ) : (
            <div
              className="overflow-hidden rounded-lg bg-surface-raised"
              style={{ border: "var(--p-hairline) solid var(--color-line)" }}
            >
              {shown.map((m) => (
                <Row
                  key={m.remoteId}
                  model={m}
                  checked={selected.has(m.remoteId)}
                  onToggle={() => toggle(m.remoteId)}
                  onEdit={() => onEdit(m.entry, m.exists ? m.entry.id : undefined)}
                />
              ))}
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}

function Row({
  model,
  checked,
  onToggle,
  onEdit,
}: {
  model: DiscoveredModel;
  checked: boolean;
  onToggle: () => void;
  onEdit: () => void;
}) {
  const e = model.entry;
  const metaBadge = {
    catalog: { text: "目录补全", cls: "bg-success-soft text-success" },
    api: { text: "接口提供", cls: "bg-accent-soft text-accent" },
    none: { text: "无参数", cls: "bg-attention-soft text-attention" },
  }[model.source as "catalog" | "api" | "none"] ?? {
    text: model.source,
    cls: "bg-subtle text-ink-muted",
  };

  return (
    <div
      className="flex items-start gap-3 px-3 py-2.5 transition-colors duration-[var(--duration-fast)] hover:bg-hover"
      style={{ borderTop: "var(--p-hairline) solid var(--color-line)" }}
    >
      <input
        type="checkbox"
        checked={checked}
        onChange={onToggle}
        className="mt-1 h-3.5 w-3.5 shrink-0 cursor-pointer accent-accent"
      />
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="text-xs font-medium text-ink">
            {model.displayName}
          </span>
          <span className="font-mono text-[10px] text-ink-quaternary">
            {model.remoteId}
          </span>
          <span className={`rounded px-1.5 py-[1px] text-[10px] font-medium ${metaBadge.cls}`}>
            {metaBadge.text}
          </span>
          {model.exists && (
            <span className="rounded bg-subtle px-1.5 py-[1px] text-[10px] font-medium text-ink-muted">
              已存在
            </span>
          )}
        </div>
        <div className="mt-1 flex flex-wrap items-center gap-1.5">
          {e.capabilities.includes("image_in") && (
            <MiniTag cls="bg-accent-soft text-accent">识图</MiniTag>
          )}
          {e.capabilities.includes("video_in") && (
            <MiniTag cls="bg-accent-soft text-accent">视频</MiniTag>
          )}
          {e.capabilities.includes("thinking") && (
            <MiniTag cls="bg-attention-soft text-attention">思考</MiniTag>
          )}
          {e.capabilities.includes("tool_use") && (
            <MiniTag cls="bg-subtle text-ink-muted">工具</MiniTag>
          )}
          {e.maxContextSize && (
            <MiniTag cls="bg-subtle text-ink-muted">
              {fmtCtx(e.maxContextSize)} 上下文
            </MiniTag>
          )}
          {e.supportEfforts.length > 0 && (
            <MiniTag cls="bg-subtle text-ink-muted">
              档位 {e.supportEfforts.join("/")}
            </MiniTag>
          )}
          {!e.maxContextSize && e.capabilities.length === 0 && (
            <span className="text-[10px] text-attention">
              接口和目录都没给参数，导入后请手动补全
            </span>
          )}
        </div>
      </div>
      <Button size="sm" variant="ghost" onClick={onEdit}>
        编辑
      </Button>
    </div>
  );
}

function MiniTag({ children, cls }: { children: React.ReactNode; cls: string }) {
  return (
    <span className={`rounded px-1.5 py-[1px] text-[10px] font-medium ${cls}`}>
      {children}
    </span>
  );
}

function fmtCtx(n: number): string {
  if (n >= 1048576) return `${(n / 1048576).toFixed(n % 1048576 === 0 ? 0 : 1)}M`;
  if (n >= 1024) return `${Math.round(n / 1024)}K`;
  return String(n);
}

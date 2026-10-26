import { createContext, useContext, useEffect, useState } from "react";
import { api } from "@/lib/ipc";
import type {
  AppPaths,
  McpServer,
  ModelEntry,
  Plugin,
  Provider,
  ProvidersSnapshot,
  Skill,
} from "@/lib/types";
import { Button, Empty, ErrorBar, Modal, Spinner, Tag } from "@/components/ui";
import { ModelEditor, emptyModel } from "@/components/ModelEditor";
import { ProviderEditor } from "@/components/ProviderEditor";
import { McpEditor } from "@/components/McpEditor";
import { DiscoverDialog } from "@/components/DiscoverDialog";
import type { MetaInfo } from "@/lib/types";

type Page = "providers" | "mcp" | "skills" | "plugins" | "settings";

const NAV: {
  key: Page;
  label: string;
  icon: React.ReactNode;
}[] = [
  {
    key: "providers",
    label: "API 与模型",
    icon: (
      <path d="M3 6.5h10M3 10h10M6.5 3v10M9.5 3v10" />
    ),
  },
  {
    key: "mcp",
    label: "MCP",
    icon: (
      <>
        <rect x="2.5" y="4" width="11" height="8" rx="1.5" />
        <path d="M5 7.2h6M5 9.4h3.5" />
      </>
    ),
  },
  {
    key: "skills",
    label: "Skills",
    icon: (
      <path d="M8 2.5 9.6 6l3.9.4-2.9 2.6.9 3.8L8 10.9 4.5 12.8l.9-3.8L2.5 6.4 6.4 6z" />
    ),
  },
  {
    key: "plugins",
    label: "Plugins",
    icon: (
      <>
        <path d="M6 3.5h4v2a1.3 1.3 0 0 0 2.5 0v-2h.5a1 1 0 0 1 1 1V9h-2a1.3 1.3 0 0 0 0 2.5h2v.5a1 1 0 0 1-1 1H10v-2a1.3 1.3 0 0 0-2.5 0v2H4a1 1 0 0 1-1-1V9h2a1.3 1.3 0 0 0 0-2.5H3V4.5a1 1 0 0 1 1-1h2z" />
      </>
    ),
  },
  {
    key: "settings",
    label: "设置",
    icon: (
      <>
        <circle cx="8" cy="8" r="2.1" />
        <path d="M6.9 2.4h2.2l.36 1.6 1.28.74 1.5-.65 1.1 1.9-1.16 1.15v1.48l1.16 1.15-1.1 1.9-1.5-.65-1.28.74-.36 1.6H6.9l-.36-1.6-1.28-.74-1.5.65-1.1-1.9L3.82 8.62V7.14L2.66 5.99l1.1-1.9 1.5.65 1.28-.74z" />
      </>
    ),
  },
];

/* ── 主题与侧栏折叠状态的持久化 ── */

type Theme = "light" | "dark";

/** 主题状态跨页面共享：设置页负责切换，App 负责应用 */
const ThemeCtx = createContext<{
  theme: Theme;
  toggle: () => void;
  collapsed: boolean;
  toggleCollapsed: () => void;
}>({
  theme: "light",
  toggle: () => {},
  collapsed: false,
  toggleCollapsed: () => {},
});

const STORE_KEY = "kc-switch.ui";

async function setWindowTheme(theme: Theme) {
  try {
    // 走自有命令，避免依赖 window 插件权限；非 Tauri 环境会抛错后被忽略
    await api.setWindowTheme(theme);
  } catch (e) {
    console.warn("[kc-switch] 设置标题栏主题失败:", e);
  }
}

function readUiState(): { theme: Theme; collapsed: boolean } {
  try {
    const raw = localStorage.getItem(STORE_KEY);
    if (raw) {
      const v = JSON.parse(raw) as Partial<{ theme: Theme; collapsed: boolean }>;
      return {
        theme: v.theme === "dark" ? "dark" : "light",
        collapsed: Boolean(v.collapsed),
      };
    }
  } catch {
    /* 忽略损坏的本地状态 */
  }
  const prefersDark =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-color-scheme: dark)").matches;
  return { theme: prefersDark ? "dark" : "light", collapsed: false };
}

export default function App() {
  const [page, setPage] = useState<Page>("providers");
  const [ui, setUi] = useState(readUiState);

  // 主题写进 <html data-theme>，深色令牌在 styles.css 里生效
  useEffect(() => {
    document.documentElement.dataset.theme = ui.theme;
    // 让原生标题栏跟随主题，否则浅色内容配深色标题栏会割裂
    void setWindowTheme(ui.theme);
  }, [ui.theme]);

  useEffect(() => {
    try {
      localStorage.setItem(STORE_KEY, JSON.stringify(ui));
    } catch {
      /* 存储不可用时不影响使用 */
    }
  }, [ui]);

  const toggleTheme = () =>
    setUi((v) => ({ ...v, theme: v.theme === "dark" ? "light" : "dark" }));
  const toggleCollapsed = () =>
    setUi((v) => ({ ...v, collapsed: !v.collapsed }));

  const collapsed = ui.collapsed;
  const sidebarW = collapsed ? "56px" : "var(--p-sidebar-w)";

  return (
    <ThemeCtx.Provider
      value={{ theme: ui.theme, toggle: toggleTheme, collapsed, toggleCollapsed }}
    >
    <div className="flex h-full min-w-0 flex-col overflow-hidden bg-bg">
      {/* 原生标题栏正下方的贯穿分隔线 */}
      <div className="h-px w-full shrink-0 bg-line" />
      <div className="flex min-h-0 min-w-0 flex-1 overflow-hidden">
        <aside
          className="flex shrink-0 flex-col overflow-hidden border-r border-line bg-sidebar-bg transition-[width] duration-[var(--duration-base)] ease-[var(--ease-out)]"
          style={{ width: sidebarW, minWidth: sidebarW }}
        >
          {/* 品牌区（折叠按钮已移到主区标题栏，这里只留品牌标识） */}
          <div
            className={`flex shrink-0 items-center gap-2.5 ${
              collapsed ? "justify-center px-2" : "px-4"
            }`}
            style={{ height: 48 }}
          >
            <img
              src="/icon.png"
              alt="KC-Switch"
              className="h-6 w-6 shrink-0 rounded-[5px]"
              onError={(e) => {
                (e.currentTarget as HTMLImageElement).style.display = "none";
              }}
            />
            {!collapsed && (
              <div className="min-w-0 leading-tight">
                <div className="truncate text-[13px] font-semibold tracking-tight text-ink">
                  KC-Switch
                </div>
                <div className="truncate text-[10px] text-ink-faint">
                  Kimi Code 配置管理
                </div>
              </div>
            )}
          </div>

          <nav
            className={`flex flex-col gap-0.5 ${collapsed ? "px-2" : "px-2"} pt-1`}
          >
            {NAV.map((item) => {
              const active = page === item.key;
              return (
                <button
                  key={item.key}
                  onClick={() => setPage(item.key)}
                  title={collapsed ? item.label : undefined}
                  className={`flex h-8 items-center rounded-md text-[13px] transition-colors duration-[var(--duration-fast)] ${
                    collapsed ? "justify-center px-0" : "gap-2.5 px-2.5"
                  } ${
                    active
                      ? "bg-selected font-medium text-ink"
                      : "text-ink-muted hover:bg-hover hover:text-ink"
                  }`}
                >
                  <svg
                    viewBox="0 0 16 16"
                    className={`h-4 w-4 shrink-0 ${
                      active ? "text-ink" : "text-ink-faint"
                    }`}
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.4"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    {item.icon}
                  </svg>
                  {!collapsed && <span className="truncate">{item.label}</span>}
                </button>
              );
            })}
          </nav>

        </aside>

        <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-bg">
          {page === "providers" && <ProvidersPage />}
          {page === "mcp" && <McpPage />}
          {page === "skills" && <SkillsPage />}
          {page === "plugins" && <PluginsPage />}
          {page === "settings" && <SettingsPage />}
        </main>
      </div>
    </div>
    </ThemeCtx.Provider>
  );
}

function PageHeader({
  title,
  subtitle,
  action,
}: {
  title: string;
  subtitle?: string;
  action?: React.ReactNode;
}) {
  const { collapsed, toggleCollapsed } = useContext(ThemeCtx);
  return (
    <header
      className="flex shrink-0 items-center justify-between gap-4 border-b border-line pr-7"
      style={{ height: "var(--p-panel-head-h)" }}
    >
      <div className="flex min-w-0 items-center gap-2.5">
        {/* 侧栏折叠按钮固定在标题栏最左，位置不随侧栏宽度变化 */}
        <button
          onClick={toggleCollapsed}
          title={collapsed ? "展开侧栏" : "收起侧栏"}
          className="ml-3 flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-ink-faint transition-colors duration-[var(--duration-fast)] hover:bg-hover hover:text-ink"
        >
          <svg
            viewBox="0 0 16 16"
            className="h-4 w-4"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <rect x="2.5" y="3" width="11" height="10" rx="1.8" />
            <path d="M6.5 3v10" />
          </svg>
        </button>
        <h1 className="shrink-0 text-[15px] font-semibold tracking-tight text-ink">
          {title}
        </h1>
        {subtitle && (
          <p className="truncate text-xs text-ink-faint">{subtitle}</p>
        )}
      </div>
      {action && <div className="shrink-0">{action}</div>}
    </header>
  );
}

// ─────────────────────────── API / Providers ───────────────────────────

function ProvidersPage() {
  const [data, setData] = useState<ProvidersSnapshot | null>(null);
  const [meta, setMeta] = useState<MetaInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [creatingProvider, setCreatingProvider] = useState(false);
  const [modelEdit, setModelEdit] = useState<
    { initial: ModelEntry; originalId?: string } | null
  >(null);
  const [confirmDelete, setConfirmDelete] = useState<
    { kind: "provider"; name: string } | { kind: "model"; id: string } | null
  >(null);
  const [discovering, setDiscovering] = useState<string | null>(null);

  const load = () => {
    api.providers
      .list()
      .then((d) => {
        setData(d);
        setError(null);
      })
      .catch((e) => setError(e.message));
    api.providers.meta().then(setMeta).catch(() => {});
  };

  useEffect(load, []);

  const doDelete = async () => {
    if (!confirmDelete) return;
    try {
      if (confirmDelete.kind === "provider") {
        await api.providers.remove(confirmDelete.name, true);
      } else {
        await api.providers.removeModel(confirmDelete.id);
      }
      setConfirmDelete(null);
      load();
    } catch (e) {
      setError((e as Error).message);
    }
  };

  const providerNames = data?.providers.map((p) => p.name) ?? [];

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader
        title="API 与模型"
        subtitle="添加官方未收录模型时，务必勾选「图片输入」与思考档位"
        action={
          <Button
            variant="primary"
            onClick={() => setCreatingProvider(true)}
            icon={
              <svg
                viewBox="0 0 16 16"
                className="h-3.5 w-3.5"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.8"
                strokeLinecap="round"
              >
                <path d="M8 3.5v9M3.5 8h9" />
              </svg>
            }
          >
            添加 Provider
          </Button>
        }
      />
      {error && <ErrorBar message={error} />}

      {!data ? (
        <Spinner label="读取中…" />
      ) : (
        <div className="min-w-0 flex-1 overflow-x-hidden overflow-y-auto px-7 py-4">
          <div className="flex w-full min-w-0 flex-col gap-3">
            {data.providers.map((p) => (
              <section
                key={p.name}
                className="w-full max-w-full min-w-0 overflow-hidden rounded-lg border border-line bg-surface-raised shadow-subtle"
              >
                <div className="flex min-w-0 flex-wrap items-center justify-between gap-x-3 gap-y-1.5 px-4 py-2.5">
                  <div className="flex min-w-0 flex-1 flex-wrap items-center gap-x-2 gap-y-1">
                    <span className="shrink-0 text-[13px] font-medium text-ink">
                      {p.name}
                    </span>
                    {p.managed && <Tag tone="attention">托管只读</Tag>}
                    {p.providerType && <Tag>{p.providerType}</Tag>}
                    <span
                      className="max-w-[220px] min-w-0 truncate font-mono text-[10px] text-ink-quaternary"
                      title={p.baseUrl}
                    >
                      {p.baseUrl ?? "—"}
                    </span>
                  </div>
                  <div className="flex shrink-0 items-center gap-1.5">
                    {!p.managed && (
                      <>
                        <Button
                          size="sm"
                          variant="secondary"
                          onClick={() => setDiscovering(p.name)}
                          disabled={!p.hasApiKey || !p.baseUrl}
                        >
                          获取模型列表
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => setEditingProvider(p)}
                        >
                          编辑
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() =>
                            setConfirmDelete({ kind: "provider", name: p.name })
                          }
                        >
                          删除
                        </Button>
                      </>
                    )}
                  </div>
                </div>

                <div className="flex flex-col border-t border-line">
                  {p.models.length === 0 && (
                    <div className="px-4 py-3 text-xs text-ink-faint">
                      还没有模型条目 —— 点上方「获取模型列表」可自动拉取并补全参数
                    </div>
                  )}
                  {p.models.map((m) => {
                    const active = data.defaultModel === m.id;
                    return (
                      <div
                        key={m.id}
                        className="group flex h-9 min-w-0 items-center justify-between gap-3 px-4 transition-colors duration-[var(--duration-fast)] hover:bg-hover"
                      >
                        <div className="flex min-w-0 flex-1 items-center gap-2 overflow-hidden">
                          <span
                            className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                              active ? "bg-success" : "bg-ink-quaternary/40"
                            }`}
                            title={active ? "当前使用中" : undefined}
                          />
                          <span className="truncate text-xs text-ink">
                            {m.displayName || m.id}
                          </span>
                          <span className="shrink-0 font-mono text-[10px] text-ink-quaternary">
                            {m.model}
                          </span>
                          {m.capabilities.includes("image_in") && (
                            <Tag tone="accent" title="支持图片输入">
                              识图
                            </Tag>
                          )}
                          {m.capabilities.includes("thinking") && (
                            <Tag
                              tone="attention"
                              title={`思考档位：${m.supportEfforts.join(", ") || "未设置"}`}
                            >
                              思考
                            </Tag>
                          )}
                          {m.maxContextSize && (
                            <span className="shrink-0 font-mono text-[10px] text-ink-quaternary">
                              {fmtContext(m.maxContextSize)}
                            </span>
                          )}
                        </div>
                        <div className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() =>
                              setModelEdit({ initial: m, originalId: m.id })
                            }
                          >
                            编辑
                          </Button>
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() =>
                              setConfirmDelete({ kind: "model", id: m.id })
                            }
                          >
                            删除
                          </Button>
                        </div>
                      </div>
                    );
                  })}
                </div>

                {!p.managed && (
                  <button
                    onClick={() => setModelEdit({ initial: emptyModel(p.name) })}
                    className="flex h-8 w-full items-center gap-1.5 px-4 text-left text-xs text-ink-faint transition-colors duration-[var(--duration-fast)] hover:bg-hover hover:text-accent"

                  >
                    <svg
                      viewBox="0 0 16 16"
                      className="h-3 w-3"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="1.8"
                      strokeLinecap="round"
                    >
                      <path d="M8 3.5v9M3.5 8h9" />
                    </svg>
                    在此 provider 下添加模型
                  </button>
                )}
              </section>
            ))}
          </div>
        </div>
      )}

      {(creatingProvider || editingProvider) && (
        <ProviderEditor
          initial={editingProvider ?? undefined}
          onClose={() => {
            setCreatingProvider(false);
            setEditingProvider(null);
          }}
          onSaved={load}
        />
      )}

      {modelEdit && (
        <ModelEditor
          initial={modelEdit.initial}
          originalId={modelEdit.originalId}
          providers={providerNames}
          meta={meta}
          onClose={() => setModelEdit(null)}
          onSaved={load}
        />
      )}

      {discovering && (
        <DiscoverDialog
          providerName={discovering}
          onClose={() => setDiscovering(null)}
          onImported={load}
          onEdit={(entry, existingId) => {
            setDiscovering(null);
            setModelEdit({ initial: entry, originalId: existingId });
          }}
        />
      )}

      {confirmDelete && (
        <Modal
          title="确认删除"
          onClose={() => setConfirmDelete(null)}
          footer={
            <>
              <div className="flex-1" />
              <Button variant="ghost" onClick={() => setConfirmDelete(null)}>
                取消
              </Button>
              <Button variant="danger" onClick={doDelete}>
                删除
              </Button>
            </>
          }
        >
          <p className="text-[13px] leading-relaxed text-ink-muted">
            {confirmDelete.kind === "provider"
              ? `将删除 provider「${confirmDelete.name}」及其下所有模型条目。删除前会自动备份 config.toml。`
              : `将删除模型条目「${confirmDelete.id}」。删除前会自动备份 config.toml。`}
          </p>
        </Modal>
      )}
    </div>
  );
}

function fmtContext(n: number): string {
  if (n >= 1048576) return `${(n / 1048576).toFixed(n % 1048576 === 0 ? 0 : 1)}M`;
  if (n >= 1024) return `${Math.round(n / 1024)}K`;
  return String(n);
}

// ─────────────────────────── MCP ───────────────────────────

function McpPage() {
  const [items, setItems] = useState<McpServer[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState<McpServer | "new" | null>(null);
  const [confirm, setConfirm] = useState<string | null>(null);

  const load = () => {
    api.mcp
      .list()
      .then((d) => {
        setItems(d);
        setError(null);
      })
      .catch((e) => setError(e.message));
  };
  useEffect(load, []);

  const toggle = async (name: string, deferred: boolean) => {
    try {
      await api.mcp.toggleDeferred(name, deferred);
      load();
    } catch (e) {
      setError((e as Error).message);
    }
  };

  const remove = async (name: string) => {
    try {
      await api.mcp.remove(name);
      setConfirm(null);
      load();
    } catch (e) {
      setError((e as Error).message);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader
        title="MCP"
        subtitle="延迟加载让不常用的服务器在启动时不拉起"
        action={
          <Button variant="primary" onClick={() => setEditing("new")}>
            添加 MCP
          </Button>
        }
      />
      {error && <ErrorBar message={error} />}

      {!items ? (
        <Spinner label="读取中…" />
      ) : items.length === 0 ? (
        <Empty text="还没有 MCP 服务器" />
      ) : (
        <div className="min-w-0 flex-1 overflow-x-hidden overflow-y-auto px-7 py-4">
          <div className="w-full min-w-0 overflow-hidden rounded-lg border border-line bg-surface-raised shadow-subtle">
            <table className="w-full text-left">
              <thead>
                <tr className="border-b border-line bg-surface-sunken text-[10px] uppercase tracking-wider text-ink-faint">
                  <th className="px-4 py-2 font-medium">名称</th>
                  <th className="px-4 py-2 font-medium">类型</th>
                  <th className="px-4 py-2 font-medium">命令 / URL</th>
                  <th className="px-4 py-2 font-medium">延迟加载</th>
                  <th className="w-28 px-4 py-2" />
                </tr>
              </thead>
              <tbody>
                {items.map((s) => (
                  <tr
                    key={s.name}
                    className="group transition-colors duration-[var(--duration-fast)] hover:bg-hover"

                  >
                    <td className="px-4 py-2 text-xs text-ink">{s.name}</td>
                    <td className="px-4 py-2">
                      <Tag>{s.transport}</Tag>
                    </td>
                    <td className="max-w-96 truncate px-4 py-2 font-mono text-[10px] text-ink-faint">
                      {s.transport === "http"
                        ? s.url
                        : `${s.command ?? ""} ${s.args.join(" ")}`.trim()}
                    </td>
                    <td className="px-4 py-2">
                      <input
                        type="checkbox"
                        checked={s.deferred}
                        onChange={(e) => toggle(s.name, e.target.checked)}
                        className="h-3.5 w-3.5 cursor-pointer accent-accent"
                      />
                    </td>
                    <td className="px-4 py-2">
                      <div className="flex justify-end gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => setEditing(s)}
                        >
                          编辑
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => setConfirm(s.name)}
                        >
                          删除
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {editing && (
        <McpEditor
          initial={editing === "new" ? undefined : editing}
          onClose={() => setEditing(null)}
          onSaved={load}
        />
      )}

      {confirm && (
        <Modal
          title="删除 MCP"
          onClose={() => setConfirm(null)}
          footer={
            <>
              <div className="flex-1" />
              <Button variant="ghost" onClick={() => setConfirm(null)}>
                取消
              </Button>
              <Button variant="danger" onClick={() => remove(confirm)}>
                删除
              </Button>
            </>
          }
        >
          <p className="text-[13px] text-ink-muted">
            将从 mcp.json 移除「{confirm}」，删除前会自动备份。
          </p>
        </Modal>
      )}
    </div>
  );
}

// ─────────────────────────── Skills ───────────────────────────

function SkillsPage() {
  const [items, setItems] = useState<Skill[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<"all" | "user" | "plugin">("user");

  const load = () => {
    api.skills
      .list()
      .then((d) => {
        setItems(d);
        setError(null);
      })
      .catch((e) => setError(e.message));
  };
  useEffect(load, []);

  const toggle = async (path: string, enable: boolean) => {
    try {
      await api.skills.toggle(path, enable);
      load();
    } catch (e) {
      setError((e as Error).message);
    }
  };

  const shown = (items ?? []).filter((s) => {
    if (filter === "user" && s.source !== "user") return false;
    if (filter === "plugin" && s.source !== "plugin") return false;
    if (!query.trim()) return true;
    const q = query.toLowerCase();
    return (
      s.name.toLowerCase().includes(q) || s.description.toLowerCase().includes(q)
    );
  });

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader
        title="Skills"
        subtitle="禁用会把目录移到 skills-disabled，随时可恢复"
        action={
          <div className="relative">
            <svg
              viewBox="0 0 16 16"
              className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-ink-quaternary"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.6"
              strokeLinecap="round"
            >
              <circle cx="7" cy="7" r="4.2" />
              <path d="m10.2 10.2 2.8 2.8" />
            </svg>
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="搜索…"
              className="h-8 w-52 rounded-md border border-line bg-surface-sunken pl-8 pr-3 text-xs text-ink outline-none transition-[border-color,box-shadow] placeholder:text-ink-quaternary hover:border-line-strong focus:border-accent focus:bg-surface-raised focus:shadow-[0_0_0_3px_var(--color-accent-soft)]"
            />
          </div>
        }
      />
      {error && <ErrorBar message={error} />}

      <div className="flex shrink-0 items-center gap-1 px-7 pb-1 pt-3">
        {(
          [
            ["user", "用户级"],
            ["plugin", "插件内置"],
            ["all", "全部"],
          ] as const
        ).map(([key, label]) => (
          <button
            key={key}
            onClick={() => setFilter(key)}
            className={`h-6 rounded-md px-2 text-[11px] transition-colors duration-[var(--duration-fast)] ${
              filter === key
                ? "bg-selected font-medium text-ink"
                : "text-ink-faint hover:bg-hover hover:text-ink"
            }`}
          >
            {label}
          </button>
        ))}
        {items && (
          <span className="ml-auto font-mono text-[10px] text-ink-quaternary">
            {shown.length} / {items.length}
          </span>
        )}
      </div>

      {!items ? (
        <Spinner label="扫描中…" />
      ) : (
        <div className="min-w-0 flex-1 overflow-x-hidden overflow-y-auto px-7 py-3">
          <div className="grid grid-cols-1 gap-2 xl:grid-cols-2">
            {shown.map((s) => (
              <div
                key={s.path}
                className={`group rounded-lg border px-3 py-2.5 transition-[background-color,border-color] duration-[var(--duration-fast)] ${
                  s.enabled
                    ? "border-line bg-surface-raised hover:bg-surface"
                    : "border-transparent bg-surface-sunken"
                }`}
                data-enabled={s.enabled}
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-1.5">
                      <span
                        className={`truncate text-xs font-medium ${
                          s.enabled ? "text-ink" : "text-ink-faint"
                        }`}
                      >
                        {s.name}
                      </span>
                      <Tag>{s.source === "plugin" ? s.plugin : "用户级"}</Tag>
                    </div>
                    <p
                      className={`mt-1 line-clamp-2 text-[11px] leading-relaxed ${
                        s.enabled ? "text-ink-muted" : "text-ink-quaternary"
                      }`}
                    >
                      {s.description || "无描述"}
                    </p>
                  </div>
                  {s.source !== "plugin" && (
                    <button
                      onClick={() => toggle(s.path, !s.enabled)}
                      className={`mt-0.5 h-6 shrink-0 rounded-md px-2 text-[11px] transition-colors duration-[var(--duration-fast)] ${
                        s.enabled
                          ? "text-ink-faint opacity-0 hover:bg-danger-soft hover:text-danger group-hover:opacity-100"
                          : "bg-accent-soft text-accent hover:bg-accent hover:text-white"
                      }`}
                    >
                      {s.enabled ? "禁用" : "启用"}
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
          {shown.length === 0 && <Empty text="没有匹配的 skill" />}
        </div>
      )}
    </div>
  );
}

// ─────────────────────────── Plugins ───────────────────────────

function PluginsPage() {
  const [items, setItems] = useState<Plugin[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<Plugin | null>(null);

  const load = () => {
    api.plugins
      .list()
      .then((d) => {
        setItems(d);
        setError(null);
      })
      .catch((e) => setError(e.message));
  };
  useEffect(load, []);

  const toggle = async (id: string, enabled: boolean) => {
    try {
      await api.plugins.toggle(id, enabled);
      load();
    } catch (e) {
      setError((e as Error).message);
    }
  };

  const remove = async (p: Plugin) => {
    try {
      await api.plugins.remove(p.id, true);
      setConfirm(null);
      load();
    } catch (e) {
      setError((e as Error).message);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader title="Plugins" subtitle="禁用后其自带的 skills 也会一并失效" />
      {error && <ErrorBar message={error} />}

      {!items ? (
        <Spinner label="读取中…" />
      ) : (
        <div className="min-w-0 flex-1 overflow-x-hidden overflow-y-auto px-7 py-4">
          <div className="flex flex-col gap-2">
            {items.map((p) => (
              <div
                key={p.id}
                className="group flex min-w-0 items-center justify-between gap-3 rounded-lg border border-line bg-surface-raised px-4 py-2.5 transition-colors duration-[var(--duration-fast)] hover:bg-surface"
              >
                <div className="min-w-0">
                  <div className="flex items-center gap-1.5">
                    <span className="text-xs font-medium text-ink">{p.id}</span>
                    <Tag>{p.source}</Tag>
                    {p.refLabel && (
                      <span className="font-mono text-[10px] text-ink-quaternary">
                        {p.refLabel}
                      </span>
                    )}
                  </div>
                  <div className="mt-0.5 truncate font-mono text-[10px] text-ink-quaternary">
                    {p.originalSource ?? p.root}
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-1.5">
                  <button
                    onClick={() => toggle(p.id, !p.enabled)}
                    className={`h-6 rounded-md px-2 text-[11px] font-medium transition-colors duration-[var(--duration-fast)] ${
                      p.enabled
                        ? "bg-success-soft text-success hover:bg-success/15"
                        : "bg-subtle text-ink-faint hover:bg-selected hover:text-ink"
                    }`}
                  >
                    {p.enabled ? "已启用" : "已禁用"}
                  </button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => setConfirm(p)}
                  >
                    删除
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {confirm && (
        <Modal
          title="删除插件"
          subtitle={confirm.id}
          onClose={() => setConfirm(null)}
          footer={
            <>
              <div className="flex-1" />
              <Button variant="ghost" onClick={() => setConfirm(null)}>
                取消
              </Button>
              <Button variant="danger" onClick={() => remove(confirm)}>
                删除
              </Button>
            </>
          }
        >
          <p className="text-[13px] leading-relaxed text-ink-muted">
            将把插件目录移到回收站，并从 installed.json 移除注册。
          </p>
          <p className="mt-2 break-all rounded-md bg-surface-sunken px-2.5 py-2 font-mono text-[10px] text-ink-faint">
            {confirm.root}
          </p>
        </Modal>
      )}
    </div>
  );
}

// ─────────────────────────── Settings ───────────────────────────

function SettingsPage() {
  const { theme, toggle } = useContext(ThemeCtx);
  const [paths, setPaths] = useState<AppPaths | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .paths()
      .then(setPaths)
      .catch((e) => setError(e.message));
  }, []);

  const rows: [string, string | undefined][] = paths
    ? [
        ["Kimi Code 目录", paths.kimiHome],
        ["provider 配置", paths.configFile],
        ["MCP 配置", paths.mcpFile],
        ["插件注册表", paths.pluginsFile],
        ["用户 Skills", paths.userSkillsDir],
        ["已禁用 Skills", paths.disabledSkillsDir],
        ["KC-Switch 数据", paths.dataDir],
      ]
    : [];

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader title="设置" subtitle="路径与运行信息" />
      {error && <ErrorBar message={error} />}
      {!paths ? (
        <Spinner label="读取中…" />
      ) : (
        <div className="min-w-0 flex-1 overflow-x-hidden overflow-y-auto px-7 py-4">
          <div className="mb-2 text-xs font-medium text-ink">外观</div>
          <div className="mb-5 w-full min-w-0 overflow-hidden rounded-lg border border-line bg-surface-raised shadow-subtle">
            <div className="flex min-w-0 items-center justify-between gap-4 px-4 py-3">
              <div className="min-w-0">
                <div className="text-xs text-ink">主题</div>
                <div className="mt-0.5 text-[11px] text-ink-faint">
                  {theme === "dark" ? "当前为深色" : "当前为浅色"}
                </div>
              </div>
              <button
                onClick={toggle}
                className="flex h-8 shrink-0 items-center gap-2 rounded-md border border-line bg-surface-raised px-3 text-[12px] text-ink transition-colors duration-[var(--duration-fast)] hover:bg-hover"
              >
                <svg
                  viewBox="0 0 16 16"
                  className="h-3.5 w-3.5 shrink-0 text-ink-muted"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.4"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  {theme === "dark" ? (
                    <path d="M13.2 9.6A5.6 5.6 0 0 1 6.4 2.8a5.6 5.6 0 1 0 6.8 6.8z" />
                  ) : (
                    <>
                      <circle cx="8" cy="8" r="3" />
                      <path d="M8 1.8v1.4M8 12.8v1.4M1.8 8h1.4M12.8 8h1.4M3.6 3.6l1 1M11.4 11.4l1 1M12.4 3.6l-1 1M4.6 11.4l-1 1" />
                    </>
                  )}
                </svg>
                {theme === "dark" ? "切换到浅色" : "切换到深色"}
              </button>
            </div>
          </div>

          <div className="mb-2 text-xs font-medium text-ink">路径</div>
          <div className="w-full min-w-0 overflow-hidden rounded-lg border border-line bg-surface-raised shadow-subtle">
            {rows.map(([label, value], i) => (
              <div
                key={label}
                className={`flex min-w-0 items-start justify-between gap-4 px-4 py-2.5 ${
                  i > 0 ? "border-t border-line" : ""
                }`}
              >
                <span className="shrink-0 text-xs text-ink-muted">{label}</span>
                <span className="break-all text-right font-mono text-[10px] leading-relaxed text-ink-faint">
                  {value}
                </span>
              </div>
            ))}
          </div>
          <p className="mt-3 text-[11px] leading-relaxed text-ink-faint">
            所有写操作先备份到 KC-Switch 数据目录的 backups 下再原子替换；
            删除的 skill / 插件移到 trash，可手动恢复。
          </p>
        </div>
      )}
    </div>
  );
}

import { invoke } from "@tauri-apps/api/core";
import type {
  AppPaths,
  DiscoverResult,
  McpUpsert,
  McpServer,
  MetaInfo,
  ModelEntry,
  ModelIssue,
  Plugin,
  Provider,
  ProvidersSnapshot,
  Skill,
} from "./types";

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw new Error(typeof e === "string" ? e : String(e));
  }
}

export const api = {
  paths: () => call<AppPaths>("get_paths"),

  setWindowTheme: (theme: "light" | "dark") =>
    call<void>("set_window_theme", { theme }),

  providers: {
    list: () => call<ProvidersSnapshot>("providers_list"),
    setDefaultModel: (model: string) =>
      call<void>("providers_set_default_model", { model }),
    upsert: (input: {
      name: string;
      originalName?: string | null;
      baseUrl: string;
      apiKey?: string | null;
      providerType?: string | null;
    }) => call<void>("providers_upsert", { input }),
    remove: (name: string, cascadeModels: boolean) =>
      call<void>("providers_remove", { name, cascadeModels }),
    upsertModel: (entry: ModelEntry, originalId?: string | null) =>
      call<void>("providers_upsert_model", { entry, originalId: originalId ?? null }),
    removeModel: (id: string) => call<void>("providers_remove_model", { id }),
    diagnose: (entry: ModelEntry) =>
      call<ModelIssue[]>("providers_diagnose", { entry }),
    meta: () => call<MetaInfo>("providers_meta"),
    discoverModels: (providerName: string, refreshCatalog = false) =>
      call<DiscoverResult>("providers_discover_models", {
        providerName,
        refreshCatalog,
      }),
    importModels: (entries: ModelEntry[]) =>
      call<number>("providers_import_models", { entries }),
  },

  mcp: {
    list: () => call<McpServer[]>("mcp_list"),
    upsert: (input: McpUpsert) => call<void>("mcp_upsert", { input }),
    remove: (name: string) => call<void>("mcp_remove", { name }),
    toggleDeferred: (name: string, deferred: boolean) =>
      call<void>("mcp_set_deferred", { name, deferred }),
  },

  skills: {
    list: (projectRoot?: string) =>
      call<Skill[]>("skills_list", { projectRoot: projectRoot ?? null }),
    toggle: (path: string, enable: boolean) =>
      call<void>("skills_toggle", { path, enable }),
    remove: (path: string) => call<void>("skills_delete", { path }),
  },

  plugins: {
    list: () => call<Plugin[]>("plugins_list"),
    toggle: (id: string, enabled: boolean) =>
      call<void>("plugins_toggle", { id, enabled }),
    remove: (id: string, deleteFiles: boolean) =>
      call<void>("plugins_remove", { id, deleteFiles }),
  },
};

export type { Provider };

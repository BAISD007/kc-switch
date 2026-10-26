export type ModelEntry = {
  id: string;
  provider: string;
  model: string;
  displayName?: string | null;
  maxContextSize?: number | null;
  maxOutputSize?: number | null;
  capabilities: string[];
  supportEfforts: string[];
  defaultEffort?: string | null;
  adaptiveThinking?: boolean | null;
  reasoningKey?: string | null;
};

export type ModelIssue = { level: "error" | "warn" | "info"; message: string };

export type DiscoveredModel = {
  remoteId: string;
  displayName: string;
  entry: ModelEntry;
  exists: boolean;
  metadataOk: boolean;
  source: "catalog" | "api" | "none" | string;
};

export type DiscoverResult = {
  models: DiscoveredModel[];
  notes: string[];
};

export type MetaInfo = {
  knownCapabilities: string[];
  knownEfforts: string[];
};

export type Provider = {
  name: string;
  baseUrl?: string;
  apiKeyMasked?: string;
  providerType?: string;
  managed: boolean;
  models: ModelEntry[];
  hasApiKey: boolean;
};

export type ProvidersSnapshot = {
  providers: Provider[];
  defaultModel?: string;
  configPath: string;
};

export type McpServer = {
  name: string;
  transport: "stdio" | "http";
  command?: string;
  args: string[];
  url?: string;
  bearerTokenEnvVar?: string;
  envKeys: string[];
  deferred: boolean;
};

export type McpUpsert = {
  name: string;
  transport: "stdio" | "http";
  command?: string | null;
  args: string[];
  url?: string | null;
  bearerTokenEnvVar?: string | null;
  env: [string, string][];
  deferred: boolean;
};

export type Skill = {
  name: string;
  description: string;
  source: "user" | "project" | "plugin";
  enabled: boolean;
  path: string;
  plugin?: string;
  hasSkillMd: boolean;
};

export type Plugin = {
  id: string;
  root: string;
  source: string;
  enabled: boolean;
  originalSource?: string;
  installedAt?: string;
  updatedAt?: string;
  refLabel?: string;
  hasUpdate?: boolean;
};

export type AppPaths = {
  kimiHome: string;
  configFile: string;
  mcpFile: string;
  pluginsFile: string;
  userSkillsDir: string;
  disabledSkillsDir: string;
  dataDir: string;
};

export const CAPABILITY_LABELS: Record<string, { label: string; hint: string }> = {
  image_in: { label: "图片输入", hint: "让模型能识别图片（vision）" },
  video_in: { label: "视频输入", hint: "让模型能处理视频" },
  thinking: { label: "思考", hint: "支持思考/推理模式" },
  always_thinking: { label: "始终思考", hint: "强制开启思考，通常需同时勾选「思考」" },
  tool_use: { label: "工具调用", hint: "支持 function calling" },
  dynamically_loaded_tools: {
    label: "动态加载工具",
    hint: "按需加载工具，减少上下文占用",
  },
};

export const EFFORT_LABELS: Record<string, string> = {
  low: "低",
  medium: "中",
  high: "高",
  max: "最高",
};

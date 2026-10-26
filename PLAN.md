# KC-Switch — Kimi Code 配置管理工具规划

## 1. 定位

跨平台桌面应用，统一管理 Kimi Code（CLI 与 Desktop 共用同一套配置）的 **API Provider / MCP / Skills / Plugins**。对标 CC-Switch，但**不做网关**（无本地路由、熔断、故障转移），只做「配置读写 + 一键切换 + 生命周期管理」。

## 2. 非目标

- 不做 API 网关/代理/路由/熔断/健康检查自动切换
- 不做多 AI 应用支持（只管 Kimi Code）
- 不做 Prompts 管理（Kimi Code 无独立 prompts 配置面，暂不引入）
- 不接管 Kimi Code 自身的安装/升级

## 2.5 约束（硬性）

- **禁止向 C 盘产生新文件**（源码、依赖缓存、构建产物、临时文件、日志等一律落 F 盘）；C 盘**已有**的内容（如 Rust 工具链 `~/.cargo` / `~/.rustup`）直接读取使用，不重复下载、不搬移。确需向 C 盘写新文件必须经用户审批，附上：是什么文件、为什么必须放 C 盘、大小。

## 3. 数据面盘点（已在本机核实）

| 数据 | 文件 | 结构要点 |
|---|---|---|
| Provider / API | `~/.kimi-code/config.toml` | `[providers."名称"]`（含 baseUrl/key/oauth），`[models."provider/model"]`（`provider`、`default_effort`），顶层 `default_model` 指向当前生效模型 |
| MCP | `~/.kimi-code/mcp.json` | `mcpServers`：stdio（`command`/`args`/`env`）或 HTTP（`url`/`headers`/`bearerTokenEnvVar`），`deferred` 开关 |
| Plugins | `~/.kimi-code/plugins/installed.json` | 数组：`id`、`root`、`source`（`zip-url`/`github`）、`enabled`、`github.ref/sha`、`originalSource`；实体在 `plugins/managed/<id>/` |
| Skills | `~/.agents/skills/<name>/SKILL.md`（用户级），`<project>/.agents/skills/`（项目级），插件目录内 skills（只读展示） | **无官方启停字段**，纯目录扫描发现 → 禁用 = 移动目录 |
| 其他 | `~/.kimi-code/tui.toml`、`[[hooks]]` | 二期再考虑 |

## 4. 技术选型（已与用户确认）

**Tauri 2 + Rust 后端 + React 18 + TypeScript + Vite**

| 层 | 选型 | 理由 |
|---|---|---|
| 桌面壳 | Tauri 2 | 安装包 ~10MB、内存低；CC-Switch 同栈 |
| 后端 | Rust + `toml_edit` / `serde_json` | `toml_edit` 是关键：改 config.toml 时**保留注释与格式**（config.toml 含用户手写注释，用 `serde` 序列化会毁掉它们） |
| 文件监听 | `notify` | 检测 Kimi Code 端的配置改动，UI 自动刷新 |
| 前端 | React 18 + TS + Vite + Tailwind CSS + zustand + shadcn/ui | 主流组合，组件质量高 |
| KC 自身数据 | `~/.kc-switch/state.json`（serde_json） | 只存 UI 偏好/收藏/预设，量小，不需要 SQLite |
| 打包 | tauri-builder：NSIS（Windows）+ dmg（macOS）+ AppImage/deb（Linux） | |
| 测试 | Rust 侧纯函数单测（临时目录夹具）；前端 vitest | |

## 5. 架构

```
kc-switch/
├── src-tauri/
│   ├── src/
│   │   ├── core/                 # 纯 Rust 库，不依赖 tauri，可独立单测
│   │   │   ├── kimi_paths.rs     # 定位 ~/.kimi-code/*，兼容 portable 场景
│   │   │   ├── providers.rs      # config.toml 读写（toml_edit）
│   │   │   ├── mcp.rs            # mcp.json 读写 + schema 校验
│   │   │   ├── plugins.rs        # installed.json 读写
│   │   │   ├── skills.rs         # 目录扫描/启停/安装
│   │   │   ├── backup.rs         # 写前备份 + 原子写（temp+rename）
│   │   │   └── watcher.rs        # notify 封装，事件去抖
│   │   ├── commands/             # #[tauri::command] 薄封装，按模块分文件
│   │   └── main.rs / lib.rs
│   └── tauri.conf.json
├── src/                          # React 前端
│   ├── pages/  Providers | Mcp | Skills | Plugins | Settings
│   ├── stores/ (zustand) / components/ / lib/ipc.ts (typed invoke 封装)
└── package.json
```

- IPC：所有操作走 `invoke`，命令名按域分组（`providers_list`、`mcp_upsert`、`skills_toggle`…），前后端共享 TS 类型（`ts-rs` 从 Rust struct 生成）。
- 写入纪律：任何写操作 = 读最新 → 备份（`config.toml.kc-bak-<时间戳>`，保留最近 10 份）→ 修改 → 原子替换。检测到 Kimi Code 正在写（文件 mtime 抖动）则重读后再改。
- 写完提示「重启 Kimi Code 生效」（无网关，不做热加载）。

## 6. 功能模块

### 6.1 Provider / API（完整配置管理）
- 列表页：全部 `[providers.*]` + 其下 `[models.*]`，密钥默认打码（`sk-***last4`），点击可见需确认
- 一键切换：改顶层 `default_model`（下拉展示 `provider/model` 全量条目）
- 增/改/删 provider：名称、baseUrl、API key（表单）、OAuth 型 provider（`managed:kimi-code`）标记为只读
- 增/改/删 model 条目：`provider/model` 名、`default_effort`
- 写入用 `toml_edit` 保注释；`managed:` 前缀条目禁改
- 可选连通性测试：直接 `GET <baseUrl>/models`，仅提示用

### 6.2 MCP
- mcp.json 全量 CRUD；表单区分 stdio（command/args/env/cwd）与 HTTP（url/headers/bearerTokenEnvVar）
- `deferred` 开关一列直改
- JSON 实时校验 + 常见错误提示（npx 命令不存在等静态检查，可选）
- 环境变量引用（`bearerTokenEnvVar`）只存变量名，不存值

### 6.3 Skills（全生命周期）
- 扫描三个来源：用户级 `~/.agents/skills`、项目级 `<cwd>/.agents/skills`（选择器选项目）、插件内置（只读）
- 列表展示：名称、描述（解析 SKILL.md frontmatter）、来源、大小
- **禁用/启用**：移入 `~/.agents/skills-disabled/<name>/`（位于 skills 目录之外，避免被递归扫描误发现；项目级用 `<project>/.agents/skills-disabled/`）——这是本方案的推定实现，M1 阶段先做一次真实启停验证再固化
- 删除（回收站式：移入 `.kc-switch-trash/` 而非直接 rm）
- 安装：粘贴 GitHub 仓库/release zip 或本地 zip → 解压校验含 `SKILL.md` → 落入 skills 目录；已存在则提示覆盖
- 更新检查：对 `source=github` 的记录比对最新 tag/sha（走 `git ls-remote`，无需 token）
- 详情：SKILL.md Markdown 渲染预览
- 搜索/筛选（384 个 skill 的场景必须有）

### 6.4 Plugins（全生命周期）
- 读 `installed.json` 列表：名称、来源、版本/tag/sha、`enabled` 状态、安装/更新时间
- 启用/禁用：直接改 `enabled` 字段（官方机制）
- 删除：删 `plugins/managed/<id>/` 目录 + 移除注册表条目（二次确认，同样走备份）
- 安装：zip URL 或 GitHub 仓库（owner/repo），复用 Kimi Code 的既有布局（`root` 指向解压目录），写入 `installed.json` 条目；官方源（`code.kimi.com/plugins/official/`）列表可作为内置推荐源
- 更新：github 源比对 ref/sha，zip 源提示重装

### 6.5 设置页
- Kimi Code 数据目录自定义（覆盖自动探测）、备份策略、语言（zh/en）、开机自启、托盘常驻、主题

## 7. 关键设计决策（我的判断，可复核）

1. **`toml_edit` 保格式**是硬要求 —— config.toml 是用户可手编文件，注释丢失不可接受。
2. **密钥不复制**：API key 只在 config.toml 单点存在，KC-Switch 自身不另存副本，降低泄露面；UI 打码 + 明确的「显示」交互。
3. **Skills 禁用用目录移动**而非改 SKILL.md 文件名：可逆、可批量、不污染文件内容；先验证再固化。
4. **备份 + 原子写 + 文件监听**三件套，防与 Kimi Code 自身的写入竞争。
5. 单体 Tauri 应用即可，不需要拆 CLI；core 层保持与 tauri 解耦，将来要加 CLI 直接复用 core。

## 8. 实施阶段

| 阶段 | 内容 | 验收 |
|---|---|---|
| M0 骨架 | create-tauri-app 脚手架、CI（fmt/clippy/test + 前端 build）、窗口/托盘/路由骨架 | 空应用可打包运行 |
| M1 core 层 | paths/backup/watcher + providers/mcp/plugins/skills 五个读写模块 + 单测（临时目录夹具） | 单测绿；**真实环境验证 skills 目录移动启停** |
| M2 Providers UI | 列表/切换/CRUD/打码 | 切 default_model 后 Kimi Code 重启生效 |
| M3 MCP UI | CRUD/deferred/校验 | 增删一个 MCP 后 Kimi Code 识别 |
| M4 Skills UI | 扫描/启停/删除/安装/预览/搜索 | 安装一个 GitHub skill 并启用生效 |
| M5 Plugins UI + 设置页 | enabled 切换/安装/更新/删除；设置项 | 禁用插件后其 skills/MCP 消失 |
| M6 收尾 | zh/en i18n、打包三平台安装包、自动更新（tauri updater，可选） | 安装包可装机 |

## 9. 风险与对策

- **Kimi Code 升级改变配置 schema**：core 层解析用宽松模式（未知字段原样保留——toml_edit 天然做到），schema 变更只新增不破坏。
- **Skills 启停机制未获官方支持**：M1 先实测；若发现 Kimi Code 递归扫描 skills-disabled，改用改名方案（`SKILL.md` → `SKILL.md.off`）兜底。
- **并发写竞争**：监听 + 写前重读 + 原子替换，冲突时提示用户手动合并。
- **OAuth provider（managed:kimi-code）不可编辑**：UI 明确标记只读，避免破坏登录态。

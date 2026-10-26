# KC-Switch 项目规则

## 硬性规则：禁止向 C 盘产生新文件

- **禁止向 C 盘产生新文件**（源码、依赖缓存、构建产物、临时文件、日志等一律落 F 盘）；C 盘**已有**的内容（如 Rust 工具链 `~/.cargo` / `~/.rustup`）直接读取使用，不重复下载、不搬移。确需向 C 盘写新文件必须经用户审批，附上：是什么文件、为什么必须放 C 盘、大小。
- 项目内已做的合规措施（不要破坏）：
  - `src-tauri/.cargo/config.toml` 里用 `[env]` 把 `KC_SWITCH_DATA_DIR` 指向 `../.kc-switch-data`（项目内），开发/测试产生的备份与回收站不落 C 盘。
  - 测试一律用 `tempfile` 或项目内 `.verify/` 目录，**不得写真实 `~/.kimi-code/config.toml`**。
- 注意：打包成安装版运行时，`data_dir()` 会默认用 `%LOCALAPPDATA%\KC-Switch`（备份/回收站）。这是产品的正常行为，但**开发阶段必须靠上面的 env 覆盖**，不要让它落 C 盘。

## 产品定位（重要）

KC-Switch 是 **Kimi Code 的配置编辑器**，不是模型切换器。

- **核心是「添加/编辑」**：官方未收录的模型，用户需要手动补 `capabilities`（尤其 `image_in` 让模型能识图）、`support_efforts`（思考强度档位）、`max_context_size` 等字段。这些原本只能手改 TOML，本工具要把它们变成可视化表单。
- 切换 `default_model` 只是附带能力——用户在 Kimi Code 内就能切，**不要把它当主功能来优化或大改**。
- 内置「配置检查」提示（`providers::diagnose_model`），把常见坑显式讲出来（识图没勾 image_in、档位不在 support_efforts 内、缺 max_context_size 等）。

## 数据面（已核实）

| 数据 | 路径 | 说明 |
|---|---|---|
| Provider / 模型 | `~/.kimi-code/config.toml` | `[providers.*]`、`[models."provider/model"]`、顶层 `default_model` |
| MCP | `~/.kimi-code/mcp.json` | `mcpServers`，stdio 或 http，`deferred` 延迟加载 |
| 插件 | `~/.kimi-code/plugins/installed.json` | `enabled` 字段可控启停 |
| Skills | `~/.agents/skills/` | 无官方启停字段 → 禁用 = 移到 `~/.agents/skills-disabled/`（同级兄弟目录） |

模型条目支持的字段：`provider`、`model`、`display_name`、`max_context_size`、`max_output_size`、`capabilities`、`support_efforts`、`default_effort`、`adaptive_thinking`、`reasoning_key`。

`capabilities` 已知取值：`thinking`、`always_thinking`、`image_in`、`video_in`、`tool_use`、`dynamically_loaded_tools`。
`support_efforts` 已知取值：`low`、`medium`、`high`、`max`。

## 模型发现（获取模型列表）

- 实现：`core/discovery.rs`（纯解析，可离线单测）+ `core/model_discovery.rs`（编排）+ `core/http.rs`（调系统 curl）
- **不用 reqwest/ureq**：本机 crates.io 下载极慢，且项目要求少新增依赖。`http.rs` 调系统自带 `curl.exe`（Windows 10+ / macOS / Linux 都有，带 TLS）
- 数据流：`GET {base_url}/models` → 按 base_url 在 models.dev 目录匹配供应商 → 补全 `limit.context`、`limit.output`、`modalities`、`reasoning_options`、`interleaved.field`
- 目录缓存在 `data_dir/cache/models.dev.json`，TTL 7 天
- 目录结构有坑：`interleaved` 有时是对象有时是布尔，所以用 `serde_json::Value` 接
- 对应测试：`tests/discover_e2e.rs`、`tests/catalog_merge.rs`（真实网络，失败时跳过不阻断）

### 三个必守细节（都踩过坑）

1. **必须 CREATE_NO_WINDOW**：Tauri 是 GUI 程序，调用控制台程序 `curl.exe` 时 Windows 会自动弹黑框。`http.rs::no_window()` 用 `creation_flags(0x0800_0000)` 抑制。
2. **必须 async + spawn_blocking**：网络请求同步跑在命令线程会让窗口「未响应」。`providers_discover_models` 是 `async fn`，内部 `tauri::async_runtime::spawn_blocking`。
3. **必须支持代理**：很多供应商域名需要走代理。`detect_proxy()` 依次读 `HTTPS_PROXY` 等环境变量、Windows 注册表 `ProxyServer`；`get()` 先试代理再试直连，全部失败才报错，并把"走了哪条路径"写进 notes 给 UI。

其他坑：
- `-o 文件` 下载模式下 stdout 只有状态码（没有换行），解析状态码要用 `rsplit('\n').next()` 而不是 `rsplit_once('\n')`
- 优先用 `C:\Windows\System32\curl.exe` 绝对路径，避免 PATH 里 Git 的 MSYS 版 curl 抢先（协议路径处理不同）

## 视觉风格：对齐 Kimi Code

界面风格照搬 Kimi Code Desktop，**已实测提取设计令牌，写在 `DESIGN.md`**。改 UI 前先读它。

要守住的几条：
- 浅色主题（`#ffffff` 底 + `#f9fbfc` 侧栏），**不要做深色或花哨配色**
- 单一蓝色强调 `#1783ff`，其余全靠灰阶
- 分隔线用 **0.5px 发丝线**（`var(--p-hairline)`），不是 1px —— 这是精致感的关键
- 焦点环用 `box-shadow: 0 0 0 3px var(--color-accent-soft)`，不用默认 outline
- 动效 `0.12s / 0.16s`，缓动 `cubic-bezier(.16,1,.3,1)`，只动颜色/背景/阴影
- 行内操作按钮默认隐藏，hover 行才显现

Tailwind 令牌已映射到 `src/styles.css` 的 `@theme`（`bg-bg`、`text-ink-muted`、`border-line`、`bg-accent-soft`…）。**不要写死颜色值**，用令牌。

## 图标

`icon/kcs_icon_dark.png` 是源图（1398×1398）。改完图标后重新生成：

```bash
python -c "
from PIL import Image
import os
src = Image.open('icon/kcs_icon_dark.png').convert('RGBA')
out = 'src-tauri/icons'
for name, size in {'32x32.png':32,'128x128.png':128,'128x128@2x.png':256,'icon.png':512,
                   'Square44x44Logo.png':44,'Square150x150Logo.png':150,'Square310x310Logo.png':310,
                   'StoreLogo.png':50}.items():
    src.resize((size,size), Image.LANCZOS).save(os.path.join(out,name))
src.save(os.path.join(out,'icon.ico'), sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
"
# 侧栏用的 64px 图标
python -c "
from PIL import Image
Image.open('icon/kcs_icon_light.png').convert('RGBA').resize((64,64), Image.LANCZOS).save('public/icon.png')
"
```

## 技术栈与命令

- Tauri 2（Rust）+ React 19 + TypeScript 7 + Vite 8 + Tailwind 4
- `toml_edit` 保证改 `config.toml` 时**注释与格式不丢**（有测试守着）
- 版本对齐：Rust `tauri` 2.11.x ↔ npm `@tauri-apps/api` 2.11.x（`tauri build` 会校验，不要随意升级单侧）

```bash
# 前端
npm run dev            # 仅前端
npm run build          # 前端产物到 dist/
npx tsc --noEmit       # 类型检查

# Rust（离线复用 C 盘已有 crate 缓存）
cd src-tauri
CARGO_NET_OFFLINE=true cargo test --offline          # 单元测试
CARGO_NET_OFFLINE=true cargo test --offline --test user_scenario -- --nocapture
CARGO_NET_OFFLINE=true npx tauri build --no-bundle   # 出可执行文件
```

产物：`src-tauri/target/release/kc-switch.exe`（**不要**在仓库根建 `target/`，会重复占 6G+）。

## 写盘纪律（代码层）

所有写操作走 `core::write_guard`：
1. `lock()` 拿进程内写锁（防连点并发）
2. `read_tracked()` 读文件并记录 mtime
3. `write_if_unchanged()` 写前比对 mtime，若被 Kimi Code 自己改过则**拒绝写入并提示刷新**（Kimi Code 会实时改 config.toml，实测过）
4. 底层 `safe_write` = 备份（保留 20 份）+ 原子替换（temp + rename）

## 环境

- Windows；Bash 工具走 Git Bash（Unix 语法）。
- 工具链：rustc 1.97.1（C 盘，直接复用）、Node 24（F 盘）、MSVC 在 `F:\DevTools\VS_BuildTools`、WebView2 已装。
- 产品形态：桌面 GUI，管理 Kimi Code 的 API Provider / MCP / Skills / Plugins；**不做网关**。

# Kimi Code 设计系统参考

KC-Switch 的视觉风格照搬 Kimi Code Desktop。本文记录**实测提取**的令牌，改样式前先看这里，不要凭感觉加颜色或字号。

来源（只读，勿修改上游文件）：
- `F:\Kimi-Code\Kimi Code\resources\desktop-dist\assets\style-CZWGbm1l.css`（语义色、排版、间距、`--p-*`）
- `F:\Kimi-Code\Kimi Code\resources\desktop-dist\assets\main-BvXF8mSX.css`（Tailwind 工具类、`.dark` 的 `--ms-*`）

## 色彩

Kimi Code 是**浅色优先**的灰阶系统 + 单一蓝色强调。不要用深色底。

| 用途 | 令牌 | 值 |
|---|---|---|
| 页面底 | `--color-bg` | `#ffffff` |
| 侧栏 | `--color-sidebar-bg` | `#f9fbfc` |
| 卡面 | `--color-surface-raised` | `#ffffff` |
| 下沉面/表头 | `--color-surface-sunken` | `#f5f5f5` |
| 主文字 | `--color-text` | `rgba(0,0,0,.9)` |
| 次文字 | `--color-text-muted` | `rgba(0,0,0,.6)` |
| 弱文字 | `--color-text-faint` | `rgba(0,0,0,.45)` |
| 极弱/占位 | `--color-text-quaternary` | `rgba(0,0,0,.3)` |
| 分隔线 | `--color-line` | `rgba(0,0,0,.13)` |
| 悬停 | `--color-hover` | `rgba(0,0,0,.03)` |
| 选中 | `--color-selected` | `rgba(0,0,0,.05)` |
| 强调 | `--color-accent` | `#1783ff` |
| 强调悬停 | `--color-accent-hover` | `#167ff7` |
| 强调浅底 | `--color-accent-soft` | `#e8f3ff` |
| 危险/成功/提醒 | `--color-danger` `#c0392b` / `--color-success` `#0e7a38` / `--color-attention` `#ff9f0a` | |

阴影只有三档，都很轻：
```
--shadow-subtle: 0 1px 3px 0 rgba(0,0,0,.06)
--shadow-popover: 0 4px 6px -1px rgba(0,0,0,.08), 0 2px 4px -2px rgba(0,0,0,.06)
--shadow-modal:  0 10px 15px -3px rgba(0,0,0,.12), 0 4px 6px -4px rgba(0,0,0,.08)
```

## 排版

- UI 字体：`Schibsted Grotesk Variable` → `Helvetica Neue` → `Arial` → 中文回退 `Noto Sans SC` / `PingFang SC` / `Microsoft YaHei`
- 等宽：`JetBrains Mono Variable` → `ui-monospace` → `SF Mono` → `Menlo` → `Consolas`
- 基准字号 14px，标尺：

| 令牌 | 值 | 用途 |
|---|---|---|
| `--ui-c2` | 10px | 极小标签 |
| `--ui-c1` | 12px | 说明文字 |
| `--ui-b2` | 14px | 正文基准 |
| `--ui-b1` | 15px | 稍大正文 |
| `--ui-t2` | 16px | 小标题 |
| `--ui-t1` | 18px | 标题 |
| `--ui-t0` | 20px | 页面标题 |

行高：`1.25` 紧凑 / `1.5` 常规 / `1.6` 正文 / `1.7` 宽松。

## 间距与圆角

间距是 4px 基准：`2 / 4 / 6 / 8 / 12 / 16 / 20 / 24 / 32`。

圆角：`xs 4` / `sm 6` / `md 8` / `lg 12` / `xl 16` / `2xl 20` / `full 999`。
常用：按钮 `md`，卡片 `lg`，弹窗 `xl`，标签 `xs`。

## 关键细节（照抄这几条就有神韵）

1. **发丝线**：分隔线用 `0.5px`（`--p-hairline`），不是 1px。这是 Kimi Code 界面显得精致的关键。
2. **焦点环**：`box-shadow: 0 0 0 3px var(--color-accent-soft)`，不是默认 outline。
3. **侧栏宽 264px**（`--p-sidebar-w`），顶栏高 48px。
4. **动效**：`--duration-fast: .12s` / `--duration-base: .16s`，缓动 `cubic-bezier(.16, 1, .3, 1)`。快而软，不要用 300ms。
5. **过渡只动颜色/背景/阴影**，不动布局属性。
6. **滚动条 6px**，thumb `rgba(0,0,0,.15)`，悬停 `.28`。
7. **行内操作按钮默认隐藏**，hover 行才显现（`opacity-0 group-hover:opacity-100`）。

## 已发现但未采用的部分

- `--ms-*` 是另一套 HSL 三元组令牌（`.dark` 用），Kimi Code 主界面走的是 `--color-*`，所以 KC-Switch 只对齐 `--color-*`。
- Kimi Code 深色主题的语义色由 JS 动态注入，未在 CSS 中静态存在；KC-Switch 目前只做浅色，与上游默认一致。

## 图标

`icon/kcs_icon_{dark,light}.png`（1398×1398）→ `src-tauri/icons/*`（NSIS/ICO/ICNS）与 `public/icon.png`（侧栏 64px）。
生成脚本见 `AGENTS.md`。

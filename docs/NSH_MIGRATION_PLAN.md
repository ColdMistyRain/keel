# Keel 迁移到 Native Shell（nsh）方案

> 目标：把 Keel 的 UI 框架从 GPUI 切换到 [Native Shell（nsh）](https://nsh.dev/zh-CN/)。
> 结论：**可行，且比继续堆 GPUI 更符合长期目标**；但不建议直接推倒重写，先做 shell spike，再按阶段迁移。
> 参考版本：nsh `v0.5.0`，Apache-2.0，官方文档 `https://nsh.dev/zh-CN/`。
>
> **当前决策（2026-11-12）：只保留评估文档，暂不动代码、不安装 nsh、不创建 spike。**
> 若启动迁移：UI 用 Rust-only，先验证 Windows，再扩展 macOS/Linux。

---

## 1. 为什么 nsh 适合 Keel

nsh 的定位是“Rust-first 的原生 UI 框架”，不是 WebView 壳：

- **Rust 应用不带 QuickJS**：`nsh init --template rust` 生成纯 Rust 应用，UI、状态、事件都是 Rust，编译成原生代码。
- **桌面用系统自己的视图**：macOS 用 AppKit/CoreText，Windows 用 Win32/Direct2D/DirectWrite，Linux 用 GTK 4/Pango；文本输入、IME、选区、无障碍来自系统。
- **自带 IDE 需要的基础组件**（`nsh/base`）：
  - `Editor`：50+ 语言高亮、行号、多光标、折叠、软换行、查找替换、诊断/装饰、IME；
  - `DockArea` / `ResizablePanelGroup`：可停靠、分栏、标签组；
  - `Tree`、`Command`、`Menu`、`ContextMenu`、`Popover`、`Tooltip`、`Tabs`、`Toolbar`、`Table`、`DataTable`、`DiffView`、`Terminal`、`Toast`、`Scrollbar`；
  - `nsh/desktop`：窗口、标题栏、应用菜单、右键菜单、托盘、全局快捷键。
- **主题数据化**：`nsh.json` 的 `"tailwind"` 节点扩展 Tailwind 4 的 colors/radius/fontSize/breakpoints；Rust 用类型化样式方法，TSX 用 class。正好替代当前硬编码的 `idea_theme.rs`。
- **一个 CLI**：`nsh init/run/test/package`，自带构建、热更新（TSX）、无头测试、打包和签名；Rust 模板不需要 Node。
- **可嵌入**：nsh 提供 GPUI 后端（`nsh-platform-gpui`），也支持把 nsh `AppView` 嵌入原生应用；未来插件/动态 UI 可用签名 TSX 包。

对 Keel 的直接价值：原生 IME/文本输入、跨平台桌面、内置编辑器/停靠/树/命令面板，减少自己实现 GPUI 组件的成本。

### 1.1 源码核对（2026-11-12）

源码位置：`D:\Programming\Workspace\nsh`，当前为 nightly 分支 `b37c601b`，workspace 版本仍是 `0.5.0`。

已确认：

- **Rust 工具链要求**：`edition = "2024"`、`rust-version = "1.97"`；本机 `rustc 1.97.1` 满足。
- **没有内置 `nsh` CLI**：本机尚未安装，迁移时需要先安装。
- **编辑器是完整实现**，不是占位：
  - `crates/nsh-base/src/editor/` 包含 rope 缓冲、多光标、折叠、软换行、撤销、查找替换、诊断/装饰、语法高亮、补全、定义跳转、hover、IME；
  - 位置模型是 0-based line + UTF-16 column，和 LSP 一致；
  - 有 `EditorDiagnostic`、`EditorDecoration`、`EditorCompletionRequest`、`EditorDefinitionRequest` 等接口，适合接 LSP。
- **DockArea 是 IDE 级**：
  - `crates/nsh-base/src/dock/` 支持嵌套分栏、标签组、左右/底部停靠、拖拽标签、停靠尺寸、紧凑宽度；
  - 面板内容按 key 保留状态，移动标签不会重建内容。
- **主题是 shadcn 风格 token**：
  - `crates/nsh-base/theme.rs` + `default-theme.json`；
  - 主题文件是 `cssVars`：`light` / `dark` 两套变量 + `radius`，支持应用自定义 token；
  - 不是 IDEA 的 key 命名，需要一层“IDEA key → nsh token”的映射。
- **i18n 是内置能力**：
  - `crates/nsh-i18n` 基于 `rust-i18n`，支持 `locales/` 目录、JSON/YAML/TOML、17 种语言数据；
  - 不需要从零实现 `keel-i18n`，但 Keel 自己的词条目录仍要建。
- **示例可直接参考**：
  - `examples/gallery-rs`：纯 Rust 应用（无 TSX/QuickJS）；
  - `examples/desktop/App.tsx`：自绘标题栏和桌面菜单；
  - `examples/post-editor`：编辑器示例；
  - `examples/tasks-rs`：带 store、页面、列表、系统弹窗和测试的 Rust 应用；
  - `examples/embed-gpui`：nsh 嵌入 GPUI 的反向示例（nsh 有 GPUI 后端）。
- **GPUI 关系**：nsh 仓库自带 `crates/gpui`（GPUI 后端的实现），但独立桌面应用默认走 AppKit/Win32/GTK；GPUI 只是可选后端。

### 1.2 源码导读

| 目标 | 入口 |
| --- | --- |
| 编辑器 | `crates/nsh-base/src/editor/mod.rs`、`docs/dev/editor.md` |
| 停靠/分栏/标签组 | `crates/nsh-base/src/dock/dock_area.rs` |
| 树/命令面板/菜单/弹层 | `crates/nsh-base/src/tree.rs`、`command.rs`、`menu.rs`、`popover.rs` |
| 主题 | `crates/nsh-base/src/theme.rs`、`default-theme.json` |
| i18n | `crates/nsh-i18n/src/lib.rs`、`crates/nsh-i18n/data/` |
| Rust 应用示例 | `examples/gallery-rs/native/src/lib.rs`、`shell.rs` |
| 桌面标题栏/菜单 | `examples/desktop/App.tsx`、`crates/nsh-platform-*/` |
| 编辑器示例 | `examples/post-editor/` |
| Rust 应用测试 | `examples/tasks-rs/`、`crates/nsh-testing/` |

---

## 2. 关键决策

### 2.1 UI 用 Rust，不用 TSX（推荐）

nsh 文档明确建议：

- **Rust**：业务复杂、编辑器和数据密集的工具；
- **TSX**：发布后下发的界面、快速迭代、AI 生成 UI；
- **Rust + TSX**：Rust 核心 + TSX 页面。

Keel 是 IDE，属于“编辑器/数据密集工具”，应选 **Rust-only**：

- 不引入 QuickJS；
- 保留 Rust 类型系统与 `cargo test`；
- 业务逻辑和 UI 都在同一语言，便于重构；
- 未来插件/动态设置页可以再加 TSX。

### 2.2 先 spike，不直接替换

当前已有 44 张 UI 任务卡的 GPUI 实现。直接删除会同时失去可运行基线和视觉参考。建议：

- 保留现有 GPUI 版本为 `legacy`；
- 新建 `nsh-spike` 原型，跑通主窗口壳；
- spike 通过后再按阶段迁移，最后删除 GPUI。

### 2.3 Windows 优先

当前开发环境是 Windows。先验证 Win32/Direct2D 后端的：

- 自绘标题栏、窗口控制按钮、菜单；
- IME、字体、DPI；
- `nsh package` 产物；
- `nsh test` 无头测试。

macOS/Linux 放到后续阶段。

---

## 3. 迁移对照

| 当前 GPUI | nsh 对应 |
| --- | --- |
| `KeelApp` 实体 + `Render` | `#[component] fn App()` + signal/组件 |
| `div()` / `.flex()` / `.child()` | `view()` / 类型化样式方法或 Tailwind |
| `gpui_component::button::Button` | `nsh_base::button()` |
| `gpui_component::input::Editor` | `nsh_base::editor()` |
| 自绘菜单/Popover | `nsh_base` 的 `menu()` / `popover()` / `tooltip()`，桌面菜单用 `nsh_core::element::desktop` |
| 工具窗口/标签/分栏 | `dock_area()` / `resizable_panel_group()` / `tabs()` |
| 项目树 | `tree()`（虚拟化、展开/选中） |
| 命令面板 | `command()` |
| 通知气泡 | `toast()` / `toast_stack()` |
| `img()` / `svg()` | `image()` / 资源图标（需在 spike 验证 SVG 支持） |
| `idea_theme.rs` 常量 | `nsh.json` 的 `"theme"` 文件（shadcn `cssVars` 的 light/dark + 自定义 token），加一层 IDEA key 映射 |
| `Chrome` 色板 | 统一 token 访问，不再复制三份 |
| `CommandId` 枚举 | `nsh` 的 action/命令注册（自建 `keel-commands`） |
| 硬编码中文文案 | `nsh-i18n` 的 `locales/` 目录（rust-i18n），保留语义 key |
| 自绘主题常量 | `theme.json` 的 `cssVars.light/dark` + `radius`；组件只读 token |
| GPUI 自绘圆角/岛屿 | 原生视图的 `borderRadius`、背景、阴影、渐变；需要重新对 IDEA 度量做像素校准 |
| `cargo build` | `nsh run` / `nsh package`（Rust 模板仍用 cargo 构建，但由 nsh 生成 runner） |

---

## 4. 分阶段计划

### P0：nsh shell spike（先做）

目标：在不改 `src/` 的前提下，验证 nsh 能承载 IDEA 风格工作台。

- 安装 `nsh`（Windows PowerShell 安装脚本）；
- `nsh init keel-nsh-spike --template rust`；
- 实现：
  - 自绘标题栏 + 窗口控制按钮 + 项目/分支部件；
  - 左侧工具窗口条；
  - 项目树（`tree()`）；
  - 编辑器标签 + `Editor`；
  - 底部状态栏；
  - 主题 token（Islands Light/Dark）。
- 验收：
  - `nsh run` 在 Windows 打开窗口；
  - 编辑器能打开本地文件、输入中文（IME）、保存；
  - 标题栏/窗口按钮/菜单可用；
  - 主题切换生效；
  - `nsh package` 能产出 Windows 包；
  - 截图与现有 GPUI 版本对比，差距可接受。

### P1：抽 `keel-core`

- 把与 UI 无关的逻辑从 `KeelApp` 抽出：
  - `keel-core`：id、错误、事件、取消、命令；
  - `keel-vfs`：文件系统、监听、编码；
  - `keel-workspace`：项目快照、最近项目、会话；
  - `keel-theme`：IDEA token → nsh theme 的映射与加载；
  - `keel-i18n`：包装 `nsh-i18n`，只保留 Keel 自己的词条与回退策略。
- 现有 GPUI UI 改为调用这些 core，保证两边行为一致。

### P2：迁移工作台壳

- 标题栏、工具窗口条、项目面板、编辑器标签、状态栏、面包屑；
- 用 nsh 的 `dock_area`/`resizable_panel_group` 取代自研分栏；
- 用 `nsh_base` 的菜单/弹层/提示取代自绘组件；
- 保留 IDEA 度量表，逐项校准。

### P3：编辑器与代码智能

- 用 `nsh_base::editor()` 替换 gpui-component Editor；
- nsh Editor 已提供 `onChange`、诊断、装饰、补全、定义、hover、查找替换、折叠、多光标等钩子，Keel 只需实现 LSP 适配层：
  - 文档同步、诊断、补全、定义、引用、hover、格式化；
  - 语法高亮先用 nsh Editor 内置的 syntect，再按需接 tree-sitter/LSP semantic tokens；
- 验证大文件、多标签、查找替换、IME。

### P4：功能对齐

- 项目树右键、文件操作、命令面板、设置、通知、查找替换、工具窗口内容；
- 复用现有 44 张 UI 卡的验收标准，逐项截图对比。

### P5：切换与清理

- 默认构建改为 `nsh`；
- 删除 GPUI 依赖与自绘主题常量；
- 更新 `DEVELOPMENT_OUTLINE.md`、构建脚本、CI；
- 保留 GPUI 分支作为历史参考，直至 nsh 版本稳定。

---

## 5. 风险与缓解

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| nsh v0.5.0 还年轻，源码在 nightly 分支上继续变 | 迁移后维护成本 | 固定版本/commit；UI 收敛到 `keel-ui` crate；关注 changelog |
| 原生后端（Win32/AppKit/GTK）与 GPUI 渲染差异 | IDEA 像素级视觉需要重调 | P0 先验证标题栏/岛屿/编辑器；保留度量表逐项校准 |
| `nsh_base::editor` 与 Keel 的编辑器需求 | 编辑器是核心 | 源码已确认有 rope、多光标、折叠、查找替换、诊断/装饰、补全/定义/hover 钩子、IME；P0 仍需用真实 Java 文件验证 |
| 主题/样式模型从 GPUI 改为 Tailwind tokens | 现有 `idea_theme.rs` 要重写 | 先把 IDEA 主题映射到 `nsh.json` 的 colors/radius/fontSize；保留 token 名 |
| 打包/签名流程变化 | 发布链路要重做 | P0 验证 `nsh package`；后续接 CI |
| 迁移期间两套 UI | 双份维护 | 先抽 `keel-core`，UI 只做薄层；设定切换截止点 |
| 失去 GPUI 的任意自绘能力 | 复杂岛屿/动画受限 | 先用样式/渐变/阴影/canvas 验证；必要时用 nsh 原生视图扩展 |
| 插件/动态 UI 需求 | 未来扩展 | Rust-only 先稳定；后续用 nsh 的签名 TSX 包 |

---

## 6. 不建议的做法

- **直接把 `src/` 改成 TSX**：Keel 是复杂 IDE，TSX 会引入 QuickJS 和前后端状态分裂；先 Rust-only。
- **一次性删除 GPUI**：会失去可运行基线和 44 张卡的视觉参考。
- **同时支持 GPUI + nsh 两套长期并行**：只允许迁移期共存，设定删除日期。
- **在 nsh 上继续复制三份主题/硬编码文案**：迁移时一并资源化。

---

## 7. 当前状态与启动条件

**当前不执行迁移**：现有 `src/` 继续使用 GPUI，不安装 `nsh`，不创建 spike。

启动 P0 的条件（满足任意两条再评估）：

- nsh 发布更稳定的版本，且 Editor/Dock/Tree 的 API 不再频繁变动；
- 现有 GPUI 在 IME、无障碍或跨平台上出现无法接受的瓶颈；
- Keel 需要正式支持 macOS/Linux，且希望复用同一套原生 UI；
- 决定引入 nsh 的签名 TSX 包作为插件/动态 UI 方案。

启动时按以下顺序执行：

1. 安装 `nsh` 并创建 `nsh-spike` Rust 模板；
2. 跑通 Windows 主窗口 + 编辑器 + 主题；
3. 用截图对比现有 GPUI 版本；
4. spike 通过后，按 P1–P5 迁移。

在 spike 结果出来前，不修改现有 `src/` 的 GPUI 实现。

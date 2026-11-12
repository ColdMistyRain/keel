# Keel 迁移到 Native Shell（nsh）方案

> 目标：把 Keel 的 UI 框架从 GPUI 切换到 [Native Shell（nsh）](https://nsh.dev/zh-CN/)。
> 结论：**可行，且比继续堆 GPUI 更符合长期目标**；但不建议直接推倒重写，先做 shell spike，再按阶段迁移。
> 参考版本：nsh `v0.5.0`，Apache-2.0，官方文档 `https://nsh.dev/zh-CN/`。
>
> **当前决策（2026-11-12）：迁移已启动。** nsh 0.5.0 已安装，P0 spike 落在
> `spike/keel-nsh-spike/`（Rust-only UI，Windows 优先）；现有 GPUI 版本继续作为基线保留。
> P0 的记录见 §4.1。

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

### 4.1 P0 spike 记录（2026-11-12）

**结论：P0 通过（Windows，nsh 0.5.0）。** spike 位于 `spike/keel-nsh-spike/`，
纯 Rust UI，不改动仓库根目录的 GPUI 版本。运行方式与结构见
[`spike/keel-nsh-spike/README.md`](../spike/keel-nsh-spike/README.md)。

已实现：

- 自绘 40px 标题栏：`.window_drag(true)` 拖动窗口、按 `title_bar().right`
  给系统窗口按钮留白；
- 应用菜单（文件 / 视图 / 帮助）、外观切换（自动 / 浅色 / 深色，`set_color_scheme`）；
- 左侧 40px 工具窗口条 + 项目面板（spike 自绘 `scroll` 列表，见下方问题 1），
  数据来自真实目录扫描，可展开 / 收起、单击选中、双击打开；
- 编辑器区：自绘标签页、面包屑、`nsh_base::editor`（Java 高亮、行号、折叠）；
- 状态栏：分支、提示、语言、编码、换行、缩进、光标位置；
- 主题：颜色、圆角、编辑器语法配色、最近项目调色板全部来自仓库根目录的
  `themes/idea-light.json`、`themes/idea-dark.json`，启动时由 `keel-theme`
  转成 nsh `cssVars` 并 `set_tokens` 注入；Rust 里没有色值常量；
- 原生打开对话框（`pick_paths`）+ `std::fs` 读写保存，快捷键
  `mod+o` / `shift+mod+o` / `mod+s` / `mod+w`；
- 示例项目 `samples/demo`，`KEEL_SPIKE_DEMO=1` 时启动即打开，便于截图与验收。

截图：[浅色](screenshots/nsh-spike-light.png) · [深色](screenshots/nsh-spike-dark.png)。

验证结果：

| 项 | 结果 | 说明 |
| --- | --- | --- |
| `cargo fmt` / `clippy` | 通过 | spike crate 无警告 |
| `nsh run`（Windows） | 通过 | Win32 后端打开 1180×680 窗口 |
| Rust 热重载 | 通过 | 改动后约 5 秒编译 + 7 秒重建 runner |
| 编辑器 | 通过 | Java 高亮、行号、折叠、中文显示 |
| 标签页 / 面包屑 / 状态栏 | 通过 | 截图与 headless 均确认 |
| 项目树 | 通过（自绘） | 展开到 `Main.java` 的层级、选中高亮、单击展开、双击打开 |
| `nsh_base::Tree` | **不可用** | headless 里行已挂载，但 Windows 窗口不绘制（父视口虚拟列表）；已改用自绘列表 |
| 主题配置 | 通过 | 颜色/度量来自 `themes/idea-*.json`，运行时注入；浅色 / 深色截图归档 |
| 编辑与保存 | 通过 | 编辑器输入 → `Ctrl+S` → 磁盘内容更新（测试后已还原示例） |
| 中文输入 | 基本通过 | `SendKeys` 注入「中文输入」并保存成功；IME 组合（拼音候选）仍需人工验证 |
| `nsh test` | 通过 | `1 test: 1 passed`（headless，Rust 根组件从 TypeScript 侧打开） |
| `nsh package --target windows` | 通过 | `.nsh/out/Keel.zip`（约 2 分半 release 构建）；`.iss` 已生成，安装包需要本机装 Inno Setup 6 |

需要注意的坑（已写进 spike 代码）：

1. **`nsh_base::Tree` 在 0.5.0 的 Windows 窗口里不绘制行。** 无头测试能挂载
   `TreeItem` 行，但窗口里树区域空白；最小复现：给 `tree()` 固定 `h/w` 并
   `render_item` 画纯色块，纯色块也不出现。同样的 `virtual_list`
   （`ListViewport::Parent`，放在 `scroll` 里）也不绘制，而默认视口的
   `virtual_list` 正常。spike 因此改用 `scroll` + `dynamic` 自绘列表；
   P2 再复评或向 nsh 上报。
2. **`when` / `otherwise` 的分支不参与父级 flex 高度分配。** 需要高度的内容
   要么直接作为 flex 子项，要么用 `style_when(.., |s| s.hidden())` 常驻并切换可见性；
   否则会以内容高度塌缩。
3. **`*_style` 闭包的类型是 `StyleRefinement -> StyleRefinement`**，不是 `Style`；
   写 `fn popup_style(s: Style) -> Style` 会编译失败。
4. **`nsh test` 继承环境变量**：设了 `KEEL_SPIKE_DEMO` 时测试也会打开示例项目，
   断言不能依赖「就绪」这类启动文案。
5. **自定义主题令牌可以直接进类名**：`bg-editor-bg`、`text-muted-fg`
   与 `theme().color("editor-bg")` 等价，深色值由 `dark` 节点提供。
6. Windows 上窗口尺寸默认值要小于屏幕可用高度（本机 1280×720），否则窗口按钮
   会被挤出屏幕。

下一步（P1）已开始，见 §4.2。

### 4.2 P1 进展：按 IDEA 架构分层（2026-11-12）

P1 的目标不是“抽几个工具 crate”，而是照
[INTELLIJ_ARCHITECTURE_REFERENCE.md](./INTELLIJ_ARCHITECTURE_REFERENCE.md)
的 §3、§5、§6、§8、§10 把 Keel 分成同样的层：

| IDEA 机制 | Keel 目标 | 状态 |
| --- | --- | --- |
| Service Container（Application / Project / Module） | `keel-core`：`ServiceContainer` + `AppServices` / `ProjectServices`，懒加载、可释放 | 已落地基础 |
| Disposable 树 | `keel-core::Disposer`，逆序清理 | 已落地 |
| MessageBus / Topic | `keel-core::MessageBus` + 类型化 `Topic`，订阅句柄自动退订 | 已落地 |
| Workspace Model（不可变快照 + 变更广播） | `keel-workspace`：`ProjectModel` / `ProjectSnapshot` / `PROJECT_MODEL_CHANGED` | 已落地 |
| Theme JSON + 编辑器 scheme | `themes/idea-*.json` + `keel-theme` | 已落地 |
| Action / Command + DataContext | `keel-commands`：菜单、工具栏、快捷键、命令面板共用一份动作定义 | 下一步 |
| VFS（`VirtualFile`） | `keel-vfs`：本地/远程/归档统一抽象，UI 不再直接 `std::fs` | 下一步 |
| Document / PSI / Index | `keel-document` → `keel-syntax` → `keel-index` | P3 |
| ToolWindowManager / ToolWindowPane | nsh `dock_area` 的工具窗口 + 条纹 | P2 |
| FileEditorManager / EditorsSplitters | nsh 编辑器标签组 + 分栏 | P2 |
| IdeRootPane / MainToolbar / StatusBar / LayeredPane | nsh 自绘壳（标题栏、工具条、状态栏、弹层） | P0/P2 |
| PersistentStateComponent / Configurable | `keel-settings`：统一持久化，设置页 `reset/apply/isModified` | P4 |
| JPS 外部构建 | 不照搬，直接调用 Maven/Gradle | Phase 4/5 |
| Swing / EDT / LAF | 不照搬，用 nsh 原生视图（GPUI 只作迁移期基线） | 永久 |

依赖方向照 IDEA 的 `*-api` / `*-impl` 约束，UI 只依赖下层：

```text
keel-core
  ├─ keel-vfs ─ keel-document ─ keel-syntax ─ keel-index
  ├─ keel-workspace ─ keel-commands
  └─ keel-theme / keel-settings / keel-i18n
                          ↓
                  keel-ui（nsh） / GPUI 基线
```

当前 crate：

| crate | 内容 |
| --- | --- |
| [`crates/keel-core`](../crates/keel-core/src/lib.rs) | 服务容器、Disposable、MessageBus / Topic |
| [`crates/keel-workspace`](../crates/keel-workspace/src/lib.rs) | `ProjectModel` + 不可变 `ProjectSnapshot` + 变更事件、树摊平、语言识别 |
| [`crates/keel-theme`](../crates/keel-theme/src/lib.rs) | 读取 `themes/idea-*.json`，生成 GPUI 令牌与 nsh `cssVars` |

验证：

- `cargo test -p keel-core`：3 通过（懒加载服务、逆序清理、类型化事件）；
- `cargo test -p keel-workspace`：4 通过（语言、忽略规则、扫描 + 摊平、快照刷新与退订）；
- `cargo test -p keel-theme`：3 通过（配置解析、颜色/透明度、cssVars）；
- 根 GPUI `cargo test`：14 通过；spike `nsh test`：1 通过。

还没做的 P1 部分：

- `keel-commands`、`keel-vfs` 未开始；spike 的菜单与快捷键仍是手写，下一步改成动作注册表；
- `src/project_tree.rs`、`src/model.rs` 尚未改用 `keel-workspace`（GPUI 侧仍是自己的树模型）；
- 最近项目、会话（打开的文档/光标）还没有 `PersistentStateComponent` 式的持久化；
- `keel-i18n` 未开始。

### P1：抽 `keel-core`

- 按 IDEA 分层落地：
  - `keel-core`：服务容器、Disposable、MessageBus；
  - `keel-vfs`：文件系统、监听、编码；
  - `keel-workspace`：不可变项目快照 + 变更事件；
  - `keel-commands`：Action / ActionGroup / DataContext；
  - `keel-theme`：主题 JSON 的加载与 UI 令牌；
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

## 7. 当前状态

**P0 已完成（2026-11-12）**：nsh 0.5.0 已安装，`spike/keel-nsh-spike/`
是纯 Rust 的工作台 spike；现有 `src/` 的 GPUI 版本保持可运行，作为迁移期基线。

进行中的工作：

- 待桌面解锁后补：项目树修复后的截图、深色主题截图、中文 IME 手动输入验证；
- P1：抽 `keel-workspace` / `keel-theme`，让 GPUI 与 nsh 两个 UI 共用；
- P2：用 `nsh_base::dock` 替换自研分栏与工具窗口。

约束：

- 迁移期不删除 GPUI，直到 nsh 版本功能对齐（P5）；
- 固定 nsh 版本（当前 0.5.0），升级单独做任务卡；
- 不修改 nsh 源码，只读参考。

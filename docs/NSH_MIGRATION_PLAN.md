# Keel 迁移到 Native Shell（nsh）方案

> 目标：把 Keel 的 UI 框架从 GPUI 切换到 [Native Shell（nsh）](https://nsh.dev/zh-CN/)。
> 结论：**已切换**；nsh 版本是主线，GPUI 归档在 `legacy/gpui/`。
> 参考版本：nsh `v0.5.0`，Apache-2.0，官方文档 `https://nsh.dev/zh-CN/`。
>
> **当前状态（2026-11-12）：nsh 已提升为主线。** 工作台位于仓库根目录
> （Rust-only UI，Windows 优先，`nsh run .`）；原 P0 spike 已整体迁移到根目录，
> GPUI 基线归档到 `legacy/gpui/`，不再参与默认构建。P0–P2 记录见 §4.1–§4.4。

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

> 2026-11-12 更新：shell 已通过验证并提升为主线，GPUI 归档到 `legacy/gpui/`；
> 本节保留当时的决策过程。

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

**结论：P0 通过（Windows，nsh 0.5.0）。** 当时的 spike 已提升为仓库根目录的
主工程，纯 Rust UI；GPUI 版本归档在 `legacy/gpui/`。运行方式与结构见
[`README.md`](../README.md)。

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
- 示例项目 `samples/demo`，`KEEL_DEMO=1` 时启动即打开，便于截图与验收。

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
4. **`nsh test` 继承环境变量**：设了 `KEEL_DEMO` 时测试也会打开示例项目，
   断言不能依赖「就绪」这类启动文案。
5. **自定义主题令牌可以直接进类名**：`bg-editor-bg`、`text-muted-fg`
   与 `theme().color("editor-bg")` 等价，深色值由 `dark` 节点提供。
6. Windows 上窗口尺寸默认值要小于屏幕可用高度（本机 1280×720），否则窗口按钮
   会被挤出屏幕。
7. **`resizable_panel_group` 要放在有确定高度的容器里**：组自身 `size_full`，
   外面再包一层 `view().flex_1().h_full()`；否则 100% 高度按内容塌缩，
   底部面板和状态栏会被挤出窗口（P2 实测，见 §4.3）。

下一步（P1）见 §4.2；P2 外壳进展见 §4.3。

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
| Action / Command + DataContext | `keel-commands`：菜单、工具栏、快捷键、命令面板共用一份动作定义 | 已落地基础，命令面板待做 |
| VFS（`VirtualFile`） | `keel-vfs`：本地/远程/归档统一抽象，UI 不再直接 `std::fs` | 下一步 |
| Document / PSI / Index | `keel-document` → `keel-syntax` → `keel-index` | P3 |
| ToolWindowManager / ToolWindowPane | `resizable_panel` + 自绘条纹；`dock_area` 待评估 | P2 已落地核心 |
| FileEditorManager / EditorsSplitters | 单编辑器组 + 标签页；分栏 / 拖拽标签待做 | P2 已落地核心 |
| IdeRootPane / MainToolbar / StatusBar / LayeredPane | nsh 自绘壳（标题栏、工具窗口、底部窗口、状态栏、弹层） | P0/P2 已落地 |
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
| [`crates/keel-commands`](../crates/keel-commands/src/lib.rs) | 动作注册表：`Action` / `ActionGroup` / `DataContext` / `MenuNode` |
| [`crates/keel-theme`](../crates/keel-theme/src/lib.rs) | 读取 `themes/idea-*.json`，生成 GPUI 令牌与 nsh `cssVars` |

验证：

- `cargo test -p keel-core`：3 通过（懒加载服务、逆序清理、类型化事件）；
- `cargo test -p keel-workspace`：5 通过（语言、忽略规则、扫描 + 摊平、快照刷新与退订、项目根）；
- `cargo test -p keel-commands`：7 通过（注册表、上下文、可用性/勾选、菜单解析、执行、快捷键、动作列表）；
- `cargo test -p keel-theme`：3 通过（配置解析、颜色/透明度、cssVars）；
- 根 GPUI `cargo test`：迁移前 14 通过（归档在 `legacy/gpui/`）；nsh `nsh test`：9 通过。

还没做的 P1 部分：

- `keel-vfs` 未开始；主线仍直接用 `std::fs`；
- GPUI 归档（`legacy/gpui`）里仍保留它自己的树模型，不再维护；
- 最近项目、会话（打开的文档/光标）还没有 `PersistentStateComponent` 式的持久化；
- `keel-i18n` 未开始。

### 4.3 P2 进展：工作台外壳（2026-11-12）

按“优先实现外壳”的决策，P2 先落地 IDEA 的 `IdeRootPane` 结构；动作、
菜单和快捷键共用 `keel-commands` 的注册表。

**`keel-commands`（新增）**

- `Action` 稳定 ID + `update(DataContext, Presentation)` + `perform`；`CallbackAction`
  用闭包定义动作，可挂可用性 / 勾选谓词；
- `ActionGroup` 支持动作、分隔符、嵌套子菜单；`MenuNode` 是解析后的菜单树；
- `DataContext` 按类型存值并可带父上下文（IDEA 的 `DataContext` 链）；
- `ActionRegistry`：注册 / 替换动作、取展示状态、执行（尊重 enabled/visible）、
  解析菜单、列出快捷键。

**外壳（主工程）**

| 区域 | 实现 |
| --- | --- |
| 标题栏 | 应用菜单来自注册表：文件 / 视图 / 导航 / 帮助；菜单项带 16×16 SVG 图标与助记符下划线（`文件(F)`、`打开项目(P)…`，省略号在助记符之后）；项目名；右侧搜索 / 运行 / 设置快捷图标；快捷键也从注册表绑定（`mod+o`、`shift+mod+o`、`mod+s`、`mod+w`、`alt+1`、`alt+6`、`alt+f12`） |
| 启动与项目 | `KEEL_OPEN=<路径>`（开发）或打包产物的第一个路径参数；文件会向上识别项目根（`.git` / `pom.xml` / `Cargo.toml` 等） |
| 左侧工具窗口 | 项目 / 搜索 / 版本控制；`h_resizable` + `resizable_panel`，宽度 180–460、可折叠；标题栏有刷新 / 收起全部 / 隐藏 |
| 项目树 | 单击预览、双击固定；右键菜单（打开 / 预览 / 复制路径 / 在项目中定位） |
| 编辑器区 | 标签条横向滚动（溢出）、未保存圆点、关闭按钮、右键菜单（关闭 / 关闭其他 / 关闭全部 / 在项目中定位）；**预览标签**：项目树单击斜体预览、下一个预览替换、双击或编辑后固定；面包屑、`nsh_base::editor` |
| 命令面板 | `Ctrl+Shift+A` 复用动作注册表搜索；↑↓ 选择、Enter 执行、Esc 关闭；`overlay` 主题令牌做背景遮罩 |
| 底部工具窗口 | 问题 / 运行 / 终端占位；`v_resizable`，高度 90–360；状态栏左侧有图标切换条 |
| 状态栏 | 底部窗口条、提示、分支、语言、编码、换行、缩进、光标位置 |
| 动作状态 | 可用性与勾选态来自 `UiState`：无文档时禁用保存/关闭，工具窗口与外观菜单显示勾选 |

**布局注意事项**（补进 §4.1 的坑）

- `resizable_panel_group` 自身是 `size_full`，放进 `flex_1` 的行/列里时，外面要再包一层
  `view().flex_1().h_full()`；否则组的 100% 高度会按内容塌缩，底部面板和状态栏被挤出窗口。

**验证**

- `cargo test -p keel-workspace`：5 通过（含项目根识别）；
- `cargo test -p keel-commands`：7 通过；
- `nsh test`：9 通过（外壳文本、菜单、命令面板、项目树菜单，截图测试在无头下自动跳过）；
- `nsh test --target windows`：9 通过（真实窗口；含浅色/深色/菜单/预览/命令面板/项目树菜单截图）；
- `cargo clippy --all-targets -- -D warnings`：通过。

截图：[浅色](screenshots/nsh-p2-light.png) · [深色](screenshots/nsh-p2-dark.png) ·
[文件菜单](screenshots/nsh-p2-menu.png) · [预览标签](screenshots/nsh-p2-preview.png) ·
[命令面板](screenshots/nsh-p2-palette.png) · [项目树右键菜单](screenshots/nsh-p2-tree-menu.png)。

还没做的 P2 部分：

- 编辑器分栏 / 拖拽标签（`EditorsSplitters` 的多组编辑器）；标签重排；
- `nsh_base::dock_area` 的可停靠布局未采用（当前 `resizable_panel` 已够用，继续评估）；
- 命令面板目前只搜动作，不搜文件 / 类（Search Everywhere 的其余部分）；
- 工具窗口内容的拖放、浮动、最大化，最近项目与会话持久化。

### 4.4 与 IDEA 社区版（新 UI）对照

| IDEA 社区版 | 当前实现 | 结论 |
| --- | --- | --- |
| `IdeRootPane`：标题栏 → 工具窗口 + 编辑器 → 状态栏 | 同一结构 | 对齐 |
| 标题栏：主菜单、项目名、主工具栏（运行 / 搜索 / 设置） | 菜单、项目名、三个快捷图标 | 基本对齐（工具栏简化为图标） |
| View → Appearance 切换主题 | View → 外观（跟随系统 / 浅色 / 深色，带勾选） | 对齐 |
| ToolWindow 条纹（左侧、底部）+ 拖宽 + 折叠 | 左条纹、底部图标条、`resizable_panel` | 主要行为对齐（无浮动 / 最大化） |
| FileEditorManager / EditorsSplitters | 单编辑器组、标签页、预览标签 | 核心对齐；分栏 / 拖拽标签未做 |
| 项目树：刷新、收起全部、隐藏、展开 / 选中 | 同；文件图标用字母徽标代替 | 行为对齐，视觉待图标资源 |
| 编辑器：高亮、行号、折叠、面包屑、状态栏 | `nsh_base::editor` | 对齐（无 LSP / 诊断） |
| ActionManager：菜单 / 快捷键 / 命令面板共用动作 | `keel-commands` | 对齐 |
| 设置、持久化（最近项目、会话） | 无 | 缺口 P4 |
| 运行 / 调试、VCS | 占位 | 缺口 P4/P5 |

已知视觉差异：工具窗口头没有齿轮菜单；没有“最近项目”欢迎页；项目树里的
文件图标仍是字母徽标（菜单与右键菜单已用 SVG 图标）。这些不影响最小可用流程。

### 4.5 架构审计：平台层与 IDEA 社区版对照（2026-11-12）

**结论：外壳已经像 IDEA，平台层还没有接线。** 当前主工程是“单窗口 UI 应用 +
若干独立 crate”；`keel-core` 的服务容器、`ProjectModel`、`Disposable` 等机制
已经实现并有测试，但默认启动路径没有使用它们。

| IDEA 机制 | 现状 | 结论 |
| --- | --- | --- |
| Application / Project Service Container | `keel-core::ServiceContainer` 已实现；主工程 `Shell` 直接持有状态，从未创建 `AppServices` / `ProjectServices` | 未接线 |
| Disposable 树 / 项目关闭 | `Disposer` 已实现；没有项目打开 / 关闭生命周期 | 未接线 |
| MessageBus / Topic | `ProjectModel` 内部使用；主工程直接 `scan_snapshot` + 自己的 `Signal<Arc<ProjectSnapshot>>` | 模型被绕过 |
| Workspace Model（不可变快照） | 快照类型存在，但 `src/workspace.rs` 同时承担项目模型、编辑器标签和 UI 状态 | 部分 |
| VFS（`VirtualFile`） | `src/workspace.rs` 直接 `std::fs::read_to_string` / `fs::write` | 缺失（P3） |
| Document / PSI / Index | 文本直接存进 nsh 编辑器，没有 Document / PSI 层 | 缺失（P3） |
| ActionManager / DataContext | 注册表、`DataContext`、菜单和快捷键已统一；动作闭包仍直接捕获 `Shell` | 基本对齐，耦合待解 |
| FileEditorManager / EditorsSplitters | 打开 / 预览 / 固定 / 关闭 / 脏状态都在 `src/workspace.rs`；只有一个编辑器组 | 单组、无服务 |
| ToolWindowManager / Factory | `Tool` / `Bottom` 是硬编码枚举与 `match`，没有注册表 / 工厂 | 缺失扩展点 |
| IdeRootPane / MainToolbar / StatusBar | 结构、菜单、快捷图标、工具窗口条纹、状态栏都已落地 | 对齐（见 §4.4） |
| PersistentStateComponent / Configurable | 无设置、最近项目、会话持久化 | 缺失（P4） |
| 后台任务 / 读写锁 | 目录扫描和保存都在 UI 线程同步执行 | 缺失，项目大时会卡 UI |
| i18n | 文案硬编码在组件里 | 缺失 |

**主要风险**

1. 项目模型有两个来源：`keel-workspace::ProjectModel` 与 `src/workspace.rs` 的
   快照信号；打开 / 刷新项目绕过 `ProjectModel` 和 `MessageBus`。
2. 没有服务容器接线，后续 LSP、索引、设置等服务没有统一的生命周期与释放位置。
3. `src/workspace.rs` 正在变成“项目模型 + 编辑器管理器 + 文件 IO + UI 状态”的
   上帝对象，P3/P4 继续加功能会失控。
4. 动作定义直接捕获 `Shell`，命令面板、插件和工具栏难以复用同一批动作。

**建议的最小重构顺序**

1. **接线 `keel-core`**：`Shell::new` 创建 `AppServices`；项目打开时创建
   `ProjectServices`，关闭时 `dispose()`；`ProjectService` 持有 `ProjectModel`，
   UI 只订阅快照（用一层 nsh `Signal` 适配）。
2. **抽 `FileEditorService`**：把打开 / 预览 / 固定 / 关闭 / 脏状态从
   `src/workspace.rs` 移到独立服务，UI 只保留选中索引与渲染。
3. **最小 `keel-vfs`**：文件读写走 VFS 抽象，装配本地实现，替换 `std::fs`。
4. **ToolWindow 描述表**：把硬编码枚举换成描述符注册表，先不实现浮动 / 拖放。
5. **设置与持久化**：最近项目、会话、主题选择（P4）。

### 4.6 源码级视觉对照（2026-11-12）

对照 `intellij-community/platform/platform-resources/src/themes/islands/ManyIslandsLight.theme.json`
与 `platform/util/ui/src/com/intellij/util/ui/JBUI.java`、
`platform/platform-api/src/com/intellij/ui/tabs/impl/themes/TabTheme.kt`、
`platform/platform-impl/src/com/intellij/openapi/application/impl/islands/IslandsTabPainter.kt`、
`platform/platform-impl/src/com/intellij/ui/plaf/beg/{BegMenuItemUI,IdeaMenuUI}.java`、
`platform/platform-impl/src/com/intellij/ide/ui/laf/darcula/ui/DarculaMenuItemBorder.java`，
并在本机 IDEA 2026.2（`IU-262.10968.63`）里打开 `samples/demo` 逐项对照
（源码为 `271.SNAPSHOT`，少量度量以源码为准）：

| 项 | IDEA 源值 | 本轮调整 |
| --- | --- | --- |
| 主窗口背景 | `main-window-bg = layer-0-bg = gray-150 #E9EAEE` | 一致（`canvas`） |
| Tool window 背景 | `tool-window-bg = layer-2-bg = white` | 一致（`island-bg`） |
| Tool window 头高 | `JBUI.defaultHeaderHeight() = 41` | 32 → 41，并补底部分隔线 |
| Tool window 头内缩 | `headerLabelLeftRightInsets = 0,12,0,16` | 统一 10 → 左 12 / 右 8，标题后加下拉箭头 |
| Tool window 头按钮 | 16×16 图标（刷新 / 收起全部 / 关闭） | 文本字形 → 随包 SVG 图标 |
| Tool window 头字号 | `ToolWindow.Header.font.size.offset = 0`（默认 UI 字号） | 12 → 13 |
| 选中条纹按钮 | `ToolWindow.Button.selectedBackground = accent-brand-bg #3871E1`，`selectedForeground = text-over-accent` | 灰底 → 蓝底白图标 |
| 编辑器标签（Islands） | `IslandsTabPainter`：胶囊高 28、`hOffset 4`、`firstTabOffset 3`、`arc 12`（半径 6）；`underlinedTabBackground = blue-150 #E3EBFE`、`underlinedBorderColor = blue-120 #A7C5FF` | 4px 下划线 → 胶囊标签：条高 36、标签高 28、圆角 6、选中 1px 边框 |
| 标签未选中 | `unselectedAlpha = 0.75`、`unselectedBlend = 0.9` | 仍用 `muted-fg` 近似；未选中标签无底色 |
| 标签底边 | `EditorTabs.underTabsBorderColor = editor-border #E9EAEE` | 补 1px 分隔线 |
| 标签关闭 | 仅选中标签 / 悬停时显示 | 未选中且未修改时隐藏，悬停标签后淡入（`group_hover_named`） |
| 标签溢出 | `ScrollableSingleRowLayout`：溢出时在右侧留 more 区域并自动把选中标签滚进视口 | 测量内容 / 视口宽度，溢出时右侧出现左右滚动箭头（160px 翻页） |
| 项目树行高 / 字号 | IDEA 常规 24 / 13 | 22 → 24，12 → 13 |
| 项目树缩进 | `Tree.leftChildIndent = 7`、`rightChildIndent = 11`；`Tree.border = 4,12,4,12` | 每级 12 → 18，列表左右各 12 |
| 项目树图标 | 16×16 文件 / 文件夹图标，展开箭头用 `chevronRight/Down.svg` | 字母徽标 → `folder.svg` / `file.svg`，箭头改用 SVG |
| 文件类型图标 | `platform/icons/src/expui/fileTypes/java.svg`、`xml.svg`（Apache 2.0，含深色变体） | Java / XML 使用 expui 彩色图标，其余仍用通用文件图标 |
| 菜单项度量 | 高 26（`List.rowHeight 24 + Selection.outerInsets 1+1`）、选中块左右内缩 7、`Selection.arc = 8`、内容内缩 6 | 全宽高亮 → 左右内缩 7、圆角 4、高 26 |
| 主菜单触发器 | `Menu.Selection.outerInsets = 5,0`、`MainToolbar.Dropdown.arc = 12`，选中色 `toolbar-bg-hovered #00000008` | 高 30、圆角 6、左右内缩 4 |
| 菜单分隔线 | 弹出菜单用 `gray-150 #E9EAEE` | 新增 `menu_separator` 令牌 |
| 菜单项内容 | IDEA 的 16×16 动作图标 + 下划线助记符（中文界面为 `文件(F)`） | `icon_slot` 图标槽 + `mnemonic_label`（省略号在助记符之后） |
| 面包屑 | `BreadcrumbsComponent`：crumb 圆角 2、文字内缩 7、悬停 `toolbar-bg-hovered` | 纯文本 → 可悬停 crumb |
| 底部工具窗口标签 | `DefaultTabs.underlineHeight = 3`、`underlineColor #4083C9`，选中无底色 | 灰底 → 3px 下划线 |
| Run widget | `RedesignedRunWidget`：高 30、按钮 30×30、绿色 `#4E9D6C`（dark `#338555`）、停止红 `#E4656E`（dark `#C54E58`）；`Shift+F10` / `Ctrl+F2` / `Shift+F9` | 标题栏加运行按钮 + 配置选择器 + 调试按钮；运行中变红色停止按钮，底部「运行」窗口与状态栏同步 |
| 状态栏 | `background = main-window-bg`、`topBorderWidth = 0`、`Widget.iconBorder` 上下 6 | 高 24 → 28；字号 11 → 12 |
| 条纹按钮尺寸 | `StripeToolbar.Button = 37×40` | 32 → 36（条纹宽 40） |
| 弹出菜单 | `popup-bg = layer-2-bg`、`popup-border = layer-2-border`、`PopupMenu.borderCornerRadius = 8` | 一致 |

仍存在的视觉差距：

- 只有 Java / XML 有语言专用彩色图标，其他语言仍是通用 `file.svg`；
- 主工具栏没有 IDEA 的完整搜索框、AI 与其他 widget（当前是 run widget + 搜索 + 设置）；
- 菜单栏常驻，IDEA 默认收进左上角汉堡菜单（点击后才展开菜单栏）；
- 窗口按钮由系统绘制，不是自绘；
- 状态栏缺少内存、后台任务、通知等部件；底部工具窗口条仍在状态栏左侧，
  IDEA 把问题 / 终端放在左侧条纹的下半区。
- 运行只驱动 UI，没有真正的进程执行器。


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

> 核心外壳已落地，记录见 §4.3；下面保留原计划。

- 标题栏、工具窗口条、项目面板、编辑器标签、状态栏、面包屑；
- 用 nsh 的 `dock_area`/`resizable_panel_group` 取代自研分栏；
- 用 `nsh_base` 的菜单/弹层/提示取代自绘组件；
- 保留 IDEA 度量表，逐项校准。

### P3：编辑器与代码智能

- nsh Editor 已接入（高亮、行号、折叠、多光标、查找替换、诊断/装饰钩子）；
- Keel 只需实现 LSP 适配层：
  - 文档同步、诊断、补全、定义、引用、hover、格式化；
  - 语法高亮先用 nsh Editor 内置的 syntect，再按需接 tree-sitter/LSP semantic tokens；
- 验证大文件、多标签、查找替换、IME。

### P4：功能对齐

- 项目树文件操作、查找替换、设置、通知、工具窗口内容；
- 复用现有 44 张 UI 卡的验收标准，逐项截图对比。

### P5：切换与清理

- **已完成（2026-11-12）**：默认构建改为 `nsh`，工作台提升到仓库根目录；
- GPUI 归档到 `legacy/gpui/`（独立 workspace，不参与默认构建）；
- 待办：清理 GPUI 专属依赖/资源，更新 `DEVELOPMENT_OUTLINE.md`、构建脚本、CI；
- 确认 nsh 稳定后删除归档。

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

**P0 / P1 已完成（2026-11-12）**：nsh 0.5.0 已安装；`keel-core` /
`keel-workspace` / `keel-theme` / `keel-commands` 已抽出。**nsh 工作台已提升为
仓库根目录的主线**，GPUI 基线归档到 `legacy/gpui/`（独立 workspace，不参与默认构建）。

**P2 进行中**：工作台外壳（标题栏、工具窗口、编辑器标签、底部工具窗口、
状态栏、菜单与快捷键）已按 IDEA 的 `IdeRootPane` 结构落地，见 §4.3；
最小可用流程已打通：打开文件夹 / 文件作为项目 → 项目树浏览 → 单击预览 /
双击固定 → 语法高亮。

进行中的工作：

- 中文 IME 组合（拼音候选）仍需人工在窗口里验证；
- P2 剩余：编辑器分栏 / 拖拽标签、命令面板、`dock_area` 评估、会话持久化；
- P3：LSP 适配层、文档模型与索引。

约束：

- 默认构建只维护 nsh 主线；`legacy/gpui` 只作历史参考，不再加新功能；
- 固定 nsh 版本（当前 0.5.0），升级单独做任务卡；
- 不修改 nsh 源码，只读参考。

# IDEA 复刻项目架构对照：Lithe、OpenIDEA 与 Keel

> 参考仓库：
> - `D:\Programming\Workspace\Lithe-IDEA`
> - `D:\Programming\Workspace\openidea`
> - IDEA 原版：`D:\Programming\Workspace\intellij-community`
>
> 目的：这两套复刻都已经走过“Rust 后端 + IDE 工作台”的路，本文提炼它们的架构取舍，给 Keel 后续开发做对照，不代表要照搬。

---

## 0. 一句话结论

- **OpenIDEA** 最接近 Keel 的技术方向：Rust crate 后端 + 工作台前端 + LSP 优先 + 主题/语言资源化。它的 **crate 边界、typed IPC、action/keymap/context、theme tokens、i18n 目录** 最值得直接借鉴。
- **Lithe** 最接近 Keel 的产品目标：Java/Spring Boot、Maven、轻量内存、按需 LSP。它的 **Rust LSP runtime 归属、provider 降级、Java 入口/构建/启动边界、模块运行时与资源租约** 最值得借鉴。
- **Keel** 目前是 GPUI 单实体 + 硬编码主题/文案，优势是 UI 可以完全自绘、没有 WebView 开销；短板是业务逻辑全在 `KeelApp`，没有 core/domain 分层、没有 LSP/workspace/index、没有资源化体系。

---

## 1. 三套项目的定位

| 项目 | 技术栈 | UI | 语言/编辑器 | 后端 |
| --- | --- | --- | --- | --- |
| Keel（当前） | Rust + GPUI | GPUI 自绘 | gpui-component Editor | 单进程 `KeelApp` |
| Lithe | Rust Core + SwiftUI/AppKit（macOS）+ React/Tauri（Windows） | 原生双端 | Monaco 0.55.1 | Rust Core 通过 C ABI/JSON 共享 |
| OpenIDEA | Rust + Tauri 2 | React 19 + CodeMirror 6 | CodeMirror | Rust `ide-*` crates + `ide-host` |

---

## 2. Lithe 架构

### 2.1 仓库与所有权

```text
macos/            SwiftUI/AppKit 产品（参考产品）
windows/          React/Tauri Windows 产品
rust/             Rust Core 与宿主
  lithe-core/     共享命令、模型、校验、LSP runtime、C ABI
  lithe-ide-host/ IDE 宿主、进程与协议
  lithe-git-host/ Git 子进程与进程树
  lithe-agent-host/ ACP Agent
  lithe-remote/   远程
frontend/editor/  两端共享的 Monaco 表现层
shared/           contracts / fixtures / i18n / logging
Plugins/mac|win/  声明式插件包
```

核心规则：**平台层只负责 UI 和 OS API，其余逻辑必须进 Rust Core**。macOS 和 Windows 不共享 UI 代码，只共享 JSON 契约与 fixtures。

### 2.2 模块运行时

- `LitheModuleAPI` 定义平台无关的模块、依赖、能力、生命周期、资源、lease、贡献。
- `LitheApplicationKernel` 的 `ModuleRuntime` 负责注册、依赖图、按需构造、状态转换、资源回收、能力发布。
- 每个长期资源只登记到一个 `ModuleResourceScope`；`ModuleLease` 阻止休眠/回收。
- 禁用/隔离/Safe Mode 的可选模块不会构造，也不会偷偷启动 watcher/进程。
- 工作台只通过稳定 ID、能力和贡献目录使用模块，不直接拿另一个模块的具体 service。

### 2.3 语言工具

```text
Editor
  → LanguageToolingSessionManager
  → LanguageFeatureProvider 路由
       → LanguageServerFeatureProvider（真实 LSP）
       → BuiltinLanguageFeatureProvider（当前文件符号/轻量能力）
            → Rust LSP runtime
                 → gopls / jdtls / rust-analyzer ...
```

- LSP 子进程、stdio、JSON-RPC 状态机、文档版本、deadline、诊断、重启全部在 Rust Core。
- Provider 优先级：languageServer(200) > projectSymbols(100) > builtin(0)。
- Completion 合并去重；Hover/Definition/References 取第一个非空；LSP 不可用时降级到本地。
- Java 使用 JDT LS + Java Debug Server：先解析精确入口，再构建项目，再解析 classpath/module-path，最后由 Run 模块启动一次项目 JDK。
- Maven 资源处理（`resources:resources` / `testResources`）作为启动前置步骤，因为 JDT 增量构建不处理资源。

### 2.4 工作台 UI

- 工具窗口、侧栏、底部、右侧按稳定贡献 ID 注册和保存布局。
- 命令面板、主工具栏、菜单、快捷键读取同一份动作注册表。
- 工具窗口/标签的移动、隐藏、恢复、按需激活由宿主统一处理，插件只声明数据。
- 声明式插件包不加载 Swift/JS/HTML，UI 由宿主渲染。

### 2.5 主题与 i18n

- Windows 有 CSS token 主题扩展（内置多套 JSON 主题 + 自定义主题存储 + 主题文件校验）。
- macOS 的 UI 调色板仍在 `LitheTheme.swift` 中按主题枚举硬编码；编辑器有 `editor-syntax-theme-v1.schema.json` 契约。
- i18n 的唯一来源是 `shared/i18n/`：`manifest.json` + `messages.json` + `locales/*.json`，构建前生成 macOS `Localizable.strings` 与 Windows 字典；语义 key、参数类型、平台覆盖、回退链、覆盖率都有校验。
- **教训**：Lithe 的共享 i18n 很好，但 macOS 主题硬编码说明“逻辑进 Core”这条规则没有贯彻到主题；Keel 应把主题也当资源而不是代码。

### 2.6 值得 Keel 借鉴

1. LSP runtime 的唯一归属（不要在 UI 和平台各写一套）。
2. Provider 优先级与降级策略。
3. Java/Maven 的“入口 → 构建 → classpath → 启动”边界。
4. 模块运行时、能力、资源租约、按需激活。
5. 稳定 ID 的动作/工具窗口/布局贡献。
6. 共享 i18n 源 + 生成 + 校验。

### 2.7 不要照搬

- 两套 UI 栈（SwiftUI + React）带来的双份表现层维护。
- 平台层承担过多逻辑的诱惑。
- macOS UI 主题硬编码；应以资源文件为唯一来源。

---

## 3. OpenIDEA 架构

### 3.1 分层

```text
Webview (src/)
  → Generated bindings (src/bindings.ts)
  → src-tauri/ (glue: commands, events, windows)
  → ide-host (UI-less backend: Host, Project, ops/)
  → feature crates (ide-lsp, ide-index, ide-vcs, ide-run...)
  → ide-core (shared types)
```

规则：

- **一个领域一个 crate**：数据模型、I/O、协议、IPC 转换都在该 crate 内。
- **跨 crate / IPC 的类型只放 `ide-core`**。
- **不能访问另一个 crate 的内部**；需要共享就上移到 core。
- **`ide-host` 负责组装，`src-tauri` 只做适配**，不写领域逻辑。
- 错误在 crate 边界转成统一的 `IdeError` / `IdeResult<T>`。

### 3.2 IPC 契约

- Command：UI → Rust，一次返回 `IdeResult<T>`。
- Channel：Rust → UI 的流式结果（搜索、进程输出、终端字节、进度）。
- Event：后端主动通知（VFS 变化、诊断、索引状态、调试会话）。
- 所有类型通过 `specta` 生成 `src/bindings.ts`，前端不手写类型。
- 文本坐标统一：0-based line，UTF-16 column/offset。
- 可取消：UI 分配 `RequestId`，后端注册 `CancellationToken`；长任务必须检查取消。
- 这套契约同时服务远程开发：`openidea-server` 用 JSON-RPC 跑同一个 `ide-host`。

### 3.3 前端工作台

- `features/<name>/` 每个功能导出 `activate()`，返回 disposer。
- 功能之间禁止互相 import，只通过 action、contribution、service 通信。
- 所有用户可调用行为都是稳定 dotted id 的 action；菜单、快捷键、工具栏、命令面板共用。
- Keymap 与 action 分离；`when` 表达式基于 context keys。
- `workbench/contributions.ts` 提供工具窗口、状态栏、overlay、editor kind、title item 等扩展点。
- 重视图用 `lazyComponent` 延迟加载。

### 3.4 代码智能

四层来源：

1. LSP（补全、定义、引用、hover、诊断、重命名、格式化、inlay hints、语义 token）。
2. 自有索引（文件 + 符号，redb 持久化，nucleo 模糊查询）。
3. tree-sitter（outline、折叠、无 LSP 时的高亮、结构回退）。
4. 插件 provider。

特点：

- 没有 PSI：语义来自各语言服务器；IDE 自己的层是语法级。
- 索引不阻塞：查询回答已索引部分，`IndexState` 事件相当于 dumb mode 指示。
- 能力缺失返回 `unavailable`，编辑器按文档/功能退避。
- Java 用 JDT LS + `java-usages` / `kotlin-bridge` / Lombok agent 扩展。

### 3.5 主题与 i18n

- `src/styles/tokens.css` 是颜色/度量的唯一来源；组件禁止写颜色字面量。
- 主题插件是纯数据：`plugin.toml` + `*.theme.json` + 可选 `*.scheme.json`（编辑器配色）。
- 内置 7 套主题；支持从 IntelliJ IDEA / VS Code 导入主题。
- 主题只覆盖 token，未覆盖的继承 `kind` 对应的 Islands Light/Dark。
- i18n 双目录：source-text catalog（英文原文作 key）+ keyed messages；en/zh 必须完整，其他语言可回退；测试强制新增文案必须进两套目录。

### 3.6 插件与远程

- 插件是 WASM component，`wit/` 定义 API，wasmtime 每个插件一个 actor 线程和独立 store。
- 能力/授权/风险等级/签名/市场索引都在 `ide-extension`。
- 声明式 UI 贡献在 Rust 校验后投影到前端 `register*`。
- 远程开发把同一个 `ide-host` 作为 `openidea-server` 运行，前端通过路由把命令转发过去。

### 3.7 值得 Keel 借鉴

1. crate/domain 边界与 `core` 共享类型。
2. typed command + event + channel + cancellation 的 IPC 契约（即使先做进程内）。
3. action registry + keymap + context keys + contributions。
4. LSP 优先 + 自有索引 + tree-sitter 回退的代码智能分层。
5. token 化的主题系统与纯数据主题包。
6. i18n 目录与强制校验。
7. `ide-host::ops` 这种“同一实现同时服务桌面与远程”的结构。

### 3.8 不要照搬

- WebView + TypeScript 的复杂度；Keel 用 GPUI 可以更轻。
- 没有 PSI 的取舍：对 Java 深度重构/结构搜索会有上限，需要 JDT LS 扩展或自建语义层。
- 早期就上 WASM 插件沙箱；Keel 应先做进程内扩展点。
- 大量 UI 状态放在前端 store 的模式；GPUI 可以用实体 + 服务替代。

---

## 4. 三项目对照

| 维度 | Keel | Lithe | OpenIDEA |
| --- | --- | --- | --- |
| UI | GPUI 自绘 | SwiftUI/AppKit + React/Tauri | React + CodeMirror |
| 后端 | 单 `KeelApp` | Rust Core + 宿主 | Rust `ide-*` crates + `ide-host` |
| 进程边界 | 单进程 | Core C ABI/JSON + 平台进程 | Tauri IPC + 可选远程 server |
| 命令/动作 | `CommandId` 枚举 | 稳定动作 ID + 注册表 | 稳定 dotted action + keymap + when |
| 语言能力 | 无 | LSP + 轻量 provider 降级 | LSP + 自有索引 + tree-sitter |
| 项目模型 | 目录扫描 | 模块运行时 + 工作区 | `ide-workspace`（项目/文档/设置/最近项目） |
| 构建 | 未接入 | Maven/JDT 边界 | Maven/Gradle/Cargo/Go/TS |
| 运行/调试 | 未实现 | 运行配置 + DAP/Java | `ide-run` + `ide-dap` |
| 插件 | 无 | 声明式包 + 能力/安全模式 | WIT + WASM + 能力授权 |
| 主题 | Rust 常量 | Windows token 主题；macOS 硬编码 | tokens.css + 纯数据主题插件 |
| i18n | 硬编码中文 | shared/i18n + 生成 | source-text + keyed catalog + 测试 |
| 远程 | 无 | 远程工作区路由 | `openidea-server` 同构后端 |
| 测试 | 单元测试 + 截图 | fixture + 稳定性脚本 | unit/e2e/smoke/eval/perf |

---

## 5. 对 Keel 的具体建议

### 5.1 先拆 `keel-core`

把 `KeelApp` 中与 UI 无关的部分抽成独立 crate/module：

```text
keel-core/      ids、位置、错误、事件、取消
keel-vfs/       文件系统抽象、监听、编码
keel-text/      文档缓冲、版本、变更集
keel-workspace/ 项目快照、最近项目、会话
keel-syntax/    tree-sitter、outline、折叠、符号
keel-lsp/       LSP 进程、协议、文档同步、provider 路由
keel-index/     文件/符号索引、查询
keel-search/    文件内/全局搜索
keel-build/     Maven/Gradle/Cargo 外部进程
keel-run/       运行配置、进程、输出
keel-vcs/       Git 读写
keel-theme/     主题加载、token、编辑器 scheme
keel-i18n/      文案目录、格式化、回退
keel-ui/        GPUI 视图与工作台
```

规则：

- UI 只通过 command/event 调 core，不直接碰磁盘/进程。
- core 的公共类型集中在一个 `core` 模块。
- 每个领域只暴露最小 API，错误在边界转成统一 `KeelError`。

### 5.2 命令/动作系统

参考 OpenIDEA：

- 稳定 dotted id：`editor.save`、`navigate.searchEverywhere`、`project.open`。
- action 注册表同时驱动菜单、工具栏、快捷键、命令面板。
- keymap 与 action 分离，支持 IntelliJ / VS Code 预设。
- context keys + `when` 表达式控制可用性。
- 所有动作可被测试和插件引用。

### 5.3 代码智能路线

建议采用 **LSP 优先 + tree-sitter 回退 + 自有索引**：

```text
Java: jdtls（语义）+ tree-sitter（即时高亮/outline）+ 自有索引（文件/符号）
其他语言: 按需 LSP + tree-sitter
```

- LSP runtime 只在 Rust core 中实现一份。
- Provider 优先级和降级明确写死为策略，不要散落在 UI。
- Java 入口/Maven 构建/classpath/启动参考 Lithe 的边界。
- 索引在后台构建，查询回答已索引部分，用事件通知完成状态。

### 5.4 主题与 i18n 资源化

- 主题：`assets/themes/*.theme.json` + `assets/themes/*.scheme.json`，token 化；主题包只含数据。
- 编辑器配色独立 scheme，支持后续导入 IDEA `.icls` / `.theme.json`。
- i18n：`assets/locales/<locale>.json`，语义 key；默认 `zh-CN` + `en`，构建时校验 key/参数/回退。
- 禁止在组件里写颜色字面量和用户可见文案。

### 5.5 扩展点与插件

- 第一阶段做进程内注册表：工具窗口、状态栏、编辑器 provider、设置页、语言定义。
- 语言包先做声明式（TOML/JSON），不加载代码。
- 等核心稳定后再考虑 WASM/WIT 沙箱；不要提前上插件运行时。

### 5.6 远程与测试

- 保持 core 无 UI 依赖，未来可以像 OpenIDEA 一样做 `keel-server`。
- 测试按 OpenIDEA 的层次：unit → smoke（真实语言服务器）→ e2e → perf budget → eval goldens。
- UI 改动继续保留截图基线。

---

## 6. 结论

- **Lithe 证明**：Rust Core + LSP + Maven/Java 边界 + 模块运行时是可行的产品路线。
- **OpenIDEA 证明**：Rust crate 分层 + typed IPC + action/keymap + token 主题 + i18n 目录是可行的工程路线。
- **Keel 应该组合两者**：用 GPUI 保持轻量自绘，用 OpenIDEA 的 crate/IPC/资源化结构，用 Lithe 的 LSP/Java/Maven 运行时边界。

下一步最值得做的是把 `KeelApp` 拆出 `keel-core` 与 `keel-commands`，同时把主题和文案迁到 `assets/` 资源文件；这三件事会决定后续 LSP、索引、构建、插件能不能顺利接入。

# Keel 开发大纲

> 目标：用 **Rust + GPUI** 复刻一个 IntelliJ IDEA 风格的桌面 IDE，**优先支持 Java**。
> 本文档用于指导后续 AI 按阶段、按任务卡推进实现，避免一次性铺开大架构或反复推翻已有工作。
>
> 参考项目：
> - IDEA 源码：`D:\CodeWorkSpace\intellij-community`
> - 另一个 IDEA 复刻品：`D:\CodeWorkSpace\Lithe-IDEA`
>
> 最近更新：2026-11-11（对应提交 `b94bd3e` 之后的 UI 复刻阶段）

---

## 0. 这份文档怎么用（给后续 AI 的执行规则）

1. **一次只做一张任务卡**。每张任务卡都必须能独立验证（编译 + 截图 / 单测 / 手动操作路径）。
2. 开工前先读本文件的 **§2 技术约束** 和对应阶段的 **参考源码入口**，不要凭记忆写 GPUI API。
3. 不确定某个 API 是否存在时，必须明确写「**不确定**」，并给出需要确认的签名；**禁止编造**方法名、trait、模块路径。优先去本地源码核实：
   - gpui-ce：`C:\Users\laizi\.cargo\registry\src\mirrors.tuna.tsinghua.edu.cn-4dc01642fd091eda\gpui-ce-0.3.0`
   - 组件库：`C:\Users\laizi\.cargo\git\checkouts\gpui-component-574f787505f55df1\d248729`
4. 每完成一张任务卡：跑 **§8 验证清单**，更新 **§10 进度记录**，然后才允许开下一张。
5. 不顺手做「额外改进」。任务卡没写的重构、抽象、配置项，一律不做。
6. UI 改动必须附**截图验证**（见 §8.3），不接受「应该没问题」。

---

## 1. 产品定位与边界

**做什么**
- IDEA 的桌面交互骨架：菜单、工具窗口、编辑器标签、状态栏、对话框、弹层、主题。
- Java 项目的日常闭环：打开项目 → 浏览/编辑 → Maven/Gradle 构建 → 运行/调试 → 搜索导航 → Git。
- 低内存、按需启动外部进程（语言服务、构建工具、调试器）。

**暂不做什么**
- 插件市场、AI 助手、数据库工具、远程开发（Lithe 有，但我们后排）。
- 自研 Java 编译器 / 完整 PSI。语言智能走 **LSP（jdtls）**，自研只做轻量层。
- 全语言支持。Rust/TOML/JSON 只做高亮，Java 才做深度集成。

**与参考项目的分工**
- `intellij-community`：**交互与功能的真相来源**（布局、配色、术语、行为细节、构建模型）。
- `Lithe-IDEA`：**工程化与配置契约的参考**（按需 LSP、运行配置 schema、项目清单 schema、i18n 规范、AI 协作流程）。

---

## 2. 技术约束（硬性）

### 2.1 依赖固定

```toml
gpui = { package = "gpui-ce", version = "=0.3.0" }
gpui_platform = { package = "gpui_ce_platform", version = "=0.3.0" }
gpui_component = { package = "gpui_ce_components", git = "https://github.com/gpui-ce/gpui-component", rev = "d248729826757c96133ab723da3fee1cd2ed0ad6", features = ["tree-sitter-java"] }
gpui_component_assets = { package = "gpui_ce_components_assets", git = "https://github.com/gpui-ce/gpui-component", rev = "d248729826757c96133ab723da3fee1cd2ed0ad6" }
rfd = "0.15"
roxmltree = "0.20"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- **禁止混用** `gpui-kit`、`gpui-pre`、上游 `gpui` 的类型。
- gpui-ce 只能用 crates.io 的 `0.3.0`；GitHub main 的版本号与组件库要求不一致。
- 升级依赖是一张独立任务卡，不允许夹在功能任务里。

### 2.2 环境与构建

- Windows / PowerShell，工作目录 `D:\CodeWorkSpace\keel`。
- **所有 cargo 命令必须设** `$env:CARGO_HOME="C:\Users\laizi\.cargo"`，否则沙箱下写 registry 缓存会被拒。
- **同一时间只跑一个 cargo 命令**；并发会互相等锁，曾因此卡死十几分钟。
- 构建较慢时改用后台运行，不要反复重试。

### 2.3 代码约定

- 注释与界面文案用中文；命名用英文。
- 颜色**只能**来自 `idea_theme.rs` 的令牌（取自 IDEA 官方主题 JSON），禁止随手写色值。
- 字体渲染固定 `TextRenderingMode::Grayscale`（解决 ClearType 彩色光晕），等设置页做完再改为可配置。
- 图标用 `gpui_component_assets::AllAssets`；需要语言着色时走 `file_icons.rs`。
- Git：`main` 分支 + `origin = https://github.com/ColdMistyRain/keel.git`，源码统一 LF（`.gitattributes`）。

---

## 3. 现状盘点

| 模块 | 文件 | 已有能力 |
| --- | --- | --- |
| 应用外壳 | `src/main.rs` | 窗口创建、资源加载、快捷键、**文件/视图菜单栏**、主题应用 |
| 根视图与命令 | `src/app.rs` | 动作分发、文档生命周期、状态栏分区（行列号/换行/编码/缩进/语言）、菜单栏渲染 |
| 编辑器 | `src/editor.rs` | 标签页（文件图标、未保存圆点、关闭确认）、行号、缩进参考线、4 空格缩进 |
| 项目树 | `src/project_tree.rs` | 目录展开/折叠、选中、双击打开、文件类型图标 |
| 工具窗口条 | `src/tool_window.rs` | 左侧 40px 竖条，Project/Search/Git/Maven 占位 + 设置 |
| 底部面板 | `src/panels.rs` | Problems/Run/Terminal 三个占位页 + 命令面板 |
| 主题 | `src/idea_theme.rs` | IDEA Light 令牌、RGB→HSLA 转换、主题覆盖 |
| 项目检测 | `src/project.rs` | `pom.xml` / `settings.gradle` 解析（**未接入 UI**，用户暂缓） |
| 模型 | `src/model.rs` | 目录树、过滤、排序、语言识别、持久化 |

**已明确的缺口**：编辑/导航/代码/重构/运行等菜单、右键菜单、工具窗口标题栏动作、导航栏面包屑、查找替换、代码折叠、Darcula、设置页、构建运行、语言智能、Git。

---

## 4. 参考资源地图

### 4.1 intellij-community（读这些就够起步）

| 主题 | 入口路径 | 用途 |
| --- | --- | --- |
| 配色令牌 | `platform/platform-resources/src/themes/intellijlaf.theme.json`、`Light.theme.json`、`darcula.theme.json` | 颜色的**唯一**来源 |
| 图标资源 | `platform/icons/src/`、`community-resources/src/` | 文件类型/工具窗口图标造型参考（注意版权，见 §9） |
| 工具窗口框架 | `platform/platform-impl/src/com/intellij/openapi/wm/impl/` | 工具窗口的标题栏、停靠、隐藏行为 |
| 编辑器标签 | `platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/EditorTabbedContainer*` | 标签溢出、多行、右键菜单 |
| 编辑器核心 | `platform/editor-ui-api/`、`platform/platform-impl/src/com/intellij/openapi/editor/` | 行号槽、折叠、软换行、滚动条标记 |
| 动作系统 | `platform/editor-ui-api/src/com/intellij/openapi/actionSystem/` | Action/ActionGroup/快捷键语义 |
| 项目模型 | `platform/core-api/src/com/intellij/openapi/project/`、`platform/projectModel-api/` | Project/Module/ContentRoot/Library 概念 |
| 项目导入 | `java/idea-ui/src/com/intellij/projectImport/ProjectImportProvider.java` | 导入向导抽象 |
| Maven | `plugins/maven/src/main/java/org/jetbrains/idea/maven/` | POM 模型、依赖解析、导入流程 |
| Gradle 模型导入 | `plugins/gradle/tooling-extension-api/src/org/jetbrains/plugins/gradle/model/ProjectImportModelProvider.java` | 外部构建系统 → IDE 模型的扩展点 |
| Gradle 导入向导 | `plugins/gradle/java/src/service/project/wizard/GradleProjectImportProvider.java` | 导入流程参考 |
| 运行配置 | `java/execution/impl/src/com/intellij/execution/JavaRunConfigurationBase.java` | RunConfiguration 字段与执行语义 |

### 4.2 Lithe-IDEA（借工程，不抄实现）

| 路径 | 借鉴点 |
| --- | --- |
| `shared/contracts/run-configuration-v2.schema.json` | 运行配置的字段设计，可直接转成我们的 serde 结构 |
| `shared/contracts/project-manifest-v1.schema.json` | 项目清单（模块、源码根、依赖）契约 |
| `shared/contracts/maven-launch-context-v1.schema.json`、`maven-portable-configuration-v1.schema.json` | Maven 启动上下文与可移植配置 |
| `shared/contracts/toolchain-local-v1.schema.json` | JDK/构建工具探测结果的数据结构 |
| `shared/contracts/editor-syntax-theme-v1.schema.json` | 语法高亮主题的版本化方式 |
| `AGENTS.md` | 强制 i18n、AI 协作流程、Skill 化开发约束 |
| `rust/lithe-core`、`rust/lithe-ide-host` | Rust 侧核心与 IDE 宿主进程的职责切分 |
| `docs/architecture/`、`docs/development/` | 架构决策记录与视觉验收流程 |

**原则**：Lithe 是 Swift/Tauri 技术栈，**只借数据契约与工程规范**，不借 UI 实现。

---

## 5. 目标架构与目录演进

保持**单 crate**，按职责拆模块；只有当某块需要独立测试或被外部进程复用时才拆 crate。

```
src/
  main.rs            入口、菜单、快捷键、主题
  app.rs             根视图、命令分发、状态栏
  idea_theme.rs      配色令牌（Light/Darcula）
  file_icons.rs      文件图标与着色
  editor/            编辑器标签、行号槽、查找替换、折叠
  project/           项目模型、Maven/Gradle 检测与导入
  tool_window/       工具窗口框架（标题栏、停靠、显示状态）
  panels/            底部面板、弹层（命令面板、Search Everywhere）
  build/             外部进程执行、输出解析、运行配置
  lang/              LSP 客户端、符号索引、诊断
  vcs/               Git 状态、diff、提交
  settings/          设置模型、持久化、设置页
```

**分层规则**：UI 模块只读写应用状态；外部 IO（文件、进程、网络、LSP）一律在后台任务里做，结果通过 `update` 回到 UI。

---

## 6. 分阶段路线图

> 顺序遵循用户已定的优先级：**先把 IDEA UI 复刻完，再接 Maven/Gradle**，然后才是语言智能与调试。

### Phase 1 — IDEA UI 复刻收尾（当前阶段）

目标：不看功能，单看截图能认出「这是 IDEA」。

| 编号 | 任务 | 验收 |
| --- | --- | --- |
| UI-01 | 工具窗口标题栏动作：折叠全部、刷新、齿轮、隐藏到侧边条 | 点击后树状态/可见性真实变化，截图对比 |
| UI-02 | 项目面板与编辑器区可拖拽调宽 | 拖动分隔条改变宽度并持久化 |
| UI-03 | 编辑器标签右键菜单（关闭、关闭其他、关闭全部、在项目中定位） | 菜单项全部可用 |
| UI-04 | 标签溢出处理（横向滚动或下拉列表）与拖拽排序 | 开 20 个文件不挤爆，可排序 |
| UI-05 | 顶部菜单补齐：编辑/导航/代码/重构/运行/工具/窗口/帮助 | 每项要么有真实动作，要么不列；禁用项不给假入口 |
| UI-06 | 编辑器与项目树右键上下文菜单（新建/重命名/删除/复制路径） | 文件操作真实生效并刷新树 |
| UI-07 | 导航栏面包屑（可选显示，默认关） | 路径与当前文件一致，可点击跳转选中 |
| UI-08 | Darcula 深色主题 + 主题切换入口 | 两套主题下截图无色值错误，令牌仍来自 IDEA JSON |
| UI-09 | 通知气泡（右下角）与状态栏后台任务进度 | 保存/构建等动作有可见反馈 |
| UI-10 | 关于对话框与设置页骨架 | 设置页至少能切换主题、文本渲染模式 |

**DoD**：全部截图归档到 `docs/screenshots/`（若需要），§8 全绿。

### Phase 2 — 编辑器核心

| 编号 | 任务 | 验收 |
| --- | --- | --- |
| ED-01 | 文件内查找/替换条（Ctrl+F / Ctrl+R） | 匹配高亮、上一个/下一个、替换生效 |
| ED-02 | 代码折叠（按语法块）与折叠全部/展开全部 | Java 类/方法可折叠 |
| ED-03 | 行号槽：当前行高亮、折叠箭头、断点占位 | 视觉与 IDEA 一致 |
| ED-04 | 撤销/重做、自动缩进、括号匹配、注释切换 | 快捷键与菜单一致 |
| ED-05 | 大文件保护与只读文件处理 | >8MB 有明确提示，不写坏文件 |
| ED-06 | 外部修改检测与重载 | 外部改文件后给出重载提示 |

### Phase 3 — 项目模型与工作区

| 编号 | 任务 | 验收 |
| --- | --- | --- |
| PM-01 | 统一 `ProjectModel`（模块、内容根、源码根、依赖）并替换树的数据源 | 参考 Lithe `project-manifest-v1.schema.json` |
| PM-02 | 最近项目列表与欢迎页 | 启动可选择最近项目 |
| PM-03 | 项目内新建/删除模块、源码根标记 | 项目树按源码根着色 |
| PM-04 | 工作区状态持久化（打开的文件、光标、展开节点） | 重启后现场恢复 |

### Phase 4 — Maven 接入（用户点名的下一件大事）

| 编号 | 任务 | 验收 |
| --- | --- | --- |
| MV-01 | 工具链探测：JAVA_HOME、M2_HOME、mvn -version | 设置页可见，参考 Lithe `toolchain-local-v1.schema.json` |
| MV-02 | POM 解析完整化：parent、properties、dependencyManagement、profiles | 已有 `project.rs` 扩展，单测覆盖 |
| MV-03 | 通过 `mvn -q -DforceStdout help:effective-pom` / `dependency:build-classpath` 拿到真实模型与 classpath | 外部进程后台执行，输出落盘可诊断 |
| MV-04 | 模块树按 Maven 模块展示，依赖节点（外部库） | 项目树出现「External Libraries」 |
| MV-05 | Maven 工具窗口：生命周期与插件目标，双击执行 | 输出进 Run 面板，退出码可见 |
| MV-06 | 源码根与编译级别进入项目模型，供 LSP 使用 | jdtls 能拿到正确 classpath |

### Phase 5 — Gradle 接入

| 编号 | 任务 | 验收 |
| --- | --- | --- |
| GR-01 | 探测 gradlew / gradle，识别 Groovy 与 KTS | 单测 |
| GR-02 | 用 `gradlew projects` / `tasks` / `dependencies` 或 Tooling API 桥接进程导入模型 | 与 IDEA `ProjectImportModelProvider` 对齐字段 |
| GR-03 | Gradle 工具窗口（任务树、执行、输出） | 同 MV-05 |
| GR-04 | 多项目构建的模块映射 | 示例项目导入结果正确 |

**决策点**：优先 **命令行 + 结构化输出**；只有必须时才引入 JVM 桥接进程跑 Tooling API（见 §9）。

### Phase 6 — Java 语言智能（LSP 优先）

| 编号 | 任务 | 验收 |
| --- | --- | --- |
| LG-01 | jdtls 探测、下载/配置、按项目启动 | 进程按需启动，可关闭 |
| LG-02 | LSP 客户端：initialize、didOpen/didChange/didSave、shutdown | 日志可诊断 |
| LG-03 | 诊断 → Problems 面板 + 编辑器波浪线 | 改错一个分号能立刻看到 |
| LG-04 | 补全、悬浮文档、跳转定义、查找引用 | 常用快捷键与 IDEA 一致 |
| LG-05 | 无 LSP 时的轻量补全（当前文件符号） | 参考 Lithe「无需 LSP 的轻量补全」策略 |
| LG-06 | 格式化与 import 整理 | 可配置是否委托 LSP |

### Phase 7 — 运行与调试

| 编号 | 任务 | 验收 |
| --- | --- | --- |
| RN-01 | RunConfiguration 模型与编辑器（主类、模块、VM 参数、环境变量） | 字段参考 Lithe `run-configuration-v2.schema.json` |
| RN-02 | 运行 Java 主类 / Maven 目标，输出进 Run 面板 | 进程可停止，退出码正确 |
| RN-03 | 自动识别可运行入口（含 Spring Boot） | 参考 Lithe 的入口识别清单 |
| RN-04 | JDWP 调试：附加/启动、断点、单步、变量 | 最小闭环：打断点能看到变量 |
| RN-05 | 调试面板（Frames/Variables/Console） | UI 与 IDEA 布局一致 |

### Phase 8 — 搜索与导航

| 编号 | 任务 | 验收 |
| --- | --- | --- |
| SE-01 | Find in Files（Ctrl+Shift+F）：ripgrep 后台搜索 + 结果树 | 大项目不卡 UI |
| SE-02 | Search Everywhere（双击 Shift）：文件/符号/动作混合 | 排序合理 |
| SE-03 | Recent Files / Switcher | 快捷键切换 |
| SE-04 | 文件级符号索引（tree-sitter 提取）→ 导航栏与结构面板 | Java 类/方法可跳转 |

### Phase 9 — Git

| 编号 | 任务 | 验收 |
| --- | --- | --- |
| VC-01 | git status → 项目树着色 + Commit 面板 | 新增/修改/删除标识正确 |
| VC-02 | Diff 视图（并排/统一） | 与 IDEA 布局一致 |
| VC-03 | 提交、拉取、推送、分支切换 | 走 git CLI，输出可见 |
| VC-04 | 本地历史快照 | 可回滚到任意快照 |

### Phase 10 — 设置、打包与发布

设置页完善（主题、字体、按键映射、文本渲染、语言服务开关）、单测覆盖核心模型、Windows 安装包、更新机制、性能基线（启动时间、打开大项目耗时、常驻内存）。

---

## 7. 任务卡模板（复制使用）

```
任务编号：UI-03
目标：一句话说明用户能看到什么变化
前置：依赖哪些已完成任务
改动文件：src/editor.rs、src/app.rs
实现要点：3-5 条，含需要核实的 API
参考：intellij-community 或 Lithe 的具体路径
验收：操作路径 + 预期结果（必须可观察）
验证：cargo build / cargo test / cargo fmt --check / cargo clippy --no-deps / 截图
风险与不确定项：明确写出不确定的 API 或行为
```

---

## 8. 验证清单

### 8.1 命令

```powershell
$env:CARGO_HOME="C:\Users\laizi\.cargo"
cargo build
cargo test
cargo fmt --check
cargo clippy --no-deps
```

### 8.2 提交

任务卡完成后才提交；提交信息写「做了什么 + 为什么」，不要一次塞进多张卡。

### 8.3 截图验证（UI 必做）

当前 Windows 会话可能锁屏，合成鼠标事件不一定可达，所以用**临时自动打开文件 + PrintWindow 截图 + 恢复**：

1. 在 `KeelApp::new` 末尾临时加：`KEEL_DEMO_FILE` 环境变量存在时同步读文件并 `insert_document`。
2. `Start-Process` 启动 exe，等主窗口出现后 `SetWindowPos` 固定位置，`PrintWindow(..., 2)` 存 PNG。
3. 看完图**必须删掉临时代码**并重新构建。
4. 菜单展开类交互可补一次 `SetCursorPos + mouse_event` 点击再截图；点不到就改查组件源码确认行为。

---

## 9. 关键技术决策与风险

| 议题 | 决策 | 原因 |
| --- | --- | --- |
| Java 语义 | **LSP（jdtls）**，不自研 PSI | 自研成本是数量级差异；Lithe 也是按需 LSP |
| Maven/Gradle 模型 | 先**外部进程 + 结构化输出**，再考虑 Tooling API 桥 | 避免过早引入 JVM 常驻进程 |
| 调试 | JDWP，先「启动时附加」再「本地启动」 | 实现面最小 |
| 索引 | tree-sitter 提取符号 + ripgrep 搜索 | 满足导航与搜索，避免自建全量索引 |
| 图标 | 现阶段用 Lucide + 语言着色 | IDEA 官方图标有版权约束，正式复刻需评估许可 |
| 大项目性能 | 所有 IO 后台化、面板虚拟滚动、按需解析 | GPUI 主线程不能被树/搜索阻塞 |
| 配置格式 | 优先复用 Lithe 的 JSON Schema 思路 | 避免另起一套不兼容的运行/项目配置 |

**待确认（写到任务卡时再落实）**
- jdtls 的下载、启动参数与项目级隔离方式。
- Gradle Tooling API 是否必须引入桥接 JVM。
- GPUI 侧是否已有 Dock/分栏组件可替代自研工具窗口框架。

---

## 10. 进度记录

- [x] 依赖固定（gpui-ce 0.3.0 + gpui_ce_components）与仓库初始化
- [x] IDEA Light 主题令牌、灰度文本渲染
- [x] 项目树、编辑器标签、底部面板、命令面板、左侧工具窗口条
- [x] 状态栏分区（行列号/换行/编码/缩进/语言）
- [x] 文件/视图菜单栏（真实 Action + 快捷键）
- [x] 文件类型图标与未保存圆点
- [x] Phase 1 · UI-01：项目面板标题栏动作（折叠全部 / 从磁盘刷新 / 齿轮菜单 / 隐藏到侧边条）
- [x] Phase 1 · UI-02：项目面板与编辑器区拖拽调宽，宽度写入 `state.json` 并在重启后恢复
- [x] Phase 1 · UI-03：编辑器标签右键菜单（关闭 / 关闭其他 / 关闭全部 / 在项目中定位），右键同时激活标签
- [x] Phase 1 · UI-04：标签溢出（横向滚动 + 已打开文件下拉列表，激活标签自动滚入视野）与拖拽排序
- [x] Phase 1 · UI-05：顶部菜单补齐（文件/编辑/视图/导航/工具/窗口，仅保留有真实动作的项；代码/重构/运行/帮助待对应功能落地后再加入）
- [x] Phase 1 · UI-06：项目树右键菜单（新建文件/新建文件夹/重命名/删除/复制路径/在编辑器中打开）与编辑器标签「复制路径」，文件操作真实生效并刷新树
- [x] Phase 1 · UI-07：导航栏面包屑（按项目根拆段，点击任一段在项目树中定位选中；与 IDEA 新 UI 一致，位置在编辑器底部、默认显示，视图菜单可切换并持久化）
- [x] Phase 1 · UI-08：深色主题 + 视图菜单切换入口（启动时先读配置再应用主题，令牌来自 expUI_dark.theme.json）
- [x] Phase 1 · UI-09：右下角通知气泡（保存/刷新/文件操作成功后提示，点击或 4 秒后消失，最多 4 条）与状态栏后台任务进度提示
- [x] Phase 1 · UI-10：设置页骨架（外观页可切换主题与文本渲染方式并写入 state.json，关于页展示版本/阶段/配置路径）与「设置…」「关于 Keel…」菜单入口（Ctrl+Alt+S 开关、Esc 关闭）
- [x] Phase 2 · ED-01：文件内查找/替换条（Ctrl+F / Ctrl+R，编辑器自带面板：匹配高亮、上一个/下一个、匹配计数、替换与全部替换、Esc 关闭；焦点归面板管理，应用级快捷键在项目树聚焦时同样可用）
- [x] Phase 1 · UI-11：对照 IDEA 2025.3 新 UI（expUI）做外观对齐——移除多余工具按钮行；标签栏压到 28px、选中标签靠顶部蓝线区分；面包屑移到底部；底部工具窗口默认隐藏；状态栏改为「分支 + 行列/换行/编码/缩进/设置」；令牌全部换成 expUI 的 Gray/Blue 调色板；工具窗口侧边条与面板标题使用 expUI SVG 图标；UI 字体嵌入 Inter 13px，编辑器改为 JetBrains Mono 13px
- [x] Phase 1 · UI-12：对照社区版 2026 默认的 Islands 主题重做外观——令牌换成 ManyIslandsLight/Dark（画布 gray-150、岛屿白、选中 blue-140/blue-50、标签 blue-150 底 + 4px blue-120 上边线、悬停为半透明黑/白）；自绘 40px 标题栏（汉堡菜单、项目徽标、分支、搜索/设置、窗口按钮，菜单收进汉堡）；编辑器与项目面板改为 20px 圆角岛屿；工具窗口条 40×40、激活蓝底白图标，底部追加问题/运行/终端入口；状态栏只留行列/换行/编码/缩进；标签关闭改用 SVG、未保存改为圆点；嵌入 JetBrains Mono、行高 1.2，并按 expUI light/dark scheme 设置编辑器语法色与当前行高亮
- [x] Phase 1 · UI-13：对照 IDEA 2026 的 COMPACT 密度继续对齐细节——岛屿圆角 16/间距 4、工具条 44px；项目面板标题改为「项目 ⌄」+ 新建（真实新建文件/文件夹）+ 分隔线 + 折叠/刷新/更多/隐藏；树根行显示完整路径并默认选中，缩进改为 12+20/级；状态栏左侧补项目选择器；底部工具窗口条按原版顺序放 Profiler/Services/终端/问题/Git；选中标签改为 1px 蓝边圆角卡片、高 24；标题栏补产品图标、外链按钮与项目色渐变；编辑器字号 15px、行高 1.44
- [x] Phase 1 · UI-14：补齐编辑器缩进参考线（`INDENT_GUIDE` 取色，顺带修正 `theme.border`）与 expUI 文件类型图标——文件树、编辑器标签、面包屑统一使用 java/xml/json/yaml/toml/markdown/js/ts/html/css/shell/text 及 folder 的浅/深两套图标
- [x] Phase 1 · UI-15：按 Islands 源码补齐遗漏令牌与度量——补 `button_primary*`（Primary 按钮不再变黑）、`selection`、`caret`、`popover`，圆角改 6/8；设置浮层补焦点与 Esc 关闭；侧边条 37px、按钮 29×32（`StripeToolbar.Button.size`）；项目面板标题 41、树行 22（`ToolWindow.Header.height`、`Tree.rowHeight.compact`）；选中标签圆角 4（`EditorTabs.underlineArc`）；状态栏部件内边距 3/4/1/4；空编辑器改为 IDEA 的六行快捷键提示（`DefaultEditorEmptyTextProvider`）
- [x] Phase 1 · UI-16：继续按源码对齐——面包屑改为状态栏左侧部件（IDEA 的 `NAVBAR_WIDGET_KEY`），状态栏 24px；标签行 32、卡片 28、水平内边距 6/4、标签条下补 1px 分隔线；树选中改为满宽直角；面板标题字号 13；侧边条补分组分隔线（`toolbar-border`）；项目徽标与状态栏项目点改用项目色（`RecentProjectColorPalette`），窗口按钮宽 40（`TitlePane.Button.preferredSize.compact`）；面板与编辑器间距 4（`Island.borderWidth.compact`）；空编辑器文字改用 `Label.infoForeground`（text-secondary）；深色悬停改 `#FFFFFF10`
- [x] Phase 1 · UI-17：按配色方案补齐编辑区细节——编辑器文本选中色改为 `SELECTION_BACKGROUND`（浅色 `#A6D2FF`、深色 Darcula `#214283`，与列表选中的 blue-140/blue-50 区分）；深色当前行高亮改 `CARET_ROW_COLOR` `#1F2024`、缩进参考线 `#323438`；项目树根节点名称加粗；通知气泡改用 `feedback-bg` + 阴影（`Notification` 规范）
- [ ] Phase 2：ED-02 ~ ED-06
- [ ] Phase 3：项目模型
- [ ] Phase 4：Maven
- [ ] Phase 5：Gradle
- [ ] Phase 6：语言智能
- [ ] Phase 7：运行调试
- [ ] Phase 8：搜索导航
- [ ] Phase 9：Git
- [ ] Phase 10：设置与发布

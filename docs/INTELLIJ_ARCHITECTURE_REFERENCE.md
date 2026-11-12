# IntelliJ IDEA 社区版架构参考

> 参考源码：`D:\Programming\Workspace\intellij-community`
> 当前 checkout：`b342fc868e`（`PY-91480 save relative macro paths in all run configurations`）
> 用途：给 Keel 后续开发提供“IDEA 是怎么组织的”索引，不追求逐类逐方法说明。
> 约定：下文路径都相对于 `<IDEA> = D:\Programming\Workspace\intellij-community`。

---

## 0. 先看结论

IDEA 的架构可以概括为六层：

```text
产品/插件层        idea/、plugins/、java/
        ↓
IDE 功能层         ide-core/、project-frame/、projectView/、searchEverywhere/
        ↓
平台实现层         platform-impl/、platform-resources/、workspace/、indexing-impl/
        ↓
平台 API 层        core-api/、platform-api/、editor-ui-api/、analysis-api/、indexing-api/
        ↓
基础机制层         service-container/、extensions/、util/、util-io/、progress/
        ↓
JVM/基础设施       JBR、VFS、文件系统、SQLite、网络、远程开发
```

贯穿始终的五个机制：

1. **Service Container**：应用/项目/模块三级服务容器，构造注入，按需创建。
2. **Extension Point**：`plugin.xml` 声明扩展点，插件注册实现，平台聚合调用。
3. **依赖方向约束**：`*-api` 只放接口和稳定契约，`*-impl` 放实现；`@ApiStatus.Internal` 不保证兼容。
4. **线程模型**：EDT + 后台线程 + Read/Write Intent/Write 锁 + 可取消的读操作。
5. **数据模型分层**：`VirtualFile → Document → PSI → Index`，UI 和功能都从这四层取数据。

---

## 1. 仓库全景

| 目录 | 职责 |
| --- | --- |
| `platform/` | 平台本体：服务容器、扩展点、VFS、编辑器、PSI 基础设施、UI、索引、执行、设置、主题等 |
| `java/` | Java 语言插件：Java PSI、分析、补全、重构、编译、调试、运行 |
| `jps/` | JetBrains Project System：独立构建模型与编译器，IDEA 的外部构建基础 |
| `plugins/` | 随 IDE 分发的插件（Git、Maven、Gradle、Kotlin 等） |
| `json/`、`RegExpSupport/` 等 | 独立语言/格式支持模块 |
| `idea/` | IDE 产品层（社区版）的模块与资源 |
| `community-resources/`、`bin/`、`build/` | 资源、启动脚本、构建脚本 |
| `docs/` | 架构图、Configurable、Plugin Model 等设计文档 |
| `.agents/`、`.ai/` | 仓库约定、代码风格、构建/测试入口 |

### 1.1 构建模型

- `.iml` 是模块定义的 source of truth，`BUILD.bazel` 由 `.iml` 生成。
- 修改模块或依赖后，需要先注册 `.iml`，再运行 `build/jpsModelToBazelCommunityOnly.cmd`（见 `.ai/guide.md`）。
- 不要手工编辑 `.idea/modules.xml`。
- 依赖关系写在模块自己的描述里；模块之间不能随意访问，除非显式声明依赖。

### 1.2 模块命名规律

- `*-api`：公开接口、扩展点、数据模型，稳定契约。
- `*-impl`：实现、内部工具，可能随版本变化。
- `*-ui-api` / `*-ui` / `*-ui-impl`：UI 抽象与 Swing/Jewel 实现。
- `*-tests` / `*-testFramework`：测试夹具与测试工具。
- `backend/`、`frontend/`、`shared/`：拆分前后端或远程开发场景时的代码归属。

---

## 2. 启动流程

关键入口：

- `platform/bootstrap/src/com/intellij/idea/Main.kt`
- `platform/ide-core-impl` / `platform/starter` 中的 `AppStarter`、`startApplication`
- `platform/platform-impl/src/com/intellij/openapi/application/impl/ApplicationImpl.java`
- `platform/platform-impl/src/com/intellij/openapi/project/impl/ProjectImpl.kt`

大致顺序：

1. `Main.kt` 解析命令行、初始化 `PathManager`、日志、JBR、类加载器。
2. 启动 `ApplicationImpl`，创建应用级 Service Container。
3. 读取 `plugin.xml`，加载插件与扩展点。
4. 初始化 LAF、主题、字体、动作系统、VFS、索引服务。
5. `ProjectManager` 打开项目，创建 `ProjectImpl`（项目级 Service Container）。
6. 加载项目配置（`.idea` / `.ipr`）、Workspace Model、模块与库。
7. 启动 `StartupActivity` / `ProjectActivity`，构建主窗口 `IdeFrameImpl`、编辑器、工具窗口、状态栏。
8. 进入 Smart Mode；在此之前索引未就绪，属于 Dumb Mode。

启动性能由 `StartUpMeasurer`、`Activity`、`LoadingState` 记录；新代码通常要求“启动阶段可延迟、可并行”。

---

## 3. 服务容器与扩展点

### 3.1 Service Container

核心类：

- `platform/service-container/src/com/intellij/serviceContainer/ComponentManagerImpl.kt`
- `platform/extensions/src/com/intellij/openapi/components/ComponentManager.java`
- `platform/core-api/src/com/intellij/openapi/components/Service.java`
- `platform/core-api/src/com/intellij/openapi/components/ServiceDescriptor.java`

要点：

- `Application`、`Project`、`Module` 都实现 `ComponentManager`，各自带一个服务容器。
- 服务分应用级、项目级、模块级；`@Service(Level.APP)` / `@Service(Level.PROJECT)` 可声明轻量服务。
- 服务默认懒加载，第一次 `service<T>()` 时创建；构造器可注入 `Application`、`Project`、`CoroutineScope` 等。
- 旧 `Component` 已废弃，新代码应使用 Service。
- 生命周期由 `Disposable` 树管理：项目关闭时，项目级服务自动释放。

### 3.2 Extension Point

核心类与声明：

- `platform/extensions/src/com/intellij/openapi/extensions/ExtensionPoint.java`
- `platform/core-api/resources/intellij.platform.core.xml`
- `platform/core-api/src/com/intellij/openapi/components/ServiceDescriptor.java`

示例：

```xml
<extensionPoint name="applicationService"
                beanClass="com.intellij.openapi.components.ServiceDescriptor"
                dynamic="true"/>
<extensionPoint name="projectService"
                beanClass="com.intellij.openapi.components.ServiceDescriptor"
                dynamic="true"/>
```

使用方式：

- 平台/插件在 `plugin.xml` 里声明 `<extensionPoint>`。
- 其他插件用 `<extensions defaultExtensionNs="...">` 注册实现。
- 代码通过 `ExtensionPointName.create(...)` 获取列表，通常配合 `ProjectExtensionPointName`。
- `dynamic="true"` 表示可以在运行时增删扩展。
- 扩展点可以有 `area="IDEA_PROJECT"`，限定项目级作用域。

### 3.3 插件模型

- `PluginManagerCore`：`platform/core-impl/src/com/intellij/ide/plugins/PluginManagerCore.kt`
- 插件 = 一组模块；每个模块不能访问兄弟模块的类，除非声明依赖。
- `plugin.xml` 是插件入口，包含 `<depends>`、`<extensions>`、`<actions>`、`<applicationService>` 等。
- 插件有自己的 `PluginAwareClassLoader`，平台通过它隔离类。
- 文档：`docs/plugin.md`、`docs/plugin-model.puml`。

### 3.4 消息总线与生命周期

- `MessageBus` / `Topic`：应用级、项目级发布订阅；UI 刷新、VFS 变更、设置变更都走它。
- `Disposable` / `Disposer`：所有需要清理的对象注册到父对象；项目关闭、窗口关闭、插件卸载时按树释放。
- 新增监听器时优先用 `messageBus.connect(disposable)`，避免泄漏。

---

## 4. 线程模型与并发

核心文档与 API：

- `platform/core-api/src/com/intellij/openapi/application/Application.java`（类注释有锁兼容矩阵）
- `platform/core-api/src/com/intellij/openapi/application/ReadAction.java`
- `platform/core-api/src/com/intellij/openapi/application/NonBlockingReadAction.java`
- `platform/core-api/src/com/intellij/openapi/progress/ProgressManager.java`
- `platform/core-api/src/com/intellij/openapi/project/DumbService.kt`

要点：

- **EDT**：Swing UI 线程，所有 UI 修改必须在 EDT。
- **Read Action**：并发读 PSI/VFS/索引，可多线程同时持有。
- **Write Intent**：准备写之前持有，保证同一时刻只有一个写意图。
- **Write Action**：真正修改文档/PSI/模型；等待所有读操作结束。
- **后台任务**：`Task.Backgroundable`、`ProgressManager`、`AppExecutorUtil`。
- **可取消**：长时间读操作必须周期性 `ProgressManager.checkCanceled()`；`NonBlockingReadAction` 在写操作到来时会被取消并重试。
- **协程**：新代码优先使用 `readAction {}`、`smartReadAction {}`、`writeIntentReadAction {}`、项目级 `CoroutineScope`。
- **Dumb Mode**：索引构建期间只允许 `DumbAware` 功能；通过 `DumbService.runReadActionInSmartMode` 等待智能模式。

对 Keel 的启示：

- UI 线程只做绘制与轻量状态更新。
- 文件/索引/构建全部后台化，并提供取消令牌。
- 读操作要允许被写操作打断和重试，避免长任务阻塞编辑。

---

## 5. 数据模型：VFS → Document → PSI → Index

### 5.1 VFS

API：

- `platform/core-api/src/com/intellij/openapi/vfs/VirtualFile.java`
- `platform/core-api/src/com/intellij/openapi/vfs/VirtualFileManager.java`
- `platform/vfs-impl/`

要点：

- `VirtualFile` 是文件/目录的抽象，不直接暴露 `java.io.File`。
- `VirtualFileSystem` 按协议注册（`file://`、`jar://`、`temp://` 等）。
- 文件变更通过 `VirtualFileManager.VFS_CHANGES` 广播；支持同步/异步监听。
- `LocalFileSystem` 负责本地磁盘，`VirtualFile.refresh` 触发重新扫描。
- 所有文件访问都走 VFS，便于支持远程开发、内存文件、归档文件。

### 5.2 Document 与 Editor

API：

- `platform/core-api/src/com/intellij/openapi/editor/Document.java`
- `platform/editor-ui-api/src/com/intellij/openapi/editor/Editor.java`
- `platform/editor-ui-api/src/com/intellij/openapi/editor/EditorFactory.java`
- `platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/text/`

要点：

- `Document` 是内存中的文本模型；换行统一为 `\n`，保存时再转目标格式。
- `Editor` 是文档的视图，负责光标、选区、滚动、折叠、高亮等。
- `FileDocumentManager` 负责 `VirtualFile ↔ Document`。
- `Document` 修改后，`PsiDocumentManager` 负责把文档同步到 PSI。
- 编辑器 UI 由 `EditorComponentImpl`、`EditorEx`、`EditorImpl` 等实现。

### 5.3 PSI

API：

- `platform/core-api/src/com/intellij/psi/PsiFile.java`
- `platform/core-api/src/com/intellij/psi/PsiElement.java`
- `java/java-psi-api/src/com/intellij/psi/JavaPsiFacade.java`
- `java/java-psi-impl/`

要点：

- PSI（Program Structure Interface）是语言的结构化模型：文件、类、方法、表达式、注释等。
- `PsiFile` 对应一个文件；`FileViewProvider` 支持一个文件多种语言视图（如 HTML 内嵌 JS）。
- Java PSI 通过 `JavaPsiFacade` 查类、包、模块；`PsiManager` 负责文件到 PSI 的缓存。
- PSI 解析结果可以存成 Stub，供索引和快速导航使用。
- 修改 PSI 必须用 `WriteAction`，并通过 `PsiDocumentManager` 提交到 Document。

### 5.4 数据流

```text
磁盘文件
  → VirtualFile（VFS）
  → Document（文本缓冲）
  → PsiFile / PsiElement（语法结构）
  → Stub / FileBasedIndex（持久索引）
  → 搜索、导航、补全、检查、重构
```

这条链路是 IDEA 所有语言功能的基础。Keel 目前只有 `Document` 级的 editor，没有 VFS/PSI/索引分层，这是后续 Java 支持最大的结构差距。

---

## 6. 项目模型与 Workspace Model

### 6.1 传统项目模型

API：

- `platform/core-api/src/com/intellij/openapi/project/Project.java`
- `platform/projectModel-api/src/com/intellij/openapi/module/`
- `platform/projectModel-api/src/com/intellij/openapi/roots/`
- `platform/projectModel-api/src/com/intellij/openapi/components/PersistentStateComponent.java`

概念：

- `Project`：一个打开的工程，包含模块、库、VCS 根、运行配置等。
- `Module`：编译/依赖单元，持有 content roots、source roots、依赖、SDK。
- `ModuleRootManager`、`ProjectFileIndex`：查询文件属于哪个模块/源根。
- `.idea/` 或 `.ipr`：项目配置存储位置。

### 6.2 Workspace Model

新模型入口：

- `platform/backend/workspace/src/WorkspaceModel.kt`
- `platform/workspace/storage/`
- `platform/projectModel-impl/src/com/intellij/workspaceModel/ide/impl/WorkspaceModelImpl.kt`

要点：

- Workspace Model 用不可变快照 `ImmutableEntityStorage` 描述项目结构。
- 通过 `MutableEntityStorage` 做增量修改，再提交为新快照。
- `currentSnapshot` 可以无锁读取；修改通过 `updateProjectModel` 或异步 `update`。
- 变更通过 `eventLog` / `VersionedStorageChange` 广播。
- 旧的 `ModuleManager`、`ModuleRootManager` 通过 legacy bridge 兼容 Workspace Model。
- 这是 IDEA 目前推荐的项目结构模型，适合替换“每次全量重建项目树”的做法。

对 Keel 的启示：

- 项目树/构建模型不要直接绑定文件系统扫描结果；应有一个不可变项目快照 + 变更事件。
- Maven/Gradle 模型、源码根、依赖库都可以建模成实体，UI 只读快照。

---

## 7. 索引与搜索

API：

- `platform/indexing-api/src/com/intellij/util/indexing/FileBasedIndex.java`
- `platform/indexing-api/src/com/intellij/util/indexing/FileBasedIndexExtension.java`
- `platform/indexing-impl/backend/src/com/intellij/util/indexing/FileBasedIndexImpl.java`
- `platform/indexing-impl/`
- `platform/core-api/src/com/intellij/openapi/project/DumbService.kt`

要点：

- 文件级索引：`FileBasedIndexExtension` 定义 key/value、`DataIndexer`、输入过滤。
- Stub 索引：把 PSI 的声明信息持久化，支持快速全局查找。
- 索引构建在后台进行，期间进入 Dumb Mode。
- 查询通过 `GlobalSearchScope`、`ProjectFileIndex` 限定范围。
- 索引数据持久化在系统缓存目录，项目重新打开时增量更新。

搜索相关模块：

- `platform/find/`：文本查找。
- `platform/searchEverywhere/`：Search Everywhere。
- `platform/indexing-*`：结构化搜索的基础。
- `platform/usageView*/`：引用搜索与结果展示。

---

## 8. UI 架构

### 8.1 主窗口

关键类：

- `platform/platform-impl/src/com/intellij/openapi/wm/impl/IdeFrameImpl.kt`
- `platform/platform-impl/src/com/intellij/openapi/wm/impl/IdeRootPane.kt`
- `platform/platform-impl/src/com/intellij/openapi/wm/impl/headertoolbar/MainToolbar.kt`
- `platform/platform-impl/src/com/intellij/openapi/wm/impl/customFrameDecorations/`

结构：

```text
IdeFrame (JFrame)
  └─ IdeRootPane (JRootPane)
       ├─ CustomHeader / 标题栏
       ├─ MainToolbar（新 UI 顶部主工具栏）
       ├─ MenuBar（系统菜单）
       ├─ 内容区
       │    ├─ ToolWindowPane（左/右/底部工具窗口）
       │    ├─ EditorsSplitters（编辑器分栏）
       │    └─ StatusBar
       └─ LayeredPane（弹层、通知、Tooltip）
```

- `IdeRootPane` 支持自定义标题栏、macOS 标签、主工具栏。
- 新 UI 的标题栏/工具栏/岛屿布局在 `customFrameDecorations` 与 `headertoolbar` 中。
- 窗口按钮、工具窗口条纹、状态栏都按 `UISettings` 与主题度量绘制。

### 8.2 工具窗口

关键类：

- `platform/platform-api/src/com/intellij/openapi/wm/ToolWindowManager.kt`
- `platform/platform-impl/src/com/intellij/openapi/wm/impl/ToolWindowManagerImpl.kt`
- `platform/platform-impl/src/com/intellij/toolWindow/ToolWindowPane.kt`
- `platform/platform-impl/src/com/intellij/toolWindow/ToolWindowStripeManager.kt`

要点：

- 工具窗口通过 `toolWindow` 扩展点或 `ToolWindowFactory` 注册。
- 支持左/右/底部锚点、显示/隐藏、拖拽、浮动、最大化、条纹按钮。
- `ToolWindowPane` 负责条纹、标题栏、内容区、分隔条。
- 内容通常是 `JComponent`；新代码可以用 `ToolWindowFactory` + `Content`。

### 8.3 编辑器区域

关键类：

- `platform/analysis-api/src/com/intellij/openapi/fileEditor/FileEditorManager.kt`
- `platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/FileEditorManagerImpl.kt`
- `platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/EditorsSplitters.kt`
- `platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/EditorWindow.kt`

要点：

- `FileEditorManager` 管理“虚拟文件 → FileEditor”的映射和打开/关闭事件。
- `FileEditorProvider` 决定某类文件由哪个编辑器打开（文本、图片、设计器、差异等）。
- `EditorsSplitters` 管理编辑器分栏；`EditorWindow` 管理单个标签页集合。
- 文本编辑器走 `TextEditorProvider → EditorFactory → Editor`。
- 标签页、预览标签、分屏、关闭策略都在这一层扩展。

### 8.4 动作系统

关键类：

- `platform/editor-ui-api/src/com/intellij/openapi/actionSystem/ActionManager.java`
- `platform/editor-ui-api/src/com/intellij/openapi/actionSystem/AnAction.java`
- `platform/core-ui/src/openapi/actionSystem/DataContext.java`
- `platform/platform-impl/src/com/intellij/openapi/actionSystem/impl/ActionManagerImpl.kt`
- `platform/platform-impl/src/com/intellij/openapi/actionSystem/impl/ActionToolbarImpl.kt`

要点：

- 所有菜单项、工具栏按钮、快捷键都对应 `AnAction`。
- `ActionGroup` 组织层级；`ActionManager.createActionToolbar` / `createActionPopupMenu` 生成 UI。
- `DataContext` 提供动作执行时的上下文（当前项目、编辑器、文件、选中项）。
- 动作的可用性、选中态通过 `update()` 计算；`ActionUpdateThread` 决定在 EDT 还是后台更新。
- 快捷键由 `KeymapManager` 管理，动作 ID 是稳定契约。

对 Keel 的启示：

- 不要为每个菜单项写独立 UI 逻辑；应有统一的命令/动作注册表，菜单、工具栏、快捷键、命令面板共用同一份定义。
- 动作需要上下文（当前文件、选区、项目），而不是全局硬编码。

### 8.5 Swing/LAF/Jewel

- 主框架和大多数组件仍是 Swing。
- `JBUI`/`JBUIScale` 负责 DPI 缩放与尺寸换算。
- `JBColor` 按主题取色，支持浅色/深色回退。
- `UITheme` 解析 `.theme.json`，`UIManager` 注入 LAF 默认值。
- 新 UI 的部分组件通过 `platform/jewel`（Compose）与 `ide-laf-bridge` 接入 Swing。
- `ExperimentalUI` / `NewUiValue` 控制新 UI 开关。

---

## 9. 构建、运行与调试

### 9.1 JPS

目录：

- `jps/model-api/`：`JpsModel`、`JpsProject`、`JpsModule`、依赖、源根。
- `jps/model-impl/`：模型实现与序列化。
- `jps/jps-builders/`：增量编译器、`CompileScope`、构建目标。
- `jps/standalone-builder/`：独立构建进程。
- `jps/jps-launcher/`、`jps/antLayout/`：启动与打包布局。

要点：

- IDE 不直接调用 `javac` 完成项目构建；它把项目模型交给 JPS，由独立构建进程执行。
- 增量构建基于时间戳/内容哈希与 `CompileScope`。
- 构建模型与 IDE 项目模型分离，便于命令行构建和远程构建。

### 9.2 运行/调试

关键 API：

- `platform/execution/src/com/intellij/execution/RunManager.kt`
- `platform/execution/src/com/intellij/execution/ExecutionManager.kt`
- `platform/execution-impl/`
- `platform/xdebugger-api/`、`platform/xdebugger-impl/`
- `java/debugger/`、`java/execution/`

要点：

- `RunManager` 管理运行配置列表。
- `ConfigurationType` / `ConfigurationFactory` 创建 `RunConfiguration`。
- `ProgramRunner` 负责“如何执行”（本地进程、远程、调试）。
- `ExecutionManager` 管理运行/调试会话、控制台、进程生命周期。
- Java 调试基于 JDWP；`XDebugger` 提供通用调试 UI 与协议抽象。

---

## 10. 设置与持久化

关键 API：

- `platform/projectModel-api/src/com/intellij/openapi/components/PersistentStateComponent.java`
- `platform/projectModel-api/src/com/intellij/openapi/components/State.java`
- `platform/projectModel-api/src/com/intellij/openapi/components/Storage.java`
- `platform/ide-core/src/com/intellij/openapi/options/Configurable.java`
- `docs/configurable.md`

要点：

- 需要持久化的服务实现 `PersistentStateComponent<T>`，用 `@State` / `@Storage` 指定文件名与存储级别。
- `getState()` / `loadState()` 由平台调用；只序列化与默认值不同的字段。
- 设置页实现 `Configurable`：`createComponent`、`reset`、`apply`、`isModified`、`disposeUIResources`。
- `SearchableConfigurable` 支持设置搜索；`ConfigurableProvider` 可动态决定是否显示。
- 设置页通过 `applicationConfigurable` / `projectConfigurable` 扩展点注册。
- `UISettings` 自己就是一个持久化服务（`ui.lnf.xml`）。

对 Keel 的启示：

- 主题、语言、窗口布局、最近项目都应该走统一的持久化服务，而不是零散字段。
- 设置页应有 `reset/apply/isModified` 语义，支持“取消”不落盘。

---

## 11. 主题、图标与本地化

### 11.1 主题

- 主题 JSON：`platform/platform-resources/src/themes/islands/ManyIslandsLight.theme.json`（及其他 Islands 主题）。
- 主题元数据：`platform/platform-resources/src/themes/metadata/IntelliJPlatform.themeMetadata.json`。
- 解析与应用：`platform/platform-impl/src/com/intellij/ide/ui/laf/`、`UITheme.kt`、`JBColor.java`。
- 继承：`parentTheme`、`editorScheme`；编辑器配色是独立 XML（`themes/expUI/*.xml`）。
- 调色板：`colors` 定义 `gray-*`、`blue-*` 等；UI key 引用调色板。
- 图标：`platform/icons/`、`icons-api/`、`icons-impl/`，支持 SVG 着色、深色变体、路径替换。

### 11.2 本地化

- `DynamicBundle` / `AbstractBundle`。
- `IdeBundle`：`platform/platform-api/src/com/intellij/ide/IdeBundle.java`。
- 资源文件：`platform/platform-api/resources/messages/IdeBundle.properties`。
- 语言与地区：`platform/platform-impl/src/com/intellij/ide/ui/LanguageAndRegionConfigurable.kt`。
- 用户可见字符串必须进 `.properties`；支持占位符、语言包、插件自带 bundle。

---

## 12. 测试与质量

- `platform/testFramework/`：平台测试基类、Light/Heavy fixture。
- `platform/testIntegration/`：UI/集成测试。
- `platform/platform-tests/`：平台自带测试。
- `java/java-tests/`：Java 功能测试。
- 测试入口与模块名由 `bt.cmd --module <module> --filter <FQN>` 指定（见 `.ai/guide.md`）。
- API 兼容性通过 `api-dump.txt` 与 `@ApiStatus` 管控。
- 代码风格：`.agents/skills/code-style/SKILL.md`。

---

## 13. 对 Keel 的映射

| IDEA 机制 | Keel 现状 | 建议 |
| --- | --- | --- |
| Service Container（应用/项目/模块三级） | `keel-core` 已提供服务容器、Disposable、MessageBus；GPUI 的 `KeelApp` 仍持有大部分状态 | UI 改为从 `AppServices` / `ProjectServices` 取服务 |
| Extension Point + plugin.xml | 无 | 先做进程内扩展注册表，后续再考虑插件 |
| Action/Command + DataContext | `CommandId` + 手写菜单 | 统一 CommandRegistry，菜单/工具栏/快捷键共用 |
| VFS | 直接 `std::fs` | 抽象 `VirtualFileSystem`，支持本地/远程/归档 |
| Document/PSI/Index | 只有编辑器文本 | 引入文档缓冲、tree-sitter 语法树、索引层 |
| Workspace Model 快照 | `keel-workspace` 已有 `ProjectModel` + 不可变 `ProjectSnapshot` + 变更事件；GPUI 树尚未接入 | UI 只读快照，订阅变更刷新 |
| Read/Write 锁 + 取消 | GPUI 单线程 + 少量后台任务 | 后台任务统一取消令牌与快照读 |
| JPS 外部构建 | 尚未接入 | 直接调用 Maven/Gradle，不移植 JPS |
| RunManager/ProgramRunner | 未实现 | 运行配置模型 + 外部进程管理 |
| Theme JSON / i18n properties | 主题已外置到 `themes/idea-*.json`；文案仍硬编码 | i18n 资源文件化 + 加载器 + 继承 |
| Swing/JBUI/LAF | GPUI + 手写度量 | 保留 GPUI，但把度量/颜色/文案外置为资源 |

### 13.1 推荐给 Keel 的目标架构

```text
keel/
  src/
    app/            应用生命周期、窗口、路由
    services/       应用级/项目级服务容器
    commands/       命令、快捷键、菜单/工具栏描述
    workspace/      不可变项目快照 + 变更事件
    vfs/            VirtualFile 抽象（本地/远程/归档）
    document/       文本缓冲、编辑事务、保存
    syntax/         tree-sitter 语法树/高亮
    language/       LSP 客户端、语义层
    index/          符号索引、全文索引
    build/          Maven/Gradle 外部进程
    run/            运行配置、进程管理
    ui/             GPUI 组件、岛屿布局、主题/度量
  assets/
    themes/         *.theme.json
    locales/        *.json / *.properties
    icons/
```

### 13.2 不建议照搬的部分

- Swing/EDT/LAF 的具体实现（Keel 用 GPUI）。
- PicoContainer/Component 旧模型（只借鉴 Service + 构造注入）。
- JPS 的完整构建模型（Keel 直接对接 Maven/Gradle 更现实）。
- 插件类加载器与远程开发协议（远期再做）。
- IDEA 的内部实现细节（`@ApiStatus.Internal`）。

---

## 14. 代码阅读路线

### 14.1 想理解“应用怎么启动”

1. `platform/bootstrap/src/com/intellij/idea/Main.kt`
2. `platform/platform-impl/src/com/intellij/openapi/application/impl/ApplicationImpl.java`
3. `platform/service-container/src/com/intellij/serviceContainer/ComponentManagerImpl.kt`
4. `platform/platform-impl/src/com/intellij/openapi/project/impl/ProjectImpl.kt`

### 14.2 想理解“服务与扩展点”

1. `platform/core-api/src/com/intellij/openapi/components/Service.java`
2. `platform/service-container/src/com/intellij/serviceContainer/ComponentManagerImpl.kt`
3. `platform/extensions/src/com/intellij/openapi/extensions/ExtensionPoint.java`
4. `platform/core-api/resources/intellij.platform.core.xml`
5. `platform/core-impl/src/com/intellij/ide/plugins/PluginManagerCore.kt`

### 14.3 想理解“UI 怎么搭”

1. `platform/platform-impl/src/com/intellij/openapi/wm/impl/IdeFrameImpl.kt`
2. `platform/platform-impl/src/com/intellij/openapi/wm/impl/IdeRootPane.kt`
3. `platform/platform-impl/src/com/intellij/openapi/wm/impl/headertoolbar/MainToolbar.kt`
4. `platform/platform-impl/src/com/intellij/toolWindow/ToolWindowPane.kt`
5. `platform/platform-impl/src/com/intellij/openapi/fileEditor/impl/EditorsSplitters.kt`
6. `platform/editor-ui-api/src/com/intellij/openapi/actionSystem/ActionManager.java`

### 14.4 想理解“编辑器/PSI/索引”

1. `platform/core-api/src/com/intellij/openapi/vfs/VirtualFile.java`
2. `platform/core-api/src/com/intellij/openapi/editor/Document.java`
3. `platform/core-api/src/com/intellij/psi/PsiFile.java`
4. `platform/indexing-api/src/com/intellij/util/indexing/FileBasedIndex.java`
5. `platform/core-api/src/com/intellij/openapi/project/DumbService.kt`
6. `java/java-psi-api/src/com/intellij/psi/JavaPsiFacade.java`

### 14.5 想理解“项目模型”

1. `platform/core-api/src/com/intellij/openapi/project/Project.java`
2. `platform/backend/workspace/src/WorkspaceModel.kt`
3. `platform/workspace/storage/src/com/intellij/platform/workspace/storage/`
4. `platform/projectModel-api/src/com/intellij/openapi/module/`
5. `jps/model-api/src/org/jetbrains/jps/model/JpsModel.java`

### 14.6 想理解“设置/主题/本地化”

1. `platform/projectModel-api/src/com/intellij/openapi/components/PersistentStateComponent.java`
2. `docs/configurable.md`
3. `platform/platform-resources/src/themes/islands/ManyIslandsLight.theme.json`
4. `platform/platform-impl/src/com/intellij/ide/ui/UITheme.kt`
5. `platform/util/ui/src/com/intellij/ui/JBColor.java`
6. `platform/platform-api/src/com/intellij/ide/IdeBundle.java`
7. `platform/platform-impl/src/com/intellij/ide/ui/LanguageAndRegionConfigurable.kt`

---

## 15. 术语表

| 术语 | 含义 |
| --- | --- |
| PSI | Program Structure Interface，语言结构化模型 |
| VFS | Virtual File System，虚拟文件系统 |
| Stub | 持久化的 PSI 声明信息，供索引/导航 |
| Dumb Mode | 索引未就绪时的受限模式 |
| EDT | Event Dispatch Thread，Swing UI 线程 |
| EP | Extension Point，扩展点 |
| LAF | Look and Feel，Swing 外观 |
| JPS | JetBrains Project System，外部构建模型/编译器 |
| JBR | JetBrains Runtime |
| Workspace Model | 不可变项目结构快照模型 |
| Configurable | 设置页接口 |
| DynamicBundle | 可本地化的资源包 |

---

## 16. 维护说明

- 本文只覆盖“架构骨架”，不替代 SDK 文档。
- 同类复刻项目的架构对照见 [REPLICA_ARCHITECTURE_COMPARISON.md](./REPLICA_ARCHITECTURE_COMPARISON.md)。
- 改动 IDEA 相关结论时，优先以当前 checkout 的源码和 `docs/` 为准。
- 如果 Keel 的架构发生变化，建议同步更新第 13 节的映射与目标结构。

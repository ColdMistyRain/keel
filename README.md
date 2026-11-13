# Keel

借鉴 IntelliJ IDEA 交互范式的 Rust 原生 IDE，UI 使用
[GPUI](https://github.com/gpui-ce/gpui)（`gpui-ce 0.3.0` + `gpui-component`），
Windows 优先，优先支持 Java。

nsh 版本已归档到 [`legacy/nsh/`](legacy/nsh/)，不再参与默认构建；
迁移与回退记录见 [`docs/NSH_MIGRATION_PLAN.md`](docs/NSH_MIGRATION_PLAN.md)。

## 运行

```powershell
$env:CARGO_HOME="C:\Users\lzx\.cargo"
cargo run
```

验证：

```powershell
cargo fmt --check
cargo test
cargo clippy --no-deps --all-targets
```

窗口尺寸、主题、面板宽度、最近打开的项目等写入 `%APPDATA%\Keel\state.json`；
配色、圆角、编辑器语法色全部来自 [`themes/idea-light.json`](themes/idea-light.json)
与 [`themes/idea-dark.json`](themes/idea-dark.json)，由 `keel-theme` 在启动时载入。

## 界面结构

| 区域 | 实现 |
| --- | --- |
| 主工具栏 | 自绘标题栏：汉堡菜单、项目/分支部件、搜索与设置；右侧系统窗口按钮 |
| 应用菜单 | 文件 / 编辑 / 视图 / 导航 / 工具 / 窗口，只保留有真实动作的项；快捷键与命令面板共用动作注册表 |
| 工具窗口条 | 左侧条纹（Project / Search / Git / Maven / 底部工具窗口 / 设置），选中为 IDEA 蓝色高亮块 |
| 项目面板 | 目录展开/折叠、选中、双击打开、文件类型图标；标题栏动作、右键菜单、新建/重命名/删除/复制路径、拖拽调宽 |
| 编辑器 | 标签页（溢出滚动、下拉列表、拖拽排序、右键菜单、未保存圆点、悬停关闭）、行号、缩进参考线、查找替换、4 空格缩进 |
| 状态栏 | 左侧面包屑导航（可点击在项目树中定位），右侧行列号/换行/编码/缩进/语言 |
| 底部面板 | Problems / Run / Terminal 三个占位页，可拖高、可隐藏 |
| 命令面板 | `Ctrl+Shift+P` 按名称搜索动作，↑↓ 选择、Enter 执行、Esc 关闭 |
| 设置/关于 | `Ctrl+Alt+S` 设置页与关于页，可切换浅色/Darcula 与文本渲染方式 |
| 通知 | 右下角气泡提示保存、刷新、文件操作结果 |

## 代码结构

```
src/main.rs            入口、窗口、字体、快捷键、主题初始化
src/app.rs             根视图：工具栏、菜单、状态栏、对话框、命令面板、通知
src/editor.rs          编辑器标签、文档生命周期、查找替换
src/project_tree.rs    项目树渲染与交互
src/project.rs         pom.xml / settings.gradle 探测（尚未接入 UI）
src/model.rs           目录树、过滤、排序、语言识别、state.json
src/idea_theme.rs      IDEA 令牌 → GPUI 颜色与语法主题
src/file_icons.rs      文件类型图标与语言着色
src/tool_window.rs     左侧工具窗口条
src/panels.rs          底部面板
src/menu.rs            菜单弹层表面（1px 描边）
src/tooltip.rs         自绘工具提示
crates/keel-theme      读取 themes/idea-*.json（颜色、圆角、语法配色、调色板）
samples/demo/          Java 示例项目（pom.xml + Main.java + Greeter.java）
assets/                IDEA expui SVG 图标与 Inter / JetBrains Mono 字体
themes/                IDEA Light / Darcula 令牌配置
legacy/nsh/            归档的 nsh 版本（独立 workspace，不参与默认构建）
```

分层规则：UI 模块只读写应用状态；外部 IO（文件、进程、网络、LSP）
一律放到后台任务，结果通过 `cx.update` 回到 UI。

## 参考

- [开发大纲与任务卡](docs/DEVELOPMENT_OUTLINE.md)
- [IDEA 社区版架构参考](docs/INTELLIJ_ARCHITECTURE_REFERENCE.md)
- [复刻项目架构对照](docs/REPLICA_ARCHITECTURE_COMPARISON.md)
- [nsh 迁移与回退记录](docs/NSH_MIGRATION_PLAN.md)

## 已知缺口

- 搜索 / 版本控制 / 问题 / 运行 / 终端面板仍是占位，运行不执行真实程序；
- Maven / Gradle 探测尚未接入 UI，语言智能（jdtls）、调试、Git 未开始；
- 编辑器只有一个编辑器组，没有分栏与标签跨组拖动；
- 预览标签（单击预览、双击固定）与顶部 run widget 是 nsh 分支期间新增的，
  尚未回迁到 GPUI 主线。

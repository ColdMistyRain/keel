# Keel

借鉴 IntelliJ IDEA 交互范式的 Rust 原生 IDE，UI 使用
[Native Shell（nsh）](https://nsh.dev/zh-CN/)，**纯 Rust、无 WebView**。

原 GPUI 版本已归档到 [`legacy/gpui/`](legacy/gpui/)，不再参与默认构建；
迁移过程见 [`docs/NSH_MIGRATION_PLAN.md`](docs/NSH_MIGRATION_PLAN.md)。

## 运行

```powershell
# 安装 nsh（仅首次）
irm https://nsh.dev/install.ps1 | iex

# 启动工作台
nsh run .

# 启动时打开一个文件夹或文件（开发环境；打包产物直接用第一个路径参数）
$env:KEEL_OPEN = "d:\path\to\project"; nsh run .

# 自动打开内置示例项目（截图/验收用）
$env:KEEL_DEMO = "1"; nsh run .

# 无头测试
nsh test

# 真实 Windows 窗口测试，并刷新 docs/screenshots 下的截图
$env:KEEL_DEMO = "1"; nsh test --target windows

# 打包 Windows 产物（release 构建约 2 分半；安装包需要本机装 Inno Setup 6）
nsh package --target windows
```

`nsh run` 会监听 Rust 源码：改动后自动重新编译并重启（本机约 5 秒编译 + 7 秒重建 runner）。

## 界面结构

| 区域 | 实现 |
| --- | --- |
| 标题栏 | 自绘 40px 标题栏，`.window_drag(true)` 拖动窗口，右侧给系统窗口按钮留白（`title_bar().right`）；菜单、项目名、run widget（运行 / 配置选择 / 调试）、搜索 / 设置快捷图标 |
| 应用菜单 | 文件 / 视图 / 导航 / 帮助，全部来自 `keel-commands` 的动作注册表；菜单项带 16×16 SVG 图标与助记符下划线（`文件(F)`、`打开项目(P)…`）；快捷键也从注册表绑定 |
| 工具窗口条 | 左侧 40px 竖条：项目、搜索、版本控制 |
| 项目面板 | `resizable_panel` 宽度 180–460、可折叠；标题栏有刷新 / 收起全部 / 隐藏；树用自绘 `scroll` 列表，16×16 文件夹 / 文件图标，右键菜单（打开 / 预览 / 复制路径 / 在项目中定位） |
| 编辑器区 | IDEA Islands 胶囊标签（条高 36、标签高 28、选中蓝底 + 1px 边框、未选中无底色）、标签溢出时右侧滚动箭头、预览标签（单击斜体、双击固定）、未保存圆点、关闭、右键菜单；可悬停的面包屑、`nsh_base::editor` |
| 运行 | `Shift+F10` 运行、`Ctrl+F2` 停止；run widget 显示绿色运行 / 红色停止按钮、运行配置选择器和调试按钮，运行中打开底部「运行」窗口 |
| 命令面板 | `Ctrl+Shift+A` 按名称搜索动作注册表；↑↓ 选择、Enter 执行、Esc 关闭；背景用主题的 `overlay` 遮罩 |
| 底部工具窗口 | 问题 / 运行 / 终端占位；`v_resizable` 高度 90–360；状态栏左侧有图标切换条 |
| 状态栏 | 提示、分支、语言、编码、换行、缩进、光标位置 |

配色、圆角、编辑器语法配色全部来自
[`themes/idea-light.json`](themes/idea-light.json) 与
[`themes/idea-dark.json`](themes/idea-dark.json)：启动时由 `keel-theme`
转成 nsh 的 shadcn `cssVars`，再通过 `set_tokens` 注入，Rust 代码里没有色值常量。
外观跟随系统，可在「视图 → 外观」里切换「跟随系统 / 浅色 / 深色」（带勾选）。

## 代码结构

```
src/lib.rs            IdeRootPane：标题栏、工具窗口条、分栏、状态栏
src/shell.rs          外壳状态：工具窗口、底部窗口、外观、动作注册表、UiState
src/commands.rs       动作注册与菜单渲染：菜单 / 快捷键 / 命令面板共用一份定义
src/palette.rs        命令面板：搜索、排序、键盘选择与执行
src/project_panel.rs  左侧工具窗口：条纹、标题栏、项目树与右键菜单
src/editor_area.rs    编辑器区：标签页与右键菜单、面包屑、编辑器、欢迎页
src/bottom_panel.rs   底部工具窗口：切换条、标题栏、占位内容
src/ui.rs             共用小部件与主题取值（菜单项、助记符、图标按钮、条纹按钮）
src/icons.rs          菜单与命令面板的 16×16 SVG 图标、动作 / 分组助记符
src/workspace.rs      nsh 侧状态：把 keel-workspace 的模型包成信号，管理文档与保存
crates/keel-core      服务容器、Disposable、MessageBus / Topic
crates/keel-workspace 项目快照、目录扫描、树摊平、语言识别、项目根识别
crates/keel-commands  Action / ActionGroup / DataContext / 注册表
crates/keel-theme     读取 themes/idea-*.json，生成 GPUI 令牌与 nsh cssVars
samples/demo/         Java 示例项目（pom.xml + Main.java + Greeter.java）
legacy/gpui/          归档的 GPUI 基线（独立 workspace，不参与默认构建）
nsh.json              应用配置：窗口尺寸、隐藏标题栏（主题不在这里）
```

`workspace.rs` 只持有 nsh 信号；项目结构来自 `keel-workspace` 的不可变
`ProjectSnapshot`（IDEA Workspace Model 式快照），目录扫描、树摊平、语言识别
都在 [`keel-workspace`](crates/keel-workspace/src/lib.rs)。动作、菜单分组、
`DataContext` 与注册表在 [`keel-commands`](crates/keel-commands/src/lib.rs)。

## 验收记录

P0 / P1 / P2 的完整记录见
[`docs/NSH_MIGRATION_PLAN.md`](docs/NSH_MIGRATION_PLAN.md)。要点：

- nsh 0.5.0 在 Windows 上可构建、可运行，`nsh run` 热重载可用；
- 编辑器、标签页、面包屑、状态栏、自绘标题栏、菜单、浅色/深色主题可正常渲染；
- 编辑、`Ctrl+S` 保存、中文插入（`SendKeys` 注入）已实测；
- `nsh_base::Tree` 在 Windows 窗口里不绘制行（父视口虚拟列表不绘制），
  项目树改用 `scroll` + `dynamic` 自绘列表；
- 文件选择用 `pick_paths`（原生打开对话框，返回真实路径），配合 `std::fs` 读写。

P2 外壳（2026-11-12）：

| 项 | 结果 |
| --- | --- |
| `cargo clippy --all-targets -- -D warnings` | 通过 |
| `nsh test`（无头） | 9 通过：外壳文本、视图菜单、命令面板、项目树菜单，截图测试自动跳过 |
| `nsh test --target windows` | 9 通过：真实窗口中的菜单、命令面板、预览标签、右键菜单与截图 |
| 左侧工具窗口 | 可拖宽、可折叠；刷新 / 收起全部 / 隐藏可用 |
| 标签页 | 溢出横向滚动、预览标签（单击）、双击固定、未保存圆点、关闭、右键菜单 |
| 命令面板 | `Ctrl+Shift+A` 打开，按名称过滤，↑↓ / Enter / Esc 可用 |
| 项目树 | 单击预览、双击固定；右键打开 / 预览 / 复制路径 / 在项目中定位 |
| 底部工具窗口 | 问题 / 运行 / 终端切换与拖高可用；状态栏图标条同步 |
| 动作状态 | 无文档时保存 / 关闭变灰；工具窗口与外观菜单显示勾选 |
| 打开项目 | 菜单 / 欢迎页对话框、`KEEL_OPEN`、打包产物路径参数；文件向上识别项目根 |

截图：[浅色](docs/screenshots/nsh-p2-light.png) ·
[深色](docs/screenshots/nsh-p2-dark.png) ·
[文件菜单](docs/screenshots/nsh-p2-menu.png) ·
[预览标签](docs/screenshots/nsh-p2-preview.png) ·
[命令面板](docs/screenshots/nsh-p2-palette.png) ·
[运行中的 run widget](docs/screenshots/nsh-p2-running.png) ·
[项目树右键菜单](docs/screenshots/nsh-p2-tree-menu.png)。

## 已知缺口

- 只有 Java / XML 使用 expui 彩色图标，其他语言仍是通用 `file.svg`；
- 搜索 / 版本控制工具窗口、问题 / 运行 / 终端面板仍是占位（运行只驱动 UI，不执行真实程序）；
- 编辑器只有一个编辑器组，没有分栏、拖拽标签与标签重排；
- 查找替换、设置页未迁移；命令面板目前只搜动作，不搜文件和类；
- 每次按键都会把整段文本写回信号（`editor` 的受控用法），大文件应改用
  `default_value` + 保存时读取（P3 处理）；
- 中文 IME 组合（拼音候选）仍需人工在窗口里验证；`SendKeys` 注入中文已验证。

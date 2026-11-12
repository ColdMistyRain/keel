# Keel nsh spike（P0–P2）

这是 Keel 迁移到 [Native Shell（nsh）](https://nsh.dev/zh-CN/) 的验证工程：
用 **纯 Rust UI** 复刻 IDEA 风格工作台外壳，验证 nsh 能否承载 Keel 的主界面。

它不改动仓库根目录下 GPUI 版本的 `src/`；GPUI 版本仍是可运行基线。

## 运行

```powershell
# 安装（仅首次）
irm https://nsh.dev/install.ps1 | iex

# 启动工作台
nsh run .

# 自动打开内置示例项目（截图/验收用）
$env:KEEL_SPIKE_DEMO = "1"; nsh run .

# 无头测试
nsh test

# 真实 Windows 窗口测试，并刷新 docs/screenshots 下的浅色 / 深色 / 菜单截图
$env:KEEL_SPIKE_DEMO = "1"; nsh test --target windows

# 打包 Windows 产物（release 构建约 2 分半；安装包需要本机装 Inno Setup 6）
nsh package --target windows
```

`nsh run` 会监听 Rust 源码：改动后自动重新编译并重启（本机约 5 秒编译 + 7 秒重建 runner）。

## 界面结构

| 区域 | 实现 |
| --- | --- |
| 标题栏 | 自绘 40px 标题栏，`.window_drag(true)` 拖动窗口，右侧给系统窗口按钮留白（`title_bar().right`） |
| 应用菜单 | 文件 / 视图 / 导航 / 帮助，全部来自 `keel-commands` 的动作注册表；快捷键也从注册表绑定 |
| 工具窗口条 | 左侧 40px 竖条：项目、搜索、版本控制 |
| 项目面板 | `resizable_panel` 宽度 180–460、可折叠；标题栏有刷新 / 收起全部 / 隐藏；树用自绘 `scroll` 列表 |
| 编辑器区 | 横向滚动的标签条（未保存圆点、关闭、右键菜单）、面包屑、`nsh_base::editor` |
| 底部工具窗口 | 问题 / 运行 / 终端占位；`v_resizable` 高度 90–360；状态栏左侧有切换条 |
| 状态栏 | 提示、分支、语言、编码、换行、缩进、光标位置 |

配色、圆角、编辑器语法配色和最近项目调色板全部来自仓库根目录的
[`themes/idea-light.json`](../../themes/idea-light.json) 与
[`themes/idea-dark.json`](../../themes/idea-dark.json)：启动时由 `keel-theme`
转成 nsh 的 shadcn `cssVars`，再通过 `set_tokens` 注入，Rust 代码里没有色值常量。
外观跟随系统，标题栏右侧可切换「自动 / 浅色 / 深色」，视图菜单里也有带勾选的三项。

## 代码结构

```
src/lib.rs           IdeRootPane：标题栏、工具窗口条、分栏、状态栏
src/shell.rs         外壳状态：工具窗口、底部窗口、外观、动作注册表、UiState
src/commands.rs      动作注册与菜单渲染：菜单 / 快捷键共用一份定义
src/project_panel.rs 左侧工具窗口：条纹、标题栏、项目树
src/editor_area.rs   编辑器区：标签页与右键菜单、面包屑、编辑器、欢迎页
src/bottom_panel.rs  底部工具窗口：切换条、标题栏、占位内容
src/ui.rs            共用小部件与主题取值（菜单触发器、图标按钮、条纹按钮）
src/workspace.rs     nsh 侧状态：把 keel-workspace 的模型包成信号，管理文档与保存
samples/demo/        Java 示例项目（pom.xml + Main.java + Greeter.java）
nsh.json             应用配置：窗口尺寸、隐藏标题栏（主题不在这里）
```

主题配置在仓库根目录的 `themes/idea-light.json`、`themes/idea-dark.json`，
由 `keel-theme` 读取；spike 没有自己的主题文件。

`workspace.rs` 只持有 nsh 信号；项目结构来自 `keel-workspace` 的不可变
`ProjectSnapshot`（IDEA Workspace Model 式快照），目录扫描、树摊平、语言识别
都在 [`keel-workspace`](../../crates/keel-workspace/src/lib.rs)。动作、菜单分组、
`DataContext` 与注册表在 [`keel-commands`](../../crates/keel-commands/src/lib.rs)。

## 验收记录

P0（nsh 能否承载工作台）与 P1（core 分层）的记录见仓库文档
[`docs/NSH_MIGRATION_PLAN.md`](../../docs/NSH_MIGRATION_PLAN.md)。要点：

- nsh 0.5.0 在 Windows 上可构建、可运行，`nsh run` 热重载可用；
- 编辑器、标签页、面包屑、状态栏、自绘标题栏、菜单、浅色/深色主题可正常渲染；
- 编辑、`Ctrl+S` 保存、中文插入（`SendKeys` 注入）已实测；
- `nsh_base::Tree` 在 Windows 窗口里不绘制行（父视口虚拟列表不绘制），
  spike 改用 `scroll` + `dynamic` 自绘列表；
- 文件选择用 `pick_paths`（原生打开对话框，返回真实路径），配合 `std::fs` 读写。

P2 外壳（2026-11-12）：

| 项 | 结果 |
| --- | --- |
| `cargo clippy --all-targets -- -D warnings` | 通过 |
| `nsh test`（无头） | 4 通过：外壳文本、视图菜单、两个截图测试自动跳过 |
| `nsh test --target windows` | 4 通过：真实窗口中的菜单交互与截图 |
| 左侧工具窗口 | 可拖宽、可折叠；刷新 / 收起全部 / 隐藏可用 |
| 标签页 | 溢出横向滚动、未保存圆点、关闭、右键菜单可用 |
| 底部工具窗口 | 问题 / 运行 / 终端切换与拖高可用；状态栏切换条同步 |
| 动作状态 | 无文档时保存 / 关闭变灰；工具窗口与外观菜单显示勾选 |

截图：[浅色](../../docs/screenshots/nsh-p2-light.png) ·
[深色](../../docs/screenshots/nsh-p2-dark.png) ·
[文件菜单](../../docs/screenshots/nsh-p2-menu.png)。

## 已知缺口

- 文件图标用字母徽标代替 IDEA 图标；
- 搜索 / 版本控制工具窗口、问题 / 运行 / 终端面板仍是占位；
- 编辑器只有一个编辑器组，没有分栏、拖拽标签与标签重排；
- 命令面板、查找替换、设置页未迁移；
- 每次按键都会把整段文本写回信号（`editor` 的受控用法），大文件应改用
  `default_value` + 保存时读取（P3 处理）；
- 中文 IME 组合（拼音候选）仍需人工在窗口里验证；`SendKeys` 注入中文已验证。

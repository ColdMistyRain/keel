# Keel nsh spike（P0）

这是 Keel 迁移到 [Native Shell（nsh）](https://nsh.dev/zh-CN/) 的 P0 验证工程：
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

# 打包 Windows 产物（release 构建约 2 分半；安装包需要本机装 Inno Setup 6）
nsh package --target windows
```

`nsh run` 会监听 Rust 源码：改动后自动重新编译并重启（本机约 5 秒编译 + 7 秒重建 runner）。

## 界面结构

| 区域 | 实现 |
| --- | --- |
| 标题栏 | 自绘 40px 标题栏，`.window_drag(true)` 拖动窗口，右侧给系统窗口按钮留白（`title_bar().right`） |
| 应用菜单 | 文件 / 视图 / 帮助，`nsh_base::menu` + `menu_item` |
| 工具窗口条 | 左侧 40px 竖条：项目、搜索、版本控制、终端 |
| 项目面板 | 自绘 `scroll` 列表（`nsh_base::Tree` 在 0.5.0 Windows 下不绘制行，见下文） |
| 编辑器区 | 自绘标签页 + 面包屑 + `nsh_base::editor`（Java 高亮、行号、折叠） |
| 状态栏 | 分支、提示、语言、编码、换行、缩进、光标位置 |

配色、圆角、编辑器语法配色和最近项目调色板全部来自仓库根目录的
[`themes/idea-light.json`](../../themes/idea-light.json) 与
[`themes/idea-dark.json`](../../themes/idea-dark.json)：启动时由 `keel-theme`
转成 nsh 的 shadcn `cssVars`，再通过 `set_tokens` 注入，Rust 代码里没有色值常量。
外观跟随系统，标题栏右侧可切换「自动 / 浅色 / 深色」。

## 代码结构

```
src/lib.rs        界面：标题栏、工具窗口、项目树、标签页、编辑器、状态栏
src/workspace.rs  nsh 侧状态：把 keel-workspace 的模型包成信号，管理文档与保存
samples/demo/     Java 示例项目（pom.xml + Main.java + Greeter.java）
nsh.json          应用配置：窗口尺寸、隐藏标题栏（主题不在这里）
```

主题配置在仓库根目录的 `themes/idea-light.json`、`themes/idea-dark.json`，
由 `keel-theme` 读取；spike 没有自己的主题文件。

`workspace.rs` 只持有 nsh 信号；目录扫描、树摊平、语言识别来自仓库里的
[`keel-workspace`](../../crates/keel-workspace/src/lib.rs)，与 GPUI 版本共用。

## P0 验收记录

见仓库文档 [`docs/NSH_MIGRATION_PLAN.md`](../../docs/NSH_MIGRATION_PLAN.md) 的
「P0 spike 记录」一节。要点：

- nsh 0.5.0 在 Windows 上可构建、可运行，`nsh run` 热重载可用；
- 编辑器、标签页、面包屑、状态栏、自绘标题栏、菜单、浅色/深色主题可正常渲染；
- 编辑、`Ctrl+S` 保存、中文插入（`SendKeys` 注入）已实测；
- `nsh_base::Tree` 在 Windows 窗口里不绘制行（父视口虚拟列表不绘制），
  spike 改用 `scroll` + `dynamic` 自绘列表；
- 文件选择用 `pick_paths`（原生打开对话框，返回真实路径），配合 `std::fs` 读写；
- 字体、图标、停靠布局、LSP 适配留给 P2/P3。

## 已知缺口

- 文件图标用字母徽标代替 IDEA 图标；
- 工具窗口只有「项目」有内容，其余为占位；
- 底部面板、命令面板、查找替换、设置页未迁移；
- 每次按键都会把整段文本写回信号（`editor` 的受控用法），大文件应改用
  `default_value` + 保存时读取（P3 处理）；
- 中文 IME 组合（拼音候选）仍需人工在窗口里验证；`SendKeys` 注入中文已验证。

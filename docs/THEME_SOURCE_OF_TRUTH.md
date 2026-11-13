# 外观真值的取法与校验

> 适用对象：所有“复刻 IDEA 新 UI”的界面代码。
> 本文只讲**值从哪来、怎么验**，不涉及架构。

## 0. 一句话结论

外观差得远，通常不是“某个色值抄错了”，而是**取值方式错**：IDEA 的最终外观 =
`5 层主题继承` × `别名链` × `平台覆盖` × `组件绘制代码派生量` × `字体/DPI 对齐`。
只抄单个 `.theme.json` 或截图吸色，必然漏掉后四项。

`tools/theme-merge.mjs` 把前两项变成可复现的解析结果，输出在 `themes/generated/`。

## 1. 真值链（Islands，2024+ 新 UI）

| 层 | 文件（相对 `platform/platform-resources/src`） | 注册 id | 说明 |
|---|---|---|---|
| 5（最具体） | `themes/islands/ManyIslandsLight.theme.json` | `Islands Light` | `LafManagerImpl.getPlatformDefaultId()` 返回它，**平台默认 Light** |
| 4 | `themes/expUI/expUI_light_with_light_header.theme.json` | `ExperimentalLightWithLightHeader` | 浅色标题栏 |
| 3 | `themes/expUI/expUI_light.theme.json` | `ExperimentalLight` | 定义 `Gray1..14` / `Blue1..12` |
| 2 | `themes/intellijlaf.theme.json` | `IntelliJ` | LAF 基线 |
| 1（兜底） | `themes/darcula.theme.json` | `Darcula` | 底层兜底值 |

Dark 链：`Islands Dark` → `ExperimentalDark` → `Darcula`。

主题 id 的注册位置：`platform/platform-impl/resources/intellij.platform.ide.impl.xml`（`<themeProvider>`）。

## 2. 六条铁律

1. **`arc` 是直径，不是半径。**
   `IslandsUICustomization.kt`：`arcSizeF = JBUIScale.scale(arc / 2f)`、`drawArc(x, y, arcSize, arcSize, …)`。
   `Island.arc = 20` ⇒ 半径 **10**；`Button.arc = 8` ⇒ 半径 **4**；`TabbedPane.tabSelectionArc = 4` ⇒ 半径 **2**。
2. **`insets` 顺序是 `top,left,bottom,right`，且它决定尺寸。**
   `ActionButton.java`：`size = max(minimumSize, icon + myInsets) + getInsets()`。
   Islands Light：`MainToolbar.Icon.insets = 6,5,4,5`、`iconSize` 默认 20 ⇒ 按钮 **30×30**，
   但图标纵向是 **上 6 下 4**——**不是垂直居中**，差 1px。所有“flex 居中”的复刻在这里就已经不同了。
   compact：`insets = 3,4,3,4`、`iconSize = 16`、`size = 26,26`（来自 Darcula 兜底）。
3. **缺省值来自代码，不来自主题。**
   `MainToolbar.Button.arc` 不在任何主题文件里，默认值在 `JBUI.java:1426`：`UIInteger("MainToolbar.Button.arc", 12)` ⇒ 半径 6。
   `MainToolbar.Button.iconSize` 默认 20（`JBUI.java:1303`）。
   ⇒ 解析器必须同时输出“键缺失 → 代码默认值”的清单。
4. **别名链要递归解析。**
   `ActionButton.hoverBackground` = `toolbar-bg-hovered` → `core-bg-transparent-hovered` → `transparent-black-20` → `#00000012`（5 层文件里跨 3 级别名）。
5. **平台覆盖最后合并。**
   `Window.undecorated.border = "1,1,1,1,#5A5D6B"`（Windows 才有）；`ComboPopup.border` 同理。
6. **`transparent` 是关键字** ⇒ `#00000000`；`ActionButton.hoverBorderColor`、`pressedBorderColor`、`ToolWindow.borderColor`、`StatusBar.borderColor` 都是它——**描边画了但全透明**，几何上仍可能影响布局。

## 3. 关键真值卡（Islands Light，均带来源行）

| 键 | 值 | 来源 |
|---|---|---|
| `ActionButton.hoverBackground` | `#00000012` | `ManyIslandsLight.theme.json:445` |
| `ActionButton.pressedBackground` | `#00000020` | `:447` |
| `ActionButton.hoverBorderColor` / `pressedBorderColor` | `#00000000` | `:446` / `:448` |
| `ActionButton.focusedBorderColor` | `#3871E1`（`blue-80`） | `:449` |
| `ActionButton.hoverSeparatorColor` | `#00000020` | `:450` |
| `ActionButton.background` | **不存在** | 任何一层都没有 ⇒ normal 态没有独立底色 |
| `MainToolbar.background` | `#E9EAEE` | `:1179` |
| `MainToolbar.separatorColor` | `#00000020` | `:1183` |
| `MainToolbar.Icon.insets` | `6,5,4,5` | `:1188` |
| `MainToolbar.Button.arc` | 缺省 `12`（直径）⇒ 半径 6 | `JBUI.java:1426` |
| `MainToolbar.Button.arc.compact` | `8` ⇒ 半径 4 | `:1198` |
| `Island.arc` | `20`（直径）⇒ 半径 10 | `:416` |
| `Island.borderWidth` | `6`（compact 4） | `:420` |
| `Island.borderArcLength` | `14`（compact 10） | `:418` |
| `Island.inactiveAlpha` | `0.56` | `:423` |
| `Island.toolWindowAlpha` | `0.2` | `:424` |
| `TabbedPane.tabHeight` | `40`（compact 36） | `:1040` |
| `TabbedPane.tabSelectionArc` | `4`（直径）⇒ 半径 2 | `:1041` |
| `TabbedPane.tabSelectionHeight` | `3` | `intellijlaf.theme.json:865` |
| `TabbedPane.contentBorderInsets` | `3,1,1,1` | `intellijlaf.theme.json:831` |
| `TitlePane.Button.preferredSize` | `40,40`（compact `40,30`） | `:1209` |
| `StatusBar.topBorderWidth` | `0` | `:1021` |
| `StatusBar.background` | `#E9EAEE` | `:1019` |
| `Window.undecorated.border` | `1,1,1,1,#5A5D6B` | `:1201` |
| `EditorTabs.underlineHeight` | `4`（compact 3） | `:670` |
| `EditorTabs.underlinedTabBackground` | `#E3EBFE` | `:679` |
| `EditorTabs.underlinedBorderColor` | `#A7C5FF` | `:678` |
| `EditorTabs.hoverBackground` | `#00000012` | `:674` |
| `Button.arc` | `8`（直径）⇒ 半径 4 | `:510` |
| `Component.errorFocusColor` | `#E4656E` | `:600` |
| `Borders.color` | `#E9EAEE` | `:504` |
| `*.selectionBackground` | `#D0DFFE`（`blue-140`） | `ui["*"]` |
| `*.selectionInactiveBackground` | `#E9EAEE` | `ui["*"]` |
| `*.hoverBackground` | `#00000012` | `ui["*"]` |
| `*.separatorColor` | `#E9EAEE` | `ui["*"]` |

### Islands Dark 对照（同一键，值不同）

| 键 | Light | Dark |
|---|---|---|
| `ActionButton.hoverBackground` | `#00000012` | `#FFFFFF17` |
| `ActionButton.pressedBackground` | `#00000020` | `#FFFFFF29` |
| `MainToolbar.background` | `#E9EAEE` | `#26282C` |
| `Window.undecorated.border` | `1,1,1,1,#5A5D6B` | `1,1,1,1,#5A5D63` |

> 注意：pressed 的 alpha 在两套主题里**不对称**（12.5% vs 16%）。
> 复刻时若两套共用一个系数，深色主题会偏淡。

## 4. 已有差异（已核实 / 已修正）

| 项 | keel 原值 | 真值 | 状态 |
|---|---|---|---|
| 岛屿圆角 | 半径 20（把 `Island.arc` 当半径） | `Island.arc = 20` 是**弧直径** ⇒ 半径 10 | ✅ 已改为 10 |
| 弹层圆角 | 10（借用 island_radius） | `PopupMenu.borderCornerRadius = 8` | ✅ 已改为 8 |
| 通用悬停色 | `#00000008` | `*.hoverBackground = #00000012` | ✅ 已改为 `#00000012` / 深色 `#FFFFFF17` |
| 菜单分隔线 | `#CDCDCD` | `Menu/Popup.separatorColor = #D9D9D9` | ✅ 已改为 `#D9D9D9` |
| 工具栏图标 | flex 居中（上 5 下 5） | `insets = 6,5,4,5`（上 6 下 4） | ✅ 已给 30px 按钮加 `pt(6)/pb(4)` |
| `destructive: #DB3D3C` | — | 主题中无此值 | ⏳ 待查代码默认值 |
| `editor_selection / line_number / active_line_number` | — | 不在 UI 主题里 | ⏳ 属编辑区配色方案 |
| `stop_background: #E4656E` | — | `Component.errorFocusColor = #E4656E` | ✅ 有出处 |
| `brand: #3871E1` | — | `blue-80 = #3871E1` | ✅ |
| `control_hover/pressed` | — | `ActionButton.*` | ✅ 值正确 |
| `tab_height: 41` | — | `TabbedPane.tabHeight = 40`，但**编辑器标签用 Jewel/`SingleHeightTabs`**：行高 28 | ✅ GPUI 端 28 正确；41 只被 `legacy/nsh` 使用 |
| `brand: #3871E1` | `blue-80 = #3871E1` | ✅ |
| `control_hover/pressed: #00000012 / #00000020` | `ActionButton.*` | ✅ 值正确 |
| `island_arc 20` + `island_radius 10` | `Island.arc = 20`（直径） | ✅ 但要在类型上标注“直径”，避免二次换算 |

## 5. 尚未覆盖（必须承认的缺口）

1. **组件绘制代码派生量**：`ActionButton.getPreferredSize()` 的 `max(minimumSize, …)`、`IslandsUICustomization.paintIslandAreaRaw` 的 4 段矩形 + 4 段圆角填充、`IslandsGradientPainter` 的渐变。
2. **字体**：主题里没有字号/字重；`MainToolbar.fontSizeOffset.compact = -1` 只是补偿。真实值来自 `UISettings` 与 Swing `FontMetrics`，需要运行时量。
3. **DPI 取整**：`PaintUtil.alignIntToInt(…, RoundingMode.CEIL)` 是**向上取整**，非 100% 缩放下与 GPUI 的四舍五入不一致。
4. **图标**：`expui` / `expui_dark` 两套 SVG，20×20 与 16×16 变体，工具栏用 20。
5. **编辑区配色**：`editorScheme` 指向的方案文件不在本仓库的 `platform` 下（`git ls-files "*.icls"` 为空），需另找来源。

## 6. 工具用法

```powershell
# 默认解析 Islands Light + Islands Dark，输出到 themes/generated
node tools/theme-merge.mjs

# 指定仓库路径 / 主题 / 输出目录
node tools/theme-merge.mjs --root D:\Programming\Workspace\intellij-community `
  --themes "Islands Light,Islands Dark" --out themes/generated
```

输出：

| 文件 | 内容 |
|---|---|
| `<slug>.resolved.json` | 每个键的最终值、别名链、`来源文件:行`、`kind`（class/icon/insets/literal） |
| `<slug>.unresolved.json` | 解析不出的键（当前 0 个） |
| `keel-diff-<slug>.md` | 与 `themes/idea-light.json` 的逐值对照 + 焦点组件清单 |

## 7. 验收建议

1. **IDEA 内置 UI Inspector**（`platform/platform-impl/ui-inspector/.../UiInspectorAction.java`）：在真实 IDE 里直接量任意组件的 bounds / 颜色 / insets，作为代码派生量的唯一裁判。是否在你的发行版可用需确认（内部 action 可能需要 `-Didea.is.internal=true`）。
2. **截图 diff**：同尺寸窗口下截两张图，按区域逐像素比；差异超过 2 级才算问题（抗锯齿边缘不计）。
3. 每次改 UI 后重跑 `theme-merge`，确保没有新增“主题里查无出处”的值。

## 8. 截图对照记录（第一轮）

对比本地 IDEA 与 keel 同尺寸截图，确认已一致的部分：

| 项 | 依据 |
|---|---|
| 岛屿外边距 6px、岛屿圆角（半径 10） | `Island.borderWidth = 6`、`Island.arc = 20`（直径） |
| 左侧条 37 宽、按钮 37×40、高亮块 30×30、圆角 6 | `StripeToolbar.Button.size = 37,40`、`leftStripeIcon.padding = 5,5,5,2` |
| 项目树行高 24、选中块圆角 4 | `Tree.rowHeight = 24`、`Tree.Selection.arc = 8`（直径） |
| 编辑器标签：行高 28、胶囊高 20、圆角 6、选中 1px 蓝边 | `SingleHeightTabs.UNSCALED_PREF_HEIGHT = 28`、`IslandsTabPainter.paintTab` |
| 状态栏 / 工具栏 / 主窗口底色 #E9EAEE | `MainToolbar.background`、`StatusBar.background`、`MainWindow.background` |

第一轮修正：

- **项目头像**：改为 `AvatarUtils` 的真实画法——`GradientPaint(0, size, End, size, 0, Start)`（左下 End → 右上 Start，sRGB 插值）、`AvatarIcon(targetSize = 20, arcRatio = 0.4)` ⇒ 圆角半径 4、文字用 JetBrains Mono DEMIBOLD 13px、内容为 `AvatarUtils.initials(name)`（`"demo"` → `"D"`，`"MyProject"` → `"MP"`）。调色板取 `RecentProject.ColorN.Avatar.Start/End`（Islands 覆盖值，不是 `ProjectIconPalette` 的兜底色）。

仍待确认（没有足够证据，未改）：

- 状态栏高度：代码里 `minIconHeight = scale(20)`，部件边框来自 `StatusBar.Widget.border`（主题未定义，走代码默认），截图看比 keel 的 28 略矮。
- 工具窗口头 1px 分隔线（`ToolWindow.Header.borderColor`）是否绘制。
- `RecentProject` 的颜色序号在 IDEA 里由 `RecentProjectManager` 分配，keel 目前按路径哈希，具体色相可能不同（调色板与画法已一致）。


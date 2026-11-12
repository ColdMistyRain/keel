//! Keel 的主题令牌：取 src/idea_theme.rs 里的 IDEA Islands 数值，抽成不依赖 UI 框架的 crate。
//!
//! - GPUI 版本：`src/idea_theme.rs` 把这里的 `u32` 颜色转成 `Hsla` 后写进组件库主题；
//! - nsh 版本：`spike/keel-nsh-spike/theme.json` 是同一组令牌的 shadcn `cssVars` 表达。
//!
//! 这里只放数据，不放任何渲染或 UI 框架类型，两个 UI 才能共用。

/// 支持的 IDEA 主题。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeKind {
    /// IntelliJ Light
    #[default]
    Light,
    /// Darcula
    Dark,
}

impl ThemeKind {
    pub fn is_dark(self) -> bool {
        self == ThemeKind::Dark
    }

    pub fn tokens(self) -> &'static Tokens {
        match self {
            ThemeKind::Light => &LIGHT,
            ThemeKind::Dark => &DARK,
        }
    }
}

/// 一套主题的颜色令牌，取值全部来自 IDEA 2026 默认的 Islands 主题。
pub struct Tokens {
    /// `MainWindow.background`（岛屿之间的画布）
    pub canvas: u32,
    /// `editor-bg`（编辑器岛内部）
    pub content: u32,
    /// `text-default`
    pub foreground: u32,
    /// `editor-border` / `dialog-border`
    pub border: u32,
    /// 编辑器缩进参考线颜色（`INDENT_GUIDE`）。
    pub indent_guide: u32,
    /// `text-muted`：状态栏部件、树中的次要文字
    pub muted: u32,
    /// `text-secondary`：更次要的文字
    pub secondary: u32,
    /// `selection-bg-active`
    pub selection: u32,
    /// 选中态文字色
    pub selection_fg: u32,
    /// `selection-bg-hovered`：列表/树的悬停
    pub hover: u32,
    /// `toolbar-bg-hovered`：控件（按钮、侧边条、状态栏部件）的悬停
    pub control_hover: u32,
    /// `ToolWindow.background`
    pub tool_window_header: u32,
    /// `tab-selected-border-active`
    pub tab_underline: u32,
    /// 工具窗口标签选中下划线（`ToolWindow.HeaderTab.underlineColor`，默认 `DefaultTabs.underlineColor`）
    pub tool_window_tab_underline: u32,
    /// `EditorTabs.background`
    pub tab_background: u32,
    /// `tab-selected-bg-active`
    pub tab_selected_background: u32,
    /// `StatusBar.background`
    pub status_bar: u32,
    /// `accent-brand-bg`
    pub brand: u32,
    /// 品牌色的悬停/按下态（blue-70 / blue-60）
    pub brand_hover: u32,
    pub brand_active: u32,
    /// `CARET_COLOR`：编辑器光标
    pub caret: u32,
    /// 编辑器文本选中色（配色方案里的 `SELECTION_BACKGROUND`，比列表选中更饱和）
    pub editor_selection: u32,
    /// `popup-bg`：弹层背景
    pub popover: u32,
    /// `Popup.borderColor`：弹层的 1px 描边（Windows 用 `popup-border-windows`）
    pub popup_border: u32,
    /// `toolbar-border`：侧边条分组、工具栏分隔线
    pub separator: u32,
    /// `Separator.separatorColor`：菜单/设置页里的单线分隔条。
    pub separator_line: u32,
    /// `feedback-bg`：通知气泡、工具提示的底色（layer-2-bg）
    pub feedback: u32,
    /// `Notification.borderColor` → `feedback-border`（浅色 layer-0-border，深色与底色相同）
    pub feedback_border: u32,
    /// `ScrollBar.thumbColor`：半透明滚动条滑块，轨道透明
    pub scrollbar_thumb: u32,
    pub scrollbar_thumb_hover: u32,
    /// `EditorTabs.unselectedBlend`：未选中标签的文字混入背景的比例。
    pub unselected_blend: f32,
    /// `EditorTabs.unselectedAlpha`：未选中标签图标的透明度。
    pub unselected_alpha: f32,
    /// `Island.arc`
    pub island_arc: f32,
    /// 图标资源是否使用 `_dark` 变体。
    pub dark_icons: bool,
}

/// Islands Light 主题令牌。
///
/// 取值来自 intellij-community 源码：
/// `platform/platform-resources/src/themes/islands/ManyIslandsLight.theme.json`
/// 及其 gray-*/blue-* 调色板。
pub const LIGHT: Tokens = Tokens {
    canvas: 0xe9eaee,       // gray-150 / main-window-bg
    content: 0xffffff,      // layer-2-bg / editor-bg
    foreground: 0x000000,   // text-default
    border: 0xe9eaee,       // layer-2-border / editor-border
    indent_guide: 0xebecf0, // INDENT_GUIDE
    muted: 0x5f6269,        // gray-70 / text-muted
    secondary: 0x73767c,    // gray-80 / text-secondary
    selection: 0xd0dffe,    // blue-140 / selection-bg-active
    selection_fg: 0x000000,
    hover: 0x00000008,                   // transparent-black-10（8/255 透明度）
    control_hover: 0x00000012,           // transparent-black-20
    tool_window_header: 0xffffff,        // tool-window-bg
    tab_underline: 0xa7c5ff,             // blue-120 / tab-selected-border-active
    tool_window_tab_underline: 0x4083c9, // DefaultTabs.underlineColor（浅色）
    tab_background: 0xffffff,            // editor-bg
    tab_selected_background: 0xe3ebfe,   // blue-150
    status_bar: 0xe9eaee,                // main-window-bg
    brand: 0x3871e1,                     // blue-80
    brand_hover: 0x2f5eb9,               // blue-70
    brand_active: 0x2e4d89,              // blue-60
    caret: 0x000000,                     // CARET_COLOR（浅色方案未覆盖，取默认黑）
    editor_selection: 0xa6d2ff,          // expUI_lightScheme.xml / SELECTION_BACKGROUND
    popover: 0xffffff,                   // popup-bg = layer-2-bg
    popup_border: 0xd1d3d9,              // popup-border-windows = gray-130
    separator: 0x00000020,               // toolbar-border = transparent-black-30
    separator_line: 0xcdcdcd,            // Separator.separatorColor 未覆盖时回退 Gray.xCD
    feedback: 0xffffff,                  // feedback-bg = layer-2-bg
    feedback_border: 0xd1d3d9,           // feedback-border = layer-0-border(gray-130)
    scrollbar_thumb: 0x73737333,         // ScrollBar.Transparent.thumbColor
    scrollbar_thumb_hover: 0x73737347,   // ScrollBar.Transparent.hoverThumbColor
    unselected_blend: 0.9,               // EditorTabs.unselectedBlend（浅色）
    unselected_alpha: 0.75,              // EditorTabs.unselectedAlpha
    island_arc: 20.,                     // Island.arc（默认密度）
    dark_icons: false,
};

/// Islands Dark 主题令牌。
///
/// 取值来自 `ManyIslandsDark.theme.json`。
pub const DARK: Tokens = Tokens {
    canvas: 0x26282c,       // gray-30 / main-window-bg(layer-1)
    content: 0x191a1c,      // gray-10 / editor-bg(layer-0)
    foreground: 0xd1d3d9,   // gray-130 / text-default
    border: 0x26282c,       // gray-30 / editor-border
    indent_guide: 0x323438, // IslandSchemeDark.xml / INDENT_GUIDE
    muted: 0x9fa2a8,        // gray-100 / text-muted
    secondary: 0x73767c,    // gray-80 / text-secondary
    selection: 0x2a4371,    // blue-50 / selection-bg-active
    selection_fg: 0xd1d3d9,
    hover: 0xffffff10,                   // transparent-white-10
    control_hover: 0xffffff17,           // transparent-white-20
    tool_window_header: 0x191a1c,        // tool-window-bg(layer-0)
    tab_underline: 0x2e4d89,             // blue-60 / tab-selected-border-active
    tool_window_tab_underline: 0x4a88c7, // DefaultTabs.underlineColor（深色）
    tab_background: 0x191a1c,            // editor-bg
    tab_selected_background: 0x233558,   // blue-40
    status_bar: 0x26282c,                // main-window-bg(layer-1)
    brand: 0x3871e1,                     // blue-80
    brand_hover: 0x2f5eb9,               // blue-70
    brand_active: 0x2e4d89,              // blue-60
    caret: 0xced0d6,                     // CARET_COLOR（expUI_darkScheme.xml）
    editor_selection: 0x214283,          // Darcula 配色方案的 SELECTION_BACKGROUND
    popover: 0x26282c,                   // popup-bg = layer-1-bg
    popup_border: 0x33353b,              // popup-border = layer-1-border(gray-40)
    separator: 0xffffff21,               // toolbar-border = transparent-white-30
    separator_line: 0x33353b,            // Separator.separatorColor = popup-border（gray-40）
    feedback: 0x33353b,                  // feedback-bg = layer-2-bg(gray-40)
    feedback_border: 0x33353b,           // feedback-border = layer-2-bg，与底色相同
    scrollbar_thumb: 0xa6a6a647,         // ScrollBar.Transparent.thumbColor
    scrollbar_thumb_hover: 0xa6a6a659,   // ScrollBar.Transparent.hoverThumbColor
    unselected_blend: 0.7,               // EditorTabs.unselectedBlend（深色）
    unselected_alpha: 0.75,              // EditorTabs.unselectedAlpha
    island_arc: 20.,                     // Island.arc（默认密度）
    dark_icons: true,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_and_dark_differ_on_surfaces() {
        assert_ne!(LIGHT.canvas, DARK.canvas);
        assert_ne!(LIGHT.content, DARK.content);
        assert!(LIGHT.canvas > DARK.canvas);
        assert!(LIGHT.content > DARK.content);
    }

    #[test]
    fn selection_colors_stay_blue() {
        for tokens in [&LIGHT, &DARK] {
            let r = (tokens.selection >> 16) & 0xff;
            let b = tokens.selection & 0xff;
            assert!(b > r, "selection should be blue-ish");
        }
    }

    #[test]
    fn theme_kind_selects_tokens() {
        assert_eq!(ThemeKind::Light.tokens().canvas, LIGHT.canvas);
        assert_eq!(ThemeKind::Dark.tokens().canvas, DARK.canvas);
        assert!(!ThemeKind::Light.is_dark());
        assert!(ThemeKind::Dark.is_dark());
    }
}

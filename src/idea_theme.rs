use gpui::{App, Hsla, hsla, px};
use gpui_component::Theme;
use gpui_component::highlighter::HighlightTheme;
use gpui_component::theme::ThemeMode;
use std::sync::Arc;

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
    /// `feedback-bg`：通知气泡、工具提示的底色（layer-2-bg）
    pub feedback: u32,
    /// `ScrollBar.thumbColor`：半透明滚动条滑块，轨道透明
    pub scrollbar_thumb: u32,
    pub scrollbar_thumb_hover: u32,
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
    hover: 0x00000008,                 // transparent-black-10（8/255 透明度）
    control_hover: 0x00000012,         // transparent-black-20
    tool_window_header: 0xffffff,      // tool-window-bg
    tab_underline: 0xa7c5ff,           // blue-120 / tab-selected-border-active
    tab_background: 0xffffff,          // editor-bg
    tab_selected_background: 0xe3ebfe, // blue-150
    status_bar: 0xe9eaee,              // main-window-bg
    brand: 0x3871e1,                   // blue-80
    brand_hover: 0x2f5eb9,             // blue-70
    brand_active: 0x2e4d89,            // blue-60
    caret: 0x000000,                   // CARET_COLOR（浅色方案未覆盖，取默认黑）
    editor_selection: 0xa6d2ff,        // expUI_lightScheme.xml / SELECTION_BACKGROUND
    popover: 0xffffff,                 // popup-bg = layer-2-bg
    popup_border: 0xd1d3d9,            // popup-border-windows = gray-130
    separator: 0x00000020,             // toolbar-border = transparent-black-30
    feedback: 0xffffff,                // feedback-bg = layer-2-bg
    scrollbar_thumb: 0x73737333,       // ScrollBar.Transparent.thumbColor
    scrollbar_thumb_hover: 0x73737347, // ScrollBar.Transparent.hoverThumbColor
    island_arc: 16.,                   // Island.compactArc（COMPACT 密度）
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
    hover: 0xffffff10,                 // transparent-white-10
    control_hover: 0xffffff17,         // transparent-white-20
    tool_window_header: 0x191a1c,      // tool-window-bg(layer-0)
    tab_underline: 0x2e4d89,           // blue-60 / tab-selected-border-active
    tab_background: 0x191a1c,          // editor-bg
    tab_selected_background: 0x233558, // blue-40
    status_bar: 0x26282c,              // main-window-bg(layer-1)
    brand: 0x3871e1,                   // blue-80
    brand_hover: 0x2f5eb9,             // blue-70
    brand_active: 0x2e4d89,            // blue-60
    caret: 0xced0d6,                   // CARET_COLOR（expUI_darkScheme.xml）
    editor_selection: 0x214283,        // Darcula 配色方案的 SELECTION_BACKGROUND
    popover: 0x26282c,                 // popup-bg = layer-1-bg
    popup_border: 0x33353b,            // popup-border = layer-1-border(gray-40)
    separator: 0xffffff21,             // toolbar-border = transparent-white-30
    feedback: 0x33353b,                // feedback-bg = layer-2-bg(gray-40)
    scrollbar_thumb: 0xa6a6a647,       // ScrollBar.Transparent.thumbColor
    scrollbar_thumb_hover: 0xa6a6a659, // ScrollBar.Transparent.hoverThumbColor
    island_arc: 16.,                   // Island.compactArc（COMPACT 密度）
    dark_icons: true,
};

/// 把 `0xRRGGBB` 转成 GPUI 的 `Hsla`。
pub fn color(value: u32) -> Hsla {
    let r = ((value >> 16) & 0xff) as f32 / 255.0;
    let g = ((value >> 8) & 0xff) as f32 / 255.0;
    let b = (value & 0xff) as f32 / 255.0;
    rgb_to_hsla(r, g, b)
}

/// 把 `0xRRGGBBAA` 转成带透明度的 `Hsla`，用于 Islands 的悬停覆盖色。
pub fn color_rgba(value: u32) -> Hsla {
    let mut hsla = color(value >> 8);
    hsla.alpha = (value & 0xff) as f32 / 255.0;
    hsla
}

fn rgb_to_hsla(r: f32, g: f32, b: f32) -> Hsla {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) / 2.0;
    let delta = max - min;
    if delta <= f32::EPSILON {
        return hsla(0.0, 0.0, lightness, 1.0);
    }

    let saturation = if lightness > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };

    let hue = if (max - r).abs() < f32::EPSILON {
        ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() < f32::EPSILON {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };

    hsla(
        (hue * 60.0).rem_euclid(360.0) / 360.0,
        saturation,
        lightness,
        1.0,
    )
}

/// 把 IDEA 颜色写入 gpui-component 的全局主题。
///
/// 先让组件库切到对应的明暗模式（输入框、菜单、滚动条据此取色），
/// 再用 IDEA 令牌覆盖窗口主体颜色。
pub fn apply(cx: &mut App, kind: ThemeKind) {
    Theme::change(
        if kind.is_dark() {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        None,
        cx,
    );

    let tokens = kind.tokens();
    {
        let theme = Theme::global_mut(cx);
        theme.font_family = ui_font_family().into();
        theme.font_size = px(13.);
        theme.mono_font_family = "JetBrains Mono".into();
        // IDEA 在 Windows 上的编辑器默认字体是 JetBrains Mono 13px。
        theme.mono_font_size = px(13.);
        // 按钮与弹层的圆角是 8px（`Button.arc`、`PopupMenu.borderCornerRadius`）。
        // 输入框在 COMPACT 密度下是 6px（`Component.arc.compact`），
        // 组件库共用同一个 `radius`，输入框在使用处单独覆盖。
        theme.radius = px(8.);
        theme.radius_lg = px(8.);

        theme.background = color(tokens.canvas);
        theme.foreground = color(tokens.foreground);
        // 编辑器的缩进参考线直接复用 `theme.border` 取色。
        theme.border = color(tokens.indent_guide);

        theme.sidebar = color(tokens.tool_window_header);
        theme.sidebar_foreground = color(tokens.foreground);
        theme.sidebar_border = color(tokens.border);

        theme.muted_foreground = color(tokens.muted);
        theme.primary = color(tokens.brand);
        theme.primary_foreground = color(0xffffff);
        theme.primary_hover = color(tokens.brand_hover);
        theme.primary_active = color(tokens.brand_active);
        theme.accent = color(tokens.selection);
        theme.accent_foreground = color(tokens.selection_fg);
        theme.selection = color(tokens.editor_selection);
        theme.caret = color(tokens.caret);

        // Primary 按钮的背景是 `Button.default.startBackground`（control-brand-bg）。
        theme.button_primary = color(tokens.brand);
        theme.button_primary_hover = color(tokens.brand_hover);
        theme.button_primary_active = color(tokens.brand_active);
        theme.button_primary_foreground = color(0xffffff);
        theme.tokens.button_primary = color(tokens.brand).into();
        theme.tokens.button_primary_hover = color(tokens.brand_hover).into();
        theme.tokens.button_primary_active = color(tokens.brand_active).into();
        theme.tokens.button_primary_foreground = color(0xffffff).into();
        theme.tokens.selection = color(tokens.editor_selection).into();
        theme.tokens.caret = color(tokens.caret).into();

        // 弹层背景取 `popup-bg`，正文色沿用 `text-default`。
        theme.popover = color(tokens.popover);
        theme.popover_foreground = color(tokens.foreground);
        theme.tokens.popover = color(tokens.popover).into();
        theme.tokens.popover_foreground = color(tokens.foreground).into();

        // 新 UI 的滚动条是透明的轨道 + 半透明滑块（`ScrollBar.Transparent.*`）。
        theme.scrollbar = hsla(0., 0., 0., 0.);
        theme.scrollbar_thumb = color_rgba(tokens.scrollbar_thumb);
        theme.scrollbar_thumb_hover = color_rgba(tokens.scrollbar_thumb_hover);
        theme.tokens.scrollbar = theme.scrollbar.into();
        theme.tokens.scrollbar_thumb = theme.scrollbar_thumb.into();
        theme.tokens.scrollbar_thumb_hover = theme.scrollbar_thumb_hover.into();
        theme.highlight_theme = highlight_theme(kind);
    }
    Theme::sync_base(cx);
}

fn ui_font_family() -> &'static str {
    // 随程序打包的 Inter，与 IDEA 新 UI 一致。
    "Inter"
}

/// IDEA 顶部工具栏的项目色渐变，来自 `RecentProject.ColorN.MainToolbarGradientStart`。
pub fn project_gradient(kind: ThemeKind, seed: &str) -> Hsla {
    const LIGHT: [u32; 9] = [
        0xf7d7cb, 0xf5e6b8, 0xe2f2c9, 0xcbe4f7, 0xd4d2fc, 0xecd3f5, 0xe4d3f5, 0xcef0e9, 0xceedd4,
    ];
    const DARK: [u32; 9] = [
        0x664832, 0x5e4724, 0x45522f, 0x335661, 0x394773, 0x613861, 0x503d70, 0x325959, 0x365439,
    ];
    let palette = if kind.is_dark() { &DARK } else { &LIGHT };
    color(palette[project_color_index(seed)])
}

/// 项目徽标的底色。
///
/// 徽标用的是 `RecentProjectColorPalette` 里的高饱和色（主题文件只为渐变提供了
/// 浅色覆盖，徽标仍取代码里的默认值）。
pub fn project_badge(kind: ThemeKind, seed: &str) -> Hsla {
    const LIGHT: [u32; 9] = [
        0xdb3d3c, 0xf57236, 0x2bc8bb, 0x359af2, 0x8379fb, 0x7e54b5, 0xd63cc8, 0x954294, 0xe75371,
    ];
    const DARK: [u32; 9] = [
        0xce443c, 0xe27237, 0x2dbcad, 0x3895e1, 0x7b75e8, 0x7854ad, 0x8f4593, 0xc840b9, 0xd75370,
    ];
    let palette = if kind.is_dark() { &DARK } else { &LIGHT };
    color(palette[project_color_index(seed)])
}

/// 由项目路径算出的调色板下标，同一项目在工具栏渐变与徽标上保持一致。
fn project_color_index(seed: &str) -> usize {
    seed.bytes().fold(0u32, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as u32)
    }) as usize
        % 9
}

/// 编辑器配色，取自 IDEA 的 `expUI_lightScheme.xml`。
const LIGHT_SYNTAX: &str = r##"
{
  "name": "Keel Light",
  "appearance": "light",
  "style": {
    "editor.background": "#FFFFFF",
    "editor.foreground": "#000000",
    "editor.active_line.background": "#F5F8FE",
    "editor.line_number": "#AEB3C2",
    "editor.active_line_number": "#767A8A",
    "editor.gutter.background": "#FFFFFF",
    "syntax": {
      "keyword": { "color": "#0033B3" },
      "string": { "color": "#067D17" },
      "string.escape": { "color": "#0037A6" },
      "number": { "color": "#1750EB" },
      "comment": { "color": "#8C8C8C", "font_style": "italic" },
      "comment.doc": { "color": "#8C8C8C", "font_style": "italic" },
      "constant": { "color": "#871094", "font_style": "italic" },
      "function": { "color": "#00627A" },
      "type": { "color": "#000000" },
      "variable": { "color": "#000000" },
      "property": { "color": "#871094" },
      "attribute": { "color": "#9E880D" },
      "operator": { "color": "#000000" },
      "punctuation": { "color": "#000000" }
    }
  }
}
"##;

/// 编辑器配色，取自 IDEA 的 `expUI_darkScheme.xml`。
const DARK_SYNTAX: &str = r##"
{
  "name": "Keel Dark",
  "appearance": "dark",
  "style": {
    "editor.background": "#191A1C",
    "editor.foreground": "#BCBEC4",
    "editor.active_line.background": "#1F2024",
    "editor.line_number": "#4B5059",
    "editor.active_line_number": "#A1A3AB",
    "editor.gutter.background": "#191A1C",
    "syntax": {
      "keyword": { "color": "#CF8E6D" },
      "string": { "color": "#6AAB73" },
      "string.escape": { "color": "#2AACB8" },
      "number": { "color": "#2AACB8" },
      "comment": { "color": "#7A7E85", "font_style": "italic" },
      "comment.doc": { "color": "#7A7E85", "font_style": "italic" },
      "constant": { "color": "#C77DBB", "font_style": "italic" },
      "function": { "color": "#56A8F5" },
      "type": { "color": "#D1D3D9" },
      "variable": { "color": "#D1D3D9" },
      "property": { "color": "#C77DBB" },
      "attribute": { "color": "#BABABA" },
      "operator": { "color": "#D1D3D9" },
      "punctuation": { "color": "#D1D3D9" }
    }
  }
}
"##;

fn highlight_theme(kind: ThemeKind) -> Arc<HighlightTheme> {
    let json = if kind.is_dark() {
        DARK_SYNTAX
    } else {
        LIGHT_SYNTAX
    };
    serde_json::from_str(json).unwrap_or_else(|_| {
        if kind.is_dark() {
            HighlightTheme::default_dark()
        } else {
            HighlightTheme::default_light()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 十六进制颜色转换保持明度与饱和度() {
        let white = color(0xffffff);
        assert!((white.color.lightness - 1.0).abs() < 0.001);
        assert!((white.color.saturation - 0.0).abs() < 0.001);

        let black = color(0x000000);
        assert!((black.color.lightness - 0.0).abs() < 0.001);

        let panel = color(LIGHT.canvas);
        assert!((panel.color.lightness - 0.92).abs() < 0.03);
        assert!(panel.color.saturation < 0.3);
    }

    #[test]
    fn 蓝色系颜色落在预期色相范围() {
        for tokens in [&LIGHT, &DARK] {
            let selection = color(tokens.selection);
            let hue_degrees = selection.color.hue.into_degrees().rem_euclid(360.0);
            assert!(
                hue_degrees > 190.0 && hue_degrees < 230.0,
                "hue={hue_degrees}"
            );
        }
    }

    #[test]
    fn 深色主题背景比前景暗() {
        for tokens in [&LIGHT, &DARK] {
            let content = color(tokens.content).color.lightness;
            let foreground = color(tokens.foreground).color.lightness;
            assert!(
                (content - foreground).abs() > 0.3,
                "content={content} foreground={foreground}"
            );
        }
        assert!(color(DARK.content).color.lightness < 0.3);
        assert!(color(DARK.foreground).color.lightness > 0.6);
    }
}

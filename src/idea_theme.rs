use gpui::{App, Hsla, hsla, px};
use gpui_component::Theme;
use gpui_component::theme::ThemeMode;

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

/// 一套主题的颜色令牌，取值全部来自 IDEA 新 UI（expUI）主题 JSON。
pub struct Tokens {
    /// `ToolWindow.background`
    pub panel: u32,
    /// `colors.contentBackground`
    pub content: u32,
    /// `colors.foreground`
    pub foreground: u32,
    /// `colors.border`（新 UI 用 Gray12 / Gray3）
    pub border: u32,
    /// 状态栏、次要文字（Gray6 / Gray9）
    pub muted: u32,
    /// 面包屑等更次要的文字（Gray7）
    pub secondary: u32,
    /// `colors.selectionBackground`
    pub selection: u32,
    /// `colors.selectionForeground`
    pub selection_fg: u32,
    /// `colors.hoverBackground`
    pub tree_hover: u32,
    /// 控件（按钮、侧边条、状态栏部件）的中性悬停底色（Gray12 / Gray3）
    pub control_hover: u32,
    /// `ToolWindow.Header.background`
    pub tool_window_header: u32,
    /// `EditorTabs.underlineColor`
    pub tab_underline: u32,
    /// `EditorTabs.background`
    pub tab_background: u32,
    /// `StatusBar.background`
    pub status_bar: u32,
    /// 图标资源是否使用 `_dark` 变体。
    pub dark_icons: bool,
}

/// IntelliJ Light 主题令牌。
///
/// 取值来自 intellij-community 源码：
/// `platform/platform-resources/src/themes/expUI/expUI_light.theme.json`
/// 及其调色板 Gray1..Gray14、Blue1..Blue13。
pub const LIGHT: Tokens = Tokens {
    panel: 0xf7f8fa,      // Gray13
    content: 0xffffff,    // Gray14
    foreground: 0x000000, // Gray1
    border: 0xebecf0,     // Gray12
    muted: 0x6c707e,      // Gray6
    secondary: 0x818594,  // Gray7
    selection: 0xd4e2ff,  // Blue11
    selection_fg: 0x000000,
    tree_hover: 0xedf3ff,    // Blue12
    control_hover: 0xebecf0, // Gray12
    tool_window_header: 0xf7f8fa,
    tab_underline: 0x3574f0, // Blue4
    tab_background: 0xffffff,
    status_bar: 0xf7f8fa,
    dark_icons: false,
};

/// Darcula（expUI Dark）主题令牌。
///
/// 取值来自 `expUI_dark.theme.json` 的 Gray/Blue 调色板。
pub const DARK: Tokens = Tokens {
    panel: 0x2b2d30,      // Gray2
    content: 0x1e1f22,    // Gray1
    foreground: 0xdfe1e5, // Gray12
    border: 0x393b40,     // Gray3，用于面板与编辑器之间的分隔线
    muted: 0x9da0a8,      // Gray9
    secondary: 0x6f737a,  // Gray7
    selection: 0x2e436e,  // Blue2
    selection_fg: 0xdfe1e5,
    tree_hover: 0x393b40,    // Gray3
    control_hover: 0x393b40, // Gray3
    tool_window_header: 0x2b2d30,
    tab_underline: 0x3574f0, // Blue6
    tab_background: 0x1e1f22,
    status_bar: 0x2b2d30,
    dark_icons: true,
};

/// 把 `0xRRGGBB` 转成 GPUI 的 `Hsla`。
pub fn color(value: u32) -> Hsla {
    let r = ((value >> 16) & 0xff) as f32 / 255.0;
    let g = ((value >> 8) & 0xff) as f32 / 255.0;
    let b = (value & 0xff) as f32 / 255.0;
    rgb_to_hsla(r, g, b)
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
        theme.mono_font_size = px(13.);
        // IDEA 新 UI 的圆角基准值是 4px。
        theme.radius = px(4.);
        theme.radius_lg = px(6.);

        theme.background = color(tokens.content);
        theme.foreground = color(tokens.foreground);
        theme.border = color(tokens.border);

        theme.sidebar = color(tokens.panel);
        theme.sidebar_foreground = color(tokens.foreground);
        theme.sidebar_border = color(tokens.border);

        theme.muted_foreground = color(tokens.muted);
        theme.primary = color(tokens.tab_underline);
        theme.primary_foreground = color(0xffffff);
        theme.accent = color(tokens.tree_hover);
        theme.accent_foreground = color(tokens.foreground);
    }
    Theme::sync_base(cx);
}

fn ui_font_family() -> &'static str {
    // 随程序打包的 Inter，与 IDEA 新 UI 一致。
    "Inter"
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

        let panel = color(LIGHT.panel);
        assert!((panel.color.lightness - 0.974).abs() < 0.01);
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

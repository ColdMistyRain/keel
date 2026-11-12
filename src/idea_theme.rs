use gpui::{App, Hsla, hsla};
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

/// 一套主题的颜色令牌，取值全部来自 IDEA 的主题 JSON。
pub struct Tokens {
    /// `colors.panel`
    pub panel: u32,
    /// `colors.contentBackground`
    pub content: u32,
    /// `colors.foreground`
    pub foreground: u32,
    /// `colors.border`
    pub border: u32,
    /// `colors.grey03`，禁用/次要文字
    pub muted: u32,
    /// `ui.*.selectionBackground`
    pub selection: u32,
    /// `ui.Tree.hoverBackground`
    pub tree_hover: u32,
    /// `ui.ToolWindow.Header.background`
    pub tool_window_header: u32,
    /// `ui.EditorTabs.underlineColor`
    pub tab_underline: u32,
}

/// IntelliJ Light 主题令牌。
///
/// 取值来自 intellij-community 源码：
/// `platform/platform-resources/src/themes/intellijlaf.theme.json`
/// 与 `Light.theme.json`（后者覆盖 ToolWindow.background = #FFFFFF）。
pub const LIGHT: Tokens = Tokens {
    panel: 0xf2f2f2,
    content: 0xffffff,
    foreground: 0x000000,
    border: 0xd1d1d1,
    muted: 0x8c8c8c,
    selection: 0x2675bf,
    tree_hover: 0xedf5fc,
    tool_window_header: 0xe2e6ec,
    tab_underline: 0x4083c9,
};

/// Darcula 主题令牌。
///
/// 取值来自 intellij-community 源码：
/// `platform/platform-resources/src/themes/darcula.theme.json`
/// （contentBackground #2B2B2B、panel #3C3F41、border #323232、
/// selectionBackground #214283、grey03 #7A7A7A）。
pub const DARK: Tokens = Tokens {
    panel: 0x3c3f41,
    content: 0x2b2b2b,
    foreground: 0xbbbbbb,
    border: 0x323232,
    muted: 0x7a7a7a,
    selection: 0x214283,
    tree_hover: 0x45494a,
    tool_window_header: 0x3c3f41,
    tab_underline: 0x4a88c7,
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

        theme.background = color(tokens.content);
        theme.foreground = color(tokens.foreground);
        theme.border = color(tokens.border);

        theme.sidebar = color(tokens.panel);
        theme.sidebar_foreground = color(tokens.foreground);
        theme.sidebar_border = color(tokens.border);

        theme.muted_foreground = color(tokens.muted);
        theme.primary = color(tokens.selection);
        theme.primary_foreground = color(if kind.is_dark() {
            0xffffff
        } else {
            tokens.content
        });
        theme.accent = color(tokens.tree_hover);
        theme.accent_foreground = color(tokens.foreground);
    }
    Theme::sync_base(cx);
}

fn ui_font_family() -> &'static str {
    if cfg!(target_os = "windows") {
        "Microsoft YaHei UI"
    } else if cfg!(target_os = "macos") {
        "PingFang SC"
    } else {
        "Noto Sans CJK SC"
    }
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
        assert!((panel.color.lightness - 0.949).abs() < 0.01);
        assert!((panel.color.saturation - 0.0).abs() < 0.001);
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

use gpui::{App, Hsla, hsla};
use gpui_component::Theme;

/// IntelliJ Light 主题令牌。
///
/// 取值来自 intellij-community 源码：
/// `platform/platform-resources/src/themes/intellijlaf.theme.json`
/// 与 `Light.theme.json`（后者覆盖 ToolWindow.background = #FFFFFF）。
pub mod token {
    /// `colors.panel`
    pub const PANEL: u32 = 0xf2f2f2;
    /// `colors.contentBackground`
    pub const CONTENT: u32 = 0xffffff;
    /// `colors.foreground`
    pub const FOREGROUND: u32 = 0x000000;
    /// `colors.border`
    pub const BORDER: u32 = 0xd1d1d1;
    /// `colors.grey03`，禁用/次要文字
    pub const MUTED: u32 = 0x8c8c8c;
    /// `ui.*.selectionBackground`
    pub const SELECTION: u32 = 0x2675bf;
    /// `ui.Tree.hoverBackground`
    pub const TREE_HOVER: u32 = 0xedf5fc;
    /// `ui.ToolWindow.Header.background`
    pub const TOOL_WINDOW_HEADER: u32 = 0xe2e6ec;
    /// `ui.EditorTabs.underlineColor`
    pub const TAB_UNDERLINE: u32 = 0x4083c9;
}

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
pub fn apply(cx: &mut App) {
    {
        let theme = Theme::global_mut(cx);
        theme.font_family = ui_font_family().into();

        theme.background = color(token::CONTENT);
        theme.foreground = color(token::FOREGROUND);
        theme.border = color(token::BORDER);

        theme.sidebar = color(token::PANEL);
        theme.sidebar_foreground = color(token::FOREGROUND);
        theme.sidebar_border = color(token::BORDER);

        theme.muted_foreground = color(token::MUTED);
        theme.primary = color(token::SELECTION);
        theme.primary_foreground = color(token::CONTENT);
        theme.accent = color(token::TREE_HOVER);
        theme.accent_foreground = color(token::FOREGROUND);
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

        let panel = color(token::PANEL);
        assert!((panel.color.lightness - 0.949).abs() < 0.01);
        assert!((panel.color.saturation - 0.0).abs() < 0.001);
    }

    #[test]
    fn 蓝色系颜色落在预期色相范围() {
        let selection = color(token::SELECTION);
        let hue_degrees = selection.color.hue.into_degrees().rem_euclid(360.0);
        assert!(
            hue_degrees > 190.0 && hue_degrees < 230.0,
            "hue={hue_degrees}"
        );
    }
}

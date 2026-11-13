use gpui::{App, Hsla, hsla, px};
use gpui_component::Theme;
use gpui_component::highlighter::HighlightTheme;
use gpui_component::theme::ThemeMode;
use std::sync::Arc;

/// 主题令牌（IDEA Islands 的数值）来自 `keel-theme`，这里只负责转成 GPUI 类型。
pub use keel_theme::ThemeKind;

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
        // 通用控件的圆角半径 = 4：IDEA 的 `Button.arc`(8)、`Component.arc`(8)、
        // `PopupMenu.Selection.arc`(8) 都是传给 `RoundRectangle2D` 的**弧直径**，
        // 实际半径是一半（`RectanglePainter2D` / `DarculaButtonPainter`）。
        // 组件库的 `radius` 是半径，直接取 4 才能和 IDEA 的按钮、输入框、
        // 菜单选中块一致。
        theme.radius = px(tokens.radius);
        // 弹层/对话框表面的圆角仍是 8（`PopupMenu.borderCornerRadius`），
        // 与控件内部高亮的 4 区分开。
        theme.radius_lg = px(tokens.island_radius);

        theme.background = color(tokens.canvas);
        theme.foreground = color(tokens.foreground);
        // 编辑器的缩进参考线直接复用 `theme.border` 取色。
        theme.border = color(tokens.indent_guide);

        theme.sidebar = color(tokens.tool_window_header);
        theme.sidebar_foreground = color(tokens.foreground);
        theme.sidebar_border = color(tokens.border);

        theme.muted_foreground = color(tokens.muted);
        theme.primary = color(tokens.brand);
        theme.primary_foreground = color(tokens.on_brand);
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
        theme.button_primary_foreground = color(tokens.on_brand);
        theme.tokens.button_primary = color(tokens.brand).into();
        theme.tokens.button_primary_hover = color(tokens.brand_hover).into();
        theme.tokens.button_primary_active = color(tokens.brand_active).into();
        theme.tokens.button_primary_foreground = color(tokens.on_brand).into();
        // 组件库的菜单项/列表项用 `tokens.accent` 画悬停与选中块
        // （`MenuItemElement` 读的是 `tokens.accent`，不是 `theme.accent`）。
        // 不覆盖时会落回组件库自带的灰色 accent，而不是 IDEA 的蓝色
        // `selection-bg-active`。`primary` 同样用于单选/复选框指示器。
        theme.tokens.primary = color(tokens.brand).into();
        theme.tokens.primary_hover = color(tokens.brand_hover).into();
        theme.tokens.primary_active = color(tokens.brand_active).into();
        theme.tokens.primary_foreground = color(tokens.on_brand).into();
        theme.tokens.accent = color(tokens.selection).into();
        theme.tokens.accent_foreground = color(tokens.selection_fg).into();
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
    let palette = &kind.tokens().project_gradient;
    color(
        palette
            .get(project_color_index(seed))
            .copied()
            .unwrap_or(kind.tokens().brand),
    )
}

/// 项目徽标的底色，来自 `RecentProjectColorPalette`。
pub fn project_badge(kind: ThemeKind, seed: &str) -> Hsla {
    let palette = &kind.tokens().project_badge;
    color(
        palette
            .get(project_color_index(seed))
            .copied()
            .unwrap_or(kind.tokens().brand),
    )
}

/// 由项目路径算出的调色板下标，同一项目在工具栏渐变与徽标上保持一致。
fn project_color_index(seed: &str) -> usize {
    seed.bytes().fold(0u32, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(byte as u32)
    }) as usize
        % 9
}

/// 编辑器配色来自 `themes/idea-*.json` 的 `syntax` 节点。
fn highlight_theme(kind: ThemeKind) -> Arc<HighlightTheme> {
    let json = keel_theme::theme(kind).syntax_json();
    serde_json::from_str(&json).unwrap_or_else(|_| {
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

        let panel = color(ThemeKind::Light.tokens().canvas);
        assert!((panel.color.lightness - 0.92).abs() < 0.03);
        assert!(panel.color.saturation < 0.3);
    }

    #[test]
    fn 蓝色系颜色落在预期色相范围() {
        for kind in [ThemeKind::Light, ThemeKind::Dark] {
            let selection = color(kind.tokens().selection);
            let hue_degrees = selection.color.hue.into_degrees().rem_euclid(360.0);
            assert!(
                hue_degrees > 190.0 && hue_degrees < 230.0,
                "hue={hue_degrees}"
            );
        }
    }

    #[test]
    fn 深色主题背景比前景暗() {
        for kind in [ThemeKind::Light, ThemeKind::Dark] {
            let tokens = kind.tokens();
            let content = color(tokens.content).color.lightness;
            let foreground = color(tokens.foreground).color.lightness;
            assert!(
                (content - foreground).abs() > 0.3,
                "content={content} foreground={foreground}"
            );
        }
        let dark = ThemeKind::Dark.tokens();
        assert!(color(dark.content).color.lightness < 0.3);
        assert!(color(dark.foreground).color.lightness > 0.6);
    }
}

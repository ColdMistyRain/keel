//! Keel 的主题配置加载：IDEA Islands 的全部颜色与度量都在
//! `themes/idea-light.json`、`themes/idea-dark.json`，Rust 里不再写死任何色值。
//!
//! - GPUI 版本：`src/idea_theme.rs` 读取 `ThemeKind::tokens()`，把 `u32` 转成 `Hsla`；
//! - nsh 版本：`nsh_theme_json()` 输出 shadcn `cssVars`，运行时交给 nsh 的
//!   `set_tokens`。
//!
//! 两个 UI 共用同一份配置文件，但都不依赖对方。

use std::collections::HashMap;
use std::sync::OnceLock;

const LIGHT_JSON: &str = include_str!("../../../themes/idea-light.json");
const DARK_JSON: &str = include_str!("../../../themes/idea-dark.json");

/// 支持的 IDEA 主题。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeKind {
    #[default]
    Light,
    Dark,
}

impl ThemeKind {
    pub fn is_dark(self) -> bool {
        self == ThemeKind::Dark
    }

    /// 这套主题的令牌（来自配置文件）。
    pub fn tokens(self) -> &'static Tokens {
        theme(self).tokens()
    }
}

/// 一套解析后的主题。
pub struct Theme {
    name: String,
    dark: bool,
    tokens: Tokens,
    /// 配置里的原始颜色字符串（`#RRGGBB` / `#RRGGBBAA`），供 nsh 的 cssVars 输出。
    raw: HashMap<String, String>,
    /// 编辑器语法配色的原始对象（gpui-component 的 HighlightTheme 格式）。
    syntax: serde_json::Value,
}

impl Theme {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_dark(&self) -> bool {
        self.dark
    }

    pub fn tokens(&self) -> &Tokens {
        &self.tokens
    }

    /// 令牌 `key` 的 CSS 颜色（大写、带 `#`）。
    pub fn css(&self, key: &str) -> String {
        css(self.raw.get(key).map(String::as_str).unwrap_or("#000000"))
    }

    /// 编辑器语法配色（gpui-component `HighlightTheme` 的 JSON）。
    pub fn syntax_json(&self) -> String {
        serde_json::to_string(&self.syntax).unwrap_or_else(|_| "{}".to_owned())
    }
}

/// 主题的数值令牌。
///
/// 6 位颜色保持 `0xRRGGBB`，带透明度的 8 位颜色是 `0xRRGGBBAA`，
/// 与 GPUI 的 `color` / `color_rgba` 约定一致；nsh 侧直接使用配置里的原始字符串。
pub struct Tokens {
    pub canvas: u32,
    pub content: u32,
    pub foreground: u32,
    pub border: u32,
    pub indent_guide: u32,
    pub muted: u32,
    pub secondary: u32,
    pub selection: u32,
    pub selection_fg: u32,
    pub hover: u32,
    pub control_hover: u32,
    /// 按下态叠加色（条纹按钮的“激活但未聚焦”也用这个）。
    pub control_pressed: u32,
    pub tool_window_header: u32,
    pub tab_underline: u32,
    pub tool_window_tab_underline: u32,
    pub tab_background: u32,
    pub tab_selected_background: u32,
    /// 选中编辑器标签的 1px 边框（IDEA `EditorTabs.underlinedBorderColor`）。
    pub tab_selected_border: u32,
    /// 选中但编辑器没有焦点时的标签底色与边框。
    pub tab_selected_background_inactive: u32,
    pub tab_selected_border_inactive: u32,
    /// 标签条底部的 1px 分隔线（IDEA `EditorTabs.underTabsBorderColor`）。
    pub editor_border: u32,
    /// 弹出菜单分隔线（IDEA 菜单里的 `gray-150`）。
    pub menu_separator: u32,
    /// 文本输入框（含主工具栏搜索框）的背景与边框（IDEA `control-bg` / `control-border`）。
    pub control_bg: u32,
    pub control_border: u32,
    pub status_bar: u32,
    pub brand: u32,
    pub brand_hover: u32,
    pub brand_active: u32,
    /// 运行 / 调试按钮的绿色与停止按钮的红色（IDEA `RunWidget`）。
    pub run_background: u32,
    pub stop_background: u32,
    /// 品牌色上的文字 / 图标。
    pub on_brand: u32,
    pub caret: u32,
    pub editor_selection: u32,
    pub popover: u32,
    pub popup_border: u32,
    pub separator: u32,
    pub separator_line: u32,
    /// 模态浮层后面的遮罩（命令面板、对话框）。
    pub overlay: u32,
    pub feedback: u32,
    pub feedback_border: u32,
    pub scrollbar_thumb: u32,
    pub scrollbar_thumb_hover: u32,
    pub destructive: u32,
    pub active_line: u32,
    pub line_number: u32,
    pub active_line_number: u32,
    pub unselected_blend: f32,
    pub unselected_alpha: f32,
    pub island_arc: f32,
    /// 控件圆角（按钮、输入框）。
    pub radius: f32,
    /// 岛屿 / 弹层圆角。
    pub island_radius: f32,
    /// 岛屿之间的帧背景间隙。
    pub island_gap: f32,
    pub tab_height: f32,
    pub status_bar_height: f32,
    pub dark_icons: bool,
    /// 最近项目工具栏渐变与徽标的备选色。
    pub project_gradient: Vec<u32>,
    pub project_badge: Vec<u32>,
}

static LIGHT: OnceLock<Theme> = OnceLock::new();
static DARK: OnceLock<Theme> = OnceLock::new();

/// 读取一套主题文件（编译期内嵌；文件缺失或格式错误会直接 panic，由测试兜住）。
pub fn theme(kind: ThemeKind) -> &'static Theme {
    match kind {
        ThemeKind::Light => LIGHT.get_or_init(|| parse(LIGHT_JSON)),
        ThemeKind::Dark => DARK.get_or_init(|| parse(DARK_JSON)),
    }
}

fn parse(json: &str) -> Theme {
    let value: serde_json::Value =
        serde_json::from_str(json).expect("themes/idea-*.json 不是合法 JSON");
    let name = value["name"].as_str().unwrap_or("Keel").to_owned();
    let dark = value["dark"].as_bool().unwrap_or(false);
    let colors = &value["colors"];
    let metrics = &value["metrics"];
    let mut raw = HashMap::new();
    if let Some(entries) = colors.as_object() {
        for (key, value) in entries {
            if let Some(text) = value.as_str() {
                raw.insert(key.clone(), text.to_owned());
            }
        }
    }

    let color = |key: &str| parse_color(raw.get(key).map(String::as_str).unwrap_or("#000000"));
    let number = |key: &str| metrics[key].as_f64().unwrap_or(0.0) as f32;
    let flag = |key: &str| metrics[key].as_bool().unwrap_or(false);
    let palette = |key: &str| {
        value["palettes"][key]
            .as_array()
            .map(|colors| {
                colors
                    .iter()
                    .filter_map(|color| color.as_str())
                    .map(parse_color)
                    .collect()
            })
            .unwrap_or_default()
    };

    let tokens = Tokens {
        canvas: color("canvas"),
        content: color("content"),
        foreground: color("foreground"),
        border: color("border"),
        indent_guide: color("indent_guide"),
        muted: color("muted"),
        secondary: color("secondary"),
        selection: color("selection"),
        selection_fg: color("selection_fg"),
        hover: color("hover"),
        control_hover: color("control_hover"),
        control_pressed: color("control_pressed"),
        tool_window_header: color("tool_window_header"),
        tab_underline: color("tab_underline"),
        tool_window_tab_underline: color("tool_window_tab_underline"),
        tab_background: color("tab_background"),
        tab_selected_background: color("tab_selected_background"),
        tab_selected_border: color("tab_selected_border"),
        tab_selected_background_inactive: color("tab_selected_background_inactive"),
        tab_selected_border_inactive: color("tab_selected_border_inactive"),
        editor_border: color("editor_border"),
        menu_separator: color("menu_separator"),
        control_bg: color("control_bg"),
        control_border: color("control_border"),
        status_bar: color("status_bar"),
        brand: color("brand"),
        brand_hover: color("brand_hover"),
        brand_active: color("brand_active"),
        run_background: color("run_background"),
        stop_background: color("stop_background"),
        on_brand: color("on_brand"),
        caret: color("caret"),
        editor_selection: color("editor_selection"),
        popover: color("popover"),
        popup_border: color("popup_border"),
        separator: color("separator"),
        separator_line: color("separator_line"),
        overlay: color("overlay"),
        feedback: color("feedback"),
        feedback_border: color("feedback_border"),
        scrollbar_thumb: color("scrollbar_thumb"),
        scrollbar_thumb_hover: color("scrollbar_thumb_hover"),
        destructive: color("destructive"),
        active_line: color("active_line"),
        line_number: color("line_number"),
        active_line_number: color("active_line_number"),
        unselected_blend: number("unselected_blend"),
        unselected_alpha: number("unselected_alpha"),
        island_arc: number("island_arc"),
        radius: number("radius"),
        island_radius: number("island_radius"),
        island_gap: number("island_gap"),
        tab_height: number("tab_height"),
        status_bar_height: number("status_bar_height"),
        dark_icons: flag("dark_icons"),
        project_gradient: palette("project_gradient"),
        project_badge: palette("project_badge"),
    };

    Theme {
        name,
        dark,
        raw,
        syntax: value["syntax"].clone(),
        tokens,
    }
}

/// `#RRGGBB` → `0xRRGGBB`，`#RRGGBBAA` → `0xRRGGBBAA`。
fn parse_color(value: &str) -> u32 {
    let hex = value.trim().trim_start_matches('#');
    u32::from_str_radix(hex, 16).unwrap_or(0)
}

fn css(value: &str) -> String {
    format!(
        "#{}",
        value.trim().trim_start_matches('#').to_ascii_uppercase()
    )
}

/// 把两套 IDEA 配置转换成 nsh 的 shadcn `cssVars` 主题。
pub fn nsh_theme_json() -> String {
    let light = theme(ThemeKind::Light);
    let dark = theme(ThemeKind::Dark);
    let metrics = light.tokens();
    format!(
        r#"{{"cssVars":{{"theme":{{"radius":"{}px","font-sans":"Inter, Segoe UI, sans-serif","font-mono":"JetBrains Mono, Consolas, monospace","island-radius":"{}px","island-gap":"{}px","tab-height":"{}px","status-bar-height":"{}px"}},"light":{{{}}},"dark":{{{}}}}}}}"#,
        metrics.radius,
        metrics.island_radius,
        metrics.island_gap,
        metrics.tab_height,
        metrics.status_bar_height,
        nsh_vars(light),
        nsh_vars(dark)
    )
}

fn nsh_vars(t: &Theme) -> String {
    format!(
        concat!(
            r##""background":"{background}","foreground":"{foreground}","##,
            r##""card":"{content}","card-foreground":"{foreground}","##,
            r##""popover":"{popover}","popover-foreground":"{foreground}","##,
            r##""primary":"{brand}","primary-foreground":"{on_brand}","##,
            r##""secondary":"{canvas}","secondary-foreground":"{foreground}","##,
            r##""muted":"{feedback}","muted-foreground":"{muted}","##,
            r##""accent":"{selection}","accent-foreground":"{selection_fg}","##,
            r##""destructive":"{destructive}","border":"{separator}","input":"{popup_border}","ring":"{brand}","##,
            r##""sidebar":"{tool_window_header}","sidebar-foreground":"{foreground}","##,
            r##""sidebar-primary":"{brand}","sidebar-primary-foreground":"{on_brand}","##,
            r##""sidebar-accent":"{selection}","sidebar-accent-foreground":"{selection_fg}","##,
            r##""sidebar-border":"{border}","sidebar-ring":"{brand}","##,
            r##""selection":"{editor_selection}","##,
            r##""canvas":"{canvas}","island-bg":"{tool_window_header}","##,
            r##""editor-bg":"{content}","editor-fg":"{foreground}","header-bg":"{tool_window_header}","##,
            r##""tab-bg":"{tab_background}","tab-selected-bg":"{tab_selected_background}","##,
            r##""tab-selected-bg-inactive":"{tab_selected_background_inactive}","tab-selected-border":"{tab_selected_border}","##,
            r##""tab-selected-border-inactive":"{tab_selected_border_inactive}","editor-border":"{editor_border}","##,
            r##""menu-separator":"{menu_separator}","##,
            r##""control-bg":"{control_bg}","control-border":"{control_border}","##,
            r##""tab-underline":"{tab_underline}","status-bar":"{status_bar}","##,
            r##""tool-window-tab-underline":"{tool_window_tab_underline}","##,
            r##""muted-fg":"{muted}","secondary-fg":"{secondary}","##,
            r##""hover-overlay":"{hover}","control-hover":"{control_hover}","##,
            r##""control-pressed":"{control_pressed}","##,
            r##""popup-border":"{popup_border}","separator-line":"{separator_line}","overlay":"{overlay}","##,
            r##""feedback":"{feedback}","scrollbar-thumb":"{scrollbar_thumb}","##,
            r##""active-line":"{active_line}","line-number":"{line_number}","##,
            r##""active-line-number":"{active_line_number}","indent-guide":"{indent_guide}","##,
            r##""brand":"{brand}","brand-hover":"{brand_hover}","##,
            r##""run-background":"{run_background}","stop-background":"{stop_background}""##
        ),
        background = t.css("canvas"),
        foreground = t.css("foreground"),
        content = t.css("content"),
        popover = t.css("popover"),
        brand = t.css("brand"),
        run_background = t.css("run_background"),
        stop_background = t.css("stop_background"),
        on_brand = t.css("on_brand"),
        canvas = t.css("canvas"),
        feedback = t.css("feedback"),
        muted = t.css("muted"),
        selection = t.css("selection"),
        selection_fg = t.css("selection_fg"),
        destructive = t.css("destructive"),
        separator = t.css("separator"),
        popup_border = t.css("popup_border"),
        tool_window_header = t.css("tool_window_header"),
        border = t.css("border"),
        editor_selection = t.css("editor_selection"),
        tab_background = t.css("tab_background"),
        tab_selected_background = t.css("tab_selected_background"),
        tab_selected_background_inactive = t.css("tab_selected_background_inactive"),
        tab_selected_border = t.css("tab_selected_border"),
        tab_selected_border_inactive = t.css("tab_selected_border_inactive"),
        editor_border = t.css("editor_border"),
        menu_separator = t.css("menu_separator"),
        control_bg = t.css("control_bg"),
        control_border = t.css("control_border"),
        tab_underline = t.css("tab_underline"),
        tool_window_tab_underline = t.css("tool_window_tab_underline"),
        status_bar = t.css("status_bar"),
        secondary = t.css("secondary"),
        hover = t.css("hover"),
        control_hover = t.css("control_hover"),
        control_pressed = t.css("control_pressed"),
        separator_line = t.css("separator_line"),
        overlay = t.css("overlay"),
        scrollbar_thumb = t.css("scrollbar_thumb"),
        active_line = t.css("active_line"),
        line_number = t.css("line_number"),
        active_line_number = t.css("active_line_number"),
        indent_guide = t.css("indent_guide"),
        brand_hover = t.css("brand_hover"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_both_theme_files() {
        let light = theme(ThemeKind::Light);
        let dark = theme(ThemeKind::Dark);
        assert!(!light.is_dark());
        assert!(dark.is_dark());
        assert_ne!(light.tokens().canvas, dark.tokens().canvas);
        assert_eq!(light.tokens().canvas, 0xE9EAEE);
        assert_eq!(dark.tokens().canvas, 0x26282C);
        assert_eq!(light.tokens().brand, 0x3871E1);
        assert!(dark.tokens().dark_icons);
        assert_eq!(light.tokens().island_radius, 10.0);
        assert_eq!(light.tokens().island_gap, 6.0);
        assert_eq!(light.tokens().tab_height, 41.0);
        assert_eq!(light.tokens().tab_selected_background_inactive, 0xE9EAEE);
        assert_eq!(light.tokens().project_gradient.len(), 9);
        assert_eq!(dark.tokens().project_badge.len(), 9);
        assert!(light.syntax_json().contains("keyword"));
    }

    #[test]
    fn parses_colors_with_alpha() {
        assert_eq!(parse_color("#00000008"), 0x0000_0008);
        assert_eq!(parse_color("#FFFFFF10"), 0xFFFF_FF10);
        assert_eq!(parse_color("#F5F8FE"), 0xF5F8FE);
        assert_eq!(css("#f5f8fe"), "#F5F8FE");
        assert_eq!(css("#00000008"), "#00000008");
        assert_eq!(theme(ThemeKind::Light).css("canvas"), "#E9EAEE");
    }

    #[test]
    fn nsh_theme_has_both_schemes_and_custom_tokens() {
        let json = nsh_theme_json();
        assert!(json.contains("\"light\""));
        assert!(json.contains("\"dark\""));
        assert!(json.contains("\"editor-bg\""));
        assert!(json.contains("\"active-line\""));
        assert!(json.contains("\"island-gap\""));
        assert!(json.contains("\"tab-selected-bg-inactive\""));
        assert!(json.contains("#3871E1"));
    }

    #[test]
    fn nsh_theme_is_valid_json() {
        let json = nsh_theme_json();
        serde_json::from_str::<serde_json::Value>(&json).expect("nsh 主题必须是合法 JSON");
    }
}

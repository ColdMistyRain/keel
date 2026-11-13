//! 共用的小部件与主题取值。
//!
//! 组件只读主题令牌，不在 Rust 里写色值；主题来自 `themes/idea-*.json`。

use std::rc::Rc;

use nsh_base::{menu_item, menu_separator as base_menu_separator, theme};
use nsh_core::element::StyleRefinement;
use nsh_core::prelude::*;

/// 取主题令牌；缺失时退回透明，避免出现字面色值。
pub fn color_of(name: &str) -> AdaptiveColor {
    theme().color(name).unwrap_or(AdaptiveColor::TRANSPARENT)
}

/// 菜单与浮层的弹出样式。
pub fn popup_style(s: StyleRefinement) -> StyleRefinement {
    s.p_1()
        .rounded(px(8.0))
        .min_w(px(220.0))
        .bg(move || theme().popover)
        .border_1()
        .border_color(move || color_of("popup-border"))
        .shadow_md()
}

/// 带助记符的标签：`文件(F)`，括号里的字母带下划线；省略号排在助记符之后
/// （`打开项目(P)…`，与 IDEA 中文界面一致）。
pub fn mnemonic_label(label: impl Into<SharedString>, mnemonic: Option<char>) -> View {
    let label = label.into();
    let (body, ellipsis) = match label.as_ref().strip_suffix('…') {
        Some(body) => (SharedString::from(body.to_owned()), Some("…")),
        None => (label, None),
    };
    let mut row = view()
        .flex_row()
        .items_center()
        .child(text(body).text_size(px(13.0)));
    if let Some(key) = mnemonic {
        row = row
            .child(
                text("(")
                    .text_size(px(13.0))
                    .text_color(move || color_of("muted-fg")),
            )
            .child(
                text(SharedString::from(key.to_ascii_uppercase().to_string()))
                    .text_size(px(13.0))
                    .underline(),
            )
            .child(
                text(")")
                    .text_size(px(13.0))
                    .text_color(move || color_of("muted-fg")),
            );
    }
    if let Some(ellipsis) = ellipsis {
        row = row.child(text(ellipsis).text_size(px(13.0)));
    }
    row
}

/// 菜单项的图标槽：没有图标时也占 16px，保证文字对齐。
pub fn icon_slot(icon: Option<ImageAsset>) -> View {
    let mut slot = view()
        .w(px(16.0))
        .h(px(16.0))
        .items_center()
        .justify_center();
    if let Some(icon) = icon {
        slot = slot.child(
            svg(icon)
                .w(px(16.0))
                .h(px(16.0))
                .text_color(move || color_of("secondary-fg")),
        );
    }
    slot
}

/// 主工具栏左上角的汉堡按钮。
///
/// IDEA 新 UI 默认把主菜单收进这个按钮（`MainMenuButton`），图标
/// `AllIcons.General.WindowsMenu_20x20` 为 20×20，按钮最小 30×30，
/// 悬停用 `toolbar-bg-hovered`（`#00000012`）。
pub fn burger_button() -> View {
    view()
        .w(px(30.0))
        .h(px(30.0))
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .role(AccessibilityRole::Button)
        .label("主菜单")
        .hover(|s| s.bg(move || color_of("control-hover")))
        .child(
            svg(crate::icons::MENU)
                .w(px(20.0))
                .h(px(20.0))
                .text_color(move || color_of("muted-fg")),
        )
}

/// 主工具栏的 30×30 图标按钮（搜索、设置等，IDEA `MainToolbar.Icon`）。
pub fn toolbar_button(icon: ImageAsset, label: &'static str, action: impl Fn() + 'static) -> View {
    view()
        .w(px(30.0))
        .h(px(30.0))
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .role(AccessibilityRole::Button)
        .label(label)
        .hover(|s| s.bg(move || color_of("control-hover")))
        .on_press(action)
        .child(
            svg(icon)
                .w(px(20.0))
                .h(px(20.0))
                .text_color(move || color_of("muted-fg")),
        )
}

/// 子菜单的触发器：左标签、右箭头。
///
/// IDEA 的弹出菜单项高 26（`List.rowHeight 24 + Selection.outerInsets 1+1`），
/// 选中块的左右内缩 7、圆角 4（arc 8）。
pub fn submenu_trigger(label: impl Into<SharedString>, mnemonic: Option<char>) -> View {
    view()
        .h(px(26.0))
        .mx(px(7.0))
        .my(px(1.0))
        .px(px(6.0))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .rounded(px(4.0))
        .hover(|s| s.bg(move || theme().accent))
        .child(mnemonic_label(label, mnemonic).flex_1())
        .child(
            svg(crate::icons::CHEVRON_RIGHT)
                .w(px(12.0))
                .h(px(12.0))
                .text_color(move || color_of("muted-fg")),
        )
}

/// 菜单项：图标、左标签、右快捷键。
pub fn menu_action(
    icon: Option<ImageAsset>,
    label: impl Into<SharedString>,
    mnemonic: Option<char>,
    shortcut: Option<String>,
    action: impl Fn() + 'static,
) -> impl IntoElement {
    let shortcut = SharedString::from(shortcut.unwrap_or_default());
    menu_item()
        .on_select(action)
        .h(px(26.0))
        .mx(px(7.0))
        .my(px(1.0))
        .px(px(6.0))
        .rounded(px(4.0))
        .hover(|s| s.bg(move || theme().accent))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(icon_slot(icon))
                .child(mnemonic_label(label, mnemonic).flex_1())
                .child(
                    text(shortcut)
                        .text_size(px(12.0))
                        .text_color(move || color_of("muted-fg")),
                ),
        )
}

/// 菜单分隔线。IDEA 弹出菜单的分隔线是 `gray-150`，比工具窗口里的分隔线更浅。
pub fn menu_separator() -> impl IntoElement {
    base_menu_separator()
        .h(px(1.0))
        .mx(px(7.0))
        .my(px(3.0))
        .bg(move || color_of("menu-separator"))
}

/// 工具窗口标题栏 / 标题栏里的小图标按钮。
pub fn icon_button(icon: ImageAsset, label: &'static str, action: impl Fn() + 'static) -> View {
    view()
        .w(px(24.0))
        .h(px(24.0))
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .role(AccessibilityRole::Button)
        .label(label)
        .hover(|s| s.bg(move || color_of("control-hover")))
        .on_press(action)
        .child(
            svg(icon)
                .w(px(16.0))
                .h(px(16.0))
                .text_color(move || color_of("muted-fg")),
        )
}

/// 项目色调色板下标：按项目名做 FNV-1a，保证同一项目每次启动颜色一致。
fn project_palette_index(name: &str, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let hash = name.as_bytes().iter().fold(2_166_136_261u32, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(16_777_619)
    });
    hash as usize % len
}

fn project_palette_color(name: &str, dark: bool, badge: bool) -> u32 {
    let kind = if dark {
        keel_theme::ThemeKind::Dark
    } else {
        keel_theme::ThemeKind::Light
    };
    let tokens = keel_theme::theme(kind).tokens();
    let colors = if badge {
        &tokens.project_badge
    } else {
        &tokens.project_gradient
    };
    colors
        .get(project_palette_index(name, colors.len()))
        .copied()
        .unwrap_or(0x80_8080)
}

/// 标题栏左上角项目色渐变的起始色（IDEA `RecentProject.ColorN.MainToolbarGradientStart`）。
pub fn project_gradient_rgb(name: &str, dark: bool) -> (u8, u8, u8) {
    rgb_parts(project_palette_color(name, dark, false))
}

/// 项目头像的饱和底色（IDEA 的项目徽标色）。
pub fn project_badge_rgb(name: &str, dark: bool) -> u32 {
    project_palette_color(name, dark, true)
}

/// 按比例压暗一个 `0xRRGGBB` 颜色，用于头像渐变的末端。
pub fn shade_rgb(hex: u32, factor: f32) -> u32 {
    let (r, g, b) = rgb_parts(hex);
    let scale = |channel: u8| (f32::from(channel) * factor).round().clamp(0.0, 255.0) as u32;
    (scale(r) << 16) | (scale(g) << 8) | scale(b)
}

fn rgb_parts(hex: u32) -> (u8, u8, u8) {
    (
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    )
}

fn light_tokens() -> &'static keel_theme::Tokens {
    keel_theme::theme(keel_theme::ThemeKind::Light).tokens()
}

/// 岛屿圆角：IDEA `Island.arc=20`（非紧凑）。
pub fn island_radius() -> f32 {
    light_tokens().island_radius
}

/// 岛屿之间的帧背景间隙：IDEA `Island.borderWidth=6`。
pub fn island_gap() -> f32 {
    light_tokens().island_gap
}

/// 编辑器标签条高度：IDEA `ToolWindowHeader` 高度 41。
pub fn tab_height() -> f32 {
    light_tokens().tab_height
}

/// 工具窗口头部的 24×24 图标按钮（无动作，供菜单触发器使用）。
pub fn header_action(icon: ImageAsset, label: &'static str) -> View {
    view()
        .w(px(24.0))
        .h(px(24.0))
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .role(AccessibilityRole::Button)
        .label(label)
        .hover(|s| s.bg(move || color_of("control-hover")))
        .child(
            svg(icon)
                .w(px(16.0))
                .h(px(16.0))
                .text_color(move || color_of("muted-fg")),
        )
}

/// 工具窗口条 / 底部条上的方形按钮。
///
/// IDEA `SquareStripeButtonLook`：聚焦且选中的工具窗口用蓝底白图标；
/// 工具窗口可见但没有焦点时用按下态叠加色 + 深色图标；其余用普通图标。
pub fn stripe_button(
    icon: ImageAsset,
    label: &'static str,
    active: impl Fn() -> bool + 'static,
    focused: impl Fn() -> bool + 'static,
    action: impl Fn() + 'static,
) -> View {
    let active: Rc<dyn Fn() -> bool> = Rc::new(active);
    let focused: Rc<dyn Fn() -> bool> = Rc::new(focused);
    let bg_focused = {
        let (active, focused) = (active.clone(), focused.clone());
        move || active() && focused()
    };
    let bg_pressed = {
        let (active, focused) = (active.clone(), focused.clone());
        move || active() && !focused()
    };
    let fg_focused = {
        let (active, focused) = (active.clone(), focused.clone());
        move || active() && focused()
    };
    let fg_pressed = {
        let (active, focused) = (active.clone(), focused.clone());
        move || active() && !focused()
    };
    let hover_bg = {
        let (active, focused) = (active.clone(), focused.clone());
        move || {
            if active() {
                if focused() {
                    theme().primary
                } else {
                    color_of("control-pressed")
                }
            } else {
                color_of("control-hover")
            }
        }
    };
    view()
        .w(px(37.0))
        .h(px(40.0))
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .role(AccessibilityRole::Button)
        .label(label)
        .style_when(bg_focused, |s| s.bg(move || theme().primary))
        .style_when(bg_pressed, |s| s.bg(move || color_of("control-pressed")))
        .hover({
            let hover_bg = hover_bg.clone();
            move |s| s.bg(hover_bg.clone())
        })
        .on_press(action)
        .child(svg(icon).w(px(20.0)).h(px(20.0)).text_color(move || {
            if fg_focused() {
                theme().primary_foreground
            } else if fg_pressed() {
                theme().foreground
            } else {
                color_of("muted-fg")
            }
        }))
}

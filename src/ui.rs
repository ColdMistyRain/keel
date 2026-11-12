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

/// 菜单栏 / 子菜单的触发器。
pub fn menu_trigger(label: impl Into<SharedString>) -> impl IntoElement {
    let label = label.into();
    view()
        .h(px(28.0))
        .px(px(8.0))
        .items_center()
        .rounded(px(4.0))
        .hover(|s| s.bg(move || color_of("control-hover")))
        .child(text(label).text_size(px(13.0)))
}

/// 子菜单的触发器：左标签、右箭头。
pub fn submenu_trigger(label: impl Into<SharedString>) -> View {
    let label = label.into();
    view()
        .h(px(28.0))
        .px(px(10.0))
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .rounded(px(4.0))
        .hover(|s| s.bg(move || theme().accent))
        .child(text(label).text_size(px(13.0)).flex_1())
        .child(
            text("›")
                .text_size(px(12.0))
                .text_color(move || color_of("muted-fg")),
        )
}

/// 菜单项：左标签、右快捷键。
pub fn menu_action(
    label: impl Into<SharedString>,
    shortcut: Option<String>,
    action: impl Fn() + 'static,
) -> impl IntoElement {
    let label = label.into();
    let shortcut = SharedString::from(shortcut.unwrap_or_default());
    menu_item()
        .on_select(action)
        .px(px(10.0))
        .py(px(5.0))
        .rounded(px(4.0))
        .hover(|s| s.bg(move || theme().accent))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(20.0))
                .child(text(label).text_size(px(13.0)).flex_1())
                .child(
                    text(shortcut)
                        .text_size(px(12.0))
                        .text_color(move || color_of("muted-fg")),
                ),
        )
}

/// 菜单分隔线。
pub fn menu_separator() -> impl IntoElement {
    base_menu_separator()
        .h(px(1.0))
        .my(px(4.0))
        .bg(move || color_of("separator-line"))
}

/// 工具窗口标题栏里的小图标按钮。
pub fn icon_button(glyph: &'static str, label: &'static str, action: impl Fn() + 'static) -> View {
    view()
        .w(px(22.0))
        .h(px(22.0))
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .role(AccessibilityRole::Button)
        .label(label)
        .hover(|s| s.bg(move || color_of("control-hover")))
        .on_press(action)
        .child(
            text(glyph)
                .text_size(px(13.0))
                .text_color(move || color_of("muted-fg")),
        )
}

/// 工具窗口条 / 底部条上的方形按钮。
pub fn stripe_button(
    glyph: &'static str,
    label: &'static str,
    active: impl Fn() -> bool + 'static,
    action: impl Fn() + 'static,
) -> View {
    let active: Rc<dyn Fn() -> bool> = Rc::new(active);
    let (bg_active, fg_active) = (active.clone(), active.clone());
    view()
        .w(px(36.0))
        .h(px(36.0))
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .role(AccessibilityRole::Button)
        .label(label)
        .style_when(move || bg_active(), |s| s.bg(move || theme().primary))
        .hover(|s| s.bg(move || color_of("hover-overlay")))
        .on_press(action)
        .child(text(glyph).text_size(px(15.0)).text_color(move || {
            if fg_active() {
                theme().primary_foreground
            } else {
                color_of("muted-fg")
            }
        }))
}

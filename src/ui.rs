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
/// `AllIcons.General.WindowsMenu_20x20` 为 20×20，按钮悬停用
/// `toolbar-bg-hovered`。
pub fn burger_button() -> View {
    view()
        .w(px(24.0))
        .h(px(24.0))
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .role(AccessibilityRole::Button)
        .label("主菜单")
        .hover(|s| s.bg(move || color_of("hover-overlay")))
        .child(
            svg(crate::icons::MENU)
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

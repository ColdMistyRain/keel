//! 命令面板（IDEA 的 Search Everywhere / Find Action）。
//!
//! 直接复用 `keel-commands` 的注册表：同一批动作既出现在菜单、快捷键里，
//! 也在这里按名称搜索和执行。

use keel_commands::Presentation;
use nsh_base::{dialog, theme};
use nsh_core::ModalAnimation;
use nsh_core::element::input;
use nsh_core::prelude::*;

use crate::shell::Shell;
use crate::ui::color_of;

/// 命令面板的浮层；由 `App` 在 `palette` 为真时挂载。
pub fn overlay(shell: Shell) -> impl IntoElement {
    dialog()
        .animation(ModalAnimation::None)
        .close_on_escape(true)
        .close_on_backdrop_press(true)
        .on_cancel(move || shell.close_palette())
        .child(
            view()
                .absolute()
                .inset_0()
                .bg(move || color_of("overlay"))
                .focusable(false)
                .on_press(move || shell.close_palette()),
        )
        .child(
            view()
                .absolute()
                .top(px(76.0))
                .left_0()
                .right_0()
                .items_center()
                .child(
                    view()
                        .w(px(560.0))
                        .max_h(px(430.0))
                        .rounded(px(10.0))
                        .bg(move || theme().popover)
                        .border_1()
                        .border_color(move || color_of("popup-border"))
                        .shadow_md()
                        .child(search_field(shell))
                        .child(separator())
                        .child(results(shell))
                        .child(separator())
                        .child(footer(shell)),
                ),
        )
}

fn search_field(shell: Shell) -> impl IntoElement {
    view()
        .h(px(46.0))
        .px(px(14.0))
        .flex_row()
        .items_center()
        .child(
            input()
                .value(move || shell.palette_query.get().to_string())
                .placeholder("搜索操作…")
                .auto_focus(true)
                .text_size(px(14.0))
                .on_change(move |value: &str| {
                    shell.palette_query.set(value.to_owned().into());
                    shell.palette_query_changed();
                })
                .on_submit(move |_: &str| shell.perform_palette_selection())
                .flex_1(),
        )
}

fn results(shell: Shell) -> AnyElement {
    dynamic(move || {
        // 显式读取，查询或高亮变化时重建列表。
        let _ = shell.palette_query.get();
        let selected = shell.palette_selected.get();
        let items = shell.palette_actions();
        if items.is_empty() {
            return view()
                .h(px(90.0))
                .items_center()
                .justify_center()
                .child(
                    text("没有匹配的操作")
                        .text_size(px(12.0))
                        .text_color(move || color_of("muted-fg")),
                )
                .into_element();
        }
        view()
            .max_h(px(320.0))
            .child(
                scroll().max_h(px(320.0)).child(
                    view().py(px(4.0)).children(
                        items
                            .into_iter()
                            .enumerate()
                            .map(move |(index, (_, item))| {
                                row(shell, index, item, index == selected)
                            }),
                    ),
                ),
            )
            .into_element()
    })
}

fn row(shell: Shell, index: usize, item: Presentation, selected: bool) -> impl IntoElement {
    let label: SharedString = item.text.into();
    let shortcut: SharedString = item.shortcut.unwrap_or_default().into();
    let enabled = item.enabled;
    view()
        .h(px(34.0))
        .mx(px(6.0))
        .px(px(10.0))
        .rounded(px(4.0))
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .style_when(selected, |s| s.bg(move || theme().accent))
        .hover(|s| s.bg(move || color_of("hover-overlay")))
        .role(AccessibilityRole::Button)
        .label(label.clone())
        .on_press(move || shell.perform_palette_at(index))
        .child(
            text(label)
                .text_size(px(13.0))
                .flex_1()
                .text_color(move || {
                    if enabled {
                        theme().foreground
                    } else {
                        color_of("muted-fg")
                    }
                }),
        )
        .child(
            text(shortcut)
                .text_size(px(11.0))
                .text_color(move || color_of("muted-fg")),
        )
}

fn footer(shell: Shell) -> impl IntoElement {
    view()
        .h(px(30.0))
        .px(px(12.0))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            text("↑↓ 选择 · Enter 执行 · Esc 关闭")
                .text_size(px(11.0))
                .text_color(move || color_of("muted-fg")),
        )
        .child(view().flex_1())
        .child(
            text(move || format!("{} 项", shell.palette_actions().len()))
                .text_size(px(11.0))
                .text_color(move || color_of("muted-fg")),
        )
}

fn separator() -> impl IntoElement {
    view().h(px(1.0)).bg(move || color_of("separator-line"))
}

//! 主工具栏的 run widget（IDEA `RedesignedRunWidget`）。
//!
//! 结构：绿色运行按钮 + 运行配置选择器 + 调试按钮。运行中主按钮变成
//! 红色停止按钮（`RunWidget.runningBackground / stopBackground`）。
//! 目前没有真正的执行器，运行 / 停止只驱动「运行」工具窗口与状态栏提示。

use nsh_base::{menu, theme};
use nsh_core::prelude::*;

use crate::icons;
use crate::shell::Shell;
use crate::ui::{color_of, menu_action, menu_separator, popup_style};

/// run widget 整体高度与按钮边长（`RunWidget.toolbarHeight / actionButtonWidth`）。
const HEIGHT: f32 = 30.0;
const BUTTON: f32 = 30.0;

pub fn run_widget(shell: Shell) -> impl IntoElement {
    view()
        .h(px(HEIGHT))
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .child(config_selector(shell))
        .child(run_button(shell))
        .child(debug_button(shell))
}

fn run_button(shell: Shell) -> impl IntoElement {
    let running = move || shell.running.get();
    view()
        .w(px(BUTTON))
        .h(px(BUTTON))
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .role(AccessibilityRole::Button)
        .label(move || if running() { "停止" } else { "运行" })
        .style_when(running, |s| s.bg(move || color_of("stop-background")))
        .hover(|s| s.bg(move || color_of("hover-overlay")))
        .on_press(move || shell.toggle_run())
        .child(when(
            move || shell.running.get(),
            move || {
                svg(icons::STOP)
                    .w(px(14.0))
                    .h(px(14.0))
                    .text_color(move || theme().primary_foreground)
            },
        ))
        .child(when(
            move || !shell.running.get(),
            move || svg(icons::RUN).w(px(16.0)).h(px(16.0)),
        ))
}

fn config_selector(shell: Shell) -> impl IntoElement {
    let trigger = view()
        .h(px(HEIGHT))
        .px(px(6.0))
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .rounded(px(6.0))
        .role(AccessibilityRole::Button)
        .label("运行配置")
        .hover(|s| s.bg(move || color_of("hover-overlay")))
        .child(
            text(move || shell.run_config_name())
                .text_size(px(13.0))
                .number_of_lines(1)
                .text_color(move || theme().foreground),
        )
        .child(
            svg(icons::CHEVRON_DOWN)
                .w(px(12.0))
                .h(px(12.0))
                .text_color(move || color_of("muted-fg")),
        );

    let config_name = shell.run_config_name();
    menu()
        .offset(px(4.0))
        .popup_style(popup_style)
        .trigger(trigger)
        .child(menu_action(
            Some(icons::JAVA),
            config_name.clone(),
            Some('M'),
            None,
            move || {
                shell.ws.say(format!("运行配置：{config_name}"));
            },
        ))
        .child(menu_separator())
        .child(menu_action(
            Some(icons::SETTINGS),
            "编辑配置…",
            Some('E'),
            None,
            move || shell.ws.say("运行配置编辑将在 P4 迁移"),
        ))
}

fn debug_button(shell: Shell) -> impl IntoElement {
    let running = move || shell.running.get();
    let enabled = move || !shell.running.get();
    view()
        .w(px(BUTTON))
        .h(px(BUTTON))
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .role(AccessibilityRole::Button)
        .label("调试")
        .hover(|s| s.bg(move || color_of("hover-overlay")))
        .style_when(running, |s| s.opacity(0.4))
        .on_press(move || {
            if enabled() {
                shell.toggle_debug();
            }
        })
        .child(svg(icons::DEBUG).w(px(16.0)).h(px(16.0)))
}

//! 底部工具窗口：状态栏左侧的切换条 + 面板（IDEA 的底部 `ToolWindow`）。
//!
//! 问题 / 运行 / 终端在 P2 只是外壳占位；内容迁移在 P3/P4（诊断、运行配置、
//! `nsh_base::terminal`）。

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::shell::{Bottom, Shell};
use crate::ui::{color_of, icon_button};

/// 状态栏左侧的底部工具窗口条（IDEA 里是图标条）。
pub fn bottom_bar(shell: Shell) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .children(Bottom::ALL.into_iter().map(move |bottom| {
            let active = move || shell.bottom.get() == Some(bottom);
            view()
                .w(px(24.0))
                .h(px(20.0))
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .role(AccessibilityRole::Button)
                .label(bottom.label())
                .style_when(active, |s| s.bg(move || color_of("control-hover")))
                .hover(|s| s.bg(move || color_of("hover-overlay")))
                .on_press(move || shell.toggle_bottom(bottom))
                .child(
                    text(bottom.glyph())
                        .text_size(px(11.0))
                        .text_color(move || {
                            if active() {
                                color_of("secondary-fg")
                            } else {
                                color_of("muted-fg")
                            }
                        }),
                )
        }))
}

/// 底部工具窗口面板。
pub fn bottom_panel(shell: Shell) -> impl IntoElement {
    view()
        .flex_1()
        .rounded(px(8.0))
        .bg(move || color_of("island-bg"))
        .child(panel_header(shell))
        .child(panel_content(shell))
}

fn panel_header(shell: Shell) -> impl IntoElement {
    view()
        .h(px(30.0))
        .flex_row()
        .items_center()
        .px(px(8.0))
        .gap(px(2.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .flex_1()
                .children(
                    Bottom::ALL
                        .into_iter()
                        .map(move |bottom| tab_button(shell, bottom)),
                ),
        )
        .child(icon_button("⚙", "运行配置", move || {
            shell.ws.say("运行配置将在 P4 迁移")
        }))
        .child(icon_button("×", "关闭底部窗口", move || {
            shell.bottom.set(None)
        }))
}

/// 面板标题栏里的标签（问题 / 运行 / 终端）。
fn tab_button(shell: Shell, bottom: Bottom) -> View {
    let active = move || shell.bottom.get() == Some(bottom);
    view()
        .h(px(22.0))
        .px(px(8.0))
        .items_center()
        .rounded(px(4.0))
        .role(AccessibilityRole::Button)
        .label(bottom.label())
        .style_when(active, |s| s.bg(move || color_of("control-hover")))
        .hover(|s| s.bg(move || color_of("hover-overlay")))
        .on_press(move || shell.bottom.set(Some(bottom)))
        .child(
            text(bottom.label())
                .text_size(px(11.0))
                .text_color(move || {
                    if active() {
                        theme().foreground
                    } else {
                        color_of("muted-fg")
                    }
                }),
        )
}

fn panel_content(shell: Shell) -> impl IntoElement {
    dynamic(move || {
        let bottom = shell.bottom.get().unwrap_or(Bottom::Problems);
        let (title, hint) = match bottom {
            Bottom::Problems => ("没有发现问题", "编译、检查与 LSP 诊断将在 P3/P4 接入"),
            Bottom::Run => ("没有正在运行的应用", "运行配置将在 P4 迁移"),
            Bottom::Terminal => ("终端尚未连接", "nsh_base::terminal 将在 P4 接入"),
        };
        view()
            .flex_1()
            .p(px(12.0))
            .gap(px(6.0))
            .child(text(title).text_size(px(12.0)))
            .child(
                text(hint)
                    .text_size(px(11.0))
                    .text_color(move || color_of("muted-fg")),
            )
    })
}

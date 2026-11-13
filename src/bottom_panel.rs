//! 底部工具窗口：面板（IDEA 的底部 `ToolWindow`）。
//!
//! 问题 / 运行 / 终端在 P2 只是外壳占位；内容迁移在 P3/P4（诊断、运行配置、
//! `nsh_base::terminal`）。切换按钮在左侧条纹的下半区。

use nsh_base::{menu, theme};
use nsh_core::prelude::*;

use crate::icons;
use crate::shell::{Bottom, Shell};
use crate::ui::{
    color_of, header_action, icon_button, island_radius, menu_action, menu_separator, popup_style,
};

/// 底部工具窗口面板。
pub fn bottom_panel(shell: Shell) -> impl IntoElement {
    view()
        .flex_1()
        .rounded(px(island_radius()))
        .bg(move || color_of("island-bg"))
        .child(panel_header(shell))
        .child(view().h(px(1.0)).bg(move || color_of("separator-line")))
        .child(panel_content(shell))
}

fn panel_header(shell: Shell) -> impl IntoElement {
    view()
        .h(px(41.0))
        .flex_row()
        .items_center()
        .px(px(8.0))
        .gap(px(2.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .flex_1()
                .children(
                    Bottom::ALL
                        .into_iter()
                        .map(move |bottom| tab_button(shell, bottom)),
                ),
        )
        .child(bottom_more(shell))
        .child(icon_button(icons::HIDE, "隐藏底部窗口", move || {
            shell.hide_bottom()
        }))
}

/// 「更多」菜单：运行配置入口和隐藏底部窗口。
fn bottom_more(shell: Shell) -> impl IntoElement {
    menu()
        .offset(px(4.0))
        .popup_style(popup_style)
        .trigger(header_action(icons::MORE, "更多底部窗口操作"))
        .child(menu_action(
            Some(icons::SETTINGS),
            "运行配置",
            Some('E'),
            None,
            move || shell.ws.say("运行配置将在 P4 迁移"),
        ))
        .child(menu_separator())
        .child(menu_action(
            Some(icons::HIDE),
            "隐藏底部窗口",
            Some('H'),
            None,
            move || shell.hide_bottom(),
        ))
}

/// 面板标题栏里的标签（问题 / 运行 / 终端）。
///
/// 2026.2 的底部工具窗口和编辑器标签一样用 Islands 胶囊：
/// 高 28、圆角 6；聚焦且选中时 `#E3EBFE/#A7C5FF`，未聚焦时
/// `#E9EAEE/gray-130`。
fn tab_button(shell: Shell, bottom: Bottom) -> View {
    let active = move || shell.bottom.get() == Some(bottom);
    let active_focused = move || shell.bottom.get() == Some(bottom) && shell.bottom_focused.get();
    let active_inactive = move || shell.bottom.get() == Some(bottom) && !shell.bottom_focused.get();
    view()
        .h(px(28.0))
        .px(px(10.0))
        .flex_row()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .role(AccessibilityRole::Button)
        .label(bottom.label())
        .hover(|s| s.bg(move || color_of("control-hover")))
        .on_press(move || {
            shell.bottom.set(Some(bottom));
            shell.focus_bottom();
        })
        .style_when(active_focused, |s| {
            s.bg(move || color_of("tab-selected-bg"))
                .border_1()
                .border_color(move || color_of("tab-selected-border"))
        })
        .style_when(active_inactive, |s| {
            s.bg(move || color_of("tab-selected-bg-inactive"))
                .border_1()
                .border_color(move || color_of("tab-selected-border-inactive"))
        })
        .child(
            text(bottom.label())
                .text_size(px(13.0))
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
        let running = shell.running.get();
        let (title, hint): (SharedString, &'static str) = match bottom {
            Bottom::Problems => (
                "没有发现问题".into(),
                "编译、检查与 LSP 诊断将在 P3/P4 接入",
            ),
            Bottom::Run => {
                if running {
                    (
                        format!("正在运行 {}…", shell.run_config_name()).into(),
                        "按 Ctrl+F2 或点击工具栏的停止按钮结束运行",
                    )
                } else {
                    (
                        "没有正在运行的应用".into(),
                        "按 Shift+F10 或点击工具栏的绿色按钮运行当前配置",
                    )
                }
            }
            Bottom::Terminal => ("终端尚未连接".into(), "nsh_base::terminal 将在 P4 接入"),
        };
        view()
            .flex_1()
            .p(px(12.0))
            .gap(px(6.0))
            .child(text(title).text_size(px(13.0)))
            .child(
                text(hint)
                    .text_size(px(12.0))
                    .text_color(move || color_of("muted-fg")),
            )
    })
}

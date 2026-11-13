//! 左侧工具窗口：条纹、标题栏、项目树（IDEA 的 `ToolWindowPane` /
//! `ToolWindowStripeManager`）。
//!
//! 条纹常驻；面板本身由 `resizable_panel` 控制宽度，可拖宽、可折叠。
//! 项目树用 `scroll` + `dynamic` 自绘（`nsh_base::Tree` 在 0.5.0 Windows
//! 下不绘制行，见 `docs/NSH_MIGRATION_PLAN.md`）。

use std::path::PathBuf;

use nsh_base::{button, context_menu, menu, theme};
use nsh_core::prelude::*;

use crate::icons;
use crate::shell::{Bottom, Shell, Tool};
use crate::ui::{
    color_of, header_action, icon_button, island_radius, menu_action, menu_separator, popup_style,
    stripe_button,
};
use crate::workspace::{TreeRow, Workspace};

/// 左侧 40 点宽的工具窗口条。
///
/// IDEA 的条纹上半区是工具窗口按钮，下半区是问题 / 终端等底部工具窗口按钮。
pub fn tool_window_bar(shell: Shell) -> impl IntoElement {
    view()
        .w(px(40.0))
        .h_full()
        .flex_col()
        .items_center()
        .gap(px(4.0))
        .py(px(6.0))
        .children(Tool::ALL.into_iter().map(move |tool| {
            stripe_button(
                icons::for_action(tool.action_id()).unwrap_or(icons::FILE),
                tool.label(),
                move || shell.tools.get() && shell.tool.get() == tool,
                move || shell.tool_focused.get(),
                move || shell.toggle_tool(tool),
            )
        }))
        .child(view().flex_1())
        .children(Bottom::ALL.into_iter().map(move |bottom| {
            stripe_button(
                icons::for_action(bottom.action_id()).unwrap_or(icons::FILE),
                bottom.label(),
                move || shell.bottom.get() == Some(bottom),
                move || shell.bottom_focused.get(),
                move || shell.toggle_bottom(bottom),
            )
        }))
}

/// 工具窗口面板。
pub fn side_panel(shell: Shell) -> View {
    view()
        .flex_1()
        .rounded(px(island_radius()))
        .bg(move || color_of("island-bg"))
        .child(panel_header(shell))
        .child(
            project_panel(shell)
                .style_when(move || shell.tool.get() != Tool::Project, |s| s.hidden()),
        )
        .child(
            placeholder(shell)
                .style_when(move || shell.tool.get() == Tool::Project, |s| s.hidden()),
        )
}

/// 面板标题栏：标题 + 下拉箭头；上下文动作只在工具窗口聚焦时出现，
/// 右侧始终是「更多」和「隐藏」。
///
/// IDEA `ToolWindow.Header`：高 41、标题左右内缩 12 / 16，右侧工具条
/// 内缩 0,12,0,8；未聚焦时只保留 `⋮` 和 `—`。
fn panel_header(shell: Shell) -> impl IntoElement {
    view()
        .h(px(41.0))
        .flex_row()
        .items_center()
        .pl(px(12.0))
        .pr(px(8.0))
        .gap(px(2.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .flex_1()
                .child(
                    text(move || shell.tool.get().label())
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    svg(icons::CHEVRON_DOWN)
                        .w(px(12.0))
                        .h(px(12.0))
                        .text_color(move || color_of("muted-fg")),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .style_when(
                    move || !shell.tool_focused.get() || shell.tool.get() != Tool::Project,
                    |s| s.hidden(),
                )
                .child(icon_button(icons::REFRESH, "刷新项目", move || {
                    shell.ws.refresh_project()
                }))
                .child(icon_button(
                    icons::COLLAPSE_ALL,
                    "收起全部",
                    move || shell.ws.expanded.set(Vec::new()),
                ))
                .child(
                    view()
                        .w(px(1.0))
                        .h(px(16.0))
                        .mx(px(2.0))
                        .bg(move || color_of("separator-line")),
                ),
        )
        .child(tool_window_more(shell))
        .child(icon_button(icons::HIDE, "隐藏工具窗口", move || {
            shell.hide_tool()
        }))
}

/// 「更多」菜单：聚焦时的刷新 / 收起全部，以及始终可用的隐藏。
fn tool_window_more(shell: Shell) -> impl IntoElement {
    dynamic(move || {
        let project = shell.tool.get() == Tool::Project;
        let mut element = menu()
            .offset(px(4.0))
            .popup_style(popup_style)
            .trigger(header_action(icons::MORE, "更多工具窗口操作"));
        if project {
            element = element
                .child(menu_action(
                    Some(icons::REFRESH),
                    "刷新项目",
                    Some('R'),
                    None,
                    move || shell.ws.refresh_project(),
                ))
                .child(menu_action(
                    Some(icons::COLLAPSE_ALL),
                    "收起全部",
                    Some('L'),
                    None,
                    move || shell.ws.expanded.set(Vec::new()),
                ))
                .child(menu_separator());
        }
        element
            .child(menu_action(
                Some(icons::HIDE),
                "隐藏工具窗口",
                Some('H'),
                None,
                move || shell.hide_tool(),
            ))
            .into_element()
    })
}

fn placeholder(shell: Shell) -> View {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .child(
            text(move || format!("{}工具窗口", shell.tool.get().label()))
                .text_size(px(13.0))
                .text_color(move || color_of("muted-fg")),
        )
        .child(
            text("内容将在 P4 迁移")
                .text_size(px(12.0))
                .text_color(move || color_of("muted-fg")),
        )
}

fn project_panel(shell: Shell) -> View {
    view().flex_1().child(project_tree(shell)).child(when(
        move || shell.ws.root.get().is_none(),
        move || {
            view()
                .p(px(12.0))
                .gap(px(8.0))
                .child(
                    text("未打开项目")
                        .text_size(px(12.0))
                        .text_color(move || color_of("muted-fg")),
                )
                .child(
                    button()
                        .accessibility_label("打开项目")
                        .on_press(move || shell.pick_project())
                        .h(px(28.0))
                        .px(px(10.0))
                        .items_center()
                        .justify_center()
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(move || color_of("popup-border"))
                        .hover(|s| s.bg(move || color_of("hover-overlay")))
                        .child(text("打开项目…").text_size(px(12.0))),
                )
        },
    ))
}

/// 项目树：自绘的 `scroll` 列表。
///
/// IDEA `Tree`：行高 24、左右边框各 12，子层级缩进 = `leftChildIndent 7 +
/// rightChildIndent 11`。
fn project_tree(shell: Shell) -> View {
    view()
        .flex_1()
        .child(scroll().flex_1().child(dynamic(move || {
            view().py(px(4.0)).px(px(12.0)).children(
                shell
                    .ws
                    .visible_rows()
                    .into_iter()
                    .map(|row| tree_row(shell, row)),
            )
        })))
}

fn tree_row(shell: Shell, row: TreeRow) -> AnyElement {
    let ws = shell.ws;
    let selected_id = row.id.clone();
    let select_id = row.id.clone();
    let menu_id = row.id.clone();
    let toggle_id = row.id.clone();
    let folder = row.folder;
    let open = row.open;
    let dark = shell.appearance.get() == Appearance::Dark;
    let row_path = row.id.clone();
    let selected = move || ws.selected.get().as_ref() == Some(&selected_id);
    let body = view()
        .h(px(24.0))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .pl(px(8.0 + row.depth as f32 * 18.0))
        .pr(px(8.0))
        .rounded(px(4.0))
        .style_when(selected, |s| s.bg(move || theme().accent))
        .hover(|s| s.bg(move || color_of("hover-overlay")))
        .role(AccessibilityRole::Button)
        .label(row.label.clone())
        .on_press(move |event: &PressEvent| {
            shell.focus_tool();
            ws.selected.set(Some(select_id.clone()));
            if folder {
                toggle_expanded(ws, &select_id);
                return;
            }
            let path = PathBuf::from(select_id.as_ref());
            if !path.is_file() {
                return;
            }
            if event.clicks >= 2 {
                // 双击固定标签。
                ws.open_files(vec![path]);
            } else {
                // 单击预览（IDEA 的 preview tab）。
                ws.preview_file(path);
            }
        })
        .child(
            view()
                .w(px(16.0))
                .h(px(16.0))
                .items_center()
                .justify_center()
                .child(if folder {
                    svg(if open {
                        icons::CHEVRON_DOWN
                    } else {
                        icons::CHEVRON_RIGHT
                    })
                    .w(px(12.0))
                    .h(px(12.0))
                    .text_color(move || color_of("muted-fg"))
                    .into_element()
                } else {
                    view().into_element()
                }),
        )
        .child(
            svg(icons::for_file(&row_path, folder, dark))
                .w(px(16.0))
                .h(px(16.0))
                .text_color(move || color_of("secondary-fg")),
        )
        .child(
            text(row.label)
                .text_size(px(13.0))
                .text_color(move || theme().foreground)
                .number_of_lines(1),
        );

    let path = PathBuf::from(menu_id.as_ref());
    let mut menu = context_menu()
        .offset(px(4.0))
        .popup_style(popup_style)
        .trigger(body);
    if folder {
        menu = menu
            .child(menu_action(
                Some(if open {
                    icons::CHEVRON_DOWN
                } else {
                    icons::CHEVRON_RIGHT
                }),
                if open { "收起" } else { "展开" },
                Some(if open { 'C' } else { 'X' }),
                None,
                move || toggle_expanded(ws, &toggle_id),
            ))
            .child(menu_action(
                Some(icons::REFRESH),
                "刷新项目",
                Some('R'),
                None,
                move || ws.refresh_project(),
            ))
            .child(menu_action(
                Some(icons::COLLAPSE_ALL),
                "收起全部",
                Some('L'),
                None,
                move || ws.expanded.set(Vec::new()),
            ));
    } else {
        let (open_path, preview_path, reveal_path) = (path.clone(), path.clone(), path.clone());
        let clipboard = path.display().to_string();
        menu = menu
            .child(menu_action(
                Some(icons::FILE),
                "打开",
                Some('O'),
                None,
                move || ws.open_files(vec![open_path.clone()]),
            ))
            .child(menu_action(
                Some(icons::EYE),
                "预览",
                Some('P'),
                None,
                move || ws.preview_file(preview_path.clone()),
            ))
            .child(menu_separator())
            .child(menu_action(
                Some(icons::COPY),
                "复制路径",
                Some('C'),
                None,
                move || {
                    set_clipboard(clipboard.clone());
                    ws.say("已复制路径");
                },
            ))
            .child(menu_action(
                Some(icons::LOCATE),
                "在项目中定位",
                Some('E'),
                None,
                move || shell.reveal_path(&reveal_path),
            ));
    }
    menu.into_element()
}

fn toggle_expanded(ws: Workspace, id: &SharedString) {
    ws.expanded.update(|ids| {
        if let Some(index) = ids.iter().position(|item| item == id) {
            ids.remove(index);
        } else {
            ids.push(id.clone());
        }
    });
}

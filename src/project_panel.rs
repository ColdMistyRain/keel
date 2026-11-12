//! 左侧工具窗口：条纹、标题栏、项目树（IDEA 的 `ToolWindowPane` /
//! `ToolWindowStripeManager`）。
//!
//! 条纹常驻；面板本身由 `resizable_panel` 控制宽度，可拖宽、可折叠。
//! 项目树用 `scroll` + `dynamic` 自绘（`nsh_base::Tree` 在 0.5.0 Windows
//! 下不绘制行，见 `docs/NSH_MIGRATION_PLAN.md`）。

use std::path::{Path, PathBuf};

use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::shell::{Shell, Tool};
use crate::ui::{color_of, icon_button, stripe_button};
use crate::workspace::{TreeRow, Workspace};

/// 左侧 40 点宽的工具窗口条。
pub fn tool_window_bar(shell: Shell) -> impl IntoElement {
    view()
        .w(px(40.0))
        .items_center()
        .gap(px(4.0))
        .py(px(6.0))
        .children(Tool::ALL.into_iter().map(move |tool| {
            stripe_button(
                tool.glyph(),
                tool.label(),
                move || shell.tools.get() && shell.tool.get() == tool,
                move || shell.toggle_tool(tool),
            )
        }))
}

/// 工具窗口面板。
pub fn side_panel(shell: Shell) -> View {
    view()
        .flex_1()
        .rounded(px(8.0))
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

/// 面板标题栏：标题 + 刷新 / 收起全部 / 隐藏。
fn panel_header(shell: Shell) -> impl IntoElement {
    view()
        .h(px(32.0))
        .flex_row()
        .items_center()
        .px(px(10.0))
        .gap(px(2.0))
        .child(
            text(move || shell.tool.get().label())
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .flex_1(),
        )
        .child(
            icon_button("⟳", "刷新项目", move || shell.ws.refresh_project())
                .style_when(move || shell.tool.get() != Tool::Project, |s| s.hidden()),
        )
        .child(
            icon_button("⊟", "收起全部", move || {
                shell.ws.expanded.set(Vec::new())
            })
            .style_when(move || shell.tool.get() != Tool::Project, |s| s.hidden()),
        )
        .child(icon_button("×", "隐藏工具窗口", move || {
            shell.tools.set(false)
        }))
}

fn placeholder(shell: Shell) -> View {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .child(
            text(move || format!("{}工具窗口", shell.tool.get().label()))
                .text_size(px(12.0))
                .text_color(move || color_of("muted-fg")),
        )
        .child(
            text("内容将在 P4 迁移")
                .text_size(px(11.0))
                .text_color(move || color_of("muted-fg")),
        )
}

fn project_panel(shell: Shell) -> View {
    view().flex_1().child(project_tree(shell.ws)).child(when(
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
fn project_tree(ws: Workspace) -> View {
    view()
        .flex_1()
        .child(scroll().flex_1().child(dynamic(move || {
            view()
                .py(px(4.0))
                .children(ws.visible_rows().into_iter().map(|row| tree_row(ws, row)))
        })))
}

fn tree_row(ws: Workspace, row: TreeRow) -> impl IntoElement {
    let id = row.id.clone();
    let select_id = row.id.clone();
    let folder = row.folder;
    let open = row.open;
    let selected = move || ws.selected.get().as_ref() == Some(&id);
    view()
        .h(px(22.0))
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .pl(px(6.0 + row.depth as f32 * 12.0))
        .pr(px(6.0))
        .rounded(px(4.0))
        .style_when(selected, |s| s.bg(move || theme().accent))
        .hover(|s| s.bg(move || color_of("hover-overlay")))
        .role(AccessibilityRole::Button)
        .label(row.label.clone())
        .on_press(move |event: &PressEvent| {
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
                .w(px(12.0))
                .h(px(12.0))
                .items_center()
                .justify_center()
                .child(
                    text(if !folder {
                        ""
                    } else if open {
                        "▾"
                    } else {
                        "▸"
                    })
                    .text_size(px(9.0))
                    .text_color(move || color_of("muted-fg")),
                ),
        )
        .child(file_badge(row.id.as_ref(), row.folder))
        .child(
            text(row.label)
                .text_size(px(12.0))
                .text_color(move || theme().foreground)
                .number_of_lines(1),
        )
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

/// 文件类型徽标：文件夹用方块，文件用首字母，先代替 IDEA 的文件图标。
fn file_badge(path: &str, folder: bool) -> impl IntoElement {
    let letter = if folder {
        "▣".to_owned()
    } else {
        Path::new(path)
            .extension()
            .map(|ext| {
                let ext = ext.to_string_lossy();
                ext.chars()
                    .next()
                    .map(|c| c.to_ascii_uppercase().to_string())
                    .unwrap_or_else(|| "F".to_owned())
            })
            .unwrap_or_else(|| "F".to_owned())
    };
    view()
        .w(px(14.0))
        .h(px(14.0))
        .items_center()
        .justify_center()
        .rounded(px(3.0))
        .bg(move || color_of("hover-overlay"))
        .child(
            text(letter)
                .text_size(px(9.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || color_of("secondary-fg")),
        )
}

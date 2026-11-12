//! 编辑器区：标签页、面包屑、编辑器与欢迎页（IDEA 的
//! `FileEditorManager` / `EditorsSplitters` 的最小实现）。
//!
//! 标签页支持未保存圆点、关闭按钮、横向溢出滚动与右键菜单；编辑器仍是
//! `nsh_base::editor`，后续 P3 再补 LSP 与查找替换。

use std::path::{Path, PathBuf};

use nsh_base::{EditorClasses, button, context_menu, editor, theme};
use nsh_core::prelude::*;

use crate::icons;
use crate::shell::Shell;
use crate::ui::{color_of, menu_action, popup_style};
use crate::workspace::{Doc, Workspace, language_label, language_of};

/// 编辑器区（岛屿容器）。
pub fn editor_area(shell: Shell) -> impl IntoElement {
    let ws = shell.ws;
    view()
        .flex_1()
        .min_h(px(0.0))
        .rounded(px(8.0))
        .bg(move || color_of("editor-bg"))
        .child(tab_strip(shell))
        .child(view().h(px(1.0)).bg(move || color_of("editor-border")))
        .child(breadcrumbs(ws))
        .child(editor_pane(shell))
}

/// 标签条：IDEA Islands 的胶囊标签（`IslandsTabPainter`）。
///
/// 条高 36，标签高 28、圆角 6（arc 12），横向内缩 4；选中标签是
/// `#E3EBFE` 底色 + `#A7C5FF` 1px 边框，未选中没有底色。
fn tab_strip(shell: Shell) -> impl IntoElement {
    let ws = shell.ws;
    view()
        .h(px(36.0))
        .flex_row()
        .items_center()
        .px(px(4.0))
        .child(
            scroll()
                .horizontal()
                .scrollbar(false)
                .h(px(36.0))
                .flex_1()
                .child(
                    view()
                        .h(px(36.0))
                        .flex_row()
                        .items_center()
                        .gap(px(2.0))
                        .pl(px(3.0))
                        .child(each(
                            ws.docs,
                            |doc: &Doc| doc.path.display().to_string(),
                            move |doc, index| tab(shell, doc, index),
                        )),
                ),
        )
}

fn tab(shell: Shell, doc: ReadSignal<Doc>, index: ReadSignal<usize>) -> AnyElement {
    let ws = shell.ws;
    let path = doc.get().path;
    let label_path = path.clone();
    let select_path = path.clone();
    let close_path = path.clone();
    let name_path = path.clone();
    let dirty_path = path.clone();
    let close_dirty_path = path.clone();
    let preview_path = path.clone();
    let others_path = path.clone();
    let reveal_path = path.clone();
    let selected_bg = move || ws.active.get() == Some(index.get());
    let selected_fg = move || ws.active.get() == Some(index.get());
    let selected_close = move || ws.active.get() == Some(index.get());
    let icon = icons::for_file(
        &path.display().to_string(),
        false,
        shell.appearance.get() == Appearance::Dark,
    );
    let body = view()
        .h(px(28.0))
        .max_w(px(220.0))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(10.0))
        .rounded(px(6.0))
        .group_named("keel-tab")
        .style_when(selected_bg, |s| {
            s.bg(move || color_of("tab-selected-bg"))
                .border_1()
                .border_color(move || color_of("tab-selected-border"))
        })
        .hover(|s| s.bg(move || color_of("hover-overlay")))
        .role(AccessibilityRole::Button)
        .label(move || doc_name(ws, &label_path))
        .on_press(move || {
            ws.activate_path(&select_path);
        })
        .child(
            svg(icon)
                .w(px(16.0))
                .h(px(16.0))
                .text_color(move || color_of("secondary-fg")),
        )
        .child(
            text(move || doc_name(ws, &name_path))
                .text_size(px(13.0))
                .number_of_lines(1)
                .style_when(move || doc_preview(ws, &preview_path), |s| s.italic())
                .text_color(move || {
                    if selected_fg() {
                        theme().foreground
                    } else {
                        color_of("muted-fg")
                    }
                }),
        )
        .child(
            text(move || {
                if doc_dirty(ws, &dirty_path) {
                    "●"
                } else {
                    ""
                }
            })
            .text_size(px(8.0))
            .text_color(move || color_of("muted-fg")),
        )
        .child(
            view()
                .w(px(16.0))
                .h(px(16.0))
                .items_center()
                .justify_center()
                .rounded(px(3.0))
                .role(AccessibilityRole::Button)
                .label("关闭")
                .hover(|s| s.bg(move || color_of("control-hover")))
                .on_press(move || ws.close_path(&close_path))
                .style_when(
                    move || !selected_close() && !doc_dirty(ws, &close_dirty_path),
                    |s| s.opacity(0.0),
                )
                .group_hover_named("keel-tab", |s| s.opacity(1.0))
                .child(
                    svg(icons::CLOSE)
                        .w(px(12.0))
                        .h(px(12.0))
                        .text_color(move || color_of("muted-fg")),
                ),
        );

    context_menu()
        .offset(px(4.0))
        .popup_style(popup_style)
        .trigger(body)
        .child(menu_action(
            Some(icons::CLOSE),
            "关闭",
            Some('C'),
            None,
            move || ws.close_path(&path),
        ))
        .child(menu_action(
            Some(icons::CLOSE),
            "关闭其他标签页",
            Some('O'),
            None,
            move || {
                if let Some(index) = ws
                    .docs
                    .with(|docs| docs.iter().position(|doc| doc.path == others_path))
                {
                    ws.close_others(index);
                }
            },
        ))
        .child(menu_action(
            Some(icons::CLOSE),
            "关闭所有标签页",
            Some('A'),
            None,
            move || ws.close_all(),
        ))
        .child(crate::ui::menu_separator())
        .child(menu_action(
            Some(icons::LOCATE),
            "在项目中定位",
            Some('E'),
            None,
            move || shell.reveal_path(&reveal_path),
        ))
        .into_element()
}

/// 面包屑：每段是可悬停的 crumb，圆角 2、左右内缩 7（IDEA `BreadcrumbsComponent`）。
fn breadcrumbs(ws: Workspace) -> impl IntoElement {
    dynamic(move || {
        let parts = ws.breadcrumbs();
        let last = parts.len().saturating_sub(1);
        let mut row = view()
            .h(px(28.0))
            .px(px(12.0))
            .flex_row()
            .items_center()
            .gap(px(2.0));
        for (index, part) in parts.into_iter().enumerate() {
            row = row.child(
                view()
                    .h(px(22.0))
                    .px(px(7.0))
                    .items_center()
                    .justify_center()
                    .rounded(px(2.0))
                    .hover(|s| s.bg(move || color_of("hover-overlay")))
                    .child(
                        text(part)
                            .text_size(px(12.0))
                            .text_color(move || color_of("muted-fg"))
                            .number_of_lines(1),
                    ),
            );
            if index < last {
                row = row.child(
                    text("›")
                        .text_size(px(12.0))
                        .text_color(move || color_of("muted-fg")),
                );
            }
        }
        row
    })
}

fn editor_pane(shell: Shell) -> impl IntoElement {
    let ws = shell.ws;
    view()
        .flex_1()
        .child(
            view()
                .flex_1()
                .style_when(move || ws.active.get().is_none(), |s| s.hidden())
                .child(code_editor(ws)),
        )
        .child(
            view()
                .flex_1()
                .style_when(move || ws.active.get().is_some(), |s| s.hidden())
                .child(welcome(shell)),
        )
}

fn code_editor(ws: Workspace) -> impl IntoElement {
    let text = computed(move || {
        ws.active.with(|active| {
            active.and_then(|index| {
                ws.docs
                    .with(|docs| docs.get(index).map(|doc| doc.text.clone()))
            })
        })
    });
    let language = computed(move || {
        ws.active.with(|active| {
            active.and_then(|index| {
                ws.docs.with(|docs| {
                    docs.get(index)
                        .map(|doc| SharedString::from(language_of(&doc.path)))
                })
            })
        })
    });
    editor()
        .value(move || text.get())
        .language(move || language.get())
        .font_size(px(13.0))
        .line_height(px(20.0))
        .classes(idea_editor_classes())
        .on_change(move |value: &SharedString| ws.update_active_text(value))
        .on_selection_change(move |selections: &Vec<nsh_base::EditorSelection>| {
            if let Some(selection) = selections.last() {
                ws.cursor.set(
                    format!(
                        "Ln {}, Col {}",
                        selection.head.line + 1,
                        selection.head.column + 1
                    )
                    .into(),
                );
            }
        })
        .flex_1()
}

/// 编辑器各部分套上 IDEA 的岛屿配色。
fn idea_editor_classes() -> EditorClasses {
    let mut classes = EditorClasses::default();
    classes.gutter = Some("bg-editor-bg".into());
    classes.content = Some("bg-editor-bg text-editor-fg".into());
    classes.line_number = Some("text-line-number".into());
    classes.active_line_number = Some("text-active-line-number".into());
    classes.active_line = Some("bg-active-line".into());
    classes.selection = Some("bg-selection".into());
    classes.indent_guide = Some("bg-indent-guide".into());
    classes
}

fn welcome(shell: Shell) -> impl IntoElement {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(14.0))
        .child(
            text("Keel")
                .text_size(px(30.0))
                .font_weight(FontWeight::SEMIBOLD),
        )
        .child(
            text("Rust 原生 IDE · nsh 迁移 spike")
                .text_size(px(13.0))
                .text_color(move || color_of("muted-fg")),
        )
        .child(welcome_actions(shell))
}

fn welcome_actions(shell: Shell) -> impl IntoElement {
    view()
        .flex_row()
        .gap(px(8.0))
        .child(
            button()
                .accessibility_label("打开项目")
                .on_press(move || shell.pick_project())
                .h(px(30.0))
                .px(px(14.0))
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .bg(move || theme().primary)
                .active(|s| s.bg(move || color_of("brand-hover")))
                .child(
                    text("打开项目…")
                        .text_size(px(13.0))
                        .text_color(move || theme().primary_foreground),
                ),
        )
        .child(
            button()
                .accessibility_label("打开文件")
                .on_press(move || shell.pick_files())
                .h(px(30.0))
                .px(px(14.0))
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .border_1()
                .border_color(move || color_of("popup-border"))
                .hover(|s| s.bg(move || color_of("hover-overlay")))
                .child(text("打开文件…").text_size(px(13.0))),
        )
}

fn doc_name(ws: Workspace, path: &Path) -> SharedString {
    ws.docs.with(|docs| {
        docs.iter()
            .find(|doc| doc.path == path)
            .map(|doc| doc.name.clone())
            .unwrap_or_default()
    })
}

fn doc_dirty(ws: Workspace, path: &Path) -> bool {
    ws.docs.with(|docs| {
        docs.iter()
            .find(|doc| doc.path == path)
            .is_some_and(|doc| doc.dirty)
    })
}

fn doc_preview(ws: Workspace, path: &Path) -> bool {
    ws.docs.with(|docs| {
        docs.iter()
            .find(|doc| doc.path == path)
            .is_some_and(|doc| doc.preview)
    })
}

/// 当前文档相对项目根的路径片段（给状态栏 / 面包屑复用）。
pub fn active_path(ws: Workspace) -> Option<PathBuf> {
    ws.active_path()
}

/// 当前文档的语言名（状态栏）。
pub fn active_language(ws: Workspace) -> String {
    active_path(ws)
        .map(|path| language_label(&path))
        .unwrap_or_else(|| "Plain Text".to_owned())
}

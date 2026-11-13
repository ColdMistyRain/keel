//! 编辑器区：标签页、面包屑、编辑器与欢迎页（IDEA 的
//! `FileEditorManager` / `EditorsSplitters` 的最小实现）。
//!
//! 标签页支持未保存圆点、关闭按钮、横向溢出滚动与右键菜单；编辑器仍是
//! `nsh_base::editor`，后续 P3 再补 LSP 与查找替换。

use std::path::Path;

use nsh_base::{EditorClasses, button, context_menu, editor, theme};
use nsh_core::prelude::*;

use crate::icons;
use crate::shell::Shell;
use crate::ui::{color_of, island_radius, menu_action, popup_style, tab_height};
use crate::workspace::{Doc, Workspace, language_of};

/// 编辑器区（岛屿容器）。
pub fn editor_area(shell: Shell) -> impl IntoElement {
    view()
        .flex_1()
        .min_h(px(0.0))
        .rounded(px(island_radius()))
        .bg(move || color_of("editor-bg"))
        .child(tab_strip(shell))
        .child(view().h(px(1.0)).bg(move || color_of("editor-border")))
        .child(editor_pane(shell))
}

/// 标签条：IDEA Islands 的胶囊标签（`IslandsTabPainter`）。
///
/// 条高 36，标签高 28、圆角 6（arc 12），横向内缩 4；选中标签是
/// `#E3EBFE` 底色 + `#A7C5FF` 1px 边框，未选中没有底色。
/// 标签溢出时右侧出现左右滚动箭头（IDEA 的 `ScrollableSingleRowLayout`）。
fn tab_strip(shell: Shell) -> impl IntoElement {
    let ws = shell.ws;
    measure_tab_overflow(shell);
    view()
        .h(px(tab_height()))
        .flex_row()
        .items_center()
        .px(px(4.0))
        .child(
            scroll()
                .horizontal()
                .scrollbar(false)
                .h(px(tab_height()))
                .flex_1()
                .element_ref(&shell.tab_viewport)
                .child(
                    view()
                        .h(px(tab_height()))
                        .flex_row()
                        .items_center()
                        .gap(px(2.0))
                        .pl(px(3.0))
                        .element_ref(&shell.tab_content)
                        .child(each(
                            ws.docs,
                            |doc: &Doc| doc.path.display().to_string(),
                            move |doc, index| tab(shell, doc, index),
                        )),
                ),
        )
        .child(overflow_arrows(shell))
}

/// 打开 / 关闭标签或切换激活标签后，测量标签内容与视口宽度，决定是否显示箭头。
fn measure_tab_overflow(shell: Shell) {
    let viewport = shell.tab_viewport;
    let content = shell.tab_content;
    effect(move || {
        // 订阅文档与激活标签，变化后重新测量。
        let _count = shell.ws.docs.with(|docs| docs.len());
        let active = shell.ws.active.get_untracked();
        spawn(async move {
            let (Some(viewport_size), Some(content_size)) =
                (viewport.measure().await, content.measure().await)
            else {
                return;
            };
            let viewport_width = viewport_size.frame.size.width.0;
            let content_width = content_size.frame.size.width.0;
            // 布局稳定前会量到很小的宽度（例如 92），忽略这种中间值。
            if viewport_width < 200.0 {
                return;
            }
            shell.tab_viewport_width.set(viewport_width);
            shell.tab_content_width.set(content_width);
            let overflow = content_width > viewport_width + 0.5;
            shell.tab_overflow.set(overflow);
            if !overflow {
                shell.tab_scroll.set(0.0);
                viewport.scroll_to(point(px(0.0), px(0.0)), false);
            } else if active.is_some_and(|index| index + 1 == shell.ws.docs.with(|docs| docs.len()))
            {
                // 新打开的标签在最右侧：把它滚进视口。
                let end = (content_width - viewport_width).max(0.0);
                shell.tab_scroll.set(end);
                viewport.scroll_to(point(px(end), px(0.0)), false);
            }
        });
    });
}

/// 溢出时显示的左右滚动箭头。
fn overflow_arrows(shell: Shell) -> View {
    view()
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .pl(px(2.0))
        .style_when(move || !shell.tab_overflow.get(), |s| s.hidden())
        .child(scroll_arrow(shell, -1.0))
        .child(scroll_arrow(shell, 1.0))
}

fn scroll_arrow(shell: Shell, direction: f32) -> View {
    let left = direction < 0.0;
    let icon = if left {
        icons::CHEVRON_LEFT
    } else {
        icons::CHEVRON_RIGHT
    };
    let dimmed = move || {
        let available = (shell.tab_content_width.get() - shell.tab_viewport_width.get()).max(0.0);
        if left {
            shell.tab_scroll.get() <= 0.5
        } else {
            shell.tab_scroll.get() + 0.5 >= available
        }
    };
    view()
        .w(px(24.0))
        .h(px(28.0))
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .role(AccessibilityRole::Button)
        .label(if left {
            "向左滚动标签"
        } else {
            "向右滚动标签"
        })
        .hover(|s| s.bg(move || color_of("hover-overlay")))
        .style_when(dimmed, |s| s.opacity(0.4))
        .on_press(move || {
            let available = (shell.tab_content_width.get_untracked()
                - shell.tab_viewport_width.get_untracked())
            .max(0.0);
            let next = (shell.tab_scroll.get_untracked() + direction * 160.0).clamp(0.0, available);
            if next == shell.tab_scroll.get_untracked() {
                return;
            }
            shell.tab_scroll.set(next);
            shell.tab_viewport.scroll_to(point(px(next), px(0.0)), true);
        })
        .child(
            svg(icon)
                .w(px(12.0))
                .h(px(12.0))
                .text_color(move || color_of("muted-fg")),
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
    let selected_bg_active =
        move || ws.active.get() == Some(index.get()) && shell.editor_focused.get();
    let selected_bg_inactive =
        move || ws.active.get() == Some(index.get()) && !shell.editor_focused.get();
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
        .style_when(selected_bg_active, |s| {
            s.bg(move || color_of("tab-selected-bg"))
                .border_1()
                .border_color(move || color_of("tab-selected-border"))
        })
        .style_when(selected_bg_inactive, |s| {
            s.bg(move || color_of("tab-selected-bg-inactive"))
                .border_1()
                .border_color(move || color_of("tab-selected-border-inactive"))
        })
        .hover(|s| s.bg(move || color_of("control-hover")))
        .role(AccessibilityRole::Button)
        .label(move || doc_name(ws, &label_path))
        .on_press(move || {
            ws.activate_path(&select_path);
            shell.focus_editor();
        })
        .child(
            svg(icon)
                .w(px(16.0))
                .h(px(16.0))
                .text_color(move || color_of("secondary-fg"))
                .opacity(move || if selected_fg() { 1.0_f32 } else { 0.75_f32 }),
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
                    svg(icons::CLOSE_SMALL)
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

/// 状态栏左侧的面包屑（IDEA `NavBarLocation.BOTTOM`）。
///
/// 从项目根开始逐段渲染：文件夹用通用文件夹图标，最后一段用当前文件
/// 的类型图标；段与段之间是 12px 的 `chevronRight`（IDEA 的
/// `AllIcons.Ide.NavBarSeparator`）。每段高 22、左右内缩 6、圆角 2。
pub fn breadcrumbs(shell: Shell) -> impl IntoElement {
    let ws = shell.ws;
    dynamic(move || {
        let root = ws.root_name();
        let Some(root_path) = ws.root.get() else {
            return view().into_element();
        };
        let parts = ws.breadcrumbs();
        if root.is_empty() && parts.is_empty() {
            return view().into_element();
        }
        let dark = shell.appearance.get() == Appearance::Dark;
        let active_path = ws.active_path();
        let mut row = view().h(px(22.0)).flex_row().items_center().gap(px(2.0));
        let mut accumulated = root_path;
        let mut has_previous = false;
        if !root.is_empty() {
            let reveal = accumulated.clone();
            row = row.child(crumb(root, icons::FOLDER, move || {
                shell.reveal_path(&reveal)
            }));
            has_previous = true;
        }
        let last = parts.len().saturating_sub(1);
        for (index, part) in parts.into_iter().enumerate() {
            accumulated.push(part.as_ref());
            if has_previous {
                row = row.child(
                    svg(icons::CHEVRON_RIGHT)
                        .w(px(12.0))
                        .h(px(12.0))
                        .text_color(move || color_of("muted-fg")),
                );
            }
            let icon = if index == last {
                active_path
                    .as_deref()
                    .map(|path| icons::for_file(&path.display().to_string(), false, dark))
                    .unwrap_or(icons::FILE)
            } else {
                icons::FOLDER
            };
            let reveal = accumulated.clone();
            row = row.child(crumb(part, icon, move || shell.reveal_path(&reveal)));
            has_previous = true;
        }
        row.into_element()
    })
}

fn crumb(label: SharedString, icon: ImageAsset, action: impl Fn() + 'static) -> View {
    let accessible = format!("面包屑 {label}");
    view()
        .h(px(22.0))
        .px(px(6.0))
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .rounded(px(2.0))
        .role(AccessibilityRole::Button)
        .label(accessible)
        .hover(|s| s.bg(move || color_of("control-hover")))
        .on_press(action)
        .child(
            svg(icon)
                .w(px(16.0))
                .h(px(16.0))
                .text_color(move || color_of("secondary-fg")),
        )
        .child(
            text(label)
                .text_size(px(12.0))
                .text_color(move || color_of("muted-fg"))
                .number_of_lines(1),
        )
}

fn editor_pane(shell: Shell) -> impl IntoElement {
    let ws = shell.ws;
    view()
        .flex_1()
        .child(
            view()
                .flex_1()
                .style_when(move || ws.active.get().is_none(), |s| s.hidden())
                .child(code_editor(shell)),
        )
        .child(
            view()
                .flex_1()
                .style_when(move || ws.active.get().is_some(), |s| s.hidden())
                .child(welcome(shell)),
        )
}

fn code_editor(shell: Shell) -> impl IntoElement {
    let ws = shell.ws;
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
        .font_family(crate::fonts::MONO.family)
        .font_size(px(13.0))
        .line_height(px(20.0))
        .classes(idea_editor_classes())
        .on_change(move |value: &SharedString| ws.update_active_text(value))
        .on_selection_change(move |selections: &Vec<nsh_base::EditorSelection>| {
            shell.focus_editor();
            if let Some(selection) = selections.last() {
                ws.cursor.set(
                    format!("{}:{}", selection.head.line + 1, selection.head.column + 1).into(),
                );
            }
        })
        .flex_1()
}

/// 编辑器各部分套上 IDEA 的岛屿配色。
fn idea_editor_classes() -> EditorClasses {
    let mut classes = EditorClasses::default();
    classes.gutter = Some("bg-editor-bg border-r border-indent-guide".into());
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

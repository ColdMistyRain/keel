//! Keel 的 nsh 迁移 spike：IDEA 风格工作台外壳。
//!
//! P0 只验证 nsh 能否承载 Keel 的主界面，不改动仓库里 GPUI 版本的
//! `src/`。界面结构：
//!
//! - 自绘标题栏（窗口拖动、窗口按钮留白、应用菜单、主题切换）；
//! - 左侧工具窗口条 + 项目面板（`nsh_base::tree`）；
//! - 编辑器区：标签页、面包屑、`nsh_base::editor`；
//! - 底部状态栏。
//!
//! 颜色全部来自 `theme.json`（IDEA Islands Light/Dark 的 shadcn 令牌），
//! 不在组件里写死色值。

mod workspace;

use std::path::{Path, PathBuf};

use nsh_base::{EditorClasses, button, editor, menu, menu_item, theme};
use nsh_core::{element::desktop::title_bar, file_dialog::OpenDialog, prelude::*};

use crate::workspace::{Doc, TreeRow, Workspace, language_label, language_of};

/// 主题里没有的令牌一律退回透明，避免在组件里出现字面色值。
fn color_of(name: &str) -> AdaptiveColor {
    theme().color(name).unwrap_or(AdaptiveColor::TRANSPARENT)
}

/// 左侧工具窗口。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Project,
    Search,
    Version,
    Terminal,
}

impl Tool {
    fn glyph(self) -> &'static str {
        match self {
            Tool::Project => "▤",
            Tool::Search => "⌕",
            Tool::Version => "⑂",
            Tool::Terminal => "▥",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Tool::Project => "项目",
            Tool::Search => "搜索",
            Tool::Version => "版本控制",
            Tool::Terminal => "终端",
        }
    }
}

/// 应用菜单里用到的界面开关。
#[derive(Clone, Copy)]
struct Shell {
    ws: Workspace,
    tools: Signal<bool>,
    tool: Signal<Tool>,
    appearance: Signal<Appearance>,
}

/// 注册给 TSX 入口的 Rust 组件集合。
#[native_module("keel/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

/// 应用根组件，由 `nsh.json` 的 `"root": "App"` 指定。
#[component]
pub fn App() -> impl IntoElement {
    // 主题全部来自 themes/idea-*.json：启动时转成 nsh 的 shadcn 令牌，
    // 不再依赖 nsh.json 的 "theme" 或 Rust 里的色值常量。
    if let Ok(tokens) = nsh_base::tokens_of(Some(&keel_theme::nsh_theme_json())) {
        set_tokens(tokens);
    }

    let shell = Shell {
        ws: Workspace::new(),
        tools: signal(true),
        tool: signal(Tool::Project),
        appearance: signal(Appearance::System),
    };

    on_key("mod+o", move |_| pick_files(shell.ws));
    on_key("shift+mod+o", move |_| pick_project(shell.ws));
    on_key("mod+s", move |_| shell.ws.save_active());
    on_key("mod+w", move |_| shell.ws.close_active());

    view()
        .flex_1()
        .bg(move || color_of("canvas"))
        .text_color(move || theme().foreground)
        .child(title_bar_row(shell))
        .child(body(shell))
        .child(status_bar(shell.ws))
}

/// 自绘标题栏：40 点高，右侧留给系统窗口按钮。
fn title_bar_row(shell: Shell) -> impl IntoElement {
    view()
        .window_drag(true)
        .h(move || {
            let top = title_bar().top;
            if top > px(0.0) { top } else { px(40.0) }
        })
        .pl(move || title_bar().left + px(8.0))
        .pr(move || title_bar().right + px(8.0))
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .child(
            text("Keel")
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .pr(px(8.0)),
        )
        .child(
            menu()
                .offset(px(4.0))
                .popup_style(popup_style)
                .trigger(menu_trigger("文件"))
                .child(menu_action("打开项目…", move || {
                    pick_project(shell.ws)
                }))
                .child(menu_action("打开文件…", move || pick_files(shell.ws)))
                .child(menu_separator())
                .child(menu_action("保存", move || shell.ws.save_active()))
                .child(menu_action("关闭标签页", move || {
                    shell.ws.close_active()
                })),
        )
        .child(
            menu()
                .offset(px(4.0))
                .popup_style(popup_style)
                .trigger(menu_trigger("视图"))
                .child(menu_action("项目工具窗口", move || {
                    shell.tools.update(|tools| *tools = !*tools);
                }))
                .child(menu_action("切换到项目面板", move || {
                    shell.tool.set(Tool::Project);
                    shell.tools.set(true);
                }))
                .child(menu_action("浅色 / 深色", move || {
                    cycle_appearance(shell.appearance)
                }))
                .child(menu_action("关闭当前标签页", move || {
                    shell.ws.close_active()
                })),
        )
        .child(
            menu()
                .offset(px(4.0))
                .popup_style(popup_style)
                .trigger(menu_trigger("帮助"))
                .child(menu_action("关于 Keel", move || {
                    shell.ws.say("Keel 0.1.0 · nsh 0.5.0 spike · Rust UI");
                })),
        )
        .child(view().flex_1())
        .child(
            text(move || shell.ws.root_name())
                .text_size(px(12.0))
                .text_color(move || color_of("muted-fg"))
                .pr(px(8.0)),
        )
        .child(appearance_button(shell.appearance))
}

fn menu_trigger(label: &'static str) -> impl IntoElement {
    view()
        .h(px(28.0))
        .px(px(8.0))
        .items_center()
        .rounded(px(4.0))
        .hover(|s| s.bg(move || color_of("control-hover")))
        .child(text(label).text_size(px(12.0)))
}

fn popup_style(s: nsh_core::element::StyleRefinement) -> nsh_core::element::StyleRefinement {
    s.p_1()
        .rounded(px(8.0))
        .bg(move || theme().popover)
        .border_1()
        .border_color(move || color_of("popup-border"))
        .shadow_md()
}

fn menu_action(label: &'static str, action: impl Fn() + 'static) -> impl IntoElement {
    menu_item()
        .on_select(action)
        .px(px(10.0))
        .py(px(5.0))
        .rounded(px(4.0))
        .hover(|s| s.bg(move || theme().accent))
        .child(text(label).text_size(px(12.0)))
}

fn menu_separator() -> impl IntoElement {
    nsh_base::menu_separator()
        .h(px(1.0))
        .my(px(4.0))
        .bg(move || color_of("separator-line"))
}

fn appearance_button(appearance: Signal<Appearance>) -> impl IntoElement {
    view()
        .h(px(28.0))
        .px(px(8.0))
        .items_center()
        .rounded(px(4.0))
        .hover(|s| s.bg(move || color_of("control-hover")))
        .role(AccessibilityRole::Button)
        .label("切换外观")
        .on_press(move || cycle_appearance(appearance))
        .child(
            text(move || match appearance.get() {
                Appearance::System => "外观：自动",
                Appearance::Light => "外观：浅色",
                Appearance::Dark => "外观：深色",
            })
            .text_size(px(12.0))
            .text_color(move || color_of("muted-fg")),
        )
}

fn cycle_appearance(appearance: Signal<Appearance>) {
    let next = match appearance.get_untracked() {
        Appearance::System => Appearance::Light,
        Appearance::Light => Appearance::Dark,
        Appearance::Dark => Appearance::System,
    };
    appearance.set(next);
    set_color_scheme(next);
}

/// 工具窗口条 + 面板 + 编辑器区。
fn body(shell: Shell) -> impl IntoElement {
    view()
        .flex_1()
        .flex_row()
        .gap(px(4.0))
        .p(px(4.0))
        .child(tool_window_bar(shell))
        .child(side_panel(shell).style_when(move || !shell.tools.get(), |s| s.hidden()))
        .child(editor_area(shell.ws))
}

fn tool_window_bar(shell: Shell) -> impl IntoElement {
    let button = move |tool: Tool| {
        view()
            .w(px(32.0))
            .h(px(32.0))
            .items_center()
            .justify_center()
            .rounded(px(6.0))
            .role(AccessibilityRole::Button)
            .label(tool.label())
            .style_when(
                move || shell.tool.get() == tool && shell.tools.get(),
                |s| s.bg(move || color_of("control-hover")),
            )
            .hover(|s| s.bg(move || color_of("hover-overlay")))
            .on_press(move || {
                let same = shell.tool.get_untracked() == tool && shell.tools.get_untracked();
                shell.tool.set(tool);
                shell.tools.set(!same);
            })
            .child(text(tool.glyph()).text_size(px(16.0)).text_color(move || {
                if shell.tool.get() == tool && shell.tools.get() {
                    theme().foreground
                } else {
                    color_of("muted-fg")
                }
            }))
    };
    view()
        .w(px(40.0))
        .items_center()
        .gap(px(4.0))
        .py(px(6.0))
        .child(button(Tool::Project))
        .child(button(Tool::Search))
        .child(button(Tool::Version))
        .child(button(Tool::Terminal))
}

fn side_panel(shell: Shell) -> View {
    view()
        .w(px(260.0))
        .rounded(px(8.0))
        .bg(move || color_of("island-bg"))
        .child(panel_header(shell))
        .child(
            project_panel(shell.ws)
                .style_when(move || shell.tool.get() != Tool::Project, |s| s.hidden()),
        )
        .child(
            view()
                .flex_1()
                .items_center()
                .justify_center()
                .style_when(move || shell.tool.get() == Tool::Project, |s| s.hidden())
                .child(
                    text("该工具窗口的内容在 P2 迁移")
                        .text_size(px(12.0))
                        .text_color(move || color_of("muted-fg")),
                ),
        )
}

fn panel_header(shell: Shell) -> impl IntoElement {
    view()
        .h(px(32.0))
        .flex_row()
        .items_center()
        .px(px(10.0))
        .gap(px(6.0))
        .child(
            text(move || shell.tool.get().label())
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .flex_1(),
        )
        .child(
            view()
                .w(px(22.0))
                .h(px(22.0))
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .role(AccessibilityRole::Button)
                .label("收起全部")
                .hover(|s| s.bg(move || color_of("control-hover")))
                .on_press(move || shell.ws.expanded.set(Vec::new()))
                .child(
                    text("−")
                        .text_size(px(14.0))
                        .text_color(move || color_of("muted-fg")),
                ),
        )
}

fn project_panel(ws: Workspace) -> View {
    view().flex_1().child(project_tree(ws)).child(when(
        move || ws.root.get().is_none(),
        move || {
            view().p(px(12.0)).child(
                text("未打开项目")
                    .text_size(px(12.0))
                    .text_color(move || color_of("muted-fg")),
            )
        },
    ))
}

/// 项目树：自绘的 `scroll` 列表。
///
/// 没有用 `nsh_base::tree`：0.5.0 在 Windows 上树的行（父视口虚拟列表）
/// 不会绘制，`nsh test --trace` 里行已挂载但窗口里不可见。
/// 细节见 `docs/NSH_MIGRATION_PLAN.md` 的 P0 记录；P2 再评估或上报 nsh。
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
            } else if event.clicks >= 2 {
                let path = PathBuf::from(select_id.as_ref());
                if path.is_file() {
                    ws.open_files(vec![path]);
                }
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
                        "−"
                    } else {
                        "+"
                    })
                    .text_size(px(10.0))
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

/// 文件类型徽标：一个带底色的字母，先代替 IDEA 的文件图标。
fn file_badge(path: &str, folder: bool) -> impl IntoElement {
    let letter = if folder {
        "D".to_owned()
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

fn editor_area(ws: Workspace) -> impl IntoElement {
    view()
        .flex_1()
        .rounded(px(8.0))
        .bg(move || color_of("editor-bg"))
        .child(tab_strip(ws))
        .child(breadcrumbs(ws))
        .child(editor_pane(ws))
}

fn tab_strip(ws: Workspace) -> impl IntoElement {
    view()
        .h(px(34.0))
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .px(px(4.0))
        .child(
            view()
                .flex_1()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .child(each(
                    ws.docs,
                    |doc: &Doc| doc.path.display().to_string(),
                    move |doc, index| tab(ws, doc, index),
                )),
        )
}

fn tab(ws: Workspace, doc: ReadSignal<Doc>, index: ReadSignal<usize>) -> impl IntoElement {
    let path = doc.get().path;
    let label_path = path.clone();
    let select_path = path.clone();
    let close_path = path.clone();
    let name_path = path.clone();
    let dirty_path = path.clone();
    let selected = move || ws.active.get() == Some(index.get());
    view()
        .h(px(28.0))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(10.0))
        .rounded(px(4.0))
        .style_when(selected, |s| s.bg(move || color_of("tab-selected-bg")))
        .hover(|s| s.bg(move || color_of("control-hover")))
        .role(AccessibilityRole::Button)
        .label(move || doc_name(ws, &label_path))
        .on_press(move || activate_path(ws, &select_path))
        .child(
            text(move || doc_name(ws, &name_path))
                .text_size(px(12.0))
                .number_of_lines(1)
                .text_color(move || {
                    if selected() {
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
                .on_press(move || close_path_now(ws, &close_path))
                .child(
                    text("×")
                        .text_size(px(12.0))
                        .text_color(move || color_of("muted-fg")),
                ),
        )
}

fn breadcrumbs(ws: Workspace) -> impl IntoElement {
    dynamic(move || {
        let parts = ws.breadcrumbs();
        let mut text_value = String::new();
        for part in parts {
            if !text_value.is_empty() {
                text_value.push_str("  ›  ");
            }
            text_value.push_str(&part);
        }
        view().h(px(26.0)).px(px(12.0)).items_center().child(
            text(text_value)
                .text_size(px(11.0))
                .text_color(move || color_of("muted-fg"))
                .number_of_lines(1),
        )
    })
}

fn editor_pane(ws: Workspace) -> View {
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
                .child(welcome(ws)),
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

fn welcome(ws: Workspace) -> impl IntoElement {
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
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(welcome_actions(ws, false)),
        )
}

fn welcome_actions(ws: Workspace, compact: bool) -> impl IntoElement {
    let size = if compact { 12.0 } else { 13.0 };
    view()
        .flex_row()
        .gap(px(8.0))
        .p(if compact { px(8.0) } else { px(0.0) })
        .child(
            button()
                .accessibility_label("打开项目")
                .on_press(move || pick_project(ws))
                .h(px(30.0))
                .px(px(14.0))
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .bg(move || theme().primary)
                .active(|s| s.bg(move || color_of("brand-hover")))
                .child(
                    text("打开项目…")
                        .text_size(px(size))
                        .text_color(move || theme().primary_foreground),
                ),
        )
        .child(
            button()
                .accessibility_label("打开文件")
                .on_press(move || pick_files(ws))
                .h(px(30.0))
                .px(px(14.0))
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .border_1()
                .border_color(move || color_of("popup-border"))
                .hover(|s| s.bg(move || color_of("hover-overlay")))
                .child(text("打开文件…").text_size(px(size))),
        )
}

fn status_bar(ws: Workspace) -> impl IntoElement {
    view()
        .h(px(24.0))
        .px(px(10.0))
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .bg(move || color_of("status-bar"))
        .text_color(move || color_of("muted-fg"))
        .child(text("main").text_size(px(11.0)))
        .child(
            text(move || ws.message.get())
                .text_size(px(11.0))
                .number_of_lines(1),
        )
        .child(view().flex_1())
        .child(
            text(move || {
                active_path(ws)
                    .map(|path| language_label(&path))
                    .unwrap_or_else(|| "Plain Text".to_owned())
            })
            .text_size(px(11.0)),
        )
        .child(text("UTF-8").text_size(px(11.0)))
        .child(text("LF").text_size(px(11.0)))
        .child(text("4 spaces").text_size(px(11.0)))
        .child(text(move || ws.cursor.get()).text_size(px(11.0)))
}

fn active_path(ws: Workspace) -> Option<PathBuf> {
    ws.active.with(|active| {
        active.and_then(|index| {
            ws.docs
                .with(|docs| docs.get(index).map(|doc| doc.path.clone()))
        })
    })
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

fn activate_path(ws: Workspace, path: &Path) {
    if let Some(index) = ws
        .docs
        .with(|docs| docs.iter().position(|doc| doc.path == path))
    {
        ws.active.set(Some(index));
    }
}

fn close_path_now(ws: Workspace, path: &Path) {
    if let Some(index) = ws
        .docs
        .with(|docs| docs.iter().position(|doc| doc.path == path))
    {
        ws.close_doc(index);
    }
}

fn pick_project(ws: Workspace) {
    spawn(async move {
        match pick_paths(OpenDialog::folders().title("打开项目")).await {
            Ok(Some(paths)) => {
                if let Some(dir) = paths.into_iter().next() {
                    ws.open_project(dir);
                }
            }
            Ok(None) => {}
            Err(error) => ws.say(format!("打开项目失败：{error}")),
        }
    });
}

fn pick_files(ws: Workspace) {
    spawn(async move {
        match pick_paths(OpenDialog::files().multiple(true).title("打开文件")).await {
            Ok(Some(paths)) => ws.open_files(paths),
            Ok(None) => {}
            Err(error) => ws.say(format!("打开文件失败：{error}")),
        }
    });
}

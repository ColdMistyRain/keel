//! Keel 的 nsh 迁移 spike：IDEA 风格工作台外壳。
//!
//! 结构照 IDEA 的 `IdeRootPane`：标题栏（含应用菜单）→ 内容区（工具窗口条、
//! 可拖宽的工具窗口、编辑器区、底部工具窗口）→ 状态栏。菜单与快捷键共用
//! `keel-commands` 的动作注册表；颜色全部来自 `themes/idea-*.json`。

mod bottom_panel;
mod commands;
mod editor_area;
mod palette;
mod project_panel;
mod shell;
mod ui;
mod workspace;

use nsh_base::{h_resizable, resizable_panel, theme, v_resizable};
use nsh_core::keymap::When;
use nsh_core::{element::desktop::title_bar, prelude::*};

use crate::shell::{Bottom, Shell, Tool};
use crate::ui::{color_of, icon_button};

/// 注册给 TSX 入口的 Rust 组件集合。
#[native_module("keel/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

/// 应用根组件，由 `nsh.json` 的 `"root": "App"` 指定。
#[component]
pub fn App() -> impl IntoElement {
    // 主题全部来自 themes/idea-*.json：启动时转成 nsh 的 shadcn 令牌。
    if let Ok(tokens) = nsh_base::tokens_of(Some(&keel_theme::nsh_theme_json())) {
        set_tokens(tokens);
    }

    let shell = Shell::new();

    // 快捷键也从动作注册表来：菜单、快捷键、命令面板共用一份定义。
    for (shortcut, id) in shell.registry().shortcuts() {
        on_key(&shortcut, move |_| shell.perform(id));
    }
    // 命令面板导航：输入框聚焦时也生效，未打开时把按键原样放行。
    let keys = KeyOptions {
        when: When::Always,
        pass_through: true,
        ..Default::default()
    };
    on_key_with("up", keys, move |_| shell.palette_move(-1));
    on_key_with("down", keys, move |_| shell.palette_move(1));
    on_key_with("escape", keys, move |_| shell.close_palette());

    view()
        .flex_1()
        .bg(move || color_of("canvas"))
        .text_color(move || theme().foreground)
        .child(title_bar_row(shell))
        .child(body(shell))
        .child(status_bar(shell))
        .child(when(
            move || shell.palette.get(),
            move || palette::overlay(shell),
        ))
}

/// 自绘标题栏：40 点高，右侧留给系统窗口按钮；菜单来自动作注册表。
fn title_bar_row(shell: Shell) -> impl IntoElement {
    let registry = shell.registry();
    let menus = registry
        .groups()
        .into_iter()
        .map(|group| commands::app_menu(shell, group).into_element())
        .collect::<Vec<_>>();

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
        .children(menus)
        .child(
            text(move || shell.ws.root_name())
                .text_size(px(13.0))
                .text_color(move || color_of("muted-fg"))
                .number_of_lines(1)
                .pl(px(8.0)),
        )
        .child(view().flex_1())
        .child(icon_button("⌕", "搜索", move || {
            shell.toggle_tool(Tool::Search)
        }))
        .child(icon_button("▷", "运行", move || {
            shell.toggle_bottom(Bottom::Run)
        }))
        .child(icon_button("⚙", "设置", move || {
            shell.ws.say("设置页将在 P4 迁移")
        }))
}

/// 工具窗口条 + 左侧工具窗口 + 编辑器区（可拖宽、可折叠）。
fn body(shell: Shell) -> impl IntoElement {
    view()
        .flex_1()
        .flex_row()
        .gap(px(4.0))
        .p(px(4.0))
        .child(project_panel::tool_window_bar(shell))
        .child(main_area(shell))
}

fn main_area(shell: Shell) -> impl IntoElement {
    view().flex_1().h_full().child(
        h_resizable()
            .gap(px(4.0))
            .handle_class("w-px bg-separator-line")
            .child(
                resizable_panel()
                    .size(px(260.0))
                    .size_range(px(180.0)..px(460.0))
                    .visible(move || shell.tools.get())
                    .child(project_panel::side_panel(shell)),
            )
            .child(resizable_panel().child(editor_column(shell))),
    )
}

/// 编辑器 + 底部工具窗口（垂直分栏）。
fn editor_column(shell: Shell) -> impl IntoElement {
    view().flex_1().h_full().child(
        v_resizable()
            .gap(px(4.0))
            .handle_class("h-px bg-separator-line")
            .child(resizable_panel().child(editor_area::editor_area(shell)))
            .child(
                resizable_panel()
                    .size(px(170.0))
                    .size_range(px(90.0)..px(360.0))
                    .visible(move || shell.bottom.get().is_some())
                    .child(bottom_panel::bottom_panel(shell)),
            ),
    )
}

/// 状态栏：底部工具窗口条、提示、语言、编码、换行、缩进、光标位置。
fn status_bar(shell: Shell) -> impl IntoElement {
    let ws = shell.ws;
    view()
        .h(px(24.0))
        .px(px(6.0))
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .bg(move || color_of("status-bar"))
        .text_color(move || color_of("muted-fg"))
        .child(bottom_panel::bottom_bar(shell))
        .child(
            view()
                .w(px(1.0))
                .h(px(14.0))
                .bg(move || color_of("separator-line")),
        )
        .child(
            text(move || ws.message.get())
                .text_size(px(12.0))
                .number_of_lines(1),
        )
        .child(view().flex_1())
        .child(text("main").text_size(px(12.0)))
        .child(text(move || editor_area::active_language(ws)).text_size(px(12.0)))
        .child(text("UTF-8").text_size(px(12.0)))
        .child(text("LF").text_size(px(12.0)))
        .child(text("4 spaces").text_size(px(12.0)))
        .child(text(move || ws.cursor.get()).text_size(px(12.0)))
}

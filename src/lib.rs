//! Keel 的 nsh 迁移 spike：IDEA 风格工作台外壳。
//!
//! 结构照 IDEA 的 `IdeRootPane`：标题栏（含应用菜单）→ 内容区（工具窗口条、
//! 可拖宽的工具窗口、编辑器区、底部工具窗口）→ 状态栏。菜单与快捷键共用
//! `keel-commands` 的动作注册表；颜色全部来自 `themes/idea-*.json`。

mod bottom_panel;
mod commands;
mod editor_area;
mod icons;
mod palette;
mod project_panel;
mod run_widget;
mod shell;
mod ui;
mod workspace;

use nsh_base::{h_resizable, menu, resizable_panel, theme, v_resizable};
use nsh_core::keymap::When;
use nsh_core::{element::desktop::title_bar, prelude::*};

use crate::shell::Shell;
use crate::ui::{color_of, menu_action, popup_style, project_badge_rgb, shade_rgb, toolbar_button};

/// 注册给 TSX 入口的 Rust 组件集合与随包资源。
#[native_module("keel/native")]
fn native(registry: &mut Registry) {
    registry.assets(assets!());
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
        .child(project_gradient(shell))
        .child(title_bar_row(shell))
        .child(body(shell))
        .child(status_bar(shell))
        .child(when(
            move || shell.palette.get(),
            move || palette::overlay(shell),
        ))
}

/// 自绘标题栏：40 点高，右侧留给系统窗口按钮。
///
/// 菜单不常驻，收进左上角的汉堡按钮（IDEA 新 UI 的 `MainMenuButton`）。
/// 左区是 IDEA 的项目头像 / 项目名 / 分支控件，右侧是 run widget、搜索
/// 图标和设置（2026.2 主工具栏默认不使用常驻宽搜索框）。
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
        .child(commands::main_menu(shell))
        .child(project_widget(shell))
        .child(branch_widget(shell))
        .child(view().flex_1())
        .child(run_widget::run_widget(shell))
        .child(toolbar_button(icons::SEARCH, "搜索操作", move || {
            shell.open_palette()
        }))
        .child(toolbar_button(icons::SETTINGS, "设置", move || {
            shell.ws.say("设置页将在 P4 迁移")
        }))
}

/// 项目左上角的项目色渐变（IDEA `IslandsGradientPainter`）。
///
/// 600×300 的径向渐变铺在帧背景上，颜色取自项目色调色板；工具窗口、
/// 编辑器与状态栏是不透明岛屿，只有标题栏和条纹会透出这层颜色。
fn project_gradient(shell: Shell) -> impl IntoElement {
    dynamic(move || {
        let root = shell.ws.root_name();
        if root.is_empty() {
            return view().into_element();
        }
        let dark = shell.appearance.get() == Appearance::Dark;
        let (r, g, b) = crate::ui::project_gradient_rgb(root.as_ref(), dark);
        // 1200×600 的盒子放在 (-600,-300)：径向渐变的中心正好落在窗口
        // 左上角，farthest-corner 的可见范围是向右 600、向下 300，与 IDEA
        // 的 `600px 300px at 0 0` 一致。
        let color = (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
        view()
            .absolute()
            .top(px(-300.0))
            .left(px(-600.0))
            .w(px(1200.0))
            .h(px(600.0))
            .bg_radial()
            .gradient_from(rgb(color).alpha(0.45))
            .gradient_to(rgb(color).alpha(0.0))
            .pointer_events(nsh_core::PointerEvents::None)
            .into_element()
    })
}

/// 项目控件：20×20 项目色头像 + 项目名 + 下拉箭头。
///
/// 头像用项目徽标色做 135° 渐变（IDEA `ProjectToolbarWidgetAction`）；
/// 没有打开项目时退回产品名。点击弹出项目动作。
fn project_widget(shell: Shell) -> impl IntoElement {
    let ws = shell.ws;
    dynamic(move || {
        let name = ws.root_name();
        if name.is_empty() {
            return text("Keel")
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .pr(px(8.0))
                .into_element();
        }
        let dark = shell.appearance.get() == Appearance::Dark;
        let badge = project_badge_rgb(name.as_ref(), dark);
        let badge_end = shade_rgb(badge, 0.86);
        let letter = name
            .chars()
            .next()
            .map(|ch| ch.to_uppercase().to_string())
            .unwrap_or_default();
        let trigger = view()
            .h(px(30.0))
            .px(px(8.0))
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .rounded(px(8.0))
            .role(AccessibilityRole::Button)
            .label("项目菜单")
            .hover(|s| s.bg(move || color_of("control-hover")))
            .child(
                view()
                    .w(px(20.0))
                    .h(px(20.0))
                    .items_center()
                    .justify_center()
                    .rounded(px(4.0))
                    .bg_linear(deg(135.0))
                    .gradient_from(rgb(badge))
                    .gradient_to(rgb(badge_end))
                    .child(
                        text(letter)
                            .text_size(px(10.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(move || theme().primary_foreground),
                    ),
            )
            .child(
                text(name)
                    .text_size(px(13.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .number_of_lines(1),
            )
            .child(
                svg(icons::CHEVRON_DOWN)
                    .w(px(12.0))
                    .h(px(12.0))
                    .text_color(move || color_of("muted-fg")),
            );
        menu()
            .offset(px(4.0))
            .popup_style(popup_style)
            .trigger(trigger)
            .child(menu_action(
                Some(icons::FOLDER),
                "打开项目…",
                Some('P'),
                None,
                move || shell.pick_project(),
            ))
            .child(menu_action(
                Some(icons::REFRESH),
                "刷新项目",
                Some('R'),
                None,
                move || shell.ws.refresh_project(),
            ))
            .into_element()
    })
}

/// 分支控件：分支图标 + 当前分支 + 下拉箭头（IDEA 的 VCS 工具栏控件）。
fn branch_widget(shell: Shell) -> impl IntoElement {
    view()
        .h(px(30.0))
        .px(px(8.0))
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .rounded(px(6.0))
        .style_when(move || shell.ws.root_name().is_empty(), |s| s.hidden())
        .child(
            svg(icons::BRANCH)
                .w(px(16.0))
                .h(px(16.0))
                .text_color(move || color_of("muted-fg")),
        )
        .child(text("main").text_size(px(13.0)))
        .child(
            svg(icons::CHEVRON_DOWN)
                .w(px(12.0))
                .h(px(12.0))
                .text_color(move || color_of("muted-fg")),
        )
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

/// 状态栏：左侧面包屑（临时提示优先），右侧行列 / 换行 / 编码 / 缩进。
///
/// IDEA 默认把导航栏（面包屑）放在状态栏左侧，状态项从位置开始依次为
/// `行:列`、换行符、编码、缩进；分支在标题栏，不再重复显示。
fn status_bar(shell: Shell) -> impl IntoElement {
    let ws = shell.ws;
    view()
        .h(px(28.0))
        .px(px(10.0))
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .bg(move || color_of("status-bar"))
        .text_color(move || color_of("muted-fg"))
        .child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .flex_row()
                .items_center()
                .child(dynamic(move || {
                    let message = ws.message.get();
                    if message.is_empty() {
                        editor_area::breadcrumbs(shell).into_element()
                    } else {
                        view()
                            .h(px(22.0))
                            .px(px(6.0))
                            .items_center()
                            .child(
                                text(message)
                                    .text_size(px(12.0))
                                    .text_color(move || color_of("muted-fg"))
                                    .number_of_lines(1),
                            )
                            .into_element()
                    }
                })),
        )
        .child(text(move || ws.cursor.get()).text_size(px(12.0)))
        .child(text("LF").text_size(px(12.0)))
        .child(text("UTF-8").text_size(px(12.0)))
        .child(text("4 个空格").text_size(px(12.0)))
}

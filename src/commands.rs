//! 动作注册表：菜单栏、快捷键共用一份定义（IDEA 的 `ActionManager`）。
//!
//! 菜单只负责把 `keel-commands` 解析出的节点画出来；可用性、选中态和执行
//! 都回到注册表，后续的命令面板可以直接复用同一批动作。

use keel_commands::{
    ActionGroup, ActionId, ActionRegistry, CallbackAction, MenuNode, Presentation,
};
use nsh_base::{menu, menu_item, submenu, theme};
use nsh_core::prelude::*;

use crate::shell::{Bottom, Shell, Tool, UiState, appearance_id};
use crate::ui::{color_of, menu_separator, menu_trigger, popup_style, submenu_trigger};

/// 注册全部动作与菜单分组。
pub fn build(shell: Shell) -> ActionRegistry {
    let mut registry = ActionRegistry::new();

    registry.register(
        CallbackAction::new("file.open_project", "打开项目…", move |_| {
            shell.pick_project()
        })
        .description("选择目录作为项目打开")
        .shortcut("shift+mod+o"),
    );
    registry.register(
        CallbackAction::new("file.open", "打开文件…", move |_| shell.pick_files())
            .description("打开一个或多个文件")
            .shortcut("mod+o"),
    );
    registry.register(
        CallbackAction::new("file.save", "保存", move |_| shell.ws.save_active())
            .shortcut("mod+s")
            .enabled(|context| state(context).is_some_and(|state| state.can_save)),
    );
    registry.register(
        CallbackAction::new("file.close", "关闭标签页", move |_| {
            shell.ws.close_active()
        })
        .shortcut("mod+w")
        .enabled(|context| state(context).is_some_and(|state| state.has_editor)),
    );
    registry.register(
        CallbackAction::new("file.close_others", "关闭其他标签页", move |_| {
            if let Some(index) = shell.ws.active.get_untracked() {
                shell.ws.close_others(index);
            }
        })
        .enabled(|context| state(context).is_some_and(|state| state.open_docs > 1)),
    );
    registry.register(
        CallbackAction::new("file.close_all", "关闭所有标签页", move |_| {
            shell.ws.close_all()
        })
        .enabled(|context| state(context).is_some_and(|state| state.open_docs > 0)),
    );

    for tool in Tool::ALL {
        let action = CallbackAction::new(tool.action_id(), tool.label(), move |_| {
            shell.toggle_tool(tool)
        })
        .checked(move |context| {
            state(context)
                .is_some_and(|state| state.tool_window_visible && state.active_tool == tool.id())
        });
        let action = match tool {
            Tool::Project => action.shortcut("alt+1"),
            Tool::Search | Tool::Version => action,
        };
        registry.register(action);
    }

    for bottom in Bottom::ALL {
        let action = CallbackAction::new(bottom.action_id(), bottom.label(), move |_| {
            shell.toggle_bottom(bottom)
        })
        .checked(move |context| {
            state(context)
                .is_some_and(|state| state.bottom_visible && state.active_bottom == bottom.id())
        });
        let action = match bottom {
            Bottom::Problems => action.shortcut("alt+6"),
            Bottom::Terminal => action.shortcut("alt+f12"),
            Bottom::Run => action,
        };
        registry.register(action);
    }

    for (id, label, value) in [
        ("view.appearance.system", "跟随系统", Appearance::System),
        ("view.appearance.light", "浅色", Appearance::Light),
        ("view.appearance.dark", "深色", Appearance::Dark),
    ] {
        registry.register(
            CallbackAction::new(id, label, move |_| shell.set_appearance(value)).checked(
                move |context| {
                    state(context).is_some_and(|state| state.appearance == appearance_id(value))
                },
            ),
        );
    }

    registry.register(
        CallbackAction::new("navigate.reveal", "在项目中定位", move |_| {
            shell.reveal_active()
        })
        .description("在项目树里展开并选中当前文件")
        .enabled(|context| {
            state(context).is_some_and(|state| state.project_open && state.has_editor)
        }),
    );
    registry.register(
        CallbackAction::new("help.find_action", "搜索操作…", move |_| {
            shell.open_palette()
        })
        .description("按名称搜索菜单动作")
        .shortcut("mod+shift+a"),
    );
    registry.register(
        CallbackAction::new("help.about", "关于 Keel", move |_| shell.about())
            .description("版本与运行时信息"),
    );

    let tool_windows = ActionGroup::new("menu.view.tool_windows", "工具窗口")
        .add_action(Tool::Project.action_id())
        .add_action(Tool::Search.action_id())
        .add_action(Tool::Version.action_id());
    let bottom_windows = ActionGroup::new("menu.view.bottom", "底部工具窗口")
        .add_action(Bottom::Problems.action_id())
        .add_action(Bottom::Run.action_id())
        .add_action(Bottom::Terminal.action_id());
    let appearance = ActionGroup::new("menu.view.appearance", "外观")
        .add_action("view.appearance.system")
        .add_action("view.appearance.light")
        .add_action("view.appearance.dark");

    registry.register_group(
        ActionGroup::new("menu.file", "文件")
            .add_action("file.open_project")
            .add_action("file.open")
            .add_separator()
            .add_action("file.save")
            .add_separator()
            .add_action("file.close")
            .add_action("file.close_others")
            .add_action("file.close_all"),
    );
    registry.register_group(
        ActionGroup::new("menu.view", "视图")
            .add_group(tool_windows)
            .add_separator()
            .add_group(bottom_windows)
            .add_separator()
            .add_group(appearance),
    );
    registry
        .register_group(ActionGroup::new("menu.navigate", "导航").add_action("navigate.reveal"));
    registry.register_group(
        ActionGroup::new("menu.help", "帮助")
            .add_action("help.find_action")
            .add_separator()
            .add_action("help.about"),
    );

    registry
}

fn state(context: &keel_commands::DataContext) -> Option<&UiState> {
    context.get::<UiState>()
}

/// 把动作组渲染成标题栏菜单。
pub fn app_menu(shell: Shell, group: &ActionGroup) -> impl IntoElement {
    let mut element = menu()
        .offset(px(4.0))
        .popup_style(popup_style)
        .trigger(menu_trigger(group.text()));
    for node in shell.registry().menu(group.id(), &shell.context()) {
        element = element.child(menu_node(shell, node));
    }
    element
}

fn menu_node(shell: Shell, node: MenuNode) -> AnyElement {
    match node {
        MenuNode::Action { id, presentation } => action_item(shell, id, presentation),
        MenuNode::Separator => menu_separator().into_element(),
        MenuNode::Submenu { text, entries, .. } => {
            let mut element = submenu().offset(px(4.0)).trigger(submenu_trigger(text));
            for entry in entries {
                element = element.child(menu_node(shell, entry));
            }
            element.into_element()
        }
    }
}

/// 菜单项：可用性、勾选态和执行都回到注册表。
fn action_item(shell: Shell, id: ActionId, presentation: Presentation) -> AnyElement {
    let label = SharedString::from(presentation.text);
    let shortcut = SharedString::from(presentation.shortcut.unwrap_or_default());
    menu_item()
        .disabled(move || !shell.action_enabled(id))
        .checked(move || shell.action_checked(id))
        .on_select(move || shell.perform(id))
        .px(px(10.0))
        .py(px(5.0))
        .rounded(px(4.0))
        .hover(|s| s.bg(move || theme().accent))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(20.0))
                .child(text(label).text_size(px(12.0)).flex_1())
                .child(
                    text(shortcut)
                        .text_size(px(11.0))
                        .text_color(move || color_of("muted-fg")),
                ),
        )
        .into_element()
}

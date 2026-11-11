mod actions;
mod app;
mod editor;
mod file_icons;
mod idea_theme;
mod model;
mod panels;
mod project;
mod project_tree;
mod tool_window;

use gpui::{prelude::*, *};
use gpui_component_assets::AllAssets;

use crate::actions::{
    CloseEditor, OpenProject, PaletteCancel, PaletteConfirm, PaletteDown, PaletteUp, SaveFile,
    ShowCommandPalette, ToggleBottomPanel, ToggleProjectPanel,
};
use crate::app::KeelApp;
use crate::model::PersistedState;

fn main() {
    gpui_platform::application()
        .with_assets(AllAssets)
        .run(|cx| {
            gpui_component::init(cx);
            // 关闭 ClearType 次像素抗锯齿，避免字形边缘出现彩色光晕。
            cx.set_text_rendering_mode(TextRenderingMode::Grayscale);
            idea_theme::apply(cx);
            register_key_bindings(cx);
            install_menus(cx);

            let persisted = PersistedState::load();
            let width = persisted.window_width.unwrap_or(1280.);
            let height = persisted.window_height.unwrap_or(820.);
            let bounds = Bounds::centered(None, size(px(width), px(height)), cx);

            cx.open_window(
                WindowOptions::default()
                    .window_bounds(WindowBounds::Windowed(bounds))
                    .titlebar(Some(TitlebarOptions {
                        title: Some("Keel".into()),
                        ..Default::default()
                    })),
                move |window, cx| {
                    let view = cx.new(|cx| KeelApp::new(window, persisted, cx));
                    cx.new(|cx| gpui_component::Root::new(view, window, cx))
                },
            )
            .expect("无法创建主窗口");

            cx.activate(true);
        });
}

/// 顶部菜单栏的内容。菜单项全部绑定到真实存在的动作，不留空壳。
fn install_menus(cx: &mut App) {
    cx.set_menus(build_menus());
    let owned = build_menus()
        .into_iter()
        .map(|menu| menu.owned())
        .collect::<Vec<_>>();
    gpui_component::GlobalState::global_mut(cx).set_app_menus(owned);
}

fn build_menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "文件".into(),
            items: vec![
                MenuItem::action("打开项目目录…", OpenProject),
                MenuItem::separator(),
                MenuItem::action("保存", SaveFile),
                MenuItem::action("关闭编辑器", CloseEditor),
            ],
            disabled: false,
        },
        Menu {
            name: "视图".into(),
            items: vec![
                MenuItem::action("显示或隐藏项目面板", ToggleProjectPanel),
                MenuItem::action("显示或隐藏底部面板", ToggleBottomPanel),
                MenuItem::separator(),
                MenuItem::action("命令面板…", ShowCommandPalette),
            ],
            disabled: false,
        },
    ]
}

fn register_key_bindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-shift-o", OpenProject, Some("Keel")),
        KeyBinding::new("ctrl-s", SaveFile, Some("Keel")),
        KeyBinding::new("ctrl-w", CloseEditor, Some("Keel")),
        KeyBinding::new("alt-1", ToggleProjectPanel, Some("Keel")),
        KeyBinding::new("alt-4", ToggleBottomPanel, Some("Keel")),
        KeyBinding::new("ctrl-shift-p", ShowCommandPalette, Some("Keel")),
        KeyBinding::new("enter", PaletteConfirm, Some("KeelPalette")),
        KeyBinding::new("escape", PaletteCancel, Some("KeelPalette")),
        KeyBinding::new("up", PaletteUp, Some("KeelPalette")),
        KeyBinding::new("down", PaletteDown, Some("KeelPalette")),
    ]);
}

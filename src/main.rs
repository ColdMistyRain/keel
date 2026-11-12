mod actions;
mod app;
mod assets;
mod editor;
mod file_icons;
mod idea_theme;
mod model;
mod panels;
mod project;
mod project_tree;
mod tool_window;

use gpui::{prelude::*, *};
use gpui_component::input::{Copy, Cut, Paste, Redo, SelectAll, Undo};

use crate::actions::{
    CloseAllTabs, CloseEditor, CloseOtherTabs, CloseSettings, CollapseProjectTree, FindInFile,
    LocateInProject, NextTab, OpenProject, PaletteCancel, PaletteConfirm, PaletteDown, PaletteUp,
    PreviousTab, RefreshProjectTree, ReplaceInFile, SaveFile, ShowAbout, ShowCommandPalette,
    ShowSettings, ToggleBottomPanel, ToggleBreadcrumbs, ToggleProjectPanel, ToggleTheme,
};
use crate::app::KeelApp;
use crate::model::PersistedState;

fn main() {
    gpui_platform::application()
        .with_assets(assets::KeelAssets)
        .run(|cx| {
            // IDEA 新 UI 用 Inter，随程序打包，避免依赖系统是否安装。
            cx.text_system()
                .add_fonts(vec![
                    std::borrow::Cow::Borrowed(
                        include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice(),
                    ),
                    std::borrow::Cow::Borrowed(
                        include_bytes!("../assets/fonts/Inter-SemiBold.ttf").as_slice(),
                    ),
                ])
                .expect("无法加载 Inter 字体");
            gpui_component::init(cx);
            // 先读配置，保证启动时就是上次使用的主题与文本渲染方式，避免闪一下浅色。
            let persisted = PersistedState::load();
            // 关闭 ClearType 次像素抗锯齿，避免字形边缘出现彩色光晕；用户可在设置里改回。
            cx.set_text_rendering_mode(if persisted.subpixel_text.unwrap_or(false) {
                TextRenderingMode::Subpixel
            } else {
                TextRenderingMode::Grayscale
            });
            let theme = if persisted.theme_dark.unwrap_or(false) {
                idea_theme::ThemeKind::Dark
            } else {
                idea_theme::ThemeKind::Light
            };
            idea_theme::apply(cx, theme);
            register_key_bindings(cx);
            install_menus(cx);

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
    let menu = |name: &str, items: Vec<MenuItem>| Menu {
        name: name.into(),
        items,
        disabled: false,
    };

    vec![
        menu(
            "文件",
            vec![
                MenuItem::action("打开项目目录…", OpenProject),
                MenuItem::separator(),
                MenuItem::action("保存", SaveFile),
                MenuItem::action("关闭编辑器", CloseEditor),
                MenuItem::separator(),
                MenuItem::action("设置…", ShowSettings),
            ],
        ),
        // 编辑菜单直接复用编辑器的撤销/剪贴板动作，作用于当前聚焦的编辑区。
        menu(
            "编辑",
            vec![
                MenuItem::action("撤销", Undo),
                MenuItem::action("重做", Redo),
                MenuItem::separator(),
                MenuItem::action("剪切", Cut),
                MenuItem::action("复制", Copy),
                MenuItem::action("粘贴", Paste),
                MenuItem::separator(),
                MenuItem::action("全选", SelectAll),
                MenuItem::separator(),
                MenuItem::action("查找…", FindInFile),
                MenuItem::action("替换…", ReplaceInFile),
            ],
        ),
        menu(
            "视图",
            vec![
                MenuItem::action("显示或隐藏项目面板", ToggleProjectPanel),
                MenuItem::action("显示或隐藏底部面板", ToggleBottomPanel),
                MenuItem::action("显示或隐藏导航栏面包屑", ToggleBreadcrumbs),
                MenuItem::separator(),
                MenuItem::action("切换深色 / 浅色主题", ToggleTheme),
                MenuItem::separator(),
                MenuItem::action("命令面板…", ShowCommandPalette),
            ],
        ),
        menu(
            "导航",
            vec![
                MenuItem::action("上一个标签", PreviousTab),
                MenuItem::action("下一个标签", NextTab),
                MenuItem::separator(),
                MenuItem::action("在项目中定位", LocateInProject),
            ],
        ),
        menu(
            "工具",
            vec![
                MenuItem::action("从磁盘重新加载项目", RefreshProjectTree),
                MenuItem::action("折叠全部目录", CollapseProjectTree),
            ],
        ),
        menu(
            "窗口",
            vec![
                MenuItem::action("关闭其他标签", CloseOtherTabs),
                MenuItem::action("关闭全部标签", CloseAllTabs),
            ],
        ),
        menu("帮助", vec![MenuItem::action("关于 Keel…", ShowAbout)]),
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
        KeyBinding::new("ctrl-alt-s", ShowSettings, Some("Keel")),
        KeyBinding::new("ctrl-f", FindInFile, Some("Keel")),
        KeyBinding::new("ctrl-r", ReplaceInFile, Some("Keel")),
        KeyBinding::new("escape", CloseSettings, Some("KeelSettings")),
        KeyBinding::new("ctrl-tab", NextTab, Some("Keel")),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, Some("Keel")),
        KeyBinding::new("alt-f1", LocateInProject, Some("Keel")),
        KeyBinding::new("enter", PaletteConfirm, Some("KeelPalette")),
        KeyBinding::new("escape", PaletteCancel, Some("KeelPalette")),
        KeyBinding::new("up", PaletteUp, Some("KeelPalette")),
        KeyBinding::new("down", PaletteDown, Some("KeelPalette")),
    ]);
}

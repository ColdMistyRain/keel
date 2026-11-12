mod actions;
mod app;
mod assets;
mod editor;
mod file_icons;
mod idea_theme;
mod menu;
mod model;
mod panels;
mod project;
mod project_tree;
mod tool_window;
mod tooltip;

use gpui::{prelude::*, *};

use crate::actions::{
    CloseEditor, CloseSettings, FindInFile, LocateInProject, NextTab, OpenProject, PaletteCancel,
    PaletteConfirm, PaletteDown, PaletteUp, PreviousTab, ReplaceInFile, SaveFile,
    ShowCommandPalette, ShowSettings, ToggleBottomPanel, ToggleProjectPanel,
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
                    std::borrow::Cow::Borrowed(
                        include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice(),
                    ),
                    std::borrow::Cow::Borrowed(
                        include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf").as_slice(),
                    ),
                ])
                .expect("无法加载 Keel 的内置字体");
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

            let width = persisted.window_width.unwrap_or(1280.);
            let height = persisted.window_height.unwrap_or(820.);
            let bounds = Bounds::centered(None, size(px(width), px(height)), cx);

            cx.open_window(
                WindowOptions::default()
                    .window_bounds(WindowBounds::Windowed(bounds))
                    .titlebar(Some(TitlebarOptions {
                        title: Some("Keel".into()),
                        // IDEA 新 UI 自绘标题栏，系统标题栏让位给顶部主工具栏。
                        appears_transparent: true,
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

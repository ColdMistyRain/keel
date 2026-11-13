//! 菜单与工具窗口图标、助记符。
//!
//! 除应用图标和调色板图标外，全部使用 IntelliJ 社区版 expui 原图标
//! （Apache 2.0，SVG 内保留 JetBrains 版权头），单色图标已改为
//! `currentColor` 以跟随深浅主题。

use nsh_core::prelude::*;

pub const APP: ImageAsset = asset!("icons/app.svg");
pub const MENU: ImageAsset = asset!("icons/expui/menu.svg");
pub const MORE: ImageAsset = asset!("icons/expui/more.svg");
pub const HIDE: ImageAsset = asset!("icons/expui/hide.svg");
pub const FOLDER: ImageAsset = asset!("icons/expui/folder.svg");
pub const PROJECT: ImageAsset = asset!("icons/expui/project.svg");
pub const PROJECT_DARK: ImageAsset = asset!("icons/expui/project_dark.svg");
pub const PROJECT_TOOLWINDOW: ImageAsset = asset!("icons/expui/toolwindow_project.svg");
pub const PROJECT_TOOLWINDOW_20: ImageAsset = asset!("icons/expui/toolwindow_project_20.svg");
pub const MAVEN: ImageAsset = asset!("icons/expui/maven.svg");
pub const LIBRARY: ImageAsset = asset!("icons/expui/library.svg");
pub const STRUCTURE: ImageAsset = asset!("icons/expui/toolwindow_structure.svg");
pub const STRUCTURE_20: ImageAsset = asset!("icons/expui/toolwindow_structure_20.svg");
pub const SERVICES: ImageAsset = asset!("icons/expui/toolwindow_services.svg");
pub const SERVICES_20: ImageAsset = asset!("icons/expui/toolwindow_services_20.svg");
pub const FILE: ImageAsset = asset!("icons/expui/file.svg");
pub const JAVA: ImageAsset = asset!("icons/expui/java.svg");
pub const JAVA_DARK: ImageAsset = asset!("icons/expui/java_dark.svg");
pub const XML: ImageAsset = asset!("icons/expui/xml.svg");
pub const XML_DARK: ImageAsset = asset!("icons/expui/xml_dark.svg");
pub const SAVE: ImageAsset = asset!("icons/expui/save.svg");
pub const CLOSE: ImageAsset = asset!("icons/expui/close.svg");
pub const SEARCH: ImageAsset = asset!("icons/expui/search.svg");
pub const SEARCH_20: ImageAsset = asset!("icons/expui/search_20.svg");
pub const BRANCH: ImageAsset = asset!("icons/expui/branch.svg");
pub const WARNING: ImageAsset = asset!("icons/expui/toolwindow_problems.svg");
pub const PROBLEMS_20: ImageAsset = asset!("icons/expui/toolwindow_problems_20.svg");
pub const PLAY: ImageAsset = asset!("icons/expui/play.svg");
pub const TERMINAL: ImageAsset = asset!("icons/expui/terminal.svg");
pub const PALETTE: ImageAsset = asset!("icons/palette.svg");
pub const SETTINGS: ImageAsset = asset!("icons/expui/settings.svg");
pub const SETTINGS_20: ImageAsset = asset!("icons/expui/settings_20.svg");
pub const LOCATE: ImageAsset = asset!("icons/expui/locate.svg");
pub const INFO: ImageAsset = asset!("icons/expui/info.svg");
pub const EYE: ImageAsset = asset!("icons/expui/preview.svg");
pub const COPY: ImageAsset = asset!("icons/expui/copy.svg");
pub const REFRESH: ImageAsset = asset!("icons/expui/refresh.svg");
pub const COLLAPSE_ALL: ImageAsset = asset!("icons/expui/collapse_all.svg");
pub const CHEVRON_RIGHT: ImageAsset = asset!("icons/expui/chevron_right.svg");
pub const CHEVRON_LEFT: ImageAsset = asset!("icons/expui/chevron_left.svg");
pub const CHEVRON_DOWN: ImageAsset = asset!("icons/expui/chevron_down.svg");
pub const RUN: ImageAsset = asset!("icons/expui/toolwindow_run.svg");
pub const RUN_20: ImageAsset = asset!("icons/expui/toolwindow_run_20.svg");
pub const COMMIT: ImageAsset = asset!("icons/expui/toolwindow_commit.svg");
pub const COMMIT_20: ImageAsset = asset!("icons/expui/toolwindow_commit_20.svg");
pub const FIND: ImageAsset = asset!("icons/expui/toolwindow_find.svg");
pub const FIND_20: ImageAsset = asset!("icons/expui/toolwindow_find_20.svg");
pub const DEBUG: ImageAsset = asset!("icons/expui/debug.svg");
pub const STOP: ImageAsset = asset!("icons/expui/stop.svg");

/// 树 / 标签页用的文件类型图标：Java、XML 用 expui 彩色图标，其余用通用文件图标。
pub fn for_file(path: &str, folder: bool, dark: bool) -> ImageAsset {
    if folder {
        return FOLDER;
    }
    if std::path::Path::new(path)
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("pom.xml"))
    {
        return MAVEN;
    }
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    match extension {
        "java" => {
            if dark {
                JAVA_DARK
            } else {
                JAVA
            }
        }
        "xml" => {
            if dark {
                XML_DARK
            } else {
                XML
            }
        }
        _ => FILE,
    }
}

/// 动作对应的菜单图标。
pub fn for_action(id: &str) -> Option<ImageAsset> {
    Some(match id {
        "file.open_project" => FOLDER,
        "file.open" => FILE,
        "file.save" => SAVE,
        "file.close" | "file.close_others" | "file.close_all" => CLOSE,
        "view.tool_window.project" => PROJECT_TOOLWINDOW,
        "view.tool_window.search" => FIND,
        "view.tool_window.version" => COMMIT,
        "view.tool_window.structure" => STRUCTURE,
        "view.bottom.services" => SERVICES,
        "view.bottom.problems" => WARNING,
        "view.bottom.run" => RUN,
        "view.bottom.terminal" => TERMINAL,
        "run.start" => PLAY,
        "run.stop" => STOP,
        "run.debug" => DEBUG,
        "view.appearance.system" | "view.appearance.light" | "view.appearance.dark" => PALETTE,
        "navigate.reveal" => LOCATE,
        "help.find_action" => SEARCH,
        "help.about" => INFO,
        _ => return None,
    })
}

/// 条纹按钮使用 expui 的 20×20 变体；其余工具窗口图标保持 16×16。
pub fn for_stripe_action(id: &str) -> ImageAsset {
    match id {
        "view.tool_window.project" => PROJECT_TOOLWINDOW_20,
        "view.tool_window.search" => FIND_20,
        "view.tool_window.version" => COMMIT_20,
        "view.tool_window.structure" => STRUCTURE_20,
        "view.bottom.services" => SERVICES_20,
        "view.bottom.run" => RUN_20,
        "view.bottom.problems" => PROBLEMS_20,
        _ => for_action(id).unwrap_or(FILE),
    }
}

/// 动作在菜单 / 命令面板里的助记符。
pub fn action_mnemonic(id: &str) -> Option<char> {
    Some(match id {
        "file.open_project" => 'P',
        "file.open" => 'O',
        "file.save" => 'S',
        "file.close" => 'C',
        "file.close_others" => 'T',
        "file.close_all" => 'A',
        "view.tool_window.project" => 'P',
        "view.tool_window.search" => 'S',
        "view.tool_window.version" => 'V',
        "view.bottom.problems" => 'B',
        "view.bottom.run" => 'R',
        "view.bottom.terminal" => 'T',
        "run.start" => 'R',
        "run.stop" => 'S',
        "run.debug" => 'D',
        "view.appearance.system" => 'Y',
        "view.appearance.light" => 'L',
        "view.appearance.dark" => 'D',
        "navigate.reveal" => 'E',
        "help.find_action" => 'A',
        "help.about" => 'K',
        _ => return None,
    })
}

/// 顶层菜单 / 子菜单的助记符。
pub fn group_mnemonic(id: &str) -> Option<char> {
    Some(match id {
        "menu.file" => 'F',
        "menu.view" => 'V',
        "menu.navigate" => 'N',
        "menu.help" => 'H',
        "menu.view.tool_windows" => 'T',
        "menu.view.bottom" => 'B',
        "menu.view.appearance" => 'A',
        _ => return None,
    })
}

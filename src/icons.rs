//! 菜单与命令面板的图标、助记符。
//!
//! 图标是 16×16 的 SVG，`currentColor` 跟随主题；路径相对 `assets/`。
//! `java*.svg` / `xml*.svg` / `run.svg` / `debug.svg` 取自 IntelliJ 社区版
//! `platform/icons/src/expui`（Apache 2.0，文件内保留版权头）。

use nsh_core::prelude::*;

pub const FOLDER: ImageAsset = asset!("icons/folder.svg");
pub const FILE: ImageAsset = asset!("icons/file.svg");
pub const JAVA: ImageAsset = asset!("icons/java.svg");
pub const JAVA_DARK: ImageAsset = asset!("icons/java_dark.svg");
pub const XML: ImageAsset = asset!("icons/xml.svg");
pub const XML_DARK: ImageAsset = asset!("icons/xml_dark.svg");
pub const SAVE: ImageAsset = asset!("icons/save.svg");
pub const CLOSE: ImageAsset = asset!("icons/close.svg");
pub const SEARCH: ImageAsset = asset!("icons/search.svg");
pub const BRANCH: ImageAsset = asset!("icons/branch.svg");
pub const WARNING: ImageAsset = asset!("icons/warning.svg");
pub const PLAY: ImageAsset = asset!("icons/play.svg");
pub const TERMINAL: ImageAsset = asset!("icons/terminal.svg");
pub const PALETTE: ImageAsset = asset!("icons/palette.svg");
pub const SETTINGS: ImageAsset = asset!("icons/settings.svg");
pub const LOCATE: ImageAsset = asset!("icons/locate.svg");
pub const INFO: ImageAsset = asset!("icons/info.svg");
pub const EYE: ImageAsset = asset!("icons/eye.svg");
pub const COPY: ImageAsset = asset!("icons/copy.svg");
pub const REFRESH: ImageAsset = asset!("icons/refresh.svg");
pub const COLLAPSE_ALL: ImageAsset = asset!("icons/collapse_all.svg");
pub const CHEVRON_RIGHT: ImageAsset = asset!("icons/chevron_right.svg");
pub const CHEVRON_LEFT: ImageAsset = asset!("icons/chevron_left.svg");
pub const CHEVRON_DOWN: ImageAsset = asset!("icons/chevron_down.svg");
pub const RUN: ImageAsset = asset!("icons/run.svg");
pub const DEBUG: ImageAsset = asset!("icons/debug.svg");
pub const STOP: ImageAsset = asset!("icons/stop.svg");

/// 树 / 标签页用的文件类型图标：Java、XML 用 expui 彩色图标，其余用通用文件图标。
pub fn for_file(path: &str, folder: bool, dark: bool) -> ImageAsset {
    if folder {
        return FOLDER;
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
        "view.tool_window.project" => FOLDER,
        "view.tool_window.search" => SEARCH,
        "view.tool_window.version" => BRANCH,
        "view.bottom.problems" => WARNING,
        "view.bottom.run" => PLAY,
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

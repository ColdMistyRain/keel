use std::path::Path;

use gpui::Hsla;
use gpui_component_assets::IconName;

use crate::idea_theme::color;
use crate::model::language_for_path;

/// 按 IDEA 的习惯给不同语言的文件配图标和颜色。
pub fn file_icon(path: &Path, fallback: Hsla) -> (IconName, Hsla) {
    match language_for_path(path) {
        Some("java") => (IconName::FileCode, color(0xe76f00)),
        Some("rust") => (IconName::FileCode, color(0xb7410e)),
        Some("json" | "toml" | "yaml") => (IconName::FileBraces, color(0x8c8c8c)),
        Some("markdown") => (IconName::FileText, color(0x519aba)),
        Some("javascript" | "jsx") => (IconName::FileCode, color(0xa88b00)),
        Some("typescript" | "tsx") => (IconName::FileCode, color(0x3178c6)),
        Some("python") => (IconName::FileCode, color(0x3572a5)),
        Some("html") => (IconName::FileCode, color(0xe34f26)),
        Some("css") => (IconName::FileCode, color(0x563d7c)),
        Some("go") => (IconName::FileCode, color(0x00add8)),
        Some("bash") => (IconName::FileTerminal, color(0x4e9a06)),
        Some(_) => (IconName::FileCode, fallback),
        None => (IconName::File, fallback),
    }
}

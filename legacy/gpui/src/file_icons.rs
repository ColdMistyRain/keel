use std::path::Path;

use gpui::SharedString;

use crate::model::language_for_path;

/// 按 IDEA 的文件类型图标返回 SVG 资源路径；`suffix` 为 `""` 或 `"_dark"`。
pub fn file_icon(path: &Path, suffix: &str) -> SharedString {
    let name = match language_for_path(path) {
        Some("java") => "ft_java",
        Some("json") => "ft_json",
        Some("yaml") => "ft_yaml",
        Some("toml") => "ft_toml",
        Some("markdown") => "ft_markdown",
        Some("javascript" | "jsx") => "ft_js",
        Some("typescript" | "tsx") => "ft_ts",
        Some("html") => "ft_html",
        Some("css") => "ft_css",
        Some("bash") => "ft_shell",
        Some(_) | None => "ft_text",
    };
    SharedString::from(format!("icons/{name}{suffix}.svg"))
}

/// 目录图标，展开与收起共用同一张 expUI 图标。
pub fn folder_icon(suffix: &str) -> SharedString {
    SharedString::from(format!("icons/ft_folder{suffix}.svg"))
}

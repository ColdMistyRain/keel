use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};
use gpui_component_assets::AllAssets;

/// 项目自己的资源：IDEA 的 SVG 图标（取自 intellij-community 的 expUI 图标集），
/// 其余请求原样转交给 gpui-component 的资源包。
pub struct KeelAssets;

macro_rules! icon_assets {
    ($($path:literal => $file:literal),* $(,)?) => {
        const ICONS: &[(&str, &[u8])] = &[
            $(($path, include_bytes!($file).as_slice()),)*
        ];
    };
}

icon_assets! {
    "icons/project.svg" => "../assets/icons/project.svg",
    "icons/project_dark.svg" => "../assets/icons/project_dark.svg",
    "icons/commit.svg" => "../assets/icons/commit.svg",
    "icons/commit_dark.svg" => "../assets/icons/commit_dark.svg",
    "icons/bookmarks.svg" => "../assets/icons/bookmarks.svg",
    "icons/bookmarks_dark.svg" => "../assets/icons/bookmarks_dark.svg",
    "icons/structure.svg" => "../assets/icons/structure.svg",
    "icons/structure_dark.svg" => "../assets/icons/structure_dark.svg",
    "icons/services.svg" => "../assets/icons/services.svg",
    "icons/services_dark.svg" => "../assets/icons/services_dark.svg",
    "icons/vcs.svg" => "../assets/icons/vcs.svg",
    "icons/vcs_dark.svg" => "../assets/icons/vcs_dark.svg",
    "icons/search.svg" => "../assets/icons/search.svg",
    "icons/search_dark.svg" => "../assets/icons/search_dark.svg",
    "icons/settings.svg" => "../assets/icons/settings.svg",
    "icons/settings_dark.svg" => "../assets/icons/settings_dark.svg",
    "icons/collapse.svg" => "../assets/icons/collapse.svg",
    "icons/collapse_dark.svg" => "../assets/icons/collapse_dark.svg",
    "icons/locate.svg" => "../assets/icons/locate.svg",
    "icons/locate_dark.svg" => "../assets/icons/locate_dark.svg",
    "icons/link_drop_triangle.svg" => "../assets/icons/link_drop_triangle.svg",
    "icons/link_drop_triangle_dark.svg" => "../assets/icons/link_drop_triangle_dark.svg",
    "icons/menu_20.svg" => "../assets/icons/menu_20.svg",
    "icons/menu_20_dark.svg" => "../assets/icons/menu_20_dark.svg",
    "icons/search_20.svg" => "../assets/icons/search_20.svg",
    "icons/search_20_dark.svg" => "../assets/icons/search_20_dark.svg",
    "icons/settings_20.svg" => "../assets/icons/settings_20.svg",
    "icons/settings_20_dark.svg" => "../assets/icons/settings_20_dark.svg",
    "icons/more.svg" => "../assets/icons/more.svg",
    "icons/more_dark.svg" => "../assets/icons/more_dark.svg",
    "icons/branch.svg" => "../assets/icons/branch.svg",
    "icons/branch_dark.svg" => "../assets/icons/branch_dark.svg",
    "icons/chevron_right.svg" => "../assets/icons/chevron_right.svg",
    "icons/chevron_right_dark.svg" => "../assets/icons/chevron_right_dark.svg",
    "icons/chevron_down.svg" => "../assets/icons/chevron_down.svg",
    "icons/chevron_down_dark.svg" => "../assets/icons/chevron_down_dark.svg",
    "icons/chevron_left.svg" => "../assets/icons/chevron_left.svg",
    "icons/chevron_left_dark.svg" => "../assets/icons/chevron_left_dark.svg",
    "icons/window_minimize.svg" => "../assets/icons/window_minimize.svg",
    "icons/window_maximize.svg" => "../assets/icons/window_maximize.svg",
    "icons/window_close.svg" => "../assets/icons/window_close.svg",
    "icons/close_small.svg" => "../assets/icons/close_small.svg",
    "icons/close_small_dark.svg" => "../assets/icons/close_small_dark.svg",
    "icons/close_small_hovered.svg" => "../assets/icons/close_small_hovered.svg",
    "icons/close_small_hovered_dark.svg" => "../assets/icons/close_small_hovered_dark.svg",
    "icons/plus.svg" => "../assets/icons/plus.svg",
    "icons/plus_dark.svg" => "../assets/icons/plus_dark.svg",
    "icons/ft_java.svg" => "../assets/icons/ft_java.svg",
    "icons/ft_java_dark.svg" => "../assets/icons/ft_java_dark.svg",
    "icons/ft_xml.svg" => "../assets/icons/ft_xml.svg",
    "icons/ft_xml_dark.svg" => "../assets/icons/ft_xml_dark.svg",
    "icons/ft_json.svg" => "../assets/icons/ft_json.svg",
    "icons/ft_json_dark.svg" => "../assets/icons/ft_json_dark.svg",
    "icons/ft_yaml.svg" => "../assets/icons/ft_yaml.svg",
    "icons/ft_yaml_dark.svg" => "../assets/icons/ft_yaml_dark.svg",
    "icons/ft_toml.svg" => "../assets/icons/ft_toml.svg",
    "icons/ft_toml_dark.svg" => "../assets/icons/ft_toml_dark.svg",
    "icons/ft_markdown.svg" => "../assets/icons/ft_markdown.svg",
    "icons/ft_markdown_dark.svg" => "../assets/icons/ft_markdown_dark.svg",
    "icons/ft_text.svg" => "../assets/icons/ft_text.svg",
    "icons/ft_text_dark.svg" => "../assets/icons/ft_text_dark.svg",
    "icons/ft_css.svg" => "../assets/icons/ft_css.svg",
    "icons/ft_css_dark.svg" => "../assets/icons/ft_css_dark.svg",
    "icons/ft_html.svg" => "../assets/icons/ft_html.svg",
    "icons/ft_html_dark.svg" => "../assets/icons/ft_html_dark.svg",
    "icons/ft_js.svg" => "../assets/icons/ft_js.svg",
    "icons/ft_js_dark.svg" => "../assets/icons/ft_js_dark.svg",
    "icons/ft_ts.svg" => "../assets/icons/ft_ts.svg",
    "icons/ft_ts_dark.svg" => "../assets/icons/ft_ts_dark.svg",
    "icons/ft_shell.svg" => "../assets/icons/ft_shell.svg",
    "icons/ft_shell_dark.svg" => "../assets/icons/ft_shell_dark.svg",
    "icons/ft_folder.svg" => "../assets/icons/ft_folder.svg",
    "icons/ft_folder_dark.svg" => "../assets/icons/ft_folder_dark.svg",
}

impl AssetSource for KeelAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = ICONS.iter().find(|(name, _)| *name == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        AllAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = AllAssets.list(path)?;
        // AssetRegistry 只会登记 list("") 返回的路径，自定义图标必须在这里补上。
        if path.is_empty() {
            paths.extend(ICONS.iter().map(|(name, _)| SharedString::from(*name)));
        }
        Ok(paths)
    }
}

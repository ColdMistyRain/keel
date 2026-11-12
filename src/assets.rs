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
    "icons/refresh.svg" => "../assets/icons/refresh.svg",
    "icons/refresh_dark.svg" => "../assets/icons/refresh_dark.svg",
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
    "icons/close.svg" => "../assets/icons/close.svg",
    "icons/close_dark.svg" => "../assets/icons/close_dark.svg",
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

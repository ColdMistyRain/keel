use std::path::{Path, PathBuf};

use gpui::{prelude::*, *};
use gpui_component::Sizable as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenuItem};
use gpui_component_assets::IconName;

use crate::actions::{
    CollapseProjectTree, LocateInProject, RefreshProjectTree, ToggleProjectPanel,
};
use crate::app::{Chrome, CommandId, KeelApp};
use crate::model::VisibleRow;

impl KeelApp {
    pub fn render_project_panel(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let root = self.project.root.clone();
        let icon = |name: &str| format!("icons/{name}{}.svg", chrome.icon_suffix);

        let header = div()
            .h(px(28.))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .flex_shrink_0()
            .bg(chrome.header)
            .child(
                Button::new("project-title")
                    .ghost()
                    .small()
                    .h(px(24.))
                    .pl_0()
                    .label("项目")
                    .icon(IconName::ChevronDown)
                    .dropdown_menu(|menu, _, _| {
                        menu.menu("在项目中定位", Box::new(LocateInProject))
                            .separator()
                            .menu("折叠全部", Box::new(CollapseProjectTree))
                            .menu("从磁盘重新加载", Box::new(RefreshProjectTree))
                    }),
            )
            .child(div().flex_1())
            .child({
                let new_app = cx.entity().downgrade();
                let file_dir = root.clone().unwrap_or_default();
                let folder_dir = file_dir.clone();
                crate::app::icon_button("project-new", icon("plus"), chrome.muted_fg)
                    .tooltip("新建")
                    .dropdown_menu(move |menu, _, _| {
                        let file_app = new_app.clone();
                        let file_dir = file_dir.clone();
                        let folder_app = new_app.clone();
                        let folder_dir = folder_dir.clone();
                        menu.item(
                            PopupMenuItem::new("新建文件…").on_click(move |_, window, cx| {
                                file_app
                                    .update(cx, |this, cx| {
                                        this.open_new_file_dialog(file_dir.clone(), window, cx)
                                    })
                                    .ok();
                            }),
                        )
                        .item(
                            PopupMenuItem::new("新建文件夹…").on_click(move |_, window, cx| {
                                folder_app
                                    .update(cx, |this, cx| {
                                        this.open_new_folder_dialog(folder_dir.clone(), window, cx)
                                    })
                                    .ok();
                            }),
                        )
                    })
            })
            .child(div().w(px(1.)).h(px(16.)).mx_1().bg(chrome.border))
            .child(
                crate::app::icon_button("project-collapse-all", icon("collapse"), chrome.muted_fg)
                    .tooltip("折叠全部")
                    .on_click(
                        cx.listener(|this, _event, _window, cx| this.collapse_all_project_tree(cx)),
                    ),
            )
            .child(
                crate::app::icon_button("project-refresh", icon("refresh"), chrome.muted_fg)
                    .tooltip("从磁盘重新加载")
                    .on_click(
                        cx.listener(|this, _event, _window, cx| this.refresh_project_tree(cx)),
                    ),
            )
            .child(
                crate::app::icon_button("project-options", icon("more"), chrome.muted_fg)
                    .dropdown_menu(|menu, _, _| {
                        menu.menu("从磁盘重新加载", Box::new(RefreshProjectTree))
                            .menu("折叠全部", Box::new(CollapseProjectTree))
                            .separator()
                            .menu("隐藏工具窗口", Box::new(ToggleProjectPanel))
                    }),
            )
            .child(
                crate::app::icon_button("project-hide", icon("chevron_left"), chrome.muted_fg)
                    .tooltip("隐藏到侧边条")
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.run_command(CommandId::ToggleProjectPanel, cx)
                    })),
            );

        let body = if self.project.rows.is_empty() {
            let message = match &self.project.root {
                Some(_) => "当前目录没有可显示的内容",
                None => "尚未打开项目目录",
            };
            div()
                .p_3()
                .text_color(chrome.muted_fg)
                .child(message)
                .into_any_element()
        } else {
            let rows = self.project.rows.clone();
            let selected = self.project.selected.clone();
            let scroll = self.project.scroll.clone();
            let mut list = div()
                .id("project-tree-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&scroll)
                .py_1();

            let mut children = Vec::new();
            for (ix, row) in rows.iter().enumerate() {
                children.push(self.render_row(ix, row, selected.as_deref(), chrome, cx));
            }
            list = list.children(children);
            list.into_any_element()
        };

        div()
            .w_full()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(chrome.panel)
            .text_color(chrome.panel_fg)
            .child(header)
            .child(body)
            .into_any_element()
    }

    fn render_row(
        &mut self,
        ix: usize,
        row: &VisibleRow,
        selected: Option<&Path>,
        chrome: &Chrome,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_selected = selected == Some(row.path.as_path());
        let row_path: PathBuf = row.path.clone();
        let left_path = row_path.clone();
        let right_path = row_path.clone();
        let is_dir = row.is_dir;
        // 根行在项目名后附带完整路径，和 IDEA 的项目树一致。
        let root_path = (row.depth == 0).then(|| row.path.display().to_string());
        // IDEA 树左右各留 12px，每级缩进 20px。
        let indent = px(8.0 + row.depth as f32 * 20.0);
        let icon_path = if is_dir {
            crate::file_icons::folder_icon(chrome.icon_suffix)
        } else {
            crate::file_icons::file_icon(&row.path, chrome.icon_suffix)
        };
        let chevron = is_dir.then(|| {
            svg()
                .path(SharedString::from(format!(
                    "icons/chevron_{}{}.svg",
                    if row.expanded { "down" } else { "right" },
                    chrome.icon_suffix
                )))
                .size(px(16.))
                .text_color(chrome.muted_fg)
        });

        let row = div()
            .id(("tree-row", ix))
            .h(px(24.))
            .mx_1()
            .flex()
            .items_center()
            .gap_1()
            .pl(indent)
            .pr_1()
            .rounded_sm()
            .cursor_pointer()
            .when(is_selected, |row| {
                row.bg(chrome.selection).text_color(chrome.selection_fg)
            })
            .hover(|style| style.bg(chrome.hover_bg))
            .child(
                div()
                    .w(px(16.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(chrome.muted_fg)
                    .children(chevron),
            )
            .child(
                svg()
                    .path(icon_path)
                    .size(px(16.))
                    .text_color(chrome.muted_fg),
            )
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(row.name.clone()),
            )
            .children(root_path.map(|path| {
                div()
                    .min_w_0()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_xs()
                    .text_color(chrome.secondary_fg)
                    .child(path)
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.project.selected = Some(left_path.clone());
                    if is_dir {
                        this.toggle_dir(left_path.clone(), cx);
                    } else if event.click_count >= 2 {
                        this.open_file(left_path.clone(), window, cx);
                    } else {
                        cx.notify();
                    }
                }),
            )
            // 右键只选中，不展开目录，随后弹出文件操作菜单。
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                    this.project.selected = Some(right_path.clone());
                    cx.notify();
                }),
            );

        let menu_app = cx.entity().downgrade();
        let menu_path = row_path.clone();
        let menu_is_dir = is_dir;
        row.context_menu(move |menu, _, _| {
            let path = menu_path.clone();
            let dir = if menu_is_dir {
                path.clone()
            } else {
                path.parent()
                    .map(|parent| parent.to_path_buf())
                    .unwrap_or_else(|| path.clone())
            };
            let mut menu = menu;

            let new_file_app = menu_app.clone();
            let new_file_dir = dir.clone();
            menu = menu.item(
                PopupMenuItem::new("新建文件…").on_click(move |_, window, cx| {
                    new_file_app
                        .update(cx, |this, cx| {
                            this.open_new_file_dialog(new_file_dir.clone(), window, cx)
                        })
                        .ok();
                }),
            );

            let new_folder_app = menu_app.clone();
            let new_folder_dir = dir;
            menu = menu.item(
                PopupMenuItem::new("新建文件夹…").on_click(move |_, window, cx| {
                    new_folder_app
                        .update(cx, |this, cx| {
                            this.open_new_folder_dialog(new_folder_dir.clone(), window, cx)
                        })
                        .ok();
                }),
            );

            menu = menu.separator();

            let rename_app = menu_app.clone();
            let rename_path = path.clone();
            menu = menu.item(
                PopupMenuItem::new("重命名…").on_click(move |_, window, cx| {
                    rename_app
                        .update(cx, |this, cx| {
                            this.open_rename_dialog(rename_path.clone(), window, cx)
                        })
                        .ok();
                }),
            );

            let delete_app = menu_app.clone();
            let delete_path = path.clone();
            menu = menu.item(PopupMenuItem::new("删除").on_click(move |_, _, cx| {
                delete_app
                    .update(cx, |this, cx| {
                        this.request_delete(delete_path.clone(), menu_is_dir, cx)
                    })
                    .ok();
            }));

            menu = menu.separator();

            let copy_app = menu_app.clone();
            let copy_path = path.clone();
            menu = menu.item(PopupMenuItem::new("复制路径").on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(copy_path.display().to_string()));
                copy_app
                    .update(cx, |this, cx| {
                        this.status = crate::app::StatusMessage::info("已复制路径");
                        cx.notify();
                    })
                    .ok();
            }));

            if !menu_is_dir {
                let open_app = menu_app.clone();
                let open_path = path;
                menu = menu.separator();
                menu = menu.item(PopupMenuItem::new("在编辑器中打开").on_click(
                    move |_, window, cx| {
                        open_app
                            .update(cx, |this, cx| this.open_file(open_path.clone(), window, cx))
                            .ok();
                    },
                ));
            }

            menu
        })
        .into_any_element()
    }
}

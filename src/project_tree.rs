use std::path::{Path, PathBuf};

use gpui::{prelude::*, *};
use gpui_component::Sizable as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{PopupMenu, PopupMenuItem};

use crate::actions::{
    CollapseProjectTree, LocateInProject, RefreshProjectTree, ToggleProjectPanel,
};
use crate::app::{Chrome, CommandId, KeelApp};
use crate::model::VisibleRow;

/// 项目面板的悬停分组名：标题栏动作只在这个分组被悬停时显示。
const PROJECT_PANEL_GROUP: &str = "project-panel";

impl KeelApp {
    pub fn render_project_panel(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let root = self.project.root.clone();
        let icon = |name: &str| format!("icons/{name}{}.svg", chrome.icon_suffix);
        let separator = chrome.separator_line;

        // 标题栏右侧的动作组和 IDEA 一样：面板未悬停时不可见，鼠标进入面板
        // 才淡入（`ToolWindowHeader` 的 `headerActions`）。标题下拉保持常显。
        let actions = div()
            .flex()
            .items_center()
            .gap_1()
            .opacity(0.)
            .group_hover(PROJECT_PANEL_GROUP, |style| style.opacity(1.))
            .child({
                let new_app = cx.entity().downgrade();
                let file_dir = root.clone().unwrap_or_default();
                let folder_dir = file_dir.clone();
                crate::tooltip::with_tooltip(
                    "tip-project-new",
                    "新建",
                    crate::menu::dropdown_menu(
                        "project-new",
                        chrome.popup_border,
                        crate::app::icon_button("project-new", icon("plus"), chrome.muted_fg),
                        move |menu, _, _| {
                            let file_app = new_app.clone();
                            let file_dir = file_dir.clone();
                            let folder_app = new_app.clone();
                            let folder_dir = folder_dir.clone();
                            menu.item(PopupMenuItem::new("新建文件…").on_click(
                                move |_, window, cx| {
                                    file_app
                                        .update(cx, |this, cx| {
                                            this.open_new_file_dialog(file_dir.clone(), window, cx)
                                        })
                                        .ok();
                                },
                            ))
                            .item(
                                PopupMenuItem::new("新建文件夹…").on_click(move |_, window, cx| {
                                    folder_app
                                        .update(cx, |this, cx| {
                                            this.open_new_folder_dialog(
                                                folder_dir.clone(),
                                                window,
                                                cx,
                                            )
                                        })
                                        .ok();
                                }),
                            )
                        },
                    ),
                    cx,
                )
            })
            .child(div().w(px(1.)).h(px(16.)).mx_1().bg(chrome.border))
            // `ProjectViewToolbar` 的标题动作依次是 SelectInProjectView、
            // 展开、折叠；这里先补齐「在项目中定位」与「折叠全部」。
            .child(crate::tooltip::with_tooltip(
                "tip-project-locate",
                "在项目中定位",
                crate::app::icon_button("project-locate", icon("locate"), chrome.muted_fg)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.locate_active_document(cx);
                    })),
                cx,
            ))
            .child(crate::tooltip::with_tooltip(
                "tip-project-collapse-all",
                "折叠全部",
                crate::app::icon_button("project-collapse-all", icon("collapse"), chrome.muted_fg)
                    .on_click(
                        cx.listener(|this, _event, _window, cx| this.collapse_all_project_tree(cx)),
                    ),
                cx,
            ))
            .child(crate::menu::dropdown_menu(
                "project-options",
                chrome.popup_border,
                crate::app::icon_button("project-options", icon("more"), chrome.muted_fg),
                move |menu, _, _| {
                    menu.menu("从磁盘重新加载", Box::new(RefreshProjectTree))
                        .menu("折叠全部", Box::new(CollapseProjectTree))
                        .item(crate::menu::separator(separator))
                        .menu("隐藏工具窗口", Box::new(ToggleProjectPanel))
                },
            ))
            .child(crate::tooltip::with_tooltip(
                "tip-project-hide",
                "隐藏到侧边条",
                crate::app::icon_button("project-hide", icon("chevron_left"), chrome.muted_fg)
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.run_command(CommandId::ToggleProjectPanel, cx)
                    })),
                cx,
            ));

        let header = div()
            .h(px(41.))
            // 标题的左边距属于标题自身（`ContentComboLabel` 的 border insets），
            // 右侧工具栏取 `toolbarLeftRightInsets`（0,12,0,8）的右边距。
            .pr(px(8.))
            .flex()
            .items_center()
            .gap_1()
            .flex_shrink_0()
            .bg(chrome.header)
            .child(crate::menu::dropdown_menu(
                "project-title",
                chrome.popup_border,
                Button::new("project-title")
                    .ghost()
                    .small()
                    .h(px(24.))
                    // `headerLabelLeftRightInsets` 的左 12；箭头紧跟文字，
                    // 因此右侧不留内边距（`ContentComboLabel` 的 3px 偏移
                    // 由组件库 small 按钮的 4px 间距近似）。
                    .pl(px(12.))
                    .pr_0()
                    // 标题字号跟随 `ToolWindow.Header.font.size.offset=0`，即 13px。
                    .text_size(px(13.))
                    .label("项目")
                    // IDEA 的标题下拉箭头在文字右侧（`ContentComboLabel` 把
                    // `LinkDropTriangle` 画在文字宽度之后），不是左侧。
                    .child(
                        svg()
                            .path(SharedString::from(icon("link_drop_triangle")))
                            .size(px(14.))
                            .text_color(chrome.muted_fg),
                    ),
                move |menu, _, _| {
                    menu.menu("在项目中定位", Box::new(LocateInProject))
                        .item(crate::menu::separator(separator))
                        .menu("折叠全部", Box::new(CollapseProjectTree))
                        .menu("从磁盘重新加载", Box::new(RefreshProjectTree))
                },
            ))
            .child(div().flex_1())
            .child(actions);

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
            .group(PROJECT_PANEL_GROUP)
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
        let menu_separator = chrome.separator_line;
        // 根行在项目名后附带完整路径，和 IDEA 的项目树一致。
        let is_root = row.depth == 0;
        let root_path = is_root.then(|| row.path.display().to_string());
        // 树的内容区左右各留 12px（`Tree.border` = "4,12,4,12"），
        // 每级缩进 = `Tree.leftChildIndent`(7) + `Tree.rightChildIndent`(11) = 18px。
        let indent = px(row.depth as f32 * 18.0);
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

        div()
            .id(("tree-row", ix))
            // Swing 树读 `Tree.rowHeight`（24），`.compact` 只给 Jewel 的密度用。
            .h(px(24.))
            .mx(px(12.))
            // 新 UI 的选中/悬停块是内容区宽度的圆角矩形：半径取
            // `Tree.Selection.arc`(8) / 2 = 4（`DefaultTreeUI` 新选中样式）。
            .rounded(px(4.))
            .flex()
            .items_center()
            .gap_1()
            .pl(indent)
            .pr_2()
            .cursor_pointer()
            .when(is_selected, |row| {
                row.bg(chrome.selection).text_color(chrome.selection_fg)
            })
            // 选中行悬停时保持选中底色（和 IDEA 的树一致）。
            .when(!is_selected, |row| {
                row.hover(|style| style.bg(chrome.hover_bg))
            })
            .child(
                div()
                    .w(px(16.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(chrome.muted_fg)
                    .children(chevron),
            )
            // 文件类型图标是 expUI 的多色图标，`svg()` 会把它压成单色蒙版，
            // 这里用 `img()` 保留原始配色（IDEA 的项目树图标是彩色的）。
            .child(img(icon_path).size(px(16.)))
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    // IDEA 把项目根节点名称加粗，路径后缀仍是次要色。
                    .when(is_root, |name| name.font_weight(FontWeight::BOLD))
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
                        // 双击把预览标签固定为普通标签。
                        this.open_file(left_path.clone(), window, cx);
                    } else {
                        // 单击预览（IDEA 的 preview tab）。
                        this.preview_file(left_path.clone(), window, cx);
                    }
                }),
            )
            // 右键只选中，不展开目录，随后在点击处弹出文件操作菜单。
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.project.selected = Some(right_path.clone());
                    let menu_app = cx.entity().downgrade();
                    let menu_path = right_path.clone();
                    let menu_is_dir = is_dir;
                    this.open_context_menu(event.position, window, cx, move |menu, _, _| {
                        build_row_menu(menu, menu_app, menu_path, menu_is_dir, menu_separator)
                    });
                }),
            )
            .into_any_element()
    }
}

/// 项目树行的右键菜单：新建、重命名、删除、复制路径、在编辑器中打开。
fn build_row_menu(
    menu: PopupMenu,
    menu_app: WeakEntity<KeelApp>,
    path: PathBuf,
    menu_is_dir: bool,
    separator: Hsla,
) -> PopupMenu {
    // 文件的新建位置取所在目录，目录则取自身。
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

    menu = menu.item(crate::menu::separator(separator));

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

    menu = menu.item(crate::menu::separator(separator));

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
        menu = menu.item(crate::menu::separator(separator));
        menu = menu.item(
            PopupMenuItem::new("在编辑器中打开").on_click(move |_, window, cx| {
                open_app
                    .update(cx, |this, cx| this.open_file(open_path.clone(), window, cx))
                    .ok();
            }),
        );
    }

    menu
}

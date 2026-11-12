use std::path::PathBuf;

use gpui::{prelude::*, *};
use gpui_component::Sizable as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Editor, EditorState};
use gpui_component::menu::PopupMenuItem;
use gpui_component_assets::IconName;

use crate::app::{Chrome, KeelApp, empty_editor_state};
use crate::tool_window::STRIPE_WIDTH;

/// 一个已打开的文件。
pub struct Document {
    pub path: PathBuf,
    pub state: Entity<EditorState>,
    pub dirty: bool,
    pub eol: &'static str,
    pub _changes: Subscription,
    /// 编辑器每次重绘（含光标移动）都会触发，用来刷新状态栏的行列号。
    pub _observe: Subscription,
}

impl Document {
    pub fn new(
        path: PathBuf,
        state: Entity<EditorState>,
        eol: &'static str,
        changes: Subscription,
        observe: Subscription,
    ) -> Self {
        Self {
            path,
            state,
            dirty: false,
            eol,
            _changes: changes,
            _observe: observe,
        }
    }

    pub fn title(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| self.path.display().to_string())
    }
}

impl KeelApp {
    /// 工具窗口条右侧的工作区：项目面板与编辑器区之间可拖拽调宽。
    pub fn render_workspace_area(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let main_column = self.render_main_column(chrome, cx);
        if !self.project_panel_visible {
            return main_column;
        }

        let panel_width = px(self.project_panel_width);
        let project_panel = self.render_project_panel(chrome, cx);
        let app = cx.entity().downgrade();

        // 拖拽状态由窗口级鼠标事件驱动；GPUI 只允许在 paint 阶段注册这些监听。
        let tracker = canvas(|_, _, _| {}, {
            let app = app.clone();
            move |_, _, window, _| {
                let move_app = app.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
                    if !phase.bubble() {
                        return;
                    }
                    move_app
                        .update(cx, |app, cx| {
                            if !app.resizing_project_panel {
                                return;
                            }
                            // 窗口内边距 4px + 侧边条 37px + 岛屿间距 4px。
                            let width = (event.position.x - px(STRIPE_WIDTH + 8.)).as_f32();
                            app.project_panel_width = width.clamp(180., 600.);
                            cx.notify();
                        })
                        .ok();
                });

                let up_app = app;
                window.on_mouse_event(move |_: &MouseUpEvent, phase, _window, cx| {
                    if !phase.bubble() {
                        return;
                    }
                    up_app
                        .update(cx, |app, cx| {
                            if !app.resizing_project_panel {
                                return;
                            }
                            app.resizing_project_panel = false;
                            app.persisted.project_panel_width = Some(app.project_panel_width);
                            app.persist();
                            cx.notify();
                        })
                        .ok();
                });
            }
        })
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let splitter = div()
            .id("project-splitter")
            // 岛屿之间的留白是 `Island.borderWidth.compact` = 4px。
            .w(px(4.))
            .h_full()
            .flex_shrink_0()
            .flex()
            .justify_center()
            .cursor_col_resize()
            .when(self.resizing_project_panel, |splitter| {
                splitter.bg(chrome.hover_bg)
            })
            .hover(|style| style.bg(chrome.hover_bg))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event: &MouseDownEvent, window, cx| {
                    this.resizing_project_panel = true;
                    window.refresh();
                    cx.notify();
                }),
            );

        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_row()
            .relative()
            .child(tracker)
            .child(
                div()
                    .w(panel_width)
                    .h_full()
                    .flex_shrink_0()
                    .rounded(chrome.island_arc)
                    .overflow_hidden()
                    .bg(chrome.panel)
                    .child(project_panel),
            )
            .child(splitter)
            .child(main_column)
            .into_any_element()
    }

    pub fn render_main_column(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let editor_area = self.render_editor_area(chrome, cx);
        let bottom_panel = if self.bottom_panel_visible {
            Some(self.render_bottom_panel(chrome, cx))
        } else {
            None
        };

        let mut column = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(editor_area);

        if let Some(panel) = bottom_panel {
            column = column.child(panel);
        }

        column.into_any_element()
    }

    fn render_editor_area(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        if self.documents.is_empty() {
            return div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .bg(chrome.bg)
                .rounded(chrome.island_arc)
                .overflow_hidden()
                .child(empty_editor_state(chrome))
                .into_any_element();
        }

        let tab_bar = self.render_tab_bar(chrome, cx);
        let editor = self.render_active_editor(chrome);

        // 面包屑由状态栏左侧承载（和 IDEA 一致），这里只留标签条与编辑器。
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(chrome.bg)
            .rounded(chrome.island_arc)
            .overflow_hidden()
            .child(tab_bar)
            .child(editor)
            .into_any_element()
    }

    /// 状态栏里的导航栏面包屑：按项目根拆成路径段，点击任一段在项目树中选中它。
    pub fn render_status_breadcrumbs(
        &mut self,
        chrome: &Chrome,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(doc) = self.active_document() else {
            return div().into_any_element();
        };
        let path = doc.path.clone();

        // 项目外的文件直接显示完整路径，避免出现拼接错误的片段。
        let mut segments: Vec<(String, PathBuf)> = Vec::new();
        match self
            .project
            .root
            .as_deref()
            .and_then(|root| path.strip_prefix(root).ok().map(|rest| (root, rest)))
        {
            Some((root, rest)) => {
                // 首段是项目根，和 IDEA 一致，点它回到项目根节点。
                let root_label = root
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| root.display().to_string());
                segments.push((root_label, root.to_path_buf()));

                let mut cursor = PathBuf::new();
                for component in rest.components() {
                    cursor.push(component);
                    segments.push((
                        component.as_os_str().to_string_lossy().to_string(),
                        root.join(&cursor),
                    ));
                }
            }
            None => segments.push((path.display().to_string(), path.clone())),
        }

        let mut children = Vec::new();
        let last = segments.len().saturating_sub(1);
        for (ix, (label, segment_path)) in segments.into_iter().enumerate() {
            if ix > 0 {
                children.push(
                    div()
                        .text_color(chrome.muted_fg)
                        .child(
                            svg()
                                .path(SharedString::from(format!(
                                    "icons/chevron_right{}.svg",
                                    chrome.icon_suffix
                                )))
                                .size(px(16.))
                                .text_color(chrome.muted_fg),
                        )
                        .into_any_element(),
                );
            }
            let is_last = ix == last;
            let is_dir = segment_path.is_dir();
            let icon_path = if is_dir {
                crate::file_icons::folder_icon(chrome.icon_suffix)
            } else {
                crate::file_icons::file_icon(&segment_path, chrome.icon_suffix)
            };
            children.push(
                div()
                    .id(("breadcrumb", ix))
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .text_color(if is_last { chrome.fg } else { chrome.muted_fg })
                    .hover(|style| style.bg(chrome.control_hover))
                    .child(
                        svg()
                            .path(icon_path)
                            .size(px(16.))
                            .text_color(chrome.muted_fg),
                    )
                    .child(div().whitespace_nowrap().child(label))
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.locate_in_project(segment_path.clone(), cx);
                    }))
                    .into_any_element(),
            );
        }

        div()
            .flex()
            .items_center()
            .gap_1()
            .h_full()
            .min_w_0()
            .overflow_hidden()
            .children(children)
            .into_any_element()
    }

    fn render_tab_bar(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let active = self.active;
        let drag = self.tab_drag.clone();
        let tab_scroll = self.tab_scroll.clone();
        let tabs: Vec<(usize, String, bool, PathBuf)> = self
            .documents
            .iter()
            .enumerate()
            .map(|(ix, doc)| (ix, doc.title(), doc.dirty, doc.path.clone()))
            .collect();
        let documents: Vec<(PathBuf, String, bool)> = self
            .documents
            .iter()
            .map(|doc| (doc.path.clone(), doc.title(), doc.dirty))
            .collect();

        let app = cx.entity().downgrade();
        let mut children = Vec::new();
        for (ix, title, dirty, path) in tabs {
            let is_active = active == Some(ix);
            let is_drop_target = drag
                .as_ref()
                .is_some_and(|drag| drag.path != path && drag.hover.as_ref() == Some(&path));
            let activate_path = path.clone();
            let activate_right_path = path.clone();
            let hover_path = path.clone();
            let close_path = path.clone();
            let icon_path = crate::file_icons::file_icon(&path, chrome.icon_suffix);

            let mut tab = div()
                .id(("editor-tab", ix))
                .h(px(28.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap_1()
                // `EditorTabs.tabInsets.compact` 是 "-2,6,-2,4"。
                .pl(px(6.))
                .pr(px(4.))
                .cursor_pointer()
                .when(is_active, |tab| {
                    // IDEA Islands 的选中标签是一张带 1px 蓝边的圆角卡片，
                    // 圆角取 `EditorTabs.underlineArc`（4px）。
                    tab.bg(chrome.tab_selected_bg)
                        .text_color(chrome.fg)
                        .rounded(px(4.))
                        .border_1()
                        .border_color(chrome.tab_underline)
                })
                .when(!is_active, |tab| {
                    // 悬停时按 IDEA 的 `paintDimmed()` 恢复满对比度。
                    tab.text_color(chrome.tab_fg_unselected)
                        .hover(|style| style.bg(chrome.control_hover).text_color(chrome.fg))
                })
                .when(is_drop_target, |tab| tab.bg(chrome.hover_bg))
                .child(
                    svg()
                        .path(icon_path)
                        .size(px(16.))
                        .text_color(chrome.muted_fg),
                )
                .child(div().whitespace_nowrap().child(title))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event: &MouseDownEvent, window, cx| {
                        this.begin_tab_drag(activate_path.clone());
                        if let Some(idx) = this.index_of(&activate_path) {
                            this.activate_document(idx, window, cx);
                        }
                    }),
                )
                // 拖拽经过时记录落点，松开鼠标才真正换位。
                .on_mouse_move(
                    cx.listener(move |this, _event: &MouseMoveEvent, _window, cx| {
                        if this.hover_tab(hover_path.clone()) {
                            cx.notify();
                        }
                    }),
                )
                // 右键同样先激活标签，和 IDEA 一致。
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, _event: &MouseDownEvent, window, cx| {
                        if let Some(idx) = this.index_of(&activate_right_path) {
                            this.activate_document(idx, window, cx);
                        }
                    }),
                );

            // IDEA 用蓝色圆点表示未保存，占的就是关闭按钮的位置。
            if dirty {
                tab = tab.child(
                    div()
                        .id(("editor-tab-dirty", ix))
                        .size(px(16.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .cursor_pointer()
                        .hover(|style| style.bg(chrome.control_hover))
                        .child(div().size(px(8.)).rounded_full().bg(chrome.accent))
                        .on_click(cx.listener(move |this, _event, _window, cx| {
                            this.request_close(close_path.clone(), cx);
                        })),
                );
            } else {
                tab = tab.child(
                    div()
                        .id(("editor-tab-close", ix))
                        .size(px(16.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_sm()
                        .cursor_pointer()
                        .hover(|style| style.bg(chrome.control_hover))
                        .child(
                            svg()
                                .path(SharedString::from(format!(
                                    "icons/close{}.svg",
                                    chrome.icon_suffix
                                )))
                                .size(px(14.))
                                .text_color(chrome.muted_fg),
                        )
                        .on_click(cx.listener(move |this, _event, _window, cx| {
                            this.request_close(close_path.clone(), cx);
                        })),
                );
            }

            // IDEA 的标签右键菜单：关闭 / 关闭其他 / 关闭全部 / 在项目中定位。
            // 菜单在点击处弹出，表面与下拉菜单一样带 1px 描边。
            let menu_path = path.clone();
            let tab = tab.on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    let menu_app = cx.entity().downgrade();
                    let menu_path = menu_path.clone();
                    this.open_context_menu(event.position, window, cx, move |menu, _, _| {
                        let close_app = menu_app.clone();
                        let close_path = menu_path.clone();
                        let other_app = menu_app.clone();
                        let other_path = menu_path.clone();
                        let all_app = menu_app.clone();
                        let copy_app = menu_app.clone();
                        let copy_path = menu_path.clone();
                        let locate_app = menu_app.clone();
                        let locate_path = menu_path.clone();

                        menu.item(PopupMenuItem::new("关闭").on_click(move |_, _, cx| {
                            close_app
                                .update(cx, |this, cx| this.request_close(close_path.clone(), cx))
                                .ok();
                        }))
                        .item(PopupMenuItem::new("关闭其他").on_click(move |_, _, cx| {
                            other_app
                                .update(cx, |this, cx| this.close_other_documents(&other_path, cx))
                                .ok();
                        }))
                        .item(PopupMenuItem::new("关闭全部").on_click(move |_, _, cx| {
                            all_app
                                .update(cx, |this, cx| this.close_all_documents(cx))
                                .ok();
                        }))
                        .separator()
                        .item(PopupMenuItem::new("复制路径").on_click(move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                copy_path.display().to_string(),
                            ));
                            copy_app
                                .update(cx, |this, cx| {
                                    this.status = crate::app::StatusMessage::info("已复制路径");
                                    cx.notify();
                                })
                                .ok();
                        }))
                        .item(
                            PopupMenuItem::new("在项目中定位").on_click(move |_, _, cx| {
                                locate_app
                                    .update(cx, |this, cx| {
                                        this.locate_in_project(locate_path.clone(), cx)
                                    })
                                    .ok();
                            }),
                        )
                    });
                }),
            );

            children.push(tab.into_any_element());
        }

        // 标签超出宽度时横向滚动；右侧按钮提供完整的已打开文件列表。
        let overflow_app = app.clone();
        let overflow = crate::menu::dropdown_menu(
            "editor-tab-overflow",
            chrome.popup_border,
            Button::new("editor-tab-overflow")
                .small()
                .ghost()
                .icon(IconName::ChevronDown)
                .tooltip("已打开的文件"),
            move |menu, _, _| {
                let mut menu = menu;
                for (path, title, dirty) in documents.clone() {
                    let item_app = overflow_app.clone();
                    let label = if dirty { format!("● {title}") } else { title };
                    menu = menu.item(PopupMenuItem::new(label).on_click(move |_, window, cx| {
                        item_app
                            .update(cx, |this, cx| this.open_file(path.clone(), window, cx))
                            .ok();
                    }));
                }
                menu
            },
        );

        let tracker_app = app.clone();
        let tracker = canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let up_app = tracker_app.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, phase, _window, cx| {
                    if !phase.bubble() {
                        return;
                    }
                    up_app.update(cx, |app, cx| app.finish_tab_drag(cx)).ok();
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        div()
            .relative()
            .flex()
            .items_center()
            // COMPACT 密度下标签行 32px（`SingleHeightTabs.UNSCALED_PREF_HEIGHT` 28 减去负内边距）。
            .h(px(32.))
            .flex_shrink_0()
            .bg(chrome.tab_bg)
            .border_b_1()
            .border_color(chrome.border)
            .child(tracker)
            .child(
                div()
                    .id("editor-tab-scroll")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .overflow_x_scroll()
                    .track_scroll(&tab_scroll)
                    .children(children),
            )
            .child(div().flex_shrink_0().pr_1().child(overflow))
            .into_any_element()
    }

    fn render_active_editor(&self, chrome: &Chrome) -> AnyElement {
        let Some(idx) = self.active else {
            return div().flex_1().into_any_element();
        };
        let Some(doc) = self.documents.get(idx) else {
            return div().flex_1().into_any_element();
        };

        div()
            .flex_1()
            .min_h_0()
            .w_full()
            .bg(chrome.bg)
            .child(
                Editor::new(&doc.state)
                    .bordered(false)
                    .p_0()
                    .h(relative(1.))
                    .font_family(chrome.mono_font.clone())
                    .text_size(chrome.mono_size)
                    // IDEA 的行高 = 字体自然高度 × `LINE_SPACING`(1.2)，
                    // JetBrains Mono 13px 约 20.8px 一行。
                    .line_height(relative(1.6)),
            )
            .into_any_element()
    }
}

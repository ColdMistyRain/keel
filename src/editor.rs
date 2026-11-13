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
    /// 预览标签（IDEA 的 preview tab）：斜体显示，下一次预览会替换它。
    pub preview: bool,
    /// 只读文件：状态栏显示「只读」，保存时给出明确提示而不是系统错误。
    pub readonly: bool,
    /// 打开/保存时记录的磁盘标识，用来发现外部修改（ED-06）。
    pub disk: Option<crate::external::DiskStamp>,
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
            preview: false,
            readonly: false,
            disk: None,
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
                            // 窗口内边距 6px + 侧边条 40px（已含右侧岛屿缝）。
                            let width = (event.position.x - px(STRIPE_WIDTH + 6.)).as_f32();
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
            // 岛屿之间的留白是 `Island.borderWidth` = 6px。
            .w(px(6.))
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
                    .rounded(chrome.island_radius)
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
                .rounded(chrome.island_radius)
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
            .rounded(chrome.island_radius)
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
            let mut segment = div()
                .id(("breadcrumb", ix))
                .flex()
                .items_center()
                .gap_1()
                .px_1()
                // 面包屑高亮块 = 内容 + `StatusBar.Breadcrumbs.itemBackgroundInsets`(2,4)，
                // 圆角取 `AbstractNavBarUI.paintHighlight` 的 arc 4（半径 2）。
                .h(px(17.))
                .rounded(px(2.))
                .cursor_pointer()
                // `NavBarItemComponent.update()`：新 UI 下所有段都是
                // `Breadcrumbs.FOREGROUND`，悬停才换成 `HOVER_FOREGROUND`。
                .text_color(chrome.muted_fg)
                .hover(|style| style.bg(chrome.control_hover).text_color(chrome.fg));
            // `effectiveIcon`：只有最后一段（或包含文件的段）才画图标，
            // 中间的目录段是纯文字。
            if is_last {
                let icon_path = if segment_path.is_dir() {
                    crate::file_icons::folder_icon(chrome.icon_suffix)
                } else {
                    crate::file_icons::file_icon(&segment_path, chrome.icon_suffix)
                };
                // 面包屑里的文件类型图标同样是多色的 expUI 图标。
                segment = segment.child(img(icon_path).size(px(16.)));
            }
            children.push(
                segment
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
            // `StatusBar.Breadcrumbs.navBarInsets` = "3,0,4,4"：面包屑条高 21，
            // 分隔箭头与相邻段紧贴，不再额外留 4px 间距。
            .h(px(21.))
            .min_w_0()
            .overflow_hidden()
            .children(children)
            .into_any_element()
    }

    fn render_tab_bar(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let active = self.active;
        let drag = self.tab_drag.clone();
        let tab_scroll = self.tab_scroll.clone();
        let tabs: Vec<(usize, String, bool, bool, PathBuf)> = self
            .documents
            .iter()
            .enumerate()
            .map(|(ix, doc)| (ix, doc.title(), doc.dirty, doc.preview, doc.path.clone()))
            .collect();
        let documents: Vec<(PathBuf, String, bool)> = self
            .documents
            .iter()
            .map(|doc| (doc.path.clone(), doc.title(), doc.dirty))
            .collect();

        let app = cx.entity().downgrade();
        let mut children = Vec::new();
        for (ix, title, dirty, preview, path) in tabs {
            let is_active = active == Some(ix);
            let is_drop_target = drag
                .as_ref()
                .is_some_and(|drag| drag.path != path && drag.hover.as_ref() == Some(&path));
            let activate_path = path.clone();
            let activate_right_path = path.clone();
            let hover_path = path.clone();
            let tab_hover_path = path.clone();
            let icon_path = crate::file_icons::file_icon(&path, chrome.icon_suffix);
            // 标签是否被悬停、关闭按钮是否被悬停（决定动作面板的可见性与图标）。
            let is_hovered = self.hovered_tab.as_deref() == Some(path.as_path());
            let close_hovered = self.hovered_tab_close.as_deref() == Some(path.as_path());

            let mut tab = div()
                .id(("editor-tab", ix))
                // 标签卡片高 = `rect.height`(28) − `minVOffset`(8) = 20，
                // 上下各留 4px（`IslandsTabPainter.paintTab` 默认密度）。
                .h(px(20.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap_1()
                // 卡片左右各留 4px（`IslandsTabPainter.getHOffsetUnscaled` 默认密度）。
                .mx(px(4.))
                // `EditorTabs.tabInsets` = "-6,8,-6,8"。
                .pl(px(8.))
                .pr(px(8.))
                // 圆角半径 = `MainToolbar.Button.arc`(12) / 2。
                .rounded(px(6.))
                .cursor_pointer()
                .when(is_active, |tab| {
                    // 选中标签是一张带 1px 蓝边（`EditorTabs.underlinedBorderColor`）的
                    // 浅蓝卡片。
                    tab.bg(chrome.tab_selected_bg)
                        .text_color(chrome.fg)
                        .border_1()
                        .border_color(chrome.tab_underline)
                })
                .when(!is_active, |tab| {
                    // 悬停时按 IDEA 的 `paintDimmed()` 恢复满对比度。
                    tab.text_color(chrome.tab_fg_unselected)
                        .hover(|style| style.bg(chrome.control_hover).text_color(chrome.fg))
                })
                .when(is_drop_target, |tab| tab.bg(chrome.hover_bg))
                // 标签上的文件类型图标保留 expUI 的多色配色；未选中且未悬停时
                // 按 `getIconAlpha()` 的 `EditorTabs.unselectedAlpha`(0.75) 变淡。
                .child(
                    img(icon_path)
                        .size(px(16.))
                        .opacity(if is_active || is_hovered {
                            1.0
                        } else {
                            chrome.tab_icon_alpha
                        }),
                )
                // 预览标签用斜体区分（`EditorTabbedContainer` 的 preview 样式）。
                .child(
                    div()
                        .whitespace_nowrap()
                        .when(preview, |label| label.italic())
                        .child(title),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                        this.begin_tab_drag(activate_path.clone());
                        if let Some(idx) = this.index_of(&activate_path) {
                            // 双击预览标签把它固定为普通标签。
                            if event.click_count >= 2 {
                                this.documents[idx].preview = false;
                            }
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
                )
                // 记录悬停的标签：IDEA 的动作面板只在选中、悬停或未保存时绘制。
                .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                    let next = if *hovered {
                        Some(tab_hover_path.clone())
                    } else {
                        None
                    };
                    if this.hovered_tab != next {
                        this.hovered_tab = next;
                        cx.notify();
                    }
                }));

            // `ActionPanel.paint()`：新 UI 下未选中、未悬停、未修改的标签不绘制动作
            // 面板，关闭按钮因此不占宽度；未保存的标签始终显示蓝点。
            if is_active || is_hovered || dirty {
                let marker_hover_path = path.clone();
                let marker_close_path = path.clone();
                let marker = div()
                    .id(("editor-tab-marker", ix))
                    .size(px(16.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                        let next = if *hovered {
                            Some(marker_hover_path.clone())
                        } else {
                            None
                        };
                        if this.hovered_tab_close != next {
                            this.hovered_tab_close = next;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.request_close(marker_close_path.clone(), cx);
                    }));

                tab = tab.child(if dirty && !close_hovered {
                    // 未保存标记取 `CloseTab.DotIcon` 的尺寸：6px 圆点（`IconBadge.INFORMATION`）。
                    marker.child(div().size(px(6.)).rounded_full().bg(chrome.accent))
                } else {
                    // 关闭按钮用 expUI 的 `closeSmall` / `closeSmallHovered`：
                    // 悬停版本自带 10% 前景色的圆形底，必须用 `img()` 才能保留圆底。
                    marker.child(
                        img(format!(
                            "icons/close_small{}{}.svg",
                            if close_hovered { "_hovered" } else { "" },
                            chrome.icon_suffix
                        ))
                        .size(px(16.)),
                    )
                });
            }

            // IDEA 的标签右键菜单：关闭 / 关闭其他 / 关闭全部 / 在项目中定位。
            // 菜单在点击处弹出，表面与下拉菜单一样带 1px 描边。
            let menu_path = path.clone();
            let menu_separator = chrome.separator_line;
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
                        .item(crate::menu::separator(menu_separator))
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
        let overflow = crate::tooltip::with_tooltip(
            "tip-tab-overflow",
            "已打开的文件",
            crate::menu::dropdown_menu(
                "editor-tab-overflow",
                chrome.popup_border,
                Button::new("editor-tab-overflow")
                    .small()
                    .ghost()
                    .icon(IconName::ChevronDown),
                move |menu, _, _| {
                    let mut menu = menu;
                    for (path, title, dirty) in documents.clone() {
                        let item_app = overflow_app.clone();
                        let label = if dirty { format!("● {title}") } else { title };
                        menu =
                            menu.item(PopupMenuItem::new(label).on_click(move |_, window, cx| {
                                item_app
                                    .update(cx, |this, cx| this.open_file(path.clone(), window, cx))
                                    .ok();
                            }));
                    }
                    menu
                },
            ),
            cx,
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
            // 标签行高 = `SingleHeightTabs.UNSCALED_PREF_HEIGHT`（28）。
            .h(px(28.))
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
                    // 第一个标签距标签行左边缘 3px（`IslandsTabPainter.firstTabOffset`）。
                    .pl(px(3.))
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

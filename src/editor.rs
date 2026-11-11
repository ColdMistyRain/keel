use std::path::PathBuf;

use gpui::{prelude::*, *};
use gpui_component::input::{Editor, EditorState};

use crate::app::{Chrome, KeelApp, empty_editor_state};

/// 一个已打开的文件。
pub struct Document {
    pub path: PathBuf,
    pub state: Entity<EditorState>,
    pub dirty: bool,
    pub language: &'static str,
    pub eol: &'static str,
    pub _changes: Subscription,
    /// 编辑器每次重绘（含光标移动）都会触发，用来刷新状态栏的行列号。
    pub _observe: Subscription,
}

impl Document {
    pub fn new(
        path: PathBuf,
        state: Entity<EditorState>,
        language: &'static str,
        eol: &'static str,
        changes: Subscription,
        observe: Subscription,
    ) -> Self {
        Self {
            path,
            state,
            dirty: false,
            language,
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
            return empty_editor_state(chrome, self.project.root.as_deref());
        }

        let tab_bar = self.render_tab_bar(chrome, cx);
        let editor = self.render_active_editor(chrome);

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(tab_bar)
            .child(editor)
            .into_any_element()
    }

    fn render_tab_bar(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let active = self.active;
        let tabs: Vec<(usize, String, bool, PathBuf)> = self
            .documents
            .iter()
            .enumerate()
            .map(|(ix, doc)| (ix, doc.title(), doc.dirty, doc.path.clone()))
            .collect();

        let mut children = Vec::new();
        for (ix, title, dirty, path) in tabs {
            let is_active = active == Some(ix);
            let activate_path = path.clone();
            let close_path = path.clone();
            let (icon, icon_color) = crate::file_icons::file_icon(&path, chrome.muted_fg);

            let mut tab = div()
                .id(("editor-tab", ix))
                .h_full()
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .border_r_1()
                .border_color(chrome.border)
                .cursor_pointer()
                .when(is_active, |tab| {
                    tab.bg(chrome.bg)
                        .text_color(chrome.fg)
                        .border_t_2()
                        .border_color(chrome.tab_underline)
                })
                .when(!is_active, |tab| {
                    tab.bg(chrome.panel).text_color(chrome.muted_fg)
                })
                .child(div().text_color(icon_color).child(icon))
                .child(div().child(title))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event: &MouseDownEvent, window, cx| {
                        if let Some(idx) = this.index_of(&activate_path) {
                            this.activate_document(idx, window, cx);
                        }
                    }),
                );

            // IDEA 用圆点表示未保存，占的就是关闭按钮的位置。
            if dirty {
                tab = tab.child(
                    div()
                        .id(("editor-tab-dirty", ix))
                        .px_1()
                        .text_sm()
                        .text_color(chrome.accent)
                        .cursor_pointer()
                        .child("●")
                        .on_click(cx.listener(move |this, _event, _window, cx| {
                            this.request_close(close_path.clone(), cx);
                        })),
                );
            } else {
                tab = tab.child(
                    div()
                        .id(("editor-tab-close", ix))
                        .px_1()
                        .rounded_sm()
                        .text_xs()
                        .text_color(chrome.muted_fg)
                        .cursor_pointer()
                        .hover(|style| style.bg(chrome.panel))
                        .child("×")
                        .on_click(cx.listener(move |this, _event, _window, cx| {
                            this.request_close(close_path.clone(), cx);
                        })),
                );
            }

            children.push(tab.into_any_element());
        }

        div()
            .flex()
            .items_center()
            .h(px(32.))
            .flex_shrink_0()
            .bg(chrome.panel)
            .border_b_1()
            .border_color(chrome.border)
            .children(children)
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
                    .text_size(chrome.mono_size),
            )
            .into_any_element()
    }
}

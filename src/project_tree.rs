use std::path::{Path, PathBuf};

use gpui::{prelude::*, *};
use gpui_component_assets::IconName;

use crate::app::{Chrome, KeelApp};
use crate::model::VisibleRow;

impl KeelApp {
    pub fn render_project_panel(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let header = div()
            .h(px(30.))
            .px_3()
            .flex()
            .items_center()
            .flex_shrink_0()
            .bg(chrome.header)
            .border_b_1()
            .border_color(chrome.border)
            .text_xs()
            .text_color(chrome.muted_fg)
            .child(self.project.model.title());

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
            let mut list = div()
                .id("project-tree-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .py_1();

            let mut children = Vec::new();
            for (ix, row) in rows.iter().enumerate() {
                children.push(self.render_row(ix, row, selected.as_deref(), chrome, cx));
            }
            list = list.children(children);
            list.into_any_element()
        };

        div()
            .w(px(260.))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(chrome.bg)
            .text_color(chrome.fg)
            .border_r_1()
            .border_color(chrome.border)
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
        let is_dir = row.is_dir;
        let indent = px(8.0 + row.depth as f32 * 14.0);
        let marker = if is_dir {
            if row.expanded { "-" } else { "+" }
        } else {
            " "
        };
        let (icon, icon_color) = if is_dir {
            let folder = if row.expanded {
                IconName::FolderOpen
            } else {
                IconName::Folder
            };
            (folder, chrome.muted_fg)
        } else {
            crate::file_icons::file_icon(&row.path, chrome.muted_fg)
        };

        div()
            .id(("tree-row", ix))
            .h(px(24.))
            .w_full()
            .flex()
            .items_center()
            .gap_1()
            .pl(indent)
            .pr_2()
            .cursor_pointer()
            .when(is_selected, |row| {
                row.bg(chrome.accent).text_color(chrome.accent_fg)
            })
            .hover(|style| style.bg(chrome.hover_bg))
            .child(
                div()
                    .w(px(10.))
                    .text_xs()
                    .text_color(chrome.muted_fg)
                    .child(marker),
            )
            .child(div().text_color(icon_color).child(icon))
            .child(div().min_w_0().child(row.name.clone()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.project.selected = Some(row_path.clone());
                    if is_dir {
                        this.toggle_dir(row_path.clone(), cx);
                    } else if event.click_count >= 2 {
                        this.open_file(row_path.clone(), window, cx);
                    } else {
                        cx.notify();
                    }
                }),
            )
            .into_any_element()
    }
}

use gpui::{prelude::*, *};
use gpui_component::input::Input;

use crate::app::{BottomTab, Chrome, KeelApp, filtered_commands};

impl KeelApp {
    pub fn render_bottom_panel(&self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let mut tabs = Vec::new();
        for tab in BottomTab::ALL {
            let is_active = self.bottom_tab == tab;
            tabs.push(
                div()
                    .id(("bottom-tab", tab.index()))
                    .h_full()
                    .px_3()
                    .flex()
                    .items_center()
                    // 工具窗口标题字号跟随 `ToolWindow.Header.font.size.offset=0`（13px）。
                    .text_size(px(13.))
                    .cursor_pointer()
                    .when(is_active, |tab| {
                        tab.text_color(chrome.fg)
                            .border_b_2()
                            .border_color(chrome.tab_underline)
                    })
                    .when(!is_active, |tab| {
                        tab.text_color(chrome.muted_fg)
                            .hover(|style| style.bg(chrome.control_hover))
                    })
                    .child(tab.label())
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.bottom_tab = tab;
                        this.persisted.bottom_tab = Some(tab.key().to_string());
                        this.persist();
                        cx.notify();
                    }))
                    .into_any_element(),
            );
        }

        let content = match self.bottom_tab {
            BottomTab::Problems => {
                let (text, color) = if self.status.is_error {
                    (self.status.text.clone(), hsla(0.0, 0.65, 0.56, 1.0))
                } else {
                    ("暂无问题".to_string(), chrome.muted_fg)
                };
                div().text_color(color).child(text)
            }
            BottomTab::Run => div()
                .text_color(chrome.muted_fg)
                .child("尚未创建运行配置，Cargo 集成将在下一阶段接入"),
            BottomTab::Terminal => div()
                .text_color(chrome.muted_fg)
                .child("终端将在后续阶段接入"),
        };

        div()
            .h(px(180.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(chrome.panel)
            .text_color(chrome.panel_fg)
            .rounded(chrome.island_arc)
            .overflow_hidden()
            .mt(px(4.))
            .child(
                div()
                    // 工具窗口标题栏（这里承载标签）的高度是 `ToolWindow.Header.height`
                    // = 41（`ToolWindowHeader.getUnscaledHeight()`，新 UI）。
                    .h(px(41.))
                    .flex()
                    .items_center()
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(chrome.border)
                    .children(tabs),
            )
            .child(
                div()
                    .id("bottom-panel-content")
                    .flex_1()
                    .min_h_0()
                    .p_3()
                    .overflow_y_scroll()
                    .child(content),
            )
            .into_any_element()
    }

    pub fn render_palette(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let Some(palette) = &self.palette else {
            return div().into_any_element();
        };
        let query = palette.query.read(cx).value().to_string();
        let selected = palette.selected;
        let matches = filtered_commands(&query);
        let selected = selected.min(matches.len().saturating_sub(1));

        let mut items = Vec::new();
        for (ix, command) in matches.iter().copied().enumerate() {
            items.push(
                div()
                    .id(("palette-item", ix))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .px_3()
                    .py_2()
                    .cursor_pointer()
                    .when(ix == selected, |item| {
                        item.bg(chrome.selection).text_color(chrome.selection_fg)
                    })
                    // 选中项悬停时保留选中底色。
                    .when(ix != selected, |item| {
                        item.hover(|style| style.bg(chrome.hover_bg))
                    })
                    .child(div().child(command.label()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(chrome.muted_fg)
                            .child(command.shortcut()),
                    )
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.palette = None;
                        this.run_command(command, cx);
                    }))
                    .into_any_element(),
            );
        }

        if items.is_empty() {
            items.push(
                div()
                    .px_3()
                    .py_3()
                    .text_color(chrome.muted_fg)
                    .child("没有匹配的命令")
                    .into_any_element(),
            );
        }

        div()
            .id("command-palette")
            .key_context("KeelPalette")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_start()
            .justify_center()
            .pt(px(120.))
            .bg(hsla(0.0, 0.0, 0.0, 0.35))
            .child(
                div()
                    .w(px(560.))
                    .max_h(px(420.))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded_lg()
                    .bg(chrome.bg)
                    .text_color(chrome.fg)
                    .border_1()
                    .border_color(chrome.border)
                    .shadow_lg()
                    .child(
                        div()
                            .px_2()
                            .py_2()
                            .border_b_1()
                            .border_color(chrome.border)
                            // 输入框圆角跟随 `Component.arc.compact`（6px）。
                            .child(Input::new(&palette.query).rounded(px(6.))),
                    )
                    .child(
                        div()
                            .id("palette-list")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .py_1()
                            .children(items),
                    ),
            )
            .into_any_element()
    }
}

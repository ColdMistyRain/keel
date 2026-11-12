use gpui::{prelude::*, *};

use crate::app::{Chrome, CommandId, KeelApp, SettingsPane, StatusMessage};

/// 左侧工具窗口侧边条的宽度，项目面板的拖拽计算也依赖它。
pub const STRIPE_WIDTH: f32 = 40.;

impl KeelApp {
    /// IDEA 风格的左侧工具窗口侧边条：图标来自 expUI 图标集。
    pub fn render_tool_window_stripe(
        &mut self,
        chrome: &Chrome,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let project = self.stripe_item(
            "stripe-project",
            "project",
            self.project_panel_visible,
            chrome,
            cx,
            |this, _window, cx| this.run_command(CommandId::ToggleProjectPanel, cx),
        );
        let commit = self.stripe_item(
            "stripe-commit",
            "commit",
            false,
            chrome,
            cx,
            |this, _, cx| this.report_not_available("提交工具窗口尚未接入", cx),
        );
        let bookmarks = self.stripe_item(
            "stripe-bookmarks",
            "bookmarks",
            false,
            chrome,
            cx,
            |this, _, cx| this.report_not_available("书签工具窗口尚未接入", cx),
        );
        let structure = self.stripe_item(
            "stripe-structure",
            "structure",
            false,
            chrome,
            cx,
            |this, _, cx| this.report_not_available("结构工具窗口尚未接入", cx),
        );
        let search = self.stripe_item(
            "stripe-search",
            "search",
            false,
            chrome,
            cx,
            |this, window, cx| this.open_palette(window, cx),
        );
        let vcs = self.stripe_item("stripe-vcs", "vcs", false, chrome, cx, |this, _, cx| {
            this.report_not_available("Git 集成尚未接入", cx)
        });
        let services = self.stripe_item(
            "stripe-services",
            "services",
            false,
            chrome,
            cx,
            |this, _, cx| this.report_not_available("服务工具窗口尚未接入", cx),
        );
        let settings = self.stripe_item(
            "stripe-settings",
            "settings",
            false,
            chrome,
            cx,
            |this, _window, cx| this.open_settings(SettingsPane::Appearance, cx),
        );

        div()
            .w(px(STRIPE_WIDTH))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .items_center()
            .py_1()
            .bg(chrome.panel)
            .border_r_1()
            .border_color(chrome.border)
            .child(project)
            .child(commit)
            .child(bookmarks)
            .child(structure)
            .child(search)
            .child(vcs)
            .child(services)
            .child(div().flex_1())
            .child(settings)
            .into_any_element()
    }

    fn stripe_item(
        &self,
        id: &'static str,
        icon: &'static str,
        active: bool,
        chrome: &Chrome,
        cx: &mut Context<Self>,
        handler: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        div()
            .id(id)
            .w(px(STRIPE_WIDTH))
            .h(px(32.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(
                div()
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .when(active, |item| item.bg(chrome.control_hover))
                    .hover(|style| style.bg(chrome.control_hover))
                    .text_color(chrome.muted_fg)
                    .child(
                        svg()
                            .path(SharedString::from(format!(
                                "icons/{icon}{}.svg",
                                chrome.icon_suffix
                            )))
                            .size(px(20.))
                            .text_color(chrome.muted_fg),
                    ),
            )
            .on_click(cx.listener(move |this, _event, window, cx| handler(this, window, cx)))
            .into_any_element()
    }

    fn report_not_available(&mut self, message: &str, cx: &mut Context<Self>) {
        self.status = StatusMessage::info(message);
        cx.notify();
    }
}

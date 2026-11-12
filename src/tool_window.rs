use gpui::{prelude::*, *};
use gpui_component_assets::IconName;

use crate::app::{Chrome, CommandId, KeelApp, StatusMessage};

/// 左侧工具窗口侧边条的宽度，项目面板的拖拽计算也依赖它。
pub const STRIPE_WIDTH: f32 = 40.;

impl KeelApp {
    /// IDEA 风格的左侧工具窗口侧边条。
    pub fn render_tool_window_stripe(
        &mut self,
        chrome: &Chrome,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let project_active = self.project_panel_visible;

        let project = self.stripe_item(
            "stripe-project",
            IconName::Folder,
            project_active,
            chrome,
            cx,
            |this, _window, cx| this.run_command(CommandId::ToggleProjectPanel, cx),
        );
        let search = self.stripe_item(
            "stripe-search",
            IconName::Search,
            false,
            chrome,
            cx,
            |this, window, cx| this.open_palette(window, cx),
        );
        let git = self.stripe_item(
            "stripe-git",
            IconName::GitBranch,
            false,
            chrome,
            cx,
            |this, _window, cx| this.report_not_available("Git 集成尚未接入", cx),
        );
        let maven = self.stripe_item(
            "stripe-maven",
            IconName::Package,
            false,
            chrome,
            cx,
            |this, _window, cx| this.report_not_available("Maven 集成尚未接入", cx),
        );
        let settings = self.stripe_item(
            "stripe-settings",
            IconName::Settings,
            false,
            chrome,
            cx,
            |this, _window, cx| this.report_not_available("设置页尚未接入", cx),
        );

        div()
            .w(px(STRIPE_WIDTH))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .py_1()
            .bg(chrome.panel)
            .border_r_1()
            .border_color(chrome.border)
            .child(project)
            .child(search)
            .child(git)
            .child(maven)
            .child(div().flex_1())
            .child(settings)
            .into_any_element()
    }

    fn stripe_item(
        &self,
        id: &'static str,
        icon: IconName,
        active: bool,
        chrome: &Chrome,
        cx: &mut Context<Self>,
        handler: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        div()
            .id(id)
            .size(px(STRIPE_WIDTH))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .when(active, |item| {
                item.bg(chrome.accent).text_color(chrome.accent_fg)
            })
            .when(!active, |item| {
                item.text_color(chrome.muted_fg)
                    .hover(|style| style.bg(chrome.panel).text_color(chrome.fg))
            })
            .child(icon)
            .on_click(cx.listener(move |this, _event, window, cx| handler(this, window, cx)))
            .into_any_element()
    }

    fn report_not_available(&mut self, message: &str, cx: &mut Context<Self>) {
        self.status = StatusMessage::info(message);
        cx.notify();
    }
}

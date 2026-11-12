use gpui::{prelude::*, *};
use gpui_component_assets::IconName;

use crate::app::{BottomTab, Chrome, CommandId, KeelApp, StatusMessage};

/// 左侧工具窗口侧边条的宽度，项目面板的拖拽计算也依赖它。
pub const STRIPE_WIDTH: f32 = 44.;

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

        // IDEA 把底部工具窗口的入口放在左侧条的最下方。
        let profiler = self.bottom_icon_item(
            "stripe-profiler",
            IconName::Gauge,
            chrome,
            cx,
            |this, _, cx| this.report_not_available("性能分析尚未接入", cx),
        );
        let services =
            self.bottom_svg_item("stripe-services", "services", chrome, cx, |this, _, cx| {
                this.report_not_available("服务工具窗口尚未接入", cx)
            });
        let terminal = self.bottom_item(
            "stripe-terminal",
            BottomTab::Terminal,
            IconName::SquareTerminal,
            chrome,
            cx,
        );
        let problems = self.bottom_item(
            "stripe-problems",
            BottomTab::Problems,
            IconName::CircleAlert,
            chrome,
            cx,
        );
        let git = self.bottom_icon_item(
            "stripe-git",
            IconName::GitBranch,
            chrome,
            cx,
            |this, _, cx| this.report_not_available("Git 工具窗口尚未接入", cx),
        );

        div()
            .w(px(STRIPE_WIDTH))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .items_center()
            .py_1()
            .bg(chrome.canvas)
            .child(project)
            .child(commit)
            .child(bookmarks)
            .child(structure)
            .child(search)
            .child(div().flex_1())
            .child(profiler)
            .child(services)
            .child(terminal)
            .child(problems)
            .child(git)
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
        let icon_color = if active {
            chrome.accent_fg
        } else {
            chrome.muted_fg
        };

        div()
            .id(id)
            .w(px(STRIPE_WIDTH))
            .h(px(40.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(
                div()
                    .size(px(34.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .when(active, |item| item.bg(chrome.accent).text_color(icon_color))
                    .hover(|style| style.bg(chrome.control_hover))
                    .text_color(icon_color)
                    .child(
                        svg()
                            .path(SharedString::from(format!(
                                "icons/{icon}{}.svg",
                                chrome.icon_suffix
                            )))
                            .size(px(20.))
                            .text_color(icon_color),
                    ),
            )
            .on_click(cx.listener(move |this, _event, window, cx| handler(this, window, cx)))
            .into_any_element()
    }

    fn bottom_item(
        &self,
        id: &'static str,
        tab: BottomTab,
        icon: IconName,
        chrome: &Chrome,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let active = self.bottom_panel_visible && self.bottom_tab == tab;
        let icon_color = if active {
            chrome.accent_fg
        } else {
            chrome.muted_fg
        };

        div()
            .id(id)
            .w(px(STRIPE_WIDTH))
            .h(px(40.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(
                div()
                    .size(px(34.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .when(active, |item| item.bg(chrome.accent))
                    .hover(|style| style.bg(chrome.control_hover))
                    .text_color(icon_color)
                    .child(icon),
            )
            .on_click(cx.listener(move |this, _event, _window, cx| {
                if this.bottom_panel_visible && this.bottom_tab == tab {
                    this.bottom_panel_visible = false;
                } else {
                    this.bottom_panel_visible = true;
                    this.bottom_tab = tab;
                }
                this.persisted.bottom_tab = Some(tab.key().to_string());
                this.persisted.bottom_panel_visible = this.bottom_panel_visible;
                this.persist();
                cx.notify();
            }))
            .into_any_element()
    }

    fn bottom_icon_item(
        &self,
        id: &'static str,
        icon: IconName,
        chrome: &Chrome,
        cx: &mut Context<Self>,
        handler: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        self.stripe_placeholder(id, chrome, cx, handler, move |_| icon.into_any_element())
    }

    fn bottom_svg_item(
        &self,
        id: &'static str,
        icon: &'static str,
        chrome: &Chrome,
        cx: &mut Context<Self>,
        handler: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let path = SharedString::from(format!("icons/{icon}{}.svg", chrome.icon_suffix));
        self.stripe_placeholder(id, chrome, cx, handler, move |color| {
            svg()
                .path(path.clone())
                .size(px(20.))
                .text_color(color)
                .into_any_element()
        })
    }

    /// 还没有真实工具窗口的占位按钮：点击后只在状态栏给出说明。
    fn stripe_placeholder(
        &self,
        id: &'static str,
        chrome: &Chrome,
        cx: &mut Context<Self>,
        handler: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        render_icon: impl Fn(Hsla) -> AnyElement + 'static,
    ) -> AnyElement {
        let icon_color = chrome.muted_fg;
        div()
            .id(id)
            .w(px(STRIPE_WIDTH))
            .h(px(40.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(
                div()
                    .size(px(34.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .hover(|style| style.bg(chrome.control_hover))
                    .text_color(icon_color)
                    .child(render_icon(icon_color)),
            )
            .on_click(cx.listener(move |this, _event, window, cx| handler(this, window, cx)))
            .into_any_element()
    }

    fn report_not_available(&mut self, message: &str, cx: &mut Context<Self>) {
        self.status = StatusMessage::info(message);
        cx.notify();
    }
}

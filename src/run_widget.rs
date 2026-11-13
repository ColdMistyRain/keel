//! 主工具栏的 run widget（IDEA `RedesignedRunWidget`）。
//!
//! 结构：运行配置选择器 + 绿色运行按钮 + 调试按钮 + 更多操作。
//! 运行中主按钮变成红色停止按钮（`RunWidget.runningBackground /
//! stopBackground`）。当前没有真正的执行器，运行 / 停止只驱动「运行」
//! 工具窗口与状态栏提示。

use gpui::{prelude::*, *};
use gpui_component::Sizable as _;
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants as _};
use gpui_component::menu::PopupMenuItem;

use crate::actions::{Debug, Run, Stop};
use crate::app::{BottomTab, Chrome, KeelApp, StatusMessage};
use crate::idea_theme::color;

/// run widget 的高度与按钮边长（`RunWidget.toolbarHeight / actionButtonWidth`）。
const HEIGHT: f32 = 30.;
const BUTTON: f32 = 30.;

impl KeelApp {
    /// 当前运行配置名：打开文件的文件名，没有文件时用 `main`。
    pub fn run_config_name(&self) -> String {
        self.active_document()
            .and_then(|doc| doc.path.file_stem())
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "main".to_string())
    }

    /// 开始运行：打开「运行」工具窗口并切换按钮状态。
    pub fn start_run(&mut self, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        self.running = true;
        self.bottom_panel_visible = true;
        self.bottom_tab = BottomTab::Run;
        self.persisted.bottom_panel_visible = true;
        self.persisted.bottom_tab = Some(BottomTab::Run.key().to_string());
        self.persist();
        self.status = StatusMessage::info(format!("正在运行 {}…", self.run_config_name()));
        cx.notify();
    }

    pub fn stop_run(&mut self, cx: &mut Context<Self>) {
        if !self.running {
            return;
        }
        self.running = false;
        self.status = StatusMessage::info("已停止运行");
        cx.notify();
    }

    /// 调试按钮：P4 之前只打开运行窗口并提示。
    pub fn toggle_debug(&mut self, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        self.bottom_panel_visible = true;
        self.bottom_tab = BottomTab::Run;
        self.persisted.bottom_panel_visible = true;
        self.persisted.bottom_tab = Some(BottomTab::Run.key().to_string());
        self.persist();
        self.status = StatusMessage::info("调试将在后续阶段接入");
        cx.notify();
    }

    pub fn on_run(&mut self, _: &Run, _window: &mut Window, cx: &mut Context<Self>) {
        self.start_run(cx);
    }

    pub fn on_stop(&mut self, _: &Stop, _window: &mut Window, cx: &mut Context<Self>) {
        self.stop_run(cx);
    }

    pub fn on_debug(&mut self, _: &Debug, _window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_debug(cx);
    }

    /// 标题栏右侧的 run widget：配置选择、运行、调试、更多。
    pub fn render_run_widget(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let name = self.run_config_name();
        let running = self.running;
        let separator = chrome.separator_line;
        // 与主工具栏一致：悬停/按下用 `MainToolbar.Icon.*` 的灰色覆盖色；
        // 文字色取 `RunWidget.foreground`（#000000），不是次要文字色。
        let run_variant = ButtonCustomVariant::new(cx)
            .foreground(chrome.fg)
            .hover(chrome.control_hover)
            .active(chrome.control_pressed);

        let config_selector = crate::menu::dropdown_menu(
            "run-config",
            chrome.popup_border,
            Button::new("run-config")
                .custom(run_variant)
                .small()
                .h(px(HEIGHT))
                .pl(px(10.))
                .pr(px(6.))
                .rounded(px(6.))
                .text_size(px(13.))
                .label(name.clone())
                .child(
                    svg()
                        .path("icons/chevron_down.svg")
                        .size(px(16.))
                        .ml(px(2.))
                        .text_color(chrome.muted_fg),
                ),
            move |menu, _, _| {
                menu.menu(format!("运行 '{name}'"), Box::new(Run))
                    .menu("调试", Box::new(Debug))
                    .item(crate::menu::separator(separator))
                    .item(PopupMenuItem::new("编辑配置…").disabled(true))
            },
        );

        let run_button = div()
            .id("run-action")
            .w(px(BUTTON))
            .h(px(BUTTON))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .cursor_pointer()
            .when(running, |button| {
                button.bg(color(self.theme_kind.tokens().stop_background))
            })
            .when(!running, |button| {
                button.hover(|style| style.bg(chrome.control_hover))
            })
            .child(if running {
                img("icons/stop.svg").size(px(16.)).into_any_element()
            } else {
                img("icons/run.svg").size(px(16.)).into_any_element()
            })
            .on_click(cx.listener(|this, _event, _window, cx| {
                if this.running {
                    this.stop_run(cx);
                } else {
                    this.start_run(cx);
                }
            }));

        let debug_button = div()
            .id("run-debug")
            .w(px(BUTTON))
            .h(px(BUTTON))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .when(running, |button| button.opacity(0.4))
            .when(!running, |button| {
                button
                    .cursor_pointer()
                    .hover(|style| style.bg(chrome.control_hover))
            })
            .child(img("icons/debug.svg").size(px(16.)))
            .on_click(cx.listener(|this, _event, _window, cx| this.toggle_debug(cx)));

        // “更多”也是 `RunWidget.actionButtonWidth`（30×30）的按钮，
        // 之前用 24×24 的 `icon_button`，与左右两个 30px 按钮不一致。
        let more_button = Button::new("run-more")
            .custom(run_variant)
            .small()
            .w(px(BUTTON))
            .h(px(BUTTON))
            .p_0()
            .rounded(px(6.))
            .child(
                svg()
                    .path("icons/more_vertical.svg")
                    // `RedesignedRunWidget` 的 `iconSize = { 16 }`。
                    .size(px(16.))
                    .text_color(chrome.muted_fg),
            );

        let more = crate::menu::dropdown_menu(
            "run-more",
            chrome.popup_border,
            more_button,
            move |menu, _, _| {
                menu.menu("运行", Box::new(Run))
                    .menu("停止", Box::new(Stop))
                    .menu("调试", Box::new(Debug))
                    .item(crate::menu::separator(separator))
                    .item(PopupMenuItem::new("编辑配置…").disabled(true))
            },
        );

        div()
            .h(px(HEIGHT))
            .flex()
            .items_center()
            .gap(px(2.))
            .child(config_selector)
            .child(run_button)
            .child(debug_button)
            .child(more)
            .into_any_element()
    }
}

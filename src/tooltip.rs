//! Keel 自绘的工具提示。
//!
//! 组件库的 `Tooltip` 把表面样式写死：圆角取 `theme.radius`（8）、描边取
//! `theme.border`（为了编辑器缩进参考线被设成了 `INDENT_GUIDE` 的 #EBECF0），
//! 而 IDEA 的工具提示是 6px 圆角 + `ToolTip.borderColor`（`feedback-border`）。
//! 组件库没有暴露工具提示样式的接口（`ManagedTooltipExt` 是 crate 私有），
//! 所以这里用 `on_hover` + 延迟 + 自绘弹层实现。

use std::time::Duration;

use gpui::{prelude::*, *};

/// IDEA 工具提示的初始显示延迟。
pub const TOOLTIP_DELAY: Duration = Duration::from_millis(500);

/// 鼠标位置到提示框左上角的偏移。
const TOOLTIP_OFFSET: Point<Pixels> = point(px(12.), px(20.));

/// 正在显示的工具提示。
pub struct KeelTip {
    pub text: SharedString,
    /// 弹层左上角位置（窗口坐标）。
    pub position: Point<Pixels>,
}

impl KeelTip {
    /// 由悬停时的鼠标位置算出提示框位置。
    pub fn at(text: SharedString, mouse: Point<Pixels>) -> Self {
        Self {
            text,
            position: mouse + TOOLTIP_OFFSET,
        }
    }
}

/// 给元素挂一个 IDEA 风格的悬停提示；`id` 需要在同一父节点下唯一。
pub fn with_tooltip(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
    element: impl IntoElement,
    cx: &mut Context<crate::app::KeelApp>,
) -> AnyElement {
    let text: SharedString = text.into();
    let app = cx.entity().downgrade();
    div()
        .id(id)
        .on_hover(move |hovered, window, cx| {
            let mouse = window.mouse_position();
            app.update(cx, |this, cx| {
                this.on_tooltip_hover(text.clone(), *hovered, mouse, cx)
            })
            .ok();
        })
        .child(element)
        .into_any_element()
}

/// 工具提示弹层的渲染。
pub fn render(tip: &KeelTip, chrome: &crate::app::Chrome, window: &Window) -> AnyElement {
    let size = window.bounds().size;
    let position = tip.position;
    deferred(
        anchored().child(
            div().w(size.width).h(size.height).child(
                anchored()
                    .position(position)
                    .snap_to_window_with_margin(px(8.))
                    .child(
                        div()
                            // IDEA 的 `ToolTip`：6px 圆角、`feedback-border`
                            // 1px 描边、`feedback-bg` 底色、13px 正文，无阴影。
                            .rounded(px(6.))
                            .border_1()
                            .border_color(chrome.feedback_border)
                            .bg(chrome.feedback)
                            .text_color(chrome.fg)
                            .text_size(px(13.))
                            .px(px(8.))
                            .py(px(4.))
                            .whitespace_nowrap()
                            .child(tip.text.clone()),
                    ),
            ),
        ),
    )
    // 工具提示盖在菜单等弹层之上。
    .priority(101)
    .into_any_element()
}

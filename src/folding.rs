//! ED-02 / ED-03：代码折叠与行号槽。
//!
//! gpui-component 的代码编辑器已经按语法块提供折叠候选，并在行号槽里绘制
//! 折叠箭头（`layout_fold_icons`：当前行、悬停行和已折叠行都会显示）。
//! 但折叠状态保存在私有的 `DisplayMap` 里（`pub(super)`），公开 API 只有
//! `folding(bool)` 与 `unfold_at(position)`，没有「折叠指定行」的入口。
//!
//! 因此这里实现可以做到的一半：「展开全部」逐行调用 `unfold_at`；
//! 「折叠全部」与行号槽的断点占位需要组件暴露 `set_folded`/gutter 标记接口，
//! 在补齐之前不做假实现。

use gpui::{prelude::*, *};
use gpui_component::input::Position;

use crate::actions::UnfoldAll;
use crate::app::{KeelApp, StatusMessage};

impl KeelApp {
    /// 展开本文档的全部折叠（IDEA 的「展开全部」）。
    pub fn on_unfold_all(&mut self, _: &UnfoldAll, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.active.and_then(|ix| self.documents.get(ix)) else {
            return;
        };
        let state = doc.state.clone();
        let lines = state.read(cx).value().lines().count();
        if lines == 0 {
            return;
        }

        // 折叠会隐藏多行，一次调用只能打开覆盖该行的所有折叠；
        // 从头到尾扫一遍就能清空全部折叠（`unfold_at` 会打开嵌套的折叠）。
        state.update(cx, |state, cx| {
            for line in 0..lines {
                state.unfold_at(Position::new(line as u32, 0), cx);
            }
        });

        self.status = StatusMessage::info("已展开全部折叠");
        cx.notify();
    }
}

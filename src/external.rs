//! ED-06：外部修改检测与重载。
//!
//! IDEA 的做法是监听虚拟文件系统的 `VFS_CHANGES` 事件：文件在 IDE 之外被改动时，
//! 没有本地未保存修改的文档自动从磁盘重载，有未保存修改的文档则弹出对话框让用户
//! 选择。这里没有 VFS，改用定时比对角「长度 + 修改时间」，语义保持一致。

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use gpui::{prelude::*, *};
use gpui_component::button::{Button, ButtonVariants as _};

use crate::actions::ReloadFromDisk;
use crate::app::{Chrome, KeelApp, StatusMessage, short_name};

/// 轮询间隔。IDEA 依赖操作系统通知，这里退化为 2 秒一次的 stat。
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// 磁盘上文件的标识：长度 + 修改时间，任一变化都视为被外部修改。
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DiskStamp {
    len: u64,
    modified: Option<SystemTime>,
}

/// 读取文件的当前标识；文件不存在或不可访问时返回 `None`。
pub fn disk_stamp(path: &Path) -> Option<DiskStamp> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(DiskStamp {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

/// 外部修改与本地未保存修改冲突时的待决状态。
pub struct ExternalConflict {
    pub path: PathBuf,
    /// 磁盘上的最新内容，用户选择「从磁盘重新加载」时直接使用。
    pub stamp: DiskStamp,
    pub text: String,
}

impl KeelApp {
    /// 启动外部修改轮询。任务随应用存续，窗口关闭时自然结束。
    pub fn watch_external_changes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let task = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(POLL_INTERVAL).await;

                let Ok(stamps) = this.update(cx, |app, _| app.document_stamps()) else {
                    break;
                };
                if stamps.is_empty() {
                    continue;
                }

                // stat 与读取都放到后台，避免在 UI 线程上碰磁盘。
                let changed = cx
                    .background_spawn(async move {
                        stamps
                            .into_iter()
                            .filter_map(|(path, known)| {
                                let stamp = disk_stamp(&path)?;
                                if Some(stamp) == known {
                                    return None;
                                }
                                let text = std::fs::read_to_string(&path).ok()?;
                                Some((path, stamp, text))
                            })
                            .collect::<Vec<_>>()
                    })
                    .await;
                if changed.is_empty() {
                    continue;
                }

                if this
                    .update_in(cx, |app, window, cx| {
                        app.apply_external_changes(changed, window, cx)
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        self.tasks.push(task);
    }

    fn document_stamps(&self) -> Vec<(PathBuf, Option<DiskStamp>)> {
        self.documents
            .iter()
            .map(|doc| (doc.path.clone(), doc.disk))
            .collect()
    }

    /// 处理一批外部修改：无本地修改的直接重载，有本地修改的弹对话框。
    fn apply_external_changes(
        &mut self,
        changed: Vec<(PathBuf, DiskStamp, String)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut reloaded = Vec::new();
        for (path, stamp, text) in changed {
            let Some(idx) = self.index_of(&path) else {
                continue;
            };
            if self.documents[idx].dirty {
                // 一次只处理一个冲突，避免多个对话框叠在一起；
                // 其余文档等下一次轮询再提示。
                if self.external_conflict.is_none() {
                    self.external_conflict = Some(ExternalConflict {
                        path: path.clone(),
                        stamp,
                        text,
                    });
                }
                continue;
            }
            self.reload_document(idx, stamp, text, window, cx);
            reloaded.push(short_name(&path));
        }

        if !reloaded.is_empty() {
            let message = if reloaded.len() == 1 {
                format!("已从磁盘重新加载 {}", reloaded[0])
            } else {
                format!("已从磁盘重新加载 {} 个文件", reloaded.len())
            };
            self.status = StatusMessage::info(message.clone());
            self.notify(message, false, cx);
            cx.notify();
        } else if self.external_conflict.is_some() {
            cx.notify();
        }
    }

    /// 用磁盘内容覆盖编辑器，并重置未保存标记与已记录的磁盘标识。
    fn reload_document(
        &mut self,
        idx: usize,
        stamp: DiskStamp,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = self.documents.get(idx) else {
            return;
        };
        let state = doc.state.clone();
        // `set_value` 会清空撤销历史，语义与 IDEA 的「从磁盘重新加载」一致。
        state.update(cx, |state, cx| state.set_value(text, window, cx));
        if let Some(doc) = self.documents.get_mut(idx) {
            doc.dirty = false;
            doc.disk = Some(stamp);
        }
        cx.notify();
    }

    // ---- 菜单 / 动作 ----

    pub fn on_reload_from_disk(
        &mut self,
        _: &ReloadFromDisk,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self.active_document().map(|doc| doc.path.clone()) else {
            self.status = StatusMessage::error("没有可重新加载的文件");
            cx.notify();
            return;
        };
        let task = cx.spawn_in(window, async move |this, cx| {
            let read_path = path.clone();
            let loaded = cx
                .background_spawn(async move {
                    let stamp = disk_stamp(&read_path)?;
                    let text = std::fs::read_to_string(&read_path).ok()?;
                    Some((stamp, text))
                })
                .await;
            this.update_in(cx, |app, window, cx| {
                match loaded {
                    Some((stamp, text)) => {
                        if let Some(idx) = app.index_of(&path) {
                            app.reload_document(idx, stamp, text, window, cx);
                        }
                        let message = format!("已从磁盘重新加载 {}", short_name(&path));
                        app.status = StatusMessage::info(message.clone());
                        app.notify(message, false, cx);
                    }
                    None => {
                        app.status =
                            StatusMessage::error(format!("无法读取 {}", short_name(&path)));
                    }
                }
                cx.notify();
            })
            .ok();
        });
        self.tasks.push(task);
    }

    /// 外部修改冲突对话框：重新加载会丢弃本地修改，保留则不再追问这个版本。
    pub fn render_external_conflict(
        &mut self,
        chrome: &Chrome,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(conflict) = &self.external_conflict else {
            return div().into_any_element();
        };
        let name = short_name(&conflict.path);

        div()
            .id("external-conflict-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(hsla(0.0, 0.0, 0.0, 0.45))
            .child(
                div()
                    .w(px(460.))
                    .p_5()
                    .rounded_lg()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .bg(chrome.bg)
                    .text_color(chrome.fg)
                    .border_1()
                    .border_color(chrome.border)
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("{name} 已在外部被修改")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(chrome.muted_fg)
                            .child("磁盘上的内容已变化，同时编辑器里有未保存的修改。"),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("external-reload")
                                    .label("从磁盘重新加载")
                                    .primary()
                                    .h(px(28.))
                                    .min_w(px(72.))
                                    .px(px(14.))
                                    .font_weight(FontWeight::BOLD)
                                    .on_click(cx.listener(|this, _event, window, cx| {
                                        this.reload_external_conflict(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("external-keep")
                                    .label("保留当前内容")
                                    .ghost()
                                    .h(px(28.))
                                    .min_w(px(72.))
                                    .px(px(14.))
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.keep_external_conflict(cx);
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn reload_external_conflict(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(conflict) = self.external_conflict.take() else {
            return;
        };
        let Some(idx) = self.index_of(&conflict.path) else {
            return;
        };
        self.reload_document(idx, conflict.stamp, conflict.text, window, cx);
        let message = format!("已从磁盘重新加载 {}", short_name(&conflict.path));
        self.status = StatusMessage::info(message.clone());
        self.notify(message, false, cx);
        cx.notify();
    }

    fn keep_external_conflict(&mut self, cx: &mut Context<Self>) {
        let Some(conflict) = self.external_conflict.take() else {
            return;
        };
        // 把磁盘的当前版本记为基线：本地修改仍然保留，保存时会覆盖它。
        if let Some(idx) = self.index_of(&conflict.path)
            && let Some(doc) = self.documents.get_mut(idx)
        {
            doc.disk = Some(conflict.stamp);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::disk_stamp;
    use std::io::Write;

    #[test]
    fn 写入磁盘后文件标识会变化() {
        let path = std::env::temp_dir().join("keel_disk_stamp_test.txt");
        std::fs::write(&path, "one").expect("写入测试文件");
        let first = disk_stamp(&path).expect("读取标识");

        // 长度不同的写入一定改变标识；避免依赖修改时间的精度。
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("打开测试文件");
        file.write_all(b" more").expect("追加内容");
        drop(file);

        let second = disk_stamp(&path).expect("再次读取标识");
        assert!(first != second);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn 不存在的文件没有标识() {
        let path = std::env::temp_dir().join("keel_disk_stamp_missing.txt");
        let _ = std::fs::remove_file(&path);
        assert!(disk_stamp(&path).is_none());
    }
}

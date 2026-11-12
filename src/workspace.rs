//! Keel spike 的 UI 状态层：把 `keel-workspace` 的模型包成 nsh 信号。
//!
//! 目录扫描、树摊平、语言识别都在 `crates/keel-workspace`（与 UI 框架无关）；
//! 这里只保留 nsh 需要的信号、文档和保存逻辑。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use keel_workspace::{FlatRow, ProjectSnapshot, find_project_root, flatten, scan_snapshot};
use nsh_core::prelude::*;

pub use keel_workspace::language_name as language_of;

/// 编辑器里打开的一个文档。
#[derive(Clone, PartialEq)]
pub struct Doc {
    pub path: PathBuf,
    /// 标签页显示的文件名。
    pub name: SharedString,
    pub text: SharedString,
    /// 有未保存修改时标签显示圆点。
    pub dirty: bool,
    /// 预览标签（IDEA 的 preview tab）：斜体显示，下一次预览会替换它。
    pub preview: bool,
}

impl Doc {
    fn open(path: PathBuf) -> io::Result<Self> {
        Self::open_as(path, false)
    }

    fn open_as(path: PathBuf, preview: bool) -> io::Result<Self> {
        let text = fs::read_to_string(&path)?;
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Ok(Self {
            path,
            name: name.into(),
            text: text.into(),
            dirty: false,
            preview,
        })
    }
}

/// 工作区状态。所有字段都是信号句柄，可以自由复制。
#[derive(Clone, Copy)]
pub struct Workspace {
    /// 已打开项目的根目录。
    pub root: Signal<Option<PathBuf>>,
    /// 项目树的不可变快照（`keel-workspace` 的 Workspace Model）。
    pub tree: Signal<Arc<ProjectSnapshot>>,
    /// 展开的节点 id（绝对路径）。
    pub expanded: Signal<Vec<SharedString>>,
    pub selected: Signal<Option<SharedString>>,
    /// 打开的文档，顺序就是标签顺序。
    pub docs: Signal<Vec<Doc>>,
    pub active: Signal<Option<usize>>,
    /// 状态栏的光标位置。
    pub cursor: Signal<SharedString>,
    /// 状态栏左侧的提示（保存、打开失败等）。
    pub message: Signal<SharedString>,
    /// 提示的序号：新提示覆盖旧提示时，旧的自动清除定时器不再生效。
    pub message_seq: Signal<u64>,
}

impl Workspace {
    /// 在根组件里创建一份空工作区；启动路径由 [`Self::open_startup`] 处理。
    pub fn new() -> Self {
        Self {
            root: signal(None),
            tree: signal(Arc::new(ProjectSnapshot::empty())),
            expanded: signal(Vec::new()),
            selected: signal(None),
            docs: signal(Vec::new()),
            active: signal(None),
            cursor: signal("1:1".into()),
            message: signal("".into()),
            message_seq: signal(0),
        }
    }

    /// 启动时打开：`KEEL_DEMO=1` 打开示例项目；否则用
    /// `KEEL_OPEN` 或命令行里的第一个路径。
    pub fn open_startup(&self) {
        if std::env::var_os("KEEL_DEMO").is_some() {
            self.open_demo();
            return;
        }
        if let Some(path) = startup_path() {
            self.open_path(path);
        }
    }

    /// 打开一个路径：目录作为项目；文件先识别项目根，再打开文件。
    pub fn open_path(&self, path: PathBuf) {
        if path.is_dir() {
            self.open_project(path);
            return;
        }
        if !path.is_file() {
            self.say(format!("路径不存在：{}", path.display()));
            return;
        }
        let root = find_project_root(&path)
            .or_else(|| path.parent().map(Path::to_path_buf))
            .unwrap_or_default();
        if root.is_dir() {
            self.open_project(root);
        }
        self.open_files(vec![path.clone()]);
        self.reveal(&path);
    }

    /// 打开仓库内置的示例项目。
    fn open_demo(&self) {
        let Some(dir) = demo_dir() else {
            self.say("找不到 samples/demo");
            return;
        };
        self.open_project(dir.clone());
        let main = dir
            .join("src")
            .join("main")
            .join("java")
            .join("com")
            .join("example")
            .join("demo")
            .join("Main.java");
        if main.is_file() {
            self.open_files(vec![main.clone()]);
            self.reveal(&main);
        }
    }

    /// 打开 `dir`，重建项目树，并展开第一层目录。
    pub fn open_project(&self, dir: PathBuf) {
        if !dir.is_dir() {
            self.say(format!("不是目录：{}", dir.display()));
            return;
        }
        let snapshot = scan_snapshot(&dir);
        let expanded = snapshot
            .nodes
            .iter()
            .filter(|node| node.is_dir && !node.children.is_empty())
            .map(|node| SharedString::from(node.path.display().to_string()))
            .collect();
        self.tree.set(Arc::new(snapshot));
        self.expanded.set(expanded);
        self.selected.set(None);
        self.message
            .set(format!("已打开项目 {}", dir.display()).into());
        self.root.set(Some(dir));
    }

    /// 打开一组文件（固定标签）；已经打开的文件直接切换过去并取消预览态。
    pub fn open_files(&self, paths: Vec<PathBuf>) {
        let mut failures = Vec::new();
        for path in paths {
            if path.is_dir() {
                self.open_project(path);
                continue;
            }
            if let Some(index) = self
                .docs
                .with(|docs| docs.iter().position(|doc| doc.path == path))
            {
                self.docs.update(|docs| {
                    if let Some(doc) = docs.get_mut(index) {
                        doc.preview = false;
                    }
                });
                self.active.set(Some(index));
                continue;
            }
            match Doc::open(path.clone()) {
                Ok(doc) => {
                    let index = self.docs.with(|docs| docs.len());
                    self.docs.update(|docs| docs.push(doc));
                    self.active.set(Some(index));
                }
                Err(error) => failures.push(open_error(&path, &error)),
            }
        }
        match failures.first() {
            Some(error) => self.say(error.clone()),
            None => {
                let name = self
                    .active_doc()
                    .map(|doc| doc.name.to_string())
                    .unwrap_or_default();
                if !name.is_empty() {
                    self.say(format!("已打开 {name}"));
                }
            }
        }
    }

    /// 打开预览标签（IDEA 的 preview tab）：斜体显示，下一个预览会替换它；
    /// 编辑或双击后固定为普通标签。
    pub fn preview_file(&self, path: PathBuf) {
        if path.is_dir() {
            self.open_project(path);
            return;
        }
        if let Some(index) = self
            .docs
            .with(|docs| docs.iter().position(|doc| doc.path == path))
        {
            self.active.set(Some(index));
            return;
        }
        let doc = match Doc::open_as(path.clone(), true) {
            Ok(doc) => doc,
            Err(error) => {
                self.say(open_error(&path, &error));
                return;
            }
        };
        let name = doc.name.clone();
        if let Some(index) = self
            .docs
            .with(|docs| docs.iter().position(|doc| doc.preview))
        {
            self.docs.update(|docs| docs[index] = doc);
            self.active.set(Some(index));
        } else {
            let index = self.docs.with(|docs| docs.len());
            self.docs.update(|docs| docs.push(doc));
            self.active.set(Some(index));
        }
        self.say(format!("预览 {name}"));
    }

    /// 保存当前的文档。
    pub fn save_active(&self) {
        let Some(index) = self.active.get_untracked() else {
            self.say("没有可保存的文档");
            return;
        };
        let Some((path, text, name)) = self.docs.with(|docs| {
            docs.get(index)
                .map(|doc| (doc.path.clone(), doc.text.clone(), doc.name.clone()))
        }) else {
            return;
        };
        match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                self.docs.update(|docs| {
                    if let Some(doc) = docs.get_mut(index) {
                        doc.dirty = false;
                    }
                });
                self.say(format!("已保存 {name}"));
            }
            Err(error) => self.say(format!("保存失败：{error}")),
        }
    }

    /// 关闭第 `index` 个标签页。
    pub fn close_doc(&self, index: usize) {
        let len = self.docs.with(|docs| docs.len());
        if index >= len {
            return;
        }
        self.docs.update(|docs| {
            docs.remove(index);
        });
        let active = self.active.get_untracked();
        let next = match active {
            _ if len == 1 => None,
            Some(current) if current > index => Some(current - 1),
            Some(current) if current == index => Some(current.min(len - 2)),
            other => other,
        };
        self.active.set(next);
    }

    /// 关闭当前标签页。
    pub fn close_active(&self) {
        if let Some(index) = self.active.get_untracked() {
            self.close_doc(index);
        }
    }

    /// 关闭除 `index` 外的所有标签页（标签页右键菜单）。
    pub fn close_others(&self, index: usize) {
        let len = self.docs.with(|docs| docs.len());
        if index >= len {
            return;
        }
        let kept = self.docs.with(|docs| docs[index].clone());
        self.docs.update(|docs| {
            docs.clear();
            docs.push(kept);
        });
        self.active.set(Some(0));
        self.say("已关闭其他标签页");
    }

    /// 关闭全部标签页（标签页右键菜单）。
    pub fn close_all(&self) {
        let count = self.docs.with(|docs| docs.len());
        if count == 0 {
            return;
        }
        self.docs.update(|docs| docs.clear());
        self.active.set(None);
        self.say(format!("已关闭 {count} 个标签页"));
    }

    /// 激活 `path` 对应的标签页；没有打开时返回 `false`。
    pub fn activate_path(&self, path: &Path) -> bool {
        if let Some(index) = self
            .docs
            .with(|docs| docs.iter().position(|doc| doc.path == path))
        {
            self.active.set(Some(index));
            true
        } else {
            false
        }
    }

    /// 关闭 `path` 对应的标签页。
    pub fn close_path(&self, path: &Path) {
        if let Some(index) = self
            .docs
            .with(|docs| docs.iter().position(|doc| doc.path == path))
        {
            self.close_doc(index);
        }
    }

    /// 重新扫描当前项目，保留展开状态（工具窗口标题栏的刷新按钮）。
    pub fn refresh_project(&self) {
        let Some(root) = self.root.get_untracked() else {
            self.say("未打开项目");
            return;
        };
        let snapshot = scan_snapshot(&root);
        self.tree.set(Arc::new(snapshot));
        self.say("已刷新项目");
    }

    /// 在项目树里展开并选中 `path`（Navigate → 在项目中定位）。
    pub fn reveal(&self, path: &Path) -> bool {
        let Some(root) = self.root.get_untracked() else {
            self.say("未打开项目");
            return false;
        };
        if !path.starts_with(&root) {
            self.say("文件不在当前项目中");
            return false;
        }
        let mut ancestors = Vec::new();
        let mut current = path.parent();
        while let Some(dir) = current {
            if dir == root {
                break;
            }
            if !dir.starts_with(&root) {
                break;
            }
            ancestors.push(dir.display().to_string());
            current = dir.parent();
        }
        ancestors.reverse();
        self.expanded.update(|ids| {
            for id in ancestors {
                if !ids.iter().any(|item| item.as_ref() == id) {
                    ids.push(id.into());
                }
            }
        });
        self.selected.set(Some(path.display().to_string().into()));
        true
    }

    /// 当前激活的文档（克隆一份文本，供编辑器绑定）。
    pub fn active_doc(&self) -> Option<Doc> {
        let index = self.active.get_untracked();
        self.docs
            .with(|docs| index.and_then(|index| docs.get(index).cloned()))
    }

    /// 当前激活文档的路径。
    pub fn active_path(&self) -> Option<PathBuf> {
        self.active_doc().map(|doc| doc.path)
    }

    /// 打开的文档数。
    pub fn open_docs(&self) -> usize {
        self.docs.with(|docs| docs.len())
    }

    /// 把编辑器的新文本写回当前文档。
    pub fn update_active_text(&self, text: &str) {
        let Some(index) = self.active.get_untracked() else {
            return;
        };
        self.docs.update(|docs| {
            if let Some(doc) = docs.get_mut(index)
                && doc.text.as_ref() != text
            {
                doc.text = text.to_owned().into();
                doc.dirty = true;
                // 编辑预览标签即固定它（IDEA 的行为）。
                doc.preview = false;
            }
        });
    }

    /// 状态栏提示：显示 4 秒后自动清除，让面包屑回到状态栏左侧。
    pub fn say(&self, text: impl Into<SharedString>) {
        self.message.set(text.into());
        let sequence = self.message_seq.get_untracked().wrapping_add(1);
        self.message_seq.set(sequence);
        let message = self.message;
        let message_seq = self.message_seq;
        set_timeout(Duration::from_secs(4), move || {
            if message_seq.get_untracked() == sequence {
                message.set("".into());
            }
        });
    }

    /// 项目名：根目录的最后一段；未打开项目时为空。
    pub fn root_name(&self) -> SharedString {
        self.root
            .with(|root| {
                root.as_ref()
                    .and_then(|root| root.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .into()
    }

    /// 展开状态下的可见行（spike 不做虚拟化，项目规模足够小）。
    pub fn visible_rows(&self) -> Vec<TreeRow> {
        let expanded: Vec<String> = self
            .expanded
            .get()
            .iter()
            .map(|id| id.to_string())
            .collect();
        flatten(&self.tree.get(), &expanded, 0)
            .into_iter()
            .map(TreeRow::from)
            .collect()
    }

    /// 当前文档相对项目根的路径片段，用于面包屑（读取信号，供 `dynamic` 跟随）。
    pub fn breadcrumbs(&self) -> Vec<SharedString> {
        let index = self.active.get();
        let root = self.root.get();
        let path = index.and_then(|index| {
            self.docs
                .with(|docs| docs.get(index).map(|doc| doc.path.clone()))
        });
        let Some(path) = path else {
            return Vec::new();
        };
        let mut parts = Vec::new();
        if let Some(root) = root
            && let Ok(relative) = path.strip_prefix(&root)
        {
            for part in relative.iter() {
                parts.push(part.to_string_lossy().into_owned().into());
            }
        }
        if parts.is_empty() {
            parts.push(
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
                    .into(),
            );
        }
        parts
    }
}

/// 项目树里显示的一行（nsh 侧，`keel-workspace` 的 `FlatRow` 转成共享字符串）。
#[derive(Clone, PartialEq)]
pub struct TreeRow {
    pub id: SharedString,
    pub label: SharedString,
    pub depth: u32,
    /// 有子项：可以展开 / 收起。
    pub folder: bool,
    pub open: bool,
}

impl From<FlatRow> for TreeRow {
    fn from(row: FlatRow) -> Self {
        Self {
            id: row.id.into(),
            label: row.label.into(),
            depth: row.depth,
            folder: row.folder,
            open: row.open,
        }
    }
}

/// 示例项目的位置：应用目录就是仓库根目录。
fn demo_dir() -> Option<PathBuf> {
    let path = PathBuf::from("samples/demo");
    path.is_dir().then_some(path)
}

/// 启动路径：`KEEL_OPEN` 优先，其次命令行里的第一个存在的路径。
fn startup_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("KEEL_OPEN") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    std::env::args_os().skip(1).map(PathBuf::from).find(|path| {
        let text = path.to_string_lossy();
        !text.starts_with('-') && text != "." && path.exists()
    })
}

/// 打开失败的提示：区分二进制 / 非 UTF-8 与普通 IO 错误。
fn open_error(path: &Path, error: &io::Error) -> String {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    if error.kind() == io::ErrorKind::InvalidData {
        format!("无法预览 {name}：二进制或非 UTF-8 文件")
    } else {
        format!("无法打开 {name}：{error}")
    }
}

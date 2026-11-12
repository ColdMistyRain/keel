//! Keel spike 的 UI 状态层：把 `keel-workspace` 的模型包成 nsh 信号。
//!
//! 目录扫描、树摊平、语言识别都在 `crates/keel-workspace`（与 UI 框架无关）；
//! 这里只保留 nsh 需要的信号、文档和保存逻辑。

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use keel_workspace::{FlatRow, ProjectSnapshot, flatten, scan_snapshot};
use nsh_core::prelude::*;

pub use keel_workspace::{language_label, language_name as language_of};

/// 编辑器里打开的一个文档。
#[derive(Clone, PartialEq)]
pub struct Doc {
    pub path: PathBuf,
    /// 标签页显示的文件名。
    pub name: SharedString,
    pub text: SharedString,
    /// 有未保存修改时标签显示圆点。
    pub dirty: bool,
}

impl Doc {
    fn open(path: PathBuf) -> io::Result<Self> {
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
}

impl Workspace {
    /// 在根组件里创建一份空工作区。
    ///
    /// 设置环境变量 `KEEL_SPIKE_DEMO=1` 时自动打开 `samples/demo`，
    /// 方便截图和手动验证，不影响正常启动。
    pub fn new() -> Self {
        let workspace = Self {
            root: signal(None),
            tree: signal(Arc::new(ProjectSnapshot::empty())),
            expanded: signal(Vec::new()),
            selected: signal(None),
            docs: signal(Vec::new()),
            active: signal(None),
            cursor: signal("Ln 1, Col 1".into()),
            message: signal("就绪".into()),
        };
        if std::env::var_os("KEEL_SPIKE_DEMO").is_some() {
            workspace.open_demo();
        }
        workspace
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
            self.open_files(vec![main]);
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

    /// 打开一组文件；已经打开的文件直接切换过去。
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
                self.active.set(Some(index));
                continue;
            }
            match Doc::open(path) {
                Ok(doc) => {
                    let index = self.docs.with(|docs| docs.len());
                    self.docs.update(|docs| docs.push(doc));
                    self.active.set(Some(index));
                }
                Err(error) => failures.push(error.to_string()),
            }
        }
        match failures.first() {
            Some(error) => self.say(format!("打开失败：{error}")),
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

    /// 当前激活的文档（克隆一份文本，供编辑器绑定）。
    pub fn active_doc(&self) -> Option<Doc> {
        let index = self.active.get_untracked();
        self.docs
            .with(|docs| index.and_then(|index| docs.get(index).cloned()))
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
            }
        });
    }

    /// 状态栏提示。
    pub fn say(&self, text: impl Into<SharedString>) {
        self.message.set(text.into());
    }

    /// 项目名：根目录的最后一段。
    pub fn root_name(&self) -> SharedString {
        self.root
            .with(|root| {
                root.as_ref()
                    .and_then(|root| root.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Keel".to_owned())
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

/// 示例项目的位置：`nsh run` 的工作目录是应用目录，直接从仓库根运行时再退一层。
fn demo_dir() -> Option<PathBuf> {
    for candidate in ["samples/demo", "spike/keel-nsh-spike/samples/demo"] {
        let path = PathBuf::from(candidate);
        if path.is_dir() {
            return Some(path);
        }
    }
    None
}

//! Keel 的工作区模型：项目目录快照、树摊平、语言识别。
//!
//! 对应 IDEA 的 **Workspace Model**：`ProjectModel` 持有一个不可变
//! [`ProjectSnapshot`]，刷新时整体替换并广播变更；UI 只读快照。
//! 与 UI 框架无关，GPUI 版本和 nsh spike 共用。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use keel_core::{MessageBus, Subscription, Topic};

/// 项目树的一个节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectNode {
    pub name: String,
    /// 绝对路径，也是节点 id。
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<ProjectNode>,
}

impl ProjectNode {
    /// 是否是可以展开 / 收起的目录（空目录也算目录，但没有子项）。
    pub fn is_folder(&self) -> bool {
        self.is_dir
    }
}

/// 一个不可变的项目结构快照（IDEA 的 `ImmutableEntityStorage`）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectSnapshot {
    pub root: PathBuf,
    pub nodes: Vec<ProjectNode>,
}

impl ProjectSnapshot {
    pub fn empty() -> Self {
        Self::default()
    }
}

/// 项目模型变更事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectModelChanged {
    pub root: PathBuf,
}

/// 项目模型变更的主题。
pub static PROJECT_MODEL_CHANGED: Topic<ProjectModelChanged> =
    Topic::new("keel.workspace.projectChanged");

/// 一个打开的项目：当前快照 + 变更广播。
#[derive(Clone)]
pub struct ProjectModel {
    root: PathBuf,
    snapshot: Arc<RwLock<Arc<ProjectSnapshot>>>,
    bus: Arc<MessageBus>,
}

impl ProjectModel {
    /// 打开 `root` 并扫描一次。
    pub fn open(root: PathBuf) -> Self {
        let snapshot = Arc::new(scan_snapshot(&root));
        Self {
            root,
            snapshot: Arc::new(RwLock::new(snapshot)),
            bus: MessageBus::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 当前快照（无锁读）。
    pub fn snapshot(&self) -> Arc<ProjectSnapshot> {
        self.snapshot.read().unwrap().clone()
    }

    /// 重新扫描并替换快照，然后广播变更。
    pub fn refresh(&self) {
        let next = Arc::new(scan_snapshot(&self.root));
        *self.snapshot.write().unwrap() = next;
        self.bus.publish(
            &PROJECT_MODEL_CHANGED,
            &ProjectModelChanged {
                root: self.root.clone(),
            },
        );
    }

    /// 订阅本项目的快照变更；返回的句柄释放时自动退订。
    pub fn subscribe(
        &self,
        listener: impl Fn(&ProjectSnapshot) + Send + Sync + 'static,
    ) -> Subscription {
        let snapshot = self.snapshot.clone();
        let root = self.root.clone();
        self.bus.subscribe(
            &PROJECT_MODEL_CHANGED,
            move |event: &ProjectModelChanged| {
                if event.root == root {
                    let current = snapshot.read().unwrap().clone();
                    listener(&current);
                }
            },
        )
    }
}

/// 扫描 `root` 生成一个不可变快照。
pub fn scan_snapshot(root: &Path) -> ProjectSnapshot {
    ProjectSnapshot {
        root: root.to_path_buf(),
        nodes: scan_dir(root, 0),
    }
}

/// 兼容旧调用：只要节点。
pub fn scan_project(root: &Path) -> Vec<ProjectNode> {
    scan_snapshot(root).nodes
}

/// 项目根的标记文件 / 目录（IDEA 的 project root 识别）。
pub const PROJECT_ROOT_MARKERS: [&str; 8] = [
    ".git",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
    "Cargo.toml",
    "package.json",
];

/// 从文件或目录向上找项目根：最近的、含标记的祖先目录。
///
/// 命令行传入单个文件时用它决定打开哪个项目；找不到时调用方退回文件所在目录。
pub fn find_project_root(path: &Path) -> Option<PathBuf> {
    let start = if path.is_dir() {
        path
    } else {
        path.parent()?
    };
    start.ancestors().find_map(|dir| {
        PROJECT_ROOT_MARKERS
            .iter()
            .any(|marker| dir.join(marker).exists())
            .then(|| dir.to_path_buf())
    })
}

fn scan_dir(dir: &Path, depth: usize) -> Vec<ProjectNode> {
    if depth >= 12 {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in entries.flatten().take(500) {
        let path = entry.path();
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        if skip(&name) {
            continue;
        }
        let is_dir = path.is_dir();
        let children = if is_dir {
            scan_dir(&path, depth + 1)
        } else {
            Vec::new()
        };
        let node = ProjectNode {
            name,
            path,
            is_dir,
            children,
        };
        if is_dir {
            dirs.push(node);
        } else {
            files.push(node);
        }
    }
    dirs.sort_by_key(|node| node.name.to_lowercase());
    files.sort_by_key(|node| node.name.to_lowercase());
    dirs.extend(files);
    dirs
}

/// 构建产物和隐藏目录不出现在项目树里。
pub fn skip(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".idea" | ".nsh" | "target" | "node_modules" | "build" | "out" | ".gradle"
    ) || name.starts_with('.')
}

/// 摊平后的一行，给列表直接渲染。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatRow {
    pub id: String,
    pub label: String,
    pub depth: u32,
    pub folder: bool,
    pub open: bool,
}

/// 把快照按展开状态摊平成可见行。
pub fn flatten(snapshot: &ProjectSnapshot, expanded: &[String], depth: u32) -> Vec<FlatRow> {
    flatten_nodes(&snapshot.nodes, expanded, depth)
}

fn flatten_nodes(nodes: &[ProjectNode], expanded: &[String], depth: u32) -> Vec<FlatRow> {
    let mut rows = Vec::new();
    for node in nodes {
        let folder = node.is_folder();
        let id = node.path.display().to_string();
        let open = folder && expanded.iter().any(|item| *item == id);
        rows.push(FlatRow {
            id: id.clone(),
            label: node.name.clone(),
            depth,
            folder,
            open,
        });
        if open {
            rows.extend(flatten_nodes(&node.children, expanded, depth + 1));
        }
    }
    rows
}

/// 按扩展名给出编辑器语言（nsh Editor 接受语言名、扩展名或文件名）。
pub fn language_name(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "rs" => "rust",
        "xml" | "pom" => "xml",
        "toml" => "toml",
        "json" => "json",
        "md" => "markdown",
        "gradle" => "groovy",
        "properties" => "properties",
        "yaml" | "yml" => "yaml",
        "ts" => "typescript",
        "tsx" => "tsx",
        "js" => "javascript",
        "css" => "css",
        "html" => "html",
        "sh" => "shell",
        "sql" => "sql",
        _ => "text",
    }
}

/// 状态栏显示的语言大写名。
pub fn language_label(path: &Path) -> String {
    let ext = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    if ext.is_empty() {
        "Plain Text".to_owned()
    } else {
        ext
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn recognizes_java_and_web_languages() {
        assert_eq!(language_name(Path::new("Main.java")), "java");
        assert_eq!(language_name(Path::new("build.gradle.kts")), "kotlin");
        assert_eq!(language_name(Path::new("pom.xml")), "xml");
        assert_eq!(language_name(Path::new("App.tsx")), "tsx");
        assert_eq!(language_name(Path::new("README")), "text");
        assert_eq!(language_label(Path::new("Main.java")), "JAVA");
        assert_eq!(language_label(Path::new("README")), "Plain Text");
    }

    #[test]
    fn skips_hidden_and_build_outputs() {
        assert!(skip(".git"));
        assert!(skip("target"));
        assert!(skip("node_modules"));
        assert!(!skip("src"));
        assert!(!skip("pom.xml"));
    }

    fn fixture() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("keel-workspace-{}-{stamp}", std::process::id()));
        fs::create_dir_all(dir.join("src").join("main")).unwrap();
        fs::create_dir_all(dir.join(".git")).unwrap();
        fs::write(dir.join("pom.xml"), "<project/>").unwrap();
        fs::write(
            dir.join("src").join("main").join("Main.java"),
            "class Main {}",
        )
        .unwrap();
        fs::write(dir.join(".git").join("HEAD"), "ref").unwrap();
        dir
    }

    #[test]
    fn scans_directories_first_and_skips_git() {
        let dir = fixture();
        let nodes = scan_project(&dir);
        let names: Vec<_> = nodes.iter().map(|node| node.name.as_str()).collect();
        assert_eq!(names, vec!["src", "pom.xml"]);
        assert_eq!(nodes[0].children[0].name, "main");

        let snapshot = scan_snapshot(&dir);
        let flattened = flatten(
            &snapshot,
            &[snapshot.nodes[0].path.display().to_string()],
            0,
        );
        let labels: Vec<_> = flattened.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(labels, vec!["src", "main", "pom.xml"]);
        assert!(flattened[0].open);
        assert!(!flattened[1].open);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn finds_the_nearest_project_root() {
        let dir = fixture();
        let nested_file = dir.join("src").join("main").join("Main.java");
        assert_eq!(find_project_root(&nested_file), Some(dir.clone()));
        assert_eq!(find_project_root(&dir.join("src")), Some(dir.clone()));
        assert_eq!(find_project_root(&dir.join("pom.xml")), Some(dir.clone()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn snapshot_is_replaced_and_broadcast_on_refresh() {
        let dir = fixture();
        let model = ProjectModel::open(dir.clone());
        let seen = Arc::new(Mutex::new(Vec::<usize>::new()));
        let subscription = {
            let seen = seen.clone();
            model.subscribe(move |snapshot| {
                seen.lock().unwrap().push(snapshot.nodes.len());
            })
        };

        assert_eq!(model.snapshot().nodes.len(), 2);
        fs::write(dir.join("README.md"), "# demo").unwrap();
        model.refresh();
        assert_eq!(model.snapshot().nodes.len(), 3);
        assert_eq!(*seen.lock().unwrap(), vec![3]);

        drop(subscription);
        model.refresh();
        assert_eq!(*seen.lock().unwrap(), vec![3]);

        let _ = fs::remove_dir_all(&dir);
    }
}

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 默认不在项目树中展示的目录。
const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "target",
    "node_modules",
    ".gradle",
    ".idea",
    "dist",
];

/// 项目树中的一个节点。目录的子项按需加载，避免打开大仓库时阻塞。
#[derive(Clone, Debug)]
pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub children: Vec<TreeNode>,
    pub loaded: bool,
}

impl TreeNode {
    pub fn directory(path: PathBuf, name: String) -> Self {
        Self {
            path,
            name,
            is_dir: true,
            children: Vec::new(),
            loaded: false,
        }
    }

    fn file(path: PathBuf, name: String) -> Self {
        Self {
            path,
            name,
            is_dir: false,
            children: Vec::new(),
            loaded: true,
        }
    }

    pub fn find(&self, path: &Path) -> Option<&TreeNode> {
        if self.path == path {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(path))
    }

    pub fn find_mut(&mut self, path: &Path) -> Option<&mut TreeNode> {
        if self.path == path {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.find_mut(path))
    }
}

pub fn is_ignored_dir(name: &str) -> bool {
    IGNORED_DIRS.contains(&name)
}

/// 目录优先，其次按名称排序（忽略大小写，保留原始顺序作为兜底）。
pub fn sort_nodes(nodes: &mut [TreeNode]) {
    nodes.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// 读取目录并返回排好序的节点。该函数只在后台线程调用。
pub fn read_dir_sorted(path: &Path) -> io::Result<Vec<TreeNode>> {
    let mut nodes = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let is_dir = file_type.is_dir();
        if !is_dir && !file_type.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if is_dir && is_ignored_dir(&name) {
            continue;
        }
        let node = if is_dir {
            TreeNode::directory(entry.path(), name)
        } else {
            TreeNode::file(entry.path(), name)
        };
        nodes.push(node);
    }
    sort_nodes(&mut nodes);
    Ok(nodes)
}

/// 项目树中实际绘制的一行。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleRow {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
}

pub fn flatten_visible(root: &TreeNode, expanded: &HashSet<PathBuf>) -> Vec<VisibleRow> {
    let mut rows = Vec::new();
    push_rows(root, 0, expanded, &mut rows);
    rows
}

fn push_rows(
    node: &TreeNode,
    depth: usize,
    expanded: &HashSet<PathBuf>,
    rows: &mut Vec<VisibleRow>,
) {
    let node_expanded = node.is_dir && expanded.contains(&node.path);
    rows.push(VisibleRow {
        path: node.path.clone(),
        name: node.name.clone(),
        depth,
        is_dir: node.is_dir,
        expanded: node_expanded,
    });
    if node_expanded {
        for child in &node.children {
            push_rows(child, depth + 1, expanded, rows);
        }
    }
}

/// 需要在两次启动之间保留的界面状态。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersistedState {
    #[serde(default)]
    pub last_workspace: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub project_panel_visible: bool,
    #[serde(default = "default_true")]
    pub bottom_panel_visible: bool,
    #[serde(default)]
    pub bottom_tab: Option<String>,
    #[serde(default)]
    pub window_width: Option<f32>,
    #[serde(default)]
    pub window_height: Option<f32>,
}

fn default_true() -> bool {
    true
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            last_workspace: None,
            project_panel_visible: true,
            bottom_panel_visible: true,
            bottom_tab: None,
            window_width: None,
            window_height: None,
        }
    }
}

impl PersistedState {
    pub fn load() -> Self {
        let Some(path) = config_file() else {
            return Self::default();
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let Some(path) = config_file() else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "找不到配置目录"));
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        std::fs::write(path, text)
    }
}

fn config_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("state.json"))
}

fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("Keel"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|dir| PathBuf::from(dir).join("Library/Application Support/Keel"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|dir| PathBuf::from(dir).join(".config")))
            .map(|dir| dir.join("keel"))
    }
}

/// 根据扩展名返回编辑器识别的语言名称；无法判断时返回 `None`，编辑器按纯文本处理。
pub fn language_for_path(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    Some(match extension.as_str() {
        "rs" => "rust",
        "java" => "java",
        "toml" => "toml",
        "json" => "json",
        "md" | "markdown" => "markdown",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "jsx",
        "ts" | "mts" => "typescript",
        "tsx" => "tsx",
        "py" => "python",
        "go" => "go",
        "c" | "h" => "c",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" => "cpp",
        "html" | "htm" => "html",
        "css" => "css",
        "yaml" | "yml" => "yaml",
        "sh" | "bash" => "bash",
        _ => return None,
    })
}

/// 状态栏展示用的语言名称，对应 IDEA 状态栏右端的文件类型部件。
pub fn language_display_name(language: &str) -> &'static str {
    match language {
        "rust" => "Rust",
        "java" => "Java",
        "toml" => "TOML",
        "json" => "JSON",
        "markdown" => "Markdown",
        "javascript" => "JavaScript",
        "jsx" => "JSX",
        "typescript" => "TypeScript",
        "tsx" => "TSX",
        "python" => "Python",
        "go" => "Go",
        "c" => "C",
        "cpp" => "C++",
        "html" => "HTML",
        "css" => "CSS",
        "yaml" => "YAML",
        "bash" => "Shell Script",
        _ => "Plain Text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, is_dir: bool) -> TreeNode {
        TreeNode {
            path: PathBuf::from(name),
            name: name.to_string(),
            is_dir,
            children: Vec::new(),
            loaded: !is_dir,
        }
    }

    #[test]
    fn 目录排在文件前面并按名称忽略大小写排序() {
        let mut nodes = vec![
            node("zeta.rs", false),
            node("Alpha", true),
            node("beta.rs", false),
            node("gamma", true),
        ];

        sort_nodes(&mut nodes);

        let names: Vec<&str> = nodes.iter().map(|node| node.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "gamma", "beta.rs", "zeta.rs"]);
    }

    #[test]
    fn 常见构建目录会被忽略() {
        assert!(is_ignored_dir(".git"));
        assert!(is_ignored_dir("target"));
        assert!(is_ignored_dir("node_modules"));
        assert!(!is_ignored_dir("src"));
        assert!(!is_ignored_dir("Cargo.toml"));
    }

    #[test]
    fn 折叠的目录不会展开子节点() {
        let mut root = node("keel", true);
        let mut src = node("src", true);
        src.children.push(node("main.rs", false));
        root.children.push(src);

        let mut expanded = HashSet::new();
        expanded.insert(root.path.clone());
        let rows = flatten_visible(&root, &expanded);
        let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, vec!["keel", "src"]);

        expanded.insert(PathBuf::from("src"));
        let rows = flatten_visible(&root, &expanded);
        let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, vec!["keel", "src", "main.rs"]);
        assert_eq!(rows[2].depth, 2);
    }

    #[test]
    fn 持久化配置可以往返序列化() {
        let state = PersistedState {
            last_workspace: Some(PathBuf::from("D:/CodeWorkSpace/keel")),
            project_panel_visible: false,
            bottom_panel_visible: false,
            bottom_tab: Some("run".to_string()),
            window_width: Some(1280.0),
            window_height: Some(800.0),
        };

        let json = serde_json::to_string(&state).unwrap();
        let restored: PersistedState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.last_workspace, state.last_workspace);
        assert!(!restored.project_panel_visible);
        assert!(!restored.bottom_panel_visible);
        assert_eq!(restored.bottom_tab.as_deref(), Some("run"));
        assert_eq!(restored.window_width, Some(1280.0));
    }

    #[test]
    fn 缺失字段时使用默认值() {
        let restored: PersistedState = serde_json::from_str("{}").unwrap();
        assert!(restored.project_panel_visible);
        assert!(restored.bottom_panel_visible);
        assert!(restored.last_workspace.is_none());
    }
}

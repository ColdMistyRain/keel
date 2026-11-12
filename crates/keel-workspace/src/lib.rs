//! Keel 的工作区模型：项目目录扫描、树摊平、语言识别。
//!
//! 与 UI 框架无关：GPUI 版本和 nsh spike 都只拿到普通 Rust 结构，
//! 各自决定怎么画树、怎么持有状态（信号 / 实体）。
//! 对应迁移方案 P1 里的 `keel-workspace`。

use std::fs;
use std::path::{Path, PathBuf};

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

/// 摊平后的一行，给列表直接渲染。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatRow {
    pub id: String,
    pub label: String,
    pub depth: u32,
    pub folder: bool,
    pub open: bool,
}

/// 扫描 `root` 的直接子项：目录在前、文件在后，按名称排序并跳过构建产物。
pub fn scan_project(root: &Path) -> Vec<ProjectNode> {
    scan_dir(root, 0)
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

/// 把节点按展开状态摊平成可见行。
pub fn flatten(nodes: &[ProjectNode], expanded: &[String], depth: u32) -> Vec<FlatRow> {
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
            rows.extend(flatten(&node.children, expanded, depth + 1));
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

        let flattened = flatten(&nodes, &[nodes[0].path.display().to_string()], 0);
        let labels: Vec<_> = flattened.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(labels, vec!["src", "main", "pom.xml"]);
        assert!(flattened[0].open);
        assert!(!flattened[1].open);

        let _ = fs::remove_dir_all(&dir);
    }
}

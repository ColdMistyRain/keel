use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 工作区识别出的构建系统类型。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectKind {
    #[default]
    Plain,
    Maven,
    Gradle,
}

/// 从构建文件中提取出的项目信息，仅用于界面展示与后续 LSP 定位，不做依赖解析。
/// Maven/Gradle 接入暂缓，坐标、模块与源码根目前只解析不使用。
#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct ProjectModel {
    pub kind: ProjectKind,
    pub name: String,
    pub coordinates: Option<String>,
    pub modules: Vec<String>,
    pub source_roots: Vec<PathBuf>,
}

/// Maven `pom.xml` 中的关键字段。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MavenInfo {
    pub group_id: Option<String>,
    pub artifact_id: Option<String>,
    pub version: Option<String>,
    pub modules: Vec<String>,
    pub source_directory: Option<String>,
}

/// 识别工作区使用的构建系统。只读取根目录的构建文件，不递归。
pub fn detect(root: &Path) -> ProjectModel {
    if root.join("pom.xml").is_file() {
        return detect_maven(root);
    }
    if has_gradle_files(root) {
        return detect_gradle(root);
    }
    ProjectModel {
        kind: ProjectKind::Plain,
        name: root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
        ..Default::default()
    }
}

fn detect_maven(root: &Path) -> ProjectModel {
    let info = std::fs::read_to_string(root.join("pom.xml"))
        .ok()
        .and_then(|text| parse_pom(&text))
        .unwrap_or_default();

    let name = info
        .artifact_id
        .clone()
        .unwrap_or_else(|| directory_name(root));

    let coordinates = match (&info.group_id, &info.artifact_id, &info.version) {
        (Some(group), Some(artifact), Some(version)) => {
            Some(format!("{group}:{artifact}:{version}"))
        }
        (Some(group), Some(artifact), None) => Some(format!("{group}:{artifact}")),
        (None, Some(artifact), Some(version)) => Some(format!("{artifact}:{version}")),
        _ => None,
    };

    let mut source_roots = Vec::new();
    if let Some(custom) = &info.source_directory {
        source_roots.push(PathBuf::from(custom));
    } else if root.join("src/main/java").is_dir() {
        source_roots.push(PathBuf::from("src/main/java"));
    }
    if root.join("src/test/java").is_dir() {
        source_roots.push(PathBuf::from("src/test/java"));
    }

    ProjectModel {
        kind: ProjectKind::Maven,
        name,
        coordinates,
        modules: info.modules,
        source_roots,
    }
}

fn detect_gradle(root: &Path) -> ProjectModel {
    let settings = ["settings.gradle", "settings.gradle.kts"]
        .iter()
        .find_map(|name| std::fs::read_to_string(root.join(name)).ok())
        .unwrap_or_default();

    let name = parse_gradle_root_name(&settings).unwrap_or_else(|| directory_name(root));

    let mut source_roots = Vec::new();
    if root.join("src/main/java").is_dir() {
        source_roots.push(PathBuf::from("src/main/java"));
    }
    if root.join("src/test/java").is_dir() {
        source_roots.push(PathBuf::from("src/test/java"));
    }

    ProjectModel {
        kind: ProjectKind::Gradle,
        name,
        coordinates: None,
        modules: parse_gradle_includes(&settings),
        source_roots,
    }
}

fn has_gradle_files(root: &Path) -> bool {
    [
        "build.gradle",
        "build.gradle.kts",
        "settings.gradle",
        "settings.gradle.kts",
    ]
    .iter()
    .any(|name| root.join(name).is_file())
}

fn directory_name(root: &Path) -> String {
    root.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// 解析 `pom.xml`。无法解析时返回 `None`，调用方按普通目录处理。
pub fn parse_pom(xml: &str) -> Option<MavenInfo> {
    let document = roxmltree::Document::parse(xml).ok()?;
    let root = document.root_element();
    if root.tag_name().name() != "project" {
        return None;
    }

    let text_of = |parent: roxmltree::Node<'_, '_>, name: &str| {
        parent
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == name)
            .and_then(|node| node.text())
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
    };

    let modules = root
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "modules")
        .map(|modules| {
            modules
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "module")
                .filter_map(|node| node.text())
                .map(|text| text.trim().to_string())
                .filter(|text| !text.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let source_directory = root
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "build")
        .and_then(|build| text_of(build, "sourceDirectory"));

    Some(MavenInfo {
        group_id: text_of(root, "groupId"),
        artifact_id: text_of(root, "artifactId"),
        version: text_of(root, "version"),
        modules,
        source_directory,
    })
}

/// 解析 `settings.gradle(.kts)` 中的 `include` 语句。
pub fn parse_gradle_includes(text: &str) -> Vec<String> {
    let mut modules = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("//") {
            continue;
        }
        let Some(rest) = line.strip_prefix("include") else {
            continue;
        };
        if !(rest.starts_with('(') || rest.starts_with(' ') || rest.starts_with('\t')) {
            continue;
        }
        modules.extend(quoted_strings(rest));
    }
    modules
}

/// 解析 `rootProject.name = "demo"`。
pub fn parse_gradle_root_name(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("//") || !line.starts_with("rootProject.name") {
            continue;
        }
        let Some((_, value)) = line.split_once('=') else {
            continue;
        };
        if let Some(name) = quoted_strings(value).into_iter().next() {
            return Some(name);
        }
    }
    None
}

fn quoted_strings(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(current) = chars.next() {
        if current != '"' && current != '\'' {
            continue;
        }
        let quote = current;
        let mut buffer = String::new();
        for next in chars.by_ref() {
            if next == quote {
                break;
            }
            buffer.push(next);
        }
        let value = buffer.trim().to_string();
        if !value.is_empty() {
            values.push(value);
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    const POM: &str = r#"
<project xmlns="http://maven.apache.org/POM/4.0.0">
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.example</groupId>
  <artifactId>demo</artifactId>
  <version>1.0.0</version>
  <modules>
    <module>core</module>
    <module>app</module>
  </modules>
  <build>
    <sourceDirectory>src/custom/java</sourceDirectory>
  </build>
</project>
"#;

    #[test]
    fn 解析_pom_关键字段与模块() {
        let info = parse_pom(POM).expect("应能解析 pom");
        assert_eq!(info.group_id.as_deref(), Some("com.example"));
        assert_eq!(info.artifact_id.as_deref(), Some("demo"));
        assert_eq!(info.version.as_deref(), Some("1.0.0"));
        assert_eq!(info.modules, vec!["core", "app"]);
        assert_eq!(info.source_directory.as_deref(), Some("src/custom/java"));
    }

    #[test]
    fn 缺失字段时返回空值而不是报错() {
        let info = parse_pom("<project><artifactId>only</artifactId></project>").unwrap();
        assert_eq!(info.artifact_id.as_deref(), Some("only"));
        assert!(info.group_id.is_none());
        assert!(info.modules.is_empty());
    }

    #[test]
    fn 非_project_根节点视为无效() {
        assert!(parse_pom("<settings><artifactId>x</artifactId></settings>").is_none());
        assert!(parse_pom("not xml").is_none());
    }

    #[test]
    fn 解析_gradle_groovy_与_kts_的_include() {
        let groovy = "include ':core', ':app'\n// include ':ignored'\ninclude ':feature:api'";
        assert_eq!(
            parse_gradle_includes(groovy),
            vec![":core", ":app", ":feature:api"]
        );

        let kts = "include(\"core\")\ninclude(\"app\", \"feature\")";
        assert_eq!(parse_gradle_includes(kts), vec!["core", "app", "feature"]);
    }

    #[test]
    fn 解析_gradle_根项目名() {
        assert_eq!(
            parse_gradle_root_name("rootProject.name = \"shop\"\n").as_deref(),
            Some("shop")
        );
        assert_eq!(
            parse_gradle_root_name("rootProject.name='x'").as_deref(),
            Some("x")
        );
        assert!(parse_gradle_root_name("// rootProject.name = \"skip\"").is_none());
    }
}

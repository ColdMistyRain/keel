//! 行注释切换（Ctrl+/），对应 IDEA 的 `CommentByLineCommentAction`。

use std::ops::Range;
use std::path::Path;

use gpui::{prelude::*, *};

use crate::actions::ToggleComment;
use crate::app::KeelApp;

impl KeelApp {
    pub fn on_toggle_comment(
        &mut self,
        _: &ToggleComment,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_comment(window, cx);
    }

    /// 给选区覆盖的行加上或去掉行注释前缀。
    ///
    /// 选区按 IDEA 的规则扩展到完整行：起点所在的整行到终点所在的整行；
    /// 选区恰好停在行首时不把下一行算进来。缩进保留，`//` 插在缩进之后。
    pub fn toggle_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.active.and_then(|ix| self.documents.get(ix)) else {
            return;
        };
        let Some(prefix) = line_comment_prefix(&doc.path) else {
            return;
        };
        let state = doc.state.clone();
        let (selection, text) = {
            let state = state.read(cx);
            (state.selected_range(), state.text().to_string())
        };

        let block = line_block(&text, selection);
        let replacement = toggle_lines(&text[block.clone()], prefix);
        if replacement == text[block.clone()] {
            return;
        }

        state.update(cx, |state, cx| {
            state.set_selected_range(block, cx);
            state.replace(replacement, window, cx);
        });
    }
}

/// 各类文件的行注释前缀；不支持行注释的类型返回 `None`。
fn line_comment_prefix(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    Some(match extension.as_str() {
        "rs" | "java" | "kt" | "kts" | "scala" | "swift" | "proto" | "js" | "mjs" | "cjs"
        | "jsx" | "ts" | "mts" | "tsx" | "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "go" => {
            "//"
        }
        "py" | "sh" | "bash" | "yaml" | "yml" | "toml" | "rb" => "#",
        _ => return None,
    })
}

/// 把字节选区扩展到完整行（不含结尾换行）。
fn line_block(text: &str, range: Range<usize>) -> Range<usize> {
    let start = text[..range.start].rfind('\n').map_or(0, |ix| ix + 1);
    // 选区停在行首时（前一个字符是换行），最后一行不应算进来。
    let probe = if range.end > range.start && text.as_bytes().get(range.end - 1) == Some(&b'\n') {
        range.end - 1
    } else {
        range.end
    };
    let end = text[probe..].find('\n').map_or(text.len(), |ix| probe + ix);
    start..end
}

/// 返回整块加/去注释后的文本。块内每一行都已有前缀时视为「去注释」。
fn toggle_lines(block: &str, prefix: &str) -> String {
    let lines: Vec<&str> = block.split('\n').collect();
    let mut code_lines = 0;
    let mut commented_lines = 0;
    for line in &lines {
        let rest = line.trim_start();
        if rest.is_empty() {
            continue;
        }
        code_lines += 1;
        if rest.starts_with(prefix) {
            commented_lines += 1;
        }
    }
    let uncomment = code_lines > 0 && commented_lines == code_lines;

    let mut out = String::with_capacity(block.len() + lines.len() * (prefix.len() + 1));
    for (ix, line) in lines.iter().enumerate() {
        if ix > 0 {
            out.push('\n');
        }
        let indent_len = line.len() - line.trim_start().len();
        let (indent, rest) = line.split_at(indent_len);
        if rest.is_empty() {
            out.push_str(line);
        } else if uncomment {
            out.push_str(indent);
            let rest = rest.strip_prefix(prefix).unwrap_or(rest);
            out.push_str(rest.strip_prefix(' ').unwrap_or(rest));
        } else {
            out.push_str(indent);
            out.push_str(prefix);
            out.push(' ');
            out.push_str(rest);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    // 不用 `use super::*`：那会把 gpui 的 `test` 宏带进作用域，覆盖内置的 `#[test]`。
    use super::{line_block, line_comment_prefix, toggle_lines};
    use std::path::PathBuf;

    fn prefix_for(name: &str) -> Option<&'static str> {
        line_comment_prefix(&PathBuf::from(name))
    }

    #[test]
    fn 注释前缀按扩展名区分() {
        assert_eq!(prefix_for("Main.java"), Some("//"));
        assert_eq!(prefix_for("main.rs"), Some("//"));
        assert_eq!(prefix_for("script.py"), Some("#"));
        assert_eq!(prefix_for("data.json"), None);
        assert_eq!(prefix_for("page.html"), None);
    }

    #[test]
    fn 整行选区加注释保留缩进() {
        let block = "fn main() {\n    run();\n}";
        assert_eq!(
            toggle_lines(block, "//"),
            "// fn main() {\n    // run();\n// }"
        );
    }

    #[test]
    fn 已注释的整块再次切换会去掉前缀() {
        let block = "// fn main() {\n//     run();\n// }";
        assert_eq!(toggle_lines(block, "//"), "fn main() {\n    run();\n}");
    }

    #[test]
    fn 部分注释的块按加注释处理() {
        let block = "// first\nsecond";
        assert_eq!(toggle_lines(block, "//"), "// // first\n// second");
    }

    #[test]
    fn 选区停在行首时不包含下一行() {
        let text = "a\nb\nc";
        // 选中 "a\n"：只覆盖第 1 行。
        let first = line_block(text, 0..2);
        assert_eq!(first.start, 0);
        assert_eq!(first.end, 1);
        // 光标在行首：只覆盖当前行。
        let caret = line_block(text, 2..2);
        assert_eq!(caret.start, 2);
        assert_eq!(caret.end, 3);
    }
}

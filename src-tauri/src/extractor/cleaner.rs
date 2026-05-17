use super::pagination::extracted_from_lines;
use super::types::{ExtractedCode, ExtractionConfig};
use encoding_rs_io::DecodeReaderBytesBuilder;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Clone, Copy)]
enum BlockComment {
    SlashStar,
    Html,
    PythonTripleDouble,
    PythonTripleSingle,
    PowerShell,
}

impl BlockComment {
    fn end_marker(self) -> &'static str {
        match self {
            BlockComment::SlashStar => "*/",
            BlockComment::Html => "-->",
            BlockComment::PythonTripleDouble => "\"\"\"",
            BlockComment::PythonTripleSingle => "'''",
            BlockComment::PowerShell => "#>",
        }
    }
}

#[derive(Clone, Copy)]
struct CommentSyntax {
    slash_line: bool,
    slash_block: bool,
    hash_line: bool,
    sql_line: bool,
    html_block: bool,
    python_triple: bool,
    powershell_block: bool,
    backtick_string: bool,
}

impl CommentSyntax {
    fn for_extension(ext: &str) -> Self {
        let ext = ext.trim_start_matches('.').to_ascii_lowercase();
        let ext = ext.as_str();
        let c_like = matches!(
            ext,
            "c" | "cpp"
                | "cc"
                | "h"
                | "hpp"
                | "cs"
                | "java"
                | "js"
                | "jsx"
                | "ts"
                | "tsx"
                | "rs"
                | "go"
                | "php"
                | "swift"
                | "m"
                | "mm"
                | "kt"
                | "scala"
                | "dart"
                | "fs"
                | "vb"
        );
        let css_like = matches!(ext, "css" | "scss" | "less");

        Self {
            slash_line: c_like || matches!(ext, "scss" | "less" | "vue"),
            slash_block: c_like || css_like || matches!(ext, "sql" | "vue"),
            hash_line: matches!(ext, "py" | "sh" | "rb" | "ps1"),
            sql_line: ext == "sql",
            html_block: matches!(ext, "html" | "htm" | "xml" | "vue" | "svelte"),
            python_triple: ext == "py",
            powershell_block: ext == "ps1",
            backtick_string: matches!(ext, "js" | "jsx" | "ts" | "tsx" | "vue"),
        }
    }
}

pub fn extract_and_clean(
    path_str: &str,
    config: &ExtractionConfig,
) -> anyhow::Result<ExtractedCode> {
    let path = Path::new(path_str);
    let file = File::open(path)?;

    let mut reader = DecodeReaderBytesBuilder::new().encoding(None).build(file);

    let mut raw_content = String::new();
    reader.read_to_string(&mut raw_content)?;

    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    Ok(clean_source_content(&raw_content, &ext, config))
}

fn clean_source_content(raw_content: &str, ext: &str, config: &ExtractionConfig) -> ExtractedCode {
    let syntax = CommentSyntax::for_extension(ext);
    let mut block_state = None;
    let mut lines = Vec::new();

    for line in raw_content.lines() {
        let processed_line = if config.remove_comments {
            strip_comments_from_line(line, syntax, &mut block_state)
                .trim_end()
                .to_string()
        } else {
            line.to_string()
        };
        let trimmed = processed_line.trim();

        if config.compact_lines && trimmed.is_empty() {
            continue;
        }

        lines.push(processed_line);
    }

    extracted_from_lines(lines)
}

fn strip_comments_from_line(
    line: &str,
    syntax: CommentSyntax,
    block_state: &mut Option<BlockComment>,
) -> String {
    let mut output = String::with_capacity(line.len());
    let mut i = 0;
    let mut in_double = false;
    let mut in_single = false;
    let mut in_backtick = false;
    let mut escaped = false;

    while i < line.len() {
        if let Some(state) = *block_state {
            if starts_with_at(line, i, state.end_marker()) {
                i += state.end_marker().len();
                *block_state = None;
            } else {
                i += next_char_len(line, i);
            }
            continue;
        }

        let ch = next_char(line, i);

        if in_double {
            output.push(ch);
            i += ch.len_utf8();
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }

        if in_single {
            output.push(ch);
            i += ch.len_utf8();
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        if in_backtick {
            output.push(ch);
            i += ch.len_utf8();
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '`' {
                in_backtick = false;
            }
            continue;
        }

        if syntax.python_triple && output.trim().is_empty() && starts_with_at(line, i, "\"\"\"") {
            i = skip_block_or_set(
                line,
                i,
                "\"\"\"",
                BlockComment::PythonTripleDouble,
                block_state,
            );
            continue;
        }

        if syntax.python_triple && output.trim().is_empty() && starts_with_at(line, i, "'''") {
            i = skip_block_or_set(
                line,
                i,
                "'''",
                BlockComment::PythonTripleSingle,
                block_state,
            );
            continue;
        }

        if syntax.html_block && starts_with_at(line, i, "<!--") {
            i = skip_block_or_set(line, i, "<!--", BlockComment::Html, block_state);
            continue;
        }

        if syntax.powershell_block && starts_with_at(line, i, "<#") {
            i = skip_block_or_set(line, i, "<#", BlockComment::PowerShell, block_state);
            continue;
        }

        if syntax.slash_block && starts_with_at(line, i, "/*") {
            i = skip_block_or_set(line, i, "/*", BlockComment::SlashStar, block_state);
            continue;
        }

        if syntax.slash_line && starts_with_at(line, i, "//") {
            break;
        }

        if syntax.sql_line && starts_with_at(line, i, "--") {
            break;
        }

        if syntax.hash_line && starts_with_at(line, i, "#") {
            break;
        }

        if ch == '"' {
            in_double = true;
            escaped = false;
        } else if ch == '\'' && has_closing_quote(line, i + ch.len_utf8(), '\'') {
            in_single = true;
            escaped = false;
        } else if syntax.backtick_string && ch == '`' {
            in_backtick = true;
            escaped = false;
        }

        output.push(ch);
        i += ch.len_utf8();
    }

    output
}

fn skip_block_or_set(
    line: &str,
    start: usize,
    start_marker: &str,
    block: BlockComment,
    block_state: &mut Option<BlockComment>,
) -> usize {
    let content_start = start + start_marker.len();
    if let Some(end) = line[content_start..].find(block.end_marker()) {
        content_start + end + block.end_marker().len()
    } else {
        *block_state = Some(block);
        line.len()
    }
}

fn starts_with_at(line: &str, index: usize, marker: &str) -> bool {
    line.get(index..)
        .map(|rest| rest.starts_with(marker))
        .unwrap_or(false)
}

fn next_char(line: &str, index: usize) -> char {
    line[index..].chars().next().unwrap()
}

fn next_char_len(line: &str, index: usize) -> usize {
    next_char(line, index).len_utf8()
}

fn has_closing_quote(line: &str, start: usize, quote: char) -> bool {
    let mut escaped = false;
    for ch in line[start..].chars() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == quote {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean_config() -> ExtractionConfig {
        ExtractionConfig {
            remove_comments: true,
            compact_lines: true,
        }
    }

    #[test]
    fn removes_c_like_comments_without_touching_strings() {
        let raw = concat!(
            "// top comment\n",
            "const url = \"https://example.com/a//b\";\n",
            "const single = '/* not a comment */';\n",
            "const tpl = `// not a comment`;\n",
            "let x = 1; // trailing comment\n",
            "let y = 2; /* block comment */ let z = 3;\n",
            "/*\n",
            " * doc block\n",
            " */\n",
            "let life: &'a str = name; // rust lifetime style\n",
        );

        let result = clean_source_content(raw, "ts", &clean_config());

        assert_eq!(
            result.content,
            concat!(
                "const url = \"https://example.com/a//b\";\n",
                "const single = '/* not a comment */';\n",
                "const tpl = `// not a comment`;\n",
                "let x = 1;\n",
                "let y = 2;  let z = 3;\n",
                "let life: &'a str = name;"
            )
        );
        assert_eq!(result.line_count, 6);
    }

    #[test]
    fn removes_python_and_html_comments_without_touching_string_markers() {
        let python = concat!(
            "def build_url():\n",
            "    \"\"\"doc comment\n",
            "    still comment\n",
            "    \"\"\"\n",
            "    url = 'https://example.com/#anchor'\n",
            "    value = 1 # trailing comment\n",
            "# whole line comment\n",
            "    return url, value\n",
        );

        let py_result = clean_source_content(python, "py", &clean_config());
        assert_eq!(
            py_result.content,
            concat!(
                "def build_url():\n",
                "    url = 'https://example.com/#anchor'\n",
                "    value = 1\n",
                "    return url, value"
            )
        );

        let html = concat!(
            "<div><!-- remove -->ok</div>\n",
            "<!-- full line -->\n",
            "<span>keep</span>\n",
        );
        let html_result = clean_source_content(html, "html", &clean_config());
        assert_eq!(html_result.content, "<div>ok</div>\n<span>keep</span>");
    }

    #[test]
    fn keeps_dependency_declarations_while_cleaning_comments() {
        let cases = [
            ("c", "#include <stdio.h>\nint main() { return 0; }\n"),
            ("h", "# include \"core.h\"\nvoid run(void);\n"),
            (
                "ts",
                "import React from \"react\";\nconst x = require(\"x\");\n",
            ),
            (
                "py",
                "from pathlib import Path\nimport os\nprint(os.name)\n",
            ),
            ("cs", "using System;\nclass Program {}\n"),
            (
                "php",
                "include \"bootstrap.php\";\nrequire_once \"lib.php\";\n",
            ),
        ];

        for (ext, raw) in cases {
            let result = clean_source_content(raw, ext, &clean_config());
            for line in raw.lines() {
                assert!(
                    result.content.contains(line),
                    "expected cleaned {ext} output to contain {line:?}, got {:?}",
                    result.content
                );
            }
        }
    }
}

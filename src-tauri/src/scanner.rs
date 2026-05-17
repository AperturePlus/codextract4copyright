use ignore::WalkBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

pub const DEFAULT_EXTENSIONS: &[&str] = &[
    "c", "cpp", "cc", "h", "hpp", "cs", "java", "js", "ts", "py", "rs", "go", "php", "rb", "swift",
    "m", "mm", "kt", "scala", "sql", "sh", "bat", "ps1", "html", "css", "vue", "tsx", "jsx",
    "dart", "fs", "vb", "asm", "s", "less", "scss",
];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileInfo {
    pub id: usize,
    pub absolute_path: String,
    pub relative_path: String,
    pub lines: usize,
    pub extension: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScanResult {
    pub total_files: usize,
    pub total_original_lines: usize,
    pub language_counts: HashMap<String, usize>,
    pub gitignore_rules: Vec<String>,
    pub files: Vec<FileInfo>,
}

#[tauri::command]
pub async fn scan_project(
    root: String,
    custom_excludes: Vec<String>,
    extensions: Vec<String>,
) -> Result<ScanResult, String> {
    scan_project_sync(&root, &custom_excludes, &extensions)
}

pub fn scan_project_sync(
    root: &str,
    custom_excludes: &[String],
    extensions: &[String],
) -> Result<ScanResult, String> {
    let root_path = Path::new(&root);
    if !root_path.exists() {
        return Err("Root path does not exist".to_string());
    }

    let gitignore_rules = read_gitignore_rules(root_path);

    // 预编译全部正则规则，凡是有语法错误的给出警告并跳过
    let compiled_regexes: Vec<Regex> = custom_excludes
        .iter()
        .filter_map(|r| Regex::new(&r).ok())
        .collect();

    // 构建后缀白名单哈希集，如果前端没传则使用默认集
    let ext_whitelist: Vec<String> = if extensions.is_empty() {
        DEFAULT_EXTENSIONS.iter().map(|&s| s.to_string()).collect()
    } else {
        extensions
            .iter()
            .map(|e| e.to_lowercase().replace(".", ""))
            .collect()
    };

    let mut language_counts = HashMap::new();
    let mut files = Vec::new();
    let mut total_files = 0;
    let mut total_original_lines = 0;
    let mut next_id = 0;

    let mut walk_builder = WalkBuilder::new(root_path);
    walk_builder.standard_filters(true); // Respects .gitignore
    walk_builder.require_git(false); // Apply .gitignore even for folders that are not git repos.
    walk_builder.hidden(true); // Ignores hidden files

    let walker = walk_builder.build();

    for entry in walker.flatten() {
        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            let path = entry.path();

            // 1. Check extension whitelist
            let ext = match path.extension().and_then(|e| e.to_str()) {
                Some(e) => e.to_lowercase(),
                None => continue,
            };

            if !ext_whitelist.contains(&ext) {
                continue;
            }

            // 2. Custom exclude rules (True Regex matched against relative path)
            let relative_path = path
                .strip_prefix(root_path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string()
                .replace("\\", "/");

            let mut is_excluded = false;
            for re in &compiled_regexes {
                // 如果正则匹配中了绝对路径或相对路径，则直接杀掉
                if re.is_match(&relative_path) || re.is_match(&path.to_string_lossy()) {
                    is_excluded = true;
                    break;
                }
            }
            if is_excluded {
                continue;
            }

            // 3. Original lines counting
            let mut lines = 0;
            if let Ok(file) = File::open(path) {
                let reader = BufReader::new(file);
                lines = reader.lines().count();
                total_original_lines += lines;
            }

            // 4. Record
            let count = language_counts.entry(ext.clone()).or_insert(0);
            *count += 1;

            files.push(FileInfo {
                id: next_id,
                absolute_path: path.to_string_lossy().to_string(),
                relative_path,
                lines,
                extension: ext,
            });

            next_id += 1;
            total_files += 1;
        }
    }

    Ok(ScanResult {
        total_files,
        total_original_lines,
        language_counts,
        gitignore_rules,
        files,
    })
}

fn read_gitignore_rules(root_path: &Path) -> Vec<String> {
    let gitignore_path = root_path.join(".gitignore");
    let Ok(content) = fs::read_to_string(gitignore_path) else {
        return Vec::new();
    };

    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "codextract_scan_test_{}_{}",
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn scan_project_applies_and_reports_gitignore_rules() {
        let root = unique_temp_dir();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::create_dir_all(root.join("dist")).unwrap();

        fs::write(
            root.join(".gitignore"),
            "node_modules/\ndist/\n*.log\n# ignored comment\n\n",
        )
        .unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(root.join("node_modules/pkg/lib.js"), "console.log(1);\n").unwrap();
        fs::write(root.join("dist/bundle.js"), "console.log(2);\n").unwrap();
        fs::write(root.join("debug.log"), "log line\n").unwrap();

        let custom_excludes = Vec::new();
        let extensions = vec!["rs".to_string(), "js".to_string(), "log".to_string()];
        let result = scan_project_sync(root.to_str().unwrap(), &custom_excludes, &extensions)
            .expect("scan should succeed");

        assert_eq!(
            result.gitignore_rules,
            vec![
                "node_modules/".to_string(),
                "dist/".to_string(),
                "*.log".to_string()
            ]
        );
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].relative_path, "src/main.rs");

        fs::remove_dir_all(root).ok();
    }
}

mod cleaner;
mod docx_export;
mod layout;
mod pagination;
mod types;

use cleaner::extract_and_clean;
use pagination::{extracted_from_lines, select_submission_lines};

pub use types::{ExportConfig, ExtractedCode, ExtractionConfig};

#[tauri::command]
pub async fn execute_extraction(
    files: Vec<String>,
    config: ExtractionConfig,
) -> Result<ExtractedCode, String> {
    execute_extraction_sync(files, config)
}

pub fn execute_extraction_sync(
    files: Vec<String>,
    config: ExtractionConfig,
) -> Result<ExtractedCode, String> {
    let mut all_lines = Vec::new();
    let mut errors = Vec::new();

    for file_path in files {
        match extract_and_clean(&file_path, &config) {
            Ok(ext_res) => {
                all_lines.extend(ext_res.content.lines().map(ToOwned::to_owned));
            }
            Err(e) => {
                errors.push(format!("{}: {}", file_path, e));
            }
        }
    }

    if !errors.is_empty() {
        return Err(format!(
            "以下文件提取失败，未生成结果:\n{}",
            errors.join("\n")
        ));
    }

    Ok(extracted_from_lines(select_submission_lines(all_lines)))
}

#[tauri::command]
pub async fn export_to_docx(content: String, config: ExportConfig) -> Result<String, String> {
    docx_export::export_content_to_docx(content, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn clean_config() -> ExtractionConfig {
        ExtractionConfig {
            remove_comments: true,
            compact_lines: true,
        }
    }

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "codextract_extract_test_{}_{}",
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn execute_extraction_outputs_only_source_lines() {
        let root = unique_temp_dir();
        fs::create_dir_all(&root).unwrap();
        let file = root.join("main.rs");
        fs::write(&file, "fn main() {}\n// trailing file comment\n").unwrap();

        let result =
            execute_extraction_sync(vec![file.to_string_lossy().to_string()], clean_config())
                .expect("extraction should succeed");

        assert_eq!(result.content, "fn main() {}");
        assert_eq!(result.line_count, 1);
        assert!(!result.content.contains("File:"));

        fs::remove_dir_all(root).ok();
    }
}

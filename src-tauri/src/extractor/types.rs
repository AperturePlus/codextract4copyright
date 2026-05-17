use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExtractionConfig {
    pub remove_comments: bool,
    pub compact_lines: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExtractedCode {
    pub content: String,
    pub line_count: usize,
    pub lines_per_page: usize,
    pub page_count: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportConfig {
    pub software_name: String,
    pub software_version: String,
    pub save_path: String,
}

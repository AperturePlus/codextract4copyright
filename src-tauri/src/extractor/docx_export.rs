use super::layout::{
    code_line_spacing, code_lines_per_page, code_page_margin, code_run_fonts,
    CODE_FONT_SIZE_HALF_POINTS, PAGE_HEIGHT_TWIPS, PAGE_WIDTH_TWIPS,
};
use super::pagination::submission_lines_from_content;
use super::types::ExportConfig;
use docx_rs::*;
use std::fs::File;
use std::path::Path;

pub fn export_content_to_docx(content: String, config: ExportConfig) -> Result<String, String> {
    let path = Path::new(&config.save_path);
    let file = File::create(path).map_err(|e| format!("无法创建 Word 文件: {}", e))?;
    let lines = submission_lines_from_content(&content);

    if lines.is_empty() {
        return Err("没有可导出的源码内容".to_string());
    }

    let header_text = format_document_title(&config.software_name, &config.software_version);

    let header = Header::new().add_paragraph(
        Paragraph::new()
            .add_run(Run::new().add_text(header_text).size(20))
            .align(AlignmentType::Center),
    );

    let footer = Footer::new().add_paragraph(
        Paragraph::new()
            .add_page_num(PageNum::new())
            .align(AlignmentType::Center),
    );

    let mut doc = Docx::new()
        .header(header)
        .footer(footer)
        .page_size(PAGE_WIDTH_TWIPS, PAGE_HEIGHT_TWIPS)
        .page_margin(code_page_margin())
        .default_size(CODE_FONT_SIZE_HALF_POINTS)
        .default_fonts(code_run_fonts())
        .default_line_spacing(code_line_spacing());
    let lines_per_page = code_lines_per_page();
    let page_count = lines.len().div_ceil(lines_per_page);

    for (page_index, page_lines) in lines.chunks(lines_per_page).enumerate() {
        let paragraph = build_code_paragraph(page_lines, page_index + 1 < page_count);
        doc = doc.add_paragraph(paragraph);
    }

    doc.build()
        .pack(file)
        .map_err(|e| format!("封装 Docx 失败: {}", e))?;

    Ok(format!("导出成功，文件已保存至: {}", config.save_path))
}

fn format_document_title(software_name: &str, software_version: &str) -> String {
    let name = software_name.trim();
    if name.is_empty() {
        return String::from("源代码提取报告");
    }

    let version = software_version.trim();
    let version = if version.is_empty() { "V1.0" } else { version };
    format!("《{}》{}源代码", name, version)
}

fn build_code_paragraph(page_lines: &[String], add_page_break: bool) -> Paragraph {
    let mut paragraph = Paragraph::new().line_spacing(code_line_spacing());

    for (line_index, line) in page_lines.iter().enumerate() {
        if line_index > 0 {
            paragraph = paragraph.add_run(Run::new().add_break(BreakType::TextWrapping));
        }

        paragraph = paragraph.add_run(
            Run::new()
                .add_text(line.to_string())
                .fonts(code_run_fonts())
                .size(CODE_FONT_SIZE_HALF_POINTS),
        );
    }

    if add_page_break {
        paragraph = paragraph.add_run(Run::new().add_break(BreakType::Page));
    }

    paragraph
}

#[cfg(test)]
mod tests {
    use super::*;
    use docx_rs::BuildXML;

    #[test]
    fn formats_source_document_title_from_metadata() {
        assert_eq!(
            format_document_title("测试系统", "V1.0"),
            "《测试系统》V1.0源代码"
        );
        assert_eq!(
            format_document_title("  测试系统  ", ""),
            "《测试系统》V1.0源代码"
        );
        assert_eq!(format_document_title("", "V2.0"), "源代码提取报告");
    }

    #[test]
    fn code_paragraph_uses_exact_line_spacing_and_no_trailing_blank_break() {
        let lines = vec!["line1".to_string(), "line2".to_string()];
        let paragraph = build_code_paragraph(&lines, true);
        let xml = String::from_utf8(paragraph.build()).unwrap();

        assert!(xml.contains(r#"w:rFonts"#));
        assert!(xml.contains(r#"w:ascii="Times New Roman""#));
        assert!(xml.contains(r#"w:hAnsi="Times New Roman""#));
        assert!(xml.contains(r#"w:eastAsia="Times New Roman""#));
        assert!(xml.contains(r#"w:cs="Times New Roman""#));
        assert!(xml.contains(r#"<w:sz w:val="18" />"#));
        assert!(xml.contains(r#"w:before="0""#));
        assert!(xml.contains(r#"w:after="0""#));
        assert!(xml.contains(r#"w:line="200""#));
        assert!(xml.contains(r#"w:lineRule="exact""#));
        assert_eq!(xml.matches(r#"<w:br w:type="textWrapping" />"#).count(), 1);
        assert_eq!(xml.matches(r#"<w:br w:type="page" />"#).count(), 1);
        assert!(xml.contains(">line1</w:t>"));
        assert!(xml.contains(">line2</w:t>"));
    }

    #[test]
    fn page_margin_matches_layout_constants() {
        let xml = String::from_utf8(code_page_margin().build()).unwrap();

        assert!(xml.contains(r#"w:top="1985""#));
        assert!(xml.contains(r#"w:right="1701""#));
        assert!(xml.contains(r#"w:bottom="1701""#));
        assert!(xml.contains(r#"w:left="1701""#));
    }
}

use super::layout::{
    code_lines_per_page, copyright_side_lines, max_submission_lines, page_count_for_line_count,
};
use super::types::ExtractedCode;

pub fn extracted_from_lines(lines: Vec<String>) -> ExtractedCode {
    let lines = non_empty_lines(lines);
    let line_count = lines.len();
    let lines_per_page = code_lines_per_page();
    ExtractedCode {
        content: lines.join("\n"),
        line_count,
        lines_per_page,
        page_count: page_count_for_line_count(line_count),
    }
}

pub fn select_submission_lines(lines: Vec<String>) -> Vec<String> {
    let lines = non_empty_lines(lines);
    let max_lines = max_submission_lines();
    if lines.len() <= max_lines {
        return lines;
    }

    let total = lines.len();
    let side_lines = copyright_side_lines();
    let mut selected = Vec::with_capacity(max_lines);
    selected.extend(lines[..side_lines].iter().cloned());
    selected.extend(lines[total - side_lines..].iter().cloned());
    selected
}

pub fn submission_lines_from_content(content: &str) -> Vec<String> {
    select_submission_lines(content.lines().map(ToOwned::to_owned).collect())
}

fn non_empty_lines(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_submission_lines_keeps_all_until_sixty_pages() {
        let max_lines = max_submission_lines();
        let lines_before_limit: Vec<String> =
            (1..max_lines).map(|i| format!("line{}", i)).collect();
        assert_eq!(
            select_submission_lines(lines_before_limit).len(),
            max_lines - 1
        );

        let lines_at_limit: Vec<String> = (1..=max_lines).map(|i| format!("line{}", i)).collect();
        let selected_at_limit = select_submission_lines(lines_at_limit);
        assert_eq!(selected_at_limit.len(), max_lines);
        assert_eq!(
            selected_at_limit[max_lines - 1],
            format!("line{}", max_lines)
        );
    }

    #[test]
    fn select_submission_lines_keeps_first_and_last_thirty_pages() {
        let max_lines = max_submission_lines();
        let side_lines = copyright_side_lines();
        let lines: Vec<String> = (1..=max_lines + 1).map(|i| format!("line{}", i)).collect();
        let selected = select_submission_lines(lines);

        assert_eq!(selected.len(), max_lines);
        assert_eq!(selected[0], "line1");
        assert_eq!(selected[side_lines - 1], format!("line{}", side_lines));
        assert_eq!(selected[side_lines], format!("line{}", side_lines + 2));
        assert_eq!(selected[max_lines - 1], format!("line{}", max_lines + 1));
    }

    #[test]
    fn removes_blank_lines_before_counting_and_selecting() {
        let result = extracted_from_lines(vec![
            "alpha".to_string(),
            "".to_string(),
            "   ".to_string(),
            "beta".to_string(),
        ]);

        assert_eq!(result.content, "alpha\nbeta");
        assert_eq!(result.line_count, 2);
        assert_eq!(result.lines_per_page, code_lines_per_page());
        assert_eq!(result.page_count, 1);
    }
}

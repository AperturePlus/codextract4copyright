use docx_rs::{LineSpacing, LineSpacingType, PageMargin, RunFonts};

pub const PAGE_WIDTH_TWIPS: u32 = 11906;
pub const PAGE_HEIGHT_TWIPS: u32 = 16838;

pub const PAGE_MARGIN_TOP_TWIPS: i32 = 1985;
pub const PAGE_MARGIN_RIGHT_TWIPS: i32 = 1701;
pub const PAGE_MARGIN_BOTTOM_TWIPS: i32 = 1701;
pub const PAGE_MARGIN_LEFT_TWIPS: i32 = 1701;
pub const PAGE_MARGIN_HEADER_TWIPS: i32 = 851;
pub const PAGE_MARGIN_FOOTER_TWIPS: i32 = 992;

pub const CODE_FONT_NAME: &str = "Times New Roman";
pub const CODE_FONT_SIZE_HALF_POINTS: usize = 18;
pub const CODE_LINE_HEIGHT_TWIPS: i32 = 200;
pub const COPYRIGHT_SIDE_PAGES: usize = 30;
pub const MAX_SUBMISSION_PAGES: usize = COPYRIGHT_SIDE_PAGES * 2;

pub fn code_lines_per_page() -> usize {
    let usable_height = PAGE_HEIGHT_TWIPS as i32 - PAGE_MARGIN_TOP_TWIPS - PAGE_MARGIN_BOTTOM_TWIPS;
    (usable_height / CODE_LINE_HEIGHT_TWIPS).max(1) as usize
}

pub fn page_count_for_line_count(line_count: usize) -> usize {
    let lines_per_page = code_lines_per_page();
    if line_count == 0 {
        0
    } else {
        line_count.div_ceil(lines_per_page)
    }
}

pub fn max_submission_lines() -> usize {
    code_lines_per_page() * MAX_SUBMISSION_PAGES
}

pub fn copyright_side_lines() -> usize {
    code_lines_per_page() * COPYRIGHT_SIDE_PAGES
}

pub fn code_page_margin() -> PageMargin {
    PageMargin::new()
        .top(PAGE_MARGIN_TOP_TWIPS)
        .right(PAGE_MARGIN_RIGHT_TWIPS)
        .bottom(PAGE_MARGIN_BOTTOM_TWIPS)
        .left(PAGE_MARGIN_LEFT_TWIPS)
        .header(PAGE_MARGIN_HEADER_TWIPS)
        .footer(PAGE_MARGIN_FOOTER_TWIPS)
        .gutter(0)
}

pub fn code_line_spacing() -> LineSpacing {
    LineSpacing::new()
        .before(0)
        .after(0)
        .line_rule(LineSpacingType::Exact)
        .line(CODE_LINE_HEIGHT_TWIPS)
}

pub fn code_run_fonts() -> RunFonts {
    RunFonts::new()
        .ascii(CODE_FONT_NAME)
        .hi_ansi(CODE_FONT_NAME)
        .east_asia(CODE_FONT_NAME)
        .cs(CODE_FONT_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_lines_per_page_from_layout_constants() {
        let usable_height =
            PAGE_HEIGHT_TWIPS as i32 - PAGE_MARGIN_TOP_TWIPS - PAGE_MARGIN_BOTTOM_TWIPS;

        assert_eq!(
            code_lines_per_page(),
            (usable_height / CODE_LINE_HEIGHT_TWIPS) as usize
        );
        assert!(code_lines_per_page() > 50);
    }
}

use crate::pass::blank_line::collapse_blank_lines;
use crate::pass::blank_line::ensure_trailing_newline;
use crate::pass::chain::reformat_chains;
use crate::pass::derive::sort_derive_args;
use crate::pass::doc_comment::format_doc_comments;
use crate::pass::import::hoist_late_imports;
use crate::pass::import::sort_and_group_imports;
use crate::pass::line_length::collapse_opening_braces;
use crate::pass::line_length::expand_long_inline_blocks;
use crate::pass::section_header::format_section_headers;
use crate::pass::whitespace::format_whitespace;


// =====================
// === format_source ===
// =====================

pub fn format_source(source: &str) -> String {
    let source = sort_derive_args(source);
    let source = hoist_late_imports(&source);
    let source = sort_and_group_imports(&source);
    let source = format_whitespace(&source);
    let source = reformat_chains(&source);
    let source = expand_long_inline_blocks(&source);
    let source = format_whitespace(&source);
    let source = expand_long_inline_blocks(&source);
    let source = format_whitespace(&source);
    let source = collapse_opening_braces(&source);
    let source = format_section_headers(&source);
    let source = collapse_blank_lines(&source);
    let source = format_doc_comments(&source);
    ensure_trailing_newline(source)
}


// ========================
// === leading_whitespace ===
// ========================

pub(crate) fn leading_whitespace(line: &str) -> &str {
    let trimmed = line.trim_start();
    &line[..line.len() - trimmed.len()]
}

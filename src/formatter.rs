use crate::config::Config;
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

/// Formats Rust source code using default configuration.
pub fn format_source(source: &str) -> String {
    format_source_with_config(source, &Config::default())
}

/// Formats Rust source code by running all enabled formatting passes in sequence.
/// The pipeline is: derives -> imports -> whitespace -> chains -> line-length -> whitespace
/// (fixpoint loop) -> section headers -> blank lines -> doc comments -> trailing newline.
pub fn format_source_with_config(source: &str, config: &Config) -> String {
    let mut source = if config.sort_derives {
        sort_derive_args(source)
    } else {
        source.to_string()
    };
    if config.hoist_imports {
        source = hoist_late_imports(&source);
    }
    if config.sort_imports {
        source = sort_and_group_imports(&source);
    }
    source = format_whitespace(&source, config);
    if config.reformat_chains {
        source = reformat_chains(&source, config);
    }
    loop {
        if config.enforce_line_length {
            source = expand_long_inline_blocks(&source, config);
        }
        let next = format_whitespace(&source, config);
        if next == source {
            break;
        }
        source = next;
    }
    if config.enforce_line_length {
        source = collapse_opening_braces(&source, config);
    }
    if config.format_section_headers {
        source = format_section_headers(&source);
    }
    if config.collapse_blank_lines {
        source = collapse_blank_lines(&source);
    }
    if config.reflow_doc_comments {
        source = format_doc_comments(&source, config);
    }
    ensure_trailing_newline(source)
}


// ==========================
// === leading_whitespace ===
// ==========================

/// Returns the leading whitespace prefix of a line.
pub(crate) fn leading_whitespace(line: &str) -> &str {
    let trimmed = line.trim_start();
    &line[..line.len() - trimmed.len()]
}

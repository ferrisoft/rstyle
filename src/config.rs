// ==============
// === Config ===
// ==============

/// Formatter configuration. All options have sensible defaults; load overrides from
/// `rustformat.toml` or construct with `Config::default()`.
pub struct Config {
    /// Maximum allowed line length before the formatter breaks lines. Default: 120.
    pub max_line_length: usize,
    /// Number of spaces per indentation level. Default: 4.
    pub indent_width: usize,
    /// Sort `#[derive(...)]` arguments alphabetically. Default: true.
    pub sort_derives: bool,
    /// Sort and group `use` imports (one-per-line, alphabetical, grouped). Default: true.
    pub sort_imports: bool,
    /// Move `use` statements that appear after non-import items back to the import section. Default: true.
    pub hoist_imports: bool,
    /// Reflow doc comments (`///`, `//!`) to fit within `max_line_length`. Default: true.
    pub reflow_doc_comments: bool,
    /// Normalize `// === Name ===` section-header borders. Default: true.
    pub format_section_headers: bool,
    /// Collapse runs of multiple blank lines into a single blank line. Default: true.
    pub collapse_blank_lines: bool,
    /// Reformat method chains that exceed `max_line_length`. Default: true.
    pub reformat_chains: bool,
    /// Expand inline blocks (e.g. `{ ... }`) onto multiple lines when they exceed `max_line_length`. Default: true.
    pub enforce_line_length: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            max_line_length: 120,
            indent_width: 4,
            sort_derives: true,
            sort_imports: true,
            hoist_imports: true,
            reflow_doc_comments: true,
            format_section_headers: true,
            collapse_blank_lines: true,
            reformat_chains: true,
            enforce_line_length: true,
        }
    }
}

impl Config {
    /// Returns a string of `indent_width` spaces, used as one indentation level.
    pub fn indent_str(&self) -> String {
        " ".repeat(self.indent_width)
    }
}

// ==============
// === Config ===
// ==============

pub struct Config {
    pub max_line_length: usize,
    pub indent_width: usize,
    pub sort_derives: bool,
    pub sort_imports: bool,
    pub hoist_imports: bool,
    pub reflow_doc_comments: bool,
    pub format_section_headers: bool,
    pub collapse_blank_lines: bool,
    pub reformat_chains: bool,
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
    pub fn indent_str(&self) -> String {
        " ".repeat(self.indent_width)
    }
}

use crate::config::Config;
use crate::formatter::leading_whitespace;


// =============================
// === format_doc_comments ===
// =============================

pub(crate) fn format_doc_comments(source: &str, config: &Config) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let n = lines.len();
    let mut result: Vec<String> = Vec::new();
    let mut i = 0;
    while i < n {
        let trimmed = lines[i].trim();
        let prefix = if trimmed.starts_with("///") {
            "///"
        } else if trimmed.starts_with("//!") {
            "//!"
        } else {
            result.push(lines[i].to_string());
            i += 1;
            continue;
        };
        let indent = leading_whitespace(lines[i]);
        let mut block_lines: Vec<&str> = Vec::new();
        let block_start = i;
        while i < n {
            let t = lines[i].trim();
            if let Some(after_prefix) = t.strip_prefix(prefix) {
                let content = after_prefix.strip_prefix(' ').unwrap_or(after_prefix);
                block_lines.push(content);
                i += 1;
            } else {
                break;
            }
        }
        let md_input = block_lines.join("\n");
        let available_width = config.max_line_length
            .saturating_sub(indent.len())
            .saturating_sub(prefix.len() + 1);
        if available_width < 20 {
            for line in &lines[block_start..i] {
                result.push(line.to_string());
            }
            continue;
        }
        let mut options = comrak::Options::default();
        options.render.width = available_width;
        let formatted = comrak::markdown_to_commonmark(&md_input, &options);
        let formatted = formatted.trim_end();
        for line in formatted.split('\n') {
            if line.is_empty() {
                result.push(format!("{indent}{prefix}"));
            } else {
                result.push(format!("{indent}{prefix} {line}"));
            }
        }
    }
    result.join("\n")
}

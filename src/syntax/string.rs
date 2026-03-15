use ra_ap_syntax::SyntaxToken;

use crate::config::Config;


// ==============================
// === reindent_string_token ===
// ==============================

/// Re-indents a multiline string literal to match the surrounding code's indentation level.
/// Content lines get indent+1; closing quote gets indent+0. Relative indentation within the
/// string is preserved.
pub(crate) fn reindent_string_token(output: &mut String, token: &SyntaxToken, config: &Config) {
    let text = token.text();
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() <= 1 {
        output.push_str(text);
        return;
    }
    let last_newline = output.rfind('\n').map_or(0, |p| p + 1);
    let current_line = &output[last_newline..];
    let visual_indent = current_line.len() - current_line.trim_start().len();
    let indent_level = visual_indent / config.indent_width;
    let content_indent: String = config.indent_str().repeat(indent_level + 1);
    let closing_indent: String = config.indent_str().repeat(indent_level);
    let last_idx = lines.len() - 1;
    let last_is_just_quote = lines[last_idx].trim() == "\"";
    let content_end = if last_is_just_quote { last_idx } else { last_idx + 1 };
    let min_indent = lines[1..content_end]
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    output.push_str(lines[0]);
    for line in &lines[1..content_end] {
        output.push('\n');
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let leading = line.len() - line.trim_start().len();
        let relative = leading.saturating_sub(min_indent);
        output.push_str(&content_indent);
        for _ in 0..relative {
            output.push(' ');
        }
        output.push_str(trimmed);
    }
    if last_is_just_quote {
        output.push('\n');
        output.push_str(&closing_indent);
        output.push('"');
    }
}

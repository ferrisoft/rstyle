use ra_ap_syntax::AstNode;
use ra_ap_syntax::Edition;
use ra_ap_syntax::SourceFile;
use ra_ap_syntax::SyntaxKind::*;

use crate::config::Config;
use crate::formatter::leading_whitespace;


// ==================================
// === expand_long_inline_blocks ===
// ==================================

pub(crate) fn expand_long_inline_blocks(source: &str, config: &Config) -> String {
    let parse = SourceFile::parse(source, Edition::CURRENT);
    let tree = parse.tree();
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();
    for node in tree.syntax().descendants() {
        let kind = node.kind();
        let expandable = match kind {
            STMT_LIST => node.children().count() >= 1,
            PARAM_LIST | RECORD_EXPR_FIELD_LIST => node.children().count() >= 2,
            TOKEN_TREE => {
                let first = node.first_child_or_token().map(|f| f.kind());
                let has_curly = first == Some(L_CURLY);
                let is_macro_call_paren = first == Some(L_PAREN)
                    && node.parent().map(|p| p.kind()) == Some(MACRO_CALL);
                let content_count = node
                    .children_with_tokens()
                    .filter(|c| !matches!(c.kind(), WHITESPACE | L_CURLY | R_CURLY | L_PAREN | R_PAREN))
                    .count();
                (has_curly || is_macro_call_paren) && content_count >= 1
            }
            _ => continue,
        };
        if !expandable {
            continue;
        }
        let start: usize = node.text_range().start().into();
        let end: usize = node.text_range().end().into();
        let text = &source[start..end];
        if text.contains('\n') {
            continue;
        }
        let line_start = source[..start].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line_end = source[end..].find('\n').map(|p| end + p).unwrap_or(source.len());
        if line_end - line_start <= config.max_line_length {
            continue;
        }
        let children: Vec<_> = node.children_with_tokens().collect();
        let break_at_commas_only = kind == TOKEN_TREE;
        for (idx, child) in children.iter().enumerate() {
            if child.kind() == WHITESPACE {
                let should_break = if break_at_commas_only {
                    idx > 0 && matches!(children[idx - 1].kind(), L_CURLY | COMMA)
                        || children.get(idx + 1).map(|c| c.kind()) == Some(R_CURLY)
                } else {
                    true
                };
                if should_break {
                    let ws_start: usize = child.text_range().start().into();
                    let ws_end: usize = child.text_range().end().into();
                    replacements.push((ws_start, ws_end, "\n".to_string()));
                }
            } else if matches!(child.kind(), L_PAREN | L_BRACK | L_CURLY)
                && let Some(next) = children.get(idx + 1)
                && next.kind() != WHITESPACE
            {
                let pos: usize = child.text_range().end().into();
                replacements.push((pos, pos, "\n".to_string()));
            } else if matches!(child.kind(), R_PAREN | R_BRACK | R_CURLY)
                && let Some(prev) = children.get(idx.wrapping_sub(1))
                && prev.kind() != WHITESPACE
            {
                let pos: usize = child.text_range().start().into();
                replacements.push((pos, pos, "\n".to_string()));
            }
        }
    }
    if replacements.is_empty() {
        return source.to_string();
    }
    replacements.sort_by(|a, b| b.0.cmp(&a.0));
    let mut result = source.to_string();
    for (start, end, new_ws) in replacements {
        result.replace_range(start..end, &new_ws);
    }
    result
}


// ================================
// === collapse_opening_braces ===
// ================================

pub(crate) fn collapse_opening_braces(source: &str, config: &Config) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut result: Vec<String> = Vec::new();
    for line in &lines {
        let trimmed = line.trim();
        if !result.is_empty() {
            let prev_trimmed = result.last().map(|l| l.trim().to_string()).unwrap_or_default();
            if trimmed == "{" && !prev_trimmed.is_empty() && !prev_trimmed.ends_with('{') {
                result.last_mut().unwrap().push_str(" {");
                continue;
            }
            if trimmed.starts_with("where") && !prev_trimmed.is_empty() {
                let prev = result.last().unwrap();
                let merged_len = prev.len() + 1 + trimmed.len();
                if merged_len <= config.max_line_length {
                    result.last_mut().unwrap().push_str(&format!(" {trimmed}"));
                } else {
                    let indent = leading_whitespace(line);
                    let after_where = trimmed.strip_prefix("where").unwrap().trim_start();
                    result.last_mut().unwrap().push_str(" where");
                    if !after_where.is_empty() {
                        result.push(format!("{indent}{after_where}"));
                    }
                }
                continue;
            }
        }
        result.push(line.to_string());
    }
    result.join("\n")
}

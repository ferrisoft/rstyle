use ra_ap_syntax::AstNode;
use ra_ap_syntax::Edition;
use ra_ap_syntax::SourceFile;
use ra_ap_syntax::SyntaxKind::*;
use ra_ap_syntax::SyntaxNode;

use crate::syntax::indentation::compute_indent_level;

const MAX_LINE_LENGTH: usize = 120;


// ========================
// === reformat_chains ===
// ========================

pub(crate) struct ChainBreakPoint {
    dot_offset: usize,
    ws_start: usize,
    is_method_call: bool,
    has_newline: bool,
}

pub(crate) fn reformat_chains(source: &str) -> String {
    let parse = SourceFile::parse(source, Edition::CURRENT);
    let tree = parse.tree();
    let chain_ranges: Vec<_> = tree
        .syntax()
        .descendants()
        .filter(is_chain_root)
        .map(|n| n.text_range())
        .collect();
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();
    for node in tree.syntax().descendants() {
        if !is_chain_root(&node) {
            continue;
        }
        let range = node.text_range();
        let is_nested = chain_ranges.iter().any(|other| {
            *other != range && other.start() <= range.start() && range.end() <= other.end()
        });
        if is_nested {
            let chain_start: usize = node.text_range().start().into();
            let chain_end: usize = node.text_range().end().into();
            let line_start = source[..chain_start].rfind('\n').map(|p| p + 1).unwrap_or(0);
            let line_end = source[chain_end..].find('\n').map(|p| chain_end + p).unwrap_or(source.len());
            if line_end - line_start <= MAX_LINE_LENGTH {
                continue;
            }
        }
        let break_points = collect_chain_break_points(&node, source);
        if break_points.is_empty() {
            continue;
        }
        let chain_start: usize = node.text_range().start().into();
        let chain_end: usize = node.text_range().end().into();
        let chain_text = &source[chain_start..chain_end];
        let flat_text = flatten_chain_text(chain_text, &break_points, chain_start);
        let can_collapse = !flat_text.contains('\n');
        if can_collapse {
            let line_start = source[..chain_start].rfind('\n').map(|p| p + 1).unwrap_or(0);
            let prefix_len = chain_start - line_start;
            let after_chain = &source[chain_end..];
            let suffix_len = after_chain
                .find('\n')
                .map(|p| after_chain[..p].trim_end().len())
                .unwrap_or_else(|| after_chain.trim_end().len());
            let total_flat_len = prefix_len + flat_text.len() + suffix_len;
            if total_flat_len <= MAX_LINE_LENGTH {
                for bp in &break_points {
                    if bp.has_newline {
                        replacements.push((bp.ws_start, bp.dot_offset, String::new()));
                    }
                }
                continue;
            }
            // Too long to collapse — break at method dots
            let indent_level = compute_chain_indent(&node);
            let indent = "    ".repeat(indent_level + 1);
            let has_existing_breaks = break_points.iter().any(|bp| bp.has_newline);
            if has_existing_breaks {
                let first_break_idx = break_points.iter()
                    .position(|bp| bp.has_newline)
                    .expect("has_existing_breaks is true");
                let first_break_bp = &break_points[first_break_idx];
                let line_start = source[..chain_start].rfind('\n').map(|p| p + 1).unwrap_or(0);
                let first_line_len = first_break_bp.ws_start - line_start;
                if first_line_len > MAX_LINE_LENGTH {
                    for bp in &break_points[..first_break_idx] {
                        if bp.is_method_call && !bp.has_newline {
                            replacements.push((bp.ws_start, bp.dot_offset, format!("\n{indent}")));
                        }
                    }
                }
                for bp in &break_points[first_break_idx..] {
                    if bp.is_method_call && !bp.has_newline {
                        replacements.push((bp.ws_start, bp.dot_offset, format!("\n{indent}")));
                    }
                }
            } else {
                for bp in &break_points {
                    if bp.is_method_call {
                        replacements.push((bp.ws_start, bp.dot_offset, format!("\n{indent}")));
                    }
                }
            }
        } else {
            // Multi-line chain (closures/blocks) — only break dots on lines > MAX_LINE_LENGTH,
            // and collapse breaks after multi-line content if the resulting line fits.
            let indent_level = compute_chain_indent(&node);
            let indent = "    ".repeat(indent_level + 1);
            for bp in &break_points {
                if !bp.is_method_call {
                    continue;
                }
                if bp.has_newline {
                    // Preserve existing breaks in multi-line chains. Chain dots
                    // should stay aligned — don't collapse })\n.method() into
                    // }).method().
                } else {
                    // No existing break — add one if the line is too long
                    let line_start = source[..bp.dot_offset]
                        .rfind('\n')
                        .map(|p| p + 1)
                        .unwrap_or(0);
                    let line_end = source[bp.dot_offset..]
                        .find('\n')
                        .map(|p| bp.dot_offset + p)
                        .unwrap_or(source.len());
                    let line_len = line_end - line_start;
                    if line_len > MAX_LINE_LENGTH {
                        replacements.push((bp.ws_start, bp.dot_offset, format!("\n{indent}")));
                    }
                }
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

fn is_chain_root(node: &SyntaxNode) -> bool {
    if !matches!(node.kind(), METHOD_CALL_EXPR | FIELD_EXPR | AWAIT_EXPR) {
        return false;
    }
    let mut ancestor = node.parent();
    loop {
        match ancestor {
            None => return true,
            Some(a) => match a.kind() {
                TRY_EXPR => {
                    ancestor = a.parent();
                }
                METHOD_CALL_EXPR | FIELD_EXPR | AWAIT_EXPR => return false,
                _ => return true,
            },
        }
    }
}

fn collect_chain_break_points(root: &SyntaxNode, source: &str) -> Vec<ChainBreakPoint> {
    let mut points = Vec::new();
    let mut current = root.clone();
    loop {
        match current.kind() {
            METHOD_CALL_EXPR | FIELD_EXPR => {
                let is_method = current.kind() == METHOD_CALL_EXPR;
                if let Some(dot) = current.children_with_tokens().find(|c| c.kind() == DOT) {
                    let dot_offset: usize = dot.text_range().start().into();
                    let ws_start = find_ws_start_before(source, dot_offset);
                    let has_newline = source[ws_start..dot_offset].contains('\n');
                    points.push(ChainBreakPoint { dot_offset, ws_start, is_method_call: is_method, has_newline });
                }
                match current.children().next() {
                    Some(child) => current = child,
                    None => break,
                }
            }
            AWAIT_EXPR => {
                if let Some(dot) = current.children_with_tokens().find(|c| c.kind() == DOT) {
                    let dot_offset: usize = dot.text_range().start().into();
                    let ws_start = find_ws_start_before(source, dot_offset);
                    let has_newline = source[ws_start..dot_offset].contains('\n');
                    points.push(ChainBreakPoint { dot_offset, ws_start, is_method_call: true, has_newline });
                }
                match current.children().next() {
                    Some(child) => current = child,
                    None => break,
                }
            }
            TRY_EXPR => {
                match current.children().next() {
                    Some(child) => current = child,
                    None => break,
                }
            }
            _ => break,
        }
    }
    points.sort_by_key(|bp| bp.dot_offset);
    points
}

fn find_ws_start_before(source: &str, pos: usize) -> usize {
    let bytes = source.as_bytes();
    let mut start = pos;
    while start > 0 && matches!(bytes[start - 1], b' ' | b'\t' | b'\n' | b'\r') {
        start -= 1;
    }
    start
}

fn flatten_chain_text(chain_text: &str, break_points: &[ChainBreakPoint], chain_start: usize) -> String {
    let mut result = String::new();
    let mut pos = 0;
    for bp in break_points {
        if !bp.has_newline {
            continue;
        }
        let ws_local = bp.ws_start - chain_start;
        let dot_local = bp.dot_offset - chain_start;
        result.push_str(&chain_text[pos..ws_local]);
        pos = dot_local;
    }
    result.push_str(&chain_text[pos..]);
    result
}

fn compute_chain_indent(root: &SyntaxNode) -> usize {
    root.children_with_tokens()
        .find(|c| c.kind() == DOT)
        .and_then(|dot| dot.into_token())
        .map(|t| compute_indent_level(&t))
        .unwrap_or(0)
}

use ra_ap_syntax::SyntaxKind::*;
use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::SyntaxToken;


// ============================
// === compute_indent_level ===
// ============================

pub(crate) fn compute_indent_level(token: &SyntaxToken) -> usize {
    let mut level: usize = 0;
    let mut node = token.parent();
    while let Some(n) = node {
        if is_indent_node(&n, token) {
            level += 1;
        }
        node = n.parent();
    }
    if is_macro_repetition_delimiter(token) {
        level = level.saturating_sub(1);
    }
    level
}

fn is_indent_node(node: &SyntaxNode, token: &SyntaxToken) -> bool {
    let kind = node.kind();
    let indenting = matches!(
        kind,
        STMT_LIST
            | ITEM_LIST
            | ASSOC_ITEM_LIST
            | MATCH_ARM_LIST
            | RECORD_FIELD_LIST
            | RECORD_EXPR_FIELD_LIST
            | RECORD_PAT_FIELD_LIST
            | VARIANT_LIST
            | USE_TREE_LIST
            | EXTERN_ITEM_LIST
            | PARAM_LIST
    );
    if indenting {
        return !is_delimiter_of(node, token);
    }
    if kind == TOKEN_TREE {
        if is_macro_repetition(node) {
            return false;
        }
        return !is_delimiter_of(node, token);
    }
    false
}

fn is_delimiter_of(node: &SyntaxNode, token: &SyntaxToken) -> bool {
    if token.parent().as_ref() != Some(node) {
        return false;
    }
    let tk = token.kind();
    matches!(tk, L_CURLY | R_CURLY | L_BRACK | R_BRACK | L_PAREN | R_PAREN)
}

pub(crate) fn is_macro_repetition(node: &SyntaxNode) -> bool {
    match node.prev_sibling_or_token() {
        Some(p) if p.kind() == DOLLAR => true,
        Some(p) if p.kind() == WHITESPACE => p
            .prev_sibling_or_token()
            .map(|pp| pp.kind() == DOLLAR)
            .unwrap_or(false),
        _ => false,
    }
}

fn is_macro_repetition_delimiter(token: &SyntaxToken) -> bool {
    matches!(token.kind(), R_PAREN | L_PAREN)
        && token
            .parent()
            .map(|p| p.kind() == TOKEN_TREE && is_macro_repetition(&p))
            .unwrap_or(false)
}


// ==============================
// === emit_newline_whitespace ===
// ==============================

pub(crate) fn emit_newline_whitespace(output: &mut String, ws: &str, next_token: &SyntaxToken) {
    let newline_count = ws.chars().filter(|c| *c == '\n').count();
    for _ in 0..newline_count {
        output.push('\n');
    }
    let mut indent_level = compute_indent_level(next_token);
    if next_token.kind() == DOT {
        indent_level += 1;
    }
    indent_level += count_continuation_ancestors(next_token);
    for _ in 0..indent_level {
        output.push_str("    ");
    }
}


// ==============================
// === has_continuation_dot ===
// ==============================

fn has_continuation_dot(node: &SyntaxNode) -> bool {
    if let Some(dot) = node.children_with_tokens().find(|c| c.kind() == DOT)
        && let Some(prev) = dot.prev_sibling_or_token()
        && prev.kind() == WHITESPACE
    {
        return prev.as_token().map(|t| t.text().contains('\n')).unwrap_or(false);
    }
    false
}


// ======================================
// === count_continuation_ancestors ===
// ======================================

fn count_continuation_ancestors(token: &SyntaxToken) -> usize {
    let mut count = 0;
    let Some(first_parent) = token.parent() else { return 0 };
    let mut prev_node = first_parent;
    let mut current = prev_node.parent();
    while let Some(node) = current {
        if matches!(node.kind(), METHOD_CALL_EXPR | FIELD_EXPR | AWAIT_EXPR) {
            let came_from_receiver = node
                .children()
                .next()
                .map(|fc| fc == prev_node)
                .unwrap_or(false);
            if !came_from_receiver && has_continuation_dot(&node) {
                count += 1;
            }
        }
        prev_node = node;
        current = prev_node.parent();
    }
    count
}

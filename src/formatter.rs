use ra_ap_syntax::AstNode;
use ra_ap_syntax::Edition;
use ra_ap_syntax::SourceFile;
use ra_ap_syntax::SyntaxKind;
use ra_ap_syntax::SyntaxKind::*;
use ra_ap_syntax::SyntaxNode;
use ra_ap_syntax::SyntaxToken;


// ========================
// === sort_derive_args ===
// ========================

fn sort_derive_args(source: &str) -> String {
    let parse = SourceFile::parse(source, Edition::CURRENT);
    let tree = parse.tree();
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();
    for node in tree.syntax().descendants() {
        if node.kind() != ATTR {
            continue;
        }
        let text = node.text().to_string();
        if !text.starts_with("#[derive(") {
            continue;
        }
        let Some(start_paren) = text.find('(') else { continue };
        let Some(end_paren) = text.rfind(')') else { continue };
        let inner = &text[start_paren + 1..end_paren];
        let args: Vec<&str> = inner.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let mut sorted = args.clone();
        sorted.sort();
        if args == sorted {
            continue;
        }
        let new_text = format!("#[derive({})]", sorted.join(", "));
        let range_start: usize = node.text_range().start().into();
        let range_end: usize = node.text_range().end().into();
        replacements.push((range_start, range_end, new_text));
    }
    if replacements.is_empty() {
        return source.to_string();
    }
    let mut result = source.to_string();
    for (start, end, replacement) in replacements.into_iter().rev() {
        result.replace_range(start..end, &replacement);
    }
    result
}


// ============================
// === hoist_late_imports ===
// ============================

fn hoist_late_imports(source: &str) -> String {
    let parse = SourceFile::parse(source, Edition::CURRENT);
    let tree = parse.tree();
    let root = tree.syntax();
    let mut past_initial = false;
    let mut seen_import = false;
    let mut insert_after: usize = 0;
    let mut late_ranges: Vec<(usize, usize)> = Vec::new();
    let mut late_texts: Vec<String> = Vec::new();
    for child in root.children() {
        match child.kind() {
            USE => {
                if past_initial {
                    let start: usize = child.text_range().start().into();
                    let end: usize = child.text_range().end().into();
                    late_ranges.push((start, end));
                    late_texts.push(child.text().to_string());
                } else {
                    seen_import = true;
                    insert_after = child.text_range().end().into();
                }
            }
            MODULE => {
                let has_body = child.children().any(|c| c.kind() == ITEM_LIST);
                if has_body {
                    if seen_import {
                        past_initial = true;
                    }
                } else if !past_initial {
                    seen_import = true;
                    insert_after = child.text_range().end().into();
                }
            }
            _ => {
                if seen_import {
                    past_initial = true;
                }
            }
        }
    }
    if late_ranges.is_empty() {
        return source.to_string();
    }
    let mut result = source.to_string();
    for &(start, end) in late_ranges.iter().rev() {
        let bytes = result.as_bytes();
        let mut actual_end = end;
        while actual_end < bytes.len() && matches!(bytes[actual_end], b'\n' | b'\r' | b' ' | b'\t') {
            actual_end += 1;
        }
        result.replace_range(start..actual_end, "");
    }
    let insert_text = late_texts.join("\n");
    result.insert_str(insert_after, &format!("\n{insert_text}"));
    result
}


// ===============================
// === sort_and_group_imports ===
// ===============================

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum ImportGroup {
    Mod,
    Star,
    Foreign,
    Crate,
    PubReexport,
}

fn sort_and_group_imports(source: &str) -> String {
    let parse = SourceFile::parse(source, Edition::CURRENT);
    let tree = parse.tree();
    let root = tree.syntax();

    struct RawImport {
        attrs: Vec<String>,
        visibility: String,
        paths: Vec<String>,
        is_mod: bool,
        sort_key: String,
    }

    let mut items: Vec<RawImport> = Vec::new();
    let mut section_start: Option<usize> = None;
    let mut section_end: usize = 0;
    let mut past_imports = false;

    for child in root.children() {
        if past_imports {
            break;
        }
        match child.kind() {
            USE => {
                let start: usize = child.text_range().start().into();
                let end: usize = child.text_range().end().into();
                if section_start.is_none() {
                    section_start = Some(start);
                }
                section_end = end;
                let attrs: Vec<String> = child
                    .children()
                    .filter(|c| c.kind() == ATTR)
                    .map(|a| a.text().to_string())
                    .collect();
                let vis = child
                    .children()
                    .find(|c| c.kind() == VISIBILITY)
                    .map(|v| format!("{} ", v.text()))
                    .unwrap_or_default();
                let use_tree = child.children().find(|c| c.kind() == USE_TREE);
                let paths = use_tree
                    .map(|t| flatten_use_tree("", &t))
                    .unwrap_or_default();
                items.push(RawImport {
                    attrs,
                    visibility: vis,
                    paths,
                    is_mod: false,
                    sort_key: String::new(),
                });
            }
            MODULE => {
                if child.children().any(|c| c.kind() == ITEM_LIST) {
                    past_imports = true;
                    continue;
                }
                let start: usize = child.text_range().start().into();
                let end: usize = child.text_range().end().into();
                if section_start.is_none() {
                    section_start = Some(start);
                }
                section_end = end;
                let vis = child
                    .children()
                    .find(|c| c.kind() == VISIBILITY)
                    .map(|v| format!("{} ", v.text()))
                    .unwrap_or_default();
                let name = child
                    .children()
                    .find(|c| c.kind() == NAME)
                    .and_then(|n| n.first_token())
                    .map(|t| t.text().to_string())
                    .unwrap_or_default();
                items.push(RawImport {
                    attrs: Vec::new(),
                    visibility: vis.clone(),
                    paths: Vec::new(),
                    is_mod: true,
                    sort_key: name.clone(),
                });
            }
            _ => {
                if section_start.is_some() {
                    past_imports = true;
                }
            }
        }
    }

    if items.is_empty() {
        return source.to_string();
    }
    let section_start = section_start.expect("section_start must be set if items is non-empty");

    let bytes = source.as_bytes();
    let mut end = section_end;
    while end < bytes.len() && matches!(bytes[end], b'\n' | b'\r' | b' ' | b'\t') {
        end += 1;
    }
    section_end = end;

    struct FlatImport {
        attrs: Vec<String>,
        vis: String,
        path: String,
        group: ImportGroup,
    }

    let mut flat: Vec<FlatImport> = Vec::new();
    for item in &items {
        if item.is_mod {
            flat.push(FlatImport {
                attrs: Vec::new(),
                vis: item.visibility.clone(),
                path: item.sort_key.clone(),
                group: ImportGroup::Mod,
            });
            continue;
        }
        let is_pub = !item.visibility.is_empty();
        for path in &item.paths {
            let is_star = path.ends_with("::*") || path == "*";
            let group = if is_star {
                ImportGroup::Star
            } else if is_pub {
                ImportGroup::PubReexport
            } else if path.starts_with("crate::")
                || path.starts_with("self::")
                || path.starts_with("super::")
            {
                ImportGroup::Crate
            } else {
                ImportGroup::Foreign
            };
            flat.push(FlatImport {
                attrs: item.attrs.clone(),
                vis: item.visibility.clone(),
                path: path.clone(),
                group,
            });
        }
    }

    flat.sort_by(|a, b| a.group.cmp(&b.group).then_with(|| a.path.cmp(&b.path)));
    flat.dedup_by(|a, b| a.group == b.group && a.path == b.path && a.vis == b.vis);

    let mut groups: Vec<Vec<String>> = vec![Vec::new(); 5];
    for imp in &flat {
        let line = if imp.group == ImportGroup::Mod {
            format!("{}mod {};", imp.vis, imp.path)
        } else {
            let mut s = String::new();
            for attr in &imp.attrs {
                s.push_str(attr);
                s.push('\n');
            }
            s.push_str(&format!("{}use {};", imp.vis, imp.path));
            s
        };
        groups[imp.group as usize].push(line);
    }

    let non_empty: Vec<String> = groups
        .into_iter()
        .filter(|g| !g.is_empty())
        .map(|g| g.join("\n"))
        .collect();
    let replacement = non_empty.join("\n\n");

    let before = &source[..section_start];
    let after = &source[section_end..];
    if after.is_empty() {
        format!("{before}{replacement}\n")
    } else {
        format!("{before}{replacement}\n\n{after}")
    }
}

fn flatten_use_tree(prefix: &str, node: &SyntaxNode) -> Vec<String> {
    let path_child = node.children().find(|c| c.kind() == PATH);
    let segment = path_child.map(|p| p.text().to_string()).unwrap_or_default();
    let full = match (prefix.is_empty(), segment.is_empty()) {
        (true, _) => segment.clone(),
        (_, true) => prefix.to_string(),
        _ => format!("{prefix}::{segment}"),
    };
    if let Some(list) = node.children().find(|c| c.kind() == USE_TREE_LIST) {
        return list
            .children()
            .filter(|c| c.kind() == USE_TREE)
            .flat_map(|child| flatten_use_tree(&full, &child))
            .collect();
    }
    if node
        .children_with_tokens()
        .any(|c| c.kind() == STAR)
    {
        if full.is_empty() {
            return vec!["*".to_string()];
        }
        return vec![format!("{full}::*")];
    }
    if segment == "self" {
        return vec![prefix.to_string()];
    }
    if let Some(rename) = node.children().find(|c| c.kind() == RENAME) {
        let name = rename
            .descendants_with_tokens()
            .filter_map(|c| c.into_token())
            .find(|t| t.kind() == IDENT || t.kind() == UNDERSCORE)
            .map(|t| t.text().to_string())
            .unwrap_or_default();
        return vec![format!("{full} as {name}")];
    }
    vec![full]
}


// =====================
// === format_source ===
// =====================

const MAX_LINE_LENGTH: usize = 120;

pub fn format_source(source: &str) -> String {
    let source = sort_derive_args(source);
    let source = hoist_late_imports(&source);
    let source = sort_and_group_imports(&source);
    let source = format_whitespace(&source);
    let source = reformat_chains(&source);
    let source = expand_long_inline_blocks(&source);
    let source = format_whitespace(&source);
    let source = collapse_opening_braces(&source);
    let source = format_section_headers(&source);
    let source = format_doc_comments(&source);
    ensure_trailing_newline(source)
}


// =========================
// === format_whitespace ===
// =========================

fn format_whitespace(source: &str) -> String {
    let parse = SourceFile::parse(source, Edition::CURRENT);
    let tree = parse.tree();
    let tokens: Vec<SyntaxToken> = tree.syntax()
        .descendants_with_tokens()
        .filter_map(|el| el.into_token())
        .collect();
    if tokens.is_empty() {
        return source.to_string();
    }
    let mut output = String::with_capacity(source.len());
    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        if token.kind() == WHITESPACE {
            i += 1;
            continue;
        }
        if token.kind() == COMMENT {
            output.push_str(token.text().trim_end());
        } else if token.kind() == STRING && token.text().contains('\n') {
            reindent_string_token(&mut output, token);
        } else {
            output.push_str(token.text());
        }
        let mut ws_text = String::new();
        let mut j = i + 1;
        while j < tokens.len() && tokens[j].kind() == WHITESPACE {
            ws_text.push_str(tokens[j].text());
            j += 1;
        }
        if j < tokens.len() {
            let next = &tokens[j];
            if ws_text.contains('\n') {
                emit_newline_whitespace(&mut output, &ws_text, next);
            } else {
                output.push_str(compute_spacing(token, next));
            }
        } else if ws_text.contains('\n') {
            let newlines = ws_text.chars().filter(|c| *c == '\n').count();
            for _ in 0..newlines {
                output.push('\n');
            }
        }
        i = j;
    }
    output
}


// ========================
// === reformat_chains ===
// ========================

struct ChainBreakPoint {
    dot_offset: usize,
    ws_start: usize,
    is_method_call: bool,
    has_newline: bool,
}

fn reformat_chains(source: &str) -> String {
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
            continue;
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


// ==================================
// === expand_long_inline_blocks ===
// ==================================

fn expand_long_inline_blocks(source: &str) -> String {
    let parse = SourceFile::parse(source, Edition::CURRENT);
    let tree = parse.tree();
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();
    for node in tree.syntax().descendants() {
        let kind = node.kind();
        let expandable = match kind {
            STMT_LIST => node.children().count() >= 2,
            PARAM_LIST | ARG_LIST | RECORD_EXPR_FIELD_LIST => node.children().count() >= 2,
            TOKEN_TREE => {
                let has_curly = node
                    .first_child_or_token()
                    .map(|f| f.kind() == L_CURLY)
                    .unwrap_or(false);
                let content_count = node
                    .children_with_tokens()
                    .filter(|c| !matches!(c.kind(), WHITESPACE | L_CURLY | R_CURLY))
                    .count();
                has_curly && content_count >= 3
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
        if line_end - line_start <= MAX_LINE_LENGTH {
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

fn collapse_opening_braces(source: &str) -> String {
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
                if merged_len <= MAX_LINE_LENGTH {
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


// ================================
// === format_section_headers ===
// ================================

fn format_section_headers(source: &str) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let n = lines.len();
    let mut header_starts: Vec<usize> = Vec::new();
    let mut i = 0;
    while i + 2 < n {
        let l1 = lines[i].trim();
        let l2 = lines[i + 1].trim();
        let l3 = lines[i + 2].trim();
        if is_section_border(l1) && is_section_middle(l2) && is_section_border(l3) {
            header_starts.push(i);
            i += 3;
        } else {
            i += 1;
        }
    }
    if header_starts.is_empty() {
        return source.to_string();
    }
    let mut result: Vec<String> = Vec::new();
    let mut i = 0;
    while i < n {
        if header_starts.contains(&i) {
            let middle_trimmed = lines[i + 1].trim();
            let indent = leading_whitespace(lines[i + 1]);
            let border_eq_count = middle_trimmed.len() - 3;
            let border = format!("{indent}// {}", "=".repeat(border_eq_count));
            let middle = format!("{indent}{middle_trimmed}");
            while !result.is_empty()
                && result.last().map(|l| l.trim().is_empty()).unwrap_or(false)
            {
                result.pop();
            }
            if !result.is_empty() {
                result.push(String::new());
                result.push(String::new());
            }
            result.push(border.clone());
            result.push(middle);
            result.push(border);
            i += 3;
            while i < n && lines[i].trim().is_empty() {
                i += 1;
            }
            result.push(String::new());
            continue;
        }
        result.push(lines[i].to_string());
        i += 1;
    }
    result.join("\n")
}

fn is_section_border(line: &str) -> bool {
    line.starts_with("// ") && line.len() > 6 && line[3..].chars().all(|c| c == '=')
}

fn is_section_middle(line: &str) -> bool {
    line.starts_with("// === ") && line.ends_with(" ===")
}

fn leading_whitespace(line: &str) -> &str {
    let trimmed = line.trim_start();
    &line[..line.len() - trimmed.len()]
}


// =============================
// === format_doc_comments ===
// =============================

fn format_doc_comments(source: &str) -> String {
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
        let available_width = MAX_LINE_LENGTH
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


// ==============================
// === emit_newline_whitespace ===
// ==============================

fn emit_newline_whitespace(output: &mut String, ws: &str, next_token: &SyntaxToken) {
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
// === reindent_string_token ===
// ==============================

fn reindent_string_token(output: &mut String, token: &SyntaxToken) {
    let text = token.text();
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() <= 1 {
        output.push_str(text);
        return;
    }
    let indent_level = compute_indent_level(token);
    let content_indent: String = "    ".repeat(indent_level + 1);
    let closing_indent: String = "    ".repeat(indent_level);
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


// ============================
// === compute_indent_level ===
// ============================

fn compute_indent_level(token: &SyntaxToken) -> usize {
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

fn is_macro_repetition_delimiter(token: &SyntaxToken) -> bool {
    matches!(token.kind(), R_PAREN | L_PAREN)
        && token
            .parent()
            .map(|p| p.kind() == TOKEN_TREE && is_macro_repetition(&p))
            .unwrap_or(false)
}


// ======================
// === is_indent_node ===
// ======================

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

fn is_macro_repetition(node: &SyntaxNode) -> bool {
    match node.prev_sibling_or_token() {
        Some(p) if p.kind() == DOLLAR => true,
        Some(p) if p.kind() == WHITESPACE => p
            .prev_sibling_or_token()
            .map(|pp| pp.kind() == DOLLAR)
            .unwrap_or(false),
        _ => false,
    }
}


// ======================
// === is_delimiter_of ===
// ======================

fn is_delimiter_of(node: &SyntaxNode, token: &SyntaxToken) -> bool {
    if token.parent().as_ref() != Some(node) {
        return false;
    }
    let tk = token.kind();
    matches!(tk, L_CURLY | R_CURLY | L_BRACK | R_BRACK | L_PAREN | R_PAREN)
}


// =======================
// === compute_spacing ===
// =======================

#[inline(always)]
fn compute_spacing(prev: &SyntaxToken, next: &SyntaxToken) -> &'static str {
    let pk = prev.kind();
    let nk = next.kind();

    // Path separator :: (also handles two COLON tokens inside TOKEN_TREE)
    if pk == COLON2 || nk == COLON2 {
        return "";
    }
    if pk == COLON && nk == COLON {
        return "";
    }
    // Field/method access .
    if pk == DOT || nk == DOT {
        return "";
    }
    // Range operators .. ..= ...
    if matches!(pk, DOT2 | DOT2EQ | DOT3) || matches!(nk, DOT2 | DOT2EQ | DOT3) {
        return "";
    }
    // No space after opening delimiters ( [
    if matches!(pk, L_PAREN | L_BRACK) {
        return "";
    }
    // No space before closing delimiters ) ]
    if matches!(nk, R_PAREN | R_BRACK) {
        return "";
    }
    // No space before , ;
    if matches!(nk, COMMA | SEMICOLON) {
        return "";
    }
    // No space before postfix ?
    if nk == QUESTION {
        return "";
    }
    // Attribute #: no space after
    if pk == POUND {
        return "";
    }
    // Macro pattern $ metavar: no space after $
    if pk == DOLLAR && is_in_token_tree(prev) {
        return "";
    }
    // Macro metavar $name:frag — no space around the :
    if nk == COLON && is_in_token_tree(next) && is_macro_metavar_colon(next) {
        return "";
    }
    if pk == COLON && is_in_token_tree(prev) && is_macro_metavar_colon(prev) {
        return "";
    }
    // Macro repetition ),* )* )+ )? in TOKEN_TREE: no space before operator
    if matches!(nk, STAR | PLUS) && is_in_token_tree(next)
        && matches!(pk, R_PAREN | COMMA)
    {
        return "";
    }
    // &< and *> in TOKEN_TREE (reference/pointer type syntax in macros)
    if pk == AMP && nk == L_ANGLE && is_in_token_tree(prev) {
        return "";
    }
    if matches!(pk, STAR | PLUS) && nk == R_ANGLE && is_in_token_tree(prev)
        && prev_non_whitespace_token(prev).map(|t| t.kind() != R_ANGLE).unwrap_or(true)
    {
        return "";
    }
    // Macro ident!: no space before !
    if nk == BANG && pk == IDENT {
        return "";
    }
    // Macro invocation !( ![: no space (but !{ gets space via space-before-{ rule)
    if pk == BANG && matches!(nk, L_PAREN | L_BRACK) {
        return "";
    }
    // => and -> inside TOKEN_TREE (split into EQ/MINUS + R_ANGLE)
    if matches!(pk, EQ | MINUS) && nk == R_ANGLE
        && is_in_token_tree(prev) {
            return "";
        }
    // Function call ident(: no space
    if pk == IDENT && nk == L_PAREN {
        return "";
    }
    // Chained )(: no space
    if pk == R_PAREN && nk == L_PAREN {
        return "";
    }
    // Indexing ident[ )[ ][ >[: no space
    if nk == L_BRACK && matches!(pk, IDENT | R_PAREN | R_BRACK | R_ANGLE) {
        return "";
    }
    // pub(crate): no space
    if pk == PUB_KW && nk == L_PAREN
        && prev.parent().map(|p| p.kind()) == Some(VISIBILITY) {
            return "";
        }
    // fn pointer type fn(args): no space
    if pk == FN_KW && nk == L_PAREN
        && prev.parent().map(|p| p.kind()) == Some(FN_PTR_TYPE) {
            return "";
        }
    // Unary operators: no space after (but not macro repetition * + after ) or ,)
    if matches!(pk, MINUS | STAR | AMP | BANG) && is_unary(prev)
        && !is_macro_repetition_op(prev)
    {
        return "";
    }
    // &'lifetime: no space
    if pk == AMP && nk == LIFETIME_IDENT {
        return "";
    }
    // Generic angle brackets: no space inside
    if pk == L_ANGLE && (is_in_generic_context(prev) || is_turbofish_angle(prev)) {
        return "";
    }
    if nk == R_ANGLE && (is_in_generic_context(next) || is_turbofish_angle(next)) {
        return "";
    }
    if nk == L_ANGLE && is_in_generic_context(next)
        && next.parent().map(|p| p.kind()) != Some(TYPE_ANCHOR) {
            return "";
        }
    // Turbofish >( in TOKEN_TREE: no space
    if pk == R_ANGLE && nk == L_PAREN
        && prev.parent().map(|p| p.kind()) == Some(TOKEN_TREE)
        && is_turbofish_angle(prev) {
            return "";
        }
    // Adjacent pipes || (empty closure or logical OR in TOKEN_TREE)
    if pk == PIPE && nk == PIPE {
        return "";
    }
    // Closure |params|: no space after opening | and before closing |
    if pk == PIPE && is_in_closure_params(prev) && is_opening_closure_pipe(prev) {
        return "";
    }
    if nk == PIPE && is_in_closure_params(next) && !is_opening_closure_pipe(next) {
        return "";
    }
    // No space before : (type annotations)
    if nk == COLON {
        return "";
    }
    // @ in patterns: no space around
    if pk == AT || nk == AT {
        return "";
    }
    // Empty braces {}: no space
    if pk == L_CURLY && nk == R_CURLY {
        return "";
    }

    // --- Space rules ---

    // After , and ;
    if matches!(pk, COMMA | SEMICOLON) {
        return " ";
    }
    // After : (but not if this is the second : of :: in TOKEN_TREE)
    if pk == COLON {
        if prev_non_whitespace_token(prev).map(|t| t.kind()) == Some(COLON) {
            return "";
        }
        return " ";
    }
    // Default type param T=Value: no space around =
    if (pk == EQ || nk == EQ)
        && (prev.parent().map(|p| p.kind()) == Some(TYPE_PARAM)
            || next.parent().map(|p| p.kind()) == Some(TYPE_PARAM))
    {
        return "";
    }
    // Assignment = and compound assignments
    if pk == EQ || nk == EQ {
        return " ";
    }
    if is_compound_assign(pk) || is_compound_assign(nk) {
        return " ";
    }
    // Comparison == != <= >=
    if matches!(pk, EQ2 | NEQ | LTEQ | GTEQ) || matches!(nk, EQ2 | NEQ | LTEQ | GTEQ) {
        return " ";
    }
    // Logical && ||
    if matches!(pk, AMP2 | PIPE2) || matches!(nk, AMP2 | PIPE2) {
        return " ";
    }
    // Fat/thin arrows => ->
    if matches!(pk, FAT_ARROW | THIN_ARROW) || matches!(nk, FAT_ARROW | THIN_ARROW) {
        return " ";
    }
    // Shift operators << >>
    if matches!(pk, SHL | SHR) || matches!(nk, SHL | SHR) {
        return " ";
    }
    // Shift operators as two separate angle tokens in TOKEN_TREE: << >>
    if is_in_token_tree(prev) {
        if pk == L_ANGLE && nk == L_ANGLE {
            return "";
        }
        if pk == R_ANGLE && nk == R_ANGLE {
            return "";
        }
    }
    // < > as comparison operators (generics already handled above)
    // Inside TOKEN_TREE, < > are ambiguous — only add spaces when both sides are
    // value-like (likely comparison), not when adjacent to type-like tokens.
    if pk == L_ANGLE && !is_in_token_tree(prev) {
        return " ";
    }
    if nk == L_ANGLE && !is_in_token_tree(next)  {
        return " ";
    }
    // Space around << (two L_ANGLE) in TOKEN_TREE
    if nk == L_ANGLE && is_in_token_tree(next)
        && next.next_token().map(|t| t.kind()) == Some(L_ANGLE)
        && is_word_like(pk)
    {
        return " ";
    }
    if pk == L_ANGLE && is_in_token_tree(prev)
        && prev_non_whitespace_token(prev).map(|t| t.kind()) == Some(L_ANGLE)
    {
        return " ";
    }
    if nk == R_ANGLE && !is_in_token_tree(next) {
        return " ";
    }
    // > as prev: space if comparison or followed by word-like token
    if pk == R_ANGLE && !is_in_token_tree(prev)
        && (!is_in_generic_context(prev) || is_word_like(nk)) {
            return " ";
        }
    // Space around >> (two R_ANGLE) in TOKEN_TREE
    if nk == R_ANGLE && is_in_token_tree(next)
        && next.next_token().map(|t| t.kind()) == Some(R_ANGLE)
        && is_word_like(pk)
    {
        return " ";
    }
    if pk == R_ANGLE && is_in_token_tree(prev)
        && prev_non_whitespace_token(prev).map(|t| t.kind()) == Some(R_ANGLE)
    {
        return " ";
    }
    // => and -> in TOKEN_TREE (R_ANGLE preceded by EQ or MINUS): space after
    if pk == R_ANGLE && is_in_token_tree(prev)
        && prev_non_whitespace_token(prev)
            .map(|t| matches!(t.kind(), EQ | MINUS))
            .unwrap_or(false)
    {
        return " ";
    }
    // Binary arithmetic/bitwise (non-unary)
    if is_binary_op_token(pk) && !is_unary(prev) {
        return " ";
    }
    if is_binary_op_token(nk) && !is_unary(next) {
        return " ";
    }
    // Braces: space around
    if nk == L_CURLY {
        return " ";
    }
    if pk == L_CURLY {
        return " ";
    }
    if nk == R_CURLY {
        return " ";
    }
    if pk == R_CURLY {
        return " ";
    }
    // Keywords: space around (but not after < or before > in TOKEN_TREE — macro type syntax)
    if is_keyword_kind(pk) && !(nk == R_ANGLE && is_in_token_tree(next)) {
        return " ";
    }
    if is_keyword_kind(nk) && !(pk == L_ANGLE && is_in_token_tree(prev)) {
        return " ";
    }
    // BANG before word-like: space (e.g., macro_rules! my_macro)
    if pk == BANG && is_word_like(nk) {
        return " ";
    }
    // Comments: space around
    if pk == COMMENT || nk == COMMENT {
        return " ";
    }
    // Lifetime before type-starting delimiter: 'a [T], 'a (T)
    if pk == LIFETIME_IDENT && matches!(nk, L_BRACK | L_PAREN) {
        return " ";
    }
    // Word-like tokens: space between
    if is_word_like(pk) && is_word_like(nk) {
        return " ";
    }
    ""
}


// ========================
// === is_in_token_tree ===
// ========================

fn is_macro_repetition_op(token: &SyntaxToken) -> bool {
    matches!(token.kind(), STAR | PLUS)
        && is_in_token_tree(token)
        && prev_non_whitespace_token(token)
            .map(|t| matches!(t.kind(), R_PAREN | COMMA))
            .unwrap_or(false)
}

fn is_in_token_tree(token: &SyntaxToken) -> bool {
    token.parent().map(|p| p.kind()) == Some(TOKEN_TREE)
}


// ===============================
// === is_macro_metavar_colon ===
// ===============================

fn is_macro_metavar_colon(colon: &SyntaxToken) -> bool {
    let prev = match prev_non_whitespace_token(colon) {
        Some(t) => t,
        None => return false,
    };
    if !matches!(prev.kind(), IDENT) && !is_keyword_kind(prev.kind()) {
        return false;
    }
    prev_non_whitespace_token(&prev)
        .map(|t| t.kind() == DOLLAR)
        .unwrap_or(false)
}


// ==========================
// === is_turbofish_angle ===
// ==========================

fn is_double_colon(token: &SyntaxToken) -> bool {
    token.kind() == COLON2
        || (token.kind() == COLON
            && prev_non_whitespace_token(token)
                .map(|t| t.kind() == COLON)
                .unwrap_or(false))
}

fn is_turbofish_angle(token: &SyntaxToken) -> bool {
    if token.parent().map(|p| p.kind()) != Some(TOKEN_TREE) {
        return false;
    }
    match token.kind() {
        L_ANGLE => prev_non_whitespace_token(token)
            .map(|t| is_double_colon(&t))
            .unwrap_or(false),
        R_ANGLE => {
            let mut depth = 1;
            let mut tok = token.prev_token();
            while let Some(t) = tok {
                match t.kind() {
                    R_ANGLE => depth += 1,
                    L_ANGLE if depth > 1 => depth -= 1,
                    L_ANGLE => {
                        return prev_non_whitespace_token(&t)
                            .map(|p| is_double_colon(&p))
                            .unwrap_or(false);
                    }
                    WHITESPACE => {}
                    _ => {}
                }
                tok = t.prev_token();
            }
            false
        }
        _ => false,
    }
}


// ================
// === is_unary ===
// ================

fn is_unary(token: &SyntaxToken) -> bool {
    if !matches!(token.kind(), MINUS | STAR | AMP | BANG) {
        return false;
    }
    match token.parent() {
        Some(parent) => {
            if matches!(
                parent.kind(),
                PREFIX_EXPR | REF_EXPR | REF_PAT | REF_TYPE | PTR_TYPE | SELF_PARAM
            ) {
                return true;
            }
            if parent.kind() == TOKEN_TREE {
                return is_likely_unary_in_token_tree(token);
            }
            false
        }
        None => false,
    }
}

fn is_likely_unary_in_token_tree(token: &SyntaxToken) -> bool {
    match prev_non_whitespace_token(token) {
        Some(prev) => matches!(
            prev.kind(),
            L_PAREN
                | L_BRACK
                | L_CURLY
                | COMMA
                | SEMICOLON
                | EQ
                | COLON
                | FAT_ARROW
                | THIN_ARROW
                | BANG
                | PIPE
                | PLUSEQ
                | MINUSEQ
                | STAREQ
                | SLASHEQ
                | PERCENTEQ
                | AMPEQ
                | PIPEEQ
                | CARETEQ
                | SHLEQ
                | SHREQ
                | RETURN_KW
                | MOVE_KW
                | EQ2
                | NEQ
                | LTEQ
                | GTEQ
                | L_ANGLE
                | R_ANGLE
                | AMP2
                | PIPE2
                | PLUS
                | MINUS
                | STAR
                | SLASH
                | PERCENT
                | AMP
                | CARET
                | SHL
                | SHR
        ),
        None => true,
    }
}

fn prev_non_whitespace_token(token: &SyntaxToken) -> Option<SyntaxToken> {
    let mut tok = token.prev_token();
    while let Some(t) = tok {
        if t.kind() != WHITESPACE {
            return Some(t);
        }
        tok = t.prev_token();
    }
    None
}


// ==============================
// === is_in_generic_context ===
// ==============================

fn is_in_generic_context(token: &SyntaxToken) -> bool {
    match token.parent() {
        Some(parent) => matches!(
            parent.kind(),
            GENERIC_ARG_LIST | GENERIC_PARAM_LIST | TYPE_BOUND_LIST | TYPE_ANCHOR
        ),
        None => false,
    }
}


// ==============================
// === is_in_closure_params ===
// ==============================

fn is_in_closure_params(token: &SyntaxToken) -> bool {
    match token.parent() {
        Some(parent) => parent.kind() == PARAM_LIST,
        None => false,
    }
}


// =================================
// === is_opening_closure_pipe ===
// =================================

fn is_opening_closure_pipe(token: &SyntaxToken) -> bool {
    token.prev_sibling_or_token().is_none()
}


// ==========================
// === is_compound_assign ===
// ==========================

fn is_compound_assign(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        PLUSEQ | MINUSEQ | STAREQ | SLASHEQ | PERCENTEQ | AMPEQ | PIPEEQ | CARETEQ | SHLEQ
            | SHREQ
    )
}


// ==========================
// === is_binary_op_token ===
// ==========================

fn is_binary_op_token(kind: SyntaxKind) -> bool {
    matches!(kind, PLUS | MINUS | STAR | SLASH | PERCENT | AMP | PIPE | CARET)
}


// ========================
// === is_keyword_kind ===
// ========================

fn is_keyword_kind(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        AS_KW
            | ASYNC_KW
            | AWAIT_KW
            | BOX_KW
            | BREAK_KW
            | CONST_KW
            | CONTINUE_KW
            | CRATE_KW
            | DYN_KW
            | ELSE_KW
            | ENUM_KW
            | EXTERN_KW
            | FALSE_KW
            | FN_KW
            | FOR_KW
            | IF_KW
            | IMPL_KW
            | IN_KW
            | LET_KW
            | LOOP_KW
            | MACRO_KW
            | MATCH_KW
            | MOD_KW
            | MOVE_KW
            | MUT_KW
            | PUB_KW
            | REF_KW
            | RETURN_KW
            | SELF_KW
            | SELF_TYPE_KW
            | STATIC_KW
            | STRUCT_KW
            | SUPER_KW
            | TRAIT_KW
            | TRUE_KW
            | TYPE_KW
            | UNSAFE_KW
            | USE_KW
            | WHERE_KW
            | WHILE_KW
            | YIELD_KW
            | AUTO_KW
            | DEFAULT_KW
            | SAFE_KW
            | UNION_KW
    )
}


// ====================
// === is_word_like ===
// ====================

fn is_word_like(kind: SyntaxKind) -> bool {
    kind.is_literal() || matches!(kind, IDENT | LIFETIME_IDENT | UNDERSCORE) || is_keyword_kind(kind)
}


// ===============================
// === ensure_trailing_newline ===
// ===============================

fn ensure_trailing_newline(mut s: String) -> String {
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

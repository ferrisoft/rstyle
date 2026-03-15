use ra_ap_syntax::AstNode;
use ra_ap_syntax::Edition;
use ra_ap_syntax::SourceFile;
use ra_ap_syntax::SyntaxKind::*;


// ========================
// === sort_derive_args ===
// ========================

pub(crate) fn sort_derive_args(source: &str) -> String {
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

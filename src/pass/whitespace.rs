use ra_ap_syntax::AstNode;
use ra_ap_syntax::Edition;
use ra_ap_syntax::SourceFile;
use ra_ap_syntax::SyntaxKind::*;
use ra_ap_syntax::SyntaxToken;

use crate::config::Config;
use crate::syntax::indentation::emit_newline_whitespace;
use crate::syntax::spacing::compute_spacing;
use crate::syntax::string::reindent_string_token;


// =========================
// === format_whitespace ===
// =========================

pub(crate) fn format_whitespace(source: &str, config: &Config) -> String {
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
            reindent_string_token(&mut output, token, config);
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
                emit_newline_whitespace(&mut output, &ws_text, next, config);
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

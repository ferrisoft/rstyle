use ra_ap_syntax::SyntaxKind;
use ra_ap_syntax::SyntaxKind::*;
use ra_ap_syntax::SyntaxToken;


// =======================
// === compute_spacing ===
// =======================

#[inline(always)]
/// Decides the horizontal whitespace between two adjacent non-whitespace tokens on the same line.
/// Returns `""` (no space), `" "` (one space), based on Rust syntax rules.
pub(crate) fn compute_spacing(prev: &SyntaxToken, next: &SyntaxToken) -> &'static str {
    let pk = prev.kind();
    let nk = next.kind();
    no_space_rule(prev, next, pk, nk)
        .or_else(|| space_rule(prev, next, pk, nk))
        .unwrap_or("")
}

fn no_space_rule(
    prev: &SyntaxToken,
    next: &SyntaxToken,
    pk: SyntaxKind,
    nk: SyntaxKind,
) -> Option<&'static str> {
    no_space_for_punctuation(pk, nk)
        .or_else(|| no_space_for_context(prev, next, pk, nk))
}

fn no_space_for_punctuation(pk: SyntaxKind, nk: SyntaxKind) -> Option<&'static str> {
    if pk == COLON2 || nk == COLON2 { return Some(""); }
    if pk == COLON && nk == COLON { return Some(""); }
    if pk == DOT || nk == DOT { return Some(""); }
    if matches!(pk, DOT2 | DOT2EQ | DOT3) || matches!(nk, DOT2 | DOT2EQ | DOT3) { return Some(""); }
    if matches!(pk, L_PAREN | L_BRACK) { return Some(""); }
    if matches!(nk, R_PAREN | R_BRACK) { return Some(""); }
    if matches!(nk, COMMA | SEMICOLON) { return Some(""); }
    if nk == QUESTION { return Some(""); }
    if pk == POUND { return Some(""); }
    if nk == COLON { return Some(""); }
    if pk == AT || nk == AT { return Some(""); }
    if pk == L_CURLY && nk == R_CURLY { return Some(""); }
    if nk == BANG && pk == IDENT { return Some(""); }
    if pk == BANG && matches!(nk, L_PAREN | L_BRACK) { return Some(""); }
    if pk == IDENT && nk == L_PAREN { return Some(""); }
    if pk == R_PAREN && nk == L_PAREN { return Some(""); }
    if nk == L_BRACK && matches!(pk, IDENT | R_PAREN | R_BRACK | R_ANGLE) { return Some(""); }
    if pk == AMP && nk == LIFETIME_IDENT { return Some(""); }
    if pk == PIPE && nk == PIPE { return Some(""); }
    None
}

fn no_space_for_context(
    prev: &SyntaxToken,
    next: &SyntaxToken,
    pk: SyntaxKind,
    nk: SyntaxKind,
) -> Option<&'static str> {
    if pk == DOLLAR && is_in_token_tree(prev) { return Some(""); }
    if nk == COLON && is_in_token_tree(next) && is_macro_metavar_colon(next) { return Some(""); }
    if pk == COLON && is_in_token_tree(prev) && is_macro_metavar_colon(prev) { return Some(""); }
    if matches!(nk, STAR | PLUS) && is_in_token_tree(next) && matches!(pk, R_PAREN | COMMA) {
        return Some("");
    }
    if pk == AMP && nk == L_ANGLE && is_in_token_tree(prev) { return Some(""); }
    if matches!(pk, STAR | PLUS) && nk == R_ANGLE && is_in_token_tree(prev)
        && prev_non_whitespace_token(prev).is_none_or(|t| t.kind() != R_ANGLE)
    {
        return Some("");
    }
    if matches!(pk, EQ | MINUS) && nk == R_ANGLE && is_in_token_tree(prev) { return Some(""); }
    if pk == PUB_KW && nk == L_PAREN
        && prev.parent().map(|p| p.kind()) == Some(VISIBILITY) { return Some(""); }
    if pk == FN_KW && nk == L_PAREN
        && prev.parent().map(|p| p.kind()) == Some(FN_PTR_TYPE) { return Some(""); }
    if matches!(pk, MINUS | STAR | AMP | BANG) && is_unary(prev) && !is_macro_repetition_op(prev) {
        return Some("");
    }
    if pk == L_ANGLE && (is_in_generic_context(prev) || is_turbofish_angle(prev)) { return Some(""); }
    if nk == R_ANGLE && (is_in_generic_context(next) || is_turbofish_angle(next)) { return Some(""); }
    if nk == L_ANGLE && is_in_generic_context(next)
        && next.parent().map(|p| p.kind()) != Some(TYPE_ANCHOR) { return Some(""); }
    if pk == R_ANGLE && nk == L_PAREN
        && prev.parent().map(|p| p.kind()) == Some(TOKEN_TREE)
        && is_turbofish_angle(prev) { return Some(""); }
    if pk == PIPE && is_in_closure_params(prev) && is_opening_closure_pipe(prev) { return Some(""); }
    if nk == PIPE && is_in_closure_params(next) && !is_opening_closure_pipe(next) { return Some(""); }
    None
}

fn space_rule(
    prev: &SyntaxToken,
    next: &SyntaxToken,
    pk: SyntaxKind,
    nk: SyntaxKind,
) -> Option<&'static str> {
    space_for_operators(prev, next, pk, nk)
        .or_else(|| space_for_structure(prev, next, pk, nk))
}

fn space_for_operators(
    prev: &SyntaxToken,
    next: &SyntaxToken,
    pk: SyntaxKind,
    nk: SyntaxKind,
) -> Option<&'static str> {
    if matches!(pk, COMMA | SEMICOLON) { return Some(" "); }
    if pk == COLON {
        if prev_non_whitespace_token(prev).map(|t| t.kind()) == Some(COLON) { return Some(""); }
        return Some(" ");
    }
    if (pk == EQ || nk == EQ)
        && (prev.parent().map(|p| p.kind()) == Some(TYPE_PARAM)
            || next.parent().map(|p| p.kind()) == Some(TYPE_PARAM))
    {
        return Some("");
    }
    if pk == EQ || nk == EQ { return Some(" "); }
    if is_compound_assign(pk) || is_compound_assign(nk) { return Some(" "); }
    if matches!(pk, EQ2 | NEQ | LTEQ | GTEQ) || matches!(nk, EQ2 | NEQ | LTEQ | GTEQ) { return Some(" "); }
    if matches!(pk, AMP2 | PIPE2) || matches!(nk, AMP2 | PIPE2) { return Some(" "); }
    if matches!(pk, FAT_ARROW | THIN_ARROW) || matches!(nk, FAT_ARROW | THIN_ARROW) { return Some(" "); }
    if matches!(pk, SHL | SHR) || matches!(nk, SHL | SHR) { return Some(" "); }
    if is_binary_op_token(pk) && !is_unary(prev) { return Some(" "); }
    if is_binary_op_token(nk) && !is_unary(next) { return Some(" "); }
    None
}

fn space_for_structure(
    prev: &SyntaxToken,
    next: &SyntaxToken,
    pk: SyntaxKind,
    nk: SyntaxKind,
) -> Option<&'static str> {
    if is_in_token_tree(prev) {
        if pk == L_ANGLE && nk == L_ANGLE { return Some(""); }
        if pk == R_ANGLE && nk == R_ANGLE { return Some(""); }
    }
    if pk == L_ANGLE && !is_in_token_tree(prev) { return Some(" "); }
    if nk == L_ANGLE && !is_in_token_tree(next) { return Some(" "); }
    if nk == L_ANGLE && is_in_token_tree(next)
        && next.next_token().map(|t| t.kind()) == Some(L_ANGLE) && is_word_like(pk)
    {
        return Some(" ");
    }
    if pk == L_ANGLE && is_in_token_tree(prev)
        && prev_non_whitespace_token(prev).map(|t| t.kind()) == Some(L_ANGLE)
    {
        return Some(" ");
    }
    if nk == R_ANGLE && !is_in_token_tree(next) { return Some(" "); }
    if pk == R_ANGLE && !is_in_token_tree(prev)
        && (!is_in_generic_context(prev) || is_word_like(nk)) { return Some(" "); }
    if nk == R_ANGLE && is_in_token_tree(next)
        && next.next_token().map(|t| t.kind()) == Some(R_ANGLE) && is_word_like(pk)
    {
        return Some(" ");
    }
    if pk == R_ANGLE && is_in_token_tree(prev)
        && prev_non_whitespace_token(prev).map(|t| t.kind()) == Some(R_ANGLE)
    {
        return Some(" ");
    }
    if pk == R_ANGLE && is_in_token_tree(prev)
        && prev_non_whitespace_token(prev).is_some_and(|t| matches!(t.kind(), EQ | MINUS))
    {
        return Some(" ");
    }
    if matches!(pk, L_CURLY | R_CURLY) || matches!(nk, L_CURLY | R_CURLY) { return Some(" "); }
    if is_keyword_kind(pk) && !(nk == R_ANGLE && is_in_token_tree(next)) { return Some(" "); }
    if is_keyword_kind(nk) && !(pk == L_ANGLE && is_in_token_tree(prev)) { return Some(" "); }
    if pk == BANG && is_word_like(nk) { return Some(" "); }
    if pk == COMMENT || nk == COMMENT { return Some(" "); }
    if pk == LIFETIME_IDENT && matches!(nk, L_BRACK | L_PAREN) { return Some(" "); }
    if is_word_like(pk) && is_word_like(nk) { return Some(" "); }
    None
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

pub(crate) fn prev_non_whitespace_token(token: &SyntaxToken) -> Option<SyntaxToken> {
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


// ========================
// === is_in_token_tree ===
// ========================

fn is_macro_repetition_op(token: &SyntaxToken) -> bool {
    matches!(token.kind(), STAR | PLUS)
        && is_in_token_tree(token)
        && prev_non_whitespace_token(token)
            .is_some_and(|t| matches!(t.kind(), R_PAREN | COMMA))
}

fn is_in_token_tree(token: &SyntaxToken) -> bool {
    token.parent().map(|p| p.kind()) == Some(TOKEN_TREE)
}


// ===============================
// === is_macro_metavar_colon ===
// ===============================

fn is_macro_metavar_colon(colon: &SyntaxToken) -> bool {
    let Some(prev) = prev_non_whitespace_token(colon) else { return false };
    if !matches!(prev.kind(), IDENT) && !is_keyword_kind(prev.kind()) {
        return false;
    }
    prev_non_whitespace_token(&prev)
        .is_some_and(|t| t.kind() == DOLLAR)
}


// ==========================
// === is_turbofish_angle ===
// ==========================

fn is_double_colon(token: &SyntaxToken) -> bool {
    token.kind() == COLON2
        || (token.kind() == COLON
            && prev_non_whitespace_token(token)
                .is_some_and(|t| t.kind() == COLON))
}

fn is_turbofish_angle(token: &SyntaxToken) -> bool {
    if token.parent().map(|p| p.kind()) != Some(TOKEN_TREE) {
        return false;
    }
    match token.kind() {
        L_ANGLE => prev_non_whitespace_token(token)
            .is_some_and(|t| is_double_colon(&t)),
        R_ANGLE => {
            let mut depth = 1;
            let mut tok = token.prev_token();
            while let Some(t) = tok {
                match t.kind() {
                    R_ANGLE => depth += 1,
                    L_ANGLE if depth > 1 => depth -= 1,
                    L_ANGLE => {
                        return prev_non_whitespace_token(&t)
                            .is_some_and(|p| is_double_colon(&p));
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

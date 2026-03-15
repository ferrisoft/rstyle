use ra_ap_syntax::SyntaxKind;
use ra_ap_syntax::SyntaxKind::*;
use ra_ap_syntax::SyntaxToken;


// =======================
// === compute_spacing ===
// =======================

#[inline(always)]
pub(crate) fn compute_spacing(prev: &SyntaxToken, next: &SyntaxToken) -> &'static str {
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

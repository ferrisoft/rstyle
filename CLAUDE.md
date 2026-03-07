# rust_formatter

A Rust code formatter that respects line breaks. Unlike rustfmt, this formatter
does NOT break or join lines. It only normalizes horizontal spacing, indentation,
and trailing whitespace.

## Build & Test
```
cargo build
cargo test
cargo clippy
```

## Architecture
- `src/main.rs` - CLI entry point (clap). Supports stdin, `--write`, `--check`.
- `src/lib.rs` - Re-exports `formatter` module.
- `src/formatter.rs` - Core formatting logic using `ra_ap_syntax` lossless CST.

## Key Design Decisions
- Uses `ra_ap_syntax` (rust-analyzer's parser) for lossless CST parsing.
- Iterates tokens in document order, applying spacing rules between adjacent
  non-whitespace tokens.
- Line breaks are always preserved; only horizontal spacing is modified.
- Indentation is computed from the CST tree structure (not preserved from source).
  Indent-increasing nodes: STMT_LIST, ITEM_LIST, ASSOC_ITEM_LIST, MATCH_ARM_LIST,
  RECORD_FIELD_LIST, RECORD_EXPR_FIELD_LIST, RECORD_PAT_FIELD_LIST, VARIANT_LIST,
  USE_TREE_LIST, EXTERN_ITEM_LIST, and TOKEN_TREE (only `{}` and `[]` delimited).
  Delimiter tokens ({, }, [, ]) stay at the parent indent level.
- Continuation indent: lines starting with `.` (method chains) get +1 indent level.
- Multiline string literals: content is re-indented to match surrounding code's
  indent level (content at indent+1, closing quote at indent). Relative indentation
  within strings is preserved.
- Derive args sorted alphabetically: `#[derive(Debug, Clone)]` → `#[derive(Clone, Debug)]`.
- Imports: flattened to one-per-line, sorted alphabetically, grouped with blank line
  separators. Group order: mod declarations → star imports → foreign → crate → pub re-exports.
- Trailing whitespace removed, trailing newline ensured.

use std::fs;
use std::path::Path;


// ====================
// === run_fixture ===
// ====================

fn run_fixture(name: &str) {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let input_path = fixtures_dir.join(format!("{name}.input.rs"));
    let expected_path = fixtures_dir.join(format!("{name}.expected.rs"));
    let input = fs::read_to_string(&input_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", input_path.display()));
    let expected = if expected_path.exists() {
        fs::read_to_string(&expected_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", expected_path.display()))
    } else {
        input.clone()
    };
    let actual = rust_formatter::formatter::format_source(&input);
    if actual != expected {
        panic!(
            "Fixture '{name}' failed.\n\n--- expected ---\n{expected}\n--- actual ---\n{actual}\n--- end ---"
        );
    }
}


// ======================
// === fixture_tests! ===
// ======================

macro_rules! fixture_tests {
    ($($name:ident),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                run_fixture(stringify!($name));
            }
        )*
    };
}

fixture_tests! {
    // Spacing
    space_around_eq,
    space_after_comma,
    space_after_colon,
    no_space_around_double_colon,
    no_space_around_dot,
    no_space_inside_parens,
    binary_operators,
    unary_minus,
    reference,
    no_space_in_generics,
    comparison_operators,
    logical_operators,
    compound_assignment,
    fat_arrow,
    thin_arrow,
    question_mark,
    empty_braces,
    pub_crate,
    closure,
    ref_mut,
    qualified_path_no_spaces,
    lifetime_before_bracket,
    lifetime_before_bracket_nested,
    type_param_default_no_spaces,

    // Trailing whitespace & newlines
    trailing_whitespace_removed,
    trailing_newline_added,

    // Indentation
    tabs_to_spaces,
    indent_fn_body,
    indent_nested_blocks,
    indent_impl_body,
    indent_enum_variants,
    indent_struct_fields,
    indent_match_arms,
    indent_fixes_wrong_indent,
    indent_macro_rules_fat_arrow,
    indent_fn_params_multiline,
    expand_long_fn_params,

    // Strings
    reindent_multiline_string,
    reindent_multiline_string_preserves_relative_indent,
    string_indent_in_chain,

    // Line breaks
    preserves_line_breaks,
    multiple_blank_lines_preserved,

    collapse_brace_to_prev_line,
    where_on_same_line,
    where_long_splits_bounds,

    // Control flow
    for_loop,
    if_else,
    match_arms_multiline,

    // Macros
    macro_call,
    macro_path_separator,
    macro_reference,
    macro_empty_closure,
    macro_braces,
    turbofish_in_macro,
    macro_angle_brackets_no_spaces,
    macro_pattern_spacing,
    macro_repetition_space_before_brace,
    macro_shift_operators,
    macro_arrows,
    macro_ref_angle_no_spaces,
    macro_call_string_indent,

    // Structs & impls
    struct_literal,
    expand_long_struct_literal,
    impl_block,
    attribute,
    already_formatted_sample,

    // Imports
    use_statement,
    flatten_use_tree_list,
    sort_imports_alphabetically,
    group_imports,
    flatten_and_sort_imports,
    import_groups_with_mods,
    star_imports_group,
    use_hoisted_to_top,
    use_inside_mod_stays,

    // Derives
    sort_derive,

    // Chains
    chain_short_stays_single_line,
    chain_collapse_when_fits,
    chain_break_long_line,
    chain_preserve_existing_break_and_break_long,
    chain_field_access_not_broken,
    chain_user_field_break_preserved,
    chain_closure_body_indent,
    chain_already_correct,
    chain_preserve_break_after_multiline_closure,
    expand_long_inline_closure,

    // Continuation indent
    continuation_indent_dot,

    // Section headers
    section_header_border_length,
    section_header_blank_lines,
    section_header_start_of_file,

    // Doc comments
    doc_comment_reflow_long,
    doc_comment_join_short,
    doc_comment_preserve_code_block,
    doc_comment_inner,
}

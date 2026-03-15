use ra_ap_syntax::AstNode;
use ra_ap_syntax::Edition;
use ra_ap_syntax::SourceFile;
use ra_ap_syntax::SyntaxKind::*;
use ra_ap_syntax::SyntaxNode;


// ==========================
// === hoist_late_imports ===
// ==========================

pub(crate) fn hoist_late_imports(source: &str) -> String {
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
pub(crate) enum ImportGroup {
    Mod,
    Star,
    Foreign,
    Crate,
    PubReexport,
}

pub(crate) fn sort_and_group_imports(source: &str) -> String {
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

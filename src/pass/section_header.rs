use crate::formatter::leading_whitespace;


// ================================
// === format_section_headers ===
// ================================

/// Normalizes `// === Name ===` section headers: fixes border length, ensures consistent blank
/// lines around each header, and preserves indentation.
pub(crate) fn format_section_headers(source: &str) -> String {
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
                && result.last().is_some_and(|l| l.trim().is_empty())
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

pub(crate) fn is_section_border(line: &str) -> bool {
    line.starts_with("// ") && line.len() > 6 && line[3..].chars().all(|c| c == '=')
}

fn is_section_middle(line: &str) -> bool {
    line.starts_with("// === ") && line.ends_with(" ===")
}

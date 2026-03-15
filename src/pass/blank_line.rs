use crate::pass::section_header::is_section_border;


// ============================
// === collapse_blank_lines ===
// ============================

pub(crate) fn collapse_blank_lines(source: &str) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut result: Vec<&str> = Vec::new();
    let mut blank_count: usize = 0;
    for line in &lines {
        if line.trim().is_empty() {
            blank_count += 1;
        } else {
            let max_blanks = if is_section_border(line.trim()) { 2 } else { 1 };
            result.extend(std::iter::repeat_n("", blank_count.min(max_blanks)));
            blank_count = 0;
            result.push(line);
        }
    }
    result.extend(std::iter::repeat_n("", blank_count.min(1)));
    result.join("\n")
}


// ===============================
// === ensure_trailing_newline ===
// ===============================

pub(crate) fn ensure_trailing_newline(mut s: String) -> String {
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

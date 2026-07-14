// Responsibility: repo-file-extract-css-imports
use crate::repo::css_import_re;
use std::collections::BTreeSet;

pub(crate) fn extract_css_import_specs(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut in_block_comment = false;
    for line in text.lines() {
        let visible = css_line_without_block_comments(line, &mut in_block_comment);
        let Some(cap) = css_import_re().captures(&visible) else {
            continue;
        };
        if let Some(spec) = cap.name("spec") {
            out.insert(spec.as_str().trim().to_string());
        }
    }
    out
}

pub(crate) fn css_line_without_block_comments(line: &str, in_block_comment: &mut bool) -> String {
    let mut out = String::new();
    let mut rest = line;
    loop {
        if *in_block_comment {
            if let Some(end) = rest.find("*/") {
                *in_block_comment = false;
                rest = &rest[end + 2..];
            } else {
                break;
            }
        } else if let Some(start) = rest.find("/*") {
            out.push_str(&rest[..start]);
            rest = &rest[start + 2..];
            *in_block_comment = true;
        } else {
            out.push_str(rest);
            break;
        }
    }
    out
}

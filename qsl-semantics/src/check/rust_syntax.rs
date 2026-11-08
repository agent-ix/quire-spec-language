// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-450 "Rust spellings": the syntax of a `rust-path` a
//! `guarantee` declaration's Rust strings must have.
//!
//! A Rust string's content is its source bytes between the quotes. A
//! `rust-path` is one or more Rust identifiers separated by `::`; a Rust
//! identifier is `IDENTIFIER` of the Rust Reference for the 2021 edition,
//! in Unicode Normalization Form C. These are lexical keys: this module
//! resolves none of them.

use ix_trace_rs::string_edge;
use unicode_normalization::is_nfc;

/// The 2021 edition's strict and reserved keywords.
const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
];

/// The raw identifiers `r#` may not introduce.
const NOT_RAW: &[&str] = &["crate", "self", "super", "Self", "_"];

/// The first malformed segment of a path: its text and the byte offset of
/// its start within the content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathDefect {
    /// The malformed segment, as written (empty for an empty segment).
    pub segment: String,
    /// The byte offset of the segment within the path's content.
    pub offset: usize,
}

/// Whether `text` has the shape of a Rust identifier, keyword or not.
#[string_edge]
fn identifier_shaped(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || unicode_ident::is_xid_start(first))
        && chars.all(unicode_ident::is_xid_continue)
        && is_nfc(text)
}

/// Whether `segment` is a Rust identifier (QSpec FR-450).
#[string_edge]
fn is_identifier(segment: &str) -> bool {
    if let Some(raw) = segment.strip_prefix("r#") {
        return identifier_shaped(raw) && !NOT_RAW.contains(&raw);
    }
    identifier_shaped(segment) && segment != "_" && !KEYWORDS.contains(&segment)
}

/// The first segment of `content` that is not a Rust identifier, or `None`
/// when `content` is a `rust-path`.
#[string_edge]
pub fn path_defect(content: &str) -> Option<PathDefect> {
    let mut offset = 0;
    for segment in content.split("::") {
        if !is_identifier(segment) {
            return Some(PathDefect {
                segment: segment.to_owned(),
                offset,
            });
        }
        offset += segment.len() + 2;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_defects() {
        assert_eq!(path_defect("counter::Counter::new"), None);
        assert_eq!(path_defect("r#type::x"), None);
        assert_eq!(
            path_defect("a::9lives"),
            Some(PathDefect {
                segment: "9lives".into(),
                offset: 3
            })
        );
        assert_eq!(path_defect("a::").map(|d| d.segment), Some(String::new()));
        assert!(path_defect("crate::x").is_some());
        assert!(path_defect("r#self").is_some());
        assert!(path_defect("a\\b").is_some());
        assert!(path_defect("").is_some());
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-783 S1: the grammar half of the one-source-text rule (FR-362, TC-1026).
//! A source selects a profile by identity alone.

use ix_trace_rs::trace;
use qsl_cst::ParsedSource;
use qsl_forms::{build_unit, FormsCause};
use qsl_foundation::{Code, SourceIdentity};

const PREDICATE: &str = "predicate P using v(x: Boolean): Boolean { x }\n";

fn parse(text: &str) -> ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit")
}

/// The grammar half of TC-1026 step 2: a `profile` line that carries `version` or
/// `digest` after the identity recovers with `invalid_syntax` at that token,
/// records no profile selection, and `build_unit` refuses the source with the
/// `invalid_syntax` cause.
#[trace("TC-1026", "FR-362")]
#[test]
fn a_profile_line_names_only_its_identity() {
    let hex = "d".repeat(64);
    for (profile_tail, first_extra) in [
        (" version \"1\"".to_owned(), "version"),
        (format!(" digest \"sha256:{hex}\""), "digest"),
        (format!(" version \"1\" digest \"sha256:{hex}\""), "version"),
    ] {
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\"{profile_tail};\n\
             {PREDICATE}"
        );
        let parsed = parse(&text);
        assert!(!parsed.is_admissible(), "{text}");
        assert!(parsed.selections().profiles.is_empty(), "{text}");
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.code, Code::InvalidSyntax, "{text}");
        assert_eq!(
            diagnostic.byte_span().expect("located").start,
            text.find(first_extra)
                .expect("the extra token is in the unit"),
            "{text}"
        );
        match build_unit(&parsed) {
            Err(refusal) => {
                assert_eq!(refusal.cause, FormsCause::RecoveringCst, "{text}");
                assert_eq!(refusal.cause.catalog_code(), Code::InvalidSyntax, "{text}");
            }
            other => panic!("RecoveringCst, not {other:?}"),
        }
    }
}

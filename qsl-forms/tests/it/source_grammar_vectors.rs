// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-783 S1: the grammar half of the one-source-text rule (FR-362, TC-1026).
//! A source selects a profile by identity alone, and the `relation` and `hyper`
//! clauses, which have no S2 dispatch entry, refuse the whole unit.

use ix_trace_rs::trace;
use qsl_cst::ParsedSource;
use qsl_forms::{build_unit, FormsCause, ParsedUnit};
use qsl_foundation::{Code, SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";

const PREDICATE: &str = "predicate P using v(x: Boolean): Boolean { x }\n";
const INVARIANT: &str = "invariant Stable using v on M::Thing at current { true }\n";
const RELATION: &str =
    "relation Same using v over (left: M::Execution, right: M::Execution) { true }";
const HYPER: &str =
    "hyper Secret using v over traces in M::Execution bounded 2 forall trace a exists trace b { true }";

fn parse(text: &str) -> ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn build(declarations: &str) -> ParsedUnit {
    let text = format!("{HEADER}{declarations}");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"))
}

fn span_of(text: &str, needle: &str) -> Span {
    let start = text.rfind(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

/// FR-362-AC-2 (TC-1026 step 2): a `profile` line that carries `version` or
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

/// FR-362-AC-7 (TC-1026 step 5, grammar half): a unit of a `predicate` and an
/// `invariant` builds; the same unit with a `relation` or `hyper` clause is
/// refused with `NoDispatchEntry`, mapped to `unsupported_construct`, at that
/// clause, and builds no `ParsedUnit`.
#[trace("TC-1026", "FR-362")]
#[test]
fn a_relation_or_hyper_clause_refuses_the_unit_with_no_dispatch_entry() {
    let baseline = build(&format!("{PREDICATE}{INVARIANT}"));
    assert_eq!(baseline.forms().len(), 2);

    for (clause, spelling) in [(RELATION, "relation"), (HYPER, "hyper")] {
        let declarations = format!("{PREDICATE}{INVARIANT}{clause}\n");
        let text = format!("{HEADER}{declarations}");
        let parsed = parse(&text);
        assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
        match build_unit(&parsed) {
            Err(refusal) => {
                assert_eq!(
                    refusal.cause,
                    FormsCause::NoDispatchEntry {
                        spelling: spelling.to_owned()
                    }
                );
                assert_eq!(refusal.cause.catalog_code(), Code::UnsupportedConstruct);
                assert_eq!(refusal.span, Some(span_of(&text, clause)));
            }
            Ok(unit) => panic!("{spelling}: a refusal, not {unit:?}"),
        }
    }
}

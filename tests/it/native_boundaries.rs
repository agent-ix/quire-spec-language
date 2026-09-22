// SPDX-License-Identifier: AGPL-3.0-or-later
//! Parse/format round-trip boundaries and diagnostics for native syntax.
use ix_trace_rs::trace;
use quire_spec_language::format::{format, format_with_limit};
use quire_spec_language::{parse, Code, Diagnostic, Limits, Phase, SourceIdentity};

fn parse_text(text: &str) -> Result<quire_spec_language::ParsedUnit, Box<Diagnostic>> {
    parse(
        SourceIdentity {
            identity: "test:boundary".into(),
            revision: "fixture:1".into(),
        },
        "boundary.native",
        text.as_bytes(),
        Limits::default(),
    )
}

#[trace("TC-016", "FR-003-AC-4", "FR-003-AC-5", "FR-003-AC-6")]
#[test]
fn formatter_byte_ceiling_is_inclusive_and_counts_final_newline() {
    let unit = parse_text(include_str!("../fixtures/parent.native")).unwrap();
    let expected = format(&unit).unwrap();
    assert!(expected.ends_with('\n'));
    for limit in 0..expected.len() {
        let error = format_with_limit(&unit, limit).unwrap_err();
        assert_eq!(error.code, Code::ResourceExhausted, "limit {limit}");
        assert_eq!(error.phase, Phase::Format);
        assert_eq!(&error.source, unit.source().identity());
        assert!(error.span.end.byte <= unit.source().text().len());
    }
    for limit in [
        expected.len(),
        expected.len() + 1,
        Limits::default().source_bytes,
        usize::MAX,
    ] {
        assert_eq!(format_with_limit(&unit, limit).unwrap(), expected);
    }
}

#[trace("TC-016", "FR-003-AC-4", "FR-003-AC-6")]
#[test]
fn formatter_cannot_raise_the_hard_content_ceiling() {
    let header = concat!(
        "language\"ix:native\"edition\"0-draft\";profile\"state-finite/0-draft\";",
        "model M=\"test/model\"version\"1\"digest\"unresolved\";",
        "invariant Test on M::Thing at current{true}"
    );
    let ceiling = Limits::default().source_bytes;
    // Whitespace inserted around header tokens expands an exactly admitted source.
    let text = format!("{header}//{}", "x".repeat(ceiling - header.len() - 2));
    let unit = parse_text(&text).unwrap();
    assert_eq!(unit.source().text().len(), ceiling);
    for limit in [ceiling, usize::MAX] {
        let error = format_with_limit(&unit, limit).unwrap_err();
        assert_eq!(error.code, Code::ResourceExhausted);
        assert_eq!(error.phase, Phase::Format);
    }
    assert_eq!(format(&unit).unwrap_err().code, Code::ResourceExhausted);
}

#[trace("TC-018", "FR-010-AC-8")]
#[test]
fn native_diagnostic_propagates_as_an_error_and_codes_roundtrip() {
    fn caller() -> Result<(), Box<dyn std::error::Error>> {
        parse_text("language \"other\" edition \"0-draft\";")?;
        Ok(())
    }
    let error = caller().unwrap_err();
    // The public API returns Box<Diagnostic>; standard `?` preserves that type.
    let diagnostic = error.downcast_ref::<Box<Diagnostic>>().unwrap();
    assert_eq!(diagnostic.code, Code::UnknownLanguage);
    assert_eq!(diagnostic.source.identity, "test:boundary");
    assert_eq!(
        error.to_string(),
        format!("unknown_language: {}", diagnostic.message)
    );
    assert!(error.source().is_none());
    let mut seen = std::collections::BTreeSet::new();
    for &code in Code::all() {
        assert!(seen.insert(code.as_str()), "duplicate code {code}");
        assert_eq!(Code::from_code(code.as_str()), Some(code));
        assert_eq!(code.to_string(), code.as_str());
        // FR-301's exit-code ladder checks is_unsupported() before
        // is_incomplete(); that ordering is only meaningful if the two
        // categories never both claim one code.
        assert!(
            !(code.is_unsupported() && code.is_incomplete()),
            "{code} claims both unsupported and incomplete"
        );
        // command/output.rs's report() picks the highest-severity code in a
        // diagnostic bag by taking the numeric minimum of Code::exit_code()
        // over its members. That is only correct because every Code's
        // exit_code() is confined to {20, 21, 22}, where FR-301's severity
        // order (invalid > unsupported > incomplete) happens to coincide
        // with ascending numeric order. Widen this range (a Code that
        // ladders to 30, 10 or 0) and min() silently inverts; this
        // assertion is what would catch it.
        assert!(
            matches!(code.exit_code(), 20..=22),
            "{code} exit_code {} outside {{20, 21, 22}}, the range command/output.rs's min() depends on",
            code.exit_code()
        );
    }
    // Counted directly against `Code::all()` on this branch (46 pre-existing
    // variants from `main` after #143/#144/#149's FR-153 codes
    // (`ForeignReference`, `CardinalityOutOfBound`), QSL#213 S-2's
    // `DuplicateSelection`, ADR-013 O-01/QC-5), plus QSL-6's own
    // `UnsupportedDependencySelections`) rather than derived by arithmetic —
    // `Code::all()` lists exactly 47 entries, one per `pub enum Code`
    // variant, none missing and none duplicated. FR-322 I2's own envelope-
    // shape refusals (`UnknownContractVersion`, `MalformedWire`,
    // `DuplicateMember`, `UnknownMember`, `NoncanonicalWire`,
    // `DigestDomainMismatch`) are not mirrored here at all: they collapse
    // onto the existing `Code::InvalidPackage`, exactly like
    // `UnsupportedNodeTag`/`InvalidSemanticGraph` already do (no-copy rule).
    assert_eq!(seen.len(), 47);
    assert_eq!(Code::InvalidRuntimeInput.as_str(), "invalid_runtime_input");
    assert_eq!(
        Code::from_code("invalid_runtime_input"),
        Some(Code::InvalidRuntimeInput)
    );
    assert_eq!(Code::WrongSnapshot.as_str(), "wrong_snapshot");
    assert_eq!(Code::from_code("wrong_snapshot"), Some(Code::WrongSnapshot));
    assert_eq!(Code::IllTyped.as_str(), "ill_typed");
    assert_eq!(Code::UndefinedExpression.as_str(), "undefined_expression");
    assert_eq!(Phase::Check.as_str(), "check");
    for unknown in ["", "INVALID_SYNTAX", "future_code"] {
        assert_eq!(Code::from_code(unknown), None);
    }
}

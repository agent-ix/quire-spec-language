// SPDX-License-Identifier: AGPL-3.0-only
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
    let unit = parse_text(include_str!("fixtures/parent.native")).unwrap();
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
    assert!(diagnostic.related.is_empty());
    assert!(diagnostic.upstream.is_none());
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
    }
    assert_eq!(seen.len(), 18);
    assert_eq!(Code::WrongSnapshot.as_str(), "wrong_snapshot");
    assert_eq!(Code::from_code("wrong_snapshot"), Some(Code::WrongSnapshot));
    assert_eq!(Code::IllTyped.as_str(), "ill_typed");
    assert_eq!(Code::UndefinedExpression.as_str(), "undefined_expression");
    assert_eq!(Phase::Check.as_str(), "check");
    for unknown in ["", "INVALID_SYNTAX", "future_code"] {
        assert_eq!(Code::from_code(unknown), None);
    }
}

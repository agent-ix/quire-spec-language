// SPDX-License-Identifier: AGPL-3.0-or-later
//! Parse/format round-trip boundaries and diagnostics for native syntax.
use ix_trace_rs::trace;
use qsl_cst::ParsedSource;
use qsl_foundation::{Code, Diagnostic, Phase, SourceIdentity};
use quire_spec_language::format::{format, format_with_limit, FormatRefusal, OUTPUT_BYTE_CEILING};
use quire_spec_language::{parse, Limits};

fn parse_text(text: &str) -> Result<quire_spec_language::ParsedUnit, Box<Diagnostic>> {
    parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:boundary".into(),
            revision_namespace: "test".into(),
            revision: "fixture:1".into(),
        },
        "boundary.native",
        text.as_bytes(),
        Limits::default(),
    )
}

fn parse_complete(text: &str) -> ParsedSource {
    qsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:boundary".into(),
            revision_namespace: "test".into(),
            revision: "fixture:1".into(),
        },
        "boundary.quire",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .unwrap()
}

const COMPLETE: &str = concat!(
    "language \"ix:native\" edition \"1-draft\";\n",
    "// café: a multibyte comment counts in bytes\n",
    "profile v = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n",
    "type Digit = Int[0, 9];\n",
    "function inc using v(x: Digit): Int[0, 10] pure { x + 1 }\n",
);

#[trace("TC-016", "FR-003-AC-4", "FR-003-AC-5", "FR-003-AC-6")]
#[test]
fn formatter_byte_ceiling_is_inclusive_and_counts_final_newline() {
    let parsed = parse_complete(COMPLETE);
    let expected = format(&parsed).unwrap();
    assert!(expected.ends_with('\n'));
    for limit in 0..expected.len() {
        let refusal = format_with_limit(&parsed, limit).unwrap_err();
        assert!(
            matches!(refusal, FormatRefusal::OutputBudgetExhausted(_)),
            "limit {limit}"
        );
        // QSL-236: the output byte ceiling is `SyntaxLimit::SourceBytes`,
        // one of the four kinds that map onto `stage_limit_exceeded`.
        assert_eq!(refusal.code(), Code::StageLimitExceeded, "limit {limit}");
        let diagnostic = refusal.diagnostic();
        assert_eq!(diagnostic.phase, Phase::Format);
        assert_eq!(&diagnostic.source, parsed.source().identity());
        assert!(diagnostic.byte_span().unwrap().end <= parsed.source().text().len());
    }
    for limit in [
        expected.len(),
        expected.len() + 1,
        OUTPUT_BYTE_CEILING,
        usize::MAX,
    ] {
        assert_eq!(format_with_limit(&parsed, limit).unwrap(), expected);
    }
}

#[trace("TC-016", "FR-003-AC-4", "FR-003-AC-6")]
#[test]
fn formatter_cannot_raise_the_hard_content_ceiling() {
    let header = concat!(
        "language\"ix:native\"edition\"1-draft\";",
        "profile v=\"quire.value.complete/v1\"version\"1\"digest\"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";",
        "type Digit=Int[0,9];"
    );
    assert_eq!(OUTPUT_BYTE_CEILING, 1_048_576);
    // Whitespace inserted around header tokens expands an exactly admitted source.
    let text = format!(
        "{header}//{}",
        "x".repeat(OUTPUT_BYTE_CEILING - header.len() - 2)
    );
    let parsed = parse_complete(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.source().text().len(), OUTPUT_BYTE_CEILING);
    for limit in [OUTPUT_BYTE_CEILING, usize::MAX] {
        let refusal = format_with_limit(&parsed, limit).unwrap_err();
        assert_eq!(refusal.code(), Code::StageLimitExceeded);
        assert_eq!(refusal.diagnostic().phase, Phase::Format);
    }
    assert_eq!(
        format(&parsed).unwrap_err().code(),
        Code::StageLimitExceeded
    );
}

/// Everything in `src/format.rs` except its `mod tests` block and comment
/// lines, for the TC-404 step-1 edge scan.
fn format_module_production_code() -> String {
    let text = include_str!("../../src/format.rs");
    let production = text
        .split_once("#[cfg(test)]\nmod tests {")
        .map_or(text, |(production, _)| production);
    production
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A `use` line of any visibility, with its visibility removed.
fn use_path(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let line = ["pub(crate) ", "pub(super) ", "pub "]
        .iter()
        .find_map(|visibility| line.strip_prefix(visibility))
        .unwrap_or(line);
    line.strip_prefix("use ")
}

/// Every macro invoked as `name!(`, `name![` or `name!{`.
fn invoked_macros(code: &str) -> std::collections::BTreeSet<String> {
    let bytes = code.as_bytes();
    let mut names = std::collections::BTreeSet::new();
    for (at, window) in bytes.windows(2).enumerate() {
        if window[0] != b'!' || !matches!(window[1], b'(' | b'[' | b'{') {
            continue;
        }
        let start = code[..at]
            .rfind(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
            .map_or(0, |boundary| boundary + 1);
        if start < at {
            names.insert(code[start..at].to_owned());
        }
    }
    names
}

#[trace("FR-003-AC-7", "FR-003-AC-8", "TC-404")]
#[test]
fn format_takes_the_cst_and_depends_on_layer_1_and_f_only() {
    // The signature is the CST's parse type; this fails to compile otherwise.
    let _: fn(&ParsedSource) -> Result<String, FormatRefusal> = format;
    let _: fn(&ParsedSource, usize) -> Result<String, FormatRefusal> = format_with_limit;
    let code = format_module_production_code();
    let uses: Vec<&str> = code.lines().filter_map(use_path).collect();
    assert!(!uses.is_empty());
    for path in &uses {
        assert!(
            ["qsl_cst", "qsl_foundation", "std"]
                .iter()
                .any(|allowed| path.starts_with(allowed)),
            "use {path}"
        );
    }
    for forbidden in [
        "crate::",
        "super::",
        "self::",
        "quire_spec_language",
        "ParsedUnit",
        "syntax::",
        "parser::",
    ] {
        assert!(!code.contains(forbidden), "src/format.rs names {forbidden}");
    }
    // A crate-local `macro_rules!` macro reaches the root crate without a
    // path; only standard-library macros are invoked.
    for name in invoked_macros(&code) {
        assert!(
            ["matches"].contains(&name.as_str()),
            "src/format.rs invokes {name}!"
        );
    }
}

#[trace("FR-003-AC-7", "FR-003-AC-8", "TC-404")]
#[test]
fn format_formats_complete_v1_source_the_arena_parser_refuses() {
    let text = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n",
        "record Point { x: Int[0, 9]; y: Int[0, 9]; }\n",
        "function px using v(p: Point): Int[0, 9] pure {\n",
        "  // the nested block keeps this comment\n",
        "  if p.x > p.y then p.x else p.y }\n",
    );
    assert!(parse_text(text).is_err());
    let parsed = parse_complete(text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let formatted = format(&parsed).unwrap();
    assert!(formatted.contains("// the nested block keeps this comment"));
    let reparsed = parse_complete(&formatted);
    assert!(reparsed.is_admissible(), "{formatted}");
    let significant = |parsed: &ParsedSource| -> Vec<Vec<u8>> {
        parsed
            .cst()
            .tokens()
            .iter()
            .filter(|token| token.class() != qsl_cst::TokenClass::Whitespace)
            .map(|token| token.spelling().to_vec())
            .collect()
    };
    assert_eq!(significant(&parsed), significant(&reparsed));
    assert_eq!(format(&reparsed).unwrap(), formatted);
}

#[trace("FR-003-AC-7", "FR-003-AC-8", "TC-404")]
#[test]
fn format_refuses_a_recovering_or_diagnosed_parse_without_output() {
    let recovering = parse_complete(concat!(
        "language \"ix:native\" edition \"1-draft\"; ",
        "profile v = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"; ",
        "record Broken { value: Integer }"
    ));
    assert!(!recovering.cst().recoveries().is_empty());
    let first = recovering.diagnostics()[0].clone();
    for refusal in [
        format(&recovering).unwrap_err(),
        format_with_limit(&recovering, usize::MAX).unwrap_err(),
    ] {
        let FormatRefusal::RecoveringCst(diagnostic) = refusal else {
            panic!("expected RecoveringCst, got {refusal:?}");
        };
        assert_eq!(*diagnostic, first);
        assert_eq!(diagnostic.code, Code::InvalidSyntax);
    }

    let mut diagnosed = parse_complete(COMPLETE);
    assert!(diagnosed.is_admissible());
    let profile_refusal = qsl_cst::diagnostic::error(
        diagnosed.source(),
        Code::UnknownProfile,
        qsl_cst::CompleteCause::UnsupportedSelection,
        Phase::Profile,
        0,
        0,
        "profile is not in the selected catalog",
    );
    diagnosed.prepend_diagnostic(*profile_refusal.clone());
    assert!(diagnosed.cst().recoveries().is_empty());
    for refusal in [
        format(&diagnosed).unwrap_err(),
        format_with_limit(&diagnosed, usize::MAX).unwrap_err(),
    ] {
        let FormatRefusal::DiagnosedSource(diagnostic) = refusal else {
            panic!("expected DiagnosedSource, got {refusal:?}");
        };
        assert_eq!(diagnostic, profile_refusal);
        assert_eq!(diagnostic.code, Code::UnknownProfile);
    }
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
    // Counted directly against `Code::all()` on this branch (47 pre-existing
    // variants from `main` after #143/#144/#149's FR-153 codes
    // (`ForeignReference`, `CardinalityOutOfBound`), QSL#213 S-2's
    // `DuplicateSelection`, ADR-013 O-01/QC-5), plus QSL-236's
    // `StageLimitExceeded`; QSL-255 removed QSL-6's
    // `UnsupportedDependencySelections`) rather than derived by arithmetic —
    // `Code::all()` lists exactly 47
    // entries, one per `pub enum Code` variant, none missing and none
    // duplicated. FR-322 I2's own envelope-shape refusals
    // (`UnknownContractVersion`, `MalformedWire`, `DuplicateMember`,
    // `UnknownMember`, `NoncanonicalWire`, `DigestDomainMismatch`) are not
    // mirrored here at all: they collapse onto the existing
    // `Code::InvalidPackage`, exactly like
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

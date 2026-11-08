// SPDX-License-Identifier: AGPL-3.0-or-later
//! S1 parse of a selected extension's declaration forms and the
//! declaration-form refusal (FR-354 "Admission"; TC-882 parse steps,
//! TC-883 steps 5 and 6; QSpec FR-467 "Production").
use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_cst::{CompleteCause, CompleteCode, CompleteDiagnostic, Limits, ParsedSource, Production};
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::SourceIdentity;
use qsl_semantics::library::bundle::{
    Definition, DefinitionCatalog, DefinitionRole, ReaderAuthority,
};
use qsl_semantics::library::extension::{CoreVocabulary, HeldExtensions};
use quire_spec_language::complete::parse_with_extensions;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\";\n";
const FORM: &str = "retention Audit { subject: \"audit\", days: 30 }";

fn identity() -> SourceIdentity {
    SourceIdentity {
        authority: "test".into(),
        identity: "test:extension-s1".into(),
    }
}

/// TC-882's `retention` extension, held after catalog validation.
fn held(keywords: &[&str]) -> HeldExtensions {
    let reader = ReaderAuthority::fixture();
    let definitions: Vec<Definition> = keywords
        .iter()
        .map(|keyword| {
            let document = serde_json::json!({
                "grammar_schema": {"productions": [
                    {"keyword": keyword, "clauses": ["subject", "days"]}
                ]},
                "typed_node_schema": {"nodes": [
                    {"keyword": keyword, "node_kind": keyword, "clauses": [
                        {"name": "subject", "type": "Text"},
                        {"name": "days", "type": "Int[1, 3650]"}
                    ]}
                ]}
            });
            Definition::from_exact_bytes(
                &reader,
                "agent-ix",
                format!("ext.{keyword}"),
                DefinitionRole::Source,
                BTreeSet::new(),
                BTreeSet::new(),
                document.to_string().as_bytes(),
            )
            .unwrap()
        })
        .collect();
    let selected: Vec<_> = keywords
        .iter()
        .map(|keyword| DefinitionRef::new("agent-ix", format!("ext.{keyword}")).unwrap())
        .collect();
    DefinitionCatalog::new(definitions)
        .unwrap()
        .hold_extensions(
            &selected,
            &CoreVocabulary::new(["function", "record"], ["record"]),
        )
        .unwrap()
}

fn parse(held: &HeldExtensions, declarations: &str) -> ParsedSource {
    parse_with_extensions(
        identity(),
        "unit.native",
        format!("{HEADER}{declarations}\n").as_bytes(),
        held,
        Limits::default(),
    )
    .unwrap()
}

fn only_diagnostic(parsed: &ParsedSource) -> &CompleteDiagnostic {
    assert_eq!(parsed.diagnostics().len(), 1, "{:?}", parsed.diagnostics());
    &parsed.diagnostics()[0]
}

/// The first diagnostic is at the first occurrence of `token` after `from`.
fn refused_at(declarations: &str, from: &str, token: &str) -> (usize, usize) {
    let source = format!("{HEADER}{declarations}\n");
    let start = source.find(from).unwrap();
    let at = start + source[start..].find(token).unwrap();
    (at, at + token.len())
}

fn invalid_syntax(declarations: &str, from: &str, token: &str, expected: &str) {
    let parsed = parse(&held(&["retention"]), declarations);
    let diagnostic = only_diagnostic(&parsed);
    assert_eq!(
        diagnostic.code,
        CompleteCode::InvalidSyntax,
        "{declarations}"
    );
    assert_eq!(
        diagnostic.cause,
        CompleteCause::UnexpectedToken,
        "{declarations}"
    );
    let (start, end) = refused_at(declarations, from, token);
    let span = diagnostic.byte_span().unwrap();
    assert_eq!((span.start, span.end), (start, end), "{declarations}");
    assert!(
        diagnostic.message.contains(expected),
        "{declarations}: {}",
        diagnostic.message
    );
}

#[trace("TC-882", "FR-354-AC-1")]
#[test]
fn a_selected_extension_form_parses_as_one_node_spanning_the_whole_form() {
    let parsed = parse(&held(&["retention"]), FORM);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let forms: Vec<_> = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::ExtensionDeclaration)
        .collect();
    assert_eq!(forms.len(), 1);
    assert_eq!(parsed.cst().render_node(forms[0]).unwrap(), FORM.as_bytes());
    let values = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::Expression)
        .count();
    assert_eq!(values, 2, "one expression per clause value");
}

#[trace("TC-882", "FR-354-AC-2")]
#[test]
fn a_form_that_misses_its_production_refuses_at_s1_naming_the_expected_clause() {
    // `days` absent: the `}` where the production expects the next clause.
    invalid_syntax(
        "retention Audit { subject: \"audit\" }",
        "retention",
        "}",
        "clause `days`",
    );
    // Out of order: `days` where the production expects `subject`.
    invalid_syntax(
        "retention Audit { days: 30, subject: \"audit\" }",
        "retention",
        "days",
        "clause `subject`",
    );
    // Trailing separator: the `}` after the last `,`.
    for expected in ["clause-name", "clause `days`"] {
        invalid_syntax(
            "retention Audit { subject: \"audit\", }",
            "retention",
            "}",
            expected,
        );
    }
    // An unknown clause name and a repeated clause.
    invalid_syntax(
        "retention Audit { subject: \"a\", period: 3 }",
        "retention",
        "period",
        "clause `days`",
    );
    invalid_syntax(
        "retention Audit { subject: \"a\", subject: \"b\" }",
        "\"a\", ",
        "subject",
        "clause `days`",
    );
}

#[trace("TC-882", "FR-354-AC-2")]
#[test]
fn a_form_missing_a_name_brace_or_value_refuses_naming_the_actual_token() {
    invalid_syntax(
        "retention { subject: \"audit\", days: 30 }",
        "retention",
        "{",
        "identifier",
    );
    invalid_syntax(
        "retention Audit subject: \"audit\", days: 30",
        "retention",
        "subject",
        "`{`",
    );
    invalid_syntax(
        "retention Audit { subject: , days: 30 }",
        "retention",
        ",",
        "expected",
    );
}

#[trace("TC-882", "FR-354-AC-2")]
#[test]
fn a_form_cut_off_at_the_end_of_the_input_refuses_unexpected_end() {
    let parsed = parse(
        &held(&["retention"]),
        "retention Audit { subject: \"audit\",",
    );
    let diagnostic = only_diagnostic(&parsed);
    assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax);
    assert_eq!(diagnostic.cause, CompleteCause::UnexpectedEnd);
    assert!(
        diagnostic.message.contains("clause `days`"),
        "{}",
        diagnostic.message
    );
}

#[trace("TC-883", "FR-354-AC-4")]
#[test]
fn an_unselected_leading_keyword_refuses_declaration_form_whatever_the_catalog_holds() {
    let selections = "profile Complete = \"quire.value.complete/v1\"";
    let none = parse(&HeldExtensions::default(), FORM);
    let other = parse(&held(&["audit"]), FORM);
    for parsed in [&none, &other] {
        let diagnostic = only_diagnostic(parsed);
        assert_eq!(diagnostic.code, CompleteCode::UnsupportedConstruct);
        assert_eq!(diagnostic.cause, CompleteCause::DeclarationForm);
        assert_eq!(diagnostic.cause.as_str(), "declaration-form");
        assert!(diagnostic.cause.is_cause_of(diagnostic.code));
        let (start, end) = refused_at(FORM, "retention", "retention");
        let span = diagnostic.byte_span().unwrap();
        assert_eq!((span.start, span.end), (start, end));
        assert!(
            diagnostic.message.contains("`retention`"),
            "{}",
            diagnostic.message
        );
        assert!(
            diagnostic.message.contains(selections),
            "{}",
            diagnostic.message
        );
    }
    assert_eq!(
        none.diagnostics()[0].message,
        other.diagnostics()[0].message
    );
    assert_eq!(
        none.diagnostics()[0].byte_span(),
        other.diagnostics()[0].byte_span()
    );
}

#[trace("TC-883", "FR-354-AC-4")]
#[test]
fn a_word_after_another_declaration_refuses_declaration_form_at_its_own_span() {
    let declarations = "record R { datum: Integer; }\nbogus R { datum: 1 }";
    let parsed = parse(&held(&["retention"]), declarations);
    let diagnostic = only_diagnostic(&parsed);
    assert_eq!(diagnostic.cause, CompleteCause::DeclarationForm);
    let (start, end) = refused_at(declarations, "bogus", "bogus");
    let span = diagnostic.byte_span().unwrap();
    assert_eq!((span.start, span.end), (start, end));
}

#[trace("TC-883", "FR-354-AC-4")]
#[test]
fn a_leading_keyword_is_an_ordinary_identifier_outside_a_top_level_declaration_start() {
    let declarations = concat!(
        "record Policy { retention: Integer; }\n",
        "function Keep using Complete (retention: Integer): Integer pure { retention }\n",
        "retention Audit { subject: \"audit\", days: 30 }\n",
    );
    let with = parse(&held(&["retention"]), declarations);
    assert!(with.is_admissible(), "{:?}", with.diagnostics());
    let forms = with
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::ExtensionDeclaration)
        .count();
    assert_eq!(forms, 1, "only the top-level form is an extension form");

    // Without the form the same source parses identically with and without
    // the extension selected.
    let plain = declarations.replace("retention Audit { subject: \"audit\", days: 30 }\n", "");
    let selected = parse(&held(&["retention"]), &plain);
    let unselected = parse(&HeldExtensions::default(), &plain);
    assert!(selected.is_admissible(), "{:?}", selected.diagnostics());
    assert!(unselected.is_admissible(), "{:?}", unselected.diagnostics());
    let shape = |parsed: &ParsedSource| {
        parsed
            .cst()
            .nodes()
            .iter()
            .map(|node| (node.production(), node.span()))
            .collect::<Vec<_>>()
    };
    assert_eq!(shape(&selected), shape(&unselected));
}

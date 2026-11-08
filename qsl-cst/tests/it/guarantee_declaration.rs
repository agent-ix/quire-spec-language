// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-490: the shared grammar's `guarantee-decl` -- `guarantee G of P
//! using A { initial to "f"; progress ...; environment ...; }` -- parses to
//! one `GuaranteeDeclaration` node with its member nodes (QSpec TC-984, the S1
//! half). Its words are keywords only in their positions; a defect is
//! `invalid_syntax` at the offending token.
use ix_trace_rs::trace;
use qsl_cst::{CompleteCode, Limits, ParsedSource, Production};
use qsl_foundation::{SourceIdentity, Span};

fn source(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\";\n{declarations}"
    )
}

fn parse(text: &str) -> ParsedSource {
    qsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:guarantee-declaration".into(),
        },
        "guarantee.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn spans_of(parsed: &ParsedSource, production: Production) -> Vec<Span> {
    parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == production)
        .map(|node| node.span())
        .collect()
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

/// A declaration with every member kind parses to one `GuaranteeDeclaration`,
/// one `GuaranteeInitial`, two `GuaranteeProgress` and two
/// `GuaranteeEnvironment` nodes, each spanning through its own last token.
#[trace("FR-529-AC-5", "QSpec-TC-984", "QSpec-FR-490-AC-1")]
#[test]
fn a_guarantee_declaration_parses_its_member_forms() {
    let initial = "initial to \"counter::Counter::new\";";
    let decreases = "progress Spec::Counter::inc decreases \"counter::retries_left\";";
    let bound = "progress Spec::Counter::dec bound 4;";
    let fair = "environment Sched by \"scheduler\" fair weak each Spec::Counter::inc;";
    let assumes = "environment Net by \"network\" assumes Reliable;";
    let declaration = format!(
        "guarantee G of Eventually2 using A {{ {initial} {decreases} {bound} {fair} {assumes} }}"
    );
    let text = source(&declaration);
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());

    let [whole] = spans_of(&parsed, Production::GuaranteeDeclaration)[..] else {
        panic!("one guarantee declaration");
    };
    assert_eq!(slice(&text, whole), declaration);
    let spelled = |production| -> Vec<&str> {
        spans_of(&parsed, production)
            .into_iter()
            .map(|span| slice(&text, span))
            .collect()
    };
    assert_eq!(spelled(Production::GuaranteeInitial), [initial]);
    assert_eq!(spelled(Production::GuaranteeProgress), [decreases, bound]);
    assert_eq!(spelled(Production::GuaranteeEnvironment), [fair, assumes]);
}

/// The guarantee-decl words are keywords only in their positions: they stay
/// identifiers everywhere else (QSpec shared grammar). `decreases` is already
/// a reserved word of the core grammar's function measure and `bound` of its
/// population bound, so neither is a word this declaration newly reserves.
#[test]
fn the_guarantee_words_stay_identifiers_elsewhere() {
    for word in ["guarantee", "initial", "progress", "environment", "assumes"] {
        let text = source(&format!("record {word} {{ datum: Integer; }}"));
        let parsed = parse(&text);
        assert!(parsed.is_admissible(), "{word}: {:?}", parsed.diagnostics());
    }
}

/// QSpec FR-490 check 1: a declaration with no `initial` member, a second
/// `initial` member, and an unquoted Rust string each fail at the first token
/// the grammar does not admit, as `invalid_syntax`.
#[trace("FR-529-AC-5", "QSpec-TC-984", "QSpec-FR-490-AC-3")]
#[test]
fn a_malformed_guarantee_declaration_fails_at_the_offending_token() {
    for (declaration, offending) in [
        ("guarantee G of P using A { }", "}"),
        (
            "guarantee G of P using A { progress M::T::o bound 1; }",
            "progress",
        ),
        (
            "guarantee G of P using A { initial to \"f\"; initial to \"g\"; }",
            "initial",
        ),
        (
            "guarantee G of P using A { initial to counter::Counter::new; }",
            "counter",
        ),
        (
            "guarantee G of P using A { initial to \"f\"; progress M::T::o decreases m; }",
            "m",
        ),
        (
            "guarantee G of P using A { initial to \"f\"; environment E by s assumes N; }",
            "s",
        ),
    ] {
        let text = source(declaration);
        let parsed = parse(&text);
        assert!(!parsed.is_admissible(), "{declaration}");
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(
            diagnostic.code,
            CompleteCode::InvalidSyntax,
            "{declaration}"
        );
        let located = diagnostic.byte_span().expect("located");
        assert_eq!(slice(&text, located), offending, "{declaration}");
    }
}

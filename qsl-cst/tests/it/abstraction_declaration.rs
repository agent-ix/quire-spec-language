// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-450: the shared grammar's `abstraction-decl` -- `abstraction
//! <name> using <alias> { <binding>+ }` with `type`, `population` and
//! `operation` bindings -- parses to one `AbstractionDeclaration` node
//! (TC-811, the S1 half). The Rust positions are quoted strings: an unquoted
//! one is refused at its token. FR-304-AC-8 also needs the compile path, which
//! this test does not exercise.
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
            identity: "test:abstraction-declaration".into(),
            revision_namespace: "test".into(),
            revision: "1".into(),
        },
        "abstraction.native",
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

/// TC-811 step 1 (S1 half): one declaration with an object binding naming two
/// fields, a population binding and an operation binding with two parameters
/// parses to one `AbstractionDeclaration`, three binding nodes, two
/// `FieldBinding`s and two `ParameterBinding`s, each spanning through its own
/// last token.
#[test]
fn an_abstraction_declaration_parses_its_three_binding_forms() {
    let object = "type M::Thing to \"store::Thing\" { ready to \"is_ready\"; label to \"0\"; }";
    let population = "population M::Things to \"things\";";
    let frame = "operation M::Thing::act to \"store::act\" receiver \"self\" (amount to \"amount\", extra to \"extra\");";
    let declaration =
        format!("abstraction Link using Complete {{ {object} {population} {frame} }}");
    let text = source(&declaration);
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());

    let declarations = spans_of(&parsed, Production::AbstractionDeclaration);
    let [whole] = declarations[..] else {
        panic!("one abstraction declaration: {declarations:?}");
    };
    assert_eq!(slice(&text, whole), declaration);

    let single = |production| {
        let spans = spans_of(&parsed, production);
        let [span] = spans[..] else {
            panic!("one {production:?}: {spans:?}");
        };
        slice(&text, span).to_owned()
    };
    assert_eq!(single(Production::ObjectBinding), object);
    assert_eq!(single(Production::PopulationBinding), population);
    assert_eq!(single(Production::FrameBinding), frame);

    let fields: Vec<&str> = spans_of(&parsed, Production::FieldBinding)
        .into_iter()
        .map(|span| slice(&text, span))
        .collect();
    assert_eq!(fields, ["ready to \"is_ready\";", "label to \"0\";"]);
    let parameters: Vec<&str> = spans_of(&parsed, Production::ParameterBinding)
        .into_iter()
        .map(|span| slice(&text, span))
        .collect();
    assert_eq!(parameters, ["amount to \"amount\"", "extra to \"extra\""]);
}

/// TC-811 steps 3 and 4 (S1 half): a declaration with no binding, a `type`
/// binding whose Rust path is unquoted, and an operation with an unquoted
/// receiver each fail at the first token the grammar does not admit, and S1
/// reports it as invalid syntax.
#[test]
fn a_malformed_abstraction_declaration_fails_at_the_token_the_grammar_refuses() {
    for (declaration, offending) in [
        ("abstraction Link using Complete { }", "}"),
        (
            "abstraction Link using Complete { type M::Thing to store::Thing; }",
            "store",
        ),
        (
            "abstraction Link using Complete { population M::Things to things; }",
            "things",
        ),
        (
            "abstraction Link using Complete { operation M::Thing::act to \"act\" receiver self (); }",
            "self",
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

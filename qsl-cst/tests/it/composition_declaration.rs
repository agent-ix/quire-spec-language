// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-491: the shared grammar's `composition-decl` -- `compose S rule R
//! { component ...; member ... }` -- parses to one `CompositionDeclaration`
//! node with its member nodes (QSpec TC-985, the S1 half). Its words are
//! keywords only in their positions; a defect is `invalid_syntax` at the
//! offending token.
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
            identity: "test:composition-declaration".into(),
        },
        "composition.native",
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

/// A declaration with every member kind parses to one `CompositionDeclaration`
/// and one node per member, each spanning through its own last token, under
/// each of the four rule spellings.
#[trace("FR-530-AC-1", "QSpec-TC-985", "QSpec-FR-491-AC-1")]
#[test]
fn a_composition_declaration_parses_its_member_forms() {
    let components = [
        "component Producer from P using M;",
        "component Consumer from Q using M;",
    ];
    let guarantee = "guarantee Producer::Sends;";
    let rely = "rely Consumer::Done on Producer::Counted;";
    let depends = "depends Consumer::Done on Producer::Sends;";
    let supplies = "supplies fair weak each M::Producer::send;";
    let environment = "environment Net by \"network\" assumes Reliable;";
    for rule in [
        "assume-guarantee",
        "protocol-refinement",
        "refinement-transfer",
        "interface",
    ] {
        let declaration = format!(
            "compose Sys rule {rule} {{ {} {} {guarantee} {rely} {depends} {supplies} {environment} }}",
            components[0], components[1]
        );
        let text = source(&declaration);
        let parsed = parse(&text);
        assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
        let [whole] = spans_of(&parsed, Production::CompositionDeclaration)[..] else {
            panic!("one composition declaration");
        };
        assert_eq!(slice(&text, whole), declaration);
        let spelled = |production| -> Vec<&str> {
            spans_of(&parsed, production)
                .into_iter()
                .map(|span| slice(&text, span))
                .collect()
        };
        assert_eq!(spelled(Production::CompositionComponent), components);
        assert_eq!(spelled(Production::CompositionGuarantee), [guarantee]);
        assert_eq!(spelled(Production::CompositionRely), [rely]);
        assert_eq!(spelled(Production::CompositionDepends), [depends]);
        assert_eq!(spelled(Production::CompositionSupplies), [supplies]);
        assert_eq!(spelled(Production::GuaranteeEnvironment), [environment]);
    }
}

/// The composition words are keywords only in their positions: they stay
/// identifiers everywhere else. `depends` is already a reserved word of the
/// core grammar's `synthesize` form, so it is not a word this declaration newly
/// reserves.
#[test]
fn the_composition_words_stay_identifiers_elsewhere() {
    for word in [
        "compose",
        "rule",
        "component",
        "rely",
        "supplies",
        "interface",
    ] {
        let text = source(&format!("record {word} {{ datum: Integer; }}"));
        let parsed = parse(&text);
        assert!(parsed.is_admissible(), "{word}: {:?}", parsed.diagnostics());
    }
}

/// QSpec FR-491 check 1: one `component`, a rule outside the four, and a
/// member of another form each fail at the first token the grammar does not
/// admit, as `invalid_syntax`.
#[trace("FR-530-AC-1", "QSpec-TC-985", "QSpec-FR-491-AC-2")]
#[test]
fn a_malformed_composition_declaration_fails_at_the_offending_token() {
    for (declaration, offending) in [
        (
            "compose Sys rule interface { component A from P using M; guarantee A::G; }",
            "guarantee",
        ),
        (
            "compose Sys rule cooperative { component A from P using M; component B from Q using M; }",
            "cooperative",
        ),
        (
            "compose Sys rule interface { component A from P using M; component B from Q using M; progress M::T::o bound 1; }",
            "progress",
        ),
        (
            "compose Sys rule interface { component A from P using M; component B from Q using M; rely A::G on B; }",
            ";",
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

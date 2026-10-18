// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124-AC-4: the shared grammar's `terminal-decl` -- `terminal <name>
//! using <alias> on <model> (when <block> | any) ;` -- parses to one
//! `TerminalDeclaration` node per declaration (TC-1060).
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
            identity: "test:terminal-declaration".into(),
            revision_namespace: "test".into(),
            revision: "1".into(),
        },
        "terminal.native",
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

/// FR-124-AC-4 (TC-1060 step 1): `terminal when { P };` and `terminal any;`
/// each parse to one `TerminalDeclaration` that spans through its `;`; only
/// the `when` form holds a `Block`.
#[trace("TC-1060", "FR-124-AC-4")]
#[test]
fn terminal_when_and_terminal_any_parse_as_terminal_declarations() {
    let when = "terminal Done using Complete on M when { true };";
    let any = "terminal Halt using Complete on M any;";
    let text = source(&format!("{when}\n{any}"));
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());

    let declarations = spans_of(&parsed, Production::TerminalDeclaration);
    let [first, second] = declarations[..] else {
        panic!("two terminal declarations: {declarations:?}");
    };
    assert_eq!(slice(&text, first), when);
    assert_eq!(slice(&text, second), any);

    let blocks = spans_of(&parsed, Production::Block);
    let inside = |declaration: Span| {
        blocks
            .iter()
            .filter(|block| declaration.start <= block.start && block.end <= declaration.end)
            .map(|block| slice(&text, *block))
            .collect::<Vec<_>>()
    };
    assert_eq!(inside(first), ["{ true }"]);
    assert!(inside(second).is_empty());
}

/// FR-124-AC-4 (TC-1060 step 2): a declaration that omits `when`/`any`, gives
/// `when` no block, omits `on`, or omits its name fails at the first token
/// the grammar does not admit, and S1 reports it as invalid syntax.
#[trace("TC-1060", "FR-124-AC-4")]
#[test]
fn a_malformed_terminal_declaration_fails_at_the_token_the_grammar_refuses() {
    for (declaration, offending) in [
        ("terminal Done using Complete on M;", ";"),
        ("terminal Done using Complete on M when true;", "true"),
        ("terminal Done using Complete when { true };", "when"),
        ("terminal using Complete on M any;", "using"),
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

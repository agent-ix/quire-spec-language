// SPDX-License-Identifier: AGPL-3.0-or-later
//! S1 parses the `Probabilistic` family's productions of QSpec's shared
//! grammar -- `random-decl`, `reward-decl`, `workload-decl` and
//! `probabilistic-clause` -- and the `quantity-literal` they use. These tests
//! are parse-only: they carry the TC ids of the S2 cases that build the same
//! fixtures (TC-890, TC-1077) and back no acceptance criterion by themselves.
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
            identity: "test:probabilistic".into(),
            revision_namespace: "test".into(),
            revision: "1".into(),
        },
        "probabilistic.native",
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

/// The text of every node of `production`, in source order.
fn texts<'t>(parsed: &ParsedSource, text: &'t str, production: Production) -> Vec<&'t str> {
    spans_of(parsed, production)
        .into_iter()
        .map(|span| slice(text, span))
        .collect()
}

/// TC-890: `random`, `reward` and `workload` each parse to their own
/// production, spanning through the closing `;` or `}`, and the weights and
/// values inside them parse to their own productions in declared order.
#[trace("TC-890")]
#[test]
fn random_reward_and_workload_declarations_parse_to_their_productions() {
    let random = "random Server::attempt(d) ~ { 1 ms: 90, 3 ms: 7, 12 ms: 3, };";
    let reward = "reward duration on Server::attempt = d;";
    let workload =
        "workload Steady on Service { weight Server::request = 1; weight Server::attempt = 0.5; }";
    let text = source(&format!("{random}\n{reward}\n{workload}"));
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());

    assert_eq!(
        texts(&parsed, &text, Production::RandomDeclaration),
        [random]
    );
    assert_eq!(
        texts(&parsed, &text, Production::RewardDeclaration),
        [reward]
    );
    assert_eq!(
        texts(&parsed, &text, Production::WorkloadDeclaration),
        [workload]
    );
    assert_eq!(
        texts(&parsed, &text, Production::WeightedValue),
        ["1 ms: 90", "3 ms: 7", "12 ms: 3"]
    );
    assert_eq!(
        texts(&parsed, &text, Production::WorkloadWeight),
        [
            "weight Server::request = 1;",
            "weight Server::attempt = 0.5;"
        ]
    );
    assert_eq!(
        texts(&parsed, &text, Production::QuantityLiteral),
        ["1 ms", "3 ms", "12 ms"]
    );
}

/// TC-1077: a `probabilistic` claim parses to its production with the
/// scheduler, activation, confidence and body productions it names, for each
/// of the five body shapes.
#[trace("TC-1077")]
#[test]
fn probabilistic_claims_parse_each_body_shape() {
    let claims = [
        "probabilistic A using ev on M under Steady on origin with alpha 0.01, beta 0.01, indifference 0.0005 { probability >= 0.999 [ always[0,1000] holds(n.ok) ] }",
        "probabilistic B using ev on M under Steady on origin { quantile 0.95 of (accumulate d from holds(a) until holds(b) within 10) <= 5 ms }",
        "probabilistic C using ev on M under Steady on origin { mean of (fraction holds(a) over [0, 100] weighted by d) >= 0.99 }",
        "probabilistic D using ev on M under Steady on origin { long_run_fraction holds(a) weighted by delay >= 0.999 }",
        "probabilistic E using ev on M under Steady on origin { expected elapsed until holds(b) <= 5 ms }",
        "probabilistic F using ev on M over (n: M::Node) under every scheduler fair weak Node::tick fair strong each Node::send on each (m: M::Node) when (true) { probability <= 0.5 [ steps from holds(a) until holds(b) within 10 >= 3 ] }",
    ];
    let text = source(&claims.join("\n"));
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(
        texts(&parsed, &text, Production::ProbabilisticClause),
        claims
    );
    assert_eq!(
        texts(&parsed, &text, Production::SchedulerClause),
        [
            "under Steady",
            "under Steady",
            "under Steady",
            "under Steady",
            "under Steady",
            "under every scheduler fair weak Node::tick fair strong each Node::send"
        ]
    );
    assert_eq!(
        texts(&parsed, &text, Production::FairnessItem),
        ["fair weak Node::tick", "fair strong each Node::send"]
    );
    assert_eq!(
        texts(&parsed, &text, Production::Confidence),
        ["with alpha 0.01, beta 0.01, indifference 0.0005"]
    );
    assert_eq!(
        texts(&parsed, &text, Production::ProbabilisticForm).len(),
        6
    );
    assert_eq!(texts(&parsed, &text, Production::Fraction).len(), 1);
    assert_eq!(
        texts(&parsed, &text, Production::Weighting),
        ["weighted by d", "weighted by delay"]
    );
}

/// None of the words these productions introduce is reserved: each still
/// names a record, a field and a type.
#[trace("TC-890")]
#[test]
fn the_probabilistic_words_stay_identifiers_elsewhere() {
    let words = [
        "random",
        "reward",
        "workload",
        "weight",
        "probabilistic",
        "under",
        "every",
        "scheduler",
        "with",
        "alpha",
        "beta",
        "indifference",
        "probability",
        "quantile",
        "mean",
        "long_run_fraction",
        "expected",
        "accumulate",
        "steps",
        "elapsed",
        "fraction",
        "weighted",
        "delay",
    ];
    let declarations = words
        .iter()
        .map(|word| format!("record {word} {{ {word}: M::{word}; }}"))
        .collect::<Vec<_>>()
        .join("\n");
    let text = source(&declarations);
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(parsed.cst().render(), text.as_bytes());
}

/// A malformed declaration fails at the first token the grammar does not
/// admit, reported as invalid syntax.
#[trace("TC-890")]
#[test]
fn a_malformed_probabilistic_declaration_fails_at_the_token_the_grammar_refuses() {
    for (declaration, offending) in [
        ("workload W on M { }", "}"),
        ("random M::op(d) { 1: 1 };", "{"),
        ("random M::op(d) ~ { };", "}"),
        ("reward r on M::op d;", "d"),
        (
            "probabilistic C using ev on M under W on origin { probability > 0.5 [ holds(a) ] }",
            ">",
        ),
        (
            "probabilistic C using ev on M under W on origin with alpha { probability >= 0.5 [ holds(a) ] }",
            "{",
        ),
        (
            "probabilistic C using ev on M under W on origin { probability >= 1.5e3 [ holds(a) ] }",
            "1.5e3",
        ),
        (
            "probabilistic C using ev on M under W { probability >= 0.5 [ holds(a) ] }",
            "{",
        ),
        (
            "probabilistic C using ev on M under W on origin { quantile 0.5 of (steps from holds(a) until holds(b) within 99999999999999999999) <= 5 }",
            ")",
        ),
    ] {
        let text = source(declaration);
        let parsed = parse(&text);
        assert!(!parsed.is_admissible(), "{declaration}");
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{declaration}");
        let located = diagnostic.byte_span().expect("located");
        assert_eq!(slice(&text, located), offending, "{declaration}");
    }
}

/// A decimal literal is an `exact-value` only: as an ordinary expression it is
/// still refused, and a quantity literal is an ordinary expression.
#[trace("TC-890")]
#[test]
fn a_quantity_literal_is_an_expression_and_a_bare_decimal_is_not() {
    let quantity = source("function f using Complete (): Boolean pure { 5 ms }");
    let parsed = parse(&quantity);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(
        texts(&parsed, &quantity, Production::QuantityLiteral),
        ["5 ms"]
    );

    let decimal = source("function f using Complete (): Boolean pure { 0.5 }");
    let parsed = parse(&decimal);
    assert!(!parsed.is_admissible());
    assert_eq!(parsed.diagnostics()[0].code, CompleteCode::InvalidSyntax);
}

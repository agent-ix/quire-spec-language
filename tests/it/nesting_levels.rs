// SPDX-License-Identifier: AGPL-3.0-or-later
//! Nesting levels, chain lengths and stack safety for the native S1 parser,
//! historical and composed editions. Every test runs on a 512 KiB thread at
//! the default ceilings; see `qsl-cst/tests/it/nesting_levels.rs` for the
//! complete-V1 parser.
use ix_trace_rs::trace;
use qsl_foundation::{Code, Diagnostic, Phase, SourceIdentity, SyntaxLimit};
use quire_spec_language::syntax::composed::NativeUnit;
use quire_spec_language::{parse, parse_native, Limits, ParsedUnit};

fn identity(id: &str) -> SourceIdentity {
    SourceIdentity {
        authority: "test".into(),
        identity: format!("test:{id}"),
        revision_namespace: "test".into(),
        revision: "1".into(),
    }
}

const HEADER: &str = "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"test/model\" version \"1\" digest \"unresolved\";\n";

const COMPOSED_HEADER: &str = r#"language "ix:native" edition "1-draft";
profile S = "quire.state.graph/v1" version "test:state" digest "unresolved-state";
profile T = "quire.temporal.timestamped-event.finite-window/v1" version "test:temporal" digest "unresolved-temporal";
profile P = "quire.protocol.finite-global/v1" version "test:protocol" digest "unresolved-protocol";
model M = "test:orders-and-refunds" version "test:model" digest "unresolved-model";
"#;

/// Everything before a historical invariant's expression. Its `{` is one
/// bracket pair.
fn body_prefix() -> String {
    format!("{HEADER}invariant Test on M::Thing at current {{ ")
}

fn document(expression: &str) -> String {
    format!("{}{expression} }}\n", body_prefix())
}

fn read_expr(expression: &str) -> Result<ParsedUnit, Box<Diagnostic>> {
    parse(
        identity("nesting"),
        "test.native",
        document(expression).as_bytes(),
        Limits::default(),
    )
}

fn temporal_document(formula: &str) -> String {
    format!(
        "{COMPOSED_HEADER}temporal Test using T over (view: M::OrderView) clock \"c\" on origin {{ {formula} }}"
    )
}

fn read_composed(text: &str) -> Result<NativeUnit, Box<Diagnostic>> {
    parse_native(
        identity("composed"),
        "composed.native",
        text.as_bytes(),
        Limits::default(),
    )
}

/// Run `run` on a 512 KiB thread, the stack the verification procedure
/// names. A parser that recursed per chain element or per bracket frame
/// beyond that budget aborts the test process here.
fn on_bounded_stack<F: FnOnce() + Send + 'static>(run: F) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(run)
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("the parser must not overflow a 512 KiB stack");
}

/// The limit a refusal names, asserting it is a resource refusal whose
/// message renders that same limit, and (QSL-236) that its code matches the
/// kind: `stage_limit_exceeded` for every kind the catalog admits,
/// `resource_exhausted` for `Tokens` (no catalog cause yet, STD-95).
fn refused_limit(error: &Diagnostic) -> SyntaxLimit {
    let limit = error
        .limit()
        .expect("a syntax refusal carries its typed limit");
    let expected_code = match limit.stage_kind() {
        Some(_) => Code::StageLimitExceeded,
        None => Code::ResourceExhausted,
    };
    assert_eq!(error.code, expected_code, "{error}");
    assert_eq!(error.message, limit.to_string());
    limit
}

fn is_token_or_node_ceiling(limit: SyntaxLimit) -> bool {
    let defaults = Limits::default();
    limit
        == SyntaxLimit::Tokens {
            bound: defaults.tokens,
        }
        || limit
            == SyntaxLimit::Nodes {
                bound: defaults.nodes,
            }
}

fn repeat_join(item: &str, separator: &str, count: usize) -> String {
    std::iter::repeat_n(item, count)
        .collect::<Vec<_>>()
        .join(separator)
}

// One chain per shape. `elements` is the number of operators, prefixes,
// `let`s or `if`s.
fn sum_chain(elements: usize) -> String {
    repeat_join("1", " + ", elements + 1)
}
fn implies_chain(elements: usize) -> String {
    repeat_join("true", " implies ", elements + 1)
}
fn not_chain(elements: usize) -> String {
    format!("{}true", "not ".repeat(elements))
}
fn let_tail_chain(elements: usize) -> String {
    format!("{}0", "let v = 0 in ".repeat(elements))
}
fn if_tail_chain(elements: usize) -> String {
    format!("{}0", "if true then 1 else ".repeat(elements))
}
fn let_value_chain(elements: usize) -> String {
    format!(
        "{}0{}",
        "let v = ".repeat(elements),
        " in 0".repeat(elements)
    )
}
fn if_condition_chain(elements: usize) -> String {
    format!(
        "{}true{}",
        "if ".repeat(elements),
        " then true else true".repeat(elements)
    )
}
fn if_then_chain(elements: usize) -> String {
    format!(
        "{}0{}",
        "if true then ".repeat(elements),
        " else 0".repeat(elements)
    )
}
fn always_chain(elements: usize) -> String {
    temporal_document(&format!("{}true", "always [0,1] ".repeat(elements)))
}
fn temporal_implies_chain(elements: usize) -> String {
    temporal_document(&repeat_join("true", " implies ", elements + 1))
}
fn temporal_not_chain(elements: usize) -> String {
    temporal_document(&format!("{}true", "not ".repeat(elements)))
}

/// `elements` parses with nesting depth 0 and `elements + 1` is refused
/// naming the token or syntax-node ceiling.
fn assert_longest_chain(
    name: &str,
    elements: usize,
    read: impl Fn(usize) -> Result<(), Box<Diagnostic>>,
) {
    if let Err(error) = read(elements) {
        panic!("{name}: the {elements}-element chain must parse: {error}");
    }
    let error = read(elements + 1).expect_err("one element longer exceeds a ceiling");
    let limit = refused_limit(&error);
    assert!(
        is_token_or_node_ceiling(limit),
        "{name}: one element longer must name the token or node ceiling, got {limit:?}"
    );
}

fn historical(build: fn(usize) -> String) -> impl Fn(usize) -> Result<(), Box<Diagnostic>> {
    move |elements| read_expr(&build(elements)).map(drop)
}

fn composed(build: fn(usize) -> String) -> impl Fn(usize) -> Result<(), Box<Diagnostic>> {
    move |elements| read_composed(&build(elements)).map(drop)
}

#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn parenthesized_sum_five_deep_parses() {
    on_bounded_stack(|| {
        read_expr("(((((1 + 1) + 1) + 1) + 1) + 1)")
            .expect("five bracket pairs are far under the default ceiling");
    });
}

#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn paren_nesting_to_exactly_the_ceiling_parses_and_one_deeper_is_refused_by_the_lexer() {
    on_bounded_stack(|| {
        let bound = Limits::default().nesting;
        // The invariant's own `{` is the first pair.
        let inner = bound - 1;
        read_expr(&format!("{}1{}", "(".repeat(inner), ")".repeat(inner)))
            .expect("exactly the ceiling parses");

        let opens = "(".repeat(inner + 1);
        let text = document(&format!("{opens}1{}", ")".repeat(inner + 1)));
        let offending = body_prefix().len() + opens.len() - 1;
        let error = parse(
            identity("over"),
            "test.native",
            text.as_bytes(),
            Limits::default(),
        )
        .expect_err("one pair deeper is refused");
        assert_eq!(refused_limit(&error), SyntaxLimit::NestingDepth { bound });
        assert_eq!(error.phase, Phase::Lex);
        assert_eq!(
            (error.span.start.byte, error.span.end.byte),
            (offending, offending + 1)
        );
    });
}

// A type-argument `<` is a bracket pair the lexer does not count, so only
// the parser's own charge can refuse it. Past the ceiling the `<` is refused
// before anything after it is read; the unit is cut short after the `>` so
// no later `(` reaches the lexer's own check first.
#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn type_argument_bracket_past_the_ceiling_is_refused_by_the_parser() {
    on_bounded_stack(|| {
        let bound = Limits::default().nesting;
        let prefix = |inner: usize| {
            format!(
                "{COMPOSED_HEADER}invariant Deep using S on M::OrderView at current {{ {}",
                "(".repeat(inner)
            )
        };
        // The clause's `{` is pair 1, so after `inner` parens the `<` and
        // the call's `(` are pair `inner + 2`.
        let inner = bound - 2;
        let at_bound = format!("{}size<M::T>(1){} }}", prefix(inner), ")".repeat(inner));
        read_composed(&at_bound).expect("size<…>(…) exactly at the ceiling parses");

        let inner = bound - 1;
        let over = format!("{}size<M::T>{} }}", prefix(inner), ")".repeat(inner));
        let error = read_composed(&over).expect_err("the `<` opens pair 65");
        assert_eq!(refused_limit(&error), SyntaxLimit::NestingDepth { bound });
        assert_eq!(error.phase, Phase::Parse);
        let angle = prefix(inner).len() + "size".len();
        assert_eq!(
            (error.span.start.byte, error.span.end.byte),
            (angle, angle + 1)
        );
    });
}

#[trace("TC-012", "NFR-001-M-3")]
#[test]
fn exhausting_the_node_ceiling_names_the_node_ceiling() {
    let limits = Limits {
        nodes: 3,
        ..Limits::default()
    };
    let error = parse(
        identity("nodes"),
        "test.native",
        document("1 + 1 + 1 + 1").as_bytes(),
        limits,
    )
    .expect_err("three syntax nodes cannot hold seven");
    assert_eq!(refused_limit(&error), SyntaxLimit::Nodes { bound: 3 });
    assert_eq!(error.phase, Phase::Parse);
}

#[trace("TC-012", "NFR-001-M-2", "NFR-001-M-3")]
#[test]
fn flat_twenty_thousand_operator_chain_completes() {
    on_bounded_stack(|| {
        read_expr(&sum_chain(20_000)).expect("20000 operators fit the default ceilings");
    });
}

// The longest chain of each shape the default ceilings admit parses, and
// one element more names the ceiling it hits. Every shape has nesting
// depth 0.
#[trace("TC-012", "NFR-001-M-2", "NFR-001-M-3")]
#[test]
fn longest_historical_chains_parse_and_one_longer_names_a_ceiling() {
    on_bounded_stack(|| {
        assert_longest_chain("+", 24_999, historical(sum_chain));
        assert_longest_chain("implies", 24_999, historical(implies_chain));
        assert_longest_chain("not", 49_999, historical(not_chain));
        assert_longest_chain("let tail", 19_994, historical(let_tail_chain));
        assert_longest_chain("if tail", 16_666, historical(if_tail_chain));
    });
}

// A `let` value, an `if` condition and an `if` then-branch nest without a
// bracket just like the tail does.
#[trace("TC-012", "NFR-001-M-2", "NFR-001-M-3")]
#[test]
fn let_and_if_chains_in_every_position_parse_on_a_bounded_stack() {
    on_bounded_stack(|| {
        assert_longest_chain("let value", 19_994, historical(let_value_chain));
        assert_longest_chain("if condition", 16_666, historical(if_condition_chain));
        assert_longest_chain("if then", 16_666, historical(if_then_chain));
    });
}

#[trace("TC-012", "NFR-001-M-2", "NFR-001-M-3")]
#[test]
fn longest_composed_temporal_chains_parse_and_one_longer_names_a_ceiling() {
    on_bounded_stack(|| {
        assert_longest_chain("always", 16_656, composed(always_chain));
        assert_longest_chain("temporal implies", 24_996, composed(temporal_implies_chain));
        assert_longest_chain("temporal not", 49_992, composed(temporal_not_chain));
    });
}

// `repeat` bodies and `await` branches nest control nodes without a bracket.
#[trace("TC-012", "NFR-001-M-2", "NFR-001-M-3")]
#[test]
fn long_repeat_control_chain_parses_on_a_bounded_stack() {
    on_bounded_stack(|| {
        let depth = 3_000;
        let text = format!(
            "{COMPOSED_HEADER}protocol Loops using P over (view: M::OrderView) on origin {{
                role Service on M::OrderView;
                run {}check Body using S {{ true }};{}
                finish Closed as (closed: M::OrderView) {{ true }};
            }}",
            "repeat Loop by Service visible (true) max 1 while { true } ".repeat(depth),
            " exhausted check Exhausted using S { true };".repeat(depth),
        );
        let NativeUnit::Composed(unit) = read_composed(&text).expect("a repeat chain parses")
        else {
            panic!("composed edition");
        };
        // One `check` body, `depth` repeats and `depth` exhausted checks.
        assert_eq!(unit.controls().len(), 2 * depth + 1);
    });
}

fn longest_admitted(read: impl Fn(usize) -> Result<(), Box<Diagnostic>>) -> usize {
    let (mut low, mut high) = (0_usize, 120_000_usize);
    while low + 1 < high {
        let middle = (low + high) / 2;
        if read(middle).is_ok() {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
}

// Re-derives the lengths `assert_longest_chain` uses:
// `cargo test --test it nesting_levels::probe -- --ignored --nocapture`.
#[test]
#[ignore = "probe: prints the longest chain of each shape the default ceilings admit"]
fn probe_longest_chains_at_default_ceilings() {
    on_bounded_stack(|| {
        for (name, build) in [
            ("+", sum_chain as fn(usize) -> String),
            ("implies", implies_chain),
            ("not", not_chain),
            ("let tail", let_tail_chain),
            ("if tail", if_tail_chain),
            ("let value", let_value_chain),
            ("if condition", if_condition_chain),
            ("if then", if_then_chain),
        ] {
            println!("historical {name}: {}", longest_admitted(historical(build)));
        }
        for (name, build) in [
            ("always", always_chain as fn(usize) -> String),
            ("temporal implies", temporal_implies_chain),
            ("temporal not", temporal_not_chain),
        ] {
            println!("composed {name}: {}", longest_admitted(composed(build)));
        }
    });
}

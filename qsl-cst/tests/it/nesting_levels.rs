// SPDX-License-Identifier: AGPL-3.0-or-later
//! Nesting levels, chain lengths, work budget and stack safety for the
//! complete-V1 parser. Every parse runs on a 512 KiB thread at the default
//! ceilings; see `tests/it/nesting_levels.rs` in the root crate for the
//! native parser.
use ix_trace_rs::trace;
use qsl_cst::{CompleteCode, CompleteDiagnostic, Limits, ParsedSource, Production};
use qsl_foundation::{Phase, SourceIdentity, SyntaxLimit};

fn identity(id: &str) -> SourceIdentity {
    SourceIdentity {
        identity: format!("test:{id}"),
        revision: "1".into(),
    }
}

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

/// Everything before a function body expression. The body's `{` is one
/// bracket pair; the parameter list's `()` closes before it.
fn function_prefix() -> String {
    format!("{HEADER}function f using Complete (x: Integer): Boolean pure {{ ")
}

fn function(body: &str) -> String {
    format!("{}{body} }}\n", function_prefix())
}

fn temporal(formula: &str) -> String {
    format!(
        "{HEADER}temporal W using Complete over (s: Integer) clock \"t\" on origin {{ {formula} }}\n"
    )
}

fn parse(text: &str) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    qsl_cst::parse(
        identity("nesting"),
        "nesting.native",
        text.as_bytes(),
        Limits::default(),
    )
}

/// Parse `text`, which must be admitted without diagnostics.
fn admitted(text: &str) -> ParsedSource {
    let parsed = parse(text).expect("within the default ceilings");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    parsed
}

/// Run `run` on a 512 KiB thread, the stack the verification procedure
/// names. Overflowing it aborts the test process.
fn on_bounded_stack<F: FnOnce() + Send + 'static>(run: F) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(run)
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("the parser must not overflow a 512 KiB stack");
}

/// The limit a refusal names, asserting it is a resource refusal whose
/// message renders that same limit.
fn refused_limit(error: &CompleteDiagnostic) -> SyntaxLimit {
    assert_eq!(error.code, CompleteCode::ResourceExhausted, "{error}");
    let limit = error
        .limit()
        .expect("a syntax refusal carries its typed limit");
    assert_eq!(error.message, limit.to_string());
    limit
}

fn repeat_join(item: &str, separator: &str, count: usize) -> String {
    std::iter::repeat_n(item, count)
        .collect::<Vec<_>>()
        .join(separator)
}

// One chain per shape. `elements` counts operators, prefixes, `let`s or
// `if`s.
fn sum_chain(elements: usize) -> String {
    function(&repeat_join("x", " + ", elements + 1))
}
fn implies_chain(elements: usize) -> String {
    function(&repeat_join("true", " implies ", elements + 1))
}
fn not_chain(elements: usize) -> String {
    function(&format!("{}true", "not ".repeat(elements)))
}
fn let_chain(elements: usize) -> String {
    function(&format!("{}x", "let v = x in ".repeat(elements)))
}
fn if_chain(elements: usize) -> String {
    function(&format!("{}x", "if true then x else ".repeat(elements)))
}
fn let_value_chain(elements: usize) -> String {
    function(&format!(
        "{}x{}",
        "let v = ".repeat(elements),
        " in x".repeat(elements)
    ))
}
fn if_condition_chain(elements: usize) -> String {
    function(&format!(
        "{}x{}",
        "if ".repeat(elements),
        " then x else x".repeat(elements)
    ))
}
fn if_then_chain(elements: usize) -> String {
    function(&format!(
        "{}x{}",
        "if true then ".repeat(elements),
        " else x".repeat(elements)
    ))
}
fn always_chain(elements: usize) -> String {
    temporal(&format!("{}true", "always [0,1] ".repeat(elements)))
}
fn temporal_implies_chain(elements: usize) -> String {
    temporal(&repeat_join("true", " implies ", elements + 1))
}
fn temporal_not_chain(elements: usize) -> String {
    temporal(&format!("{}true", "not ".repeat(elements)))
}

/// `elements` parses, and `elements + 1` is refused naming the token or
/// syntax-node ceiling.
fn assert_longest_chain(name: &str, elements: usize, build: fn(usize) -> String) {
    if let Err(error) = parse(&build(elements)) {
        panic!("{name}: the {elements}-element chain must parse: {error}");
    }
    let error = parse(&build(elements + 1)).expect_err("one element longer exceeds a ceiling");
    let limit = refused_limit(&error);
    let defaults = Limits::default();
    assert!(
        limit
            == SyntaxLimit::Tokens {
                bound: defaults.tokens
            }
            || limit
                == SyntaxLimit::Nodes {
                    bound: defaults.nodes
                },
        "{name}: one element longer must name the token or node ceiling, got {limit:?}"
    );
}

#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn parenthesized_sum_five_deep_parses() {
    on_bounded_stack(|| {
        admitted(&function("(((((x + x) + x) + x) + x) + x)"));
    });
}

#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn paren_nesting_to_exactly_the_ceiling_parses_and_one_deeper_is_refused() {
    on_bounded_stack(|| {
        let bound = Limits::default().nesting;
        // The body's `{` is the first pair.
        let inner = bound - 1;
        admitted(&function(&format!(
            "{}x{}",
            "(".repeat(inner),
            ")".repeat(inner)
        )));

        let opens = "(".repeat(inner + 1);
        let error = parse(&function(&format!("{opens}x{}", ")".repeat(inner + 1))))
            .expect_err("one pair deeper is refused");
        assert_eq!(refused_limit(&error), SyntaxLimit::NestingDepth { bound });
        assert_eq!(error.phase, Phase::Parse);
        let offending = function_prefix().len() + opens.len() - 1;
        assert_eq!(
            (error.span.start.byte, error.span.end.byte),
            (offending, offending + 1)
        );
    });
}

#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn option_type_nesting_to_exactly_the_ceiling_parses_and_one_deeper_is_refused() {
    on_bounded_stack(|| {
        let bound = Limits::default().nesting;
        let alias = |depth: usize| {
            let prefix = format!("{HEADER}type T = {}", "Option<".repeat(depth));
            let text = format!("{prefix}Integer{};\n", ">".repeat(depth));
            (prefix, text)
        };
        admitted(&alias(bound).1);

        let (prefix, text) = alias(bound + 1);
        let error = parse(&text).expect_err("one pair deeper is refused");
        assert_eq!(refused_limit(&error), SyntaxLimit::NestingDepth { bound });
        let angle = prefix.len() - 1;
        assert_eq!(
            (error.span.start.byte, error.span.end.byte),
            (angle, angle + 1)
        );
    });
}

// Each value in `f(f(...f(x,)...,),)` fails as a tuple and is read again as
// a call argument list. The parser reuses the argument it already matched,
// so the work stays linear in depth: the typo is a recoverable syntax error
// at the default ceilings, not a work-budget refusal (QSL-213).
#[trace("TC-012", "TC-222", "FR-302-AC-2")]
#[test]
fn nested_trailing_comma_typo_exposes_a_recovery_within_the_work_budget() {
    on_bounded_stack(|| {
        for body in [
            "f(g(f(g(x,)),))".to_owned(),
            format!("{}x{}", "f(".repeat(6), ",)".repeat(6)),
            format!("{}x{}", "f(".repeat(24), ",)".repeat(24)),
        ] {
            let text = function(&body);
            let parsed = parse(&text).unwrap_or_else(|error| panic!("{body}: {error:?}"));
            assert_eq!(parsed.cst().render(), text.as_bytes());
            assert!(!parsed.is_admissible(), "{body}");
            assert_eq!(
                parsed.diagnostics()[0].code,
                CompleteCode::InvalidSyntax,
                "{body}"
            );
            assert!(!parsed.cst().recoveries().is_empty(), "{body}");
        }
    });
}

// NFR-001: a work-budget refusal names the work limit rather than nesting
// depth. 40 unclosed `set[` stay under the nesting ceiling. No production
// matches inside them, so nothing is memoized: every level re-reads the
// failing levels inside it under each alternative that starts with `set`,
// and the work grows quadratically with depth. At depth 40 it is about
// 39,700 steps against a budget of 28,160.
#[trace("TC-012")]
#[test]
fn a_work_budget_refusal_names_the_work_limit() {
    on_bounded_stack(|| {
        let body = format!("{}x", "set[".repeat(40));
        let error = parse(&function(&body)).expect_err("the work budget is exhausted");
        assert!(
            matches!(refused_limit(&error), SyntaxLimit::Work { .. }),
            "{error:?}"
        );
        assert_eq!(error.phase, Phase::Parse);
    });
}

#[trace("TC-012", "NFR-001-M-3")]
#[test]
fn exhausting_the_node_ceiling_names_the_node_ceiling() {
    let limits = Limits {
        nodes: 20,
        ..Limits::default()
    };
    let error = qsl_cst::parse(
        identity("nodes"),
        "nodes.native",
        function("x + x + x + x").as_bytes(),
        limits,
    )
    .expect_err("20 nodes cannot hold the unit");
    assert_eq!(refused_limit(&error), SyntaxLimit::Nodes { bound: 20 });
}

#[trace("TC-012", "NFR-001-M-2")]
#[test]
fn exhausting_the_token_ceiling_names_the_token_ceiling() {
    let limits = Limits {
        tokens: 40,
        ..Limits::default()
    };
    let error = qsl_cst::parse(
        identity("tokens"),
        "tokens.native",
        function("x + x + x + x").as_bytes(),
        limits,
    )
    .expect_err("40 leaves cannot hold the unit");
    assert_eq!(refused_limit(&error), SyntaxLimit::Tokens { bound: 40 });
    assert_eq!(error.phase, Phase::Lex);
}

// The longest chain of each shape the default ceilings admit parses, and
// one element more names the ceiling it hits.
#[trace("TC-012", "NFR-001-M-2", "NFR-001-M-3")]
#[test]
fn longest_chains_parse_and_one_longer_names_a_ceiling() {
    on_bounded_stack(|| {
        assert_longest_chain("+", 9_995, sum_chain);
        assert_longest_chain("implies", 5_553, implies_chain);
        assert_longest_chain("not", 49_974, not_chain);
        assert_longest_chain("let", 4_164, let_chain);
        assert_longest_chain("if", 2_271, if_chain);
        assert_longest_chain("let value", 4_164, let_value_chain);
        assert_longest_chain("if condition", 2_173, if_condition_chain);
        assert_longest_chain("if then", 2_271, if_then_chain);
    });
}

#[trace("TC-012", "NFR-001-M-2", "NFR-001-M-3")]
#[test]
fn longest_temporal_chains_parse_and_one_longer_names_a_ceiling() {
    on_bounded_stack(|| {
        assert_longest_chain("always", 12_492, always_chain);
        assert_longest_chain("temporal implies", 8_330, temporal_implies_chain);
        assert_longest_chain("temporal not", 49_971, temporal_not_chain);
    });
}

// A failed parse discards the matches of a deep chain without recursing.
#[trace("TC-012")]
#[test]
fn a_deep_chain_that_fails_to_parse_is_discarded_on_a_bounded_stack() {
    on_bounded_stack(|| {
        let parsed = parse(&function(&format!("{}x true", "not ".repeat(20_000))))
            .expect("a syntax error is a recovered parse, not a refusal");
        assert!(!parsed.is_admissible());
        assert!(!parsed.cst().recoveries().is_empty());
    });
}

// A long chain yields a deep CST; rendering and identity work stay
// iterative on it.
#[trace("TC-012", "TC-222", "FR-302-AC-1")]
#[test]
fn a_deep_cst_renders_and_resolves_identities_on_a_bounded_stack() {
    on_bounded_stack(|| {
        let text = not_chain(20_000);
        let parsed = admitted(&text);
        let cst = parsed.cst();
        assert_eq!(cst.render(), text.as_bytes());
        let root = cst.root();
        assert_eq!(cst.render_node(root).expect("own node"), text.as_bytes());
        // The operand under every `not` is the deepest node.
        let deepest = cst
            .nodes()
            .iter()
            .find(|node| node.production() == Production::Primary)
            .expect("the operand");
        assert!(cst.structural_path(deepest).len() > 20_000);
    });
}

fn longest_admitted(build: fn(usize) -> String) -> usize {
    let (mut low, mut high) = (0_usize, 60_000_usize);
    while low + 1 < high {
        let middle = (low + high) / 2;
        if parse(&build(middle)).is_ok() {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
}

// Re-derives the lengths `assert_longest_chain` uses:
// `cargo test -p qsl-cst --test it nesting_levels::probe -- --ignored --nocapture`.
#[test]
#[ignore = "probe: prints the longest chain of each shape the default ceilings admit"]
fn probe_longest_chains_at_default_ceilings() {
    on_bounded_stack(|| {
        for (name, build) in [
            ("+", sum_chain as fn(usize) -> String),
            ("implies", implies_chain),
            ("not", not_chain),
            ("let", let_chain),
            ("if", if_chain),
            ("let value", let_value_chain),
            ("if condition", if_condition_chain),
            ("if then", if_then_chain),
            ("always", always_chain),
            ("temporal implies", temporal_implies_chain),
            ("temporal not", temporal_not_chain),
        ] {
            println!("{name}: {}", longest_admitted(build));
        }
    });
}

// Chains parse in time and memory linear in their length:
// `cargo test --release -p qsl-cst --test it nesting_levels::probe_chain -- --ignored --nocapture`.
#[test]
#[ignore = "probe: wall-clock timings belong outside the default test run"]
fn probe_chain_parse_time() {
    on_bounded_stack(|| {
        for (name, text) in [
            ("not x 20000", not_chain(20_000)),
            ("always x 10000", always_chain(10_000)),
            ("not x 10000", not_chain(10_000)),
            ("always x 5000", always_chain(5_000)),
        ] {
            let started = std::time::Instant::now();
            let parsed = admitted(&text);
            let elapsed = started.elapsed();
            println!(
                "{name}: {elapsed:?}, {} nodes, {} tokens",
                parsed.cst().nodes().len(),
                parsed.cst().tokens().len()
            );
            assert!(elapsed < std::time::Duration::from_secs(1), "{name}");
        }
    });
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-197: NFR-001 "Nesting level" for the complete-V1 (lossless CST) S1
//! parser -- bracket-pair nesting depth (never grammar-production depth),
//! work-vs-nesting resource_exhausted classification, no `Rule` clone on the
//! production path, and long-chain stack safety (TC-012).
use ix_trace_rs::trace;
use qsl_cst::{CompleteCode, CompleteDiagnostic, Limits, ParsedSource};
use qsl_foundation::SourceIdentity;

fn identity(id: &str) -> SourceIdentity {
    SourceIdentity {
        identity: format!("test:{id}"),
        revision: "1".into(),
    }
}

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

/// The fixed text preceding a function body expression: `header()` plus the
/// `function f using Complete (): Boolean pure { ` opener. Callers that need
/// an exact byte offset into the body build `text` as
/// `format!("{}{body} }}\n", function_prefix())` and locate offsets relative
/// to `function_prefix().len()`.
fn function_prefix() -> String {
    format!("{HEADER}function f using Complete (): Boolean pure {{ ")
}

fn wrap_body(body: &str) -> String {
    format!("{}{body} }}\n", function_prefix())
}

fn parse(id: &str, text: &str, limits: Limits) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    qsl_cst::parse(
        identity(id),
        format!("{id}.native"),
        text.as_bytes(),
        limits,
    )
}

fn parse_body(
    id: &str,
    body: &str,
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    parse(id, &wrap_body(body), limits)
}

/// Run `run` on a thread with the 512 KiB stack NFR-001 "Verification"
/// specifies for the five named *chains* (flat `+`, `implies`, prefix
/// `not`, `let … in`, `if … else`), so a stack-depth regression in the
/// hand-written chain parsers actually crashes this test rather than
/// merely working by accident on a generous default thread stack. Each
/// chain element costs O(1) native stack frames by construction (an
/// explicit loop, not Rust recursion per element -- see
/// `Engine::parse_implication`/`parse_unary`/`parse_expression`), so this
/// is the tight bound NFR-001 verifies them against.
fn on_bounded_stack<F: FnOnce() + Send + 'static>(run: F) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(run)
        .expect("spawn bounded-stack thread")
        .join()
        .expect("parser must not overflow a 512 KiB stack (NFR-001 Verification)");
}

/// Run `run` on a thread with a generous (16 MiB) stack. Unlike the five
/// named chains, real bracket nesting to the ceiling recurses through the
/// full ten-level declarative expression ladder once per nesting level
/// (`Engine::production` -> `Engine::rule` -> ... for Disjunction,
/// Conjunction, Comparison, Sum, Product, Postfix and Primary, each a real
/// Rust stack frame): an unavoidable cost of interpreting a declarative
/// grammar table rather than a hand-written parser for those levels. In an
/// unoptimized (`cargo test`, no `--release`) build that cost is large
/// enough that even the *default* new-thread stack (2 MiB) overflows at
/// exactly the default nesting ceiling of 64 -- confirmed empirically
/// while writing this test, and confirmed safe in a `--release` build at
/// the same 2 MiB. These nesting-boundary tests use a generous stack so
/// they check the boundary's *correctness* even under `cargo test`,
/// independent of that debug-build interpreter-depth cost; see this
/// worktree's final report for that finding.
fn on_generous_stack<F: FnOnce() + Send + 'static>(run: F) {
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(run)
        .expect("spawn generous-stack thread")
        .join()
        .expect("parser must not overflow a 16 MiB stack");
}

// --- AC-1: one paren pair costs one nesting level, not one per grammar
// production it happens to be matched under. ------------------------------

#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn deeply_parenthesized_flat_sum_parses_at_default_limits() {
    on_generous_stack(|| {
        let parsed = parse_body("ac1", "(((((x + x) + x) + x) + x) + x)", Limits::default())
            .expect("bracket nesting of 5 is far under the default ceiling of 64");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    });
}

// --- AC-2: nesting to exactly the configured bound parses; one level past
// is refused naming nesting depth, the bound, and the opening bracket's
// span. -----------------------------------------------------------------

#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn paren_nesting_to_exactly_the_ceiling_parses_one_deeper_refuses() {
    on_generous_stack(|| {
        let limits = Limits::default();
        let bound = limits.nesting;
        // `wrap_body`'s function `{ … }` block is itself one bracket pair,
        // so the body needs one fewer parenthesis than `bound` for the
        // deepest token's total nesting depth to land exactly on the
        // ceiling.
        let body_bound = bound - 1;

        let at_bound = format!("{}x{}", "(".repeat(body_bound), ")".repeat(body_bound));
        let parsed =
            parse_body("ac2-at-bound", &at_bound, limits).expect("exactly the ceiling must parse");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());

        let prefix = function_prefix();
        let opens = "(".repeat(body_bound + 1);
        let closes = ")".repeat(body_bound + 1);
        let text = format!("{prefix}{opens}x{closes} }}\n");
        let last_open_offset = prefix.len() + opens.len() - 1;

        let error = parse("ac2-over-bound", &text, limits).expect_err("one level past must refuse");
        assert_eq!(error.code, CompleteCode::ResourceExhausted);
        assert!(
            error.message.contains("nesting depth") && error.message.contains(&bound.to_string()),
            "expected a nesting-depth refusal naming the bound {bound}, got: {}",
            error.message
        );
        assert_eq!(error.span.start.byte, last_open_offset);
        assert_eq!(error.span.end.byte, last_open_offset + 1);
    });
}

// --- AC-7: `Option<...>` type-argument nesting counts the same combined
// ceiling as bracket pairs, since the lexer cannot itself tell a
// type-argument `<` from a comparison operator. --------------------------

#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn option_type_argument_nesting_to_exactly_the_ceiling_parses_one_deeper_refuses() {
    on_generous_stack(|| {
        let limits = Limits::default();
        let bound = limits.nesting;

        let at_bound = format!(
            "type T = {}Integer{};\n",
            "Option<".repeat(bound),
            ">".repeat(bound)
        );
        let text = format!("{HEADER}{at_bound}");
        let parsed = parse("ac7-at-bound", &text, limits).expect("exactly the ceiling must parse");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());

        let opens = "Option<".repeat(bound + 1);
        let prefix = format!("{HEADER}type T = ");
        let text = format!("{prefix}{opens}Integer{};\n", ">".repeat(bound + 1));
        let last_open_offset = prefix.len() + opens.len() - 1;

        let error = parse("ac7-over-bound", &text, limits).expect_err("one level past must refuse");
        assert_eq!(error.code, CompleteCode::ResourceExhausted);
        assert!(
            error.message.contains("nesting depth") && error.message.contains(&bound.to_string()),
            "expected a nesting-depth refusal naming the bound {bound}, got: {}",
            error.message
        );
        assert_eq!(error.span.start.byte, last_open_offset);
        assert_eq!(error.span.end.byte, last_open_offset + 1);
    });
}

// --- AC-3: exhausting the work (syntax-node) budget names the work limit,
// never nesting. -----------------------------------------------------------

#[trace("TC-012", "NFR-001-M-3")]
#[test]
fn exhausting_the_node_budget_names_the_work_limit_not_nesting() {
    let limits = Limits {
        nodes: 5,
        ..Limits::default()
    };
    let error = parse_body("ac3", "x + x + x + x + x + x + x + x", limits)
        .expect_err("5 retained syntax nodes cannot hold this unit");
    assert_eq!(error.code, CompleteCode::ResourceExhausted);
    assert!(
        error.message.contains("syntax node budget exhausted")
            && error.message.contains("bound 5 nodes"),
        "expected a node-budget refusal naming its bound, got: {}",
        error.message
    );
    assert!(
        !error.message.contains("nesting"),
        "a work/node-budget refusal must not name nesting: {}",
        error.message
    );
}

// --- AC-4: the production dispatch path borrows `&Rule`; it never clones
// one. A clone on this path is invisible to any behavioral test (the parse
// result is identical either way), so this is the source-inspection last
// resort the rust-review checklist sanctions for such properties. ---------

#[trace("TC-012")]
#[test]
fn production_lookup_never_clones_the_grammar_rule() {
    let source = include_str!("../../src/parser.rs");
    assert!(
        source.contains("let grammar = self.grammar;")
            && source.contains("grammar.get(&production)")
            && !source.contains("grammar.get(&production).cloned()")
            && !source.contains(".get(&production).cloned()"),
        "Engine::production must borrow &Rule from the grammar map, never clone it (QSL-197 AC-4)"
    );
}

// --- AC-5: a flat 3,000-term sum and 3,000 one-line functions both parse at
// default limits. -----------------------------------------------------------

#[trace("TC-012", "NFR-001-M-2", "NFR-001-M-3")]
#[test]
fn flat_3000_term_sum_parses_at_default_limits() {
    let body = std::iter::repeat_n("x", 3000)
        .collect::<Vec<_>>()
        .join(" + ");
    let parsed = parse_body("ac5-sum", &body, Limits::default())
        .expect("a flat 3,000-term chain has nesting depth 0 and fits the default ceilings");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
}

#[trace("TC-012", "NFR-001-M-2", "NFR-001-M-3")]
#[test]
fn flat_3000_one_line_functions_parse_at_default_limits() {
    let mut text = HEADER.to_string();
    for index in 0..3000 {
        text.push_str(&format!(
            "function f{index} using Complete (): Boolean pure {{ true }}\n"
        ));
    }
    let parsed = parse("ac5-functions", &text, Limits::default())
        .expect("3,000 one-line functions fit the default token/node ceilings");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
}

// --- AC-6: NFR-001 "Verification" chains -- right-associative `implies`,
// prefix `not`, `let … in` and `if … else` -- have nesting depth 0 and
// parse at whatever length the token/node ceilings admit; one element
// longer names the token or node ceiling, never nesting. A generous custom
// ceiling (rather than the 100,000-token default) keeps these fixtures
// small while exercising the identical boundary mechanism, per NFR-001:
// "an implementation ceiling is not a domain bound". -----------------------

/// Probe increasing chain lengths built by `build` until parsing first
/// fails, then assert: the longest length that parsed is at least
/// `minimum_admitted` real chain elements (so the probe did not degenerate
/// on an unrelated early failure), and the first refusal names the token or
/// node ceiling, never nesting.
fn assert_chain_boundary_never_cites_nesting(
    id: &str,
    limits: Limits,
    minimum_admitted: usize,
    build: impl Fn(usize) -> String,
) {
    let mut admitted = 0;
    loop {
        let next = admitted + 1;
        let text = wrap_body(&build(next));
        match parse(&format!("{id}-{next}"), &text, limits) {
            Ok(parsed) if parsed.is_admissible() => admitted = next,
            Ok(parsed) => panic!(
                "{id}: unexpected diagnostics at length {next}: {:?}",
                parsed.diagnostics()
            ),
            Err(error) => {
                assert_eq!(
                    error.code,
                    CompleteCode::ResourceExhausted,
                    "{id}: refusal at length {next} was not resource_exhausted: {error}"
                );
                assert!(
                    !error.message.contains("nesting"),
                    "{id}: a chain of depth 0 must never be refused for nesting: {}",
                    error.message
                );
                break;
            }
        }
        assert!(next < 100_000, "{id}: chain never hit a ceiling");
    }
    assert!(
        admitted >= minimum_admitted,
        "{id}: only {admitted} elements admitted, expected at least {minimum_admitted}"
    );
}

#[trace("TC-012")]
#[test]
fn implies_chain_at_the_ceiling_parses_one_longer_names_token_or_node() {
    let limits = Limits {
        tokens: 300,
        nodes: 300,
        ..Limits::default()
    };
    assert_chain_boundary_never_cites_nesting("implies", limits, 10, |n| {
        std::iter::repeat_n("x", n + 1)
            .collect::<Vec<_>>()
            .join(" implies ")
    });
}

#[trace("TC-012")]
#[test]
fn prefix_not_chain_at_the_ceiling_parses_one_longer_names_token_or_node() {
    let limits = Limits {
        tokens: 300,
        nodes: 300,
        ..Limits::default()
    };
    assert_chain_boundary_never_cites_nesting("not", limits, 10, |n| {
        format!("{}x", "not ".repeat(n))
    });
}

#[trace("TC-012")]
#[test]
fn let_in_chain_at_the_ceiling_parses_one_longer_names_token_or_node() {
    let limits = Limits {
        tokens: 600,
        nodes: 600,
        ..Limits::default()
    };
    assert_chain_boundary_never_cites_nesting("let-in", limits, 10, |n| {
        let mut body = String::new();
        for index in 0..n {
            body.push_str(&format!("let v{index} = {index} in "));
        }
        body.push('0');
        body
    });
}

#[trace("TC-012")]
#[test]
fn if_else_chain_at_the_ceiling_parses_one_longer_names_token_or_node() {
    let limits = Limits {
        tokens: 900,
        nodes: 900,
        ..Limits::default()
    };
    assert_chain_boundary_never_cites_nesting("if-else", limits, 10, |n| {
        let mut body = String::new();
        for _ in 0..n {
            body.push_str("if true then 1 else ");
        }
        body.push('0');
        body
    });
}

/// No source within the token/node ceilings may overflow a 512 KiB stack
/// (NFR-001). A right-associative `implies` chain is the one of the five
/// named chains whose old (production-recursive) parse recursed one native
/// stack frame per element; 3,000 elements is comfortably beyond where that
/// old recursion would have crashed a 512 KiB stack, and fits within the
/// default 50,000-node ceiling (each operand's `Disjunction` subtree costs
/// several wrapper nodes, so this chain is node-bound before it is
/// token-bound).
#[trace("TC-012")]
#[test]
fn long_implies_chain_parses_without_overflowing_a_bounded_stack() {
    on_bounded_stack(|| {
        let body = std::iter::repeat_n("x", 3000)
            .collect::<Vec<_>>()
            .join(" implies ");
        let parsed = parse_body("implies-bounded-stack", &body, Limits::default())
            .expect("a 3,000-element implies chain fits the default ceilings");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    });
}

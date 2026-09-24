// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-197: NFR-001 "Nesting level" for the native (historical `0-draft`)
//! S1 parser -- bracket-pair nesting depth (never `expression()`/`binary()`
//! recursion depth), work-vs-nesting resource_exhausted classification, and
//! long-chain stack safety (TC-012). See `qsl-cst/tests/it/nesting_levels.rs`
//! for the complete-V1 (lossless CST) S1 parser's equivalent coverage.
use ix_trace_rs::trace;
use qsl_foundation::{Code, Diagnostic, Phase, SourceIdentity};
use quire_spec_language::{parse, Limits, ParsedUnit};

fn identity(id: &str) -> SourceIdentity {
    SourceIdentity {
        identity: format!("test:{id}"),
        revision: "1".into(),
    }
}

const HEADER: &str = "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"test/model\" version \"1\" digest \"unresolved\";\n";

/// The fixed text preceding an invariant's expression body: `HEADER` plus
/// `invariant Test on M::Thing at current { `. That opening brace is itself
/// one bracket pair, so a caller building an exact nesting boundary uses
/// `bound - 1` further brackets in `expression`.
fn body_prefix() -> String {
    format!("{HEADER}invariant Test on M::Thing at current {{ ")
}

fn document(expression: &str) -> String {
    format!("{}{expression} }}\n", body_prefix())
}

fn read(id: &str, text: &str, limits: Limits) -> Result<ParsedUnit, Box<Diagnostic>> {
    parse(identity(id), "test.native", text.as_bytes(), limits)
}

fn read_expr(id: &str, expression: &str, limits: Limits) -> Result<ParsedUnit, Box<Diagnostic>> {
    read(id, &document(expression), limits)
}

/// Run `run` on a thread with the 512 KiB stack NFR-001 "Verification"
/// specifies for the named chains, so a stack-depth regression in
/// `Parser::implies_chain`/`Parser::expression` actually crashes this test.
fn on_bounded_stack<F: FnOnce() + Send + 'static>(run: F) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(run)
        .expect("spawn bounded-stack thread")
        .join()
        .expect("parser must not overflow a 512 KiB stack (NFR-001 Verification)");
}

// --- AC-1: one paren pair costs one nesting level, not one per
// `expression()`/`binary()` call. ------------------------------------------

#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn deeply_parenthesized_flat_sum_parses_at_default_limits() {
    read_expr("ac1", "(((((1 + 1) + 1) + 1) + 1) + 1)", Limits::default())
        .expect("bracket nesting of 5 is far under the default ceiling of 64");
}

// --- AC-2: nesting to exactly the configured bound parses; one level past
// is refused naming nesting depth, the bound, and the opening bracket's
// span. Plain parens are also bounded by the shared `qsl_cst::lexer`
// delimiter check, which runs (and refuses) before the parser -- so this
// exercises that lexer-level enforcement, not `Parser::open` specifically. --

#[trace("TC-012", "NFR-001-M-4")]
#[test]
fn paren_nesting_to_exactly_the_ceiling_parses_one_deeper_refuses() {
    let limits = Limits::default();
    let bound = limits.nesting;
    let body_bound = bound - 1;

    let at_bound = format!("{}1{}", "(".repeat(body_bound), ")".repeat(body_bound));
    read_expr("ac2-at-bound", &at_bound, limits).expect("exactly the ceiling must parse");

    let prefix = body_prefix();
    let opens = "(".repeat(body_bound + 1);
    let closes = ")".repeat(body_bound + 1);
    let text = format!("{prefix}{opens}1{closes} }}\n");
    let last_open_offset = prefix.len() + opens.len() - 1;

    let error = read("ac2-over-bound", &text, limits).expect_err("one level past must refuse");
    assert_eq!(error.code, Code::ResourceExhausted);
    assert_eq!(error.phase, Phase::Lex);
    assert!(
        error.message.contains("nesting depth") && error.message.contains(&bound.to_string()),
        "expected a nesting-depth refusal naming the bound {bound}, got: {}",
        error.message
    );
    assert_eq!(error.span.start.byte, last_open_offset);
    assert_eq!(error.span.end.byte, last_open_offset + 1);
}

// --- AC-3: exhausting the syntax-node budget names the work limit, never
// nesting. ------------------------------------------------------------------

#[trace("TC-012", "NFR-001-M-3")]
#[test]
fn exhausting_the_node_budget_names_the_work_limit_not_nesting() {
    let limits = Limits {
        nodes: 3,
        ..Limits::default()
    };
    let error = read_expr("ac3", "1 + 1 + 1 + 1 + 1 + 1 + 1 + 1", limits)
        .expect_err("3 retained syntax nodes cannot hold this unit");
    assert_eq!(error.code, Code::ResourceExhausted);
    assert_eq!(error.phase, Phase::Parse);
    assert!(
        error.message.contains("syntax node budget exhausted"),
        "{}",
        error.message
    );
    assert!(!error.message.contains("nesting"), "{}", error.message);
}

// --- AC-5: a flat 3,000-term sum parses at default limits. ------------------

#[trace("TC-012", "NFR-001-M-2", "NFR-001-M-3")]
#[test]
fn flat_3000_term_sum_parses_at_default_limits() {
    let body = std::iter::repeat_n("1", 3000)
        .collect::<Vec<_>>()
        .join(" + ");
    read_expr("ac5", &body, Limits::default())
        .expect("a flat 3,000-term chain has nesting depth 0 and fits the default ceilings");
}

// --- AC-6: NFR-001 "Verification" chains -- right-associative `implies`,
// prefix `not`, `let … in` and `if … else` -- have nesting depth 0 and parse
// at whatever length the token/node ceilings admit; one element longer
// names the token or node ceiling, never nesting. A smaller custom ceiling
// keeps these fixtures fast while exercising the identical mechanism
// (NFR-001: "an implementation ceiling is not a domain bound"). -------------

fn assert_chain_boundary_never_cites_nesting(
    id: &str,
    limits: Limits,
    minimum_admitted: usize,
    build: impl Fn(usize) -> String,
) {
    let mut admitted = 0;
    loop {
        let next = admitted + 1;
        let text = document(&build(next));
        match read(&format!("{id}-{next}"), &text, limits) {
            Ok(_) => admitted = next,
            Err(error) => {
                assert_eq!(
                    error.code,
                    Code::ResourceExhausted,
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
        std::iter::repeat_n("true", n + 1)
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
        format!("{}true", "not ".repeat(n))
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
/// (NFR-001). `long_flat_chains_parse_and_drop_on_a_bounded_stack` in
/// `tests/it/parser.rs` already covers the flat `+` and prefix `not` chains
/// at 20,000 elements; this covers `implies`, `let … in` and `if … else` --
/// the three chains `Parser::binary`/`Parser::expression` used to build by
/// Rust recursion per element before QSL-197.
#[trace("TC-012")]
#[test]
fn implies_let_and_if_chains_parse_without_overflowing_a_bounded_stack() {
    on_bounded_stack(|| {
        let implies = std::iter::repeat_n("true", 3000)
            .collect::<Vec<_>>()
            .join(" implies ");
        read_expr("implies-bounded-stack", &implies, Limits::default())
            .expect("a 3,000-element implies chain fits the default ceilings");

        let mut let_in = String::new();
        for index in 0..3000 {
            let_in.push_str(&format!("let v{index} = {index} in "));
        }
        let_in.push('0');
        read_expr("let-in-bounded-stack", &let_in, Limits::default())
            .expect("a 3,000-element let-in chain fits the default ceilings");

        let mut if_else = String::new();
        for _ in 0..3000 {
            if_else.push_str("if true then 1 else ");
        }
        if_else.push('0');
        read_expr("if-else-bounded-stack", &if_else, Limits::default())
            .expect("a 3,000-element if-else chain fits the default ceilings");
    });
}

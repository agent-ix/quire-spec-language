// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-722: 100,000-deep sources parse through the complete-V1 parser on a
//! 512 KiB thread under S1 limits raised to fit them (FR-256-AC-1). A
//! native recursion that grows with depth would overflow that stack.
use ix_trace_rs::trace;
use qsl_cst::{Limits, ParsedSource};
use qsl_foundation::SourceIdentity;

const DEPTH: usize = 100_000;

const HEADER: &str = concat!(
    "language \"ix:native\" edition \"1-draft\";\n",
    "profile Complete = \"quire.value.complete/v1\";\n",
);

fn function(body: &str) -> String {
    format!(
        "{HEADER}function f using Complete (a: Boolean, x: Integer): Integer pure {{ {body} }}\n"
    )
}

/// Limits raised past what any of these inputs needs.
fn raised() -> Limits {
    Limits::default()
        .with_source_bytes(usize::MAX)
        .with_tokens(usize::MAX)
        .with_nodes(usize::MAX)
}

/// Parse `text` on a 512 KiB thread under [`raised`] limits, which must
/// admit it.
fn parses_on_a_small_stack(text: String) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            let parsed: ParsedSource = qsl_cst::parse(
                SourceIdentity {
                    authority: "test".into(),
                    identity: "test:deep".into(),
                    revision_namespace: "test".into(),
                    revision: "1".into(),
                },
                "deep.native",
                text.as_bytes(),
                raised(),
            )
            .expect("the deep source parses under raised limits");
            assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        })
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("the parser must not overflow a 512 KiB stack");
}

#[trace("TC-722", "FR-256-AC-1")]
#[test]
fn a_100000_deep_bracket_nest_parses() {
    parses_on_a_small_stack(function(&format!(
        "{}1{}",
        "(".repeat(DEPTH),
        ")".repeat(DEPTH)
    )));
}

#[trace("TC-722", "FR-256-AC-1")]
#[test]
fn a_100000_long_not_chain_parses() {
    parses_on_a_small_stack(function(&format!("{}a", "not ".repeat(DEPTH))));
}

#[trace("TC-722", "FR-256-AC-1")]
#[test]
fn a_100000_term_sum_parses() {
    parses_on_a_small_stack(function(&vec!["x"; DEPTH].join(" + ")));
}

#[trace("TC-722", "FR-256-AC-1")]
#[test]
fn a_100000_long_else_if_chain_parses() {
    parses_on_a_small_stack(function(&format!("{}x", "if a then x else ".repeat(DEPTH))));
}

#[trace("TC-722", "FR-256-AC-1")]
#[test]
fn a_100000_deep_let_chain_parses() {
    parses_on_a_small_stack(function(&format!("{}x", "let v = x in ".repeat(DEPTH))));
}

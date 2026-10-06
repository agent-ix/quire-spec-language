// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-749: 100,000-deep sources parse through the root native parser, the
//! historical and the composed edition, on a 512 KiB thread under limits
//! raised to fit them (FR-256-AC-4). A native recursion that grows with depth
//! would overflow that stack, and a ceiling the caller cannot raise would
//! refuse the input.
use ix_trace_rs::trace;
use qsl_foundation::{Source, SourceIdentity};
use quire_spec_language::{parse, parse_native_source, Limits};

const DEPTH: usize = 100_000;

const HISTORICAL_HEADER: &str = "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"test/model\" version \"1\" digest \"unresolved\";\n";

const COMPOSED_HEADER: &str = r#"language "ix:native" edition "1-draft";
profile S = "quire.state.graph/v1" version "test:state" digest "unresolved-state";
model M = "test:orders-and-refunds" version "test:model" digest "unresolved-model";
"#;

fn identity() -> SourceIdentity {
    SourceIdentity {
        authority: "test".into(),
        identity: "test:deep".into(),
        revision_namespace: "test".into(),
        revision: "1".into(),
    }
}

/// Limits raised past what any of these inputs needs.
fn raised() -> Limits {
    Limits {
        source_bytes: usize::MAX,
        tokens: usize::MAX,
        nodes: usize::MAX,
    }
}

fn historical(expression: &str) -> String {
    format!("{HISTORICAL_HEADER}invariant Test on M::Thing at current {{ {expression} }}\n")
}

fn composed(expression: &str) -> String {
    format!(
        "{COMPOSED_HEADER}invariant Test using S on M::OrderView at current {{ {expression} }}\n"
    )
}

/// Parse `text` through `parse` on a 512 KiB thread under [`raised`] limits.
fn parses_historical(text: String) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            parse(identity(), "deep.native", text.as_bytes(), raised())
                .expect("the deep source parses under raised limits");
        })
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("the parser must not overflow a 512 KiB stack");
}

/// Parse `text` through `parse_native_source` on a 512 KiB thread under
/// [`raised`] limits.
fn parses_composed(text: String) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            let source = Source::read(identity(), "deep.native", text.as_bytes(), usize::MAX)
                .expect("the deep source reads");
            parse_native_source(source, raised())
                .expect("the deep source parses under raised limits");
        })
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("the parser must not overflow a 512 KiB stack");
}

fn brackets() -> String {
    format!("{}1{}", "(".repeat(DEPTH), ")".repeat(DEPTH))
}
fn not_chain() -> String {
    format!("{}true", "not ".repeat(DEPTH))
}
fn sum() -> String {
    vec!["1"; DEPTH].join(" + ")
}
fn else_if_chain() -> String {
    format!("{}0", "if true then 1 else ".repeat(DEPTH))
}
fn let_chain() -> String {
    format!("{}0", "let v = 0 in ".repeat(DEPTH))
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn historical_100000_deep_brackets_parse() {
    parses_historical(historical(&brackets()));
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn historical_100000_long_not_chain_parses() {
    parses_historical(historical(&not_chain()));
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn historical_100000_term_sum_parses() {
    parses_historical(historical(&sum()));
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn historical_100000_long_else_if_chain_parses() {
    parses_historical(historical(&else_if_chain()));
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn historical_100000_deep_let_chain_parses() {
    parses_historical(historical(&let_chain()));
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn composed_100000_deep_brackets_parse() {
    parses_composed(composed(&brackets()));
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn composed_100000_long_not_chain_parses() {
    parses_composed(composed(&not_chain()));
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn composed_100000_term_sum_parses() {
    parses_composed(composed(&sum()));
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn composed_100000_long_else_if_chain_parses() {
    parses_composed(composed(&else_if_chain()));
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn composed_100000_deep_let_chain_parses() {
    parses_composed(composed(&let_chain()));
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-749: 100,000-deep sources parse through the root native parser, the
//! historical and the composed edition, on a 512 KiB thread under limits
//! raised to fit them (FR-256-AC-4). A native recursion that grows with depth
//! would overflow that stack, and a ceiling the caller cannot raise would
//! refuse the input.
use ix_trace_rs::trace;
use qsl_foundation::{Source, SourceIdentity};
use quire_spec_language::linking::composed::{
    admit_namespace, ExpectedSource, SourceInventory, WorkLimits,
};
use quire_spec_language::syntax::composed::NativeUnit;
use quire_spec_language::{parse, parse_native_source, Limits};
use std::process::Command;

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
fn parses_historical(text: String, min_expressions: usize) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            let unit = parse(identity(), "deep.native", text.as_bytes(), raised())
                .expect("the deep source parses under raised limits");
            assert_eq!(unit.clauses().len(), 1, "one invariant clause");
            assert!(
                unit.expressions().len() >= min_expressions,
                "{} expressions, expected at least {min_expressions}",
                unit.expressions().len()
            );
            // Dropping a deep tree must not overflow this stack either.
            drop(unit);
        })
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("the parser must not overflow a 512 KiB stack");
}

/// Parse `text` through `parse_native_source` on a 512 KiB thread under
/// [`raised`] limits.
fn parses_composed(text: String, min_expressions: usize) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            let source = Source::read(identity(), "deep.native", text.as_bytes(), usize::MAX)
                .expect("the deep source reads");
            let unit = parse_native_source(source, raised())
                .expect("the deep source parses under raised limits");
            let NativeUnit::Composed(composed) = &unit else {
                panic!("the composed edition parses to a composed unit");
            };
            assert_eq!(
                composed.declarations().len(),
                1,
                "one invariant declaration"
            );
            assert!(
                composed.expressions().len() >= min_expressions,
                "{} expressions, expected at least {min_expressions}",
                composed.expressions().len()
            );
            drop(unit);
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
    parses_historical(historical(&brackets()), DEPTH);
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn historical_100000_long_not_chain_parses() {
    parses_historical(historical(&not_chain()), DEPTH);
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn historical_100000_term_sum_parses() {
    parses_historical(historical(&sum()), 2 * DEPTH - 1);
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn historical_100000_long_else_if_chain_parses() {
    parses_historical(historical(&else_if_chain()), DEPTH);
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn historical_100000_deep_let_chain_parses() {
    parses_historical(historical(&let_chain()), DEPTH);
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn composed_100000_deep_brackets_parse() {
    parses_composed(composed(&brackets()), DEPTH);
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn composed_100000_long_not_chain_parses() {
    parses_composed(composed(&not_chain()), DEPTH);
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn composed_100000_term_sum_parses() {
    parses_composed(composed(&sum()), 2 * DEPTH - 1);
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn composed_100000_long_else_if_chain_parses() {
    parses_composed(composed(&else_if_chain()), DEPTH);
}

#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn composed_100000_deep_let_chain_parses() {
    parses_composed(composed(&let_chain()), DEPTH);
}

/// The composed link path hands the caller's parser limits to every unit as
/// given: a source over the defaults admits under raised limits, and the
/// report carries them unchanged. A clamp at `admit_namespace` refuses it.
#[trace("TC-749", "FR-256-AC-4")]
#[test]
fn admit_namespace_uses_raised_parser_limits_as_given() {
    let text = composed(&sum());
    let source = Source::read(identity(), "deep.native", text.as_bytes(), usize::MAX)
        .expect("the deep source reads");
    let inventory = SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: vec![ExpectedSource {
            authority: "test".into(),
            identity: source.identity().clone(),
            digest: source.digest(),
        }],
    };
    let sources = [source];
    let defaulted = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(
        defaulted.namespace().is_none(),
        "the default parser limits refuse this source"
    );
    let report = admit_namespace(&inventory, &sources, WorkLimits::default(), raised());
    assert_eq!(report.parser_limits(), raised());
    assert!(report.issues().is_empty(), "{:?}", report.issues());
    let namespace = report.namespace().expect("namespace admitted");
    assert!(namespace.units()[0].expressions().len() >= 2 * DEPTH - 1);
}

/// `quire-spec parse` takes the three limits from its command line.
#[trace("TC-750", "FR-256-AC-5")]
#[test]
fn cli_parse_takes_caller_limits() {
    let temp = tempfile::tempdir().expect("temp dir");
    let file = temp.path().join("deep.native");
    std::fs::write(&file, historical(&brackets())).expect("write source");
    let parse = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_quire-spec"))
            .args(["parse", "test", "test:deep", "test", "1"])
            .arg(&file)
            .args(extra)
            .output()
            .expect("run quire-spec")
    };
    let refused = parse(&[]);
    assert_eq!(refused.status.code(), Some(22));
    assert!(
        String::from_utf8_lossy(&refused.stdout).contains("stage_limit_exceeded"),
        "{}",
        String::from_utf8_lossy(&refused.stdout)
    );
    let parsed = parse(&[
        "--source-bytes",
        "10000000",
        "--tokens",
        "1000000",
        "--nodes",
        "1000000",
    ]);
    assert_eq!(parsed.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&parsed.stdout).contains(r#""status":"parsed""#));
    for extra in [&["--tokens"][..], &["--tokens", "many"], &["--depth", "9"]] {
        assert_eq!(parse(extra).status.code(), Some(20), "{extra:?}");
    }
}

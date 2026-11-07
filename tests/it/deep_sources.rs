// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-749: 100,000-deep sources parse through the root native parser's
//! historical edition on a 512 KiB thread under limits
//! raised to fit them (FR-256-AC-4). A native recursion that grows with depth
//! would overflow that stack, and a ceiling the caller cannot raise would
//! refuse the input.
use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use quire_spec_language::{parse, Limits};
use std::process::Command;

const DEPTH: usize = 100_000;

const HISTORICAL_HEADER: &str = "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"test/model\";\n";

const CST_HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";

fn identity() -> SourceIdentity {
    SourceIdentity {
        authority: "test".into(),
        identity: "test:deep".into(),
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

/// `quire-spec parse` takes the four `s1.*` limits from its command line and
/// runs them through `qsl_cst::parse` (FR-002, FR-256-AC-5).
#[trace("TC-750", "FR-256-AC-5")]
#[test]
fn cli_parse_takes_caller_limits() {
    let temp = tempfile::tempdir().expect("temp dir");
    let file = temp.path().join("deep.native");
    std::fs::write(
        &file,
        format!(
            "{CST_HEADER}function f using v(): Boolean pure {{ {} }}\n",
            brackets()
        ),
    )
    .expect("write source");
    let parse = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_quire-spec"))
            .args(["parse", "test", "test:deep"])
            .arg(&file)
            .args(extra)
            .output()
            .expect("run quire-spec")
    };
    let refused = parse(&[]);
    assert_eq!(refused.status.code(), Some(22));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("stage_limit_exceeded"),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
    let parsed = parse(&[
        "--source-bytes",
        "10000000",
        "--tokens",
        "1000000",
        "--nodes",
        "10000000",
        "--work-units",
        "256",
    ]);
    assert_eq!(parsed.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&parsed.stdout).contains(r#""category":"success""#));
    for extra in [
        &["--tokens"][..],
        &["--tokens", "many"],
        &["--depth", "9"],
        &["--work-units"],
    ] {
        assert_eq!(parse(extra).status.code(), Some(20), "{extra:?}");
    }
}

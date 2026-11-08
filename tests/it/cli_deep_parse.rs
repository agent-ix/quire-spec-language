// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-750: `quire-spec parse` takes its S1 limits from the command line and
//! parses a 100,000-deep `1-draft` source under them (FR-256-AC-5). The
//! library-level deep cases are `qsl-cst/tests/it/deep_sources.rs`.
use ix_trace_rs::trace;
use std::process::Command;

const DEPTH: usize = 100_000;

const CST_HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";

fn brackets() -> String {
    format!("{}1{}", "(".repeat(DEPTH), ")".repeat(DEPTH))
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

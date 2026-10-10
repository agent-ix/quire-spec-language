// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#157: `cargo xtask test-suites <lane> [<cargo test args>]` -- runs
//! `cargo test <args>` and fails when a suite ran no tests without the lane
//! declaring it empty, or when a declared-empty suite is not empty.
//!
//! A suite that compiles and runs nothing prints the same `test result: ok.`
//! line as one that ran and passed, so "0 failed" cannot tell a proc-macro
//! crate that never had tests from a test target whose tests were all
//! `#[cfg]`-ed out. This gate keeps that distinction: each lane below names
//! its legitimately empty suites with the reason, and everything else must
//! run at least one test.
//!
//! **Suite identity.** A suite is named by its target's source file relative
//! to the workspace root (plus `doctests` for a library's doc-tests), taken
//! from `cargo test --no-run --message-format=json`, not from the text of the
//! run. The run's `Running ... (<executable>)` and `Doc-tests <crate>`
//! headers are mapped to that identity by executable path and library name;
//! a header naming neither is a finding, never a guess.
//!
//! **What the run text decides.** Only the counts: the first `running N
//! tests` line and the last `test result:` line between two headers. Test
//! output that happens to look like a summary precedes the real one, so it
//! does not count. This defends against mis-parsing, not against a test
//! author forging a whole suite.
//!
//! **Exit status.** `cargo` runs as a child, so its status is read directly:
//! a failing build or test run exits with cargo's own code and the suite
//! check is skipped.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use qsl_attrs::string_edge;

use crate::error::{Error, Result};

/// What a suite is.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SuiteKind {
    /// A unit-test or integration-test executable.
    Tests,
    /// A library's doc-tests.
    Doctests,
}

/// A Cargo suite: the source file of the target it comes from, relative to
/// the workspace root, and which of its suites it is.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SuiteId {
    /// The target's source file, relative to the workspace root.
    pub source: String,
    /// Tests or doc-tests.
    pub kind: SuiteKind,
}

impl fmt::Display for SuiteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            SuiteKind::Tests => f.write_str(&self.source),
            SuiteKind::Doctests => write!(f, "{} (doctests)", self.source),
        }
    }
}

/// A suite a lane declares empty, with the reason.
#[derive(Clone, Copy, Debug)]
pub struct EmptySuite {
    /// The target's source file, relative to the workspace root.
    pub source: &'static str,
    /// Tests or doc-tests.
    pub kind: SuiteKind,
    /// Why the suite has no tests.
    pub reason: &'static str,
}

impl EmptySuite {
    fn id(&self) -> SuiteId {
        SuiteId {
            source: self.source.to_owned(),
            kind: self.kind,
        }
    }
}

/// One `cargo test` invocation `make ci` runs, and the suites it declares
/// empty.
#[derive(Clone, Copy, Debug)]
pub struct Lane {
    /// The name `make ci` passes.
    pub name: &'static str,
    /// The suites that legitimately run no tests in this invocation.
    pub empty: &'static [EmptySuite],
}

const NO_DOC_EXAMPLE: &str = "no executable doc example in the library";

/// Measured from the retained `make ci` log of the QSL#670 gate (the
/// default-feature and all-feature workspace runs listed the same suites).
const WORKSPACE_EMPTY: &[EmptySuite] = &[
    EmptySuite {
        source: "qsl-attrs/src/lib.rs",
        kind: SuiteKind::Tests,
        reason:
            "proc-macro crate with no unit tests; its tests are the compile_fail integration target",
    },
    EmptySuite {
        source: "qsl-bench/src/bin/qsl-bench-probe.rs",
        kind: SuiteKind::Tests,
        reason: "benchmark probe binary; no #[test] item in it",
    },
    EmptySuite {
        source: "qsl-source/src/lib.rs",
        kind: SuiteKind::Tests,
        reason: "no unit tests in src/; the crate's tests are its tests/it target",
    },
    EmptySuite {
        source: "qsl-walk-grow/src/lib.rs",
        kind: SuiteKind::Tests,
        reason: "no unit tests in src/; the crate's tests are in its tests/ directory",
    },
    EmptySuite {
        source: "src/main.rs",
        kind: SuiteKind::Tests,
        reason: "the quire-spec binary entry point; no #[test] item in it",
    },
    EmptySuite {
        source: "xtask/src/main.rs",
        kind: SuiteKind::Tests,
        reason: "the xtask binary entry point; no #[test] item in it",
    },
    EmptySuite {
        source: "tools/arch-lint/lib.rs",
        kind: SuiteKind::Doctests,
        reason: NO_DOC_EXAMPLE,
    },
    EmptySuite {
        source: "qsl-analyze/src/lib.rs",
        kind: SuiteKind::Doctests,
        reason: NO_DOC_EXAMPLE,
    },
    EmptySuite {
        source: "qsl-attrs/src/lib.rs",
        kind: SuiteKind::Doctests,
        reason: NO_DOC_EXAMPLE,
    },
    EmptySuite {
        source: "qsl-bench/src/lib.rs",
        kind: SuiteKind::Doctests,
        reason: "its only doc code block is a `text` block, which rustdoc does not run",
    },
    EmptySuite {
        source: "qsl-eval/src/lib.rs",
        kind: SuiteKind::Doctests,
        reason: NO_DOC_EXAMPLE,
    },
    EmptySuite {
        source: "qsl-foundation/src/lib.rs",
        kind: SuiteKind::Doctests,
        reason: NO_DOC_EXAMPLE,
    },
    EmptySuite {
        source: "qsl-route/src/lib.rs",
        kind: SuiteKind::Doctests,
        reason: NO_DOC_EXAMPLE,
    },
    EmptySuite {
        source: "qsl-source/src/lib.rs",
        kind: SuiteKind::Doctests,
        reason: NO_DOC_EXAMPLE,
    },
    EmptySuite {
        source: "qsl-walk-grow/src/lib.rs",
        kind: SuiteKind::Doctests,
        reason: NO_DOC_EXAMPLE,
    },
    EmptySuite {
        source: "xtask/src/lib.rs",
        kind: SuiteKind::Doctests,
        reason: NO_DOC_EXAMPLE,
    },
];

/// The `cargo test` invocations of `make ci`, one lane each. The two `-p`
/// lanes declare nothing: every suite they run has tests.
pub const LANES: &[Lane] = &[
    Lane {
        name: "default-workspace",
        empty: WORKSPACE_EMPTY,
    },
    Lane {
        name: "default-qsl-semantics",
        empty: &[],
    },
    Lane {
        name: "default-qsl-cst",
        empty: &[],
    },
    Lane {
        name: "all-features-workspace",
        empty: WORKSPACE_EMPTY,
    },
];

/// Something the gate refuses.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Finding {
    /// A `Running`/`Doc-tests` header that names no suite of this build.
    #[error("unrecognised suite header (no matching target of this build): {header}")]
    UnknownSuite {
        /// The header line.
        header: String,
    },
    /// A suite printed no `running N tests` line.
    #[error("{suite}: no `running N tests` line")]
    NoRunningLine {
        /// The suite.
        suite: SuiteId,
    },
    /// A suite printed no readable `test result:` line.
    #[error("{suite}: no readable `test result:` line")]
    NoSummary {
        /// The suite.
        suite: SuiteId,
    },
    /// A suite's `running N tests` and its summary disagree.
    #[error("{suite}: `running {running}` but the summary accounts for {summed} tests")]
    CountMismatch {
        /// The suite.
        suite: SuiteId,
        /// The `running N tests` count.
        running: u64,
        /// The summary's passed + failed + ignored + measured.
        summed: u64,
    },
    /// The same suite ran twice in one invocation.
    #[error("{suite}: ran more than once")]
    Duplicate {
        /// The suite.
        suite: SuiteId,
    },
    /// A suite passed no tests and the lane does not declare it empty.
    #[error("{suite}: 0 tests passed and the lane does not declare it empty")]
    Undeclared {
        /// The suite.
        suite: SuiteId,
    },
    /// A suite the lane declares empty passed tests.
    #[error("{suite}: declared empty, but {passed} test(s) passed; remove the declaration")]
    DeclaredButRan {
        /// The suite.
        suite: SuiteId,
        /// Tests passed.
        passed: u64,
    },
    /// A suite the lane declares empty did not run.
    #[error("{suite}: declared empty, but it did not run")]
    DeclaredButAbsent {
        /// The suite.
        suite: SuiteId,
    },
    /// A test executable was built and never run.
    #[error("{suite}: built as a test target but never ran")]
    NeverRan {
        /// The suite.
        suite: SuiteId,
    },
    /// The invocation ran no suite at all.
    #[error("the invocation ran no test suite")]
    NoSuites,
}

/// The suites of one build: test executables by path and libraries by crate
/// name.
#[derive(Debug, Default)]
pub struct Artifacts {
    executables: BTreeMap<PathBuf, SuiteId>,
    doc_libraries: BTreeMap<String, SuiteId>,
}

impl Artifacts {
    /// Read `cargo test --no-run --message-format=json`'s stdout. A target
    /// outside `workspace_root` is a dependency, not a suite.
    #[string_edge]
    pub fn from_cargo_json(stdout: &str, workspace_root: &Path) -> serde_json::Result<Self> {
        let mut artifacts = Self::default();
        for line in stdout.lines().filter(|line| line.starts_with('{')) {
            let message: serde_json::Value = serde_json::from_str(line)?;
            if message.get("reason").and_then(|reason| reason.as_str()) != Some("compiler-artifact")
            {
                continue;
            }
            let target = &message["target"];
            let Some(source) = target["src_path"]
                .as_str()
                .and_then(|path| Path::new(path).strip_prefix(workspace_root).ok())
                .map(|path| path.to_string_lossy().into_owned())
            else {
                continue;
            };
            if let (Some(executable), Some(true)) = (
                message["executable"].as_str(),
                message["profile"]["test"].as_bool(),
            ) {
                artifacts.executables.insert(
                    PathBuf::from(executable),
                    SuiteId {
                        source: source.clone(),
                        kind: SuiteKind::Tests,
                    },
                );
            }
            if let (Some(true), Some(name)) = (target["doctest"].as_bool(), target["name"].as_str())
            {
                artifacts.doc_libraries.insert(
                    name.replace('-', "_"),
                    SuiteId {
                        source,
                        kind: SuiteKind::Doctests,
                    },
                );
            }
        }
        Ok(artifacts)
    }

    fn expected_test_suites(&self) -> BTreeSet<&SuiteId> {
        self.executables.values().collect()
    }
}

/// One suite's counts from its summary line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuiteOutcome {
    /// The suite.
    pub id: SuiteId,
    /// Tests that passed.
    pub passed: u64,
}

#[derive(Clone, Copy)]
struct Summary {
    passed: u64,
    summed: u64,
}

enum Header<'a> {
    Executable(&'a str),
    Doctests(&'a str),
    Unreadable,
}

#[string_edge]
fn parse_header(line: &str) -> Option<Header<'_>> {
    if let Some(rest) = line.strip_prefix("     Running ") {
        let path = rest
            .strip_suffix(')')
            .and_then(|rest| rest.rsplit_once(" ("))
            .map(|(_, path)| path);
        return Some(path.map_or(Header::Unreadable, Header::Executable));
    }
    let name = line.strip_prefix("   Doc-tests ")?;
    Some(Header::Doctests(name.trim()))
}

#[string_edge]
fn parse_running(line: &str) -> Option<u64> {
    let mut words = line.strip_prefix("running ")?.split(' ');
    let count = words.next()?.parse().ok()?;
    let noun = words.next()?;
    if words.next().is_some() || (noun != "test" && noun != "tests") {
        return None;
    }
    Some(count)
}

#[string_edge]
fn parse_summary(line: &str) -> Option<Summary> {
    let rest = line.strip_prefix("test result: ")?;
    let (status, counts) = rest.split_once(". ")?;
    if status != "ok" && status != "FAILED" {
        return None;
    }
    let mut fields = counts.split("; ");
    let mut count =
        |suffix: &str| -> Option<u64> { fields.next()?.strip_suffix(suffix)?.parse().ok() };
    let passed = count(" passed")?;
    let failed = count(" failed")?;
    let ignored = count(" ignored")?;
    let measured = count(" measured")?;
    Some(Summary {
        passed,
        summed: passed
            .checked_add(failed)?
            .checked_add(ignored)?
            .checked_add(measured)?,
    })
}

struct Section {
    id: SuiteId,
    running: Option<u64>,
    summary: Option<Summary>,
}

/// The suites a `cargo test` run printed, read line by line.
pub struct RunParser<'a> {
    artifacts: &'a Artifacts,
    workspace_root: &'a Path,
    current: Option<Section>,
    suites: Vec<SuiteOutcome>,
    findings: Vec<Finding>,
}

impl<'a> RunParser<'a> {
    /// A parser resolving headers against `artifacts`; `workspace_root`
    /// anchors the relative executable paths cargo prints.
    pub fn new(artifacts: &'a Artifacts, workspace_root: &'a Path) -> Self {
        Self {
            artifacts,
            workspace_root,
            current: None,
            suites: Vec::new(),
            findings: Vec::new(),
        }
    }

    /// Feed one output line (no line terminator).
    pub fn feed(&mut self, line: &str) {
        if let Some(header) = parse_header(line) {
            self.close();
            let id = match header {
                Header::Executable(path) => self
                    .artifacts
                    .executables
                    .get(&self.workspace_root.join(path)),
                Header::Doctests(name) => self.artifacts.doc_libraries.get(name),
                Header::Unreadable => None,
            };
            match id {
                Some(id) => {
                    self.current = Some(Section {
                        id: id.clone(),
                        running: None,
                        summary: None,
                    });
                }
                None => self.findings.push(Finding::UnknownSuite {
                    header: line.to_owned(),
                }),
            }
            return;
        }
        let Some(section) = self.current.as_mut() else {
            return;
        };
        if section.running.is_none() {
            section.running = parse_running(line);
        }
        if let Some(summary) = parse_summary(line) {
            section.summary = Some(summary);
        }
    }

    fn close(&mut self) {
        let Some(section) = self.current.take() else {
            return;
        };
        let Section {
            id,
            running,
            summary,
        } = section;
        let Some(summary) = summary else {
            self.findings.push(Finding::NoSummary { suite: id });
            return;
        };
        let Some(running) = running else {
            self.findings.push(Finding::NoRunningLine { suite: id });
            return;
        };
        if running != summary.summed {
            self.findings.push(Finding::CountMismatch {
                suite: id,
                running,
                summed: summary.summed,
            });
        } else if self.suites.iter().any(|seen| seen.id == id) {
            self.findings.push(Finding::Duplicate { suite: id });
        } else {
            self.suites.push(SuiteOutcome {
                id,
                passed: summary.passed,
            });
        }
    }

    /// End of output: the suites that ran and what was wrong with the rest.
    pub fn finish(mut self) -> (Vec<SuiteOutcome>, Vec<Finding>) {
        self.close();
        (self.suites, self.findings)
    }
}

/// Everything wrong with a run against its lane, in a stable order.
pub fn evaluate(
    lane: &Lane,
    artifacts: &Artifacts,
    suites: &[SuiteOutcome],
    mut findings: Vec<Finding>,
) -> Vec<Finding> {
    let declared: BTreeSet<SuiteId> = lane.empty.iter().map(EmptySuite::id).collect();
    let mut ran = BTreeSet::new();
    for suite in suites {
        ran.insert(&suite.id);
        match (suite.passed, declared.contains(&suite.id)) {
            (0, false) => findings.push(Finding::Undeclared {
                suite: suite.id.clone(),
            }),
            (0, true) => {}
            (passed, true) => findings.push(Finding::DeclaredButRan {
                suite: suite.id.clone(),
                passed,
            }),
            (_, false) => {}
        }
    }
    for suite in &declared {
        if !ran.contains(suite) {
            findings.push(Finding::DeclaredButAbsent {
                suite: suite.clone(),
            });
        }
    }
    for suite in artifacts.expected_test_suites() {
        if !ran.contains(suite) {
            findings.push(Finding::NeverRan {
                suite: suite.clone(),
            });
        }
    }
    if suites.is_empty() && findings.is_empty() {
        findings.push(Finding::NoSuites);
    }
    findings
}

#[string_edge]
fn lane_named(name: &str) -> Option<&'static Lane> {
    LANES.iter().find(|lane| lane.name == name)
}

fn cargo() -> OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into())
}

fn io_error(what: &'static str) -> impl FnOnce(io::Error) -> Error {
    move |source| Error::TestSuitesIo { what, source }
}

fn cargo_failed(step: &'static str, status: std::process::ExitStatus) -> Error {
    Error::TestSuitesCargoFailed {
        step,
        status: status.code(),
    }
}

/// Build every test target `cargo test <args>` will run and read its suite
/// identities. Compiler diagnostics go to stderr as cargo renders them.
fn read_artifacts(
    workspace_root: &Path,
    cargo_args: &[OsString],
    target_dir: Option<&Path>,
) -> Result<Artifacts> {
    let mut command = Command::new(cargo());
    command
        .args([
            "test",
            "--no-run",
            "--message-format=json-render-diagnostics",
        ])
        .args(cargo_args)
        .current_dir(workspace_root)
        .stdin(Stdio::null())
        .stderr(Stdio::inherit());
    if let Some(target_dir) = target_dir {
        command.env("CARGO_TARGET_DIR", target_dir);
    }
    let output = command
        .output()
        .map_err(io_error("cannot spawn cargo test --no-run"))?;
    if !output.status.success() {
        return Err(cargo_failed("cargo test --no-run", output.status));
    }
    Artifacts::from_cargo_json(&String::from_utf8_lossy(&output.stdout), workspace_root)
        .map_err(|source| Error::TestSuitesArtifacts { source })
}

/// Run `cargo test <args>`, echoing its output as it arrives and feeding it to
/// the parser. Stdout and stderr share one pipe so headers and summaries keep
/// their order.
fn run_tests(
    workspace_root: &Path,
    cargo_args: &[OsString],
    target_dir: Option<&Path>,
    parser: &mut RunParser<'_>,
    echo: &mut impl Write,
) -> Result<()> {
    let (reader, writer) = io::pipe().map_err(io_error("cannot create the output pipe"))?;
    let mut command = Command::new(cargo());
    command
        .arg("test")
        .args(cargo_args)
        .current_dir(workspace_root)
        .env("CARGO_TERM_COLOR", "never")
        .stdin(Stdio::null())
        .stdout(
            writer
                .try_clone()
                .map_err(io_error("cannot share the output pipe"))?,
        )
        .stderr(writer);
    if let Some(target_dir) = target_dir {
        command.env("CARGO_TARGET_DIR", target_dir);
    }
    let mut child = command
        .spawn()
        .map_err(io_error("cannot spawn cargo test"))?;
    // The command owns the pipe's write ends; reading reaches EOF only once
    // the child's copies are the last ones.
    drop(command);
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        let read = reader
            .read_until(b'\n', &mut buffer)
            .map_err(io_error("cannot read cargo test output"))?;
        if read == 0 {
            break;
        }
        echo.write_all(&buffer)
            .map_err(io_error("cannot echo cargo test output"))?;
        let line = String::from_utf8_lossy(&buffer);
        parser.feed(line.trim_end_matches(['\n', '\r']));
    }
    let status = child
        .wait()
        .map_err(io_error("cannot wait for cargo test"))?;
    if status.success() {
        Ok(())
    } else {
        Err(cargo_failed("cargo test", status))
    }
}

/// Run the lane's `cargo test` and check its suites. `target_dir` overrides
/// cargo's own build directory for both cargo invocations; `echo` receives
/// the test run's output as it arrives.
pub fn run_lane(
    workspace_root: &Path,
    lane: &Lane,
    cargo_args: &[OsString],
    target_dir: Option<&Path>,
    echo: &mut impl Write,
) -> Result<String> {
    let artifacts = read_artifacts(workspace_root, cargo_args, target_dir)?;
    let mut parser = RunParser::new(&artifacts, workspace_root);
    run_tests(workspace_root, cargo_args, target_dir, &mut parser, echo)?;
    let (suites, findings) = parser.finish();
    let findings = evaluate(lane, &artifacts, &suites, findings);
    if !findings.is_empty() {
        let mut summary = format!(
            "test-suites: lane {}: {} finding(s)\n",
            lane.name,
            findings.len()
        );
        for finding in &findings {
            summary.push_str(&format!("  {finding}\n"));
        }
        return Err(Error::TestSuitesFound { summary });
    }
    let mut summary = format!(
        "test-suites: lane {}: {} suites ran tests, {} declared empty:\n",
        lane.name,
        suites.len().saturating_sub(lane.empty.len()),
        lane.empty.len()
    );
    for empty in lane.empty {
        summary.push_str(&format!("  {}: {}\n", empty.id(), empty.reason));
    }
    Ok(summary)
}

/// `cargo xtask test-suites <lane> [<cargo test args>]`.
pub fn run(workspace_root: &Path, operands: &[OsString]) -> Result<String> {
    let known = || {
        LANES
            .iter()
            .map(|lane| lane.name)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let (name, cargo_args) =
        operands
            .split_first()
            .ok_or_else(|| Error::TestSuitesUnknownLane {
                lane: String::new(),
                known: known(),
            })?;
    let name = name.to_string_lossy();
    let lane = lane_named(&name).ok_or_else(|| Error::TestSuitesUnknownLane {
        lane: name.to_string(),
        known: known(),
    })?;
    run_lane(
        workspace_root,
        lane,
        cargo_args,
        None,
        &mut io::stdout().lock(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "/ws";

    /// A real excerpt of the retained QSL#670 gate log: a zero-test unit
    /// suite, a nonzero integration suite, a nonzero unit suite, a second
    /// zero-test suite, and an empty doc-test suite.
    const REAL_LOG: &str = "\
test zone_check::scale::tests::tc_693_constants_scale_by_the_lcm_of_their_denominators ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.55s

     Running unittests src/lib.rs (target/debug/deps/qsl_attrs-5c690327f0f96fda)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/compile_fail.rs (target/debug/deps/compile_fail-e22539e50504e019)

running 1 test
test ui ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 27.80s

     Running unittests src/lib.rs (target/debug/deps/qsl_cst-6e92daaa2fdf7d0c)

running 2 tests
test a ... ok
test b ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

   Doc-tests qsl_attrs

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
";

    const ATTRS_UNIT_SECTION: &str = "     Running unittests src/lib.rs (target/debug/deps/qsl_attrs-5c690327f0f96fda)\n\nrunning 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n";
    const EMPTY_UNIT: &str = "qsl-attrs/src/lib.rs";
    const EMPTY_DOC: &str = "qsl-attrs/src/lib.rs (doctests)";

    fn artifact_line(executable: &str, source: &str, name: &str, doctest: bool) -> String {
        serde_json::json!({
            "reason": "compiler-artifact",
            "target": {
                "src_path": format!("{ROOT}/{source}"),
                "name": name,
                "doctest": doctest,
            },
            "profile": { "test": true },
            "executable": format!("{ROOT}/{executable}"),
        })
        .to_string()
    }

    fn artifacts() -> Artifacts {
        let lines = [
            artifact_line(
                "target/debug/deps/qsl_attrs-5c690327f0f96fda",
                "qsl-attrs/src/lib.rs",
                "qsl_attrs",
                true,
            ),
            artifact_line(
                "target/debug/deps/compile_fail-e22539e50504e019",
                "qsl-attrs/tests/compile_fail.rs",
                "compile_fail",
                false,
            ),
            artifact_line(
                "target/debug/deps/qsl_cst-6e92daaa2fdf7d0c",
                "qsl-cst/src/lib.rs",
                "qsl_cst",
                true,
            ),
            "a line that is not a JSON message is skipped".to_owned(),
        ];
        Artifacts::from_cargo_json(&lines.join("\n"), Path::new(ROOT)).expect("artifact lines")
    }

    fn lane(empty: &'static [EmptySuite]) -> Lane {
        Lane {
            name: "test",
            empty,
        }
    }

    const DECLARES_ATTRS: &[EmptySuite] = &[
        EmptySuite {
            source: "qsl-attrs/src/lib.rs",
            kind: SuiteKind::Tests,
            reason: "proc-macro crate",
        },
        EmptySuite {
            source: "qsl-attrs/src/lib.rs",
            kind: SuiteKind::Doctests,
            reason: "no doc example",
        },
    ];

    fn check(lane: &Lane, log: &str) -> Vec<String> {
        let artifacts = artifacts();
        let mut parser = RunParser::new(&artifacts, Path::new(ROOT));
        for line in log.lines() {
            parser.feed(line);
        }
        let (suites, findings) = parser.finish();
        evaluate(lane, &artifacts, &suites, findings)
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn undeclared_zero_test_suites_fail() {
        let findings = check(&lane(&[]), REAL_LOG);
        assert_eq!(
            findings,
            vec![
                format!("{EMPTY_UNIT}: 0 tests passed and the lane does not declare it empty"),
                format!("{EMPTY_DOC}: 0 tests passed and the lane does not declare it empty"),
            ]
        );
    }

    #[test]
    fn declared_zero_test_suites_pass_beside_nonzero_ones() {
        assert_eq!(check(&lane(DECLARES_ATTRS), REAL_LOG), Vec::<String>::new());
    }

    #[test]
    fn a_built_test_target_that_never_ran_fails() {
        let log = REAL_LOG.replace(ATTRS_UNIT_SECTION, "");
        assert_eq!(
            check(&lane(DECLARES_ATTRS), &log),
            vec![
                format!("{EMPTY_UNIT}: declared empty, but it did not run"),
                format!("{EMPTY_UNIT}: built as a test target but never ran"),
            ]
        );
    }

    #[test]
    fn a_declared_suite_that_now_has_tests_fails() {
        let log = REAL_LOG.replacen(
            "running 0 tests\n\ntest result: ok. 0 passed",
            "running 1 test\n\ntest result: ok. 1 passed",
            1,
        );
        assert_eq!(
            check(&lane(DECLARES_ATTRS), &log),
            vec![format!(
                "{EMPTY_UNIT}: declared empty, but 1 test(s) passed; remove the declaration"
            )]
        );
    }

    #[test]
    fn a_declared_suite_that_did_not_run_fails() {
        let log = REAL_LOG
            .split("   Doc-tests")
            .next()
            .expect("without doctests");
        assert_eq!(
            check(&lane(DECLARES_ATTRS), log),
            vec![format!("{EMPTY_DOC}: declared empty, but it did not run")]
        );
    }

    #[test]
    fn a_header_naming_no_target_of_the_build_fails() {
        let log = REAL_LOG.replace("qsl_cst-6e92daaa2fdf7d0c", "qsl_cst-ffffffffffffffff");
        let findings = check(&lane(DECLARES_ATTRS), &log);
        assert!(
            findings
                .iter()
                .any(|finding| finding.starts_with("unrecognised suite header")),
            "{findings:?}"
        );
        assert!(
            findings.iter().any(
                |finding| finding == "qsl-cst/src/lib.rs: built as a test target but never ran"
            ),
            "{findings:?}"
        );
    }

    #[test]
    fn a_doc_test_header_naming_no_library_fails() {
        let log = REAL_LOG.replace("Doc-tests qsl_attrs", "Doc-tests qsl_unknown");
        assert!(check(&lane(DECLARES_ATTRS), &log)
            .iter()
            .any(|finding| finding.starts_with("unrecognised suite header")));
    }

    #[test]
    fn a_suite_without_a_summary_fails() {
        let log = REAL_LOG
            .replace(
                "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s",
                "",
            );
        assert!(check(&lane(DECLARES_ATTRS), &log)
            .contains(&"qsl-cst/src/lib.rs: no readable `test result:` line".to_owned()));
    }

    #[test]
    fn a_malformed_summary_fails() {
        let log = REAL_LOG.replace(
            "test result: ok. 2 passed; 0 failed",
            "test result: ok. two passed; 0 failed",
        );
        assert!(check(&lane(DECLARES_ATTRS), &log)
            .contains(&"qsl-cst/src/lib.rs: no readable `test result:` line".to_owned()));
    }

    #[test]
    fn a_suite_without_a_running_line_fails() {
        let log = REAL_LOG.replace("running 2 tests\n", "");
        assert!(check(&lane(DECLARES_ATTRS), &log)
            .contains(&"qsl-cst/src/lib.rs: no `running N tests` line".to_owned()));
    }

    #[test]
    fn a_summary_that_disagrees_with_the_running_line_fails() {
        let log = REAL_LOG.replace("running 2 tests", "running 3 tests");
        assert!(check(&lane(DECLARES_ATTRS), &log).contains(
            &"qsl-cst/src/lib.rs: `running 3` but the summary accounts for 2 tests".to_owned()
        ));
    }

    #[test]
    fn test_output_that_looks_like_a_zero_summary_does_not_mask_the_real_one() {
        let log = REAL_LOG.replace(
            "test a ... ok\n",
            "test a ... ok\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n",
        );
        assert_eq!(check(&lane(DECLARES_ATTRS), &log), Vec::<String>::new());
    }

    #[test]
    fn an_invocation_that_ran_no_suite_fails() {
        let artifacts = Artifacts::default();
        let parser = RunParser::new(&artifacts, Path::new(ROOT));
        let (suites, findings) = parser.finish();
        assert_eq!(
            evaluate(&lane(&[]), &artifacts, &suites, findings),
            vec![Finding::NoSuites]
        );
    }

    #[test]
    fn the_same_suite_twice_fails() {
        let twice = format!("{REAL_LOG}{REAL_LOG}");
        assert!(check(&lane(DECLARES_ATTRS), &twice)
            .contains(&format!("{EMPTY_UNIT}: ran more than once")));
    }

    #[test]
    fn every_lane_names_each_empty_suite_once_with_a_reason() {
        for lane in LANES {
            let ids: BTreeSet<SuiteId> = lane.empty.iter().map(EmptySuite::id).collect();
            assert_eq!(ids.len(), lane.empty.len(), "{}", lane.name);
            assert!(lane
                .empty
                .iter()
                .all(|empty| !empty.reason.trim().is_empty()));
        }
    }

    #[test]
    fn an_unknown_lane_is_a_usage_error() {
        let error = run(Path::new(ROOT), &[OsString::from("nonsense")]).expect_err("unknown lane");
        assert!(
            matches!(error, Error::TestSuitesUnknownLane { .. }),
            "{error}"
        );
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn make_ci_calls_the_guard_for_each_lane_with_its_original_cargo_arguments() {
        let makefile =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../Makefile"))
                .expect("Makefile");
        for (lane, arguments) in [
            ("default-workspace", "--locked --workspace"),
            ("default-qsl-semantics", "--locked -p qsl-semantics"),
            ("default-qsl-cst", "--locked -p qsl-cst"),
            (
                "all-features-workspace",
                "--locked --workspace --all-features",
            ),
        ] {
            let recipe =
                format!("\tcargo run --locked --package xtask -- test-suites {lane} {arguments}\n");
            assert!(makefile.contains(&recipe), "missing recipe: {recipe}");
            assert!(
                !makefile.contains(&format!("\tcargo test {arguments}\n")),
                "unguarded recipe remains for {lane}"
            );
            assert!(lane_named(lane).is_some(), "{lane}");
        }
    }

    mod real_cargo {
        use super::*;
        use std::fs;

        /// A one-crate workspace: a library with neither unit tests nor doc
        /// examples, and one integration test whose outcome the caller picks.
        fn project(test_body: &str) -> tempfile::TempDir {
            let dir = tempfile::tempdir().expect("tempdir");
            let write = |path: &str, text: &str| {
                let path = dir.path().join(path);
                fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
                fs::write(path, text).expect("write");
            };
            write(
                "Cargo.toml",
                "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n",
            );
            write("src/lib.rs", "pub fn f() {}\n");
            write(
                "tests/it.rs",
                &format!("#[test]\nfn t() {{ {test_body} }}\n"),
            );
            dir
        }

        const LIB_EMPTY: &[EmptySuite] = &[
            EmptySuite {
                source: "src/lib.rs",
                kind: SuiteKind::Tests,
                reason: "fixture library has no unit tests",
            },
            EmptySuite {
                source: "src/lib.rs",
                kind: SuiteKind::Doctests,
                reason: "fixture library has no doc examples",
            },
        ];

        fn run_fixture(dir: &tempfile::TempDir, empty: &'static [EmptySuite]) -> Result<String> {
            let target = dir.path().join("target");
            run_lane(
                dir.path(),
                &lane(empty),
                &[],
                Some(&target),
                &mut io::sink(),
            )
        }

        #[test]
        fn undeclared_empty_suites_fail_a_real_cargo_run() {
            let dir = project("");
            let error = run_fixture(&dir, &[]).expect_err("undeclared empty suites");
            let Error::TestSuitesFound { summary } = &error else {
                panic!("expected a suite finding, got {error}");
            };
            assert!(summary.contains("src/lib.rs: 0 tests passed"), "{summary}");
            assert!(
                summary.contains("src/lib.rs (doctests): 0 tests passed"),
                "{summary}"
            );
            assert!(!summary.contains("tests/it.rs"), "{summary}");
            assert_eq!(error.exit_code(), 1);
        }

        #[test]
        fn declared_empty_suites_pass_a_real_cargo_run() {
            let dir = project("");
            let summary = run_fixture(&dir, LIB_EMPTY).expect("declared lane passes");
            assert!(
                summary.contains("1 suites ran tests, 2 declared empty"),
                "{summary}"
            );
        }

        #[test]
        fn a_failing_test_returns_cargos_own_exit_code() {
            let dir = project("panic!(\"boom\")");
            let error = run_fixture(&dir, LIB_EMPTY).expect_err("failing test");
            assert!(
                matches!(error, Error::TestSuitesCargoFailed { .. }),
                "{error}"
            );
            assert_eq!(error.exit_code(), 101);
        }

        #[test]
        fn a_build_failure_returns_cargos_own_exit_code() {
            let dir = project("this does not compile");
            let error = run_fixture(&dir, LIB_EMPTY).expect_err("build failure");
            assert!(
                matches!(error, Error::TestSuitesCargoFailed { .. }),
                "{error}"
            );
            assert_ne!(error.exit_code(), 0);
        }
    }
}

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
//! **Empty means registers nothing.** A suite a lane declares empty must show
//! `running 0 tests` and `0 filtered out`; a declared suite that gains a test
//! of any kind -- passing, `#[ignore]`d, measured, or hidden by a filter --
//! fails, so a stale declaration cannot hide new tests. Every other suite must
//! pass at least one test.
//!
//! **Which suites are expected.** Every built test executable, and unless the
//! arguments name a target kind (`--lib`, `--tests`, `--bin x`, ...), the
//! doc-tests of every selected package's library; an expected suite that does
//! not run fails whether or not a lane declares it. `--doc` is refused:
//! `cargo test --no-run` cannot list a doc-only run.
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

/// The zero-pass suites of the retained `make ci` log of the QSL#670 gate
/// (its default-feature and all-feature workspace runs listed the same
/// suites). That log is from an older head; the guard's own first run at a
/// new head is what checks this list against the tree.
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
        reason: NO_DOC_EXAMPLE,
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
        /// The summary's passed + failed + ignored + measured (`u64::MAX`
        /// when that overflows).
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
    /// A suite the lane declares empty registers tests: it ran some, ignored
    /// some, measured some, or filtered some out.
    #[error(
        "{suite}: declared empty, but it registers tests (running {running}: {} passed, {} \
         failed, {} ignored, {} measured, {} filtered out); remove the declaration",
        .counts.passed, .counts.failed, .counts.ignored, .counts.measured, .counts.filtered
    )]
    DeclaredButNotEmpty {
        /// The suite.
        suite: SuiteId,
        /// The `running N tests` count.
        running: u64,
        /// The summary's counts.
        counts: Counts,
    },
    /// A suite the lane declares empty did not run.
    #[error("{suite}: declared empty, but it did not run")]
    DeclaredButAbsent {
        /// The suite.
        suite: SuiteId,
    },
    /// A suite the cargo arguments select (a built test executable, or the
    /// doc-tests of a selected package's library) never ran.
    #[error("{suite}: selected by the cargo arguments but never ran")]
    NeverRan {
        /// The suite.
        suite: SuiteId,
    },
    /// The invocation ran no suite at all.
    #[error("the invocation ran no test suite")]
    NoSuites,
}

/// Why `cargo test --no-run --message-format=json` could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    /// A line that starts like a message is not JSON.
    #[error("not JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// A `compiler-artifact` message lacks a field of the expected type.
    #[error("a compiler-artifact message has no {field} of the expected type")]
    MalformedField {
        /// The field, as a path.
        field: &'static str,
    },
    /// Two test executables share one path but come from different targets.
    #[error("test executable {path} is claimed by both {first} and {second}")]
    DuplicateExecutable {
        /// The executable.
        path: PathBuf,
        /// The first target's suite.
        first: SuiteId,
        /// The second target's suite.
        second: SuiteId,
    },
    /// Two workspace libraries share one crate name, which is all a
    /// `Doc-tests <crate>` header says.
    #[error("library crate {name} is claimed by both {first} and {second}")]
    DuplicateLibrary {
        /// The crate name.
        name: String,
        /// The first library's suite.
        first: SuiteId,
        /// The second library's suite.
        second: SuiteId,
    },
}

/// A workspace library whose doc-tests `cargo test` may run.
#[derive(Debug)]
struct DocLibrary {
    id: SuiteId,
    manifest: PathBuf,
}

/// What the cargo arguments select, as far as the guard supports it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selection {
    /// Whether `cargo test` runs the selected libraries' doc-tests: true
    /// unless the arguments name a target kind.
    pub doctests: bool,
}

impl Selection {
    /// Read the target selection out of the arguments before any `--`.
    /// `--lib`, `--bins`, `--tests`, `--benches`, `--examples`, `--all-targets`
    /// and the named forms (`--bin x`, `--test=x`, ...) select targets and so
    /// leave out doc-tests, as cargo does. `--doc` is refused: cargo cannot
    /// enumerate a doc-only run with `--no-run`.
    #[string_edge]
    pub fn from_cargo_args(cargo_args: &[OsString]) -> Result<Self> {
        let mut doctests = true;
        for argument in cargo_args {
            let Some(argument) = argument.to_str() else {
                continue;
            };
            if argument == "--" {
                break;
            }
            let flag = argument.split('=').next().unwrap_or(argument);
            match flag {
                "--doc" => return Err(Error::TestSuitesUnsupportedSelection { flag: "--doc" }),
                "--lib" | "--bins" | "--bin" | "--tests" | "--test" | "--benches" | "--bench"
                | "--examples" | "--example" | "--all-targets" => doctests = false,
                _ => {}
            }
        }
        Ok(Self { doctests })
    }
}

/// The suites of one build: test executables by path and libraries by crate
/// name.
#[derive(Debug, Default)]
pub struct Artifacts {
    executables: BTreeMap<PathBuf, SuiteId>,
    doc_libraries: BTreeMap<String, DocLibrary>,
    /// Manifests of the packages the build compiled tests for.
    selected_manifests: BTreeSet<PathBuf>,
}

fn field<'a>(
    value: &'a serde_json::Value,
    name: &'static str,
    path: &'static str,
) -> std::result::Result<&'a serde_json::Value, ArtifactError> {
    value
        .get(name)
        .ok_or(ArtifactError::MalformedField { field: path })
}

impl Artifacts {
    /// Read `cargo test --no-run --message-format=json`'s stdout. A target
    /// outside `workspace_root` is a dependency, not a suite. Every
    /// `compiler-artifact` message must carry the fields read here with their
    /// cargo types, and one executable path or crate name must not name two
    /// targets.
    #[string_edge]
    pub fn from_cargo_json(
        stdout: &str,
        workspace_root: &Path,
    ) -> std::result::Result<Self, ArtifactError> {
        let mut artifacts = Self::default();
        for line in stdout.lines().filter(|line| line.starts_with('{')) {
            let message: serde_json::Value = serde_json::from_str(line)?;
            if message.get("reason").and_then(|reason| reason.as_str()) != Some("compiler-artifact")
            {
                continue;
            }
            let target = field(&message, "target", "target")?;
            let source_path = field(target, "src_path", "target.src_path")?
                .as_str()
                .ok_or(ArtifactError::MalformedField {
                    field: "target.src_path",
                })?;
            let name = field(target, "name", "target.name")?
                .as_str()
                .ok_or(ArtifactError::MalformedField {
                    field: "target.name",
                })?;
            let doctest = field(target, "doctest", "target.doctest")?
                .as_bool()
                .ok_or(ArtifactError::MalformedField {
                    field: "target.doctest",
                })?;
            let test_profile = field(field(&message, "profile", "profile")?, "test", "profile.test")?
                .as_bool()
                .ok_or(ArtifactError::MalformedField {
                    field: "profile.test",
                })?;
            let manifest = field(&message, "manifest_path", "manifest_path")?
                .as_str()
                .ok_or(ArtifactError::MalformedField {
                    field: "manifest_path",
                })?;
            let executable = match field(&message, "executable", "executable")? {
                serde_json::Value::Null => None,
                serde_json::Value::String(path) => Some(path.as_str()),
                _ => {
                    return Err(ArtifactError::MalformedField {
                        field: "executable",
                    })
                }
            };
            let Ok(source) = Path::new(source_path).strip_prefix(workspace_root) else {
                continue;
            };
            let source = source.to_string_lossy().into_owned();
            if let (Some(executable), true) = (executable, test_profile) {
                let id = SuiteId {
                    source: source.clone(),
                    kind: SuiteKind::Tests,
                };
                let path = PathBuf::from(executable);
                if let Some(first) = artifacts.executables.get(&path) {
                    if *first != id {
                        return Err(ArtifactError::DuplicateExecutable {
                            path,
                            first: first.clone(),
                            second: id,
                        });
                    }
                }
                artifacts.executables.insert(path, id);
                artifacts.selected_manifests.insert(PathBuf::from(manifest));
            }
            if doctest {
                let name = name.replace('-', "_");
                let id = SuiteId {
                    source,
                    kind: SuiteKind::Doctests,
                };
                if let Some(first) = artifacts.doc_libraries.get(&name) {
                    if first.id != id {
                        return Err(ArtifactError::DuplicateLibrary {
                            name,
                            first: first.id.clone(),
                            second: id,
                        });
                    }
                }
                artifacts.doc_libraries.insert(
                    name,
                    DocLibrary {
                        id,
                        manifest: PathBuf::from(manifest),
                    },
                );
            }
        }
        Ok(artifacts)
    }

    /// The suites a run of these artifacts must show: every built test
    /// executable and, when the selection includes doc-tests, the doc-tests of
    /// each library whose package had tests built.
    fn expected_suites(&self, selection: Selection) -> BTreeSet<SuiteId> {
        let mut expected: BTreeSet<SuiteId> = self.executables.values().cloned().collect();
        if selection.doctests {
            expected.extend(
                self.doc_libraries
                    .values()
                    .filter(|library| self.selected_manifests.contains(&library.manifest))
                    .map(|library| library.id.clone()),
            );
        }
        expected
    }
}

/// A suite summary's counts, as `libtest` prints them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Counts {
    /// Tests that passed.
    pub passed: u64,
    /// Tests that failed.
    pub failed: u64,
    /// Tests marked `#[ignore]` that were not run.
    pub ignored: u64,
    /// Benchmarks measured.
    pub measured: u64,
    /// Tests excluded by a filter.
    pub filtered: u64,
}

impl Counts {
    /// The tests `running N tests` counts: all but the filtered-out ones.
    fn registered(self) -> Option<u64> {
        self.passed
            .checked_add(self.failed)?
            .checked_add(self.ignored)?
            .checked_add(self.measured)
    }
}

/// One suite's counts from its summary line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuiteOutcome {
    /// The suite.
    pub id: SuiteId,
    /// The `running N tests` count.
    pub running: u64,
    /// The summary's counts.
    pub counts: Counts,
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
fn parse_summary(line: &str) -> Option<Counts> {
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
    let filtered = count(" filtered out")?;
    Some(Counts {
        passed,
        failed,
        ignored,
        measured,
        filtered,
    })
}

struct Section {
    id: SuiteId,
    running: Option<u64>,
    summary: Option<Counts>,
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
                Header::Doctests(name) => self
                    .artifacts
                    .doc_libraries
                    .get(name)
                    .map(|library| &library.id),
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
        let registered = summary.registered().unwrap_or(u64::MAX);
        if registered != running {
            self.findings.push(Finding::CountMismatch {
                suite: id,
                running,
                summed: registered,
            });
        } else if self.suites.iter().any(|seen| seen.id == id) {
            self.findings.push(Finding::Duplicate { suite: id });
        } else {
            self.suites.push(SuiteOutcome {
                id,
                running,
                counts: summary,
            });
        }
    }

    /// End of output: the suites that ran and what was wrong with the rest.
    pub fn finish(mut self) -> (Vec<SuiteOutcome>, Vec<Finding>) {
        self.close();
        (self.suites, self.findings)
    }
}

/// Everything wrong with a run against its lane, in a stable order. A
/// declared-empty suite must register nothing: no test run, ignored or
/// measured, and none filtered out. Any other suite must pass a test.
/// `expected` is every suite the cargo arguments select.
pub fn evaluate(
    lane: &Lane,
    expected: &BTreeSet<SuiteId>,
    suites: &[SuiteOutcome],
    mut findings: Vec<Finding>,
) -> Vec<Finding> {
    let declared: BTreeSet<SuiteId> = lane.empty.iter().map(EmptySuite::id).collect();
    let mut ran = BTreeSet::new();
    for suite in suites {
        ran.insert(&suite.id);
        if declared.contains(&suite.id) {
            if suite.running != 0 || suite.counts.filtered != 0 {
                findings.push(Finding::DeclaredButNotEmpty {
                    suite: suite.id.clone(),
                    running: suite.running,
                    counts: suite.counts,
                });
            }
        } else if suite.counts.passed == 0 {
            findings.push(Finding::Undeclared {
                suite: suite.id.clone(),
            });
        }
    }
    for suite in &declared {
        if !ran.contains(suite) {
            findings.push(Finding::DeclaredButAbsent {
                suite: suite.clone(),
            });
        }
    }
    for suite in expected {
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

/// The `cargo test --no-run` that enumerates the suites of `cargo test
/// <cargo_args>`: the same arguments, directory and build directory as the
/// run itself.
fn enumerate_command(
    workspace_root: &Path,
    cargo_args: &[OsString],
    target_dir: Option<&Path>,
) -> Command {
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
    command
}

/// The `cargo test <cargo_args>` run whose output is checked. Its standard
/// streams are left for the caller to connect.
fn test_command(
    workspace_root: &Path,
    cargo_args: &[OsString],
    target_dir: Option<&Path>,
) -> Command {
    let mut command = Command::new(cargo());
    command
        .arg("test")
        .args(cargo_args)
        .current_dir(workspace_root)
        .env("CARGO_TERM_COLOR", "never")
        .stdin(Stdio::null());
    if let Some(target_dir) = target_dir {
        command.env("CARGO_TARGET_DIR", target_dir);
    }
    command
}

/// Build every test target `cargo test <args>` will run and read its suite
/// identities. Compiler diagnostics go to stderr as cargo renders them.
fn read_artifacts(
    workspace_root: &Path,
    cargo_args: &[OsString],
    target_dir: Option<&Path>,
) -> Result<Artifacts> {
    let output = enumerate_command(workspace_root, cargo_args, target_dir)
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
    let mut command = test_command(workspace_root, cargo_args, target_dir);
    command
        .stdout(
            writer
                .try_clone()
                .map_err(io_error("cannot share the output pipe"))?,
        )
        .stderr(writer);
    let mut child = command
        .spawn()
        .map_err(io_error("cannot spawn cargo test"))?;
    // The command owns the pipe's write ends; reading reaches EOF only once
    // the child's copies are the last ones.
    drop(command);
    let streamed = stream_output(reader, parser, echo);
    let (status, echo_failure) = reap(&mut child, streamed)?;
    if let Some(source) = echo_failure {
        return Err(Error::TestSuitesIo {
            what: "cannot echo cargo test output",
            source,
        });
    }
    if status.success() {
        Ok(())
    } else {
        Err(cargo_failed("cargo test", status))
    }
}

/// Read the run's output to its end, feeding every line to the parser. When
/// the echo sink fails, the first such failure is kept and the output is still
/// read to the end, so the run finishes instead of being left writing into a
/// closed pipe, and returned. A read failure ends the loop at once.
fn stream_output(
    reader: io::PipeReader,
    parser: &mut RunParser<'_>,
    echo: &mut impl Write,
) -> Result<Option<io::Error>> {
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();
    let mut echo_failure = None;
    loop {
        buffer.clear();
        let read = reader
            .read_until(b'\n', &mut buffer)
            .map_err(io_error("cannot read cargo test output"))?;
        if read == 0 {
            break;
        }
        if echo_failure.is_none() {
            if let Err(source) = echo.write_all(&buffer) {
                echo_failure = Some(source);
            }
        }
        let line = String::from_utf8_lossy(&buffer);
        parser.feed(line.trim_end_matches(['\n', '\r']));
    }
    Ok(echo_failure)
}

/// Wait for `child` so no spawned process outlives the call. After a failed
/// read (its output can no longer be drained) the child is killed first; the
/// read failure is what is returned.
fn reap(
    child: &mut std::process::Child,
    streamed: Result<Option<io::Error>>,
) -> Result<(std::process::ExitStatus, Option<io::Error>)> {
    match streamed {
        Err(read_failure) => {
            // The child may already have exited; either way it is waited for.
            let _ = child.kill();
            let _ = child.wait();
            Err(read_failure)
        }
        Ok(echo_failure) => {
            let status = child
                .wait()
                .map_err(io_error("cannot wait for cargo test"))?;
            Ok((status, echo_failure))
        }
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
    let selection = Selection::from_cargo_args(cargo_args)?;
    let artifacts = read_artifacts(workspace_root, cargo_args, target_dir)?;
    let mut parser = RunParser::new(&artifacts, workspace_root);
    run_tests(workspace_root, cargo_args, target_dir, &mut parser, echo)?;
    let (suites, findings) = parser.finish();
    let findings = evaluate(lane, &artifacts.expected_suites(selection), &suites, findings);
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

    /// Sections copied verbatim from the retained QSL#670 gate log (an older
    /// head than this one): a nonzero unit suite, a zero-test unit suite, a
    /// zero-test binary suite, and the zero-test doc-test suites of the first
    /// two crates. The unit sections come from the log's unit-test phase and
    /// the doc-test sections from its doc-test phase, so the order here is
    /// Cargo's own order for this subset, not a contiguous slice of the log.
    const PRODUCER_LOG: &str = "\
     Running unittests src/lib.rs (target/debug/deps/qsl_analyze-ef5a3cd7850e747b)

running 5 tests
test zone_check::dbm::tests::tc_693_includes_orders_a_strict_bound_inside_its_non_strict_twin ... ok
test zone_check::dbm::tests::tc_693_operations_agree_with_the_grid_reference ... ok
test zone_check::dbm::tests::tc_693_zero_up_constrain_reset_give_the_stated_zones ... ok
test zone_check::scale::tests::tc_693_a_huge_constant_scales_and_computes_without_overflow ... ok
test zone_check::scale::tests::tc_693_constants_scale_by_the_lcm_of_their_denominators ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.55s

     Running unittests src/lib.rs (target/debug/deps/qsl_attrs-5c690327f0f96fda)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/bin/qsl-bench-probe.rs (target/debug/deps/qsl_bench_probe-6b51de41aa582e2f)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests qsl_analyze

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests qsl_attrs

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
";

    const ATTRS_UNIT_SECTION: &str = "     Running unittests src/lib.rs (target/debug/deps/qsl_attrs-5c690327f0f96fda)\n\nrunning 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n";
    const ANALYZE_DOC_SECTION: &str = "   Doc-tests qsl_analyze\n\nrunning 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n";
    const ANALYZE_UNIT: &str = "qsl-analyze/src/lib.rs";
    const ANALYZE_DOC: &str = "qsl-analyze/src/lib.rs (doctests)";
    const EMPTY_UNIT: &str = "qsl-attrs/src/lib.rs";
    const EMPTY_DOC: &str = "qsl-attrs/src/lib.rs (doctests)";
    const PROBE: &str = "qsl-bench/src/bin/qsl-bench-probe.rs";

    /// One `compiler-artifact` message as cargo prints it, with every field
    /// the guard reads.
    fn artifact_json(
        executable: Option<&str>,
        source_path: &str,
        name: &str,
        doctest: bool,
        manifest_path: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "reason": "compiler-artifact",
            "manifest_path": manifest_path,
            "target": {
                "src_path": source_path,
                "name": name,
                "doctest": doctest,
            },
            "profile": { "test": executable.is_some() },
            "executable": executable,
        })
    }

    fn workspace_artifact(
        executable: Option<&str>,
        source: &str,
        name: &str,
        doctest: bool,
        package: &str,
    ) -> String {
        artifact_json(
            executable.map(|path| format!("{ROOT}/{path}")).as_deref(),
            &format!("{ROOT}/{source}"),
            name,
            doctest,
            &format!("{ROOT}/{package}/Cargo.toml"),
        )
        .to_string()
    }

    fn artifacts_from(lines: &[String]) -> Artifacts {
        Artifacts::from_cargo_json(&lines.join("\n"), Path::new(ROOT)).expect("artifact lines")
    }

    /// The build behind [`PRODUCER_LOG`], plus the targets a build also
    /// reports and `cargo test` does not run as suites: a workspace library
    /// built only as a dependency (`qsl-cst`) and a registry dependency.
    fn artifacts() -> Artifacts {
        artifacts_from(&[
            workspace_artifact(
                Some("target/debug/deps/qsl_analyze-ef5a3cd7850e747b"),
                ANALYZE_UNIT,
                "qsl_analyze",
                true,
                "qsl-analyze",
            ),
            workspace_artifact(
                Some("target/debug/deps/qsl_attrs-5c690327f0f96fda"),
                EMPTY_UNIT,
                "qsl_attrs",
                true,
                "qsl-attrs",
            ),
            workspace_artifact(
                Some("target/debug/deps/qsl_bench_probe-6b51de41aa582e2f"),
                PROBE,
                "qsl-bench-probe",
                false,
                "qsl-bench",
            ),
            workspace_artifact(None, "qsl-cst/src/lib.rs", "qsl_cst", true, "qsl-cst"),
            artifact_json(
                None,
                "/registry/serde-1.0.0/src/lib.rs",
                "serde",
                true,
                "/registry/serde-1.0.0/Cargo.toml",
            )
            .to_string(),
            "a line that is not a JSON message is skipped".to_owned(),
        ])
    }

    fn lane(empty: &'static [EmptySuite]) -> Lane {
        Lane {
            name: "test",
            empty,
        }
    }

    const DECLARES_EMPTY: &[EmptySuite] = &[
        EmptySuite {
            source: "qsl-attrs/src/lib.rs",
            kind: SuiteKind::Tests,
            reason: "proc-macro crate",
        },
        EmptySuite {
            source: "qsl-bench/src/bin/qsl-bench-probe.rs",
            kind: SuiteKind::Tests,
            reason: "probe binary",
        },
        EmptySuite {
            source: "qsl-analyze/src/lib.rs",
            kind: SuiteKind::Doctests,
            reason: "no doc example",
        },
        EmptySuite {
            source: "qsl-attrs/src/lib.rs",
            kind: SuiteKind::Doctests,
            reason: "no doc example",
        },
    ];

    const WITH_DOCTESTS: Selection = Selection { doctests: true };
    const WITHOUT_DOCTESTS: Selection = Selection { doctests: false };

    fn check_selecting(lane: &Lane, selection: Selection, log: &str) -> Vec<String> {
        let artifacts = artifacts();
        let mut parser = RunParser::new(&artifacts, Path::new(ROOT));
        for line in log.lines() {
            parser.feed(line);
        }
        let (suites, findings) = parser.finish();
        evaluate(lane, &artifacts.expected_suites(selection), &suites, findings)
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn check(lane: &Lane, log: &str) -> Vec<String> {
        check_selecting(lane, WITH_DOCTESTS, log)
    }

    fn not_empty(suite: &str, running: u64, counts: [u64; 5]) -> String {
        let [passed, failed, ignored, measured, filtered] = counts;
        format!(
            "{suite}: declared empty, but it registers tests (running {running}: {passed} passed, \
             {failed} failed, {ignored} ignored, {measured} measured, {filtered} filtered out); \
             remove the declaration"
        )
    }

    fn never_ran(suite: &str) -> String {
        format!("{suite}: selected by the cargo arguments but never ran")
    }

    #[test]
    fn undeclared_zero_test_suites_fail() {
        let undeclared = |suite: &str| {
            format!("{suite}: 0 tests passed and the lane does not declare it empty")
        };
        assert_eq!(
            check(&lane(&[]), PRODUCER_LOG),
            vec![
                undeclared(EMPTY_UNIT),
                undeclared(PROBE),
                undeclared(ANALYZE_DOC),
                undeclared(EMPTY_DOC),
            ]
        );
    }

    #[test]
    fn declared_zero_test_suites_pass_beside_a_nonzero_one() {
        assert_eq!(
            check(&lane(DECLARES_EMPTY), PRODUCER_LOG),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_built_test_target_that_never_ran_fails() {
        let log = PRODUCER_LOG.replace(ATTRS_UNIT_SECTION, "");
        assert_eq!(
            check(&lane(DECLARES_EMPTY), &log),
            vec![
                format!("{EMPTY_UNIT}: declared empty, but it did not run"),
                never_ran(EMPTY_UNIT),
            ]
        );
    }

    #[test]
    fn a_declared_suite_that_gains_a_passing_test_fails() {
        let log = PRODUCER_LOG.replacen(
            "running 0 tests\n\ntest result: ok. 0 passed",
            "running 1 test\n\ntest result: ok. 1 passed",
            1,
        );
        assert_eq!(
            check(&lane(DECLARES_EMPTY), &log),
            vec![not_empty(EMPTY_UNIT, 1, [1, 0, 0, 0, 0])]
        );
    }

    /// The ignored-drift control: `libtest` prints `running 1 test` and
    /// `0 passed; 1 ignored` for a suite whose only test is `#[ignore]`.
    #[test]
    fn a_declared_suite_that_gains_an_ignored_test_fails() {
        let log = PRODUCER_LOG.replacen(
            "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored",
            "running 1 test\n\ntest result: ok. 0 passed; 0 failed; 1 ignored",
            1,
        );
        assert_eq!(
            check(&lane(DECLARES_EMPTY), &log),
            vec![not_empty(EMPTY_UNIT, 1, [0, 0, 1, 0, 0])]
        );
    }

    #[test]
    fn a_declared_suite_that_gains_a_measured_test_fails() {
        let log = PRODUCER_LOG.replacen(
            "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured",
            "running 1 test\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 1 measured",
            1,
        );
        assert_eq!(
            check(&lane(DECLARES_EMPTY), &log),
            vec![not_empty(EMPTY_UNIT, 1, [0, 0, 0, 1, 0])]
        );
    }

    #[test]
    fn a_declared_suite_whose_tests_are_all_filtered_out_is_not_empty() {
        let log = PRODUCER_LOG.replacen(
            "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out",
            "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out",
            1,
        );
        assert_eq!(
            check(&lane(DECLARES_EMPTY), &log),
            vec![not_empty(EMPTY_UNIT, 0, [0, 0, 0, 0, 4])]
        );
    }

    #[test]
    fn an_undeclared_suite_whose_tests_are_all_ignored_fails() {
        let log = PRODUCER_LOG.replace(
            "test result: ok. 5 passed; 0 failed; 0 ignored",
            "test result: ok. 0 passed; 0 failed; 5 ignored",
        );
        assert_eq!(
            check(&lane(DECLARES_EMPTY), &log),
            vec![format!(
                "{ANALYZE_UNIT}: 0 tests passed and the lane does not declare it empty"
            )]
        );
    }

    /// A declared doctest suite that does not run is refused as declared and
    /// as selected.
    #[test]
    fn a_missing_declared_doctest_suite_fails() {
        let log = PRODUCER_LOG.replace(ANALYZE_DOC_SECTION, "");
        assert_eq!(
            check(&lane(DECLARES_EMPTY), &log),
            vec![
                format!("{ANALYZE_DOC}: declared empty, but it did not run"),
                never_ran(ANALYZE_DOC),
            ]
        );
    }

    const DECLARES_NO_ANALYZE_DOC: &[EmptySuite] = &[
        EmptySuite {
            source: "qsl-attrs/src/lib.rs",
            kind: SuiteKind::Tests,
            reason: "proc-macro crate",
        },
        EmptySuite {
            source: "qsl-bench/src/bin/qsl-bench-probe.rs",
            kind: SuiteKind::Tests,
            reason: "probe binary",
        },
        EmptySuite {
            source: "qsl-attrs/src/lib.rs",
            kind: SuiteKind::Doctests,
            reason: "no doc example",
        },
    ];

    /// The doc-tests of a selected package's library are expected whether or
    /// not the lane declares them, so one cannot vanish unseen.
    #[test]
    fn a_missing_nonallowlisted_doctest_suite_fails() {
        let log = PRODUCER_LOG.replace(ANALYZE_DOC_SECTION, "");
        assert_eq!(
            check(&lane(DECLARES_NO_ANALYZE_DOC), &log),
            vec![never_ran(ANALYZE_DOC)]
        );
        assert_eq!(
            check(&lane(DECLARES_NO_ANALYZE_DOC), PRODUCER_LOG),
            vec![format!(
                "{ANALYZE_DOC}: 0 tests passed and the lane does not declare it empty"
            )],
            "present but undeclared is the other refusal"
        );
    }

    /// `--lib` (and the other target selections) run no doc-tests, so none
    /// is expected, while the unit suites still are.
    #[test]
    fn a_target_selection_expects_no_doctest_suite() {
        let (units, _) = PRODUCER_LOG
            .split_once("   Doc-tests")
            .expect("doc-test sections");
        let declares_units = lane(&DECLARES_EMPTY[..2]);
        assert_eq!(
            check_selecting(&declares_units, WITHOUT_DOCTESTS, units),
            Vec::<String>::new()
        );
        assert_eq!(
            check_selecting(&declares_units, WITH_DOCTESTS, units),
            vec![never_ran(ANALYZE_DOC), never_ran(EMPTY_DOC)]
        );
    }

    #[test]
    fn a_doctest_library_built_only_as_a_dependency_is_not_expected() {
        let artifacts = artifacts();
        let expected = artifacts.expected_suites(WITH_DOCTESTS);
        let ids: Vec<String> = expected.iter().map(ToString::to_string).collect();
        assert_eq!(
            ids,
            [ANALYZE_UNIT, ANALYZE_DOC, EMPTY_UNIT, EMPTY_DOC, PROBE],
            "neither qsl-cst nor the registry crate is a suite of this run"
        );
    }

    #[test]
    fn a_header_naming_no_target_of_the_build_fails() {
        let log = PRODUCER_LOG.replace("qsl_analyze-ef5a3cd7850e747b", "qsl_analyze-ffffffffffffffff");
        let findings = check(&lane(DECLARES_EMPTY), &log);
        assert!(
            findings
                .iter()
                .any(|finding| finding.starts_with("unrecognised suite header")),
            "{findings:?}"
        );
        assert!(findings.contains(&never_ran(ANALYZE_UNIT)), "{findings:?}");
    }

    #[test]
    fn a_doc_test_header_naming_no_library_fails() {
        let log = PRODUCER_LOG.replace("Doc-tests qsl_attrs", "Doc-tests qsl_unknown");
        assert!(check(&lane(DECLARES_EMPTY), &log)
            .iter()
            .any(|finding| finding.starts_with("unrecognised suite header")));
    }

    #[test]
    fn a_suite_without_a_summary_fails() {
        let log = PRODUCER_LOG.replace(
            "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.55s",
            "",
        );
        assert!(check(&lane(DECLARES_EMPTY), &log)
            .contains(&format!("{ANALYZE_UNIT}: no readable `test result:` line")));
    }

    #[test]
    fn a_malformed_summary_fails() {
        for broken in [
            "test result: ok. five passed; 0 failed; 0 ignored; 0 measured; 0 filtered out",
            "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured",
            "test result: maybe. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out",
        ] {
            let log = PRODUCER_LOG.replace(
                "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out",
                broken,
            );
            assert!(
                check(&lane(DECLARES_EMPTY), &log)
                    .contains(&format!("{ANALYZE_UNIT}: no readable `test result:` line")),
                "{broken}"
            );
        }
    }

    #[test]
    fn a_suite_without_a_running_line_fails() {
        let log = PRODUCER_LOG.replace("running 5 tests\n", "");
        assert!(check(&lane(DECLARES_EMPTY), &log)
            .contains(&format!("{ANALYZE_UNIT}: no `running N tests` line")));
    }

    #[test]
    fn a_summary_that_disagrees_with_the_running_line_fails() {
        let log = PRODUCER_LOG.replace("running 5 tests", "running 6 tests");
        assert!(check(&lane(DECLARES_EMPTY), &log).contains(&format!(
            "{ANALYZE_UNIT}: `running 6` but the summary accounts for 5 tests"
        )));
    }

    #[test]
    fn test_output_that_looks_like_a_zero_summary_does_not_mask_the_real_one() {
        let log = PRODUCER_LOG.replace(
            "test zone_check::scale::tests::tc_693_a_huge_constant_scales_and_computes_without_overflow ... ok\n",
            "test zone_check::scale::tests::tc_693_a_huge_constant_scales_and_computes_without_overflow ... ok\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n",
        );
        assert_eq!(
            check(&lane(DECLARES_EMPTY), &log),
            Vec::<String>::new()
        );
    }

    #[test]
    fn an_invocation_that_ran_no_suite_fails() {
        let artifacts = Artifacts::default();
        let parser = RunParser::new(&artifacts, Path::new(ROOT));
        let (suites, findings) = parser.finish();
        assert_eq!(
            evaluate(
                &lane(&[]),
                &artifacts.expected_suites(WITH_DOCTESTS),
                &suites,
                findings
            ),
            vec![Finding::NoSuites]
        );
    }

    #[test]
    fn the_same_suite_twice_fails() {
        let twice = format!("{PRODUCER_LOG}{PRODUCER_LOG}");
        assert!(check(&lane(DECLARES_EMPTY), &twice)
            .contains(&format!("{EMPTY_UNIT}: ran more than once")));
    }

    #[test]
    fn a_relative_and_an_absolute_executable_path_both_resolve() {
        let lines = [
            workspace_artifact(
                Some("target/debug/deps/relative-1111"),
                "pkg/src/lib.rs",
                "relative",
                false,
                "pkg",
            ),
            artifact_json(
                Some("/elsewhere/target/debug/deps/absolute-2222"),
                &format!("{ROOT}/pkg/tests/abs.rs"),
                "abs",
                false,
                &format!("{ROOT}/pkg/Cargo.toml"),
            )
            .to_string(),
        ];
        let artifacts = artifacts_from(&lines);
        let log = "\
     Running unittests src/lib.rs (target/debug/deps/relative-1111)

running 1 test

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/abs.rs (/elsewhere/target/debug/deps/absolute-2222)

running 1 test

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
";
        let mut parser = RunParser::new(&artifacts, Path::new(ROOT));
        for line in log.lines() {
            parser.feed(line);
        }
        let (suites, findings) = parser.finish();
        assert_eq!(findings, Vec::new());
        let ran: Vec<String> = suites.iter().map(|suite| suite.id.to_string()).collect();
        assert_eq!(ran, vec!["pkg/src/lib.rs", "pkg/tests/abs.rs"]);
        assert_eq!(
            evaluate(
                &lane(&[]),
                &artifacts.expected_suites(WITHOUT_DOCTESTS),
                &suites,
                findings
            ),
            Vec::new()
        );
    }

    fn decode(lines: &[String]) -> std::result::Result<Artifacts, ArtifactError> {
        Artifacts::from_cargo_json(&lines.join("\n"), Path::new(ROOT))
    }

    fn valid_artifact() -> serde_json::Value {
        artifact_json(
            Some("/ws/target/debug/deps/p-1111"),
            "/ws/pkg/src/lib.rs",
            "p",
            true,
            "/ws/pkg/Cargo.toml",
        )
    }

    #[test]
    fn a_line_that_starts_like_a_message_but_is_not_json_is_refused() {
        let error = decode(&["{\"reason\": \"compiler-artifact\"".to_owned()])
            .expect_err("invalid JSON");
        assert!(matches!(error, ArtifactError::Json(_)), "{error}");
    }

    #[test]
    fn a_compiler_artifact_with_a_missing_or_mistyped_field_is_refused() {
        let cases: [(&str, fn(&mut serde_json::Value)); 9] = [
            ("target", |m| {
                m.as_object_mut().expect("object").remove("target");
            }),
            ("target.src_path", |m| m["target"]["src_path"] = 7.into()),
            ("target.name", |m| m["target"]["name"] = 7.into()),
            ("target.doctest", |m| m["target"]["doctest"] = "yes".into()),
            ("profile", |m| {
                m.as_object_mut().expect("object").remove("profile");
            }),
            ("profile.test", |m| m["profile"]["test"] = "yes".into()),
            ("manifest_path", |m| {
                m.as_object_mut().expect("object").remove("manifest_path");
            }),
            ("executable", |m| m["executable"] = 7.into()),
            ("executable", |m| {
                m.as_object_mut().expect("object").remove("executable");
            }),
        ];
        for (field, break_it) in cases {
            let mut message = valid_artifact();
            break_it(&mut message);
            let error = decode(&[message.to_string()]).expect_err(field);
            assert!(
                matches!(&error, ArtifactError::MalformedField { field: named } if *named == field),
                "{field}: {error}"
            );
        }
        decode(&[valid_artifact().to_string()]).expect("the unbroken message decodes");
    }

    #[test]
    fn messages_that_are_not_compiler_artifacts_are_ignored_whatever_their_shape() {
        let artifacts = decode(&[
            r#"{"reason":"build-finished","success":true}"#.to_owned(),
            r#"{"reason":"compiler-message","message":7}"#.to_owned(),
            r#"{"no_reason":true}"#.to_owned(),
        ])
        .expect("decodes");
        assert!(artifacts.executables.is_empty() && artifacts.doc_libraries.is_empty());
    }

    #[test]
    fn two_targets_sharing_an_executable_path_are_refused() {
        let mut other = valid_artifact();
        other["target"]["src_path"] = "/ws/pkg/tests/other.rs".into();
        let error = decode(&[valid_artifact().to_string(), other.to_string()])
            .expect_err("duplicate executable");
        assert!(
            matches!(&error, ArtifactError::DuplicateExecutable { first, second, .. }
                if first.source == "pkg/src/lib.rs" && second.source == "pkg/tests/other.rs"),
            "{error}"
        );
    }

    #[test]
    fn the_same_artifact_reported_twice_is_one_suite() {
        let artifacts = decode(&[valid_artifact().to_string(), valid_artifact().to_string()])
            .expect("a repeated artifact decodes");
        assert_eq!(artifacts.executables.len(), 1);
        assert_eq!(artifacts.doc_libraries.len(), 1);
    }

    #[test]
    fn two_libraries_sharing_a_crate_name_are_refused() {
        let mut hyphenated = valid_artifact();
        hyphenated["target"]["name"] = "my-lib".into();
        hyphenated["target"]["src_path"] = "/ws/one/src/lib.rs".into();
        hyphenated["executable"] = serde_json::Value::Null;
        hyphenated["profile"]["test"] = false.into();
        let mut underscored = hyphenated.clone();
        underscored["target"]["name"] = "my_lib".into();
        underscored["target"]["src_path"] = "/ws/two/src/lib.rs".into();
        let error = decode(&[hyphenated.to_string(), underscored.to_string()])
            .expect_err("duplicate library");
        assert!(
            matches!(&error, ArtifactError::DuplicateLibrary { name, .. } if name == "my_lib"),
            "{error}"
        );
    }

    #[test]
    fn dependencies_outside_the_workspace_are_not_suites() {
        let mut outside_test = valid_artifact();
        outside_test["target"]["src_path"] = "/elsewhere/dep/src/lib.rs".into();
        outside_test["executable"] = "/ws/target/debug/deps/dep-9999".into();
        let artifacts = decode(&[outside_test.to_string()]).expect("decodes");
        assert!(artifacts.executables.is_empty() && artifacts.doc_libraries.is_empty());
        let artifacts = artifacts();
        let libraries: Vec<&str> = artifacts.doc_libraries.keys().map(String::as_str).collect();
        assert_eq!(libraries, ["qsl_analyze", "qsl_attrs", "qsl_cst"]);
        assert_eq!(artifacts.executables.len(), 3);
    }

    fn cargo_args(arguments: &[&str]) -> Vec<OsString> {
        arguments.iter().map(OsString::from).collect()
    }

    #[test]
    fn the_lane_arguments_select_doctests_unless_they_name_a_target_kind() {
        for (arguments, doctests) in [
            (&[][..], true),
            (&["--locked", "--workspace"][..], true),
            (&["--locked", "-p", "qsl-semantics"][..], true),
            (&["--locked", "--workspace", "--all-features"][..], true),
            (&["--workspace", "--", "--lib"][..], true),
            (&["--lib"][..], false),
            (&["--tests"][..], false),
            (&["--bins"][..], false),
            (&["--benches"][..], false),
            (&["--examples"][..], false),
            (&["--all-targets"][..], false),
            (&["--bin", "x"][..], false),
            (&["--test=it"][..], false),
            (&["-p", "a", "--lib", "--tests"][..], false),
        ] {
            let selection = Selection::from_cargo_args(&cargo_args(arguments))
                .unwrap_or_else(|error| panic!("{arguments:?}: {error}"));
            assert_eq!(selection.doctests, doctests, "{arguments:?}");
        }
    }

    #[test]
    fn a_doc_only_selection_is_refused_before_cargo_runs() {
        for arguments in [&["--doc"][..], &["-p", "a", "--doc"][..], &["--doc=1"][..]] {
            let error = Selection::from_cargo_args(&cargo_args(arguments)).expect_err("--doc");
            assert!(
                matches!(error, Error::TestSuitesUnsupportedSelection { flag: "--doc" }),
                "{error}"
            );
            assert_eq!(error.exit_code(), 2);
        }
        // No cargo process is involved: the root does not exist, so a spawn
        // would fail with an I/O error instead.
        let error = run_lane(
            Path::new("/nonexistent/qsl-157"),
            &lane(&[]),
            &cargo_args(&["--doc"]),
            None,
            &mut io::sink(),
        )
        .expect_err("--doc");
        assert!(
            matches!(error, Error::TestSuitesUnsupportedSelection { .. }),
            "{error}"
        );
    }

    fn arguments_of(command: &Command) -> Vec<OsString> {
        command.get_args().map(std::ffi::OsStr::to_os_string).collect()
    }

    fn environment_of(command: &Command, name: &str) -> Option<Option<OsString>> {
        command
            .get_envs()
            .find(|(key, _)| *key == std::ffi::OsStr::new(name))
            .map(|(_, value)| value.map(std::ffi::OsStr::to_os_string))
    }

    #[test]
    fn enumeration_and_run_get_the_same_arguments_directory_and_build_dir() {
        use std::os::unix::ffi::OsStringExt;
        let mut arguments = cargo_args(&["--locked", "--workspace", "--all-features"]);
        arguments.push(OsString::from_vec(vec![b'-', b'p', 0xff]));
        arguments.extend(cargo_args(&["--", "filter"]));
        let root = Path::new("/some/root");
        let target = Path::new("/some/target");

        let enumerate = enumerate_command(root, &arguments, Some(target));
        let run = test_command(root, &arguments, Some(target));

        assert_eq!(enumerate.get_program(), cargo());
        assert_eq!(run.get_program(), cargo());
        let enumerated = arguments_of(&enumerate);
        assert_eq!(
            enumerated[..3],
            cargo_args(&[
                "test",
                "--no-run",
                "--message-format=json-render-diagnostics"
            ])[..]
        );
        assert_eq!(enumerated[3..], arguments[..]);
        let ran = arguments_of(&run);
        assert_eq!(ran[..1], cargo_args(&["test"])[..]);
        assert_eq!(ran[1..], arguments[..]);
        assert_eq!(enumerate.get_current_dir(), Some(root));
        assert_eq!(run.get_current_dir(), Some(root));
        for command in [&enumerate, &run] {
            assert_eq!(
                environment_of(command, "CARGO_TARGET_DIR"),
                Some(Some(target.as_os_str().to_os_string()))
            );
        }
        assert_eq!(
            environment_of(&run, "CARGO_TERM_COLOR"),
            Some(Some(OsString::from("never")))
        );
    }

    #[test]
    fn without_a_build_dir_both_commands_inherit_the_callers() {
        let root = Path::new("/some/root");
        for command in [
            enumerate_command(root, &[], None),
            test_command(root, &[], None),
        ] {
            assert_eq!(environment_of(&command, "CARGO_TARGET_DIR"), None);
        }
    }

    #[test]
    fn a_read_failure_kills_and_reaps_the_child_and_keeps_the_original_error() {
        let mut child = Command::new(cargo())
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn cargo");
        let failure = Error::TestSuitesIo {
            what: "cannot read cargo test output",
            source: io::Error::other("pipe broke"),
        };
        let error = reap(&mut child, Err(failure)).expect_err("read failure");
        assert!(
            matches!(&error, Error::TestSuitesIo { what, source }
                if what.contains("read") && source.to_string() == "pipe broke"),
            "{error}"
        );
        assert!(
            child.try_wait().expect("try_wait").is_some(),
            "the child was waited for"
        );
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

        fn write(root: &Path, path: &str, text: &str) {
            let path = root.join(path);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            fs::write(path, text).expect("write");
        }

        fn findings_of(error: &Error) -> &str {
            let Error::TestSuitesFound { summary } = error else {
                panic!("expected a suite finding, got {error}");
            };
            summary
        }

        /// A declared-empty library that gains an `#[ignore]` test is no
        /// longer empty: genuine cargo prints `running 1 test` and `0 passed;
        /// 1 ignored`. Restoring the file restores the pass.
        #[test]
        fn a_declared_empty_library_gaining_an_ignored_test_fails_until_restored() {
            let dir = project("");
            run_fixture(&dir, LIB_EMPTY).expect("the baseline passes");

            write(
                dir.path(),
                "src/lib.rs",
                "pub fn f() {}\n#[test]\n#[ignore]\nfn parked() {}\n",
            );
            let error = run_fixture(&dir, LIB_EMPTY).expect_err("an ignored test");
            assert!(
                findings_of(&error).contains(
                    "src/lib.rs: declared empty, but it registers tests (running 1: 0 passed, \
                     0 failed, 1 ignored, 0 measured, 0 filtered out)"
                ),
                "{error}"
            );

            write(dir.path(), "src/lib.rs", "pub fn f() {}\n");
            run_fixture(&dir, LIB_EMPTY).expect("the restored library passes again");
        }

        /// A test filter that matches nothing leaves every suite with zero
        /// passes: the nonempty one is refused as undeclared, the declared
        /// ones stay empty.
        #[test]
        fn a_filter_that_matches_nothing_fails_the_nonempty_suite() {
            let dir = project("");
            let target = dir.path().join("target");
            let error = run_lane(
                dir.path(),
                &lane(LIB_EMPTY),
                &cargo_args(&["--", "no_such_test"]),
                Some(&target),
                &mut io::sink(),
            )
            .expect_err("a filter that matches nothing");
            let findings = findings_of(&error);
            assert!(
                findings.contains("tests/it.rs: 0 tests passed and the lane does not declare it empty"),
                "{findings}"
            );
            assert!(!findings.contains("declared empty"), "{findings}");
        }

        const WORKSPACE_FIXTURE_EMPTY: &[EmptySuite] = &[
            EmptySuite {
                source: "a/src/lib.rs",
                kind: SuiteKind::Tests,
                reason: "fixture library a has no unit tests",
            },
            EmptySuite {
                source: "a/src/lib.rs",
                kind: SuiteKind::Doctests,
                reason: "fixture library a has no doc examples",
            },
            EmptySuite {
                source: "b/src/lib.rs",
                kind: SuiteKind::Tests,
                reason: "fixture library b has no unit tests",
            },
            EmptySuite {
                source: "b/src/lib.rs",
                kind: SuiteKind::Doctests,
                reason: "fixture library b has no doc examples",
            },
        ];

        /// Two packages: `a` with a feature-gated integration test, and `b`.
        fn workspace() -> tempfile::TempDir {
            let dir = tempfile::tempdir().expect("tempdir");
            let root = dir.path();
            write(
                root,
                "Cargo.toml",
                "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
            );
            write(
                root,
                "a/Cargo.toml",
                "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
                 [features]\nextra = []\n\n\
                 [[test]]\nname = \"extra\"\npath = \"tests/extra.rs\"\nrequired-features = [\"extra\"]\n",
            );
            write(root, "a/src/lib.rs", "pub fn a() {}\n");
            write(root, "a/tests/base.rs", "#[test]\nfn t() {}\n");
            write(root, "a/tests/extra.rs", "#[test]\nfn t() {}\n");
            write(
                root,
                "b/Cargo.toml",
                "[package]\nname = \"b\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
            );
            write(root, "b/src/lib.rs", "pub fn b() {}\n");
            write(root, "b/tests/base.rs", "#[test]\nfn t() {}\n");
            dir
        }

        fn run_workspace(
            dir: &tempfile::TempDir,
            empty: &'static [EmptySuite],
            arguments: &[&str],
        ) -> Result<String> {
            let target = dir.path().join("target");
            run_lane(
                dir.path(),
                &lane(empty),
                &cargo_args(arguments),
                Some(&target),
                &mut io::sink(),
            )
        }

        /// `-p`, `--features`, `--workspace` and `--all-features` change the
        /// suites cargo runs; the guard enumerates and checks the same set.
        #[test]
        fn package_and_feature_flags_select_the_suites_the_guard_checks() {
            let dir = workspace();

            let only_a = run_workspace(&dir, &WORKSPACE_FIXTURE_EMPTY[..2], &["-p", "a"])
                .expect("-p a runs only a's suites");
            assert!(only_a.contains("1 suites ran tests, 2 declared empty"), "{only_a}");

            let with_feature =
                run_workspace(&dir, &WORKSPACE_FIXTURE_EMPTY[..2], &["-p", "a", "--features", "extra"])
                    .expect("the feature-gated test target runs");
            assert!(
                with_feature.contains("2 suites ran tests, 2 declared empty"),
                "{with_feature}"
            );

            let whole = run_workspace(&dir, WORKSPACE_FIXTURE_EMPTY, &["--workspace"])
                .expect("--workspace runs both packages");
            assert!(whole.contains("2 suites ran tests, 4 declared empty"), "{whole}");

            let all_features =
                run_workspace(&dir, WORKSPACE_FIXTURE_EMPTY, &["--workspace", "--all-features"])
                    .expect("--all-features enables a's extra test target");
            assert!(
                all_features.contains("3 suites ran tests, 4 declared empty"),
                "{all_features}"
            );

            let error = run_workspace(&dir, &WORKSPACE_FIXTURE_EMPTY[..2], &["--workspace"])
                .expect_err("b's empty suites are not declared");
            let findings = findings_of(&error);
            assert!(findings.contains("b/src/lib.rs: 0 tests passed"), "{findings}");
            assert!(findings.contains("b/src/lib.rs (doctests): 0 tests passed"), "{findings}");
        }

        /// `--lib` and `--tests` run no doc-tests, and the guard does not
        /// expect any; a lane that still declares the doc-tests is told they
        /// did not run.
        #[test]
        fn target_selection_flags_leave_out_the_doctest_suite() {
            let dir = workspace();
            let lib_only = run_workspace(&dir, &WORKSPACE_FIXTURE_EMPTY[..1], &["-p", "a", "--lib"])
                .expect("--lib runs a's unit suite only");
            assert!(lib_only.contains("0 suites ran tests, 1 declared empty"), "{lib_only}");

            let tests = run_workspace(&dir, &WORKSPACE_FIXTURE_EMPTY[..1], &["-p", "a", "--tests"])
                .expect("--tests runs unit and integration suites, not doc-tests");
            assert!(tests.contains("1 suites ran tests, 1 declared empty"), "{tests}");

            let error = run_workspace(&dir, &WORKSPACE_FIXTURE_EMPTY[..2], &["-p", "a", "--lib"])
                .expect_err("the declared doc-tests did not run");
            assert!(
                findings_of(&error).contains("a/src/lib.rs (doctests): declared empty, but it did not run"),
                "{error}"
            );
        }

        /// A build directory outside the project makes cargo print absolute
        /// executable paths in its `Running` headers, and the guard still
        /// resolves them; the environment variable reaches both cargo runs.
        #[test]
        fn a_build_dir_outside_the_project_is_used_and_its_absolute_headers_resolve() {
            let dir = project("");
            let elsewhere = tempfile::tempdir().expect("tempdir");
            run_lane(
                dir.path(),
                &lane(LIB_EMPTY),
                &[],
                Some(elsewhere.path()),
                &mut io::sink(),
            )
            .expect("absolute headers resolve");
            assert!(elsewhere.path().join("debug").is_dir());
            assert!(!dir.path().join("target").exists());
        }

        /// A sink that fails on the first line of output releases the test
        /// the fixture is blocked on, so the cargo run is still in progress
        /// when the failure happens. The guard must read the run to its end
        /// and wait for it: the marker the test writes last exists on return.
        struct ReleasingFailingSink {
            release: PathBuf,
        }

        impl Write for ReleasingFailingSink {
            fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
                fs::write(&self.release, b"").expect("release the blocked test");
                Err(io::Error::other("sink closed"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        #[test]
        fn a_failing_echo_sink_leaves_no_cargo_run_behind() {
            let dir = project(
                "let root = std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")); \
                 let started = std::time::Instant::now(); \
                 while !root.join(\"release\").exists() { \
                     assert!(started.elapsed().as_secs() < 300, \"never released\"); \
                     std::hint::spin_loop(); \
                 } \
                 std::fs::write(root.join(\"finished\"), b\"\").unwrap();",
            );
            let target = dir.path().join("target");
            let mut sink = ReleasingFailingSink {
                release: dir.path().join("release"),
            };
            let error = run_lane(dir.path(), &lane(LIB_EMPTY), &[], Some(&target), &mut sink)
                .expect_err("the sink fails");
            assert!(
                matches!(&error, Error::TestSuitesIo { what, source }
                    if what.contains("echo") && source.to_string() == "sink closed"),
                "{error}"
            );
            assert!(
                dir.path().join("finished").exists(),
                "the cargo run was still going when run_lane returned"
            );
        }
    }
}

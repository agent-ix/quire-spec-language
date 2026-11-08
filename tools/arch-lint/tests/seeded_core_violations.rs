// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-284-AC-1 and AC-4 through the built `arch-lint` binary: a stub QSL
//! workspace with the core's real crate names is clean, and each seeded
//! violation (a forbidden dependency written into a real manifest, an
//! ambient read written into core source) makes `qualified-core` exit 1 with
//! the stable failure message naming the pair or the file and line.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use ix_trace_rs::trace;

const CORE: [&str; 9] = [
    "qsl-foundation",
    "qsl-cst",
    "qsl-source",
    "qsl-forms",
    "qsl-semantics",
    "qsl-package",
    "qsl-eval",
    "qsl-route",
    "qsl-replay",
];

/// `(crate, dependency)` edges of a clean core, none above the core.
const CLEAN_EDGES: [(&str, &str); 3] = [
    ("qsl-cst", "qsl-foundation"),
    ("qsl-eval", "qsl-foundation"),
    ("qsl-replay", "qsl-eval"),
];

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("a parent directory")).expect("create directory");
    fs::write(path, contents).expect("write file");
}

/// A workspace of path-only stub crates: the nine core crates and the
/// above-core `qsl-analyze`, plus the given extra edges.
fn stub_workspace(extra_edges: &[(&str, &str)]) -> tempfile::TempDir {
    stub_workspace_with(&[], extra_edges)
}

/// [`stub_workspace`] with additional stub crates, named by `extra_crates`,
/// such as an argument parser or FCD's extraction frontend.
fn stub_workspace_with(extra_crates: &[&str], extra_edges: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join("qsl");
    let mut members: Vec<&str> = CORE.to_vec();
    members.push("qsl-analyze");
    let list: Vec<String> = members.iter().map(|name| format!("\"{name}\"")).collect();
    write(
        &root,
        "Cargo.toml",
        &format!(
            "[workspace]\nresolver = \"2\"\nmembers = [{}]\n",
            list.join(", ")
        ),
    );
    // Extra crates sit beside the workspace, not in it, so they are
    // dependencies and not workspace members.
    let all: Vec<(&str, bool)> = members
        .iter()
        .map(|name| (*name, false))
        .chain(extra_crates.iter().map(|name| (*name, true)))
        .collect();
    for (name, external) in all {
        let dependencies: String = CLEAN_EDGES
            .iter()
            .chain(extra_edges)
            .filter(|(from, _)| *from == name)
            .map(|(_, to)| {
                let prefix = match (external, extra_crates.contains(to)) {
                    (false, true) => "../../ext",
                    _ => "..",
                };
                format!("{to} = {{ path = \"{prefix}/{to}\" }}\n")
            })
            .collect();
        let base = if external {
            dir.path().join("ext")
        } else {
            root.clone()
        };
        write(
            &base,
            &format!("{name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
                 [dependencies]\n{dependencies}"
            ),
        );
        write(&base, &format!("{name}/src/lib.rs"), "pub fn stub() {}\n");
    }
    let lock = Command::new("cargo")
        .args(["generate-lockfile", "--offline", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("cargo runs");
    assert!(lock.status.success(), "lockfile: {lock:?}");
    dir
}

/// The QSL workspace root inside a stub workspace's temp dir.
fn root(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().join("qsl")
}

fn qualified_core(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arch-lint"))
        .args(["qualified-core", "--qsl"])
        .arg(root)
        .output()
        .expect("arch-lint runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[trace("TC-767", "FR-284-AC-1")]
#[test]
fn the_clean_stub_workspace_passes_the_binary() {
    let dir = stub_workspace(&[]);
    let output = qualified_core(&root(&dir));
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    let report = stdout(&output);
    assert!(report.contains("Direction: PASS"), "{report}");
    assert!(report.contains("Ambient input: PASS"), "{report}");
}

#[trace("TC-767", "FR-284-AC-1")]
#[test]
fn a_seeded_forbidden_dependency_fails_the_direction_gate() {
    let dir = stub_workspace(&[("qsl-eval", "qsl-analyze")]);
    let output = qualified_core(&root(&dir));
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let report = stdout(&output);
    assert!(report.contains("Direction: FAIL"), "{report}");
    assert!(report.contains("(qsl-eval, qsl-analyze)"), "{report}");
    assert!(report.contains("Ambient input: PASS"), "{report}");
}

#[trace("TC-768", "FR-284-AC-4")]
#[test]
fn a_seeded_ambient_read_fails_the_ambient_gate() {
    let dir = stub_workspace(&[]);
    write(
        &root(&dir),
        "qsl-eval/src/lib.rs",
        "pub fn stub() {}\npub fn seeded() -> Option<String> {\n    std::env::var(\"HOME\").ok()\n}\n",
    );
    let output = qualified_core(&root(&dir));
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let report = stdout(&output);
    assert!(report.contains("Ambient input: FAIL"), "{report}");
    assert!(report.contains("qsl-eval/src/lib.rs:3"), "{report}");
    assert!(report.contains("Direction: PASS"), "{report}");
}

const FRONTEND: &str = "agent-ix-extraction-frontend";

#[trace("TC-767", "FR-284-AC-1")]
#[test]
fn a_seeded_argument_parser_fails_and_only_the_fcd_clap_edge_passes() {
    // An argument-parser crate under `qsl-replay` fails naming the pair.
    let dir = stub_workspace_with(&["clap"], &[("qsl-replay", "clap")]);
    let output = qualified_core(&root(&dir));
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let report = stdout(&output);
    assert!(report.contains("Direction: FAIL"), "{report}");
    assert!(report.contains("(qsl-replay, clap)"), "{report}");

    // `qsl-semantics` -> the FCD frontend -> `clap` is the one exemption.
    let sanctioned = [("qsl-semantics", FRONTEND), (FRONTEND, "clap")];
    let dir = stub_workspace_with(&[FRONTEND, "clap"], &sanctioned);
    let output = qualified_core(&root(&dir));
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));

    // Any other crate reaching the frontend, or `qsl-semantics` reaching
    // `clap` directly, still fails.
    let dir = stub_workspace_with(
        &[FRONTEND, "clap"],
        &[("qsl-eval", FRONTEND), (FRONTEND, "clap")],
    );
    let report = stdout(&qualified_core(&root(&dir)));
    assert!(
        report.contains(&format!("(qsl-eval, {FRONTEND})")),
        "{report}"
    );
    let dir = stub_workspace_with(&["clap"], &[("qsl-semantics", "clap")]);
    let report = stdout(&qualified_core(&root(&dir)));
    assert!(report.contains("(qsl-semantics, clap)"), "{report}");
}

/// Core source that the ambient scan must report, as
/// `(source line 3, reported category)`.
const AMBIENT_SEEDS: [(&str, &str); 6] = [
    ("std::env::var(\"HOME\").ok();", "environment read"),
    ("let _ = std::time::SystemTime::now();", "clock read"),
    (
        "let _ = std::fs::read_to_string(\"input\");",
        "filesystem or search-path access",
    ),
    ("println!(\"seeded\");", "process stream access"),
    ("std::process::exit(1);", "process exit"),
    (
        "static SEEDED: std::sync::Mutex<u8> = std::sync::Mutex::new(0);",
        "mutable global state",
    ),
];

/// Source the scan must pass: ambient spellings inside a comment, a string
/// literal and `#[cfg(test)]` code, and an atomic counter static.
const AMBIENT_EXCLUSIONS: &str = "pub fn stub() {}
// std::env::var(\"HOME\") in a comment
pub fn text() -> &'static str {
    \"std::env::var(HOME) std::process::exit(1)\"
}
pub static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
#[cfg(test)]
mod tests {
    #[test]
    fn reads() {
        let _ = std::env::var(\"HOME\");
        println!(\"test output\");
    }
}
";

#[trace("TC-768", "FR-284-AC-4")]
#[test]
fn every_seeded_ambient_category_fails_and_exclusions_pass() {
    let dir = stub_workspace(&[]);
    for (seed, category) in AMBIENT_SEEDS {
        let body = if seed.starts_with("static") {
            seed.to_owned()
        } else {
            format!("pub fn seeded() {{\n    {seed}\n}}")
        };
        write(
            &root(&dir),
            "qsl-eval/src/lib.rs",
            &format!("pub fn stub() {{}}\n{body}\n"),
        );
        let output = qualified_core(&root(&dir));
        let report = stdout(&output);
        assert_eq!(output.status.code(), Some(1), "{seed}: {report}");
        assert!(report.contains("Ambient input: FAIL"), "{seed}: {report}");
        let line = if seed.starts_with("static") { 2 } else { 3 };
        assert!(
            report.contains(&format!("qsl-eval/src/lib.rs:{line} {category}")),
            "{seed}: {report}"
        );
    }
    write(&root(&dir), "qsl-eval/src/lib.rs", AMBIENT_EXCLUSIONS);
    let output = qualified_core(&root(&dir));
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
}

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
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path();
    let mut members: Vec<&str> = CORE.to_vec();
    members.push("qsl-analyze");
    let list: Vec<String> = members.iter().map(|name| format!("\"{name}\"")).collect();
    write(
        root,
        "Cargo.toml",
        &format!(
            "[workspace]\nresolver = \"2\"\nmembers = [{}]\n",
            list.join(", ")
        ),
    );
    for name in &members {
        let dependencies: String = CLEAN_EDGES
            .iter()
            .chain(extra_edges)
            .filter(|(from, _)| from == name)
            .map(|(_, to)| format!("{to} = {{ path = \"../{to}\" }}\n"))
            .collect();
        write(
            root,
            &format!("{name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
                 [dependencies]\n{dependencies}"
            ),
        );
        write(root, &format!("{name}/src/lib.rs"), "pub fn stub() {}\n");
    }
    let lock = Command::new("cargo")
        .args(["generate-lockfile", "--offline", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("cargo runs");
    assert!(lock.status.success(), "lockfile: {lock:?}");
    dir
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
    let output = qualified_core(dir.path());
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    let report = stdout(&output);
    assert!(report.contains("Direction: PASS"), "{report}");
    assert!(report.contains("Ambient input: PASS"), "{report}");
}

#[trace("TC-767", "FR-284-AC-1")]
#[test]
fn a_seeded_forbidden_dependency_fails_the_direction_gate() {
    let dir = stub_workspace(&[("qsl-eval", "qsl-analyze")]);
    let output = qualified_core(dir.path());
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
        dir.path(),
        "qsl-eval/src/lib.rs",
        "pub fn stub() {}\npub fn seeded() -> Option<String> {\n    std::env::var(\"HOME\").ok()\n}\n",
    );
    let output = qualified_core(dir.path());
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let report = stdout(&output);
    assert!(report.contains("Ambient input: FAIL"), "{report}");
    assert!(report.contains("qsl-eval/src/lib.rs:3"), "{report}");
    assert!(report.contains("Direction: PASS"), "{report}");
}

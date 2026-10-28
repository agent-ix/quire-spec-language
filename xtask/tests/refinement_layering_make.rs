// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-868: the make target accepts only the expected layering CLI report.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use tempfile::tempdir;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a workspace root")
        .to_path_buf()
}

fn run_make(stub: &Path, corpus: &Path) -> Output {
    Command::new("make")
        .current_dir(workspace_root())
        .arg("refinement-layering")
        .arg(format!("XTASK_BIN={}", env!("CARGO_BIN_EXE_xtask")))
        .arg(format!("LAYERING_CLI={}", stub.display()))
        .arg(format!("LAYERING_CORPUS={}", corpus.display()))
        .output()
        .expect("make starts the expected-exit target")
}

#[trace("TC-868", "FR-345-AC-9", "FR-345-AC-13")]
#[test]
fn make_layering_target_accepts_only_exit_21_with_all_six_unresolved_layers() {
    let temp = tempdir().expect("temporary test directory");
    let corpus = temp.path().join("corpus");
    fs::create_dir(&corpus).expect("corpus directory");
    let source_stub = std::env::current_exe()
        .expect("test executable has a path")
        .parent()
        .and_then(Path::parent)
        .expect("test executable is under target/debug/deps")
        .join("layering-cli-stub");

    for (stub_name, expected_success) in [
        ("layering-cli-stub-0", false),
        ("layering-cli-stub-1", false),
        ("layering-cli-stub-10", false),
        ("layering-cli-stub-21-missing-layer", false),
        ("layering-cli-stub-21-complete", true),
    ] {
        let stub = temp.path().join(stub_name);
        fs::copy(&source_stub, &stub).expect("copy the Rust CLI stub");
        let output = run_make(&stub, &corpus);
        assert_eq!(
            output.status.success(),
            expected_success,
            "stub {stub_name}; stdout: {}; stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

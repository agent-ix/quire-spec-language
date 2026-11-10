// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-484: the legacy mapped and root extraction API and feature are absent,
//! while the qsl-source and qsl-replay extraction owners remain available.

use ix_trace_rs::trace;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn metadata() -> Value {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .current_dir(root())
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON")
}

fn package<'a>(metadata: &'a Value, name: &str) -> &'a Value {
    metadata["packages"]
        .as_array()
        .expect("metadata lists packages")
        .iter()
        .find(|package| package["name"] == name)
        .unwrap_or_else(|| panic!("workspace package {name} is present"))
}

/// FR-490-AC-1: downstream imports of the deleted public modules and their
/// output adapter fail during Rust name resolution.
#[trace("TC-484", "FR-490-AC-1")]
#[test]
fn retired_mapped_and_extraction_imports_fail_in_a_downstream_crate() {
    let metadata = metadata();
    let target = PathBuf::from(
        metadata["target_directory"]
            .as_str()
            .expect("metadata names the target directory"),
    );
    let dependencies = target.join("debug/deps");
    let library = std::fs::read_dir(&dependencies)
        .expect("the downstream test build has dependencies")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("libquire_spec_language-") && name.ends_with(".rlib")
                })
        })
        .max_by_key(|path| {
            std::fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::UNIX_EPOCH)
        })
        .expect("the root library is built as an rlib for this integration test");
    let temporary = tempfile::tempdir().expect("temporary downstream crate directory");
    let command_library = std::fs::read_dir(&dependencies)
        .expect("the downstream test build has dependencies")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libqsl_command-") && name.ends_with(".rlib"))
        })
        .max_by_key(|path| {
            std::fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::UNIX_EPOCH)
        })
        .expect("the command library is built for this integration test");
    let source = temporary.path().join("lib.rs");
    std::fs::write(
        &source,
        "use quire_spec_language::mapped;\n\
         use quire_spec_language::command as retired_command;\n\
         use quire_spec_language::formal_source as retired_formal_source;\n\
         use qsl_command::command::extraction as legacy_extraction;\n\
         use qsl_command::command::output::extraction as legacy_output;\n\
         use qsl_command::command::RunCause;\n\
         fn removed_cause() { let _ = RunCause::Extraction; }\n",
    )
    .expect("downstream probe source writes");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = Command::new(rustc)
        .args([
            "--crate-name",
            "qsl_835_retired_api_probe",
            "--edition=2021",
            "--crate-type=lib",
        ])
        .arg("--extern")
        .arg(format!("quire_spec_language={}", library.display()))
        .arg("--extern")
        .arg(format!("qsl_command={}", command_library.display()))
        .arg("-L")
        .arg(format!("dependency={}", dependencies.display()))
        .arg(&source)
        .arg("--out-dir")
        .arg(temporary.path())
        .output()
        .expect("rustc runs for the downstream probe");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "legacy imports compiled successfully"
    );
    for diagnostic in [
        "no `mapped` in the root",
        "no `command` in the root",
        "no `formal_source` in the root",
        "no `extraction` in `command`",
        "no `extraction` in `command::output`",
        "module `output` is private",
        "no variant, associated function, or constant named `Extraction`",
    ] {
        assert!(
            stderr.contains(diagnostic),
            "missing {diagnostic:?}: {stderr}"
        );
    }
}

/// FR-490-AC-2: Cargo metadata keeps extraction under its owning crates.
#[trace("TC-484", "FR-490-AC-2")]
#[test]
fn root_extraction_feature_and_dependency_are_absent_but_replay_owners_remain() {
    let metadata = metadata();
    let root = package(&metadata, "quire-spec-language");
    let source = package(&metadata, "qsl-source");
    let replay = package(&metadata, "qsl-replay");

    assert!(!root["features"]
        .as_object()
        .unwrap()
        .contains_key("quire-extraction"));
    assert!(!root["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dependency| dependency["name"] == "qsl-source"));
    assert!(source["features"]
        .as_object()
        .unwrap()
        .contains_key("quire-extraction"));
    assert!(replay["features"]
        .as_object()
        .unwrap()
        .contains_key("quire-extraction"));
}

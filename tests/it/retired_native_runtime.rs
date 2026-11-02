// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1276: the root native package and runtime surfaces are absent.

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn assert_no_retired_names(path: &Path, names: &[String]) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).unwrap() {
            assert_no_retired_names(&entry.unwrap().path(), names);
        }
    } else if let Ok(text) = std::fs::read_to_string(path) {
        for name in names {
            assert!(!text.contains(name), "{} contains {name}", path.display());
        }
    }
}

fn assert_import_is_absent(
    dependencies: &Path,
    library: &Path,
    probe: &str,
    source_text: &str,
    temporary: &Path,
) {
    let source = temporary.join(format!("{probe}.rs"));
    std::fs::write(&source, source_text).unwrap();
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = std::process::Command::new(rustc)
        .args(["--crate-name", probe, "--edition=2021", "--crate-type=lib"])
        .arg("--extern")
        .arg(format!("quire_spec_language={}", library.display()))
        .arg("-L")
        .arg(format!("dependency={}", dependencies.display()))
        .arg(&source)
        .arg("--out-dir")
        .arg(temporary)
        .output()
        .expect("rustc runs for the downstream probe");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{probe} import compiled");
    assert!(
        stderr.contains("E0432") || stderr.contains("E0433"),
        "{probe} failed for a reason other than an unresolved public import: {stderr}"
    );
}

/// FR-493-AC-1: no source, test or schema keeps the native package/runtime
/// wire or public API, and downstream Rust cannot import those APIs.
#[trace("TC-1276", "FR-493-AC-1")]
#[test]
fn native_package_and_runtime_surfaces_are_absent() {
    let retired_names = [
        ["Native", "Package"].concat(),
        ["runtime::", "execute"].concat(),
        ["native-linked-", "package/1"].concat(),
        ["native-state-", "input/1"].concat(),
    ];
    let repo = root();
    for area in ["src", "tests", "schemas"] {
        assert_no_retired_names(&repo.join(area), &retired_names);
    }

    let formats = qsl_foundation::wire_format::WireFormat::ALL;
    assert!(formats.iter().all(|format| {
        format.as_str() != ["native-linked-", "package/1"].concat()
            && format.as_str() != ["native-state-", "input/1"].concat()
    }));

    let metadata =
        std::process::Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .current_dir(&repo)
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
        metadata.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&metadata.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout).unwrap();
    let target = PathBuf::from(metadata["target_directory"].as_str().unwrap());
    let dependencies = target.join("debug/deps");
    let library = std::fs::read_dir(&dependencies)
        .unwrap()
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
    let temporary = tempfile::tempdir().unwrap();
    for (probe, source) in [
        (
            "qsl_836_package_module_probe",
            concat!("use quire_spec_language::", "package;\n"),
        ),
        (
            "qsl_836_runtime_module_probe",
            concat!("use quire_spec_language::", "runtime;\n"),
        ),
        (
            "qsl_836_native_package_type_probe",
            concat!("use quire_spec_language::package::", "Native", "Package;\n"),
        ),
        (
            "qsl_836_runtime_execute_probe",
            concat!("use quire_spec_language::runtime::", "execute;\n"),
        ),
    ] {
        assert_import_is_absent(&dependencies, &library, probe, source, temporary.path());
    }
}

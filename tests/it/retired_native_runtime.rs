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
    repo: &Path,
    target: &Path,
    probe: &str,
    source_text: &str,
    temporary: &Path,
) {
    let project = temporary.join(probe);
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{probe}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies]\nquire-spec-language = {{ path = \"{}\" }}\n",
            repo.display()
        ),
    )
    .unwrap();
    std::fs::copy(repo.join("Cargo.lock"), project.join("Cargo.lock")).unwrap();
    std::fs::write(project.join("src/lib.rs"), source_text).unwrap();
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = std::process::Command::new(cargo)
        .args(["check", "--offline", "--manifest-path"])
        .arg(project.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(target)
        .output()
        .expect("cargo runs for the downstream probe");
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

    let temporary = tempfile::tempdir().unwrap();
    // The integration test's Cargo process holds the workspace target lock.
    // Build downstream probes in their own temporary target directory.
    let target = temporary.path().join("target");
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
        assert_import_is_absent(&repo, &target, probe, source, temporary.path());
    }
}

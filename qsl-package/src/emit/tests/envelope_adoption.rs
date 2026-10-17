// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-766: `qsl-package` writes the `quire.checked-package/v2` envelope only
//! through IR's typed encoder. It defines no envelope wire type of its own, so
//! a member added to the envelope reaches QSL with the IR rev that adds it.

use std::fs;
use std::path::{Path, PathBuf};

/// Every non-test Rust source file under `dir`.
fn production_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("the source directory is readable");
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if name != "tests" && name != "extent_agreement" {
                production_sources(&path, found);
            }
        } else if name.ends_with(".rs") && !name.starts_with("tests") {
            found.push(path);
        }
    }
}

/// A source that writes the envelope's member names, or implements the
/// canonical `Encode` trait, defines an envelope wire type of its own.
#[test]
fn qsl_package_defines_no_envelope_wire_type() {
    let mut files = Vec::new();
    production_sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(!files.is_empty(), "the scan found no sources");
    let offending: Vec<_> = files
        .iter()
        .filter(|path| {
            let text = fs::read_to_string(path).expect("a source file is readable");
            text.contains("name(\"contract_version\")") || text.contains("Encode for ")
        })
        .collect();
    assert!(
        offending.is_empty(),
        "qsl-package defines an envelope wire type: {offending:?}"
    );
}

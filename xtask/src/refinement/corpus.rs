// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-340 "Where it runs": the real spec-versioning corpus, generated at run
//! time. FR-056 selects a model by digest, so a corpus that committed the
//! digest of `examples/config-version/model.semantic-ir.json` would churn on
//! every model edit. The corpus therefore is not committed: `make
//! refinement-versioning` calls [`generate`], which reads the model file and
//! computes its digest then, and writes the one revision pair (units, pair
//! description and snapshots) into a temporary directory the gate reads.

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::error::{Error, Result};

#[path = "../../../examples/config-version/spine_unit.rs"]
#[allow(dead_code, missing_docs)]
mod config_version;

/// The model the pair selects, relative to the repository root.
const MODEL: &str = "examples/config-version/model.semantic-ir.json";
const AUTHORITY: &str = "agent-ix";
const IDENTITY: &str = "ix://example/config-version/spine/unit";
const POPULATION: &str = "ix://example/config-version/config_history";
const CLAUSE: &str = "present(self.parent) implies";
/// A meaning-preserving revision of [`CLAUSE`] (the superseding unit).
const CLAUSE_REVISED: &str = "not not present(self.parent) implies";

/// The model file's digest, hex-encoded, computed from `root`'s copy of it.
///
/// # Errors
/// [`Error::RefinementCorpus`] when the model file cannot be read.
pub fn model_digest_hex(root: &Path) -> Result<String> {
    let path = root.join(MODEL);
    let bytes = fs::read(&path).map_err(|source| Error::RefinementCorpus {
        path: path.display().to_string(),
        source,
    })?;
    let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()]);
    let digest = packages.keys().next().expect("one supplied package");
    Ok(qsl_semantics::model::key::hex(digest))
}

fn snapshot(digest: &str, name: &str, rows: &[(&str, i64, Option<&str>)]) -> Value {
    let objects: Vec<Value> = rows
        .iter()
        .map(|(key, version, parent)| {
            let parent = match parent {
                Some(parent) => {
                    json!({"present": {"reference": {"population": POPULATION, "key": parent}}})
                }
                None => json!({"absent": {}}),
            };
            json!({
                "key": key,
                "type": "ix://example/config-version/ConfigVersion",
                "fields": {"versionNumber": {"integer": version.to_string()}, "parent": parent},
            })
        })
        .collect();
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": AUTHORITY,
            "identity": format!("ix://example/config-version/spine/current/{name}"),
            "revision_namespace": "example",
            "revision": "1",
        },
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {
            "identity": config_version::PACKAGE_IDENTITY,
            "version": config_version::PACKAGE_VERSION,
            "digest": format!("sha256-jcs:{digest}"),
        },
        "populations": [{"population": POPULATION, "complete": true, "objects": objects}],
    })
}

fn clause_case(observation: &str) -> Value {
    json!({
        "selection": {"clause": {
            "name": "ParentOrder",
            "observation": observation,
            "self": {"population": POPULATION, "key": "child"},
            "anchor": {"kind": "handler", "name": "validate"},
        }},
        "observations": {observation: format!("{observation}.json")},
    })
}

/// Write the real corpus into `out`, with the model digest computed from
/// `root`'s model file. Nothing it writes is committed.
///
/// # Errors
/// [`Error::RefinementCorpus`] when the model cannot be read or a corpus file
/// cannot be written.
pub fn generate(root: &Path, out: &Path) -> Result<()> {
    let digest = model_digest_hex(root)?;
    let prior = config_version::unit_text(&digest);
    let superseding = prior.replace(CLAUSE, CLAUSE_REVISED);
    assert_ne!(prior, superseding, "the superseding revision differs");
    let pair = json!({
        "prior": {"path": "unit.native", "authority": AUTHORITY, "identity": IDENTITY},
        "superseding": {"path": "superseding.native", "authority": AUTHORITY, "identity": IDENTITY},
        "packages": [format!("repo:{MODEL}")],
        "dependencies": [],
        "cases": [clause_case("healthy-parent"), clause_case("violating-parent")],
    });
    let dir = out.join("config-version-revision");
    let write = |name: &str, text: String| {
        let path = dir.join(name);
        fs::write(&path, text).map_err(|source| Error::RefinementCorpus {
            path: path.display().to_string(),
            source,
        })
    };
    fs::create_dir_all(&dir).map_err(|source| Error::RefinementCorpus {
        path: dir.display().to_string(),
        source,
    })?;
    write("unit.native", prior)?;
    write("superseding.native", superseding)?;
    write("pair.json", pair.to_string())?;
    let healthy = snapshot(
        &digest,
        "healthy-parent",
        &[("root", 1, None), ("child", 2, Some("root"))],
    );
    let violating = snapshot(
        &digest,
        "violating-parent",
        &[("root", 3, None), ("child", 2, Some("root"))],
    );
    write("healthy-parent.json", healthy.to_string())?;
    write("violating-parent.json", violating.to_string())
}

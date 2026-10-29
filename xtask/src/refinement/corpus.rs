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

/// The model the pair selects, relative to the repository root.
const MODEL: &str = "examples/config-version/model.semantic-ir.json";
const AUTHORITY: &str = "agent-ix";
const IDENTITY: &str = "ix://example/config-version/spine/unit";
const CLAUSE: &str = "present(self.parent) implies";
/// A meaning-preserving revision of [`CLAUSE`] (the superseding unit).
const CLAUSE_REVISED: &str = "not not present(self.parent) implies";

/// What the model file says about itself: the identity a `model` statement
/// and a snapshot's `model` member must repeat, read from the file.
struct Model {
    identity: String,
}

fn read_model(root: &Path) -> Result<Model> {
    let path = root.join(MODEL);
    let fail = |source: std::io::Error| Error::RefinementCorpus {
        path: path.display().to_string(),
        source,
    };
    let bytes = fs::read(&path).map_err(fail)?;
    let document: Value = serde_json::from_slice(&bytes)
        .map_err(|source| fail(std::io::Error::new(std::io::ErrorKind::InvalidData, source)))?;
    let member = |name: &str| {
        document["package"][name]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| {
                fail(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("no package.{name}"),
                ))
            })
    };
    Ok(Model {
        identity: member("identity")?,
    })
}

/// The ConfigVersion unit, selecting `model` by what the model file declares.
fn unit_text(model: &Model) -> String {
    format!(
        "// SPDX-License-Identifier: AGPL-3.0-only\n\
         language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model Config = {:?};\n\
         invariant ParentOrder using v on Config::ConfigVersion at current {{ \
         present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }}\n\
         invariant NoCycle using v on Config::ConfigVersion at current {{ \
         not reaches(self, self, parent) }}\n\
         post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate {{ \
         self.versionNumber = pre(self.versionNumber) }}\n\
         function sameIdentity using v(a: Config::ConfigVersion, b: Config::ConfigVersion): \
         Boolean pure {{ a = b }}\n",
        model.identity
    )
}

fn snapshot(model: &Model, name: &str, rows: &[(&str, i64, Option<&str>)]) -> Value {
    let objects: Vec<Value> = rows
        .iter()
        .map(|(key, version, parent)| {
            let parent = match parent {
                Some(parent) => {
                    json!({"present": {"reference": {"population": population(model), "key": parent}}})
                }
                None => json!({"absent": {}}),
            };
            json!({
                "key": key,
                "type": format!("ix://{}/ConfigVersion", model.identity),
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
            "identity": model.identity,
        },
        "populations": [{"population": population(model), "complete": true, "objects": objects}],
    })
}

fn clause_case(model: &Model, observation: &str) -> Value {
    json!({
        "selection": {"clause": {
            "name": "ParentOrder",
            "observation": observation,
            "self": {"population": population(model), "key": "child"},
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
    let model = read_model(root)?;
    let prior = unit_text(&model);
    let superseding = prior.replace(CLAUSE, CLAUSE_REVISED);
    assert_ne!(prior, superseding, "the superseding revision differs");
    let pair = json!({
        "prior": {"path": "unit.native", "authority": AUTHORITY, "identity": IDENTITY},
        "superseding": {"path": "superseding.native", "authority": AUTHORITY, "identity": IDENTITY},
        "packages": [format!("repo:{MODEL}")],
        "dependencies": [],
        "cases": [clause_case(&model, "healthy-parent"), clause_case(&model, "violating-parent")],
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
        &model,
        "healthy-parent",
        &[("root", 1, None), ("child", 2, Some("root"))],
    );
    let violating = snapshot(
        &model,
        "violating-parent",
        &[("root", 3, None), ("child", 2, Some("root"))],
    );
    write("healthy-parent.json", healthy.to_string())?;
    write("violating-parent.json", violating.to_string())
}

/// The ConfigVersion population's identity, under the model's package identity.
fn population(model: &Model) -> String {
    format!("ix://{}/config_history", model.identity)
}

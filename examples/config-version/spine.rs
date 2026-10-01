// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-108 (TC-469): the spine counterpart of this directory's
//! native fixtures -- the `1-draft` unit, the Semantic IR 2.0.0 domain
//! package, FR-106 snapshot/invocation documents and a `ClauseRunRequest`,
//! written from the *same* `cases::CaseSpec` catalog `fixtures.rs` already
//! reads for the native files (FR-108's own Behavior: "one datum for both
//! paths").
//!
//! `clause-run-request.json` is written for reproducibility beside the
//! other spine files, in `native-run/1`'s own idiom, but it is not read back
//! by a generic decoder: `run_clause`'s `ClauseRunRequest` is a Rust struct
//! with no wire format of its own (unlike `native-run/1`, which the CLI's
//! own `model_source`/`runtime` readers parse), and building one would be a
//! new subsystem outside this ticket's scope. [`request`] below
//! reconstructs the same `ClauseRunRequest` directly from the files this
//! module wrote (the unit, the domain package and the snapshot/invocation
//! documents), so the spine test still runs over exactly the bytes on disk,
//! not a second, only-in-memory copy of the case data.

use super::cases::{Case, Input, Row};
use qsl_replay::spine::{
    default_accounting, ClauseArgument, ClauseArgumentValue, ClauseRunRequest, ClauseRunSelection,
    ClauseRunSource, DependencyInput, SpineLimits,
};
use qsl_semantics::model::observation::{
    AnchorKind, ClauseSelection, ClauseSelectionInput, DocumentRef, ObservationLimits,
    SelectedAnchor, SelectedObject,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, io, path::Path};

/// The unit text and domain package, shared with `qsl-package`'s
/// emission-to-admission corpus.
#[path = "spine_unit.rs"]
mod unit;
use unit::PACKAGE_VERSION;
pub use unit::{DOMAIN_PACKAGE, PACKAGE_IDENTITY};

fn config_version_type() -> String {
    format!("ix://{PACKAGE_IDENTITY}/ConfigVersion")
}

fn population_identity() -> String {
    format!("ix://{PACKAGE_IDENTITY}/config_history")
}

/// `bytes`' `sha256-jcs` digest (FR-056's digest-first rule): parsed as
/// JSON, then digested over RFC 8785 canonical bytes -- the same recipe
/// `qsl_semantics::model::observation`'s own document digest uses.
fn document_digest(bytes: &[u8]) -> [u8; 32] {
    let value: Value = serde_json::from_slice(bytes).expect("spine fixture is JSON");
    let limits = quire_canonical::Limits::new(u64::MAX, quire_canonical::Limits::MAX_DEPTH)
        .expect("MAX_DEPTH is within MAX_DEPTH");
    *quire_canonical::sha256(&value, limits)
        .expect("spine fixture is RFC 8785 canonical")
        .as_bytes()
}

/// [`DOMAIN_PACKAGE`]'s own digest, as `intake::package_input` computes it
/// (the digest a `model` statement's own `digest "sha256-jcs:..."` names).
pub fn model_digest_hex() -> String {
    let packages = domain_packages();
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied domain package");
    };
    qsl_semantics::model::key::hex(digest)
}

/// [`DOMAIN_PACKAGE`], keyed the way `compile`/`run_clause` expect their
/// `packages` map.
pub fn domain_packages() -> BTreeMap<[u8; 32], Vec<u8>> {
    qsl_semantics::model::intake::package_input([DOMAIN_PACKAGE.as_bytes()])
}

/// FR-108's own fixed `SourceIdentity` for the compiled unit, shared by
/// every case (see [`request`]'s own doc for why it must not be
/// case-suffixed) and by every direct `spine::compile` call comparing
/// against it (`tests/it/config_version_spine.rs`'s step 6 tests). One
/// function, not three copies of the same literal (SR-768 FND-005).
pub fn unit_identity() -> qsl_foundation::SourceIdentity {
    qsl_foundation::SourceIdentity::new(
        "agent-ix",
        "ix://example/config-version/spine/unit",
        "example",
        "1",
    )
}

/// FR-108's own `1-draft` unit text ([`unit::unit_text`]) selecting
/// [`DOMAIN_PACKAGE`] by its digest.
pub fn unit_text() -> String {
    unit::unit_text(&model_digest_hex())
}

fn identity_json(case: Case, role: &str) -> Value {
    json!({
        "authority": "agent-ix",
        "identity": format!("ix://example/config-version/spine/{role}/{}", case.id()),
        "revision_namespace": "example",
        "revision": "1",
    })
}

fn document_ref(identity: &Value, digest: [u8; 32]) -> DocumentRef {
    DocumentRef {
        authority: identity["authority"].as_str().unwrap().to_owned(),
        identity: identity["identity"].as_str().unwrap().to_owned(),
        revision_namespace: identity["revision_namespace"].as_str().unwrap().to_owned(),
        revision: identity["revision"].as_str().unwrap().to_owned(),
        digest,
    }
}

fn model_header() -> Value {
    json!({"identity": PACKAGE_IDENTITY, "version": PACKAGE_VERSION,
        "digest": format!("sha256-jcs:{}", model_digest_hex())})
}

/// A `quire.state.snapshot/v1` document over `rows`, `complete` per the
/// case's own population completeness.
fn snapshot_bytes(case: Case, role: &str, rows: &[Row], complete: bool) -> Vec<u8> {
    let config_version = config_version_type();
    let population = population_identity();
    let objects: Vec<Value> = rows
        .iter()
        .map(|row| {
            let parent = match row.parent {
                Some(key) => {
                    json!({"present": {"reference": {"population": population, "key": key}}})
                }
                None => json!({"absent": {}}),
            };
            json!({
                "key": row.key, "type": config_version,
                "fields": {"versionNumber": {"integer": row.version.to_string()}, "parent": parent},
            })
        })
        .collect();
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": identity_json(case, role),
        "observation": role,
        "anchor": {"kind": "handler", "name": "validate"},
        "model": model_header(),
        "populations": [{"population": population, "complete": complete, "objects": objects}],
    })
    .to_string()
    .into_bytes()
}

/// A `quire.state.invocation/v1` document over `pre`/`post`'s own
/// identities and digests, self object `self_key`. FR-108's own operation
/// (`attemptUpdate`) takes no parameters, returns `Boolean` and its frame
/// modifies exactly `versionNumber`; `created`/`deleted` are always empty
/// (no case creates or deletes an object).
fn invocation_bytes(case: Case, self_key: &str, pre: &DocumentRef, post: &DocumentRef) -> Vec<u8> {
    let identity = |label: &DocumentRef| {
        json!({"authority": label.authority, "identity": label.identity,
            "revision_namespace": label.revision_namespace, "revision": label.revision})
    };
    json!({
        "format": "quire.state.invocation/v1",
        "identity": identity_json(case, "invocation"),
        "model": model_header(),
        "context": config_version_type(),
        "operation": "attemptUpdate",
        "self": {"population": population_identity(), "key": self_key},
        "pre": {"identity": identity(pre), "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&pre.digest))},
        "post": {"identity": identity(post), "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&post.digest))},
        "parameters": {},
        "result": {"boolean": true},
        "created": [],
        "deleted": [],
    })
    .to_string()
    .into_bytes()
}

fn write_json(directory: &Path, file: &str, bytes: &[u8]) -> io::Result<()> {
    std::fs::write(directory.join(file), bytes)
}

/// Every input document [`request`] needs, plus the selection it names.
struct Written {
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    invocations: BTreeMap<[u8; 32], Vec<u8>>,
    selection: ClauseRunSelection,
}

fn write_current(
    directory: &Path,
    case: Case,
    rows: &[Row],
    self_key: &str,
) -> io::Result<Written> {
    let complete = case.spec().complete;
    let bytes = snapshot_bytes(case, "current", rows, complete);
    write_json(directory, "spine-snapshot.json", &bytes)?;
    let digest = document_digest(&bytes);
    let label = document_ref(&identity_json(case, "current"), digest);
    let selection = ClauseSelection {
        name: case.spec().clause.name.to_owned(),
        input: ClauseSelectionInput::Current {
            snapshot: label,
            anchor: SelectedAnchor {
                kind: AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: SelectedObject {
                population: population_identity(),
                key: self_key.to_owned(),
            },
        },
    };
    Ok(Written {
        snapshots: BTreeMap::from([(digest, bytes)]),
        invocations: BTreeMap::new(),
        selection: ClauseRunSelection::Clause(selection),
    })
}

fn write_update(directory: &Path, case: Case, pre: &[Row], post: &[Row]) -> io::Result<Written> {
    let complete = case.spec().complete;
    let pre_bytes = snapshot_bytes(case, "pre", pre, complete);
    let post_bytes = snapshot_bytes(case, "post", post, complete);
    write_json(directory, "spine-pre-snapshot.json", &pre_bytes)?;
    write_json(directory, "spine-post-snapshot.json", &post_bytes)?;
    let pre_digest = document_digest(&pre_bytes);
    let post_digest = document_digest(&post_bytes);
    let pre_ref = document_ref(&identity_json(case, "pre"), pre_digest);
    let post_ref = document_ref(&identity_json(case, "post"), post_digest);
    let invocation_bytes = invocation_bytes(case, "child", &pre_ref, &post_ref);
    write_json(directory, "spine-invocation.json", &invocation_bytes)?;
    let invocation_digest = document_digest(&invocation_bytes);
    let invocation_ref = document_ref(&identity_json(case, "invocation"), invocation_digest);
    let selection = ClauseSelection {
        name: case.spec().clause.name.to_owned(),
        input: ClauseSelectionInput::Invocation {
            invocation: invocation_ref,
        },
    };
    Ok(Written {
        snapshots: BTreeMap::from([(pre_digest, pre_bytes), (post_digest, post_bytes)]),
        invocations: BTreeMap::from([(invocation_digest, invocation_bytes)]),
        selection: ClauseRunSelection::Clause(selection),
    })
}

/// `sameIdentity(a, b)`, `distinct-identities`' own function selection:
/// `a` the self key, `b` the other key, over the case's own current
/// snapshot (FR-108's own Corpus table row).
fn write_function(
    directory: &Path,
    case: Case,
    rows: &[Row],
    self_key: &str,
    other_key: &str,
) -> io::Result<Written> {
    let complete = case.spec().complete;
    let bytes = snapshot_bytes(case, "current", rows, complete);
    write_json(directory, "spine-snapshot.json", &bytes)?;
    let digest = document_digest(&bytes);
    let label = document_ref(&identity_json(case, "current"), digest);
    let reference = |key: &str| ClauseArgumentValue::Reference {
        population: population_identity(),
        key: key.to_owned(),
    };
    let selection = ClauseRunSelection::Function {
        name: "sameIdentity".to_owned(),
        arguments: vec![
            ClauseArgument {
                parameter: "a".to_owned(),
                value: reference(self_key),
            },
            ClauseArgument {
                parameter: "b".to_owned(),
                value: reference(other_key),
            },
        ],
        snapshot: label,
    };
    Ok(Written {
        snapshots: BTreeMap::from([(digest, bytes)]),
        invocations: BTreeMap::new(),
        selection,
    })
}

fn written(directory: &Path, case: Case) -> io::Result<Written> {
    match case.spec().input {
        Input::Current {
            rows,
            self_key,
            other: Some(other),
        } => write_function(directory, case, rows, self_key, other),
        Input::Current { rows, self_key, .. } => write_current(directory, case, rows, self_key),
        Input::Update { pre, post } => write_update(directory, case, pre, post),
    }
}

/// Emits FR-108's spine files beside the native ones `fixtures::write`
/// already wrote in `directory`: the unit, the domain package, the FR-106
/// documents, and a `clause-run-request.json` (reproducibility only, see
/// this module's own doc).
pub fn write(directory: &Path, case: Case) -> io::Result<()> {
    let unit = unit_text();
    write_json(directory, "spine-unit.native", unit.as_bytes())?;
    write_json(
        directory,
        "spine-model.semantic-ir.json",
        DOMAIN_PACKAGE.as_bytes(),
    )?;
    let written = written(directory, case)?;
    let selection = match &written.selection {
        ClauseRunSelection::Clause(clause) => json!({
            "kind": "clause",
            "name": clause.name,
            "input": match &clause.input {
                ClauseSelectionInput::Current { .. } => "current",
                ClauseSelectionInput::PreCall { .. } => "pre-call",
                ClauseSelectionInput::Invocation { .. } => "invocation",
            },
        }),
        ClauseRunSelection::Function {
            name, arguments, ..
        } => json!({
            "kind": "function",
            "name": name,
            "arguments": arguments.iter().map(|argument| json!({
                "parameter": argument.parameter,
                "value": match &argument.value {
                    ClauseArgumentValue::Integer(value) => json!(value),
                    ClauseArgumentValue::Reference { population, key } => {
                        json!({"reference": {"population": population, "key": key}})
                    }
                },
            })).collect::<Vec<_>>(),
        }),
        ClauseRunSelection::Frame { operation, .. } => json!({
            "kind": "frame",
            "operation": operation.to_string(),
        }),
    };
    let request = json!({"format": "clause-run/1", "request": {
        "source": {"file": "spine-unit.native", "identity": "example/config-version-spine-unit"},
        "models": [{"format": "semantic-ir/2.0.0", "file": "spine-model.semantic-ir.json"}],
        "snapshots": written.snapshots.keys().map(|digest| qsl_semantics::model::key::hex(digest)).collect::<Vec<_>>(),
        "invocations": written.invocations.keys().map(|digest| qsl_semantics::model::key::hex(digest)).collect::<Vec<_>>(),
        "selection": selection,
    }});
    write_json(
        directory,
        "clause-run-request.json",
        serde_json::to_vec_pretty(&request)?.as_slice(),
    )?;
    Ok(())
}

/// Reconstructs the [`ClauseRunRequest`] [`write`] wrote `directory`'s files
/// for. `written` is the one function that both writes the FR-106 documents
/// and builds the selection over them, so this reruns it (rewriting the same
/// deterministic bytes, AC-4) rather than duplicating its per-input-kind
/// match in a second, read-only copy.
// This module is compiled twice via `#[path]` (the `config_version_fixtures`
// example binary and `tests/it`'s `config_version_spine`); only the latter
// calls `request` (the example binary only ever writes files, never runs a
// clause over them), so the example target sees it as dead code.
#[cfg_attr(not(test), allow(dead_code))]
pub fn request(directory: &Path, case: Case) -> io::Result<ClauseRunRequest> {
    let unit = std::fs::read(directory.join("spine-unit.native"))?;
    // `missing-model`'s own corpus row (FR-108: "ParentOrder, no package") --
    // the unit still declares `model Config = ...`, but the request supplies
    // no package for it, exactly as native's own `include_model: false`
    // leaves `models` empty in `request.json`.
    let packages = if case.spec().include_model {
        let package = std::fs::read(directory.join("spine-model.semantic-ir.json"))?;
        qsl_semantics::model::intake::package_input([package.as_slice()])
    } else {
        BTreeMap::new()
    };
    let written = written(directory, case)?;
    Ok(ClauseRunRequest {
        // Every case reads byte-identical `unit_text()`, so the unit's own
        // `SourceIdentity` must be the same fixed value across cases too --
        // it is a genuine input to node-key/`PackageId` derivation
        // (`qsl-package`'s `identity_projection`), not just a display label.
        // A case-suffixed identity here would mint a different `PackageId`
        // per case even though the compiled unit and domain package are
        // identical (TC-469 step 6's "every case shares the same unit and
        // package"). This is the same fixed identity
        // `tests/it/config_version_spine.rs`'s own independent
        // `spine::compile` call uses for its direct-compile comparison.
        source: ClauseRunSource::Program {
            identity: unit_identity(),
            path: "spine-unit.native".to_owned(),
            bytes: unit,
        },
        packages,
        dependencies: DependencyInput::default(),
        snapshots: written.snapshots,
        invocations: written.invocations,
        selection: written.selection,
        expected_package_id: None,
        limits: SpineLimits::default(),
        observation_limits: ObservationLimits::default(),
        model_limits: qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        // `exhausted-work`'s own `work_units: 0` (FR-108's Corpus table);
        // every other case gets a generous ceiling it never approaches.
        accounting: default_accounting(case.spec().expression_steps.unwrap_or(1_000_000)),
    })
}

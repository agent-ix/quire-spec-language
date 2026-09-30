// SPDX-License-Identifier: AGPL-3.0-or-later
//! Builds the FR-059 direction-check edge list from real `cargo metadata`
//! output. This is the only module that shells out; `graph.rs` stays pure
//! and is exercised by synthetic fixtures instead.

use std::{path::Path, process::Command};

use serde_json::Value;

use crate::error::{Code, Error, Result};
use crate::graph::{classify, Edge, EdgeKind, Repo};

/// Run `cargo metadata` for the crate at `manifest_path` and return every
/// resolved normal/dev edge whose source and target both classify as one of
/// the four ADR-011 repositories (`classify`, by package name and source
/// URL, so a `[patch]` or a differently pinned revision still resolves).
pub(crate) fn edges_for_manifest(manifest_path: &Path, offline: bool) -> Result<Vec<Edge>> {
    let mut command = Command::new("cargo");
    command
        .arg("metadata")
        .arg("--format-version=1")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(manifest_path);
    if offline {
        command.arg("--offline");
    }
    let output = command
        .output()
        .map_err(|error| Error::io(manifest_path, error))?;
    if !output.status.success() {
        return Err(Error::new(
            Code::CargoMetadata,
            format!(
                "cargo metadata failed for {}: {}",
                manifest_path.display(),
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    let document: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        Error::new(
            Code::InvalidMetadata,
            format!("cargo metadata produced invalid JSON: {error}"),
        )
    })?;
    parse_edges(&document)
}

fn parse_edges(document: &Value) -> Result<Vec<Edge>> {
    let invalid = || Error::new(Code::InvalidMetadata, "unexpected cargo metadata shape");
    let packages = document
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;

    let mut id_repo: Vec<(&str, Option<Repo>, &str)> = Vec::new();
    for package in packages {
        let id = package
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        let name = package
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        let source = package.get("source").and_then(Value::as_str);
        id_repo.push((id, classify(name, source), name));
    }

    let nodes = document
        .get("resolve")
        .and_then(|resolve| resolve.get("nodes"))
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;

    let mut edges = Vec::new();
    for node in nodes {
        let from_id = node.get("id").and_then(Value::as_str).ok_or_else(invalid)?;
        let Some((_, Some(from_repo), _)) = id_repo.iter().find(|(id, _, _)| *id == from_id) else {
            continue;
        };
        let deps = node
            .get("deps")
            .and_then(Value::as_array)
            .ok_or_else(invalid)?;
        for dep in deps {
            let to_id = dep.get("pkg").and_then(Value::as_str).ok_or_else(invalid)?;
            let Some((_, Some(to_repo), to_name)) = id_repo.iter().find(|(id, _, _)| *id == to_id)
            else {
                continue;
            };
            if from_repo == to_repo {
                continue;
            }
            let dep_kinds = dep
                .get("dep_kinds")
                .and_then(Value::as_array)
                .ok_or_else(invalid)?;
            for dep_kind in dep_kinds {
                let kind = match dep_kind.get("kind") {
                    Some(Value::Null) | None => EdgeKind::Normal,
                    Some(Value::String(kind)) if kind == "dev" => EdgeKind::Dev,
                    Some(Value::String(kind)) if kind == "build" => EdgeKind::Normal,
                    Some(_) => continue,
                };
                edges.push(Edge {
                    from: *from_repo,
                    to: *to_repo,
                    kind,
                    via_crate: (*to_name).to_owned(),
                });
            }
        }
    }
    edges.sort_by(|a, b| {
        (a.from, a.to, a.kind == EdgeKind::Dev, &a.via_crate).cmp(&(
            b.from,
            b.to,
            b.kind == EdgeKind::Dev,
            &b.via_crate,
        ))
    });
    edges.dedup();
    Ok(edges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::json;

    /// tc_arch_lint_metadata_001: a minimal synthetic `cargo metadata`
    /// document with an IR -> QSL normal edge is parsed into exactly that
    /// edge, matching ADR-011 OBS-029's real, currently observed shape.
    #[trace("TC-156")]
    #[test]
    fn tc_arch_lint_metadata_001_parses_ir_to_qsl_edge() {
        let document = json!({
            "packages": [
                {"id": "ir 0.1.0", "name": "quire-contract-ir", "source": null},
                {"id": "qsl 0.2.0", "name": "quire-spec-language",
                 "source": "git+https://github.com/agent-ix/quire-spec-language?rev=f1700a9#f1700a9"},
            ],
            "resolve": {
                "nodes": [
                    {"id": "ir 0.1.0", "deps": [
                        {"name": "quire_spec_language", "pkg": "qsl 0.2.0",
                         "dep_kinds": [{"kind": null, "target": null}]}
                    ]},
                    {"id": "qsl 0.2.0", "deps": []}
                ]
            }
        });
        let edges = parse_edges(&document).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, Repo::Ir);
        assert_eq!(edges[0].to, Repo::Qsl);
        assert_eq!(edges[0].kind, EdgeKind::Normal);
    }

    /// tc_arch_lint_metadata_002: a dev-kind dependency edge is classified
    /// as `EdgeKind::Dev`.
    #[trace("TC-156")]
    #[test]
    fn tc_arch_lint_metadata_002_classifies_dev_edge() {
        let document = json!({
            "packages": [
                {"id": "qsl 0.2.0", "name": "quire-spec-language", "source": null},
                {"id": "cg 0.1.0", "name": "quire-contract-codegen",
                 "source": "git+https://github.com/agent-ix/quire-contract-codegen"},
            ],
            "resolve": {
                "nodes": [
                    {"id": "qsl 0.2.0", "deps": [
                        {"name": "quire_contract_codegen", "pkg": "cg 0.1.0",
                         "dep_kinds": [{"kind": "dev", "target": null}]}
                    ]},
                    {"id": "cg 0.1.0", "deps": []}
                ]
            }
        });
        let edges = parse_edges(&document).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::Dev);
    }

    /// tc_arch_lint_metadata_003: a dependency on a crate outside the four
    /// ADR-011 repositories (for example `serde`) contributes no edge.
    #[trace("TC-156")]
    #[test]
    fn tc_arch_lint_metadata_003_ignores_non_ecosystem_dependency() {
        let document = json!({
            "packages": [
                {"id": "qsl 0.2.0", "name": "quire-spec-language", "source": null},
                {"id": "serde 1.0.228", "name": "serde",
                 "source": "registry+https://github.com/rust-lang/crates.io-index"},
            ],
            "resolve": {
                "nodes": [
                    {"id": "qsl 0.2.0", "deps": [
                        {"name": "serde", "pkg": "serde 1.0.228",
                         "dep_kinds": [{"kind": null, "target": null}]}
                    ]}
                ]
            }
        });
        let edges = parse_edges(&document).unwrap();
        assert!(edges.is_empty());
    }
}

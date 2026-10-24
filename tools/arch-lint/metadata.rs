// SPDX-License-Identifier: AGPL-3.0-or-later
//! Builds the FR-059 direction-check edge list from real `cargo metadata`
//! output. This is the only module that shells out; `graph.rs` stays pure
//! and is exercised by synthetic fixtures instead.

use std::{path::Path, process::Command};

use serde_json::Value;

use crate::error::{Code, Error, Result};
use crate::graph::{classify, ConstructEdge, Edge, EdgeKind, Repo};

/// Run `cargo metadata` for the crate at `manifest_path` and return every
/// resolved normal/dev edge whose source and target both classify as one of
/// the four ADR-011 repositories (`classify`, by package name and source
/// URL, so a `[patch]` or a differently pinned revision still resolves).
pub(crate) fn edges_for_manifest(manifest_path: &Path) -> Result<Vec<Edge>> {
    parse_edges(&load(manifest_path)?)
}

/// Every resolved dependency, in the `cargo metadata` resolution of the crate
/// at `manifest_path`, on `quire-checked` with its `construct` feature
/// enabled (FR-059; FR-400-AC-3).
pub(crate) fn construct_edges_for_manifest(manifest_path: &Path) -> Result<Vec<ConstructEdge>> {
    parse_construct_edges(&load(manifest_path)?)
}

fn load(manifest_path: &Path) -> Result<Value> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version=1")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(manifest_path)
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
    serde_json::from_slice(&output.stdout).map_err(|error| {
        Error::new(
            Code::InvalidMetadata,
            format!("cargo metadata produced invalid JSON: {error}"),
        )
    })
}

/// The package name of the checked-term leaf (ADR-011 layer C).
const CHECKED_LEAF: &str = "quire-checked";

/// The `construct` edges of `document`: one per resolved package of an
/// ecosystem repository that depends on `quire-checked` when the leaf's
/// resolved features include `construct`. Features are the resolution's
/// unified set, which is what a build of that repository enables.
fn parse_construct_edges(document: &Value) -> Result<Vec<ConstructEdge>> {
    let invalid = || Error::new(Code::InvalidMetadata, "unexpected cargo metadata shape");
    let packages = document
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    let mut by_id: Vec<(&str, &str, Option<Repo>)> = Vec::new();
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
        by_id.push((id, name, classify(name, source)));
    }
    let nodes = document
        .get("resolve")
        .and_then(|resolve| resolve.get("nodes"))
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    let construct_on = |id: &str| {
        nodes.iter().any(|node| {
            node.get("id").and_then(Value::as_str) == Some(id)
                && node
                    .get("features")
                    .and_then(Value::as_array)
                    .is_some_and(|features| {
                        features.iter().any(|f| f.as_str() == Some("construct"))
                    })
        })
    };
    let mut edges = Vec::new();
    for node in nodes {
        let from_id = node.get("id").and_then(Value::as_str).ok_or_else(invalid)?;
        let Some((_, from_name, Some(from_repo))) = by_id.iter().find(|(id, _, _)| *id == from_id)
        else {
            continue;
        };
        let deps = node
            .get("deps")
            .and_then(Value::as_array)
            .ok_or_else(invalid)?;
        for dep in deps {
            let to_id = dep.get("pkg").and_then(Value::as_str).ok_or_else(invalid)?;
            let is_leaf = by_id
                .iter()
                .any(|(id, name, _)| *id == to_id && *name == CHECKED_LEAF);
            if is_leaf && construct_on(to_id) {
                edges.push(ConstructEdge {
                    from: *from_repo,
                    via_crate: (*from_name).to_owned(),
                });
            }
        }
    }
    edges.sort_by(|a, b| (a.from, &a.via_crate).cmp(&(b.from, &b.via_crate)));
    edges.dedup();
    Ok(edges)
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
    /// document with a synthetic IR -> QSL normal edge, the edge the FB-05
    /// check exercises, is parsed into exactly that edge.
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

    /// tc_arch_lint_metadata_010: a resolved RT dependency on `quire-checked`
    /// is a `construct` edge when the leaf's resolved features include
    /// `construct`, and none when they do not (FR-059; FR-400-AC-3).
    #[trace("TC-156", "FR-400-AC-3")]
    #[test]
    fn tc_arch_lint_metadata_010_construct_feature_on_a_backend_edge_is_reported() {
        let document = |features: Value| {
            json!({
                "packages": [
                    {"id": "rt 0.1.0", "name": "quire-contract-runtime", "source": null},
                    {"id": "qc 0.1.0", "name": "quire-checked",
                     "source": "git+https://github.com/agent-ix/quire-eval?branch=main"},
                ],
                "resolve": {"nodes": [
                    {"id": "rt 0.1.0", "deps": [
                        {"name": "quire_checked", "pkg": "qc 0.1.0",
                         "dep_kinds": [{"kind": null, "target": null}]}]},
                    {"id": "qc 0.1.0", "deps": [], "features": features},
                ]}
            })
        };
        let on = parse_construct_edges(&document(json!(["std", "construct"]))).unwrap();
        assert_eq!(
            on,
            vec![ConstructEdge {
                from: Repo::Rt,
                via_crate: "quire-contract-runtime".to_owned()
            }]
        );
        assert!(parse_construct_edges(&document(json!(["std"])))
            .unwrap()
            .is_empty());
        assert!(crate::graph::check(&parse_edges(&document(json!(["std"]))).unwrap()).is_clean());
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

    /// tc_arch_lint_metadata_006 (positive control on real data): resolving
    /// QSL's own workspace manifest through `edges_for_manifest`, the path
    /// `direction` takes for `--qsl`, yields QSL's normal edge on the
    /// git-sourced `quire-contract-model`, classified QSL -> IR. A classifier
    /// that stopped recognising the real IR crate would drop the edge and
    /// fail here.
    #[trace("TC-156", "FR-059-AC-6")]
    #[test]
    fn tc_arch_lint_metadata_006_real_qsl_manifest_yields_qsl_to_ir_model_edge() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let edges = edges_for_manifest(&workspace.join("Cargo.toml")).unwrap();
        let expected = Edge {
            from: Repo::Qsl,
            to: Repo::Ir,
            kind: EdgeKind::Normal,
            via_crate: "quire-contract-model".to_owned(),
        };
        assert!(
            edges.contains(&expected),
            "expected {expected:?} among {edges:?}"
        );
    }

    /// tc_arch_lint_metadata_010 (real data): QSL's own resolved dependency
    /// graph, read the way `direction` reads `--qsl`, has no edge back into
    /// QSL, and the FB-05 and FB-11 check over it is clean. A dependency that
    /// depended on a git-sourced QSL crate would add an edge into QSL and
    /// fail here. The QSL -> IR assertion keeps the check from passing on an
    /// empty graph.
    #[trace("TC-139", "FR-051-AC-6")]
    #[test]
    fn tc_arch_lint_metadata_010_real_qsl_graph_has_no_edge_back_into_qsl() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let edges = edges_for_manifest(&workspace.join("Cargo.toml")).unwrap();
        assert!(
            edges
                .iter()
                .any(|edge| edge.from == Repo::Qsl && edge.to == Repo::Ir),
            "no QSL -> IR edge among {edges:?}"
        );
        let into_qsl: Vec<_> = edges.iter().filter(|edge| edge.to == Repo::Qsl).collect();
        assert!(into_qsl.is_empty(), "edges into QSL: {into_qsl:?}");
        let report = crate::graph::check(&edges);
        assert!(report.is_clean(), "{report:?}");
    }

    /// tc_arch_lint_metadata_007: the kernel leaf `quire-exact`, git-sourced
    /// from its own repository, contributes no repository to the edge graph
    /// (`classify`), so an RT dependency on it yields no edge and no finding; RT's dependency on
    /// `qsl-eval`, from the same QSL git source, is still an RT -> QSL edge
    /// and an FB-05 violation.
    #[trace("TC-156", "FR-059-AC-8")]
    #[test]
    fn tc_arch_lint_metadata_007_quire_exact_leaf_is_exempt_but_qsl_eval_is_not() {
        let qsl_git = "git+https://github.com/agent-ix/quire-spec-language?branch=main";
        let exact_git = "git+https://github.com/agent-ix/quire-exact?branch=main";
        let document = json!({
            "packages": [
                {"id": "rt 0.1.0", "name": "quire-contract-runtime", "source": null},
                {"id": "exact 0.1.0", "name": "quire-exact", "source": exact_git},
                {"id": "eval 0.1.0", "name": "qsl-eval", "source": qsl_git},
            ],
            "resolve": {
                "nodes": [
                    {"id": "rt 0.1.0", "deps": [
                        {"name": "quire_exact", "pkg": "exact 0.1.0",
                         "dep_kinds": [{"kind": null, "target": null}]},
                        {"name": "qsl_eval", "pkg": "eval 0.1.0",
                         "dep_kinds": [{"kind": null, "target": null}]}
                    ]},
                    {"id": "exact 0.1.0", "deps": []},
                    {"id": "eval 0.1.0", "deps": [
                        {"name": "quire_exact", "pkg": "exact 0.1.0",
                         "dep_kinds": [{"kind": null, "target": null}]}
                    ]}
                ]
            }
        });
        let edges = parse_edges(&document).unwrap();
        let expected = Edge {
            from: Repo::Rt,
            to: Repo::Qsl,
            kind: EdgeKind::Normal,
            via_crate: "qsl-eval".to_owned(),
        };
        assert_eq!(edges, vec![expected.clone()]);
        let report = crate::graph::check(&edges);
        assert_eq!(report.fb05.len(), 1, "{report:?}");
        assert_eq!(report.fb05[0].edge, expected);
        assert!(report.fb11.is_empty(), "{report:?}");
    }

    /// FR-401-AC-7: the leaf `quire-eval`, git-sourced from its own
    /// repository, contributes no repository to the edge graph, so an RT
    /// resolved graph with a normal edge to it is clean and closes no FB-11
    /// cycle through it, even though it depends on `quire-exact` and
    /// `quire-semantic-value` in turn.
    #[trace("FR-401-AC-7", "TC-156")]
    #[test]
    fn a_normal_edge_to_the_quire_eval_leaf_is_clean() {
        let eval_git = "git+https://github.com/agent-ix/quire-eval?branch=main";
        let exact_git = "git+https://github.com/agent-ix/quire-exact?branch=main";
        let sv_git = "git+https://github.com/agent-ix/quire-semantic-value?branch=main";
        let normal = json!([{"kind": null, "target": null}]);
        let document = json!({
            "packages": [
                {"id": "rt 0.1.0", "name": "quire-contract-runtime", "source": null},
                {"id": "eval 0.1.0", "name": "quire-eval", "source": eval_git},
                {"id": "exact 0.1.0", "name": "quire-exact", "source": exact_git},
                {"id": "sv 0.1.0", "name": "quire-semantic-value", "source": sv_git},
            ],
            "resolve": {
                "nodes": [
                    {"id": "rt 0.1.0", "deps": [
                        {"name": "quire_eval", "pkg": "eval 0.1.0", "dep_kinds": normal}
                    ]},
                    {"id": "eval 0.1.0", "deps": [
                        {"name": "quire_exact", "pkg": "exact 0.1.0", "dep_kinds": normal},
                        {"name": "quire_semantic_value", "pkg": "sv 0.1.0", "dep_kinds": normal}
                    ]},
                    {"id": "exact 0.1.0", "deps": []},
                    {"id": "sv 0.1.0", "deps": [
                        {"name": "quire_exact", "pkg": "exact 0.1.0", "dep_kinds": normal}
                    ]}
                ]
            }
        });
        let edges = parse_edges(&document).unwrap();
        assert!(edges.is_empty(), "{edges:?}");
        let report = crate::graph::check(&edges);
        assert!(report.is_clean(), "{report:?}");
    }

    /// tc_arch_lint_metadata_008: the shared leaf `quire-semantic-value`,
    /// git-sourced from its own repository, contributes no repository to the
    /// edge graph, so an RT dependency on it yields no edge and no finding.
    /// RT's dependencies on `qsl-eval` and `qsl-semantics`, from the same QSL
    /// git source, are still RT -> QSL edges and FB-05 violations.
    #[trace("TC-156", "FR-059-AC-9")]
    #[test]
    fn tc_arch_lint_metadata_008_semantic_value_leaf_is_exempt_but_layers_are_not() {
        let qsl_git = "git+https://github.com/agent-ix/quire-spec-language?branch=main";
        let exact_git = "git+https://github.com/agent-ix/quire-exact?branch=main";
        let sv_git = "git+https://github.com/agent-ix/quire-semantic-value?branch=main";
        let normal = json!([{"kind": null, "target": null}]);
        let document = json!({
            "packages": [
                {"id": "rt 0.1.0", "name": "quire-contract-runtime", "source": null},
                {"id": "exact 0.1.0", "name": "quire-exact", "source": exact_git},
                {"id": "sv 0.1.0", "name": "quire-semantic-value", "source": sv_git},
                {"id": "sem 0.1.0", "name": "qsl-semantics", "source": qsl_git},
                {"id": "eval 0.1.0", "name": "qsl-eval", "source": qsl_git},
            ],
            "resolve": {
                "nodes": [
                    {"id": "rt 0.1.0", "deps": [
                        {"name": "quire_semantic_value", "pkg": "sv 0.1.0", "dep_kinds": normal},
                        {"name": "qsl_semantics", "pkg": "sem 0.1.0", "dep_kinds": normal},
                        {"name": "qsl_eval", "pkg": "eval 0.1.0", "dep_kinds": normal}
                    ]},
                    {"id": "exact 0.1.0", "deps": []},
                    {"id": "sv 0.1.0", "deps": [
                        {"name": "quire_exact", "pkg": "exact 0.1.0", "dep_kinds": normal}
                    ]},
                    {"id": "sem 0.1.0", "deps": [
                        {"name": "quire_semantic_value", "pkg": "sv 0.1.0", "dep_kinds": normal}
                    ]},
                    {"id": "eval 0.1.0", "deps": [
                        {"name": "qsl_semantics", "pkg": "sem 0.1.0", "dep_kinds": normal},
                        {"name": "quire_semantic_value", "pkg": "sv 0.1.0", "dep_kinds": normal}
                    ]}
                ]
            }
        });
        let edges = parse_edges(&document).unwrap();
        let rt_to = |via: &str| Edge {
            from: Repo::Rt,
            to: Repo::Qsl,
            kind: EdgeKind::Normal,
            via_crate: via.to_owned(),
        };
        assert_eq!(edges, vec![rt_to("qsl-eval"), rt_to("qsl-semantics")]);
        let report = crate::graph::check(&edges);
        let flagged: Vec<_> = report
            .fb05
            .iter()
            .map(|v| v.edge.via_crate.as_str())
            .collect();
        assert_eq!(flagged, ["qsl-eval", "qsl-semantics"], "{report:?}");
        assert!(report.fb11.is_empty(), "{report:?}");
    }

    /// tc_arch_lint_metadata_009: a crate outside the four ADR-011
    /// repositories (here `quire-walk`, git-sourced from its own repository)
    /// contributes no edge for any backend: IR, RT and CG may each depend on
    /// it with no edge and no finding. CG's dependency
    /// on `qsl-semantics`, from the QSL git source, is still a CG ->
    /// QSL edge, and the direction check refuses it, since only the replay
    /// facade's crate is CG's exception.
    #[trace("FR-356-AC-1")]
    #[test]
    fn tc_arch_lint_metadata_009_crate_outside_the_four_repositories_contributes_no_edge() {
        let qsl_git = "git+https://github.com/agent-ix/quire-spec-language?branch=main";
        let walk_git = "git+https://github.com/agent-ix/quire-walk?branch=main";
        let normal = json!([{"kind": null, "target": null}]);
        let document = json!({
            "packages": [
                {"id": "ir 0.1.0", "name": "quire-contract-ir", "source": null},
                {"id": "rt 0.1.0", "name": "quire-contract-runtime", "source": null},
                {"id": "cg 0.1.0", "name": "quire-contract-codegen", "source": null},
                {"id": "walk 0.1.0", "name": "quire-walk", "source": walk_git},
                {"id": "sem 0.1.0", "name": "qsl-semantics", "source": qsl_git},
            ],
            "resolve": {
                "nodes": [
                    {"id": "ir 0.1.0", "deps": [
                        {"name": "quire_walk", "pkg": "walk 0.1.0", "dep_kinds": normal}
                    ]},
                    {"id": "rt 0.1.0", "deps": [
                        {"name": "quire_walk", "pkg": "walk 0.1.0", "dep_kinds": normal}
                    ]},
                    {"id": "cg 0.1.0", "deps": [
                        {"name": "quire_walk", "pkg": "walk 0.1.0", "dep_kinds": normal}
                    ]},
                    {"id": "walk 0.1.0", "deps": []},
                    {"id": "sem 0.1.0", "deps": [
                        {"name": "quire_walk", "pkg": "walk 0.1.0", "dep_kinds": normal}
                    ]}
                ]
            }
        });
        let edges = parse_edges(&document).unwrap();
        assert!(edges.is_empty(), "{edges:?}");
        assert!(crate::graph::check(&edges).is_clean());

        let mut document = document;
        document["resolve"]["nodes"][2]["deps"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name": "qsl_semantics", "pkg": "sem 0.1.0", "dep_kinds": normal}));
        let edges = parse_edges(&document).unwrap();
        let expected = Edge {
            from: Repo::Cg,
            to: Repo::Qsl,
            kind: EdgeKind::Normal,
            via_crate: "qsl-semantics".to_owned(),
        };
        assert_eq!(edges, vec![expected.clone()]);
        let report = crate::graph::check(&edges);
        assert_eq!(report.fb05.len(), 1, "{report:?}");
        assert_eq!(report.fb05[0].edge, expected);
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

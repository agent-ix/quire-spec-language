// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-059 (ADR-011 §7.1 T-12): the backend direction check.
//!
//! Pure graph logic over the four ecosystem repositories. The CLI layer
//! (`metadata.rs`, `main.rs`) builds the edge list from real `cargo metadata`
//! output; this module never shells out and never reads a file, so its rules
//! are exercised by synthetic fixtures, including violating ones (deliverable
//! 5).

use std::fmt;

/// The four repositories ADR-011 §7.1 and §3 FB-05/FB-11 name. `Qsl` is the
/// repository this crate lives in; `Ir`, `Rt` and `Cg` are Contract IR,
/// Runtime and Codegen.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Repo {
    /// Quire Spec Language, this repository.
    Qsl,
    /// Contract IR.
    Ir,
    /// Contract Runtime.
    Rt,
    /// Contract Codegen.
    Cg,
}

impl Repo {
    /// Every repository, in declaration order.
    pub const ALL: [Repo; 4] = [Repo::Qsl, Repo::Ir, Repo::Rt, Repo::Cg];

    /// The repository's name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Qsl => "quire-spec-language",
            Self::Ir => "quire-contract-ir",
            Self::Rt => "quire-contract-runtime",
            Self::Cg => "quire-contract-codegen",
        }
    }
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Classify a resolved package as one of the four ADR-011 ecosystem
/// repositories, by package name or by its `source` string. This is QSL's one
/// definition of an ecosystem *repository* component: a package published
/// from the same git repository under a different crate name (for example
/// IR's own workspace member, published as `quire-contract-model`) still
/// classifies as that one repository, never a second, distinct one.
pub fn classify(name: &str, source: Option<&str>) -> Option<Repo> {
    let haystack = source.unwrap_or(name);
    if name == "quire-spec-language" || haystack.contains("quire-spec-language") {
        Some(Repo::Qsl)
    } else if name == "quire-contract-ir"
        || name == "quire-contract-model"
        || haystack.contains("quire-contract-ir")
    {
        Some(Repo::Ir)
    } else if name == "quire-contract-runtime" || haystack.contains("quire-contract-runtime") {
        Some(Repo::Rt)
    } else if name == "quire-contract-codegen" || haystack.contains("quire-contract-codegen") {
        Some(Repo::Cg)
    } else {
        None
    }
}

/// A normal dependency edge is a build-time link; a dev edge exists only for
/// tests, examples or benches. FB-11 forbids a cycle over either kind; FB-05
/// forbids most normal or dev edges into QSL.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EdgeKind {
    /// A build-time link.
    Normal,
    /// A link for tests, examples or benches only.
    Dev,
}

impl EdgeKind {
    /// The kind as `cargo metadata` names it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Dev => "dev",
        }
    }
}

/// One resolved cross-repository dependency edge: `from` depends on `to`
/// through `via_crate` (the dependency's package name, which may differ from
/// its owning repository's own crate name, e.g. IR's `quire-contract-model`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edge {
    /// The depending repository.
    pub from: Repo,
    /// The repository depended on.
    pub to: Repo,
    /// Normal or dev.
    pub kind: EdgeKind,
    /// The dependency's package name.
    pub via_crate: String,
}

/// One FB-05 finding: an edge into QSL that the only stated exception
/// (ADR-011 §3 FB-05: CG's normal dependency on the QSL layer-6 `replay`
/// facade) does not cover.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fb05Violation {
    /// The edge into QSL.
    pub edge: Edge,
}

/// One FB-11 finding: a cycle over QSL/IR/RT/CG, normal and dev edges
/// combined. `path` lists the repositories in cycle order, starting and
/// ending at the same repository.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fb11Cycle {
    /// The repositories in cycle order, first and last equal.
    pub path: Vec<Repo>,
}

/// Every FB-05 and FB-11 finding over one edge list.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DirectionReport {
    /// The FB-05 findings.
    pub fb05: Vec<Fb05Violation>,
    /// The FB-11 cycles.
    pub fb11: Vec<Fb11Cycle>,
}

impl DirectionReport {
    /// Whether there is no finding.
    pub fn is_clean(&self) -> bool {
        self.fb05.is_empty() && self.fb11.is_empty()
    }
}

/// The QSL crate that holds the layer-6 `replay` facade, the one QSL crate
/// CG may take a normal dependency on (ADR-011 FB-05).
const REPLAY_FACADE_CRATE: &str = "qsl-replay";

/// ADR-011 §3 FB-05: no backend (IR, RT, CG) depends on QSL, normal or dev,
/// except CG's normal dependency on the layer-6 `replay` facade's crate,
/// [`REPLAY_FACADE_CRATE`]. A CG edge to any other QSL crate, such as
/// `qsl-semantics`, is a finding. This check is crate-level only -- FR-060's
/// API-surface check is what verifies the dependency is used through
/// `replay` alone. A shared leaf lives in its own repository, which
/// `classify` places in no ecosystem repository, so its edges never reach
/// this check.
fn fb05_violations(edges: &[Edge]) -> Vec<Fb05Violation> {
    edges
        .iter()
        .filter(|edge| edge.to == Repo::Qsl && edge.from != Repo::Qsl)
        .filter(|edge| {
            !(edge.from == Repo::Cg
                && edge.kind == EdgeKind::Normal
                && edge.via_crate == REPLAY_FACADE_CRATE)
        })
        .cloned()
        .map(|edge| Fb05Violation { edge })
        .collect()
}

/// ADR-011 §3 FB-11: no dependency or test-time edge closes a cycle among
/// QSL, IR, RT and CG. Normal and dev edges are combined into one directed
/// graph over the four repositories, then every simple cycle reachable from
/// each repository is reported once (by its lexicographically smallest
/// rotation, so `A -> B -> A` and `B -> A -> B` are the same finding).
fn fb11_cycles(edges: &[Edge]) -> Vec<Fb11Cycle> {
    let mut adjacency: Vec<(Repo, Repo)> = edges
        .iter()
        .filter(|edge| edge.from != edge.to)
        .map(|edge| (edge.from, edge.to))
        .collect();
    adjacency.sort();
    adjacency.dedup();

    let mut found: Vec<Vec<Repo>> = Vec::new();
    for &start in &Repo::ALL {
        let mut stack = vec![start];
        let mut visited = vec![start];
        find_cycles(start, &adjacency, &mut stack, &mut visited, &mut found);
    }

    // Canonicalize: rotate each cycle to start at its smallest repo, then
    // dedup so a cycle found from two different starting points counts once.
    let mut canonical: Vec<Vec<Repo>> = found
        .into_iter()
        .map(|mut path| {
            path.pop(); // drop the repeated closing node; re-added below
            let min_pos = path
                .iter()
                .enumerate()
                .min_by_key(|(_, repo)| **repo)
                .map(|(index, _)| index)
                .unwrap_or(0);
            path.rotate_left(min_pos);
            path.push(path[0]);
            path
        })
        .collect();
    canonical.sort();
    canonical.dedup();
    canonical
        .into_iter()
        .map(|path| Fb11Cycle { path })
        .collect()
}

fn find_cycles(
    start: Repo,
    adjacency: &[(Repo, Repo)],
    stack: &mut Vec<Repo>,
    visited: &mut Vec<Repo>,
    found: &mut Vec<Vec<Repo>>,
) {
    let current = *stack.last().expect("stack is never empty while walking");
    for &(_, to) in adjacency.iter().filter(|(from, _)| *from == current) {
        if to == start && stack.len() > 1 {
            let mut path = stack.clone();
            path.push(to);
            found.push(path);
            continue;
        }
        if visited.contains(&to) {
            continue;
        }
        stack.push(to);
        visited.push(to);
        find_cycles(start, adjacency, stack, visited, found);
        stack.pop();
        visited.pop();
    }
}

/// FR-059's direction check over `edges`: every FB-05 violation and FB-11
/// cycle.
pub fn check(edges: &[Edge]) -> DirectionReport {
    DirectionReport {
        fb05: fb05_violations(edges),
        fb11: fb11_cycles(edges),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn edge(from: Repo, to: Repo, kind: EdgeKind, via: &str) -> Edge {
        Edge {
            from,
            to,
            kind,
            via_crate: via.to_owned(),
        }
    }

    /// tc_arch_lint_direction_001: a clean ecosystem (only the CG -> QSL
    /// normal replay-facade edge into QSL, no cycle) reports nothing.
    #[trace("TC-156", "FR-059-AC-1")]
    #[test]
    fn tc_arch_lint_direction_001_clean_graph_reports_nothing() {
        let edges = vec![
            edge(Repo::Cg, Repo::Qsl, EdgeKind::Normal, "qsl-replay"),
            edge(Repo::Cg, Repo::Ir, EdgeKind::Normal, "quire-contract-ir"),
            edge(Repo::Cg, Repo::Rt, EdgeKind::Dev, "quire-contract-runtime"),
        ];
        let report = check(&edges);
        assert!(report.is_clean(), "{report:?}");
    }

    /// tc_arch_lint_direction_002 (negative control): a synthetic IR -> QSL
    /// normal edge exercises the FB-05 check and is reported: the only
    /// permitted edge into QSL is CG's normal dependency.
    #[trace("TC-156", "FR-059-AC-2")]
    #[test]
    fn tc_arch_lint_direction_002_ir_to_qsl_is_fb05_violation() {
        let edges = vec![edge(
            Repo::Ir,
            Repo::Qsl,
            EdgeKind::Normal,
            "quire-spec-language",
        )];
        let report = check(&edges);
        assert_eq!(report.fb05.len(), 1);
        assert_eq!(report.fb05[0].edge.from, Repo::Ir);
        assert!(report.fb11.is_empty());
    }

    /// tc_arch_lint_direction_003 (negative control): a dev-only edge from
    /// RT into QSL is still forbidden -- FB-05 has no dev exception.
    #[trace("TC-156", "FR-059-AC-2")]
    #[test]
    fn tc_arch_lint_direction_003_dev_edge_into_qsl_is_violation() {
        let edges = vec![edge(
            Repo::Rt,
            Repo::Qsl,
            EdgeKind::Dev,
            "quire-spec-language",
        )];
        let report = check(&edges);
        assert_eq!(report.fb05.len(), 1);
        assert_eq!(report.fb05[0].edge.kind, EdgeKind::Dev);
    }

    /// tc_arch_lint_direction_004: CG's normal dependency on the replay
    /// facade's crate is the one stated exception and is not reported; its
    /// normal dependency on any other QSL crate, the layer-3
    /// `qsl-semantics` or the root crate, is reported.
    #[trace("TC-156", "FR-059-AC-1", "FR-356-AC-1")]
    #[test]
    fn tc_arch_lint_direction_004_cg_normal_edge_is_the_named_exception() {
        let report = check(&[edge(Repo::Cg, Repo::Qsl, EdgeKind::Normal, "qsl-replay")]);
        assert!(report.fb05.is_empty(), "{report:?}");
        let edges = vec![
            edge(Repo::Cg, Repo::Qsl, EdgeKind::Normal, "qsl-semantics"),
            edge(Repo::Cg, Repo::Qsl, EdgeKind::Normal, "quire-spec-language"),
        ];
        let flagged: Vec<_> = check(&edges)
            .fb05
            .into_iter()
            .map(|violation| violation.edge.via_crate)
            .collect();
        assert_eq!(flagged, ["qsl-semantics", "quire-spec-language"]);
    }

    /// tc_arch_lint_direction_005 (negative control): CG's *dev* dependency
    /// on QSL is not the stated exception (the exception names a normal
    /// dependency only) and is reported.
    #[trace("TC-156", "FR-059-AC-3")]
    #[test]
    fn tc_arch_lint_direction_005_cg_dev_edge_into_qsl_is_violation() {
        let edges = vec![edge(
            Repo::Cg,
            Repo::Qsl,
            EdgeKind::Dev,
            "quire-spec-language",
        )];
        let report = check(&edges);
        assert_eq!(report.fb05.len(), 1);
    }

    /// tc_arch_lint_direction_006 (negative control): QSL -> IR -> QSL is a
    /// two-repository FB-11 cycle.
    #[trace("TC-156", "FR-059-AC-4")]
    #[test]
    fn tc_arch_lint_direction_006_two_repo_cycle_is_fb11_violation() {
        let edges = vec![
            edge(
                Repo::Qsl,
                Repo::Ir,
                EdgeKind::Normal,
                "quire-contract-model",
            ),
            edge(Repo::Ir, Repo::Qsl, EdgeKind::Normal, "quire-spec-language"),
        ];
        let report = check(&edges);
        assert_eq!(report.fb11.len(), 1);
        assert_eq!(report.fb11[0].path.first(), report.fb11[0].path.last());
    }

    /// tc_arch_lint_direction_007 (negative control): a longer QSL -> CG ->
    /// RT -> QSL cycle is also caught, combining normal and dev edges.
    #[trace("TC-156", "FR-059-AC-4")]
    #[test]
    fn tc_arch_lint_direction_007_three_repo_cycle_is_fb11_violation() {
        let edges = vec![
            edge(Repo::Qsl, Repo::Cg, EdgeKind::Dev, "quire-contract-codegen"),
            edge(
                Repo::Cg,
                Repo::Rt,
                EdgeKind::Normal,
                "quire-contract-runtime",
            ),
            edge(Repo::Rt, Repo::Qsl, EdgeKind::Dev, "quire-spec-language"),
        ];
        let report = check(&edges);
        assert_eq!(report.fb11.len(), 1);
        assert_eq!(report.fb11[0].path.len(), 4);
    }

    /// tc_arch_lint_direction_009: `classify` maps a package published from
    /// IR's repository under a different crate name (`quire-contract-model`)
    /// to the same `Repo::Ir` as the repository's own facade package name,
    /// so the direction check reads QSL's edge on `quire-contract-model` as
    /// QSL -> IR.
    #[trace("TC-156", "FR-059-AC-6")]
    #[test]
    fn tc_arch_lint_direction_009_classify_shares_repo_across_package_names() {
        assert_eq!(classify("quire-contract-ir", None), Some(Repo::Ir));
        assert_eq!(
            classify(
                "quire-contract-model",
                Some("git+https://github.com/agent-ix/quire-contract-ir?rev=53cc03c#53cc03c")
            ),
            Some(Repo::Ir)
        );
        assert_eq!(classify("serde", None), None);
    }

    /// tc_arch_lint_direction_008: a linear chain with no return edge is not
    /// a cycle.
    #[trace("TC-156", "FR-059-AC-5")]
    #[test]
    fn tc_arch_lint_direction_008_acyclic_chain_reports_no_cycle() {
        let edges = vec![
            edge(Repo::Cg, Repo::Ir, EdgeKind::Normal, "quire-contract-ir"),
            edge(
                Repo::Cg,
                Repo::Rt,
                EdgeKind::Normal,
                "quire-contract-runtime",
            ),
            edge(Repo::Cg, Repo::Qsl, EdgeKind::Normal, "qsl-replay"),
        ];
        let report = check(&edges);
        assert!(report.fb11.is_empty());
    }
}

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
pub(crate) enum Repo {
    Qsl,
    Ir,
    Rt,
    Cg,
}

impl Repo {
    pub(crate) const ALL: [Repo; 4] = [Repo::Qsl, Repo::Ir, Repo::Rt, Repo::Cg];

    pub(crate) fn as_str(self) -> &'static str {
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

/// A normal dependency edge is a build-time link; a dev edge exists only for
/// tests, examples or benches. FB-11 forbids a cycle over either kind; FB-05
/// forbids most normal or dev edges into QSL.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EdgeKind {
    Normal,
    Dev,
}

impl EdgeKind {
    pub(crate) fn as_str(self) -> &'static str {
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
pub(crate) struct Edge {
    pub(crate) from: Repo,
    pub(crate) to: Repo,
    pub(crate) kind: EdgeKind,
    pub(crate) via_crate: String,
}

/// One FB-05 finding: an edge into QSL that the only stated exception
/// (ADR-011 §3 FB-05: CG's normal dependency on the QSL layer-6 `replay`
/// facade) does not cover.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Fb05Violation {
    pub(crate) edge: Edge,
}

/// One FB-11 finding: a cycle over QSL/IR/RT/CG, normal and dev edges
/// combined. `path` lists the repositories in cycle order, starting and
/// ending at the same repository.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Fb11Cycle {
    pub(crate) path: Vec<Repo>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct DirectionReport {
    pub(crate) fb05: Vec<Fb05Violation>,
    pub(crate) fb11: Vec<Fb11Cycle>,
}

impl DirectionReport {
    pub(crate) fn is_clean(&self) -> bool {
        self.fb05.is_empty() && self.fb11.is_empty()
    }
}

/// ADR-011 §3 FB-05: no backend (IR, RT, CG) depends on QSL, normal or dev,
/// except CG's normal dependency on QSL (the layer-6 `replay` facade's crate;
/// this check is crate-level only -- FR-060's API-surface check is what
/// verifies the dependency is used through `replay` alone).
fn fb05_violations(edges: &[Edge]) -> Vec<Fb05Violation> {
    edges
        .iter()
        .filter(|edge| edge.to == Repo::Qsl && edge.from != Repo::Qsl)
        .filter(|edge| !(edge.from == Repo::Cg && edge.kind == EdgeKind::Normal))
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

pub(crate) fn check(edges: &[Edge]) -> DirectionReport {
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
            edge(Repo::Cg, Repo::Qsl, EdgeKind::Normal, "quire-spec-language"),
            edge(Repo::Cg, Repo::Ir, EdgeKind::Normal, "quire-contract-ir"),
            edge(Repo::Cg, Repo::Rt, EdgeKind::Dev, "quire-contract-runtime"),
        ];
        let report = check(&edges);
        assert!(report.is_clean(), "{report:?}");
    }

    /// tc_arch_lint_direction_002 (negative control): IR depending on QSL is
    /// the real, currently observed FB-05 violation (ADR-011 OBS-029): the
    /// only permitted edge into QSL is CG's normal dependency.
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

    /// tc_arch_lint_direction_004: CG's normal dependency on QSL is the one
    /// stated exception and is not reported.
    #[trace("TC-156", "FR-059-AC-1")]
    #[test]
    fn tc_arch_lint_direction_004_cg_normal_edge_is_the_named_exception() {
        let edges = vec![edge(
            Repo::Cg,
            Repo::Qsl,
            EdgeKind::Normal,
            "quire-spec-language",
        )];
        let report = check(&edges);
        assert!(report.fb05.is_empty());
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
    /// two-repository cycle (ADR-011 §7.1: "QSL's own Cargo.lock resolves
    /// exactly one revision per quire-ecosystem crate" is a different rule;
    /// this is FB-11's direction cycle, observed today as IR root -> QSL).
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
            edge(Repo::Cg, Repo::Qsl, EdgeKind::Normal, "quire-spec-language"),
        ];
        let report = check(&edges);
        assert!(report.fb11.is_empty());
    }
}

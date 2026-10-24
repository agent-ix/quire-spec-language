// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-825: which workspace packages the all-features test lane can skip.
//!
//! The default-feature and all-feature lanes both run every package's tests.
//! A package whose compiled code is the same in both runs the same binaries
//! twice. This module proves, per package and from `cargo metadata`, which
//! packages those are, so the all-features lane runs the rest.
//!
//! **What "compiles the same" means.** With resolver 2, a package with no
//! optional features of its own still compiles differently when
//! `--all-features` turns a feature on in one of its dependencies. So the
//! proof compares the resolve of `cargo metadata` against the resolve of
//! `cargo metadata --all-features`, over the package and every package it
//! transitively depends on (dependencies of every kind, since a test build
//! uses the dev-dependencies too): the same packages must be reachable, each
//! with the same resolved feature set. Any difference keeps the package in
//! the all-features lane.
//!
//! **Metadata only nominates.** `cargo metadata` reports one merged feature
//! set per package across the host, the target and every platform, and
//! resolver 2 builds those separately, so equal metadata does not prove an
//! equal compile. The metadata comparison therefore only nominates
//! candidates; cargo's own per-unit resolve (`cargo tree` on the host platform
//! the lane compiles for, host and target units kept apart) then checks them
//! twice. First, a candidate's compile units in the default workspace resolve
//! must be exactly its units in the all-features resolve
//! ([`TreeFeatures::compiles_same_as`]), so the default lane's run of it is the
//! run the all-features lane would repeat. Second, [`settle`] puts back any
//! candidate whose absence changes the feature set of a package that stays. A
//! skip that cannot be shown is a run.
//!
//! **What skipping a package changes for the others.** `cargo test
//! --exclude P` leaves `P` out of the selection, and with it `P`'s own
//! `[dev-dependencies]` feature requests: a feature that only `P` asked of a
//! dependency is then off in the all-features run of the packages that
//! remain. A skipped package therefore also needs every feature it requests
//! of a dependency to be requested by a package that stays. A package that
//! fails this is kept, and keeping it can cover another, so the check runs to
//! a fixed point.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde_json::Value;

use crate::test_suites::{array_field, field, string_field, ArtifactError};

/// A feature one package asks of one of its dependencies in its manifest
/// (`features = [...]` on the dependency).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Request {
    dependency: String,
    feature: String,
}

/// One node of a `cargo metadata` resolve: the resolved features of a package
/// and the packages it depends on.
#[derive(Debug)]
struct Node {
    features: BTreeSet<String>,
    dependencies: BTreeSet<String>,
}

/// The parts of a `cargo metadata --format-version 1` document the proof
/// reads.
#[derive(Debug)]
pub struct Resolution {
    names: BTreeMap<String, String>,
    nodes: BTreeMap<String, Node>,
    members: Vec<String>,
    requests: BTreeMap<String, BTreeSet<Request>>,
}

fn strings(value: &Value, name: &'static str) -> Result<BTreeSet<String>, ArtifactError> {
    array_field(value, name)?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or(ArtifactError::MalformedField { field: name })
        })
        .collect()
}

impl Resolution {
    /// Read `cargo metadata --format-version 1`'s stdout (with dependencies).
    /// Every package needs a string `id` and `name` and a `dependencies`
    /// array whose entries have a `name` and a `features` array; every
    /// workspace member id must name a listed package with a resolve node,
    /// and every node needs `features` and `deps` with a `pkg` each.
    pub fn from_cargo_metadata(stdout: &str) -> Result<Self, ArtifactError> {
        let metadata: Value = serde_json::from_str(stdout)?;
        let mut names = BTreeMap::new();
        let mut requests = BTreeMap::new();
        for package in array_field(&metadata, "packages")? {
            let id = string_field(package, "id", "packages.id")?.to_owned();
            let mut asked = BTreeSet::new();
            for dependency in array_field(package, "dependencies")? {
                let name = string_field(dependency, "name", "packages.dependencies.name")?;
                for feature in strings(dependency, "features")? {
                    asked.insert(Request {
                        dependency: name.to_owned(),
                        feature,
                    });
                }
            }
            names.insert(
                id.clone(),
                string_field(package, "name", "packages.name")?.to_owned(),
            );
            requests.insert(id, asked);
        }
        let resolve = field(&metadata, "resolve", "resolve")?;
        let mut nodes = BTreeMap::new();
        for node in array_field(resolve, "nodes")? {
            let id = string_field(node, "id", "resolve.nodes.id")?.to_owned();
            let dependencies = array_field(node, "deps")?
                .iter()
                .map(|dependency| {
                    string_field(dependency, "pkg", "resolve.nodes.deps.pkg").map(str::to_owned)
                })
                .collect::<Result<_, _>>()?;
            let features = strings(node, "features")?;
            nodes.insert(
                id,
                Node {
                    features,
                    dependencies,
                },
            );
        }
        let members: Vec<String> = strings(&metadata, "workspace_members")?
            .into_iter()
            .collect();
        for member in &members {
            if !names.contains_key(member) || !nodes.contains_key(member) {
                return Err(ArtifactError::MalformedField {
                    field: "workspace_members",
                });
            }
        }
        Ok(Self {
            names,
            nodes,
            members,
            requests,
        })
    }

    fn name<'a>(&'a self, id: &'a str) -> &'a str {
        self.names.get(id).map_or(id, String::as_str)
    }

    /// The packages `root` transitively depends on, `root` included.
    fn closure<'a>(&'a self, root: &'a str) -> BTreeSet<&'a str> {
        let mut seen = BTreeSet::new();
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(node) = self.nodes.get(id) {
                pending.extend(node.dependencies.iter().map(String::as_str));
            }
        }
        seen
    }
}

/// Why a package stays in the all-features lane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeepReason {
    /// The package's own resolved features differ under `--all-features`.
    OwnFeatures,
    /// A package it transitively depends on resolves differently under
    /// `--all-features` (different features, or reachable only in one).
    DependencyResolvesDifferently {
        /// That package's name.
        dependency: String,
    },
    /// Its manifest requests a feature of a dependency that no package
    /// staying in the lane requests, so leaving it out would turn that
    /// feature off for the packages that stay.
    SuppliesFeature {
        /// The dependency.
        dependency: String,
        /// The feature requested.
        feature: String,
    },
    /// `cargo tree` on the host does not confirm the skip: the package's
    /// compile units differ between the default and the all-features
    /// workspace resolve (a host and a target unit of a dependency that
    /// `--all-features` merges), or leaving it out changes the resolved
    /// features of a package that stays (a request further down its
    /// dependencies, `default-features`, a `[features]` activation, a build
    /// edge).
    ExclusionChangesResolve,
}

impl fmt::Display for KeepReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OwnFeatures => f.write_str("its own resolved features change"),
            Self::DependencyResolvesDifferently { dependency } => {
                write!(f, "dependency {dependency} resolves differently")
            }
            Self::SuppliesFeature {
                dependency,
                feature,
            } => write!(
                f,
                "it alone requests feature {feature} of {dependency} for the remaining packages"
            ),
            Self::ExclusionChangesResolve => f.write_str(
                "cargo tree does not confirm the skip: its compile units differ between the \
                 lanes, or leaving it out changes a remaining package",
            ),
        }
    }
}

/// What the all-features lane does with a package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Verdict {
    /// The package and everything it depends on resolve identically in both
    /// lanes: the default lane already ran its tests on the same compile.
    Skip,
    /// The package's tests run in the all-features lane.
    Run(KeepReason),
}

/// The decision for one workspace package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decision {
    /// The package name.
    pub package: String,
    /// Skip or run, with the reason.
    pub verdict: Verdict,
}

impl fmt::Display for Decision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.verdict {
            Verdict::Skip => write!(
                f,
                "{}: skipped; it and its dependencies resolve identically in the default run",
                self.package
            ),
            Verdict::Run(reason) => write!(f, "{}: runs; {reason}", self.package),
        }
    }
}

/// The first reason `member` resolves differently between the two runs, by
/// package name.
fn difference(default: &Resolution, all: &Resolution, member: &str) -> Option<KeepReason> {
    let reachable_default = default.closure(member);
    let reachable_all = all.closure(member);
    let mut differing = BTreeSet::new();
    for id in reachable_default.union(&reachable_all) {
        let same = match (default.nodes.get(*id), all.nodes.get(*id)) {
            (Some(left), Some(right)) => left.features == right.features,
            _ => false,
        };
        if !same || !(reachable_default.contains(id) && reachable_all.contains(id)) {
            differing.insert(default.name(id));
        }
    }
    if differing.contains(default.name(member)) {
        return Some(KeepReason::OwnFeatures);
    }
    differing
        .into_iter()
        .next()
        .map(|dependency| KeepReason::DependencyResolvesDifferently {
            dependency: dependency.to_owned(),
        })
}

/// Decide every workspace member, from the resolve without and with
/// `--all-features`. The members come from `default`; `all` must list the
/// same ones.
pub fn decide(default: &Resolution, all: &Resolution) -> Result<Vec<Decision>, ArtifactError> {
    if default.members != all.members {
        return Err(ArtifactError::MalformedField {
            field: "workspace_members",
        });
    }
    let mut kept: BTreeMap<&str, KeepReason> = BTreeMap::new();
    for member in &default.members {
        if let Some(reason) = difference(default, all, member) {
            kept.insert(member, reason);
        }
    }
    // A skipped package takes its feature requests with it; keep packages
    // until every skipped one only asks for what a kept package also asks.
    loop {
        let covered: BTreeSet<&Request> = kept
            .keys()
            .filter_map(|id| default.requests.get(*id))
            .flatten()
            .collect();
        let uncovered = default
            .members
            .iter()
            .filter(|member| !kept.contains_key(member.as_str()))
            .find_map(|member| {
                default
                    .requests
                    .get(member)?
                    .iter()
                    .find(|request| !covered.contains(request))
                    .map(|request| (member.as_str(), request.clone()))
            });
        let Some((member, request)) = uncovered else {
            break;
        };
        kept.insert(
            member,
            KeepReason::SuppliesFeature {
                dependency: request.dependency,
                feature: request.feature,
            },
        );
    }
    let mut decisions: Vec<Decision> = default
        .members
        .iter()
        .map(|member| Decision {
            package: default.name(member).to_owned(),
            verdict: kept
                .remove(member.as_str())
                .map_or(Verdict::Skip, Verdict::Run),
        })
        .collect();
    decisions.sort_by(|left, right| left.package.cmp(&right.package));
    Ok(decisions)
}

/// The packages `decisions` skip, by name: the `--exclude` arguments of the
/// all-features lane.
pub fn skipped(decisions: &[Decision]) -> Vec<String> {
    decisions
        .iter()
        .filter(|decision| decision.verdict == Verdict::Skip)
        .map(|decision| decision.package.clone())
        .collect()
}

/// The per-unit resolve of one `cargo tree --prefix depth --format "@{p}|{f}"`
/// run on the host platform: for each package, every distinct feature set it
/// resolves to, and the dependency edges between compile units. Host (build
/// script, proc-macro) and target units of one package have their own feature
/// sets under resolver 2, and each appears as its own unit.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TreeFeatures {
    variants: BTreeMap<String, BTreeSet<String>>,
    /// Unit (`package|features`) to the units it depends on.
    edges: BTreeMap<String, BTreeSet<String>>,
    /// Workspace package name to its root unit.
    roots: BTreeMap<String, String>,
}

impl TreeFeatures {
    /// Read the stdout of `cargo tree --prefix depth --format "@{p}|{f}"`: each
    /// line is the depth, `@`, the package and `|`, then its features. Lines
    /// without that shape (blank lines, headings) are skipped. Cargo prints a
    /// repeated subtree once and marks the later occurrences `(*)`, so a unit's
    /// edges are the union over its occurrences.
    pub fn from_cargo_tree(stdout: &str) -> Self {
        let mut tree = Self::default();
        let mut stack: Vec<String> = Vec::new();
        for line in stdout.lines() {
            let Some((depth, rest)) = line.split_once('@') else {
                continue;
            };
            let Ok(depth) = depth.parse::<usize>() else {
                continue;
            };
            let rest = rest.trim_end();
            let rest = rest.strip_suffix(" (*)").unwrap_or(rest);
            let Some((package, features)) = rest.rsplit_once('|') else {
                continue;
            };
            tree.variants
                .entry(package.to_owned())
                .or_default()
                .insert(features.to_owned());
            let unit = rest.to_owned();
            stack.truncate(depth);
            if let Some(parent) = stack.last() {
                tree.edges
                    .entry(parent.clone())
                    .or_default()
                    .insert(unit.clone());
            } else if let Some(name) = package.split_whitespace().next() {
                tree.roots.insert(name.to_owned(), unit.clone());
            }
            stack.push(unit);
        }
        tree
    }

    /// Every compile unit reachable from the workspace package `name`, itself
    /// included; `None` when the run has no such package.
    fn compile_of(&self, name: &str) -> Option<BTreeSet<&str>> {
        let mut seen = BTreeSet::new();
        let mut pending = vec![self.roots.get(name)?.as_str()];
        while let Some(unit) = pending.pop() {
            if !seen.insert(unit) {
                continue;
            }
            if let Some(children) = self.edges.get(unit) {
                pending.extend(children.iter().map(String::as_str));
            }
        }
        Some(seen)
    }

    /// Whether the workspace package `name` and everything it depends on are
    /// the same compile units, with the same feature sets, in `self` and in
    /// `other`. A package missing from either run is not the same.
    pub fn compiles_same_as(&self, other: &Self, name: &str) -> bool {
        matches!(
            (self.compile_of(name), other.compile_of(name)),
            (Some(left), Some(right)) if left == right
        )
    }

    /// The packages of `self` (a run with packages left out) whose set of
    /// feature sets is not the one `full` has, or that `full` lacks. A
    /// package that only the left-out packages used is absent from `self` and
    /// is not a difference.
    pub fn differences(&self, full: &Self) -> Vec<String> {
        self.variants
            .iter()
            .filter(|(package, variants)| full.variants.get(*package) != Some(*variants))
            .map(|(package, _)| package.clone())
            .collect()
    }
}

/// Reduce `candidates` to the packages that can be left out without changing
/// the resolved features of any package that stays. `excluded_tree` runs
/// `cargo tree` with the given packages left out; `full` is the run with none.
///
/// While the run differs from `full`, the candidate whose return leaves the
/// fewest differences is put back (the first by name on a tie) and the run is
/// repeated. With no candidates left the run is `full`, so the loop ends; in
/// the worst case the whole workspace runs.
pub fn settle<E>(
    candidates: &[String],
    full: &TreeFeatures,
    excluded_tree: &mut impl FnMut(&[String]) -> Result<TreeFeatures, E>,
) -> Result<Vec<String>, E> {
    let mut skip: Vec<String> = candidates.to_vec();
    skip.sort();
    skip.dedup();
    loop {
        if skip.is_empty() || excluded_tree(&skip)?.differences(full).is_empty() {
            return Ok(skip);
        }
        let mut best: Option<(usize, usize)> = None;
        for index in 0..skip.len() {
            let mut without = skip.clone();
            without.remove(index);
            let remaining = excluded_tree(&without)?.differences(full).len();
            if best.is_none_or(|(fewest, _)| remaining < fewest) {
                best = Some((remaining, index));
            }
        }
        let Some((_, index)) = best else {
            return Ok(skip);
        };
        skip.remove(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// One fixture package: its name, the resolved features and dependencies
    /// in a run, and the feature requests its manifest makes.
    struct Pkg {
        name: &'static str,
        features: &'static [&'static str],
        deps: &'static [&'static str],
        asks: &'static [(&'static str, &'static str)],
    }

    fn pkg(name: &'static str, features: &'static [&'static str]) -> Pkg {
        Pkg {
            name,
            features,
            deps: &[],
            asks: &[],
        }
    }

    fn metadata(members: &[&'static str], packages: &[Pkg]) -> String {
        let id = |name: &str| format!("{name} 0.1.0");
        json!({
            "workspace_members": members.iter().map(|name| id(name)).collect::<Vec<_>>(),
            "packages": packages.iter().map(|p| json!({
                "id": id(p.name),
                "name": p.name,
                "dependencies": p.asks.iter().map(|(dependency, feature)| json!({
                    "name": dependency,
                    "features": [feature],
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "resolve": {"nodes": packages.iter().map(|p| json!({
                "id": id(p.name),
                "features": p.features,
                "deps": p.deps.iter().map(|d| json!({"pkg": id(d)})).collect::<Vec<_>>(),
            })).collect::<Vec<_>>()},
        })
        .to_string()
    }

    fn resolve(members: &[&'static str], packages: &[Pkg]) -> Resolution {
        Resolution::from_cargo_metadata(&metadata(members, packages)).unwrap()
    }

    fn decisions(members: &[&'static str], default: &[Pkg], all: &[Pkg]) -> Vec<Decision> {
        decide(&resolve(members, default), &resolve(members, all)).unwrap()
    }

    fn decide_fixture(
        members: &[&'static str],
        default: &[Pkg],
        all: &[Pkg],
    ) -> BTreeMap<String, Verdict> {
        decisions(members, default, all)
            .into_iter()
            .map(|decision| (decision.package, decision.verdict))
            .collect()
    }

    fn dep(name: &'static str, deps: &'static [&'static str]) -> Pkg {
        Pkg {
            name,
            features: &[],
            deps,
            asks: &[],
        }
    }

    #[test]
    fn a_package_whose_resolved_features_are_identical_is_skipped() {
        let both = [dep("leaf", &[]), pkg("featured", &[])];
        let all = [dep("leaf", &[]), pkg("featured", &["extra"])];
        let verdicts = decide_fixture(&["leaf", "featured"], &both, &all);
        assert_eq!(verdicts["leaf"], Verdict::Skip);
        assert_eq!(verdicts["featured"], Verdict::Run(KeepReason::OwnFeatures));
    }

    #[test]
    fn a_package_without_features_stays_when_a_dependency_gains_features() {
        let default = [dep("app", &["lib"]), pkg("lib", &[])];
        let all = [dep("app", &["lib"]), pkg("lib", &["extra"])];
        let verdicts = decide_fixture(&["app", "lib"], &default, &all);
        assert_eq!(
            verdicts["app"],
            Verdict::Run(KeepReason::DependencyResolvesDifferently {
                dependency: "lib".to_owned()
            })
        );
        assert_eq!(verdicts["lib"], Verdict::Run(KeepReason::OwnFeatures));
    }

    #[test]
    fn the_change_is_found_through_a_chain_of_dependencies() {
        let default = [
            dep("top", &["mid"]),
            dep("mid", &["third"]),
            pkg("third", &[]),
        ];
        let all = [
            dep("top", &["mid"]),
            dep("mid", &["third"]),
            pkg("third", &["std"]),
        ];
        let verdicts = decide_fixture(&["top"], &default, &all);
        assert_eq!(
            verdicts["top"],
            Verdict::Run(KeepReason::DependencyResolvesDifferently {
                dependency: "third".to_owned()
            })
        );
    }

    #[test]
    fn a_dependency_reachable_only_under_all_features_keeps_the_package() {
        let default = [dep("app", &[]), dep("opt", &[])];
        let all = [dep("app", &["opt"]), dep("opt", &[])];
        let verdicts = decide_fixture(&["app"], &default, &all);
        assert_eq!(
            verdicts["app"],
            Verdict::Run(KeepReason::DependencyResolvesDifferently {
                dependency: "opt".to_owned()
            })
        );
    }

    #[test]
    fn a_skipped_package_may_not_be_the_only_requester_of_a_feature() {
        let packages = || {
            [
                Pkg {
                    name: "tests-only",
                    features: &[],
                    deps: &["lib"],
                    asks: &[("lib", "test-support")],
                },
                pkg("lib", &["test-support"]),
            ]
        };
        let verdicts = decide_fixture(&["tests-only", "lib"], &packages(), &packages());
        // Neither package changes under all-features, but leaving out the only
        // package that asks for `test-support` would turn it off for `lib`.
        assert_eq!(
            verdicts["tests-only"],
            Verdict::Run(KeepReason::SuppliesFeature {
                dependency: "lib".to_owned(),
                feature: "test-support".to_owned()
            })
        );
        assert_eq!(verdicts["lib"], Verdict::Skip);
    }

    #[test]
    fn a_feature_requested_by_a_staying_package_covers_the_skipped_requester() {
        let asking = |name| Pkg {
            name,
            features: &[],
            deps: &["lib"],
            asks: &[("lib", "test-support")],
        };
        let default = [
            asking("first"),
            asking("second"),
            pkg("lib", &["test-support"]),
        ];
        let all = [
            asking("first"),
            asking("second"),
            pkg("lib", &["test-support"]),
        ];
        let verdicts = decide_fixture(&["first", "second", "lib"], &default, &all);
        // The first requester stays to supply the feature; the second asks for
        // nothing it does not supply.
        assert_eq!(
            verdicts["first"],
            Verdict::Run(KeepReason::SuppliesFeature {
                dependency: "lib".to_owned(),
                feature: "test-support".to_owned()
            })
        );
        assert_eq!(verdicts["second"], Verdict::Skip);
    }

    #[test]
    fn skipped_names_the_skip_decisions_in_package_order() {
        let both = [dep("zeta", &[]), dep("alpha", &[]), pkg("mid", &["x"])];
        let all = [dep("zeta", &[]), dep("alpha", &[]), pkg("mid", &["x", "y"])];
        let decisions = decisions(&["zeta", "alpha", "mid"], &both, &all);
        assert_eq!(
            skipped(&decisions),
            vec!["alpha".to_owned(), "zeta".to_owned()]
        );
    }

    #[test]
    fn different_member_lists_and_malformed_documents_are_refused() {
        let one = resolve(&["a"], &[dep("a", &[])]);
        let other = resolve(&["a", "b"], &[dep("a", &[]), dep("b", &[])]);
        assert!(decide(&one, &other).is_err());
        assert!(Resolution::from_cargo_metadata("{}").is_err());
        assert!(Resolution::from_cargo_metadata("not json").is_err());
        // A member with no resolve node is a malformed document, not a skip.
        let dangling = json!({
            "workspace_members": ["ghost 0.1.0"],
            "packages": [],
            "resolve": {"nodes": []},
        })
        .to_string();
        assert!(Resolution::from_cargo_metadata(&dangling).is_err());
    }

    fn tree(lines: &[&str]) -> TreeFeatures {
        TreeFeatures::from_cargo_tree(&lines.join("\n"))
    }

    #[test]
    fn tree_lines_are_read_per_package_with_markers_and_headings_dropped() {
        let read = tree(&[
            "0@app v0.1.0 (/ws/app)|",
            "[dev-dependencies]",
            "",
            "1@serde_core v1.0.228|alloc,std",
            "1@serde_core v1.0.228|std (*)",
            "1@serde_derive v1.0.228 (proc-macro)|default",
        ]);
        assert_eq!(
            read.variants["serde_core v1.0.228"],
            BTreeSet::from(["alloc,std".to_owned(), "std".to_owned()])
        );
        // Cargo prints the proc-macro marker inside the package text.
        assert_eq!(
            read.variants["serde_derive v1.0.228 (proc-macro)"],
            BTreeSet::from(["default".to_owned()])
        );
        assert_eq!(read.variants.len(), 3);
        assert_eq!(read.roots.len(), 1);
    }

    #[test]
    fn a_changed_added_or_dropped_feature_set_is_a_difference_but_a_vanished_package_is_not() {
        let full = tree(&["0@a v1|x", "0@b v1|", "0@b v1|y", "0@gone v1|"]);
        assert!(tree(&["0@a v1|x", "0@b v1|", "0@b v1|y"])
            .differences(&full)
            .is_empty());
        assert_eq!(tree(&["0@a v1|x,z"]).differences(&full), vec!["a v1"]);
        // A host variant that disappears is a difference even though the
        // remaining one also exists in the full run.
        assert_eq!(tree(&["0@b v1|"]).differences(&full), vec!["b v1"]);
        assert_eq!(tree(&["0@new v1|"]).differences(&full), vec!["new v1"]);
    }

    #[test]
    fn a_package_compiles_the_same_only_with_the_same_units_below_it() {
        // `p` reaches `d` without features in the default run, and `d` with
        // the feature in the other; `q` reaches both `d` units in both. A
        // repeated subtree (`(*)`) still counts for the package it appears
        // under.
        let default = tree(&[
            "0@p v1 (/ws/p)|",
            "1@d v1|",
            "2@e v1|",
            "0@q v1 (/ws/q)|",
            "1@d v1|f",
            "1@d v1| (*)",
            "0@r v1 (/ws/r)|",
            "1@d v1| (*)",
        ]);
        let all = tree(&[
            "0@p v1 (/ws/p)|",
            "1@d v1|f",
            "2@e v1|",
            "0@q v1 (/ws/q)|",
            "1@d v1|f (*)",
            "1@d v1|",
            "2@e v1|",
            "0@r v1 (/ws/r)|",
            "1@d v1| (*)",
        ]);
        assert!(!default.compiles_same_as(&all, "p"));
        assert!(default.compiles_same_as(&all, "q"));
        assert!(default.compiles_same_as(&all, "r"));
        assert!(!default.compiles_same_as(&all, "missing"));
    }

    /// A fake resolver: leaving out any package of `needed` changes `serde`.
    fn resolver<'a>(
        needed: &'a [&'a str],
        calls: &'a std::cell::Cell<usize>,
    ) -> impl FnMut(&[String]) -> Result<TreeFeatures, ()> + 'a {
        move |skip| {
            calls.set(calls.get() + 1);
            let changed = needed.iter().any(|n| skip.iter().any(|s| s == n));
            Ok(tree(&[if changed {
                "0@serde v1|std"
            } else {
                "0@serde v1|alloc,std"
            }]))
        }
    }

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn settle_keeps_every_candidate_when_nothing_changes() {
        let full = tree(&["0@serde v1|alloc,std"]);
        let calls = std::cell::Cell::new(0);
        let skip = settle(&names(&["b", "a"]), &full, &mut resolver(&[], &calls)).unwrap();
        assert_eq!(skip, names(&["a", "b"]));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn settle_puts_back_only_the_candidate_that_changes_a_remaining_package() {
        let full = tree(&["0@serde v1|alloc,std"]);
        let calls = std::cell::Cell::new(0);
        let skip = settle(
            &names(&["attrs", "cst", "package"]),
            &full,
            &mut resolver(&["attrs"], &calls),
        )
        .unwrap();
        assert_eq!(skip, names(&["cst", "package"]));
    }

    #[test]
    fn settle_puts_back_every_candidate_that_changes_one() {
        let full = tree(&["0@serde v1|alloc,std"]);
        let calls = std::cell::Cell::new(0);
        let skip = settle(
            &names(&["a", "b", "c"]),
            &full,
            &mut resolver(&["a", "c"], &calls),
        )
        .unwrap();
        assert_eq!(skip, names(&["b"]));
    }

    #[test]
    fn settle_runs_the_whole_workspace_when_no_skip_can_be_shown() {
        // Every candidate alone changes the resolve, and each candidate's
        // return fixes it, so all end up back.
        let full = tree(&["0@serde v1|alloc,std"]);
        let calls = std::cell::Cell::new(0);
        let skip = settle(
            &names(&["a", "b"]),
            &full,
            &mut resolver(&["a", "b"], &calls),
        )
        .unwrap();
        assert!(skip.is_empty());
    }

    #[test]
    fn settle_passes_a_failing_resolver_on() {
        let full = tree(&[]);
        let mut failing = |_: &[String]| -> Result<TreeFeatures, &'static str> { Err("no tree") };
        assert_eq!(settle(&names(&["a"]), &full, &mut failing), Err("no tree"));
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-297 typed views over checked package and result data.
//!
//! Inspection only projects values produced by the compiler or an engine. It
//! does not parse rendered text, read package bytes, rerun a producer, or
//! confer package authority on a selector.

use alloc::{string::String, vec::Vec};
use core::fmt;
use qsl_foundation::{
    diagnostic::{LimitExceeded, StageFailure, Staged},
    digest::WireNodeId,
    setting::Setting,
    source::provenance::{OccurrenceKey, SourceRegion, UnresolvedOccurrence},
};
use qsl_package::{CheckedPackage, Emission};
use qsl_semantics::check::{RequirementRecord, SemanticNode};
use quire_exact::{Cancel, NodeKey};
use quire_semantic_value::catalog::{CatalogCode, CatalogCoded};

extern crate alloc;

/// The inspection operation's cumulative materialized-entry limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InspectLimits {
    /// Maximum number of selected collection entries in one view.
    pub max_view_entries: u64,
}

impl Default for InspectLimits {
    fn default() -> Self {
        Self {
            max_view_entries: 10_000_000,
        }
    }
}

impl From<&qsl_replay::CallerLimits> for InspectLimits {
    fn from(limits: &qsl_replay::CallerLimits) -> Self {
        Self {
            max_view_entries: limits.inspect_max_view_entries,
        }
    }
}

/// Why an inspection request cannot produce its selected typed view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InspectCause {
    /// The selected package has not been emitted or does not own this emission.
    PackageEmissionMismatch,
    /// The selected checked package has no node with this key.
    UnknownNode {
        /// The node selector that was not in the checked package.
        selector: String,
        /// The package identity used to scope the lookup.
        package_id: String,
    },
    /// The node exists but the requested role and ordinal are absent.
    UnknownOccurrence {
        /// The occurrence selector that was not in the checked package map.
        selector: String,
        /// The package identity used to scope the lookup.
        package_id: String,
    },
    /// The input and requested view do not have a compatible typed pairing.
    IncompatibleSelection {
        /// The requested view selector.
        selector: String,
    },
    /// No retained state graph was supplied by its producer.
    UnavailableRetainedContent,
    /// The checked node has no source correspondence in the associated map.
    SourceCorrespondence {
        /// The node or occurrence selector that did not resolve.
        selector: String,
        /// The package identity used to scope the lookup.
        package_id: String,
    },
}

impl CatalogCoded for InspectCause {
    fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::PackageEmissionMismatch => {
                CatalogCode::new("inspection_refused", "package-emission-mismatch")
            }
            Self::UnknownNode { .. } => CatalogCode::new("inspection_refused", "unknown-node"),
            Self::UnknownOccurrence { .. } => {
                CatalogCode::new("inspection_refused", "unknown-occurrence")
            }
            Self::SourceCorrespondence { .. } => {
                CatalogCode::new("inspection_refused", "source-correspondence-unresolved")
            }
            Self::IncompatibleSelection { .. } => {
                CatalogCode::new("inspection_refused", "incompatible-selection")
            }
            Self::UnavailableRetainedContent => {
                CatalogCode::new("inspection_refused", "unavailable-retained-content")
            }
        }
    }

    fn catalog_fields(&self) -> Option<alloc::collections::BTreeMap<&'static str, String>> {
        let mut fields = alloc::collections::BTreeMap::new();
        match self {
            Self::PackageEmissionMismatch | Self::UnavailableRetainedContent => {}
            Self::IncompatibleSelection { selector } => {
                fields.insert("selector", selector.clone());
            }
            Self::UnknownNode {
                selector,
                package_id,
            }
            | Self::UnknownOccurrence {
                selector,
                package_id,
            }
            | Self::SourceCorrespondence {
                selector,
                package_id,
            } => {
                fields.insert("selector", selector.clone());
                fields.insert("package_id", package_id.clone());
            }
        }
        Some(fields)
    }
}

impl fmt::Display for InspectCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PackageEmissionMismatch => {
                f.write_str("emission does not belong to the checked package")
            }
            Self::IncompatibleSelection { selector } => {
                write!(f, "input is incompatible with the {selector} view")
            }
            Self::UnavailableRetainedContent => f.write_str("the producer retained no state graph"),
            Self::UnknownNode {
                selector,
                package_id,
            } => write!(f, "unknown node {selector} in package {package_id}"),
            Self::UnknownOccurrence {
                selector,
                package_id,
            } => write!(f, "unknown occurrence {selector} in package {package_id}"),
            Self::SourceCorrespondence {
                selector,
                package_id,
            } => write!(
                f,
                "source correspondence for {selector} is unavailable in package {package_id}"
            ),
        }
    }
}

/// A package-scoped node selector. Printed names are deliberately absent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeSelector {
    /// Select the checked node identity.
    Node(NodeKey),
    /// Select one mapped occurrence by its checked identity, role and ordinal.
    Occurrence(OccurrenceKey),
}

/// A source-map occurrence and all regions in their stored order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OccurrenceView<'a> {
    /// The occurrence identity.
    pub key: OccurrenceKey,
    /// The occurrence's regions, preserving order and multiplicity.
    pub regions: &'a [SourceRegion],
}

/// A node projected from a checked package.
#[derive(Clone, Debug)]
pub struct NodeView<'a> {
    /// The checked semantic node.
    pub node: &'a SemanticNode,
    /// All mapped source occurrences, separately keyed.
    pub occurrences: Vec<OccurrenceView<'a>>,
    /// Requirement records attached to the node's mapped occurrences.
    pub requirements: Vec<(&'a OccurrenceKey, &'a RequirementRecord)>,
}

/// A typed view of the checked package and its own emitted metadata.
#[derive(Clone, Debug)]
pub struct PackageView<'a> {
    /// The compiler-computed package identity.
    pub package_id: qsl_semantics::library::PackageId,
    /// Dependency closure, in the package's canonical identity order.
    pub dependencies: Vec<(qsl_semantics::library::PackageId, &'a CheckedPackage)>,
    /// Selected domain models, in checked declaration order.
    pub model_selections: &'a [qsl_semantics::model::domain_package::DomainPackageRef],
    /// Exported declaration nodes from the checked semantic graph.
    pub exports: Vec<&'a SemanticNode>,
    /// Per-item requirement records retained by S3.
    pub requirements: Vec<(&'a OccurrenceKey, &'a RequirementRecord)>,
    /// The exact typed v2 capability report retained by the emitter.
    pub capability_report: &'a [quire_contract_model::CheckedCapability],
}

/// A successful inspection result. The variants retain their owning types.
#[derive(Clone, Debug)]
pub enum View<'a, S = (), T = (), F = ()> {
    /// A checked package view.
    Package(PackageView<'a>),
    /// A checked node view.
    Node(NodeView<'a>),
    /// A terminal outcome.
    Outcome(OutcomeView<'a>),
    /// A scalar counterexample.
    Counterexample(CounterexampleView<'a>),
    /// A finite trace or lasso.
    Trace(TraceView<'a>),
    /// A retained state graph.
    StateGraph(StateGraphView<'a, S, T, F>),
    /// A model-check state graph and its completion status.
    ModelCheckStateGraph(ModelCheckStateGraphView<'a>),
}

mod result_views;
pub use result_views::{
    inspect_counterexample, inspect_model_check_graph, inspect_outcome, inspect_state_graph,
    inspect_trace, AssignmentView, CounterexampleView, ModelCheckStateGraphView, OutcomeView,
    StateGraphView, TracePositionView, TraceView,
};

/// Inspect a checked package, using the package's typed emission metadata.
pub fn inspect_package<'a>(
    package: &'a CheckedPackage,
    emission: &'a Emission,
    limits: InspectLimits,
    cancel: &Cancel,
) -> Result<Staged<View<'a>>, StageFailure<InspectCause>> {
    check_cancel(cancel)?;
    let Some(package_id) = package.emitted_package_id() else {
        return Err(StageFailure::Refused(InspectCause::PackageEmissionMismatch));
    };
    if package_id != emission.package().package_id() {
        return Err(StageFailure::Refused(InspectCause::PackageEmissionMismatch));
    }
    let mut budget = EntryBudget::new(limits);
    let mut dependencies = Vec::new();
    for (identity, dependency) in package.dependencies() {
        budget.charge(cancel)?;
        dependencies.push((*identity, dependency.as_ref()));
    }
    let graph = package.graph();
    for _ in graph.model_selections() {
        budget.charge(cancel)?;
    }
    let mut exports = Vec::new();
    for node in graph.semantic_graph().nodes() {
        if node.declaration().is_some() {
            budget.charge(cancel)?;
            exports.push(node);
        }
    }
    let mut requirements = Vec::new();
    for (key, record) in graph.requirements() {
        budget.charge(cancel)?;
        requirements.push((key, record));
    }
    for _ in emission.capability_report() {
        budget.charge(cancel)?;
    }
    let view = PackageView {
        package_id,
        dependencies,
        model_selections: graph.model_selections(),
        exports,
        requirements,
        capability_report: emission.capability_report(),
    };
    Ok(Staged::new(View::Package(view)))
}

/// Inspect one checked node or source occurrence without reparsing source.
pub fn inspect_node<'a>(
    package: &'a CheckedPackage,
    emission: &'a Emission,
    selector: &NodeSelector,
    limits: InspectLimits,
    cancel: &Cancel,
) -> Result<Staged<View<'a>>, StageFailure<InspectCause>> {
    check_cancel(cancel)?;
    let Some(package_id_value) = package.emitted_package_id() else {
        return Err(StageFailure::Refused(InspectCause::PackageEmissionMismatch));
    };
    let package_id = package_id_value.hex();
    if package_id_value != emission.package().package_id() {
        return Err(StageFailure::Refused(InspectCause::PackageEmissionMismatch));
    }
    let source_map = emission.source_map();
    let graph = package.graph();
    let (node_key, selected_occurrence) = match selector {
        NodeSelector::Node(key) => (*key, None),
        NodeSelector::Occurrence(key) => {
            let node = graph.semantic_graph().resolve_wire(key.node());
            let Some(node) = node else {
                return Err(StageFailure::Refused(InspectCause::UnknownNode {
                    selector: format!("{}", key.node()),
                    package_id,
                }));
            };
            (node, Some(key))
        }
    };
    let Some(node) = graph.semantic_graph().node(node_key) else {
        return Err(StageFailure::Refused(InspectCause::UnknownNode {
            selector: node_key.to_string(),
            package_id,
        }));
    };
    let wire_id = WireNodeId::from_digest(*node_key.as_bytes());
    if emission
        .omitted()
        .iter()
        .any(|omitted| omitted.node.digest.as_ref() == node_key.to_string())
    {
        return Err(StageFailure::Refused(InspectCause::SourceCorrespondence {
            selector: node_key.to_string(),
            package_id,
        }));
    }
    let mut occurrences = Vec::new();
    match selected_occurrence {
        Some(key) => match source_map.regions(key) {
            Some(regions) => {
                let mut budget = EntryBudget::new(limits);
                budget.charge(cancel)?;
                for _ in regions {
                    budget.charge(cancel)?;
                }
                occurrences.push(OccurrenceView {
                    key: key.clone(),
                    regions,
                });
            }
            None => {
                let cause = if source_map.occurrences(wire_id).next().is_some() {
                    InspectCause::UnknownOccurrence {
                        selector: format!("{key:?}"),
                        package_id,
                    }
                } else {
                    InspectCause::SourceCorrespondence {
                        selector: format!("{key:?}"),
                        package_id,
                    }
                };
                return Err(StageFailure::Refused(cause));
            }
        },
        None => {
            let mut budget = EntryBudget::new(limits);
            let mut found = false;
            for (key, regions) in source_map.occurrences(wire_id) {
                found = true;
                budget.charge(cancel)?;
                for _ in regions {
                    budget.charge(cancel)?;
                }
                occurrences.push(OccurrenceView {
                    key: key.clone(),
                    regions,
                });
            }
            if !found {
                return Err(StageFailure::Refused(InspectCause::SourceCorrespondence {
                    selector: node_key.to_string(),
                    package_id,
                }));
            }
        }
    }
    let selected = occurrences
        .iter()
        .map(|occurrence| occurrence.key.clone())
        .collect::<alloc::collections::BTreeSet<_>>();
    let mut requirements = Vec::new();
    for (key, record) in graph.requirements() {
        if selected.contains(key) {
            requirements.push((key, record));
        }
    }
    Ok(Staged::new(View::Node(NodeView {
        node,
        occurrences,
        requirements,
    })))
}

pub(crate) fn check_cancel<C>(cancel: &Cancel) -> Result<(), StageFailure<C>> {
    if let Some(cause) = cancel.cause() {
        Err(StageFailure::Cancelled(cause))
    } else {
        Ok(())
    }
}

pub(crate) struct EntryBudget {
    bound: u64,
    used: u64,
}

impl EntryBudget {
    pub(crate) fn new(limits: InspectLimits) -> Self {
        Self {
            bound: limits.max_view_entries,
            used: 0,
        }
    }

    pub(crate) fn charge<C>(&mut self, cancel: &Cancel) -> Result<(), StageFailure<C>> {
        if let Some(cause) = cancel.cause() {
            return Err(StageFailure::Cancelled(cause));
        }
        let next = u128::from(self.used) + 1;
        if next > u128::from(self.bound) {
            return Err(StageFailure::Limit(LimitExceeded::new(
                Setting::InspectMaxViewEntries,
                self.bound,
                next,
            )));
        }
        self.used += 1;
        Ok(())
    }
}

/// Convert a source-map lookup failure without changing its identity.
pub fn unresolved_cause(
    error: UnresolvedOccurrence,
    selector: &str,
    package_id: &str,
) -> InspectCause {
    match error {
        UnresolvedOccurrence::UnknownNode(_) => InspectCause::UnknownNode {
            selector: selector.into(),
            package_id: package_id.into(),
        },
        UnresolvedOccurrence::UnknownOccurrence(_) => InspectCause::UnknownOccurrence {
            selector: selector.into(),
            package_id: package_id.into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        inspect_node, inspect_outcome, inspect_package, EntryBudget, InspectCause, InspectLimits,
        NodeSelector, View,
    };
    use ix_trace_rs::trace;
    use qsl_foundation::{diagnostic::StageFailure, digest::WireNodeId, setting::Setting};
    use qsl_semantics::check::PackageDeclarations;
    use quire_exact::{Cancel, CancelCause, Location, NodeKey, Origin, Role};
    use quire_semantic_value::checking::CheckingLimits;

    fn emitted_spine_package() -> (qsl_package::CheckedPackage, qsl_package::Emission) {
        const FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/spine-compile.native");
        let parsed = qsl_cst::parse(
            qsl_foundation::SourceIdentity::new("agent-ix", "test:inspect", "fixture", "fixture:1"),
            "program.native",
            FIXTURE,
            qsl_cst::Limits::default(),
        )
        .expect("S1 admits the checked-package fixture");
        assert_eq!(parsed.diagnostics(), []);
        let raw = parsed.source().reference().clone();
        let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the checked-package fixture");
        let graph = PackageDeclarations::assemble(
            raw,
            unit,
            qsl_semantics::model::intake::SelectedModels::default(),
            Vec::new(),
        )
        .expect("the fixture assembles")
        .check(CheckingLimits::default())
        .expect("the fixture checks");
        let package = qsl_package::CheckedPackage::link(graph);
        let emission = qsl_package::emit_checked(&package).expect("the fixture emits");
        assert!(emission.omitted().is_empty());
        (package, emission)
    }

    #[trace("TC-782", "FR-297-AC-1")]
    #[test]
    fn package_view_preserves_package_metadata() {
        let (package, emission) = emitted_spine_package();
        let result = inspect_package(
            &package,
            &emission,
            InspectLimits::default(),
            &Cancel::new(),
        )
        .expect("a checked package with its own emission is inspectable");
        let View::Package(view) = result.value() else {
            panic!("package request gives package view");
        };
        assert_eq!(view.package_id, package.emitted_package_id().unwrap());
        assert_eq!(view.dependencies.len(), package.dependencies().len());
        assert_eq!(view.model_selections, package.graph().model_selections());
        assert_eq!(
            view.exports.len(),
            package
                .graph()
                .semantic_graph()
                .nodes()
                .filter(|node| node.declaration().is_some())
                .count()
        );
        assert_eq!(
            view.requirements.len(),
            package.graph().requirements().len()
        );
        assert_eq!(view.capability_report, emission.capability_report());
        let selected_entries = (view.dependencies.len()
            + view.model_selections.len()
            + view.exports.len()
            + view.requirements.len()
            + view.capability_report.len()) as u64;
        assert!(selected_entries > 0);
        assert!(inspect_package(
            &package,
            &emission,
            InspectLimits {
                max_view_entries: selected_entries,
            },
            &Cancel::new(),
        )
        .is_ok());
        assert!(matches!(
            inspect_package(
                &package,
                &emission,
                InspectLimits {
                    max_view_entries: selected_entries - 1,
                },
                &Cancel::new(),
            ),
            Err(StageFailure::Limit(limit))
                if limit.setting() == Setting::InspectMaxViewEntries
                    && limit.configured_bound() == selected_entries - 1
                    && limit.actual() == u128::from(selected_entries)
        ));
    }

    #[trace("TC-782", "FR-297-AC-2")]
    #[test]
    fn node_view_uses_checked_identity_and_unknown_node_refusal_is_typed() {
        let (package, emission) = emitted_spine_package();
        let node = package
            .graph()
            .semantic_graph()
            .nodes()
            .find(|node| {
                node.declaration()
                    .is_some_and(|name| name.last().is_some_and(|part| part.as_str() == "seven"))
            })
            .expect("fixture declares seven");
        let node_key = node.key();
        let result = inspect_node(
            &package,
            &emission,
            &NodeSelector::Node(node_key),
            InspectLimits::default(),
            &Cancel::new(),
        )
        .expect("a checked node with emitted correspondence is inspectable");
        let View::Node(view) = result.value() else {
            panic!("node request gives node view");
        };
        assert_eq!(view.node.key(), node_key);
        assert!(!view.occurrences.is_empty());
        assert!(view
            .occurrences
            .iter()
            .all(|occurrence| !occurrence.regions.is_empty()));
        assert!(view.occurrences.iter().all(
            |occurrence| occurrence.key.node() == WireNodeId::from_digest(*node_key.as_bytes())
        ));
        let selected_entries = view
            .occurrences
            .iter()
            .map(|occurrence| 1 + occurrence.regions.len())
            .sum::<usize>() as u64;
        assert!(inspect_node(
            &package,
            &emission,
            &NodeSelector::Node(node_key),
            InspectLimits {
                max_view_entries: selected_entries,
            },
            &Cancel::new(),
        )
        .is_ok());
        assert!(matches!(
            inspect_node(
                &package,
                &emission,
                &NodeSelector::Node(node_key),
                InspectLimits {
                    max_view_entries: selected_entries - 1,
                },
                &Cancel::new(),
            ),
            Err(StageFailure::Limit(limit))
                if limit.setting() == Setting::InspectMaxViewEntries
                    && limit.actual() == u128::from(selected_entries)
        ));

        let absent = NodeKey::from_digest([0xFF; 32]);
        let refusal = inspect_node(
            &package,
            &emission,
            &NodeSelector::Node(absent),
            InspectLimits::default(),
            &Cancel::new(),
        )
        .expect_err("unknown checked identity refuses");
        assert!(
            matches!(refusal, StageFailure::Refused(InspectCause::UnknownNode { selector, package_id })
            if selector == absent.to_string() && package_id == package.emitted_package_id().unwrap().hex())
        );

        let absent_occurrence = qsl_foundation::source::provenance::OccurrenceKey::new(
            WireNodeId::from_digest(*node_key.as_bytes()),
            Origin::new(Role::new("missing-role"), 0),
        );
        let refusal = inspect_node(
            &package,
            &emission,
            &NodeSelector::Occurrence(absent_occurrence),
            InspectLimits::default(),
            &Cancel::new(),
        )
        .expect_err("absent occurrence on a known node refuses separately");
        assert!(matches!(
            refusal,
            StageFailure::Refused(InspectCause::UnknownOccurrence { .. })
        ));
    }

    #[trace("TC-782", "FR-297-AC-6")]
    #[test]
    fn outcome_occurrence_locus_resolves_through_its_emission_map() {
        let (package, emission) = emitted_spine_package();
        let node = package
            .graph()
            .semantic_graph()
            .nodes()
            .find(|node| node.declaration().is_some())
            .expect("fixture has a declared node");
        let key = emission
            .source_map()
            .occurrences(WireNodeId::from_digest(*node.key().as_bytes()))
            .next()
            .expect("the declaration has a mapped occurrence")
            .0
            .clone();
        let location = Location::new(node.key(), key.origin().clone());
        let locus = qsl_foundation::diagnostic::Locus::Occurrence(location);
        let diagnostic = qsl_replay::OutcomeDiagnostic::new(
            None,
            "inspection_fixture",
            Some(qsl_replay::OutcomeLocus::from(&locus)),
            "fixture",
        );
        let document = qsl_replay::OutcomeDocument::new(
            qsl_replay::Operation::Analyze,
            None,
            quire_semantic_value::catalog::Category::Success,
        )
        .with_diagnostics(vec![diagnostic]);
        let package_id = package.emitted_package_id().expect("fixture emitted");
        let result = inspect_outcome(
            &document,
            &[Some(key.clone())],
            Some(package_id),
            Some(&emission),
            InspectLimits::default(),
            &Cancel::new(),
        )
        .expect("the occurrence resolves through the associated emission");
        let View::Outcome(view) = result.value() else {
            panic!("outcome request gives outcome view");
        };
        assert_eq!(view.loci[0].0, document.diagnostics()[0].locus().cloned());
        assert_eq!(view.loci[0].1, emission.source_map().regions(&key).unwrap());

        assert!(matches!(
            inspect_outcome(
                &document,
                &[Some(
                    qsl_foundation::source::provenance::OccurrenceKey::new(
                        WireNodeId::from_digest([0xFF; 32]),
                        key.origin().clone(),
                    )
                )],
                Some(package_id),
                Some(&emission),
                InspectLimits::default(),
                &Cancel::new(),
            ),
            Err(StageFailure::Refused(
                InspectCause::IncompatibleSelection { .. }
            ))
        ));
    }

    #[trace("TC-1168", "FR-297-AC-10")]
    #[test]
    fn entry_limit_accepts_exact_and_refuses_one_over_with_inspection_setting() {
        let cancel = Cancel::new();
        let mut exact = EntryBudget::new(InspectLimits {
            max_view_entries: 2,
        });
        exact.charge::<()>(&cancel).unwrap();
        exact.charge::<()>(&cancel).unwrap();
        assert!(
            matches!(exact.charge::<()>(&cancel), Err(StageFailure::Limit(limit))
            if limit.setting() == Setting::InspectMaxViewEntries
                && limit.configured_bound() == 2
                && limit.actual() == 3)
        );
    }

    #[trace("TC-1168", "FR-297-AC-10")]
    #[test]
    fn zero_limit_accepts_empty_and_refuses_first_entry() {
        let cancel = Cancel::new();
        let configured = qsl_replay::CallerLimits::from_operands(["inspect.max_view_entries=0"])
            .expect("the settings operation accepts the inspection setting");
        assert_eq!(InspectLimits::from(&configured).max_view_entries, 0);
        let mut zero = EntryBudget::new(InspectLimits {
            max_view_entries: 0,
        });
        assert!(
            matches!(zero.charge::<()>(&cancel), Err(StageFailure::Limit(limit))
            if limit.configured_bound() == 0 && limit.actual() == 1)
        );
    }

    #[trace("TC-1168", "FR-297-AC-10")]
    #[test]
    fn precancelled_handle_stops_entry_charging() {
        let cancel = Cancel::new();
        cancel.cancel(CancelCause::Requested);
        let mut budget = EntryBudget::new(InspectLimits::default());
        assert!(matches!(
            budget.charge::<()>(&cancel),
            Err(StageFailure::Cancelled(_))
        ));
    }
}

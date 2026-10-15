// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-767: the QSL owner report for Contract IR's source-obligation input
//! (IR FR-052).
//!
//! The report is built from QSL's own traversal: every checked requirement
//! record of a [`CheckedGraph`] and every pending obligation of a
//! [`TypeReport`], each under its native identity. The two sets stay separate
//! identity kinds. This module claims no census and no completeness: it
//! reports the rows and, when QSL's own traversal stopped short, the typed
//! cause. `stop_reason == None` means only that no stop cause was reported; it
//! is not a completeness claim. A positive owner-native completion waits on the
//! IR typed completion boundary (IR-794) and is emitted nowhere here.
//!
//! QSL-774: the rows are built from one [`CheckedUnit`], so the graph, the type
//! report and the selected source cannot be passed as three loose inputs.
//!
//! INTERIM, NOT A DIGEST-BOUND MATCH: [`CheckedUnit::new`] compares only the
//! source labels (authority and identity) of the graph, the type report's unit
//! and the selected formal source. The graph and the type report come from two
//! different parse stages over different bytes, so their digests cannot agree.
//! This label match expires when the unified single-source producer (QSL-781)
//! lands; that producer replaces [`CheckedUnit::new`].

use qsl_foundation::source::provenance::RawSourceRef;
use qsl_semantics::check::CheckedGraph;
use quire_contract_model::source_obligation::{
    SourceDeclarationHandle, SourceExpressionHandle, SourceInventoryEnvelope,
    SourceInventoryIdentity, SourceInventoryIncompleteReason, SourceInventoryOwner,
    SourceInventoryRow, SourceInventoryScope, SourceObligationSite, SourcePendingObligationId,
    SourcePendingObligationKind, SourceRequirementOccurrenceId, SourceTypeExhaustion,
    SourceTypeWorkDimension, SourceUnitHandle,
};
use quire_contract_model::SourceIdentity;

use super::composed::work::Dimension;
use super::composed::{Exhaustion, Obligation, ObligationKind, Site, TypeDisposition, TypeReport};
use crate::formal_source::FormalSource;
use crate::linking::composed::DeclarationId;

fn declaration(id: DeclarationId) -> SourceDeclarationHandle {
    SourceDeclarationHandle(id.index())
}

fn site(site: &Site) -> SourceObligationSite {
    SourceObligationSite {
        declaration: declaration(site.declaration),
        unit: SourceUnitHandle(site.unit.index()),
        expression: site.expression.map(|id| SourceExpressionHandle(id.0)),
        start: site.span.start,
        end: site.span.end,
    }
}

fn dimension(dimension: Dimension) -> SourceTypeWorkDimension {
    match dimension {
        Dimension::Bytes => SourceTypeWorkDimension::Bytes,
        Dimension::Declarations => SourceTypeWorkDimension::Declarations,
        Dimension::Expressions => SourceTypeWorkDimension::Expressions,
        Dimension::Constraints => SourceTypeWorkDimension::Constraints,
        Dimension::Edges => SourceTypeWorkDimension::Edges,
        Dimension::Normalization => SourceTypeWorkDimension::Normalization,
        Dimension::Records => SourceTypeWorkDimension::Records,
        Dimension::Depth => SourceTypeWorkDimension::Depth,
    }
}

fn exhaustion(exhaustion: &Exhaustion) -> SourceTypeExhaustion {
    SourceTypeExhaustion {
        dimension: dimension(exhaustion.dimension),
        limit: exhaustion.limit,
        prior: exhaustion.prior,
        requested: exhaustion.requested,
        site: site(&exhaustion.site),
    }
}

fn kind(kind: ObligationKind) -> SourcePendingObligationKind {
    match kind {
        ObligationKind::Presence => SourcePendingObligationKind::Presence,
        ObligationKind::ArithmeticRange => SourcePendingObligationKind::ArithmeticRange,
        ObligationKind::Nonzero => SourcePendingObligationKind::Nonzero,
        ObligationKind::SumProjection => SourcePendingObligationKind::SumProjection,
        ObligationKind::SumPrefixes => SourcePendingObligationKind::SumPrefixes,
        ObligationKind::PredicateTotal { target } => SourcePendingObligationKind::PredicateTotal {
            target: declaration(target),
        },
        ObligationKind::GraphClosure => SourcePendingObligationKind::GraphClosure,
        ObligationKind::InvocationContext => SourcePendingObligationKind::InvocationContext,
        ObligationKind::CaptureInput => SourcePendingObligationKind::CaptureInput,
    }
}

fn pending(obligation: &Obligation) -> SourcePendingObligationId {
    SourcePendingObligationId {
        site: site(&obligation.site),
        kind: kind(obligation.kind),
    }
}

/// The stop cause of QSL's own traversal of `types`, if it stopped short.
///
/// Work exhaustion is the root cause and keeps its native fields; otherwise
/// the first declaration of the binding namespace whose type admission is
/// [`TypeDisposition::Unfinished`] is named.
fn stop_reason(types: &TypeReport<'_, '_>) -> Option<SourceInventoryIncompleteReason> {
    if let Some(stopped) = types.exhaustion() {
        return Some(SourceInventoryIncompleteReason::TypeWorkExhausted(
            exhaustion(stopped),
        ));
    }
    types
        .binding()
        .namespace()
        .declaration_ids()
        .find(|id| types.disposition(*id) == Some(TypeDisposition::Unfinished))
        .map(
            |id| SourceInventoryIncompleteReason::TypeAdmissionUnfinished {
                declaration: declaration(id),
            },
        )
}

/// The authority and identity labels of one source. The digest is not part of
/// a label: this is the interim, label-only match of [`CheckedUnit`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitLabel {
    /// The source's authority.
    pub authority: String,
    /// The source's identity within that authority.
    pub identity: String,
}

impl From<&RawSourceRef> for UnitLabel {
    fn from(source: &RawSourceRef) -> Self {
        Self {
            authority: source.authority().into(),
            identity: source.identity().into(),
        }
    }
}

/// Why the inputs are not one checked unit under the interim label match.
/// This is never a digest comparison; it expires with QSL-781.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum UnitLabelMismatch {
    /// The type report does not cover exactly one source unit.
    #[error("the type report covers {count} units, not one")]
    NotOneUnit {
        /// Units in the report's namespace.
        count: usize,
    },
    /// The selected source's labels differ from the type report's unit.
    #[error("the type report's unit is not the selected source")]
    TypeReport {
        /// Labels of the selected formal source.
        selected: UnitLabel,
        /// Labels of the type report's only unit.
        unit: UnitLabel,
    },
    /// The selected source's labels differ from the checked graph's source.
    #[error("the checked graph is not of the selected source")]
    Graph {
        /// Labels of the selected formal source.
        selected: UnitLabel,
        /// Labels of the checked graph's source.
        graph: UnitLabel,
    },
}

/// One checked unit: the selected source, its checked graph and its type
/// report, borrowed together. Built only through [`Self::new`].
#[derive(Debug)]
pub struct CheckedUnit<'g, 'r, 'a> {
    identity: SourceIdentity,
    graph: &'g CheckedGraph,
    types: &'g TypeReport<'r, 'a>,
}

impl<'g, 'r, 'a> CheckedUnit<'g, 'r, 'a> {
    /// Bind `graph` and `types` to the selected `source`.
    ///
    /// INTERIM: refuses unless the type report has one unit and the labels
    /// (authority, identity; not the digest) of that unit and of the graph both
    /// equal the selected source's. Expires when QSL-781 lands.
    pub fn new(
        source: &FormalSource,
        graph: &'g CheckedGraph,
        types: &'g TypeReport<'r, 'a>,
    ) -> Result<Self, UnitLabelMismatch> {
        let selected = UnitLabel::from(source.source().reference());
        let [unit] = types.binding().namespace().units() else {
            return Err(UnitLabelMismatch::NotOneUnit {
                count: types.binding().namespace().units().len(),
            });
        };
        let unit = UnitLabel::from(unit.source().reference());
        if unit != selected {
            return Err(UnitLabelMismatch::TypeReport { selected, unit });
        }
        let graph_label = UnitLabel::from(graph.source());
        if graph_label != selected {
            return Err(UnitLabelMismatch::Graph {
                selected,
                graph: graph_label,
            });
        }
        Ok(Self {
            identity: source.identity().clone(),
            graph,
            types,
        })
    }

    /// The selected source's formal identity.
    pub fn identity(&self) -> &SourceIdentity {
        &self.identity
    }
}

/// Report QSL's inventory rows for one checked unit.
///
/// Rows are, in order, one `RequirementRecord` per entry of
/// [`CheckedGraph::requirements`] and one `PendingObligation` per retained
/// obligation of the type report. The stop reason is `None` only when QSL's
/// traversal neither exhausted its work budget nor left a declaration
/// unfinished; that is not a completeness claim (see the module docs).
pub fn qsl_inventory(unit: &CheckedUnit<'_, '_, '_>) -> SourceInventoryEnvelope {
    let (source, graph, types) = (&unit.identity, unit.graph, unit.types);
    let requirements = graph.requirements().keys().map(|key| {
        SourceInventoryIdentity::RequirementRecord(SourceRequirementOccurrenceId {
            node: *key.node().as_bytes(),
            role: key.origin().role().as_str().into(),
            ordinal: key.origin().ordinal(),
        })
    });
    let obligations = types
        .declarations()
        .iter()
        .flat_map(|typed| typed.obligations())
        .map(|obligation| SourceInventoryIdentity::PendingObligation(pending(obligation)));
    let items = requirements
        .chain(obligations)
        .map(|identity| SourceInventoryRow::new(source.clone(), identity))
        .collect();
    SourceInventoryEnvelope::new(
        SourceInventoryOwner::Qsl,
        source.clone(),
        SourceInventoryScope::QslCheckedRequirementsAndPendingObligations,
        items,
        stop_reason(types),
    )
}

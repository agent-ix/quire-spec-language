// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-767: the QSL owner report for Contract IR's source-obligation input
//! (IR FR-052).
//!
//! The report is built from QSL's own traversal: every checked requirement
//! record of a [`CheckedGraph`] and every pending obligation of a
//! [`TypeReport`], each under its native identity. The two sets stay separate
//! identity kinds. This module claims no census and no completeness: it
//! reports the rows and, when QSL's own traversal stopped short, the typed
//! cause.

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

/// Report QSL's inventory rows for the selected `source`.
///
/// Rows are, in order, one `RequirementRecord` per entry of
/// [`CheckedGraph::requirements`] and one `PendingObligation` per retained
/// obligation of `types`. The stop reason is `None` only when QSL's traversal
/// neither exhausted its work budget nor left a declaration unfinished.
pub fn qsl_inventory(
    source: &SourceIdentity,
    graph: &CheckedGraph,
    types: &TypeReport<'_, '_>,
) -> SourceInventoryEnvelope {
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

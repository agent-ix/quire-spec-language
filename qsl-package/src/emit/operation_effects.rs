// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-496: the optional FR-322 `operation_effects` member, one row per
//! operation an emitted `object_type`, `process` or `state_machine` node
//! declares, carrying the operation's frame lift.

use std::collections::HashMap;

use qsl_semantics::check::{CheckedGraph, FrameDeletesScoped, FrameModifies, FrameScope, NodeRef};
use quire_contract_model::{
    CheckedFieldScope, CheckedFrameModification, CheckedNodeId, CheckedOperationEffect,
    CheckedOperationEffectOrigin, CheckedOperationEffectProvenance, CheckedOperationEffectRow,
    CheckedScopedDeletion,
};
use quire_exact::NodeKey;

/// The `operation_effects` rows of `graph`, sorted by `(document, declaring,
/// operation)`, with every effect entry in FR-340 order over the final node
/// identities in `emitted`.
///
/// `None` omits the member: the package has no such operation, or an effect
/// names a declaration the package does not carry, so the effect is
/// unavailable and no partial member may be written. A row whose declaring
/// node is not in the package is dropped, because the reader joins rows to
/// the nodes the package holds.
pub(super) fn operation_effect_rows(
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Option<Vec<CheckedOperationEffectRow>> {
    let resolve = |node: &NodeRef| node.admitted().and_then(|key| emitted.get(key)).cloned();
    let mut rows = Vec::new();
    for record in graph.operation_effects() {
        let Some(declaring) = emitted.get(record.declaring()).cloned() else {
            continue;
        };
        let frame = record.effect();
        let mut modifies = frame
            .modifies
            .iter()
            .map(|entry| match entry {
                FrameModifies::Field(field) => Some(CheckedFrameModification::Field {
                    declaration: resolve(&field.declaration())?,
                    name: field.name().into(),
                    scope: match field.scope() {
                        FrameScope::EveryObject => CheckedFieldScope::EveryObject,
                        FrameScope::Receiver => CheckedFieldScope::Receiver,
                    },
                }),
                FrameModifies::Relationship(target) => {
                    Some(CheckedFrameModification::Relationship {
                        declaration: resolve(target)?,
                    })
                }
            })
            .collect::<Option<Vec<_>>>()?;
        modifies.sort_by(|left, right| modification_key(left).cmp(&modification_key(right)));
        let mut creates = frame
            .creates
            .iter()
            .map(&resolve)
            .collect::<Option<Vec<_>>>()?;
        creates.sort();
        let mut deletes = frame
            .deletes
            .iter()
            .map(&resolve)
            .collect::<Option<Vec<_>>>()?;
        deletes.sort();
        let mut scoped = frame.deletes_scoped.clone();
        scoped.sort();
        let deletes_scoped = scoped
            .into_iter()
            .map(|entry| match entry {
                FrameDeletesScoped::Receiver => CheckedScopedDeletion::Receiver,
                FrameDeletesScoped::Parameter(name) => {
                    CheckedScopedDeletion::Parameter { name: name.into() }
                }
            })
            .collect();
        rows.push(CheckedOperationEffectRow {
            document: record.document().into(),
            declaring,
            operation: record.operation().into(),
            effect: CheckedOperationEffect {
                modifies,
                creates,
                deletes,
                deletes_scoped,
            },
            provenance: CheckedOperationEffectProvenance {
                origin: CheckedOperationEffectOrigin::ModelDocumentOperationFrame,
                member: record.member().into(),
            },
        });
    }
    rows.sort_by(|left, right| {
        (&left.document, &left.declaring, &left.operation).cmp(&(
            &right.document,
            &right.declaring,
            &right.operation,
        ))
    });
    (!rows.is_empty()).then_some(rows)
}

/// FR-340's order of `modifies` entries: declaring node, field name (a
/// relationship's is empty), then `every-object` before `receiver`.
fn modification_key(entry: &CheckedFrameModification) -> (&CheckedNodeId, &str, u8) {
    match entry {
        CheckedFrameModification::Relationship { declaration } => (declaration, "", 0),
        CheckedFrameModification::Field {
            declaration,
            name,
            scope,
        } => (
            declaration,
            name,
            match scope {
                CheckedFieldScope::EveryObject => 0,
                CheckedFieldScope::Receiver => 1,
            },
        ),
    }
}

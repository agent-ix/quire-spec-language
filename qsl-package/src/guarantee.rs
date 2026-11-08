// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-490 "Checked-package node": the body of a
//! `correspondence`/`implementation_guarantee` node, as the v2 emitter writes
//! it. IR's reader is the authority on reading it.

use qsl_semantics::check::{
    EnvironmentEntry, ImplementationGuaranteeTerm, MeasureEntry, SuppliesEntry,
};
use quire_exact::NodeKey;
use serde_json::{json, Value};

/// The `semantic_form` and body `term` of a guarantee node.
const FORM: &str = "implementation_guarantee";

/// Why a guarantee node cannot be written.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NodeWriteRefusal {
    /// A node reference in the body is a lowering placeholder.
    #[error("the guarantee node names an unresolved lowering placeholder")]
    UnresolvedPlaceholder,
}

/// How a body writes a node id.
pub(crate) type IdWriter<'a> = &'a dyn Fn(&NodeKey) -> Result<Value, NodeWriteRefusal>;

pub(crate) fn ref_value(
    id: IdWriter<'_>,
    reference: &qsl_semantics::check::NodeRef,
) -> Result<Value, NodeWriteRefusal> {
    id(reference
        .admitted()
        .ok_or(NodeWriteRefusal::UnresolvedPlaceholder)?)
}

/// One `environment` entry as its node member (QSpec FR-490 "Checked-package
/// node"), shared with the composition node.
pub(crate) fn environment_value(
    id: IdWriter<'_>,
    entry: &EnvironmentEntry,
) -> Result<Value, NodeWriteRefusal> {
    Ok(json!({
        "name": entry.name(),
        "supplier": entry.supplier(),
        "supplies": match entry.supplies() {
            SuppliesEntry::Fairness { kind, granularity, context, operation } => json!({
                "fairness": {
                    "kind": kind,
                    "granularity": granularity,
                    "context": ref_value(id, context)?,
                    "operation": operation,
                },
            }),
            SuppliesEntry::Assumption(node) => json!({"assumption": ref_value(id, node)?}),
        },
    }))
}

/// The body of the guarantee node of `term` (QSpec FR-490 "Checked-package
/// node"), its arrays in the term's canonical order.
pub(crate) fn guarantee_body(
    id: IdWriter<'_>,
    term: &ImplementationGuaranteeTerm,
) -> Result<Value, NodeWriteRefusal> {
    let progress = term
        .progress()
        .iter()
        .map(|entry| {
            Ok(json!({
                "context": ref_value(id, entry.context())?,
                "operation": entry.operation(),
                "measure": match entry.measure() {
                    MeasureEntry::Decreases(segments) => json!({"decreases": segments}),
                    MeasureEntry::Bound(count) => json!({"bound": count}),
                },
            }))
        })
        .collect::<Result<Vec<_>, NodeWriteRefusal>>()?;
    let environment = term
        .environment()
        .iter()
        .map(|entry| environment_value(id, entry))
        .collect::<Result<Vec<_>, NodeWriteRefusal>>()?;
    Ok(json!({
        "term": FORM,
        "clause": ref_value(id, term.clause())?,
        "relation": ref_value(id, term.relation())?,
        "initial": term.initial(),
        "progress": progress,
        "environment": environment,
    }))
}

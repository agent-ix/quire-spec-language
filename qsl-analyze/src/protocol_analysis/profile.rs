// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-575: the first construct outside the `bounded-static` profile.

use qsl_package::protocol_view::{
    control_body, control_operation, protocol_view, queue_capacity, role_is_replicated,
    ControlBody, ControlOperation, QueueCapacity, ViewRefusal,
};
use qsl_replay::protocol_analysis::{UnsupportedCause, UnsupportedSlug};
use quire_contract_model::{CheckedNodeId, CheckedPackageV2, CheckedSemanticNodeV2};

fn node<'p>(
    package: &'p CheckedPackageV2,
    id: &CheckedNodeId,
) -> Result<&'p CheckedSemanticNodeV2, ViewRefusal> {
    package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &node.node_id == id)
        .ok_or_else(|| ViewRefusal::UnknownProtocol { node: id.clone() })
}

fn cause(slug: UnsupportedSlug, node: &CheckedNodeId, detail: Option<&str>) -> UnsupportedCause {
    UnsupportedCause {
        slug,
        node: node.clone(),
        detail: detail.map(ToOwned::to_owned),
    }
}

/// The first construct of the clause `protocol` outside the profile, in the
/// order roles, queues, compensations, then the controls in layout order
/// (FR-575). `None` for a clause that stays inside it.
///
/// # Errors
///
/// [`ViewRefusal`] when a view function refuses.
pub fn unsupported_construct(
    package: &CheckedPackageV2,
    protocol: &CheckedNodeId,
) -> Result<Option<UnsupportedCause>, ViewRefusal> {
    let view = protocol_view(package, protocol)?;
    for role in view.roles() {
        if role_is_replicated(node(package, role)?)? {
            return Ok(Some(cause(
                UnsupportedSlug::DynamicRolePopulation,
                role,
                None,
            )));
        }
    }
    for queue in view.queues() {
        if let QueueCapacity::Symbolic(name) = queue_capacity(node(package, queue)?)? {
            return Ok(Some(cause(
                UnsupportedSlug::SymbolicQueueCapacity,
                queue,
                Some(name),
            )));
        }
    }
    if let Some(template) = view.compensations().first() {
        return Ok(Some(cause(UnsupportedSlug::Compensation, template, None)));
    }
    // The specific causes first, in layout order; a control the profile's
    // list does not include is reported only when nothing more specific is.
    let mut other = None;
    for id in view.controls() {
        let control = node(package, id)?;
        let found = match control_operation(control)? {
            ControlOperation::Repeat => Some(cause(UnsupportedSlug::Repeat, id, None)),
            ControlOperation::Await => Some(cause(UnsupportedSlug::TimedAwait, id, None)),
            ControlOperation::Compensate => Some(cause(UnsupportedSlug::Compensation, id, None)),
            ControlOperation::Check
            | ControlOperation::Attempt
            | ControlOperation::Effect
            | ControlOperation::Event
            | ControlOperation::Commit
            | ControlOperation::Fence => {
                other.get_or_insert_with(|| cause(UnsupportedSlug::UnsupportedControl, id, None));
                None
            }
            _ => {
                match control_body(control)? {
                    ControlBody::Parallel { join, .. } if join.policy != "all" => Some(cause(
                        UnsupportedSlug::UnsupportedJoin,
                        id,
                        Some(join.policy),
                    )),
                    ControlBody::Parallel { join, .. } if join.outstanding.is_some() => Some(
                        cause(UnsupportedSlug::UnsupportedJoin, id, Some("outstanding")),
                    ),
                    _ => None,
                }
            }
        };
        if found.is_some() {
            return Ok(found);
        }
    }
    Ok(other)
}

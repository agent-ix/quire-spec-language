// SPDX-License-Identifier: AGPL-3.0-or-later
//! The JSON a protocol counterexample's members are written as (QSpec FR-433,
//! FR-426, FR-429, FR-430, FR-438): each is the member's typed canonical
//! form, written through the one canonical writer.

use quire_exact::{ObjectReference, Value};
use serde_json::Value as Json;

use super::access::location_enc;
use super::fairness::ProtocolFairness;
use super::identity::{BlockedThread, ProtocolTransition, SendRecord, WaitCause};
use super::over_value;
use crate::model_subject::ModelInstance;
use crate::simulation::model::encode::Enc;
use crate::simulation::model::{ModelSimulationRequest, ModelSystem};
use crate::simulation::Location;

use crate::simulation::model::canonical_json as json;

/// A protocol step's transition identity (QSpec FR-426).
pub fn transition_json(transition: &ProtocolTransition) -> Option<Json> {
    json(transition)
}

/// A `send` step's send record (QSpec FR-426).
pub fn send_record_json(record: &SendRecord) -> Option<Json> {
    json(record)
}

/// A member of the resolved fairness set over a protocol subject, with its
/// target and origin (QSpec FR-430).
pub fn fairness_json(fairness: &ProtocolFairness) -> Option<Json> {
    json(fairness)
}

/// A shared location (QSpec FR-438's `race.location`).
pub fn location_json(location: &Location) -> Option<Json> {
    json(&location_enc(location, None))
}

/// One waiting thread of a deadlocked state (QSpec FR-429 "Blocked threads").
pub fn blocked_json(blocked: &BlockedThread) -> Option<Json> {
    let paths = |threads: &[super::identity::ThreadPath]| {
        Enc::Array(threads.iter().map(|thread| thread.enc()).collect())
    };
    let channel = |kind: &str, channel: &String| {
        Enc::tagged(kind, vec![("channel", Enc::text(channel.clone()))])
    };
    let cause = match &blocked.cause {
        WaitCause::Precondition { clause } => {
            Enc::tagged("precondition", vec![("clause", Enc::text(clause.clone()))])
        }
        WaitCause::NoPostState => Enc::tagged("no-post-state", Vec::new()),
        WaitCause::Constraint => Enc::tagged("constraint", Vec::new()),
        WaitCause::EmptyQueue { channel: name } => channel("empty-queue", name),
        WaitCause::HeadBlocked { channel: name } => channel("head-blocked", name),
        WaitCause::FullQueue { channel: name } => channel("full-queue", name),
        WaitCause::Displaced { channel: name } => channel("displaced", name),
        WaitCause::CheckFalse { node } => {
            Enc::tagged("check-false", vec![("node", Enc::text(node.clone()))])
        }
        WaitCause::JoinWaiting { threads } => {
            Enc::tagged("join-waiting", vec![("threads", paths(threads))])
        }
        WaitCause::FinishWaiting { threads } => {
            Enc::tagged("finish-waiting", vec![("threads", paths(threads))])
        }
        WaitCause::CommitClosed { template } => Enc::tagged(
            "commit-closed",
            vec![("template", Enc::text(template.clone()))],
        ),
        WaitCause::NoRoleInstance { role } => {
            Enc::tagged("no-role-instance", vec![("role", Enc::text(role.clone()))])
        }
        WaitCause::Gate => Enc::tagged("gate", Vec::new()),
    };
    json(&Enc::Object(vec![
        ("thread", blocked.path.enc()),
        ("rest", blocked.rest_point.enc()),
        ("cause", cause),
    ]))
}

/// The reference the protocol clause `protocol`'s `over` parameter is bound
/// to under `over`, over the model `simulation` describes (QSpec FR-433's
/// `subject.over`), or `None` when the model refuses it.
pub fn over_reference(
    simulation: ModelSimulationRequest<'_>,
    protocol: &str,
    over: &ModelInstance,
) -> Option<ObjectReference> {
    let package = simulation.package;
    let model = ModelSystem::new(simulation).ok()?;
    let protocol = package.graph().protocol(protocol)?;
    match over_value(&model, protocol, over).ok()? {
        Value::Reference(reference) => Some(reference),
        _ => None,
    }
}

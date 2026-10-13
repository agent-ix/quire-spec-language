// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 state-machine binder.
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use crate::model::domain_package::{
    DomainPackageRecord, StateMachineRecord, StateMachineStateRecord, StateMachineTransitionRecord,
};
use crate::model::intake::{member_identity_name, read_record_value_type, NodeCtx};
use crate::model::key::DeclarationKey;
use crate::model::normalize::ModelRefusal;
use serde_json::Value;

use super::meaning_dispatch::MeaningBindingContext;

fn key(package: &str, node: &str) -> DeclarationKey {
    DeclarationKey {
        package: package.to_owned(),
        node: node.to_owned(),
    }
}

fn array<'a>(
    ctx: &NodeCtx<'a>,
    value: &'a Value,
    field: &'static str,
) -> Result<&'a [Value], ModelRefusal> {
    match value.get(field) {
        None => Ok(&[]),
        Some(value) => value
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| ctx.malformed_in(field, "not an array")),
    }
}

fn member_key(
    package: &str,
    owner: &str,
    value: &Value,
    at: &str,
) -> Result<DeclarationKey, ModelRefusal> {
    let ctx = NodeCtx::new(value, at);
    let identity = ctx.str_field("identity")?;
    if member_identity_name(owner, identity).is_none() {
        return Err(ctx.malformed_in(
            "identity",
            format!("{identity:?} is not a member of {owner}"),
        ));
    }
    Ok(key(package, identity))
}

fn read_states(
    package: &str,
    owner: &str,
    value: &Value,
    at: &str,
) -> Result<Vec<StateMachineStateRecord>, ModelRefusal> {
    let ctx = NodeCtx::new(value, at);
    array(&ctx, value, "states")?
        .iter()
        .enumerate()
        .map(|(position, state)| {
            let member_at = format!("{at}.states[{position}]");
            let state_ctx = NodeCtx::new(state, &member_at);
            let key = member_key(package, owner, state, &member_at)?;
            let name = state_ctx.str_field("name")?;
            if member_identity_name(owner, &key.node) != Some(name) {
                return Err(
                    state_ctx.malformed_in("name", "differs from the state's identity member name")
                );
            }
            Ok(StateMachineStateRecord {
                key,
                name: name.to_owned(),
            })
        })
        .collect()
}

fn read_transitions(
    package: &str,
    owner: &str,
    value: &Value,
    at: &str,
) -> Result<Vec<StateMachineTransitionRecord>, ModelRefusal> {
    let ctx = NodeCtx::new(value, at);
    array(&ctx, value, "transitions")?
        .iter()
        .enumerate()
        .map(|(position, transition)| {
            let member_at = format!("{at}.transitions[{position}]");
            let transition_ctx = NodeCtx::new(transition, &member_at);
            let key = member_key(package, owner, transition, &member_at)?;
            let reference = |field: &'static str| -> Result<DeclarationKey, ModelRefusal> {
                let node = transition_ctx.str_field(field)?;
                Ok(key_for(package, node))
            };
            let emits = match transition.get("emits") {
                None => Vec::new(),
                Some(value) => value
                    .as_array()
                    .ok_or_else(|| transition_ctx.malformed_in("emits", "not an array"))?
                    .iter()
                    .enumerate()
                    .map(|(emit_position, emit)| {
                        emit.as_str()
                            .map(|node| key_for(package, node))
                            .ok_or_else(|| {
                                transition_ctx.malformed_in(
                                    &format!("emits[{emit_position}]"),
                                    "not an identity string",
                                )
                            })
                    })
                    .collect::<Result<_, _>>()?,
            };
            let guard = match transition.get("guard") {
                None => None,
                Some(value) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            transition_ctx.malformed_in("guard", "not a string clauseId")
                        })?
                        .to_owned(),
                ),
            };
            Ok(StateMachineTransitionRecord {
                key,
                from: reference("from")?,
                to: reference("to")?,
                trigger: reference("trigger")?,
                guard,
                emits,
                path: format!(
                    "/types/{}/transitions/{position}",
                    at.strip_prefix("$.types[")
                        .and_then(|tail| tail.strip_suffix(']'))
                        .expect("state-machine reader receives a types[] path")
                ),
                origin: transition.get("origin").cloned().unwrap_or(Value::Null),
            })
        })
        .collect()
}

fn key_for(package: &str, node: &str) -> DeclarationKey {
    key(package, node)
}

pub(super) fn bind(context: &mut MeaningBindingContext<'_, '_>) -> Result<(), ModelRefusal> {
    let ctx = NodeCtx::new(context.type_value, context.at);
    let supertypes = ctx.identity_keys(context.package, "supertypes")?;
    let states = read_states(
        context.package,
        context.node,
        context.type_value,
        context.at,
    )?;
    let transitions = read_transitions(
        context.package,
        context.node,
        context.type_value,
        context.at,
    )?;

    // Reuse the shared field, operation and clause readers. Their temporary
    // record-value type must not apply identity or record-only ancestry rules.
    let mut shared = context.type_value.clone();
    let object = shared.as_object_mut().expect("admitted type is an object");
    object.remove("identityFields");
    object.remove("supertypes");
    read_record_value_type(
        context.package,
        &shared,
        context.node,
        context.at,
        context.type_meanings,
        context.records,
        context.pending,
    )?;
    let type_key = key(context.package, context.node);
    let Some(record) = context.records.iter_mut().find(
        |record| matches!(record, DomainPackageRecord::RecordValueType(value) if value.key == type_key),
    ) else {
        unreachable!("shared record reader adds the temporary type record")
    };
    *record = DomainPackageRecord::StateMachine(StateMachineRecord {
        key: type_key,
        supertypes,
        states,
        transitions,
    });
    Ok(())
}

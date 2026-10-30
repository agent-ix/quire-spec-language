// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-390 selection of one S3 checked expression from an S4 protocol node.

use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    CaptureOwner, CheckedActivation, CheckedBody, CheckedLifetime, CheckedOrdering,
    CheckedProtocol, CheckedRetry, CheckedSlots, ProtocolNodeId,
};
use quire_eval::{CallAdmissionFailure, CallAdmissionLocus, CallAdmissionRecord, CallFailure};
use quire_exact::{NodeKey, ValueType};
use quire_semantic_value::catalog::CatalogCode;

use super::ProtocolSlot;

#[derive(Clone, Debug)]
pub(crate) struct FrameSlot {
    pub(crate) name: String,
    pub(crate) level: usize,
    pub(crate) value_type: ValueType,
    pub(crate) node: Option<NodeKey>,
}

pub(crate) struct Selected<'a> {
    pub(crate) body: &'a CheckedBody,
    pub(crate) frame: Vec<FrameSlot>,
}

#[derive(Clone, Copy)]
enum Node<'a> {
    Clause,
    Role(&'a str),
    Channel(&'a str),
    Compensation(usize),
    Terminal,
    Run(ProtocolNodeId),
}

fn admission(code: &'static str, cause: &'static str, locus: CallAdmissionLocus) -> CallFailure {
    CallFailure::Admission(Box::new(CallAdmissionFailure::Refused(
        CallAdmissionRecord {
            code: CatalogCode::new(code, cause),
            locus,
        },
    )))
}

pub(crate) fn refused(
    code: &'static str,
    cause: &'static str,
    locus: CallAdmissionLocus,
) -> CallFailure {
    admission(code, cause, locus)
}

pub(crate) fn shape_matches(value_type: &ValueType, value: &quire_exact::Value) -> bool {
    use quire_exact::{Value, ValueType};
    matches!(
        (value_type, value),
        (ValueType::Boolean, Value::Boolean(_))
            | (ValueType::Integer | ValueType::Int(_), Value::Integer(_))
            | (ValueType::Rational(_), Value::Rational(_))
            | (ValueType::Decimal(_), Value::Decimal(_))
            | (ValueType::Float(_), Value::Float(_))
            | (
                ValueType::Quantity(_) | ValueType::BoundedQuantity(_),
                Value::Quantity(_)
            )
            | (ValueType::Text(_), Value::Text(_))
            | (ValueType::String, Value::String(_))
            | (ValueType::Uuid, Value::Uuid(_))
            | (ValueType::Timestamp, Value::Timestamp(_))
            | (ValueType::Enum(_), Value::Enum(_))
            | (ValueType::Option(_), Value::Option(_))
            | (
                ValueType::Composite(_),
                Value::Composite(_) | Value::Union(_)
            )
            | (ValueType::Collection(_), Value::Collection(_))
            | (ValueType::Reference(_), Value::Reference(_))
    )
}

fn missing_node(key: NodeKey) -> CallFailure {
    admission(
        "missing_declaration",
        "missing-name",
        CallAdmissionLocus::Node(key),
    )
}

fn malformed_node(key: NodeKey) -> CallFailure {
    admission(
        "invalid_model_binding",
        "malformed-declaration",
        CallAdmissionLocus::Node(key),
    )
}

fn missing_member(member: &str, index: Option<usize>) -> CallFailure {
    admission(
        "missing_declaration",
        "missing-name",
        CallAdmissionLocus::Member {
            name: member.to_owned(),
            index,
        },
    )
}

fn identify<'a>(
    package: &'a CheckedPackage,
    key: NodeKey,
    member: &str,
    index: Option<usize>,
) -> Result<(&'a CheckedProtocol, Node<'a>), CallFailure> {
    let graph = package.graph();
    let Some(wire) = graph.semantic_graph().node(key.clone()) else {
        return Err(missing_node(key));
    };
    if wire.node_tag() != qsl_semantics::check::NodeTag::Protocol {
        return Err(malformed_node(key));
    }
    for protocol in graph.protocols() {
        let Some(keys) = graph.protocol_node_keys(&protocol.name) else {
            continue;
        };
        if keys.clause == Some(key.clone()) {
            return Ok((protocol, Node::Clause));
        }
        if let Some((name, _)) = keys.roles.iter().find(|(_, candidate)| **candidate == key) {
            return Ok((protocol, Node::Role(name)));
        }
        if let Some((name, _)) = keys
            .channels
            .iter()
            .find(|(_, candidate)| **candidate == key)
        {
            return Ok((protocol, Node::Channel(name)));
        }
        if let Some(index) = keys
            .compensations
            .iter()
            .position(|candidate| *candidate == key)
        {
            return Ok((protocol, Node::Compensation(index)));
        }
        if keys.terminal == Some(key.clone()) {
            return Ok((protocol, Node::Terminal));
        }
        if let Some((node, _)) = keys.nodes.iter().find(|(_, candidate)| **candidate == key) {
            return Ok((protocol, Node::Run(*node)));
        }
    }
    // A protocol-tagged node outside the checked protocol member table has
    // no FR-390 expression member.
    Err(missing_member(member, index))
}

fn checked_slots(slots: &CheckedSlots) -> Vec<FrameSlot> {
    slots
        .binders
        .iter()
        .enumerate()
        .map(|(level, binder)| FrameSlot {
            name: binder.name.clone(),
            level,
            value_type: binder.value_type.clone(),
            node: None,
        })
        .collect()
}

fn capture_entries(protocol: &CheckedProtocol, indexes: &[usize]) -> Vec<FrameSlot> {
    indexes
        .iter()
        .filter_map(|index| protocol.obligations.captures.get(*index))
        .map(|capture| FrameSlot {
            name: capture.name.clone(),
            level: 0, // The enclosing protocol's checked parameter key supplies the canonical level.
            value_type: capture.declared_type.clone(),
            node: None,
        })
        .collect()
}

fn protocol_captures(protocol: &CheckedProtocol) -> Vec<FrameSlot> {
    protocol
        .obligations
        .captures
        .iter()
        .filter(|capture| capture.owner == CaptureOwner::Protocol)
        .map(|capture| FrameSlot {
            name: capture.name.clone(),
            level: 0,
            value_type: capture.declared_type.clone(),
            node: None,
        })
        .collect()
}

fn protocol_input(protocol: &CheckedProtocol) -> Vec<FrameSlot> {
    protocol
        .slots
        .binders
        .first()
        .map(|binder| {
            vec![FrameSlot {
                name: binder.name.clone(),
                level: 0,
                value_type: binder.value_type.clone(),
                node: None,
            }]
        })
        .unwrap_or_default()
}

fn activation_binder(protocol: &CheckedProtocol) -> Vec<FrameSlot> {
    match &protocol.obligations.activation {
        CheckedActivation::Origin => Vec::new(),
        CheckedActivation::OnEach {
            binder,
            record_type,
            ..
        } => vec![FrameSlot {
            name: binder.clone(),
            level: 1,
            value_type: (**record_type).clone(),
            node: None,
        }],
    }
}

fn compensation_frame(
    _protocol: &CheckedProtocol,
    _index: usize,
    names: Vec<(String, ValueType)>,
) -> Vec<FrameSlot> {
    names
        .into_iter()
        .enumerate()
        .map(|(level, (name, value_type))| FrameSlot {
            level,
            name,
            value_type,
            node: None,
        })
        .collect()
}

fn compensation_retry(compensation: &CheckedRetry) -> Vec<(String, ValueType)> {
    vec![
        (
            compensation.earlier.name.clone(),
            compensation.earlier.value_type.clone(),
        ),
        (
            compensation.later.name.clone(),
            compensation.later.value_type.clone(),
        ),
    ]
}

/// `#[string_edge]`: the member names and checked control labels are the
/// protocol slot's declared discriminator vocabulary (FR-390), not values
/// decoded from arbitrary text.
#[ix_trace_rs::string_edge]
fn run_body<'a>(
    _package: &'a CheckedPackage,
    protocol: &'a CheckedProtocol,
    node: ProtocolNodeId,
    member: &str,
    index: Option<usize>,
) -> Option<(&'a CheckedBody, Vec<FrameSlot>)> {
    let slots = checked_slots(&protocol.slots);
    if protocol
        .finish
        .as_ref()
        .is_some_and(|(finish, _)| *finish == node)
    {
        return Some((
            &protocol
                .slots
                .bodies
                .iter()
                .find(|body| body.node == node)?
                .body,
            slots,
        ));
    }
    let kind = protocol.controls.control(node)?.kind.label();
    match (kind, member) {
        ("choice" | "repeat", "visible") => {
            let index = index?;
            let body = match kind {
                "choice" => protocol
                    .slots
                    .choices
                    .iter()
                    .find(|choice| choice.node == node)?
                    .visible
                    .get(index)?,
                "repeat" => protocol
                    .slots
                    .repeats
                    .iter()
                    .find(|repeat| repeat.node == node)?
                    .visible
                    .get(index)?,
                _ => return None,
            };
            Some((body, slots))
        }
        ("choice", "cases.guard") => Some((
            &protocol
                .slots
                .choices
                .iter()
                .find(|choice| choice.node == node)?
                .cases
                .get(index?)?
                .guard,
            slots,
        )),
        ("repeat", "guard") => Some((
            &protocol
                .slots
                .repeats
                .iter()
                .find(|repeat| repeat.node == node)?
                .guard,
            slots,
        )),
        ("check", "condition") => Some((
            &protocol
                .slots
                .bodies
                .iter()
                .find(|body| body.node == node)?
                .body,
            slots,
        )),
        ("send" | "receive" | "attempt" | "effect" | "event" | "commit", "constraint") => Some((
            &protocol
                .slots
                .bodies
                .iter()
                .find(|body| body.node == node)?
                .body,
            slots,
        )),
        ("send" | "receive" | "attempt" | "effect" | "event", "related.from") => Some((
            protocol
                .slots
                .related
                .iter()
                .filter(|related| related.node == node)
                .nth(index?)?
                .ends
                .first()?,
            slots,
        )),
        ("send" | "receive" | "attempt" | "effect" | "event", "related.to") => Some((
            protocol
                .slots
                .related
                .iter()
                .filter(|related| related.node == node)
                .nth(index?)?
                .ends
                .get(1)?,
            slots,
        )),
        _ => None,
    }
}

/// `#[string_edge]`: dispatches the caller's exact FR-390 member key against
/// the checked protocol member and slot table.
#[ix_trace_rs::string_edge]
pub(crate) fn select<'a>(
    package: &'a CheckedPackage,
    slot: &ProtocolSlot,
) -> Result<Selected<'a>, CallFailure> {
    let (protocol, node) = identify(package, slot.node.clone(), &slot.member, slot.index)?;
    let absent = || missing_member(&slot.member, slot.index);
    let indexed = match node {
        Node::Clause => slot.member == "captures",
        Node::Compensation(_) => {
            matches!(
                slot.member.as_str(),
                "registration_captures" | "activation_captures"
            )
        }
        Node::Run(node) => matches!(
            (
                protocol
                    .controls
                    .control(node)
                    .map(|control| control.kind.label()),
                slot.member.as_str()
            ),
            (Some("choice"), "cases.guard" | "visible")
                | (Some("repeat"), "visible")
                | (
                    Some("send" | "receive" | "attempt" | "effect" | "event"),
                    "related.from" | "related.to"
                )
        ),
        Node::Role(_) | Node::Channel(_) | Node::Terminal => false,
    };
    if indexed != slot.index.is_some() {
        return Err(absent());
    }
    let (body, frame) = match node {
        Node::Clause => match slot.member.as_str() {
            "activation.guard" => {
                let CheckedActivation::OnEach {
                    guard: Some(body),
                    binder,
                    record_type,
                } = &protocol.obligations.activation
                else {
                    return Err(absent());
                };
                let over = protocol.slots.binders.first().ok_or_else(absent)?;
                (
                    body,
                    vec![
                        FrameSlot {
                            name: binder.clone(),
                            level: 0,
                            value_type: (**record_type).clone(),
                            node: None,
                        },
                        FrameSlot {
                            name: over.name.clone(),
                            level: 1,
                            value_type: over.value_type.clone(),
                            node: None,
                        },
                    ],
                )
            }
            "captures" => {
                let source_index = slot.index.ok_or_else(absent)?;
                let capture = protocol
                    .obligations
                    .captures
                    .iter()
                    .filter(|capture| capture.owner == CaptureOwner::Protocol)
                    .nth(source_index)
                    .ok_or_else(absent)?;
                (&capture.initializer, checked_slots(&protocol.slots))
            }
            _ => return Err(absent()),
        },
        Node::Role(name) if slot.member == "replication.until" && slot.index.is_none() => {
            let role = protocol
                .roles
                .iter()
                .find(|role| role.name == name)
                .ok_or_else(absent)?;
            let checked = protocol
                .slots
                .roles
                .iter()
                .find(|role| {
                    role.role
                        == protocol
                            .roles
                            .iter()
                            .position(|candidate| candidate.name == name)
                            .unwrap_or(usize::MAX)
                })
                .ok_or_else(absent)?;
            let CheckedLifetime::Until(body) = &checked.lifetime else {
                return Err(absent());
            };
            let mut frame = protocol_input(protocol);
            frame.extend(activation_binder(protocol));
            frame.extend(protocol_captures(protocol));
            frame.push(FrameSlot {
                name: role.name.clone(),
                level: frame.len(),
                value_type: quire_exact::ValueType::Reference(
                    role.context.clone().ok_or_else(absent)?,
                ),
                node: None,
            });
            for (level, source) in frame.iter_mut().enumerate() {
                source.level = level;
            }
            (body, frame)
        }
        Node::Channel(name) if slot.member == "ordering.key" && slot.index.is_none() => {
            let channel = protocol
                .channels
                .iter()
                .find(|channel| channel.name == name)
                .ok_or_else(absent)?;
            let CheckedOrdering::Fifo { binder, key } = &channel.ordering else {
                return Err(absent());
            };
            (
                key.as_ref(),
                vec![FrameSlot {
                    name: binder.name.clone(),
                    level: 0,
                    value_type: binder.value_type.clone(),
                    node: None,
                }],
            )
        }
        Node::Compensation(index) => {
            let compensation = protocol
                .obligations
                .compensations
                .get(index)
                .ok_or_else(absent)?;
            match slot.member.as_str() {
                "registration_captures" => {
                    let capture_index = slot.index.ok_or_else(absent)?;
                    let capture = protocol
                        .obligations
                        .captures
                        .get(
                            *compensation
                                .registration_captures
                                .get(capture_index)
                                .ok_or_else(absent)?,
                        )
                        .ok_or_else(absent)?;
                    let mut frame = Vec::new();
                    frame.push((
                        compensation.forward.name.clone(),
                        compensation.forward.value_type.clone(),
                    ));
                    frame.extend(
                        protocol_input(protocol)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    frame.extend(
                        activation_binder(protocol)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    frame.extend(
                        protocol_captures(protocol)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    frame.extend(
                        capture_entries(
                            protocol,
                            &compensation.registration_captures[..capture_index],
                        )
                        .into_iter()
                        .map(|source| (source.name, source.value_type)),
                    );
                    frame.push((capture.name.clone(), capture.declared_type.clone()));
                    (
                        &capture.initializer,
                        compensation_frame(protocol, index, frame),
                    )
                }
                "trigger.guard" if slot.index.is_none() => {
                    let mut frame = vec![(
                        compensation.trigger.name.clone(),
                        compensation.trigger.value_type.clone(),
                    )];
                    frame.extend(
                        capture_entries(protocol, &compensation.registration_captures)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    (
                        &compensation.guard,
                        compensation_frame(protocol, index, frame),
                    )
                }
                "activation_captures" => {
                    let capture_index = slot.index.ok_or_else(absent)?;
                    let capture = protocol
                        .obligations
                        .captures
                        .get(
                            *compensation
                                .activation_captures
                                .get(capture_index)
                                .ok_or_else(absent)?,
                        )
                        .ok_or_else(absent)?;
                    let mut frame = vec![(
                        compensation.trigger.name.clone(),
                        compensation.trigger.value_type.clone(),
                    )];
                    frame.extend(
                        protocol_input(protocol)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    frame.extend(
                        protocol_captures(protocol)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    frame.extend(
                        capture_entries(protocol, &compensation.registration_captures)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    frame.extend(
                        capture_entries(
                            protocol,
                            &compensation.activation_captures[..capture_index],
                        )
                        .into_iter()
                        .map(|source| (source.name, source.value_type)),
                    );
                    frame.push((capture.name.clone(), capture.declared_type.clone()));
                    (
                        &capture.initializer,
                        compensation_frame(protocol, index, frame),
                    )
                }
                "retry.relation" if slot.index.is_none() => {
                    let mut frame = compensation_retry(&compensation.retry);
                    frame.extend(
                        protocol_captures(protocol)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    frame.extend(
                        capture_entries(protocol, &compensation.registration_captures)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    frame.extend(
                        capture_entries(protocol, &compensation.activation_captures)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    (
                        &compensation.retry.relation,
                        compensation_frame(protocol, index, frame),
                    )
                }
                "recovery.predicate" if slot.index.is_none() => {
                    let mut frame = vec![(
                        compensation.recover.binder.name.clone(),
                        compensation.recover.binder.value_type.clone(),
                    )];
                    frame.extend(
                        protocol_captures(protocol)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    frame.extend(
                        capture_entries(protocol, &compensation.registration_captures)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    frame.extend(
                        capture_entries(protocol, &compensation.activation_captures)
                            .into_iter()
                            .map(|source| (source.name, source.value_type)),
                    );
                    (
                        &compensation.recover.predicate,
                        compensation_frame(protocol, index, frame),
                    )
                }
                _ => return Err(absent()),
            }
        }
        Node::Terminal if slot.member == "condition" && slot.index.is_none() => {
            let qsl_semantics::check::CheckedTerminalMember::When(body) = &protocol.slots.terminal
            else {
                return Err(absent());
            };
            (body, checked_slots(&protocol.slots))
        }
        Node::Terminal => return Err(absent()),
        Node::Role(_) | Node::Channel(_) => return Err(absent()),
        Node::Run(node) => {
            let Some((body, frame)) = run_body(package, protocol, node, &slot.member, slot.index)
            else {
                return Err(absent());
            };
            (body, frame)
        }
    };
    let mut frame = frame;
    if let Some(keys) = package.graph().protocol_node_keys(&protocol.name) {
        let location = body.root().location();
        let expression = keys
            .expressions
            .iter()
            .find(|expression| expression.location == *location);
        for source in &mut frame {
            source.node = expression
                .and_then(|expression| expression.binders.get(source.level))
                .filter(|binder| binder.name == source.name)
                .map(|binder| binder.node.clone());
        }
    }
    Ok(Selected { body, frame })
}

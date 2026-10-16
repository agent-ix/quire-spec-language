// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 process binder.
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::model::domain_package::{
    DomainPackageRecord, ProcessRecord, ProcessStepKind, ProcessStepRecord,
};
use crate::model::intake::{
    malformed_on, meaning, member_identity_name, read_clauses, read_object_type, NodeCtx,
    ObjectShape,
};
use crate::model::key::DeclarationKey;
use crate::model::normalize::{MemberPath, ModelRefusal, ModelRefusalCause};
use qsl_foundation::diagnostic::{Code, JsonPointer};

use super::meaning_dispatch::MeaningBindingContext;

#[qsl_attrs::string_edge]
pub(super) fn bind(context: &mut MeaningBindingContext<'_, '_>) -> Result<(), ModelRefusal> {
    let ctx = NodeCtx::new(context.type_value, context.at);
    let identity_fields = ctx.identity_keys(context.package, "identityFields")?;
    let identity_fields_declared = context.type_value.get("identityFields").is_some();
    let supertypes = ctx.identity_keys(context.package, "supertypes")?;
    for (position, general) in supertypes.iter().enumerate() {
        match context.meanings_by_node.get(general.node.as_str()) {
            Some(_) => {}
            None => {
                return Err(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: ModelRefusalCause::UnknownGeneral {
                        supertype: DeclarationKey {
                            package: context.package.to_owned(),
                            node: context.node.to_owned(),
                        },
                        general: general.clone(),
                    },
                    detail: format!(
                        "{}.supertypes[{position}]: {} names no type of this domain package",
                        context.at, general.node
                    ),
                })
            }
        }
    }

    let steps = ctx.array_field("steps")?;
    if steps.is_empty() {
        return Err(ctx.malformed_in("steps", "a process has one or more steps"));
    }
    let mut names = HashSet::new();
    let mut step_records = Vec::with_capacity(steps.len());
    for (position, step) in steps.iter().enumerate() {
        let step_at = format!("{}.steps[{position}]", context.at);
        let step_ctx = NodeCtx::new(step, step_at.clone());
        let identity = step_ctx.str_field("identity")?;
        let name = step_ctx.str_field("name")?;
        if member_identity_name(context.node, identity) != Some(name) {
            return Err(step_ctx.malformed_member(
                MemberPath::from_document_location(&step_at).name("identity"),
                "identity/name: step identity is not the declared process member",
            ));
        }
        let step_key = DeclarationKey {
            package: context.package.to_owned(),
            node: identity.to_owned(),
        };
        if !names.insert(name.to_owned()) {
            return Err(ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::ConflictingBinding {
                    // TC-235 M07 run 3 names the process, not the step.
                    key: DeclarationKey {
                        package: context.package.to_owned(),
                        node: context.node.to_owned(),
                    },
                    member: Some(MemberPath::from_document_location(&step_at)),
                },
                detail: format!("{step_at}: duplicate process step"),
            });
        }
        let step_kind = match step_ctx.str_field("stepKind")? {
            "command" => ProcessStepKind::Command,
            "event" => ProcessStepKind::Event,
            "decision" => ProcessStepKind::Decision,
            "compensation" => ProcessStepKind::Compensation,
            "wait" => ProcessStepKind::Wait,
            other => {
                return Err(step_ctx.malformed_in("stepKind", format!("unknown value {other:?}")))
            }
        };
        let consumes = read_step_events(
            context.package,
            context.node,
            context.meanings_by_node,
            step,
            &step_at,
            "consumes",
        )?;
        let emits = read_step_events(
            context.package,
            context.node,
            context.meanings_by_node,
            step,
            &step_at,
            "emits",
        )?;
        step_records.push(ProcessStepRecord {
            key: step_key,
            name: name.to_owned(),
            step_kind,
            consumes,
            emits,
        });
    }

    let mut shared = context.type_value.clone();
    let object = shared
        .as_object_mut()
        .expect("admitted process type is an object");
    object.remove("identityFields");
    object.remove("supertypes");
    object.remove("steps");
    read_object_type(
        context.package,
        &shared,
        context.node,
        context.at,
        ObjectShape::Process,
        context.records,
        context.pending,
    )?;
    let key = DeclarationKey {
        package: context.package.to_owned(),
        node: context.node.to_owned(),
    };
    let Some(record) = context.records.iter_mut().find(
        |record| matches!(record, DomainPackageRecord::ObjectType(value) if value.key == key),
    ) else {
        unreachable!("shared object reader adds the temporary type record")
    };
    *record = DomainPackageRecord::Process(ProcessRecord {
        key,
        identity_fields,
        identity_fields_declared,
        supertypes,
        steps: step_records,
    });
    read_clauses(context.package, context.node, &ctx, context.records)
}

fn read_step_events(
    package: &str,
    process: &str,
    type_meanings: &HashMap<&str, &str>,
    step: &Value,
    step_at: &str,
    field: &'static str,
) -> Result<Vec<DeclarationKey>, ModelRefusal> {
    let values = NodeCtx::new(step, step_at).array_field(field)?;
    let step_identity = step
        .get("identity")
        .and_then(Value::as_str)
        .unwrap_or(process);
    let mut result = Vec::with_capacity(values.len());
    for (position, value) in values.iter().enumerate() {
        let at = format!("{step_at}.{field}[{position}]");
        let event = value.as_str().ok_or_else(|| {
            malformed_on(
                process,
                value,
                &at,
                MemberPath::from_document_location(&at),
                "event reference: not a string",
            )
        })?;
        let key = DeclarationKey {
            package: package.to_owned(),
            node: event.to_owned(),
        };
        match type_meanings.get(event) {
            Some(&meaning::EVENT_TYPE) => result.push(key),
            Some(_) => {
                return Err(malformed_on(
                    process,
                    step,
                    &at,
                    MemberPath::from_document_location(&at),
                    format!("{field}: event reference names a non-event type in process {process}"),
                ))
            }
            None => {
                let path = reader_pointer(step_at, field, position);
                return Err(ModelRefusal {
                    code: Code::MissingDeclaration,
                    cause: ModelRefusalCause::IntakeMissingDeclaration {
                        node: process.to_owned(),
                        reference: key.node.clone(),
                        origin: step.get("origin").cloned().unwrap_or(Value::Null),
                        path,
                        member: MemberPath::from_document_location(&at),
                    },
                    detail: format!(
                        "{at} on {step_identity}: names no event type of this domain package"
                    ),
                });
            }
        }
    }
    Ok(result)
}

#[qsl_attrs::string_edge]
fn reader_pointer(step_at: &str, field: &str, position: usize) -> JsonPointer {
    let mut pointer = JsonPointer::root();
    let path = step_at.strip_prefix("$.").unwrap_or(step_at);
    for segment in path.split('.') {
        let mut rest = segment;
        loop {
            let Some(open) = rest.find('[') else {
                if !rest.is_empty() {
                    pointer = pointer.key(rest);
                }
                break;
            };
            if open > 0 {
                pointer = pointer.key(&rest[..open]);
            }
            let Some(close) = rest[open + 1..].find(']') else {
                break;
            };
            let index = &rest[open + 1..open + 1 + close];
            pointer = pointer.key(index);
            rest = &rest[open + 2 + close..];
        }
    }
    pointer.key(field).key(&position.to_string())
}

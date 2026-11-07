// SPDX-License-Identifier: AGPL-3.0-or-later
//! Decode the corpus replay adapter into the existing typed replay API.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use qsl_foundation::node_wire::{node_key_to_json, node_key_to_text};
use qsl_foundation::{SettingLimits, SettingName};
use qsl_replay::{
    DeclaredDomain, DomainKey, DomainKind, FiniteBound, Integer, ProofBound, ReplayRequestWire,
    ReplaySource, RequestTargetWire, WitnessValue,
};
use serde_json::{json, Value};

use super::{
    array, document, from_document, object, text, unsigned, Execution, Failure, InputKind, Inputs,
};

pub(super) fn execute(
    inputs: &Inputs<'_>,
    root: &Path,
    limits: &qsl_replay::CallerLimits,
    overrides: &[String],
    schema: &jsonschema::JSONSchema,
) -> Result<Execution, Failure> {
    let (_, bytes) = inputs.one(InputKind::ReplayRequest)?;
    let value = super::super::DeepValue::new(document(bytes, schema)?);
    if value["package"]["lock"]
        .get("refinement_internals")
        .is_some()
    {
        return super::unavailable_refinement("/package/lock/refinement_internals".to_owned());
    }
    for (index, dependency) in array(&value["dependencies"], "/dependencies")?
        .iter()
        .enumerate()
    {
        if dependency.get("refinement_internals").is_some() {
            return super::unavailable_refinement(format!(
                "/dependencies/{index}/refinement_internals"
            ));
        }
    }
    let mut request = decode(&value, inputs, root)?;
    if let Some((path, error)) = inputs.unreadable_bytes.first() {
        return Err(Failure::io(path, error));
    }
    for operand in overrides {
        let Some((name, _)) = operand.split_once('=') else {
            continue;
        };
        let bound = match SettingName::from_name(name) {
            Some(SettingName::Accounting(setting)) => limits
                .accounting
                .bounds()
                .into_iter()
                .find(|(candidate, _)| *candidate == setting)
                .map(|(_, bound)| bound),
            Some(SettingName::Stage(setting)) => limits
                .spine
                .bounds()
                .into_iter()
                .find(|(candidate, _)| *candidate == setting)
                .map(|(_, bound)| bound),
            None => None,
        };
        if let Some(bound) = bound {
            if qsl_replay::StageLimits::default()
                .with_named(name, bound)
                .is_ok()
            {
                request.stage_limits.insert(name.to_owned(), bound);
            }
        }
    }
    let result = qsl_replay::replay(request, limits.replay);
    let actual = result.as_ref().ok().map(replay_value).transpose()?;
    let mut execution = from_document(qsl_replay::OutcomeDocument::from_replay(&result), actual)?;
    if result.as_ref().is_ok_and(|result| {
        result.limit().is_some()
            || result.pre_call_limit().is_some_and(|stop| {
                matches!(
                    stop.cause,
                    quire_semantic_value::supplied_stop::SuppliedCause::Limit(_)
                        | quire_semantic_value::supplied_stop::SuppliedCause::Cancelled(_)
                )
            })
    }) {
        execution.partition = super::Partition::Incomplete;
    }
    Ok(execution)
}

fn node(
    value: &Value,
    pointer: &str,
    graph: &qsl_replay::WireGraph,
) -> Result<qsl_replay::NodeKey, Failure> {
    graph
        .key_of_value(value)
        .map_err(|message| Failure::decode(pointer, message))
}

fn source(value: &Value, pointer: &str) -> Result<(String, String), Failure> {
    Ok((
        text(&value["authority"], &format!("{pointer}/authority"))?.to_owned(),
        text(&value["identity"], &format!("{pointer}/identity"))?.to_owned(),
    ))
}

fn sources(value: &Value, pointer: &str) -> Result<Vec<(String, String)>, Failure> {
    array(value, pointer)?
        .iter()
        .enumerate()
        .map(|(index, value)| source(value, &format!("{pointer}/{index}")))
        .collect()
}

fn name(value: &Value, pointer: &str) -> Result<qsl_replay::QualifiedName, Failure> {
    let segments = text(value, pointer)?
        .split('.')
        .map(|segment| {
            qsl_replay::Identifier::new(segment)
                .map_err(|_| Failure::decode(pointer, "invalid qualified name"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    qsl_replay::QualifiedName::new(segments)
        .map_err(|_| Failure::decode(pointer, "empty qualified name"))
}

fn optional<T>(
    value: &Value,
    read: impl FnOnce(&Value) -> Result<T, Failure>,
) -> Result<Option<T>, Failure> {
    if value.is_null() {
        Ok(None)
    } else {
        read(value).map(Some)
    }
}

/// Read the closed corpus request encoding into the public request carrier;
/// package, target and semantic admission remain with the native request reader.
#[ix_trace_rs::string_edge]
fn decode(value: &Value, inputs: &Inputs<'_>, root: &Path) -> Result<ReplayRequestWire, Failure> {
    let package = &value["package"];
    let graph = qsl_replay::WireGraph::of_package_value(package)
        .map_err(|message| Failure::decode("/package", message))?;
    let target = &value["replay_target"];
    let members = &target["members"];
    let mut target_wire = RequestTargetWire {
        members: qsl_replay::ReplayTargetMembers {
            function: members
                .get("function")
                .map(|value| name(value, "/replay_target/members/function"))
                .transpose()?,
            function_owner: members
                .get("function_owner")
                .map(|value| {
                    qsl_replay::LibraryName::new(text(
                        value,
                        "/replay_target/members/function_owner",
                    )?)
                    .map_err(|_| {
                        Failure::decode(
                            "/replay_target/members/function_owner",
                            "invalid library identity",
                        )
                    })
                })
                .transpose()?,
            frame_operation: members
                .get("frame_operation")
                .map(|value| {
                    Ok(qsl_replay::FrameOperation {
                        object: name(
                            &value["object"],
                            "/replay_target/members/frame_operation/object",
                        )?,
                        operation: qsl_replay::Identifier::new(text(
                            &value["operation"],
                            "/replay_target/members/frame_operation/operation",
                        )?)
                        .map_err(|_| {
                            Failure::decode(
                                "/replay_target/members/frame_operation/operation",
                                "invalid identifier",
                            )
                        })?,
                    })
                })
                .transpose()?,
            state_clause: members
                .get("state_clause")
                .map(|value| name(value, "/replay_target/members/state_clause"))
                .transpose()?,
        },
        arguments: target["arguments"]
            .as_bool()
            .ok_or_else(|| Failure::decode("/replay_target/arguments", "expected a Boolean"))?,
        occurrence_key: optional(&target["occurrence_key"], |value| {
            Ok(qsl_replay::OccurrenceKey::new(
                node(
                    &value["node_id"],
                    "/replay_target/occurrence_key/node_id",
                    &graph,
                )?,
                qsl_replay::Origin::new(
                    qsl_replay::Role::new(text(
                        &value["role"],
                        "/replay_target/occurrence_key/role",
                    )?),
                    unsigned(&value["ordinal"], "/replay_target/occurrence_key/ordinal")?,
                ),
            ))
        })?,
        over: optional(&target["over"], |value| {
            Ok(qsl_replay::ReplayOverBinding {
                population: text(&value["population"], "/replay_target/over/population")?
                    .to_owned(),
                key: text(&value["key"], "/replay_target/over/key")?.to_owned(),
            })
        })?,
    };
    // The public request reader, rather than this adapter, decides missing or
    // conflicting target members and whether an occurrence is applicable.
    let source_value = &value["source"];
    let replay_source = match text(&source_value["kind"], "/source/kind")? {
        "witness" => ReplaySource::Witness(
            qsl_replay::Witness::parse(text(&source_value["transcript"], "/source/transcript")?)
                .map_err(|error| Failure::decode("/source/transcript", error.to_string()))?,
        ),
        "input" => ReplaySource::Input(
            array(&source_value["assignments"], "/source/assignments")?
                .iter()
                .enumerate()
                .map(|(index, assignment)| {
                    Ok(qsl_replay::CanonicalAssignment {
                        parameter: node(
                            &assignment["parameter"],
                            &format!("/source/assignments/{index}/parameter"),
                            &graph,
                        )?,
                        value: witness(
                            &assignment["value"],
                            &format!("/source/assignments/{index}/value"),
                            &graph,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, Failure>>()?,
        ),
        _ => return Err(Failure::decode("/source/kind", "unknown replay source")),
    };
    let mut stage_limits = BTreeMap::new();
    for (setting, bound) in object(&value["stage_limits"], "/stage_limits")? {
        let pointer = format!("/stage_limits/{setting}");
        let bound = unsigned(bound, &pointer)?;
        qsl_replay::StageLimits::default()
            .with_named(setting, bound)
            .map_err(|error| Failure::decode(&pointer, error.to_string()))?;
        stage_limits.insert(setting.clone(), bound);
    }
    let mut accounting = qsl_replay::AccountingLimits::default();
    for (counter, bound) in object(&value["accounting_limits"], "/accounting_limits")? {
        let Some(SettingName::Accounting(setting)) =
            SettingName::from_name(&format!("accounting.{counter}"))
        else {
            return Err(Failure::decode(
                format!("/accounting_limits/{counter}"),
                "unknown counter",
            ));
        };
        accounting.set_bound(
            setting,
            unsigned(bound, &format!("/accounting_limits/{counter}"))?,
        );
    }
    let supplied_limits = optional(&value["supplied_limits"], |value| {
        let parsed = object(value, "/supplied_limits")?
            .iter()
            .map(|(name, value)| {
                Ok((
                    name.clone(),
                    json!(unsigned(value, &format!("/supplied_limits/{name}"))?),
                ))
            })
            .collect::<Result<serde_json::Map<_, _>, Failure>>()?;
        qsl_replay::read_supplied_limits(&Value::Object(parsed))
            .map_err(|error| Failure::decode("/supplied_limits", error.to_string()))
    })?;
    let obligation = &value["obligation_identity"];
    let ordinal = obligation["ordinal"]
        .as_u64()
        .and_then(|ordinal| u32::try_from(ordinal).ok())
        .ok_or_else(|| Failure::decode("/obligation_identity/ordinal", "ordinal outside u32"))?;
    let mut byte_provision = Vec::new();
    for (index, provision) in array(&value["byte_provision"], "/byte_provision")?
        .iter()
        .enumerate()
    {
        let pointer = format!("/byte_provision/{index}/bytes_path");
        let path = text(&provision["bytes_path"], &pointer)?;
        let relative = Path::new(path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(Failure::decode(
                &pointer,
                "byte path is not relative to the corpus",
            ));
        }
        let bytes = inputs
            .files
            .iter()
            .find(|(file, _)| file.kind == InputKind::Bytes && file.path == path)
            .map(|(_, bytes)| bytes.clone())
            .ok_or_else(|| {
                Failure::decode(&pointer, "byte path is not a listed, readable bytes input")
            })?;
        let resolved = std::fs::canonicalize(root.join(relative))
            .map_err(|error| Failure::decode(&pointer, error.to_string()))?;
        if !resolved.starts_with(root) {
            return Err(Failure::decode(&pointer, "byte path escapes the corpus"));
        }
        byte_provision.push((
            text(
                &provision["authority"],
                &format!("/byte_provision/{index}/authority"),
            )?
            .to_owned(),
            text(
                &provision["identity"],
                &format!("/byte_provision/{index}/identity"),
            )?
            .to_owned(),
            bytes,
        ));
    }
    let dependencies = array(&value["dependencies"], "/dependencies")?
        .iter()
        .enumerate()
        .map(|(index, dependency)| {
            Ok(qsl_replay::DependencyEntryWire {
                identity: text(
                    &dependency["identity"],
                    &format!("/dependencies/{index}/identity"),
                )?
                .to_owned(),
                sources: sources(
                    &dependency["sources"],
                    &format!("/dependencies/{index}/sources"),
                )?,
            })
        })
        .collect::<Result<_, Failure>>()?;
    let state_environment = array(&value["state_environment"], "/state_environment")?
        .iter()
        .enumerate()
        .map(|(index, pair)| {
            let pair = array(pair, &format!("/state_environment/{index}"))?;
            let [name, value] = pair else {
                return Err(Failure::decode(
                    format!("/state_environment/{index}"),
                    "expected a string pair",
                ));
            };
            Ok((
                text(name, "/state_environment")?.to_owned(),
                text(value, "/state_environment")?.to_owned(),
            ))
        })
        .collect::<Result<_, Failure>>()?;
    Ok(ReplayRequestWire {
        package_name: text(&package["name"], "/package/name")?.to_owned(),
        profile_selections: array(&value["profile_selections"], "/profile_selections")?
            .iter()
            .map(|selection| {
                Ok(qsl_replay::ProfileSelection::new(
                    text(&selection["profile"], "/profile_selections/profile")?.to_owned(),
                    text(&selection["value"], "/profile_selections/value")?.to_owned(),
                ))
            })
            .collect::<Result<_, Failure>>()?,
        sources: sources(&package["lock"]["sources"], "/package/lock/sources")?,
        dependencies,
        replay_target: std::mem::take(&mut target_wire),
        source: replay_source,
        obligation_identity: qsl_replay::ObligationIdentity::new(
            node(
                &obligation["node_id"],
                "/obligation_identity/node_id",
                &graph,
            )?,
            optional(&obligation["model"], |value| {
                text(value, "/obligation_identity/model").map(qsl_replay::ModelIdentity::new)
            })?,
            ordinal,
        ),
        backend: text(&value["backend"], "/backend")?.to_owned(),
        state_environment: qsl_replay::StateEnvironment::new(state_environment),
        accounting_limits: accounting.0,
        stage_limits,
        supplied_limits,
        object_environment: optional(&value["object_environment"], |value| {
            qsl_replay::read_object_environment(value)
                .map_err(|error| Failure::decode("/object_environment", error.to_string()))
        })?,
        declared_domains: array(&value["declared_domains"], "/declared_domains")?
            .iter()
            .enumerate()
            .map(|(index, domain)| {
                declared_domain(domain, &format!("/declared_domains/{index}"), &graph)
            })
            .collect::<Result<_, _>>()?,
        byte_provision,
        documents: inputs
            .files
            .iter()
            .filter(|(file, _)| {
                matches!(
                    file.kind,
                    InputKind::Model | InputKind::ObjectContext | InputKind::Population
                )
            })
            .map(|(_, bytes)| bytes.clone())
            .collect(),
    })
}

/// Decode corpus domain/bound tags directly into native DomainKey, DomainKind
/// and FiniteBound; their owner constructors decide validity.
#[ix_trace_rs::string_edge]
fn declared_domain(
    value: &Value,
    pointer: &str,
    graph: &qsl_replay::WireGraph,
) -> Result<DeclaredDomain, Failure> {
    let key = &value["domain"];
    let integer32 = |value, pointer: &str| {
        u32::try_from(unsigned(value, pointer)?)
            .map_err(|_| Failure::decode(pointer, "value outside u32"))
    };
    let domain = match text(&key["kind"], pointer)? {
        "node" => DomainKey::Node {
            node: node(&key["node"], pointer, graph)?,
            path: array(&key["path"], pointer)?
                .iter()
                .map(|value| integer32(value, pointer))
                .collect::<Result<_, _>>()?,
        },
        "population" => DomainKey::Population {
            member_type: node(&key["member_type"], pointer, graph)?,
            ordinal: integer32(&key["ordinal"], pointer)?,
        },
        _ => return Err(Failure::decode(pointer, "unknown domain key")),
    };
    let kind = optional(&value["kind"], |kind| {
        DomainKind::from_wire(&text(&kind["kind"], pointer)?.replace('_', "-"))
            .ok_or_else(|| Failure::decode(pointer, "unknown domain kind"))
    })?;
    let bound = &value["bound"];
    let bound = match text(&bound["kind"], pointer)? {
        "cardinality" => FiniteBound::cardinality(unsigned(&bound["maximum"], pointer)?),
        "depth" => FiniteBound::depth(unsigned(&bound["maximum"], pointer)?)
            .map_err(|error| Failure::decode(pointer, error.to_string()))?,
        "integer_range" => FiniteBound::integer_range(
            integer(&bound["lower"], pointer)?,
            integer(&bound["upper"], pointer)?,
        )
        .map_err(|error| Failure::decode(pointer, error.to_string()))?,
        "variants" => FiniteBound::variants(
            array(&bound["members"], pointer)?
                .iter()
                .map(|member| text(member, pointer).map(str::to_owned))
                .collect::<Result<Vec<_>, _>>()?,
        )
        .map_err(|error| Failure::decode(pointer, error.to_string()))?,
        _ => return Err(Failure::decode(pointer, "unknown finite bound")),
    };
    ProofBound::new(domain, kind, bound)
        .map(DeclaredDomain::new)
        .map_err(|error| Failure::decode(pointer, error.to_string()))
}

fn integer(value: &Value, pointer: &str) -> Result<Integer, Failure> {
    text(value, pointer)?
        .parse()
        .map_err(|error| Failure::decode(pointer, format!("{error}")))
}

/// Transient corpus grammar tag. It carries no semantic value or identity;
/// the owner codec admits the completed native wire after iterative assembly.
#[derive(Clone, Copy, Eq, PartialEq)]
enum WitnessForm {
    Boolean,
    Integer,
    ExactInteger,
    Enum,
    Text,
    String,
    Uuid,
    Timestamp,
    Rational,
    Decimal,
    Float32,
    Float64,
    Reference,
    Quantity,
    Option,
    Record,
    Tuple,
    Union,
    Collection(quire_exact::CollectionKind),
}

/// Read the corpus witness discriminator once. Subsequent traversal and
/// encoding use this typed parser state, then the native owner codec.
#[ix_trace_rs::string_edge]
fn witness_form(value: &Value, pointer: &str) -> Result<WitnessForm, Failure> {
    use quire_exact::CollectionKind;
    Ok(match text(&value["kind"], pointer)? {
        "boolean" => WitnessForm::Boolean,
        "integer" => WitnessForm::Integer,
        "exact_integer" => WitnessForm::ExactInteger,
        "enum" => WitnessForm::Enum,
        "text" => WitnessForm::Text,
        "string" => WitnessForm::String,
        "uuid" => WitnessForm::Uuid,
        "timestamp" => WitnessForm::Timestamp,
        "rational" => WitnessForm::Rational,
        "decimal" => WitnessForm::Decimal,
        "float32" => WitnessForm::Float32,
        "float64" => WitnessForm::Float64,
        "reference" => WitnessForm::Reference,
        "quantity" => WitnessForm::Quantity,
        "option" => WitnessForm::Option,
        "record" => WitnessForm::Record,
        "tuple" => WitnessForm::Tuple,
        "union" => WitnessForm::Union,
        "sequence" => WitnessForm::Collection(CollectionKind::Sequence),
        "ordered_set" => WitnessForm::Collection(CollectionKind::OrderedSet),
        "set" => WitnessForm::Collection(CollectionKind::Set),
        "bag" => WitnessForm::Collection(CollectionKind::Bag),
        _ => return Err(Failure::decode(pointer, "unknown witness value")),
    })
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SlotForm {
    Present,
    Absent,
    Null,
}

/// Read a record slot's corpus tag once, retaining it until its child has
/// converted; semantic presence still belongs to the native witness reader.
#[ix_trace_rs::string_edge]
fn slot_form(value: &Value, pointer: &str) -> Result<SlotForm, Failure> {
    Ok(match text(&value["kind"], pointer)? {
        "present" => SlotForm::Present,
        "absent" => SlotForm::Absent,
        "null" => SlotForm::Null,
        _ => return Err(Failure::decode(pointer, "unknown slot")),
    })
}

fn witness(
    value: &Value,
    pointer: &str,
    graph: &qsl_replay::WireGraph,
) -> Result<WitnessValue, Failure> {
    let form = witness_form(value, pointer)?;
    match form {
        WitnessForm::Boolean => {
            Ok(WitnessValue::Boolean(value["value"].as_bool().ok_or_else(
                || Failure::decode(pointer, "expected a Boolean"),
            )?))
        }
        WitnessForm::Integer => Ok(WitnessValue::Integer(
            text(&value["decimal"], pointer)?
                .parse()
                .map_err(|_| Failure::decode(pointer, "integer outside i128"))?,
        )),
        WitnessForm::ExactInteger => Ok(WitnessValue::ExactInteger(integer(
            &value["decimal"],
            pointer,
        )?)),
        _ => {
            let typed = witness_json(value, pointer, graph, form)?;
            let bytes = quire_canonical::to_vec(&*typed, quire_canonical::Limits::new(u64::MAX))
                .map_err(|error| Failure::decode(pointer, error.to_string()))?;
            WitnessValue::from_wire_bytes(&bytes)
                .map_err(|error| Failure::decode(pointer, format!("{error:?}")))
        }
    }
}

fn node_text(
    value: &Value,
    pointer: &str,
    graph: &qsl_replay::WireGraph,
) -> Result<String, Failure> {
    node_key_to_text(&node(value, pointer, graph)?)
        .map_err(|error| Failure::decode(pointer, error.to_string()))
}

fn wire_object<const N: usize>(members: [(&str, Value); N]) -> Value {
    Value::Object(
        members
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn witness_json(
    value: &Value,
    pointer: &str,
    graph: &qsl_replay::WireGraph,
    form: WitnessForm,
) -> Result<super::super::DeepValue, Failure> {
    use super::super::DeepValue;
    enum Task<'a> {
        Read(&'a Value),
        Parsed(&'a Value, WitnessForm),
        Finish(&'a Value, usize, Option<String>, WitnessForm, Vec<SlotForm>),
    }
    let mut tasks = vec![Task::Parsed(value, form)];
    let mut values = Vec::<DeepValue>::new();
    while let Some(task) = tasks.pop() {
        match task {
            Task::Read(value) => {
                tasks.push(Task::Parsed(value, witness_form(value, pointer)?));
                continue;
            }
            Task::Parsed(value, form) => {
                let mut slots = Vec::new();
                // Tuple/union identity was read before their components by
                // the owner adapter. Preserve that refusal order while
                // moving the recursive walk onto the explicit stack.
                let declaration = if matches!(form, WitnessForm::Tuple | WitnessForm::Union) {
                    Some(node_text(&value["declaration"], pointer, graph)?)
                } else {
                    None
                };
                let children = match form {
                    WitnessForm::Quantity => vec![&value["magnitude"]],
                    WitnessForm::Option => {
                        if value["value"].is_null() {
                            Vec::new()
                        } else {
                            vec![&value["value"]]
                        }
                    }
                    WitnessForm::Record => {
                        let mut children = Vec::new();
                        for field in array(&value["fields"], pointer)? {
                            let slot = &field["slot"];
                            let form = slot_form(slot, pointer)?;
                            if form == SlotForm::Present {
                                children.push(&slot["value"]);
                            }
                            slots.push(form);
                        }
                        children
                    }
                    WitnessForm::Tuple | WitnessForm::Union => {
                        array(&value["components"], pointer)?.iter().collect()
                    }
                    WitnessForm::Collection(_) => {
                        array(&value["elements"], pointer)?.iter().collect()
                    }
                    _ => {
                        values.push(DeepValue::new(witness_leaf(value, pointer, graph, form)?));
                        continue;
                    }
                };
                tasks.push(Task::Finish(
                    value,
                    children.len(),
                    declaration,
                    form,
                    slots,
                ));
                tasks.extend(children.into_iter().rev().map(Task::Read));
            }
            Task::Finish(value, count, declaration, form, slots) => {
                let start = values
                    .len()
                    .checked_sub(count)
                    .ok_or_else(|| Failure::decode(pointer, "missing witness children"))?;
                let mut children = values.split_off(start).into_iter();
                let next = |children: &mut std::vec::IntoIter<DeepValue>| {
                    children
                        .next()
                        .map(DeepValue::into_inner)
                        .ok_or_else(|| Failure::decode(pointer, "missing witness child"))
                };
                let converted = match form {
                    WitnessForm::Quantity => {
                        let mut magnitude = DeepValue::new(next(&mut children)?);
                        let unit = format!(
                            "{{\"declared\":{}}}",
                            node_text(&value["unit"], pointer, graph)?
                        );
                        magnitude
                            .0
                            .as_object_mut()
                            .ok_or_else(|| Failure::decode(pointer, "invalid magnitude"))?
                            .insert("unit".to_owned(), Value::String(unit));
                        magnitude.into_inner()
                    }
                    WitnessForm::Option => wire_object([
                        ("type", json!("option")),
                        (
                            "value",
                            if count == 0 {
                                Value::Null
                            } else {
                                next(&mut children)?
                            },
                        ),
                    ]),
                    WitnessForm::Record => {
                        let name = node_text(&value["declaration"], pointer, graph)?;
                        let mut fields = Vec::new();
                        for (field, slot) in array(&value["fields"], pointer)?.iter().zip(slots) {
                            let converted = match slot {
                                SlotForm::Present => next(&mut children)?,
                                SlotForm::Absent => Value::Null,
                                SlotForm::Null => json!({"type":"null"}),
                            };
                            fields.push(DeepValue::new(wire_object([
                                ("name", field["name"].clone()),
                                ("value", converted),
                            ])));
                        }
                        wire_object([
                            ("type", json!("record")),
                            ("name", Value::String(name)),
                            (
                                "fields",
                                Value::Array(
                                    fields.into_iter().map(DeepValue::into_inner).collect(),
                                ),
                            ),
                        ])
                    }
                    WitnessForm::Tuple | WitnessForm::Union => {
                        let name = declaration.ok_or_else(|| {
                            Failure::decode(pointer, "missing witness declaration")
                        })?;
                        let components =
                            Value::Array(children.map(DeepValue::into_inner).collect());
                        let mut members = serde_json::Map::new();
                        members.insert(
                            "type".into(),
                            json!(if form == WitnessForm::Union {
                                "union"
                            } else {
                                "tuple"
                            }),
                        );
                        members.insert("name".into(), Value::String(name));
                        if form == WitnessForm::Union {
                            members.insert("member".into(), value["member"].clone());
                        }
                        members.insert("components".into(), components);
                        Value::Object(members)
                    }
                    WitnessForm::Collection(kind) => wire_object([
                        (
                            "type",
                            json!(match kind {
                                quire_exact::CollectionKind::Sequence => "sequence",
                                quire_exact::CollectionKind::OrderedSet => "ordered-set",
                                quire_exact::CollectionKind::Set => "set",
                                quire_exact::CollectionKind::Bag => "bag",
                            }),
                        ),
                        (
                            "elements",
                            Value::Array(children.map(DeepValue::into_inner).collect()),
                        ),
                    ]),
                    _ => return Err(Failure::decode(pointer, "unknown composite witness value")),
                };
                values.push(DeepValue::new(converted));
            }
        }
    }
    values
        .pop()
        .ok_or_else(|| Failure::decode(pointer, "missing witness value"))
}

fn witness_leaf(
    value: &Value,
    pointer: &str,
    graph: &qsl_replay::WireGraph,
    form: WitnessForm,
) -> Result<Value, Failure> {
    Ok(match form {
        WitnessForm::Boolean => json!({"type":"boolean", "value":value["value"]}),
        WitnessForm::Integer | WitnessForm::ExactInteger => {
            json!({"type":"integer", "value":value["decimal"]})
        }
        WitnessForm::Enum => {
            json!({"type":"enum", "name":node_text(&value["declaration"], pointer, graph)?, "member":value["member"]})
        }
        WitnessForm::Text => json!({"type":"text", "value":value["value"]}),
        WitnessForm::String | WitnessForm::Uuid => {
            json!({"kind":if form == WitnessForm::String { "string" } else { "uuid" },"text":value["value"]})
        }
        WitnessForm::Timestamp => json!({"kind":"timestamp","nanoseconds":value["decimal"]}),
        WitnessForm::Rational => {
            json!({"type":"rational","numerator":value["numerator"],"denominator":value["denominator"]})
        }
        WitnessForm::Decimal => {
            json!({"type":"decimal","coefficient":value["coefficient"],"scale":value["scale"]})
        }
        WitnessForm::Float32 => {
            json!({"type":"float32", "bits":format!("{:08x}", u32::try_from(unsigned(&value["bits"], pointer)?).map_err(|_| Failure::decode(pointer,"float bits outside u32"))?)})
        }
        WitnessForm::Float64 => {
            json!({"type":"float64", "bits":format!("{:016x}", unsigned(&value["bits"], pointer)?)})
        }
        WitnessForm::Reference => {
            let type_node = node(&value["object_type"], pointer, graph)?;
            let path = super::super::DeepValue::new(
                node_key_to_json(&type_node)
                    .map_err(|error| Failure::decode(pointer, error.to_string()))?,
            );
            let original = path["path"]
                .as_array()
                .and_then(|path| path.last())
                .and_then(Value::as_str)
                .ok_or_else(|| Failure::decode(pointer, "reference type is not a declared type"))?;
            json!({"type":"reference", "universe":node_text(&value["universe"], pointer, graph)?, "object_type":serde_json::to_string(&json!([{"package":type_node.package(),"node":original}])).map_err(|error| Failure::decode(pointer,error.to_string()))?, "identity":value["identity"]})
        }
        _ => return Err(Failure::decode(pointer, "unknown witness value")),
    })
}

fn replay_value(result: &qsl_replay::ReplayResult) -> Result<Value, Failure> {
    let row = result
        .to_function_json()
        .map_err(|error| Failure::decode("", error.to_string()))?;
    let row = super::super::parse_unique_json(row.as_bytes())
        .map_err(|error| Failure::decode("", error))?;
    Ok(wire_object([("results", Value::Array(vec![row]))]))
}

#[cfg(test)]
mod depth_tests {
    use ix_trace_rs::trace;

    #[trace("TC-1325", "FR-350-AC-11")]
    #[test]
    fn typed_wire_states_retain_slots_collections_and_declaration_refusal_order() {
        use qsl_replay::{WitnessSlot, WitnessValue};
        use serde_json::json;
        // A structural codec fixture, not a claim about a checked declaration.
        // The executor still owns declaration-kind and value-type admission.
        let key = quire_exact::NodeKey::new(
            "fixture",
            vec![quire_exact::PathSegment::Text("shape".into())],
            0,
        )
        .unwrap();
        let id = qsl_semantics::check::wire_id::structural_id(&key);
        let graph = qsl_replay::WireGraph::new([&id]).unwrap();
        let declaration = graph.wire_value(&key).unwrap();
        let record = json!({"kind":"record", "declaration":declaration, "fields":[
            {"name":"held", "slot":{"kind":"present", "value":{"kind":"boolean", "value":true}}},
            {"name":"omitted", "slot":{"kind":"absent"}},
            {"name":"nullish", "slot":{"kind":"null"}}
        ]});
        let native = super::witness(&record, "/value", &graph)
            .unwrap_or_else(|failure| panic!("{}", failure.message));
        let WitnessValue::Record {
            declaration: actual,
            fields,
        } = &native
        else {
            panic!("record form is retained")
        };
        assert_eq!(actual, &key);
        assert_eq!(
            fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            ["held", "omitted", "nullish"]
        );
        assert!(matches!(
            fields[0].slot,
            WitnessSlot::Present(WitnessValue::Boolean(true))
        ));
        assert!(matches!(fields[1].slot, WitnessSlot::Absent));
        assert!(matches!(fields[2].slot, WitnessSlot::Null));
        for (tag, expected) in [
            (
                "sequence",
                WitnessValue::Sequence(vec![
                    WitnessValue::Boolean(false),
                    WitnessValue::Boolean(true),
                ]),
            ),
            (
                "ordered_set",
                WitnessValue::OrderedSet(vec![
                    WitnessValue::Boolean(false),
                    WitnessValue::Boolean(true),
                ]),
            ),
            (
                "set",
                WitnessValue::Set(vec![
                    WitnessValue::Boolean(false),
                    WitnessValue::Boolean(true),
                ]),
            ),
            (
                "bag",
                WitnessValue::Bag(vec![
                    WitnessValue::Boolean(false),
                    WitnessValue::Boolean(true),
                ]),
            ),
        ] {
            let value = json!({"kind":tag,"elements":[{"kind":"boolean","value":false},{"kind":"boolean","value":true}]});
            let actual = super::witness(&value, "/value", &graph)
                .unwrap_or_else(|failure| panic!("{}", failure.message));
            assert!(
                actual == expected,
                "the native collection form and order survive"
            );
        }
        let unknown_declaration = serde_json::Value::Null;
        let expected = super::node(&unknown_declaration, "/value", &graph)
            .err()
            .unwrap();
        for value in [
            json!({"kind":"tuple", "declaration":unknown_declaration, "components":[{"kind":"unknown"}]}),
            json!({"kind":"union", "declaration":unknown_declaration, "member":"Arm", "components":[{"kind":"unknown"}]}),
        ] {
            let refused = super::witness(&value, "/value", &graph).err().unwrap();
            assert_eq!(
                refused.message, expected.message,
                "tuple and union declarations still refuse before child intake"
            );
        }
        let mut record = record;
        record["declaration"] = unknown_declaration;
        record["fields"][0]["slot"]["value"] = json!({"kind":"unknown"});
        let refused = super::witness(&record, "/value", &graph).err().unwrap();
        assert_eq!(
            refused.message, "unknown witness value",
            "record declaration remains after its children"
        );
    }

    #[trace("TC-1325", "FR-350-AC-11")]
    #[test]
    fn a_deep_corpus_witness_converts_and_releases_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let depth = 1_000;
                let mut bytes = br#"{"kind":"sequence","elements":["#.repeat(depth);
                bytes
                    .extend_from_slice(br#"{"kind":"exact_integer","decimal":"9007199254740993"}"#);
                bytes.extend_from_slice(&b"]}".repeat(depth));
                let value = super::super::super::DeepValue::new(
                    super::super::super::parse_unique_json(&bytes)
                        .expect("the byte-bounded witness reads past 128 levels"),
                );
                let graph = qsl_replay::WireGraph::new([]).expect("an unused, empty key lookup");
                let witness = super::witness(&value, "/source/assignments/0/value", &graph)
                    .unwrap_or_else(|_| panic!("the owner admits the complete typed witness"));
                let mut current = &witness;
                for _ in 0..depth {
                    let qsl_replay::WitnessValue::Sequence(elements) = current else {
                        panic!("every sequence layer is retained");
                    };
                    assert_eq!(elements.len(), 1);
                    current = &elements[0];
                }
                let qsl_replay::WitnessValue::ExactInteger(value) = current else {
                    panic!("the leaf remains an exact integer");
                };
                assert_eq!(value.to_string(), "9007199254740993");
                // Both the corpus JSON guard and the native witness release
                // their complete deep trees on this same small stack.
                drop(witness);
            })
            .expect("spawn the bounded-stack qualification control")
            .join()
            .expect("deep witness conversion and release do not overflow");
    }

    #[trace("TC-1325", "FR-350-AC-11")]
    #[test]
    fn malformed_typed_witnesses_at_depth_keep_the_decode_partition() {
        let qspec = std::env::var_os("QSPEC_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .ancestors()
                    .map(|parent| parent.join("quire-specification"))
                    .find(|path| path.is_dir())
                    .expect("a QSpec checkout or QSPEC_DIR")
            });
        let mut schema: serde_json::Value = serde_json::from_slice(
            &std::fs::read(qspec.join("proposals/conformance-corpus-v1/schema.json"))
                .expect("the published corpus schema"),
        )
        .expect("the selected schema is JSON");
        let alternatives = schema["$defs"]["WitnessValue"]["oneOf"]
            .as_array()
            .expect("the published discriminated alternatives")
            .len();
        assert_eq!(
            super::super::guard_witness_alternatives(&mut schema),
            alternatives
        );
        let schema = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .with_document("urn:quire:conformance-corpus:v1".to_owned(), schema)
            .compile(
                &serde_json::json!({"$ref":"urn:quire:conformance-corpus:v1#/$defs/WitnessValue"}),
            )
            .expect("the actual published witness schema compiles");
        let mut valid = br#"{"kind":"sequence","elements":["#.repeat(192);
        valid.extend_from_slice(br#"{"kind":"boolean","value":true}"#);
        valid.extend_from_slice(&b"]}".repeat(192));
        let admitted = super::super::document(&valid, &schema)
            .unwrap_or_else(|_| panic!("a schema-valid deep witness remains admitted"));
        drop(super::super::super::DeepValue::new(admitted));
        let mut bytes = br#"{"kind":"sequence","elements":["#.repeat(192);
        bytes.extend_from_slice(br#"{"kind":"boolean","value":"not-a-boolean"}"#);
        bytes.extend_from_slice(&b"]}".repeat(192));
        let failure = match super::super::document(&bytes, &schema) {
            Ok(value) => {
                drop(super::super::super::DeepValue::new(value));
                panic!("a mistyped witness leaf must refuse after deep JSON admission");
            }
            Err(failure) => failure,
        };
        assert_eq!(failure.cause, Some("corpus-decode"));
        assert!(!failure.message.contains("recursion limit"));
    }
}

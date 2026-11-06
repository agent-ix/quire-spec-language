// SPDX-License-Identifier: AGPL-3.0-or-later
//! Decode the corpus replay adapter into the existing typed replay API.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use qsl_foundation::node_wire::{node_key_to_json, node_key_to_text};
use qsl_foundation::SettingName;
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

fn witness(
    value: &Value,
    pointer: &str,
    graph: &qsl_replay::WireGraph,
) -> Result<WitnessValue, Failure> {
    match text(&value["kind"], pointer)? {
        "boolean" => {
            Ok(WitnessValue::Boolean(value["value"].as_bool().ok_or_else(
                || Failure::decode(pointer, "expected a Boolean"),
            )?))
        }
        "integer" => Ok(WitnessValue::Integer(
            text(&value["decimal"], pointer)?
                .parse()
                .map_err(|_| Failure::decode(pointer, "integer outside i128"))?,
        )),
        "exact_integer" => Ok(WitnessValue::ExactInteger(integer(
            &value["decimal"],
            pointer,
        )?)),
        _ => {
            let typed = witness_json(value, pointer, graph)?;
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
) -> Result<super::super::DeepValue, Failure> {
    use super::super::DeepValue;
    enum Task<'a> {
        Read(&'a Value),
        Finish(&'a Value, usize, Option<String>),
    }
    let mut tasks = vec![Task::Read(value)];
    let mut values = Vec::<DeepValue>::new();
    while let Some(task) = tasks.pop() {
        match task {
            Task::Read(value) => {
                let kind = text(&value["kind"], pointer)?;
                // Tuple/union identity was read before their components by
                // the owner adapter. Preserve that refusal order while
                // moving the recursive walk onto the explicit stack.
                let declaration = if matches!(kind, "tuple" | "union") {
                    Some(node_text(&value["declaration"], pointer, graph)?)
                } else {
                    None
                };
                let children = match kind {
                    "quantity" => vec![&value["magnitude"]],
                    "option" => {
                        if value["value"].is_null() {
                            Vec::new()
                        } else {
                            vec![&value["value"]]
                        }
                    }
                    "record" => {
                        let mut children = Vec::new();
                        for field in array(&value["fields"], pointer)? {
                            let slot = &field["slot"];
                            match text(&slot["kind"], pointer)? {
                                "present" => children.push(&slot["value"]),
                                "absent" | "null" => {}
                                _ => return Err(Failure::decode(pointer, "unknown slot")),
                            }
                        }
                        children
                    }
                    "tuple" | "union" => array(&value["components"], pointer)?.iter().collect(),
                    "sequence" | "ordered_set" | "set" | "bag" => {
                        array(&value["elements"], pointer)?.iter().collect()
                    }
                    _ => {
                        values.push(DeepValue::new(witness_leaf(value, pointer, graph)?));
                        continue;
                    }
                };
                tasks.push(Task::Finish(value, children.len(), declaration));
                tasks.extend(children.into_iter().rev().map(Task::Read));
            }
            Task::Finish(value, count, declaration) => {
                let start = values
                    .len()
                    .checked_sub(count)
                    .ok_or_else(|| Failure::decode(pointer, "missing witness children"))?;
                let mut children = values.split_off(start).into_iter();
                let kind = text(&value["kind"], pointer)?;
                let next = |children: &mut std::vec::IntoIter<DeepValue>| {
                    children
                        .next()
                        .map(DeepValue::into_inner)
                        .ok_or_else(|| Failure::decode(pointer, "missing witness child"))
                };
                let converted = match kind {
                    "quantity" => {
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
                    "option" => wire_object([
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
                    "record" => {
                        let name = node_text(&value["declaration"], pointer, graph)?;
                        let mut fields = Vec::new();
                        for field in array(&value["fields"], pointer)? {
                            let slot = &field["slot"];
                            let converted = match text(&slot["kind"], pointer)? {
                                "present" => next(&mut children)?,
                                "absent" => json!({"type":"absent"}),
                                "null" => json!({"type":"null"}),
                                _ => return Err(Failure::decode(pointer, "unknown slot")),
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
                    "tuple" | "union" => {
                        let name = declaration.ok_or_else(|| {
                            Failure::decode(pointer, "missing witness declaration")
                        })?;
                        let components =
                            Value::Array(children.map(DeepValue::into_inner).collect());
                        let mut members = serde_json::Map::new();
                        members.insert("type".into(), json!(kind));
                        members.insert("name".into(), Value::String(name));
                        if kind == "union" {
                            members.insert("member".into(), value["member"].clone());
                        }
                        members.insert("components".into(), components);
                        Value::Object(members)
                    }
                    "sequence" | "ordered_set" | "set" | "bag" => wire_object([
                        ("type", json!(kind.replace('_', "-"))),
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
) -> Result<Value, Failure> {
    let kind = text(&value["kind"], pointer)?;
    Ok(match kind {
        "boolean" => json!({"type":"boolean", "value":value["value"]}),
        "integer" | "exact_integer" => json!({"type":"integer", "value":value["decimal"]}),
        "enum" => {
            json!({"type":"enum", "name":node_text(&value["declaration"], pointer, graph)?, "member":value["member"]})
        }
        "text" => json!({"type":"text", "value":value["value"]}),
        "string" | "uuid" => json!({"kind":kind,"text":value["value"]}),
        "timestamp" => json!({"kind":"timestamp","nanoseconds":value["decimal"]}),
        "rational" => {
            json!({"type":"rational","numerator":value["numerator"],"denominator":value["denominator"]})
        }
        "decimal" => {
            json!({"type":"decimal","coefficient":value["coefficient"],"scale":value["scale"]})
        }
        "float32" => {
            json!({"type":"float32", "bits":format!("{:08x}", u32::try_from(unsigned(&value["bits"], pointer)?).map_err(|_| Failure::decode(pointer,"float bits outside u32"))?)})
        }
        "float64" => {
            json!({"type":"float64", "bits":format!("{:016x}", unsigned(&value["bits"], pointer)?)})
        }
        "reference" => {
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

use qsl_foundation::SettingLimits;

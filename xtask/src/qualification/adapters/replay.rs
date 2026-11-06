// SPDX-License-Identifier: AGPL-3.0-or-later
//! Decode the corpus replay adapter into the existing typed replay API.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use qsl_foundation::node_wire::node_key_to_json;
use qsl_foundation::SettingName;
use qsl_replay::{
    DeclaredDomain, DomainKey, DomainKind, FiniteBound, Integer, ProofBound, ReplayRequestWire,
    ReplaySource, RequestTargetWire, WitnessValue,
};
use serde::Serialize;
use serde_json::{json, Value};

use super::{
    array, document, encode, from_document, object, text, unsigned, Execution, Failure, InputKind,
    Inputs,
};

pub(super) fn execute(
    inputs: &Inputs<'_>,
    root: &Path,
    limits: &qsl_replay::CallerLimits,
    overrides: &[String],
    schema: &jsonschema::JSONSchema,
) -> Result<Execution, Failure> {
    let (_, bytes) = inputs.one(InputKind::ReplayRequest)?;
    let value = document(bytes, schema)?;
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
            WitnessValue::from_wire_json(&typed)
                .map_err(|error| Failure::decode(pointer, format!("{error:?}")))
        }
    }
}

fn witness_json(
    value: &Value,
    pointer: &str,
    graph: &qsl_replay::WireGraph,
) -> Result<Value, Failure> {
    let kind = text(&value["kind"], pointer)?;
    let id_text = |value: &Value| -> Result<String, Failure> {
        encode(
            node_key_to_json(&node(value, pointer, graph)?)
                .map_err(|error| Failure::decode(pointer, error.to_string()))?,
        )
        .and_then(|bytes| {
            String::from_utf8(bytes).map_err(|error| Failure::decode(pointer, error.to_string()))
        })
    };
    let children = |name: &str| {
        array(&value[name], pointer)?
            .iter()
            .map(|child| witness_json(child, pointer, graph))
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(match kind {
        "boolean" => json!({"type":"boolean", "value":value["value"]}),
        "integer" | "exact_integer" => json!({"type":"integer", "value":value["decimal"]}),
        "enum" => {
            json!({"type":"enum", "name":id_text(&value["declaration"])?, "member":value["member"]})
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
        "quantity" => {
            let mut magnitude = witness_json(&value["magnitude"], pointer, graph)?;
            magnitude.as_object_mut().ok_or_else(|| Failure::decode(pointer,"invalid magnitude"))?.insert("unit".into(), json!(serde_json::to_string(&json!({"declared":node_key_to_json(&node(&value["unit"], pointer, graph)?).map_err(|error| Failure::decode(pointer,error.to_string()))?})).map_err(|error| Failure::decode(pointer,error.to_string()))?));
            magnitude
        }
        "reference" => {
            let type_node = node(&value["object_type"], pointer, graph)?;
            let path = node_key_to_json(&type_node)
                .map_err(|error| Failure::decode(pointer, error.to_string()))?;
            let original = path["path"]
                .as_array()
                .and_then(|path| path.last())
                .and_then(Value::as_str)
                .ok_or_else(|| Failure::decode(pointer, "reference type is not a declared type"))?;
            json!({"type":"reference", "universe":id_text(&value["universe"])?, "object_type":serde_json::to_string(&json!([{"package":type_node.package(),"node":original}])).map_err(|error| Failure::decode(pointer,error.to_string()))?, "identity":value["identity"]})
        }
        "option" => {
            json!({"type":"option", "value":if value["value"].is_null() { Value::Null } else { witness_json(&value["value"],pointer,graph)? }})
        }
        "record" => {
            let fields = array(&value["fields"], pointer)?
                .iter()
                .map(|field| {
                    let slot = &field["slot"];
                    let converted = match text(&slot["kind"], pointer)? {
                        "present" => witness_json(&slot["value"], pointer, graph)?,
                        "absent" => json!({"type":"absent"}),
                        "null" => json!({"type":"null"}),
                        _ => return Err(Failure::decode(pointer, "unknown slot")),
                    };
                    Ok(json!({"name":field["name"],"value":converted}))
                })
                .collect::<Result<Vec<_>, Failure>>()?;
            json!({"type":"record","name":id_text(&value["declaration"])?,"fields":fields})
        }
        "tuple" => {
            json!({"type":"tuple","name":id_text(&value["declaration"])?,"components":children("components")?})
        }
        "union" => {
            json!({"type":"union","name":id_text(&value["declaration"])?,"member":value["member"],"components":children("components")?})
        }
        "sequence" | "ordered_set" | "set" | "bag" => {
            json!({"type":kind.replace('_',"-"),"elements":children("elements")?})
        }
        _ => return Err(Failure::decode(pointer, "unknown witness value")),
    })
}

#[derive(Serialize)]
struct ReplayEnvelope {
    results: Vec<Value>,
}
fn replay_value(result: &qsl_replay::ReplayResult) -> Result<Value, Failure> {
    let row = result
        .to_function_json()
        .map_err(|error| Failure::decode("", error.to_string()))?;
    let row = serde_json::from_str(&row).map_err(|error| Failure::decode("", error.to_string()))?;
    serde_json::to_value(ReplayEnvelope { results: vec![row] })
        .map_err(|error| Failure::decode("", error.to_string()))
}

use qsl_foundation::SettingLimits;

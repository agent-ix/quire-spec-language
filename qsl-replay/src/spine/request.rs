// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-312: pure reconstruction of a clause-run request after command intake.

use std::collections::BTreeMap;

use qsl_semantics::model::observation::{
    ClauseSelection, ClauseSelectionInput, DocumentRef, ObservationLimits, SelectedAnchor,
    SelectedObject, SnapshotValue,
};
use quire_exact::{Identifier, ScalarLimits};
use serde_json::Value;

use super::{
    ClauseArgument, ClauseArgumentValue, ClauseRunRequest, ClauseRunSelection, ClauseRunSource,
    DependencyInput, OperationName, SpineLimits,
};

/// A selected observation whose bytes and document identity were admitted by
/// the command path. This decoder reads no filesystem or ambient state.
#[derive(Clone)]
pub struct SuppliedObservation {
    /// The document's verified identity and content digest.
    pub reference: DocumentRef,
    /// The declared document wire identifier.
    pub format: String,
    /// A snapshot's declared observation role, if any.
    pub role: Option<String>,
    /// The exact selected document bytes.
    pub bytes: Vec<u8>,
}

/// Context members supplied for one clause selection.
pub struct ClauseSelectionParts {
    /// The clause name.
    pub name: String,
    /// Key in the selected observations table.
    pub observation: String,
    /// The current or pre snapshot's selected self object.
    pub self_object: Option<SelectedObject>,
    /// The current snapshot's selected anchor.
    pub anchor: Option<SelectedAnchor>,
    /// The pre snapshot's parameter values, in canonical typed wire form.
    pub parameters: Option<BTreeMap<String, Value>>,
}

/// One closed request selection before role-specific reconstruction.
pub enum ClauseSelectionPartsKind {
    /// A state clause over one observation.
    Clause(ClauseSelectionParts),
    /// A frame over an invocation document.
    Frame {
        /// Qualified operation name.
        operation: String,
        /// Key of the invocation document.
        invocation: String,
    },
    /// A Boolean function over a current snapshot.
    Function {
        /// Function name.
        name: String,
        /// Named arguments, each as a canonical integer or object reference.
        arguments: Vec<(String, Value)>,
        /// Key of the current snapshot.
        snapshot: String,
    },
}

/// The typed parts supplied by the path and byte-budget adapter.
pub struct ClauseRequestParts {
    /// The exact program or extracted source.
    pub source: ClauseRunSource,
    /// Canonical domain package input.
    pub packages: BTreeMap<[u8; 32], Vec<u8>>,
    /// Supplied library dependencies.
    pub dependencies: DependencyInput,
    /// Named observation documents, each already read once.
    pub observations: BTreeMap<String, SuppliedObservation>,
    /// The wire selection reconstructed against the named documents.
    pub selection: ClauseSelectionPartsKind,
    /// The caller's expected package identity claim.
    pub expected_package_id_claim: Option<String>,
    /// Spine stage limits.
    pub limits: SpineLimits,
    /// Observation admission limits.
    pub observation_limits: ObservationLimits,
    /// Evaluation meter limits.
    pub accounting: ScalarLimits,
}

/// A closed clause request member that cannot be reconstructed.
#[derive(Debug, thiserror::Error)]
#[error("{member}: {reason}")]
pub struct ClauseRequestReadError {
    /// The member or selected observation key at fault.
    pub member: String,
    /// The typed reason for refusal.
    pub reason: String,
}

fn invalid(member: impl Into<String>, reason: impl Into<String>) -> ClauseRequestReadError {
    ClauseRequestReadError {
        member: member.into(),
        reason: reason.into(),
    }
}

fn named<'a>(
    observations: &'a BTreeMap<String, SuppliedObservation>,
    key: &str,
) -> Result<&'a SuppliedObservation, ClauseRequestReadError> {
    observations
        .get(key)
        .ok_or_else(|| invalid(format!("observations.{key}"), "unbound observation key"))
}

fn snapshot_value(value: &Value) -> Result<SnapshotValue, ClauseRequestReadError> {
    // SnapshotValue itself is arena-backed. Build it bottom-up on a heap
    // worklist so a supplied present/sequence tree has no native stack cap.
    enum Work<'a> {
        Read(&'a Value),
        Present,
        Sequence(usize),
    }
    let mut work = vec![Work::Read(value)];
    let mut values = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            Work::Present => {
                let inner = values
                    .pop()
                    .ok_or_else(|| invalid("parameters", "missing present value"))?;
                values.push(SnapshotValue::present(inner));
            }
            Work::Sequence(len) => {
                if values.len() < len {
                    return Err(invalid("parameters.sequence", "missing sequence value"));
                }
                let items = values.split_off(values.len() - len);
                values.push(SnapshotValue::sequence(items));
            }
            Work::Read(value) => {
                let object = value
                    .as_object()
                    .ok_or_else(|| invalid("parameters", "expected a value object"))?;
                if object.len() != 1 {
                    return Err(invalid("parameters", "expected one value form"));
                }
                let (kind, content) = object.iter().next().expect("one member");
                match kind.as_str() {
                    "boolean" => values.push(SnapshotValue::boolean(
                        content
                            .as_bool()
                            .ok_or_else(|| invalid("parameters.boolean", "expected Boolean"))?,
                    )),
                    "integer" => values.push(SnapshotValue::integer(content.as_str().ok_or_else(
                        || invalid("parameters.integer", "expected decimal string"),
                    )?)),
                    "absent" if content.as_object().is_some_and(serde_json::Map::is_empty) => {
                        values.push(SnapshotValue::absent());
                    }
                    "present" => {
                        work.push(Work::Present);
                        work.push(Work::Read(content));
                    }
                    "reference" => {
                        let reference = content
                            .as_object()
                            .ok_or_else(|| invalid("parameters.reference", "expected object"))?;
                        if reference.len() != 2 {
                            return Err(invalid(
                                "parameters.reference",
                                "expected population and key",
                            ));
                        }
                        let population = reference
                            .get("population")
                            .and_then(Value::as_str)
                            .ok_or_else(|| {
                                invalid("parameters.reference.population", "expected string")
                            })?;
                        let key =
                            reference
                                .get("key")
                                .and_then(Value::as_str)
                                .ok_or_else(|| {
                                    invalid("parameters.reference.key", "expected string")
                                })?;
                        values.push(SnapshotValue::reference(SelectedObject {
                            population: population.to_owned(),
                            key: key.to_owned(),
                        }));
                    }
                    "sequence" => {
                        let items = content
                            .as_array()
                            .ok_or_else(|| invalid("parameters.sequence", "expected array"))?;
                        work.push(Work::Sequence(items.len()));
                        work.extend(items.iter().rev().map(Work::Read));
                    }
                    _ => return Err(invalid(format!("parameters.{kind}"), "unknown value form")),
                }
            }
        }
    }
    values
        .pop()
        .ok_or_else(|| invalid("parameters", "missing value"))
}

#[qsl_attrs::string_edge]
fn selection(
    selected: ClauseSelectionPartsKind,
    observations: &BTreeMap<String, SuppliedObservation>,
) -> Result<ClauseRunSelection, ClauseRequestReadError> {
    match selected {
        ClauseSelectionPartsKind::Clause(clause) => {
            let document = named(observations, &clause.observation)?;
            let input = match (document.format.as_str(), document.role.as_deref()) {
                ("quire.state.snapshot/v1", Some("current")) => {
                    if clause.parameters.is_some() {
                        return Err(invalid("parameters", "forbidden for current snapshot"));
                    }
                    ClauseSelectionInput::Current {
                        snapshot: document.reference.clone(),
                        self_object: clause
                            .self_object
                            .ok_or_else(|| invalid("self", "required for current snapshot"))?,
                        anchor: clause
                            .anchor
                            .ok_or_else(|| invalid("anchor", "required for current snapshot"))?,
                    }
                }
                ("quire.state.snapshot/v1", Some("pre")) => {
                    if clause.anchor.is_some() {
                        return Err(invalid("anchor", "forbidden for pre snapshot"));
                    }
                    let parameters = clause
                        .parameters
                        .ok_or_else(|| invalid("parameters", "required for pre snapshot"))?
                        .iter()
                        .map(|(name, value)| {
                            let name = Identifier::new(name)
                                .map_err(|error| invalid("parameters", error.to_string()))?;
                            Ok((name, snapshot_value(value)?))
                        })
                        .collect::<Result<BTreeMap<_, _>, ClauseRequestReadError>>()?;
                    ClauseSelectionInput::PreCall {
                        snapshot: document.reference.clone(),
                        self_object: clause
                            .self_object
                            .ok_or_else(|| invalid("self", "required for pre snapshot"))?,
                        parameters,
                    }
                }
                ("quire.state.invocation/v1", _) => {
                    if clause.self_object.is_some() {
                        return Err(invalid("self", "forbidden for invocation"));
                    }
                    if clause.anchor.is_some() {
                        return Err(invalid("anchor", "forbidden for invocation"));
                    }
                    if clause.parameters.is_some() {
                        return Err(invalid("parameters", "forbidden for invocation"));
                    }
                    ClauseSelectionInput::Invocation {
                        invocation: document.reference.clone(),
                    }
                }
                _ => return Err(invalid("observation", "unsupported observation role")),
            };
            Ok(ClauseRunSelection::Clause(ClauseSelection {
                name: clause.name,
                input,
            }))
        }
        ClauseSelectionPartsKind::Frame {
            operation,
            invocation,
        } => {
            let document = named(observations, &invocation)?;
            if document.format != "quire.state.invocation/v1" {
                return Err(invalid("invocation", "frame requires an invocation"));
            }
            let segments = operation.split("::").collect::<Vec<_>>();
            let [model, object, operation] = segments.as_slice() else {
                return Err(invalid("operation", "expected M::T::op"));
            };
            let name = |segment: &str| {
                Identifier::new(segment).map_err(|error| invalid("operation", error.to_string()))
            };
            Ok(ClauseRunSelection::Frame {
                operation: OperationName {
                    model: name(model)?,
                    object: name(object)?,
                    operation: name(operation)?,
                },
                invocation: document.reference.clone(),
            })
        }
        ClauseSelectionPartsKind::Function {
            name,
            arguments,
            snapshot,
        } => {
            let document = named(observations, &snapshot)?;
            if document.format != "quire.state.snapshot/v1"
                || document.role.as_deref() != Some("current")
            {
                return Err(invalid("snapshot", "function requires a current snapshot"));
            }
            let arguments = arguments
                .into_iter()
                .map(|(parameter, value)| {
                    let value = if let Some(integer) = value.as_i64() {
                        ClauseArgumentValue::Integer(integer)
                    } else if let Some(reference) = value.get("reference") {
                        let object = reference.as_object().ok_or_else(|| {
                            invalid("arguments.value", "expected object reference")
                        })?;
                        if object.len() != 2 || value.as_object().is_none_or(|v| v.len() != 1) {
                            return Err(invalid(
                                "arguments.value",
                                "expected one object reference",
                            ));
                        }
                        let population = object
                            .get("population")
                            .and_then(Value::as_str)
                            .ok_or_else(|| {
                                invalid("arguments.value.population", "expected string")
                            })?;
                        let key = object
                            .get("key")
                            .and_then(Value::as_str)
                            .ok_or_else(|| invalid("arguments.value.key", "expected string"))?;
                        ClauseArgumentValue::Reference {
                            population: population.to_owned(),
                            key: key.to_owned(),
                        }
                    } else {
                        return Err(invalid(
                            "arguments.value",
                            "expected integer or object reference",
                        ));
                    };
                    Ok(ClauseArgument { parameter, value })
                })
                .collect::<Result<Vec<_>, ClauseRequestReadError>>()?;
            Ok(ClauseRunSelection::Function {
                name,
                arguments,
                snapshot: document.reference.clone(),
            })
        }
    }
}

/// Reconstruct the FR-109 request from file-admitted, typed parts. The root
/// command adapter owns paths, byte ceilings and selected-file reads; this
/// function owns the FR-106 role and selection mapping and reads no path.
#[qsl_attrs::string_edge]
pub fn read_clause_run_request(
    parts: ClauseRequestParts,
) -> Result<ClauseRunRequest, ClauseRequestReadError> {
    if let Some(claim) = &parts.expected_package_id_claim {
        if claim.len() != 64
            || !claim
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(invalid("package_id", "expected 64 lowercase hex digits"));
        }
    }
    let selection = selection(parts.selection, &parts.observations)?;
    let mut snapshots = BTreeMap::new();
    let mut invocations = BTreeMap::new();
    for observation in parts.observations.into_values() {
        match observation.format.as_str() {
            "quire.state.snapshot/v1" => {
                snapshots.insert(observation.reference.digest, observation.bytes);
            }
            "quire.state.invocation/v1" => {
                invocations.insert(observation.reference.digest, observation.bytes);
            }
            _ => return Err(invalid("observation", "unsupported document format")),
        }
    }
    Ok(ClauseRunRequest {
        source: parts.source,
        packages: parts.packages,
        dependencies: parts.dependencies,
        snapshots,
        invocations,
        selection,
        expected_package_id: None,
        expected_package_id_claim: parts.expected_package_id_claim,
        limits: parts.limits,
        observation_limits: parts.observation_limits,
        accounting: parts.accounting,
    })
}

#[cfg(test)]
mod tests {
    use super::snapshot_value;
    use qsl_semantics::model::observation::RawForm;
    use serde_json::{Map, Value};

    #[test]
    fn parameter_value_reader_uses_heap_stack_past_128_levels() {
        let mut value = Value::Object(Map::from_iter([(
            "integer".to_owned(),
            Value::String("7".to_owned()),
        )]));
        for _ in 0..160 {
            value = Value::Object(Map::from_iter([("present".to_owned(), value)]));
        }
        let decoded = snapshot_value(&value).unwrap();
        let mut form = decoded.form();
        for _ in 0..160 {
            form = match form {
                RawForm::Present(inner) => inner.form(),
                other => panic!("expected present, got {other:?}"),
            };
        }
        assert!(matches!(form, RawForm::Integer("7")));
    }
}

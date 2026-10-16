// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-312: pure reconstruction of a clause-run request after command intake.

use std::collections::BTreeMap;

use qsl_semantics::model::observation::{
    ClauseSelection, ClauseSelectionInput, DocumentRef, NativeKind, ObservationLimits,
    RawNativeKind, SelectedAnchor, SelectedObject, SnapshotValue,
};
use quire_canonical::{Node, NodeRef};
use quire_exact::{Identifier, ScalarLimits};
use serde_json::value::RawValue;

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
    pub parameters: Option<BTreeMap<String, Box<RawValue>>>,
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
        arguments: Vec<(String, Box<RawValue>)>,
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

#[qsl_attrs::string_edge]
fn snapshot_value(value: NodeRef<'_>) -> Result<SnapshotValue, ClauseRequestReadError> {
    // SnapshotValue itself is arena-backed. Build it bottom-up on a heap
    // worklist so a supplied present/sequence tree has no native stack cap.
    enum Work<'a> {
        Read(NodeRef<'a>),
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
                let Node::Object(mut object) = value.node() else {
                    return Err(invalid("parameters", "expected a value object"));
                };
                // A native payload object (String, UUID, Timestamp) carries
                // its kind as a member beside the payload; every defect is
                // held for admission to judge in FR-106's native order.
                if let Some(kind) = value.get("kind") {
                    let kind = match kind.node() {
                        Node::String("string") => RawNativeKind::Known(NativeKind::String),
                        Node::String("uuid") => RawNativeKind::Known(NativeKind::Uuid),
                        Node::String("timestamp") => RawNativeKind::Known(NativeKind::Timestamp),
                        Node::String(_) => RawNativeKind::Unknown,
                        _ => RawNativeKind::NotText,
                    };
                    let members = object
                        .filter(|(name, _)| *name != "kind")
                        .map(|(name, member)| {
                            let text = match member.node() {
                                Node::String(text) => Some(text.to_owned()),
                                _ => None,
                            };
                            (name.to_owned(), text)
                        })
                        .collect();
                    values.push(SnapshotValue::native(kind, members));
                    continue;
                }
                if object.len() != 1 {
                    return Err(invalid("parameters", "expected one value form"));
                }
                let (kind, content) = object.next().expect("one member");
                match kind {
                    "boolean" => values.push(SnapshotValue::boolean(match content.node() {
                        Node::Bool(value) => value,
                        _ => return Err(invalid("parameters.boolean", "expected Boolean")),
                    })),
                    "integer" => values.push(SnapshotValue::integer(match content.node() {
                        Node::String(value) => value,
                        _ => return Err(invalid("parameters.integer", "expected decimal string")),
                    })),
                    "absent" if matches!(content.node(), Node::Object(items) if items.len() == 0) =>
                    {
                        values.push(SnapshotValue::absent());
                    }
                    "present" => {
                        work.push(Work::Present);
                        work.push(Work::Read(content));
                    }
                    "reference" => {
                        let Node::Object(reference) = content.node() else {
                            return Err(invalid("parameters.reference", "expected object"));
                        };
                        if reference.len() != 2 {
                            return Err(invalid(
                                "parameters.reference",
                                "expected population and key",
                            ));
                        }
                        let population = match content.get("population").map(|v| v.node()) {
                            Some(Node::String(value)) => value,
                            _ => {
                                return Err(invalid(
                                    "parameters.reference.population",
                                    "expected string",
                                ))
                            }
                        };
                        let key = match content.get("key").map(|v| v.node()) {
                            Some(Node::String(value)) => value,
                            _ => {
                                return Err(invalid("parameters.reference.key", "expected string"))
                            }
                        };
                        values.push(SnapshotValue::reference(SelectedObject {
                            population: population.to_owned(),
                            key: key.to_owned(),
                        }));
                    }
                    "variant" => {
                        let Node::Object(payload) = content.node() else {
                            return Err(invalid("parameters.variant", "expected object"));
                        };
                        let (mut type_identity, mut case, mut extra_member) = (None, None, false);
                        for (name, member) in payload {
                            let text = |member: NodeRef<'_>, field: &str| match member.node() {
                                Node::String(text) => Ok(text.to_owned()),
                                _ => Err(invalid(
                                    format!("parameters.variant.{field}"),
                                    "expected string",
                                )),
                            };
                            match name {
                                "type" => type_identity = Some(text(member, "type")?),
                                "case" => case = Some(text(member, "case")?),
                                _ => extra_member = true,
                            }
                        }
                        values.push(SnapshotValue::variant_payload(
                            type_identity,
                            case,
                            extra_member,
                        ));
                    }
                    "sequence" => {
                        let Node::Array(items) = content.node() else {
                            return Err(invalid("parameters.sequence", "expected array"));
                        };
                        work.push(Work::Sequence(items.len()));
                        work.extend(items.collect::<Vec<_>>().into_iter().rev().map(Work::Read));
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
                            let document = quire_canonical::read(value.get().as_bytes(), u64::MAX)
                                .map_err(|error| invalid("parameters", error.to_string()))?;
                            Ok((name, snapshot_value(document.root())?))
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
                    let document = quire_canonical::read(value.get().as_bytes(), u64::MAX)
                        .map_err(|error| invalid("arguments.value", error.to_string()))?;
                    let value = document.root();
                    let parsed_integer = match value.node() {
                        Node::Number(number) => number.text().parse::<i64>().ok(),
                        _ => None,
                    };
                    let value = if let Some(integer) = parsed_integer {
                        ClauseArgumentValue::Integer(integer)
                    } else if let Some(reference) = value.get("reference") {
                        let Node::Object(object) = reference.node() else {
                            return Err(invalid("arguments.value", "expected object reference"));
                        };
                        if object.len() != 2
                            || !matches!(value.node(), Node::Object(members) if members.len() == 1)
                        {
                            return Err(invalid(
                                "arguments.value",
                                "expected one object reference",
                            ));
                        }
                        let population = match reference.get("population").map(|v| v.node()) {
                            Some(Node::String(value)) => value,
                            _ => {
                                return Err(invalid(
                                    "arguments.value.population",
                                    "expected string",
                                ))
                            }
                        };
                        let key = match reference.get("key").map(|v| v.node()) {
                            Some(Node::String(value)) => value,
                            _ => return Err(invalid("arguments.value.key", "expected string")),
                        };
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
    use qsl_semantics::model::observation::{NativeKind, RawForm, RawNativeKind};

    #[test]
    fn parameter_value_reader_uses_heap_stack_past_128_levels() {
        let mut value = String::from("{\"integer\":\"7\"}");
        for _ in 0..160 {
            value = format!("{{\"present\":{value}}}");
        }
        let document = quire_canonical::read(value.as_bytes(), u64::MAX).unwrap();
        let decoded = snapshot_value(document.root()).unwrap();
        let mut form = decoded.form();
        for _ in 0..160 {
            form = match form {
                RawForm::Present(inner) => inner.form(),
                other => panic!("expected present, got {other:?}"),
            };
        }
        assert!(matches!(form, RawForm::Integer("7")));
    }

    /// FR-106: a native payload object parameter keeps its `kind` and every
    /// other member as read, so admission judges a defect in the native
    /// payload order rather than the request reader refusing it.
    #[test]
    fn native_parameter_keeps_its_kind_and_members_as_read() {
        let read = |text: &str| {
            let document = quire_canonical::read(text.as_bytes(), u64::MAX).unwrap();
            snapshot_value(document.root()).unwrap()
        };
        let uuid = read(r#"{"kind":"uuid","text":"t","extra":1}"#);
        let RawForm::Native(native) = uuid.form() else {
            panic!("expected a native object");
        };
        assert_eq!(native.kind(), RawNativeKind::Known(NativeKind::Uuid));
        assert_eq!(
            native.members().collect::<Vec<_>>(),
            [("text", Some("t")), ("extra", None)]
        );
        for (text, kind) in [
            (
                r#"{"kind":"string","text":""}"#,
                RawNativeKind::Known(NativeKind::String),
            ),
            (
                r#"{"kind":"timestamp","nanoseconds":"0"}"#,
                RawNativeKind::Known(NativeKind::Timestamp),
            ),
            (r#"{"kind":"String"}"#, RawNativeKind::Unknown),
            (r#"{"kind":1}"#, RawNativeKind::NotText),
        ] {
            let value = read(text);
            let RawForm::Native(native) = value.form() else {
                panic!("{text}: expected a native object");
            };
            assert_eq!(native.kind(), kind, "{text}");
        }
    }

    /// FR-360 item 9: a `variant` parameter keeps the members it was read
    /// with, so admission can refuse a missing or an extra one in its own
    /// order.
    #[test]
    fn parameter_value_reader_keeps_the_members_of_a_variant() {
        for (text, type_identity, case, extra_member) in [
            (
                r#"{"variant":{"type":"ix://p/Tier","case":"Gold"}}"#,
                Some("ix://p/Tier"),
                Some("Gold"),
                false,
            ),
            (
                r#"{"variant":{"type":"ix://p/Tier"}}"#,
                Some("ix://p/Tier"),
                None,
                false,
            ),
            (r#"{"variant":{"case":"Gold"}}"#, None, Some("Gold"), false),
            (
                r#"{"variant":{"type":"ix://p/Tier","case":"Gold","x":1}}"#,
                Some("ix://p/Tier"),
                Some("Gold"),
                true,
            ),
        ] {
            let document = quire_canonical::read(text.as_bytes(), u64::MAX).unwrap();
            let decoded = snapshot_value(document.root()).unwrap();
            let RawForm::Variant(variant) = decoded.form() else {
                panic!("a variant, not {:?}", decoded.form());
            };
            assert_eq!(variant.type_identity(), type_identity, "{text}");
            assert_eq!(variant.case(), case, "{text}");
            assert_eq!(variant.has_extra_member(), extra_member, "{text}");
        }
    }
}

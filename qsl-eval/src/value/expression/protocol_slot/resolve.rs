// SPDX-License-Identifier: AGPL-3.0-or-later
//! Layer-5 admission helpers for qsl-package's single protocol-slot selector.

use quire_eval::{CallAdmissionFailure, CallAdmissionLocus, CallAdmissionRecord, CallFailure};
use quire_exact::ValueType;
use quire_semantic_value::admission::AdmissionRecord;
use quire_semantic_value::catalog::CatalogCode;

fn admission(code: &'static str, cause: &'static str, locus: CallAdmissionLocus) -> CallFailure {
    CallFailure::Admission(Box::new(CallAdmissionFailure::Refused(
        CallAdmissionRecord {
            record: AdmissionRecord {
                catalog: CatalogCode::new(code, cause),
                fields: Default::default(),
                path: Vec::new(),
                kernel: None,
            },
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

pub(crate) fn selection_failure(failure: qsl_package::SlotSelectionFailure) -> CallFailure {
    use qsl_package::SlotSelectionFailure;
    match failure {
        SlotSelectionFailure::MissingNode(key) => admission(
            "missing_declaration",
            "missing-name",
            CallAdmissionLocus::Node(key),
        ),
        SlotSelectionFailure::MalformedNode(key) => admission(
            "invalid_model_binding",
            "malformed-declaration",
            CallAdmissionLocus::Node(key),
        ),
        SlotSelectionFailure::MissingMember { name, index } => admission(
            "missing_declaration",
            "missing-name",
            CallAdmissionLocus::Member { name, index },
        ),
        SlotSelectionFailure::Fault(fault) => CallFailure::Fault(fault),
    }
}

pub(crate) fn foreign_reference_refused(
    required: Option<&quire_exact::UniverseId>,
    supplied: &quire_exact::UniverseId,
    locus: CallAdmissionLocus,
) -> CallFailure {
    let identity_hex = |universe: &quire_exact::UniverseId| {
        qsl_foundation::node_wire::node_key_to_text(universe.root_type())
            .map(|text| lowercase_hex(text.as_bytes()))
    };
    let Ok(supplied_hex) = identity_hex(supplied) else {
        return CallFailure::Fault(quire_semantic_value::catalog::InternalFault::new(
            "FR-390",
            "foreign-reference-universe-has-a-canonical-identity",
        ));
    };
    let mut fields = std::collections::BTreeMap::new();
    if let Some(required) = required {
        let Ok(required_hex) = identity_hex(required) else {
            return CallFailure::Fault(quire_semantic_value::catalog::InternalFault::new(
                "FR-390",
                "foreign-reference-universe-has-a-canonical-identity",
            ));
        };
        fields.insert("required", required_hex);
    }
    fields.insert("supplied", supplied_hex);
    CallFailure::Admission(Box::new(CallAdmissionFailure::Refused(
        CallAdmissionRecord {
            record: AdmissionRecord {
                catalog: CatalogCode::new("foreign_reference", "foreign-universe"),
                fields,
                path: Vec::new(),
                kernel: None,
            },
            locus,
        },
    )))
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    text
}

pub(crate) fn shape_matches(
    value_type: &ValueType,
    value: &quire_exact::Value,
    types: &quire_semantic_value::declaration::TypeEnvironment,
) -> bool {
    use quire_exact::{Value, ValueType};
    if let (ValueType::Composite(expected), Value::Composite(actual)) = (value_type, value) {
        return types.composite_conforms(actual.declaration().clone(), expected.clone());
    }
    matches!(
        (value_type, value),
        (ValueType::Boolean, Value::Boolean(_))
            | (ValueType::Integer | ValueType::Int(_), Value::Integer(_))
            | (ValueType::Rational(_), Value::Rational(_))
            | (ValueType::Decimal(_), Value::Decimal(_))
            | (ValueType::Float(_), Value::Float(_))
            | (
                ValueType::Quantity(_)
                    | ValueType::CarriedQuantity(_)
                    | ValueType::BoundedQuantity(_),
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

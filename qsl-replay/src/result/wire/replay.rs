// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only serialization of the existing typed function replay results.

use quire_exact::{CancelCause, ScalarLimits};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::supplied_membership::{InvalidMembershipCause, MembershipPart, PathStep};
use quire_semantic_value::supplied_stop::{
    AttemptedCount, SuppliedCause, SuppliedLocus, SuppliedStop, SuppliedStorageUnit,
};
use serde::Serialize;
use serde_json::value::RawValue;

use super::{CauseCodecError, CauseWire, RecordWire};
use crate::{InputSettlement, ReplayResult, WitnessSettlement};

#[derive(Serialize)]
struct ReplayWire {
    arm: &'static str,
    settlement: &'static str,
    category: &'static str,
    value: Option<bool>,
    charges: Charges,
    #[serde(skip_serializing_if = "Option::is_none")]
    disagreement: Option<CauseWire<Box<RawValue>>>,
    resolved_regions: Vec<crate::OutcomeLocus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    record: Option<RecordWire<Box<RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<LimitWire>,
    #[serde(skip_serializing_if = "Option::is_none")]
    supplied: Option<SuppliedWire>,
}

impl ReplayResult {
    /// Serialize a function replay's complete typed result for the FR-323
    /// per-item result envelope. This reads the settled owner value and runs
    /// no evaluation or admission.
    ///
    /// # Errors
    ///
    /// [`CauseCodecError`] for a value the strict witness codec cannot encode,
    /// or a non-function replay arm with a different owning result contract.
    pub fn to_function_json(&self) -> Result<String, CauseCodecError> {
        let (arm, settlement, value, regions, record) = match self {
            Self::Input(result) => (
                "input",
                match result.settlement() {
                    InputSettlement::ReproducedWithoutWitness => "reproduced-without-witness",
                    InputSettlement::Inconclusive => "inconclusive",
                },
                result.value(),
                result.resolved_regions(),
                None,
            ),
            Self::Witness(result) => (
                "witness",
                match result.settlement() {
                    WitnessSettlement::ReproducedWithEvaluatedWitness => {
                        "reproduced-with-evaluated-witness"
                    }
                    WitnessSettlement::Inconclusive => "inconclusive",
                },
                result.value(),
                result.resolved_regions(),
                result.record(),
            ),
            Self::ObservedTemporal(_) => {
                return Err(CauseCodecError::UnsupportedReplayArm("observed-temporal"))
            }
            Self::ModelGraph(_) => {
                return Err(CauseCodecError::UnsupportedReplayArm("model-graph"))
            }
        };
        let wire = ReplayWire {
            arm,
            settlement,
            category: self.category().as_str(),
            value: value.map(|crate::EvaluatedValue::Boolean(value)| value),
            charges: Charges::from(self.charges()),
            disagreement: self
                .disagreement()
                .map(|cause| CauseWire::of(cause, None))
                .transpose()?,
            resolved_regions: regions
                .iter()
                .map(|region| {
                    crate::OutcomeLocus::from(&qsl_foundation::diagnostic::Locus::Region(
                        region.clone(),
                    ))
                })
                .collect(),
            record: record.map(RecordWire::of).transpose()?,
            limit: self.limit().map(|record| LimitWire {
                owner: crate::ResultLimit::from(&crate::spine::CallIncomplete {
                    record: record.clone(),
                    location: None,
                }),
                consumed: record.consumed.to_string(),
                next_charge: record.next_charge.to_string(),
                charge_point: record.charge_point.as_str(),
            }),
            supplied: self.pre_call_limit().map(SuppliedWire::from),
        };
        Ok(serde_json::to_string(&wire)?)
    }
}

#[derive(Serialize)]
struct Charges {
    integer_bits: String,
    decimal_digits: String,
    scale_expansion: String,
    text_input_bytes: String,
    text_scalars: String,
    normalized_scalars: String,
    unit_edges: String,
    value_occurrences: String,
    work_units: String,
    result_units: String,
}

impl From<ScalarLimits> for Charges {
    fn from(limits: ScalarLimits) -> Self {
        let ScalarLimits {
            integer_bits,
            decimal_digits,
            scale_expansion,
            text_input_bytes,
            text_scalars,
            normalized_scalars,
            unit_edges,
            value_occurrences,
            work_units,
            result_units,
        } = limits;
        Self {
            integer_bits: integer_bits.to_string(),
            decimal_digits: decimal_digits.to_string(),
            scale_expansion: scale_expansion.to_string(),
            text_input_bytes: text_input_bytes.to_string(),
            text_scalars: text_scalars.to_string(),
            normalized_scalars: normalized_scalars.to_string(),
            unit_edges: unit_edges.to_string(),
            value_occurrences: value_occurrences.to_string(),
            work_units: work_units.to_string(),
            result_units: result_units.to_string(),
        }
    }
}

#[derive(Serialize)]
struct LimitWire {
    #[serde(flatten)]
    owner: crate::ResultLimit,
    consumed: String,
    next_charge: String,
    charge_point: &'static str,
}

#[derive(Serialize)]
struct SuppliedWire {
    phase: &'static str,
    parameter: Option<usize>,
    event: Option<&'static str>,
    locus: Option<LocusWire>,
    consumed: String,
    cause: SuppliedCauseWire,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum SuppliedCauseWire {
    Limit {
        limit_kind: &'static str,
        field: &'static str,
        limit: String,
        next_charge: String,
        counter: CounterWire,
        #[serde(skip_serializing_if = "Option::is_none")]
        operand: Option<&'static str>,
    },
    Input {
        native: NativeWire,
    },
    Cancelled {
        native: NativeWire,
    },
    Storage {
        native: NativeWire,
        requested: String,
        unit: &'static str,
    },
    Capacity {
        native: NativeWire,
    },
}

#[derive(Serialize)]
#[serde(untagged)]
enum CounterWire {
    Count(String),
    Overflow { overflow: bool },
}

#[derive(Serialize)]
struct NativeWire {
    producer: &'static str,
    stage: &'static str,
    code: &'static str,
    cause: Option<&'static str>,
    payload: NativePayload,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum NativePayload {
    Input {
        refusal: InputWire,
        membership: Option<&'static str>,
    },
    Cancelled {
        source: &'static str,
    },
    Storage {
        requested: String,
        unit: &'static str,
    },
    Capacity {
        requested: String,
        unit: &'static str,
    },
}

impl From<&SuppliedStop> for SuppliedWire {
    fn from(stop: &SuppliedStop) -> Self {
        let native = |payload| NativeWire {
            producer: "quire-semantic-value",
            stage: stop.context.phase.as_str(),
            code: qsl_eval::value::supplied_stop_code(stop).as_str(),
            cause: qsl_eval::value::supplied_stop_cause(stop),
            payload,
        };
        let cause = match &stop.cause {
            SuppliedCause::Limit(limit) => SuppliedCauseWire::Limit {
                limit_kind: limit.field(),
                field: limit.field(),
                limit: limit.limit.to_string(),
                next_charge: limit.next_charge.to_string(),
                counter: match limit.counter {
                    AttemptedCount::Count(count) => CounterWire::Count(count.to_string()),
                    AttemptedCount::Overflow => CounterWire::Overflow { overflow: true },
                },
                operand: limit.operand.map(|operand| operand.as_str()),
            },
            SuppliedCause::Input {
                refusal,
                membership,
            } => SuppliedCauseWire::Input {
                native: native(NativePayload::Input {
                    refusal: InputWire::from(refusal),
                    membership: membership.map(|cause| match cause {
                        InvalidMembershipCause::Kind => "kind",
                        InvalidMembershipCause::Type => "type",
                        InvalidMembershipCause::Member => "member",
                        InvalidMembershipCause::Payload => "payload",
                        InvalidMembershipCause::ReferenceConformance => "reference-conformance",
                        InvalidMembershipCause::CompositeConformance => "composite-conformance",
                        InvalidMembershipCause::UnsupportedUnion => "unsupported-union",
                        InvalidMembershipCause::ForeignPath => "foreign-path",
                    }),
                }),
            },
            SuppliedCause::Cancelled(cause) => SuppliedCauseWire::Cancelled {
                native: native(NativePayload::Cancelled {
                    source: match cause {
                        CancelCause::Requested => "requested",
                        CancelCause::Deadline => "deadline",
                    },
                }),
            },
            SuppliedCause::Storage(storage) => SuppliedCauseWire::Storage {
                native: native(NativePayload::Storage {
                    requested: storage.requested.to_string(),
                    unit: storage_unit(storage.unit),
                }),
                requested: storage.requested.to_string(),
                unit: storage_unit(storage.unit),
            },
            SuppliedCause::Capacity(storage) => SuppliedCauseWire::Capacity {
                native: native(NativePayload::Capacity {
                    requested: storage.requested.to_string(),
                    unit: storage_unit(storage.unit),
                }),
            },
        };
        Self {
            phase: stop.context.phase.as_str(),
            parameter: stop.context.parameter,
            event: stop.context.event.map(|event| event.as_str()),
            locus: stop.context.locus.as_ref().map(LocusWire::from),
            consumed: stop.context.consumed.to_string(),
            cause,
        }
    }
}

fn storage_unit(unit: SuppliedStorageUnit) -> &'static str {
    match unit {
        SuppliedStorageUnit::Bytes => "bytes",
        SuppliedStorageUnit::AdditionalElements => "additional-elements",
        SuppliedStorageUnit::ArenaIdentifiers => "arena-identifiers",
        SuppliedStorageUnit::MagnitudeBits => "magnitude-bits",
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum LocusWire {
    Membership {
        argument: usize,
        steps_from_leaf: Vec<EdgeWire>,
        part: PartWire,
    },
    Witness {
        argument: usize,
        position: String,
    },
}

impl From<&SuppliedLocus> for LocusWire {
    fn from(locus: &SuppliedLocus) -> Self {
        match locus {
            SuppliedLocus::Membership {
                argument,
                steps_from_leaf,
                part,
            } => Self::Membership {
                argument: *argument,
                steps_from_leaf: steps_from_leaf
                    .iter()
                    .copied()
                    .map(EdgeWire::from)
                    .collect(),
                part: PartWire::from(*part),
            },
            SuppliedLocus::Witness { argument, position } => Self::Witness {
                argument: *argument,
                position: position.to_string(),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum PartWire {
    Value,
    TypeLink { index: String },
    Child { step: EdgeWire },
    Descriptor { index: String },
}

impl From<MembershipPart> for PartWire {
    fn from(part: MembershipPart) -> Self {
        match part {
            MembershipPart::Value => Self::Value,
            MembershipPart::TypeLink(index) => Self::TypeLink {
                index: index.to_string(),
            },
            MembershipPart::Child(step) => Self::Child {
                step: EdgeWire::from(step),
            },
            MembershipPart::Descriptor(index) => Self::Descriptor {
                index: index.to_string(),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum EdgeWire {
    OptionPayload,
    CollectionElement { index: usize },
    CompositeField { index: usize },
    TuplePosition { index: usize },
    UnionPayload { index: usize },
}

impl From<PathStep> for EdgeWire {
    fn from(step: PathStep) -> Self {
        match step {
            PathStep::OptionPayload => Self::OptionPayload,
            PathStep::CollectionElement(index) => Self::CollectionElement { index },
            PathStep::CompositeField(index) => Self::CompositeField { index },
            PathStep::TuplePosition(index) => Self::TuplePosition { index },
            PathStep::UnionPayload(index) => Self::UnionPayload { index },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum InputWire {
    UnknownFunction { name: String },
    Arity { declared: usize, supplied: usize },
    WrongValueKind { parameter: usize },
    DanglingReference { parameter: usize },
    UnknownClause { name: String },
    ObservationsMismatch,
    DuplicateClause { name: String },
}

impl From<&InputRefusal> for InputWire {
    fn from(refusal: &InputRefusal) -> Self {
        match refusal {
            InputRefusal::UnknownFunction(name) => Self::UnknownFunction { name: name.clone() },
            InputRefusal::Arity { declared, supplied } => Self::Arity {
                declared: *declared,
                supplied: *supplied,
            },
            InputRefusal::WrongValueKind { parameter } => Self::WrongValueKind {
                parameter: *parameter,
            },
            InputRefusal::DanglingReference { parameter } => Self::DanglingReference {
                parameter: *parameter,
            },
            InputRefusal::UnknownClause(name) => Self::UnknownClause { name: name.clone() },
            InputRefusal::ObservationsMismatch => Self::ObservationsMismatch,
            InputRefusal::DuplicateClause(name) => Self::DuplicateClause { name: name.clone() },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InputArmResult, Verdict, WitnessCheck};
    use ix_trace_rs::trace;
    use quire_semantic_value::catalog::Category;
    use quire_semantic_value::supplied_stop::{
        SuppliedContext, SuppliedEvent, SuppliedLimit, SuppliedLimitKind, SuppliedPhase,
        SuppliedStorage,
    };
    use serde_json::{json, Value};

    #[test]
    #[trace("TC-1325", "FR-350-AC-10", "FR-350-AC-11")]
    fn function_writer_preserves_every_native_supplied_cause_and_locus() {
        let context = SuppliedContext {
            phase: SuppliedPhase::Conversion,
            parameter: Some(3),
            event: Some(SuppliedEvent::WitnessPreflight),
            locus: Some(SuppliedLocus::Membership {
                argument: 3,
                steps_from_leaf: vec![PathStep::TuplePosition(2), PathStep::OptionPayload],
                part: MembershipPart::Descriptor(7),
            }),
            consumed: 17,
        };
        let cases = [
            (
                SuppliedCause::Limit(SuppliedLimit {
                    limit_kind: SuppliedLimitKind::ConversionWorkUnits,
                    limit: 17,
                    next_charge: 1,
                    counter: AttemptedCount::Overflow,
                    operand: None,
                }),
                "limit",
            ),
            (
                SuppliedCause::Input {
                    refusal: InputRefusal::WrongValueKind { parameter: 3 },
                    membership: Some(InvalidMembershipCause::Payload),
                },
                "input",
            ),
            (
                SuppliedCause::Cancelled(CancelCause::Requested),
                "cancelled",
            ),
            (SuppliedCause::Cancelled(CancelCause::Deadline), "cancelled"),
            (
                SuppliedCause::Storage(SuppliedStorage {
                    requested: 321,
                    unit: SuppliedStorageUnit::Bytes,
                }),
                "storage",
            ),
            (
                SuppliedCause::Capacity(SuppliedStorage {
                    requested: 654,
                    unit: SuppliedStorageUnit::ArenaIdentifiers,
                }),
                "capacity",
            ),
        ];
        for (cause, kind) in cases {
            let stop = SuppliedStop {
                context: context.clone(),
                cause,
            };
            let result = ReplayResult::Input(InputArmResult::settle(
                Verdict::from_category(Category::Success),
                Verdict::from_category(Category::Inconclusive),
                Category::Inconclusive,
                None,
                &WitnessCheck::Agrees(None),
                Vec::new(),
                ScalarLimits::zero(),
            ))
            .with_pre_call(Box::new(stop));
            let encoded = result.to_function_json().unwrap();
            assert_eq!(encoded, result.to_function_json().unwrap());
            let wire: Value = serde_json::from_str(&encoded).unwrap();
            assert_eq!(wire["value"], Value::Null);
            assert_eq!(wire["settlement"], "inconclusive");
            assert!(wire["charges"]
                .as_object()
                .unwrap()
                .values()
                .all(|charge| charge == "0"));
            let supplied = &wire["supplied"];
            assert_eq!(supplied["phase"], "conversion");
            assert_eq!(supplied["parameter"], 3);
            assert_eq!(supplied["event"], "witness.preflight");
            assert_eq!(supplied["consumed"], "17");
            assert_eq!(
                supplied["locus"],
                json!({"kind":"membership", "argument":3,
                "steps_from_leaf":[{"kind":"tuple-position", "index":2},{"kind":"option-payload"}],
                "part":{"kind":"descriptor", "index":"7"}})
            );
            assert_eq!(supplied["cause"]["kind"], kind);
            if kind == "limit" {
                assert_eq!(supplied["cause"]["counter"], json!({"overflow":true}));
                assert_eq!(supplied["cause"]["limit"], "17");
                assert_eq!(supplied["cause"]["next_charge"], "1");
            } else {
                assert_eq!(supplied["cause"]["native"]["stage"], "conversion");
                assert_eq!(
                    supplied["cause"]["native"]["producer"],
                    "quire-semantic-value"
                );
                let payload = &supplied["cause"]["native"]["payload"];
                match &result.pre_call_limit().unwrap().cause {
                    SuppliedCause::Input { .. } => {
                        assert_eq!(
                            payload["refusal"],
                            json!({"kind":"wrong-value-kind", "parameter":3})
                        );
                        assert_eq!(payload["membership"], "payload");
                    }
                    SuppliedCause::Cancelled(CancelCause::Requested) => {
                        assert_eq!(payload["source"], "requested")
                    }
                    SuppliedCause::Cancelled(CancelCause::Deadline) => {
                        assert_eq!(payload["source"], "deadline")
                    }
                    SuppliedCause::Storage(_) => {
                        assert_eq!(payload["requested"], "321");
                        assert_eq!(payload["unit"], "bytes");
                    }
                    SuppliedCause::Capacity(_) => {
                        assert_eq!(payload["requested"], "654");
                        assert_eq!(payload["unit"], "arena-identifiers");
                    }
                    SuppliedCause::Limit(_) => unreachable!(),
                }
            }
        }
    }
}

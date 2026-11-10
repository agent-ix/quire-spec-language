// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked bounded fairness, using the shared model-operation target identity.

use super::*;
use qsl_foundation::node_wire::{project_node_key, NodeIdentitySink};
use quire_canonical::{Encode, Error, Sink, Writer};
use quire_exact::{Identifier, NodeKey, NodePathIter};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

/// Actual bounded fair-proof checking stages, for default-off observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundedFairProbabilityPhase {
    /// Strict objective intake.
    Intake,
    /// Looking up an operation's actual declaring model identity.
    Declaration,
    /// Canonically encoding a resolved fairness record.
    Metadata,
    /// Independently reconstructing the bounded product and its keys.
    Graph,
    /// Checking offered values and remaining-horizon ranks.
    Equality,
}

pub(super) struct Control<'a> {
    cancel: &'a Cancel,
    #[cfg(feature = "test-support")]
    observe: std::cell::RefCell<Option<&'a mut dyn FnMut(BoundedFairProbabilityPhase)>>,
}
impl<'a> Control<'a> {
    pub(super) fn new(cancel: &'a Cancel) -> Self {
        Self {
            cancel,
            #[cfg(feature = "test-support")]
            observe: std::cell::RefCell::new(None),
        }
    }
    #[cfg(feature = "test-support")]
    pub(super) fn observing(
        cancel: &'a Cancel,
        observe: &'a mut dyn FnMut(BoundedFairProbabilityPhase),
    ) -> Self {
        Self {
            cancel,
            observe: std::cell::RefCell::new(Some(observe)),
        }
    }
    pub(super) fn phase(
        &self,
        phase: BoundedFairProbabilityPhase,
    ) -> Result<(), ProbabilityDocumentError> {
        #[cfg(feature = "test-support")]
        if let Some(observe) = self.observe.borrow_mut().as_mut() {
            observe(phase);
        }
        #[cfg(not(feature = "test-support"))]
        let _ = phase;
        poll(self.cancel)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Record {
    kind: FairnessKind,
    granularity: FairnessGranularity,
    declaration: NodeKey,
    name: Identifier,
}

impl Encode for Record {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("fairness_kind")?;
        writer.string(match self.kind {
            FairnessKind::Weak => "weak",
            FairnessKind::Strong => "strong",
        })?;
        writer.name("granularity")?;
        writer.string(match self.granularity {
            FairnessGranularity::Whole => "whole",
            FairnessGranularity::Each => "each",
        })?;
        writer.name("target")?;
        writer.begin_object()?;
        writer.name("kind")?;
        writer.string("operation")?;
        writer.name("declaration")?;
        qsl_foundation::node_wire::encode_node_key(&self.declaration, writer)?;
        writer.name("name")?;
        writer.string(self.name.as_str())?;
        writer.end_object()?;
        writer.end_object()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRecord<'a> {
    fairness_kind: &'a str,
    granularity: &'a str,
    #[serde(borrow)]
    target: WireTarget<'a>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireTarget<'a> {
    kind: &'a str,
    #[serde(borrow)]
    declaration: &'a serde_json::value::RawValue,
    name: &'a str,
}

pub(super) fn admit(claim: &CheckedProbabilisticClaim) -> Result<(), ProbabilityDocumentError> {
    let ProbForm::Probability {
        event:
            qsl_semantics::check::Event::Formula {
                formula,
                timed_horizon: None,
                ..
            },
        ..
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let profile = qsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    if claim.fairness.is_empty()
        || !matches!(claim.scheduler, ProbScheduler::Every)
        || !qsl_eval::automaton::QualifiedBoundedMonitor::supports(formula.formula(), profile)
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    Ok(())
}

struct Admission<'a> {
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    meter: &'a mut quire_exact::Meter,
    storage: &'a mut u64,
}
impl Admission<'_> {
    fn work(&mut self, amount: u64) -> Result<(), ProbabilityDocumentError> {
        poll(self.cancel)?;
        let mut admission = ProbabilityKeyAdmission::new(
            self.meter,
            self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        quire_exact::IntegerDecimalBudget::work(&mut admission, amount)
            .map_err(|_| admission.refusal(qsl_eval::simulation::model::ModelKeyError::Denied))
    }
    fn reserve(&mut self, bytes: usize) -> Result<(), ProbabilityDocumentError> {
        let mut admission = ProbabilityKeyAdmission::new(
            self.meter,
            self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        quire_exact::Reservation::reserve(&mut admission, bytes)
            .map_err(|_| admission.refusal(qsl_eval::simulation::model::ModelKeyError::Denied))
    }
    fn bytes(&mut self, record: &Record) -> Result<Vec<u8>, ProbabilityDocumentError> {
        project_node_key(&record.declaration, self)?;
        self.work(u64::try_from(record.name.as_str().len()).unwrap_or(u64::MAX))?;
        self.encoded(record)
    }
    fn declaration(&mut self, text: &str) -> Result<NodeKey, ProbabilityDocumentError> {
        let mut admission = ProbabilityKeyAdmission::new(
            self.meter,
            self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        qsl_foundation::node_wire::node_key_from_text_with_budget(text, &mut admission).map_err(
            |error| match error {
                qsl_foundation::node_wire::NodeWireError::Denied => {
                    admission.refusal(qsl_eval::simulation::model::ModelKeyError::Denied)
                }
                _ => ProbabilityDocumentError::MalformedMember("objective"),
            },
        )
    }
    fn grow<T>(&mut self, rows: &mut Vec<T>) -> Result<(), ProbabilityDocumentError> {
        if rows.len() == rows.capacity() {
            let capacity = rows.capacity().saturating_mul(2).max(4);
            self.reserve(
                capacity
                    .checked_mul(std::mem::size_of::<T>())
                    .ok_or(ProbabilityDocumentError::KeySizeOverflow)?,
            )?;
            rows.try_reserve_exact(capacity - rows.len())
                .map_err(|_| ProbabilityDocumentError::KeySizeOverflow)?;
        }
        Ok(())
    }
    fn encoded(&mut self, value: &impl Encode) -> Result<Vec<u8>, ProbabilityDocumentError> {
        let mut admission = ProbabilityKeyAdmission::new(
            self.meter,
            self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        struct Reserve<'a, 'b>(&'a mut ProbabilityKeyAdmission<'b>);
        impl quire_canonical::Reservation for Reserve<'_, '_> {
            fn reserve(&mut self, bytes: usize) -> Result<(), quire_canonical::ReservationDenied> {
                quire_exact::IntegerDecimalBudget::work(
                    self.0,
                    u64::try_from(bytes).unwrap_or(u64::MAX),
                )
                .map_err(|_| quire_canonical::ReservationDenied)?;
                quire_exact::Reservation::reserve(self.0, bytes)
                    .map_err(|_| quire_canonical::ReservationDenied)
            }
        }
        let result =
            writer::encode_admitted(value, self.limits.bytes, &mut Reserve(&mut admission));
        result.map_err(|error| match error {
            Error::ReservationDenied { .. } => {
                admission.refusal(qsl_eval::simulation::model::ModelKeyError::Denied)
            }
            Error::Limit(limit) => {
                exceeded(ProbabilityDocumentLimit::Bytes, limit.bound, limit.required)
            }
            _ => ProbabilityDocumentError::MalformedMember("objective"),
        })
    }
}
impl NodeIdentitySink for Admission<'_> {
    type Error = ProbabilityDocumentError;
    fn visit(&mut self) -> Result<(), Self::Error> {
        self.work(1)
    }
    fn begin_object(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn name(&mut self, _: &'static str) -> Result<(), Self::Error> {
        Ok(())
    }
    fn ordinal(&mut self, _: u64) -> Result<(), Self::Error> {
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<(), Self::Error> {
        self.work(u64::try_from(text.len()).unwrap_or(u64::MAX))
    }
    fn begin_array(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end_array(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end_object(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn push_path<'a>(
        &mut self,
        stack: &mut Vec<NodePathIter<'a>>,
        path: NodePathIter<'a>,
    ) -> Result<(), Self::Error> {
        self.grow(stack)?;
        stack.push(path);
        Ok(())
    }
}

pub(super) fn checked(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut quire_exact::Meter,
    storage: &mut u64,
) -> Result<Vec<Record>, ProbabilityDocumentError> {
    checked_with_control(
        system,
        claim,
        limits,
        cancel,
        meter,
        storage,
        &Control::new(cancel),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn checked_with_control(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut quire_exact::Meter,
    storage: &mut u64,
    control: &Control<'_>,
) -> Result<Vec<Record>, ProbabilityDocumentError> {
    admit(claim)?;
    let mut cx = Admission {
        limits,
        cancel,
        meter,
        storage,
    };
    let mut rows: Vec<(Vec<u8>, Record)> = Vec::new();
    for constraint in &claim.fairness {
        cx.work(1)?;
        control.phase(BoundedFairProbabilityPhase::Declaration)?;
        let operation = constraint.operation();
        let key = system
            .declaration_key_with_work(&operation.declaring_type(), &mut |work| cx.work(work))?
            .ok_or(ProbabilityDocumentError::Unsupported)?;
        cx.work(
            u64::try_from(
                key.package
                    .len()
                    .saturating_add(key.node.len())
                    .saturating_add(operation.name().len()),
            )
            .unwrap_or(u64::MAX),
        )?;
        let identity = qsl_semantics::model::key::ModelNodeIdentity::new(
            &key.package,
            "model",
            "object_type",
            &key.node,
        )
        .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?;
        identity.project(&mut cx)?;
        let identity = cx.encoded(&identity)?;
        let declaration = cx.declaration(
            std::str::from_utf8(&identity)
                .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
        )?;
        cx.reserve(operation.name().len())?;
        let record = Record {
            kind: constraint.kind(),
            granularity: constraint.granularity(),
            declaration,
            name: Identifier::new(operation.name())
                .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
        };
        control.phase(BoundedFairProbabilityPhase::Metadata)?;
        let bytes = cx.bytes(&record)?;
        cx.grow(&mut rows)?;
        let mut at = rows.len();
        while at > 0 {
            cx.work(
                u64::try_from(bytes.len().min(rows[at - 1].0.len()))
                    .unwrap_or(u64::MAX)
                    .saturating_add(1),
            )?;
            if rows[at - 1].0 <= bytes {
                break;
            }
            at -= 1;
        }
        if at > 0 && rows[at - 1].0 == bytes {
            continue;
        }
        rows.insert(at, (bytes, record));
    }
    cx.reserve(rows.len().saturating_mul(std::mem::size_of::<Record>()))?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(rows.len())
        .map_err(|_| ProbabilityDocumentError::KeySizeOverflow)?;
    records.extend(rows.into_iter().map(|(_, record)| record));
    Ok(records)
}

pub(super) fn same(
    expected: &[Record],
    offered: &[Record],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut quire_exact::Meter,
    storage: &mut u64,
) -> Result<bool, ProbabilityDocumentError> {
    let mut cx = Admission {
        limits,
        cancel,
        meter,
        storage,
    };
    cx.work(1)?;
    if expected.len() != offered.len() {
        return Ok(false);
    }
    for (expected, offered) in expected.iter().zip(offered) {
        let expected = cx.bytes(expected)?;
        let offered = cx.bytes(offered)?;
        cx.work(
            u64::try_from(expected.len().min(offered.len()))
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        )?;
        if expected != offered {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn admit_encoding(
    records: &[Record],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    let mut meter = quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let mut cx = Admission {
        limits,
        cancel,
        meter: &mut meter,
        storage: &mut storage,
    };
    for record in records {
        cx.bytes(record)?;
    }
    Ok(())
}

pub(super) fn read(
    records: &[Box<serde_json::value::RawValue>],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut quire_exact::Meter,
    storage: &mut u64,
) -> Result<Vec<Record>, ProbabilityDocumentError> {
    let mut cx = Admission {
        limits,
        cancel,
        meter,
        storage,
    };
    let mut result = Vec::new();
    let mut previous: Option<Vec<u8>> = None;
    for raw in records {
        cx.work(u64::try_from(raw.get().len()).unwrap_or(u64::MAX))?;
        let wire: WireRecord<'_> = serde_json::from_str(raw.get())
            .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?;
        cx.reserve(wire.target.name.len())?;
        let record = Record {
            kind: match wire.fairness_kind {
                "weak" => FairnessKind::Weak,
                "strong" => FairnessKind::Strong,
                _ => return Err(ProbabilityDocumentError::MalformedMember("objective")),
            },
            granularity: match wire.granularity {
                "whole" => FairnessGranularity::Whole,
                "each" => FairnessGranularity::Each,
                _ => return Err(ProbabilityDocumentError::MalformedMember("objective")),
            },
            declaration: cx.declaration(wire.target.declaration.get())?,
            name: Identifier::new(wire.target.name)
                .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
        };
        if wire.target.kind != "operation" {
            return Err(ProbabilityDocumentError::MalformedMember("objective"));
        }
        let bytes = cx.bytes(&record)?;
        if previous.as_ref().is_some_and(|previous| previous >= &bytes) {
            return Err(ProbabilityDocumentError::MalformedMember("objective"));
        }
        previous = Some(bytes);
        cx.grow(&mut result)?;
        result.push(record);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn limits() -> ProbabilityDocumentLimits {
        let mut accounting = crate::AccountingLimits::default().0;
        accounting.work_units = u64::MAX;
        ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 64_000_000,
            states: 128,
            edges: 1024,
            scalar_bits: 128,
            accounting,
        }
    }

    #[trace("TC-1406", "FR-201-AC-50")]
    #[test]
    fn shared_structural_metadata_walk_admits_deep_nodes_and_text_before_writing() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let mut declaration = NodeKey::new(
                    "pkg",
                    vec![quire_exact::PathSegment::Text("leaf".repeat(2048))],
                    0,
                )
                .unwrap();
                for _ in 0..1024 {
                    declaration =
                        NodeKey::new("pkg", vec![quire_exact::PathSegment::Node(declaration)], 0)
                            .unwrap();
                }
                let record = Record {
                    kind: FairnessKind::Weak,
                    granularity: FairnessGranularity::Each,
                    declaration,
                    name: Identifier::new("tick").unwrap(),
                };
                let cancel = Cancel::new();
                let mut meter = quire_exact::Meter::new(limits().accounting);
                let mut storage = 0;
                let bytes = Admission {
                    limits: limits(),
                    cancel: &cancel,
                    meter: &mut meter,
                    storage: &mut storage,
                }
                .bytes(&record)
                .unwrap();
                let raw =
                    serde_json::value::RawValue::from_string(String::from_utf8(bytes).unwrap())
                        .unwrap();
                let records = read(&[raw], limits(), &cancel, &mut meter, &mut storage).unwrap();
                assert!(same(
                    std::slice::from_ref(&record),
                    &records,
                    limits(),
                    &cancel,
                    &mut meter,
                    &mut storage
                )
                .unwrap());
                for cap in 0..3 {
                    let mut bound = limits();
                    match cap {
                        0 => bound.bytes = 32,
                        1 => bound.storage_bytes = 0,
                        2 => bound.accounting.work_units = 10,
                        _ => unreachable!(),
                    }
                    let mut meter = quire_exact::Meter::new(bound.accounting);
                    let mut storage = 0;
                    assert!(matches!(
                        Admission {
                            limits: bound,
                            cancel: &cancel,
                            meter: &mut meter,
                            storage: &mut storage
                        }
                        .bytes(&record),
                        Err(ProbabilityDocumentError::Limit { .. }
                            | ProbabilityDocumentError::Accounting(_))
                    ));
                }
                let text =
                    qsl_foundation::node_wire::node_key_to_text(&record.declaration).unwrap();
                let mut bound = limits();
                bound.storage_bytes = 0;
                let mut meter = quire_exact::Meter::new(bound.accounting);
                assert!(matches!(
                    Admission {
                        limits: bound,
                        cancel: &cancel,
                        meter: &mut meter,
                        storage: &mut 0
                    }
                    .declaration(&text),
                    Err(ProbabilityDocumentError::Limit {
                        limit: ProbabilityDocumentLimit::StorageBytes,
                        ..
                    })
                ));
                for cause in [CancelCause::Requested, CancelCause::Deadline] {
                    let cancel = Cancel::new();
                    cancel.cancel(cause);
                    let mut meter = quire_exact::Meter::new(limits().accounting);
                    assert_eq!(
                        Admission {
                            limits: limits(),
                            cancel: &cancel,
                            meter: &mut meter,
                            storage: &mut 0
                        }
                        .declaration(&text),
                        Err(ProbabilityDocumentError::Cancelled(cause))
                    );
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked bounded fairness, using the shared model-operation target identity.

use super::*;
use qsl_foundation::node_wire::{NodeIdentitySink, project_node_key};
use quire_canonical::{Encode, Error, Sink, Writer};
use quire_exact::{Identifier, NodeKey, NodePathIter};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

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
        writer.string(match self.kind { FairnessKind::Weak => "weak", FairnessKind::Strong => "strong" })?;
        writer.name("granularity")?;
        writer.string(match self.granularity { FairnessGranularity::Whole => "whole", FairnessGranularity::Each => "each" })?;
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
struct WireRecord {
    fairness_kind: String,
    granularity: String,
    target: WireTarget,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireTarget {
    kind: String,
    declaration: Box<serde_json::value::RawValue>,
    name: String,
}

pub(super) fn admit(claim: &CheckedProbabilisticClaim) -> Result<(), ProbabilityDocumentError> {
    let ProbForm::Probability { event: qsl_semantics::check::Event::Formula { formula, timed_horizon: None, .. }, .. } = &claim.form else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let profile = qsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    if claim.fairness.is_empty() || !matches!(claim.scheduler, ProbScheduler::Every)
        || !qsl_eval::automaton::QualifiedBoundedMonitor::supports(formula.formula(), profile) {
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
        let mut admission = ProbabilityKeyAdmission::new(self.meter, self.storage, self.limits.storage_bytes, self.cancel);
        quire_exact::IntegerDecimalBudget::work(&mut admission, amount)
            .map_err(|_| admission.refusal(qsl_eval::simulation::model::ModelKeyError::Denied))
    }
    fn reserve(&mut self, bytes: usize) -> Result<(), ProbabilityDocumentError> {
        let mut admission = ProbabilityKeyAdmission::new(self.meter, self.storage, self.limits.storage_bytes, self.cancel);
        quire_exact::Reservation::reserve(&mut admission, bytes)
            .map_err(|_| admission.refusal(qsl_eval::simulation::model::ModelKeyError::Denied))
    }
    fn bytes(&mut self, record: &Record) -> Result<Vec<u8>, ProbabilityDocumentError> {
        project_node_key(&record.declaration, self)?;
        self.work(u64::try_from(record.name.as_str().len()).unwrap_or(u64::MAX))?;
        let mut admission = ProbabilityKeyAdmission::new(self.meter, self.storage, self.limits.storage_bytes, self.cancel);
        let result = writer::encode_admitted(record, self.limits.bytes, &mut admission);
        result.map_err(|error| match error {
            Error::ReservationDenied { .. } => admission.refusal(qsl_eval::simulation::model::ModelKeyError::Denied),
            Error::Limit(limit) => exceeded(ProbabilityDocumentLimit::Bytes, limit.bound, limit.required),
            _ => ProbabilityDocumentError::MalformedMember("objective"),
        })
    }
}
impl NodeIdentitySink for Admission<'_> {
    type Error = ProbabilityDocumentError;
    fn visit(&mut self) -> Result<(), Self::Error> { self.work(1) }
    fn begin_object(&mut self) -> Result<(), Self::Error> { Ok(()) }
    fn name(&mut self, _: &'static str) -> Result<(), Self::Error> { Ok(()) }
    fn ordinal(&mut self, _: u64) -> Result<(), Self::Error> { Ok(()) }
    fn text(&mut self, text: &str) -> Result<(), Self::Error> { self.work(u64::try_from(text.len()).unwrap_or(u64::MAX)) }
    fn begin_array(&mut self) -> Result<(), Self::Error> { Ok(()) }
    fn end_array(&mut self) -> Result<(), Self::Error> { Ok(()) }
    fn end_object(&mut self) -> Result<(), Self::Error> { Ok(()) }
    fn push_path<'a>(&mut self, stack: &mut Vec<NodePathIter<'a>>, path: NodePathIter<'a>) -> Result<(), Self::Error> {
        self.reserve(std::mem::size_of::<NodePathIter<'a>>())?;
        stack.try_reserve_exact(1).map_err(|_| ProbabilityDocumentError::KeySizeOverflow)?;
        stack.push(path);
        Ok(())
    }
}

pub(super) fn checked(
    system: &ModelSystem<'_>, claim: &CheckedProbabilisticClaim,
    limits: ProbabilityDocumentLimits, cancel: &Cancel,
    meter: &mut quire_exact::Meter, storage: &mut u64,
) -> Result<Vec<Record>, ProbabilityDocumentError> {
    admit(claim)?;
    let mut cx = Admission { limits, cancel, meter, storage };
    let mut rows: Vec<(Vec<u8>, Record)> = Vec::new();
    for constraint in &claim.fairness {
        cx.work(1)?;
        let operation = constraint.operation();
        let key = system.declaration_key_with_work(&operation.declaring_type(), &mut |work| cx.work(work))?
            .ok_or(ProbabilityDocumentError::Unsupported)?;
        cx.work(u64::try_from(key.package.len().saturating_add(key.node.len()).saturating_add(operation.name().len())).unwrap_or(u64::MAX))?;
        cx.reserve(key.package.len().saturating_add(key.node.len()).saturating_add(operation.name().len()).saturating_add(3 * std::mem::size_of::<quire_exact::PathSegment>()))?;
        let record = Record {
            kind: constraint.kind(), granularity: constraint.granularity(),
            declaration: qsl_semantics::model::key::object_type_node_key(key).map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
            name: Identifier::new(operation.name()).map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
        };
        let bytes = cx.bytes(&record)?;
        cx.reserve(std::mem::size_of::<(Vec<u8>, Record)>())?;
        rows.try_reserve_exact(1).map_err(|_| ProbabilityDocumentError::KeySizeOverflow)?;
        let mut at = rows.len();
        while at > 0 {
            cx.work(u64::try_from(bytes.len().min(rows[at-1].0.len())).unwrap_or(u64::MAX).saturating_add(1))?;
            if rows[at-1].0 <= bytes { break; }
            at -= 1;
        }
        if at > 0 && rows[at-1].0 == bytes { continue; }
        rows.insert(at, (bytes, record));
    }
    Ok(rows.into_iter().map(|(_, record)| record).collect())
}

pub(super) fn read(
    records: &[Box<serde_json::value::RawValue>], limits: ProbabilityDocumentLimits,
    cancel: &Cancel, meter: &mut quire_exact::Meter,
) -> Result<Vec<Record>, ProbabilityDocumentError> {
    let mut storage = 0;
    let mut cx = Admission { limits, cancel, meter, storage: &mut storage };
    let mut result = Vec::new();
    let mut previous: Option<Vec<u8>> = None;
    for raw in records {
        cx.work(u64::try_from(raw.get().len()).unwrap_or(u64::MAX))?;
        cx.reserve(raw.get().len())?;
        let wire: WireRecord = serde_json::from_str(raw.get()).map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?;
        let record = Record {
            kind: match wire.fairness_kind.as_str() { "weak" => FairnessKind::Weak, "strong" => FairnessKind::Strong, _ => return Err(ProbabilityDocumentError::MalformedMember("objective")) },
            granularity: match wire.granularity.as_str() { "whole" => FairnessGranularity::Whole, "each" => FairnessGranularity::Each, _ => return Err(ProbabilityDocumentError::MalformedMember("objective")) },
            declaration: qsl_foundation::node_wire::node_key_from_text(wire.target.declaration.get()).map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
            name: Identifier::new(wire.target.name).map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
        };
        if wire.target.kind != "operation" { return Err(ProbabilityDocumentError::MalformedMember("objective")); }
        let bytes = cx.bytes(&record)?;
        if previous.as_ref().is_some_and(|previous| previous >= &bytes) { return Err(ProbabilityDocumentError::MalformedMember("objective")); }
        previous = Some(bytes);
        cx.reserve(std::mem::size_of::<Record>())?;
        result.try_reserve_exact(1).map_err(|_| ProbabilityDocumentError::KeySizeOverflow)?;
        result.push(record);
    }
    Ok(result)
}

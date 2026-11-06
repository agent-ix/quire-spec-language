// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-052: immutable reports of the checked-package temporal evaluator.

use qsl_eval::temporal::evidence::{
    EvidenceIncomplete, EvidenceLedger, EvidenceRequest, EvidenceResult, NonValueKind,
    ObligationKey, Relation, Settlement, Truth,
};
use qsl_eval::temporal::Valuation;
use qsl_package::checked_views::{temporal_subject, ClauseOccurrence};
use qsl_package::BarePackage;
use quire_contract_model::CheckedPackageV2;
use quire_exact::Meter;

use super::owner_codec::{BindingDimension, Reader, ReportRefusal, Writer};
use super::{ValuationEvidence, ValuationRow};

/// Opaque observation-owner references. Neither member is interpreted by QSL.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationRef {
    /// The observation reference.
    pub reference: String,
    /// The revision reference.
    pub revision: String,
}

/// An actual checked occurrence and the evidence authority used for its report.
/// No truth, settlement or support can be supplied through this type.
pub struct CheckedSubject<'a> {
    package: &'a CheckedPackageV2,
    occurrence: &'a ClauseOccurrence,
    evidence: &'a ValuationEvidence,
    request: &'a EvidenceRequest,
    binding: Vec<Vec<u8>>,
}

impl<'a> CheckedSubject<'a> {
    /// Bind an admitted occurrence and valuation relation to the native call.
    ///
    /// # Errors
    /// Refuses a foreign occurrence, an unadmitted row position or a request
    /// selecting another occurrence. Row admission remains `ValuationEvidence`'s.
    pub fn new(
        package: &'a CheckedPackageV2,
        occurrence: &'a ClauseOccurrence,
        evidence: &'a ValuationEvidence,
        request: &'a EvidenceRequest,
    ) -> Result<Self, ReportRefusal> {
        let view = temporal_subject(package, &occurrence.node)
            .map_err(|_| ReportRefusal::BindingMismatch(BindingDimension::Clause))?;
        view.arena_of(occurrence)
            .map_err(|_| ReportRefusal::BindingMismatch(BindingDimension::Occurrence))?;
        let selected = qsl_package::CheckedOccurrence {
            role: occurrence.role.clone(),
            ordinal: occurrence.ordinal,
        };
        if request
            .occurrence
            .as_ref()
            .is_some_and(|key| *key != selected)
            || (request.occurrence.is_none() && view.occurrences().len() != 1)
        {
            return Err(ReportRefusal::BindingMismatch(BindingDimension::Occurrence));
        }
        for row in evidence.rows() {
            if row.subject != *occurrence || !view.predicate_leaves().contains(&row.operand) {
                return Err(ReportRefusal::SupportNotInTrace);
            }
            if row.position >= request.positions.len() {
                return Err(ReportRefusal::SupportNotInTrace);
            }
        }
        let binding = vec![
            package.name().as_bytes().to_vec(),
            b"quire.checked-package/v2".to_vec(),
            encoded(&occurrence.node)?,
            encoded(&selected)?,
            encoded(view.source_span())?,
        ];
        Ok(Self {
            package,
            occurrence,
            evidence,
            request,
            binding,
        })
    }

    fn predecessor(&self) -> Result<Option<&[u8]>, ReportRefusal> {
        let identity = match &self.request.relation {
            Relation::Original => return Ok(None),
            Relation::Superseding(identity) | Relation::Invalidating(identity) => identity,
        };
        let occurrence = qsl_package::CheckedOccurrence {
            role: self.occurrence.role.clone(),
            ordinal: self.occurrence.ordinal,
        };
        let clause = encoded(&(&self.occurrence.node, occurrence))?;
        let obligation = ObligationKey::new(
            self.occurrence.node.package.clone(),
            clause,
            self.occurrence.ordinal,
        );
        let latest = self
            .request
            .ledger
            .as_ref()
            .and_then(|ledger| ledger.result(&obligation, self.request.trigger.as_ref()))
            .ok_or(ReportRefusal::InvalidPredecessor)?;
        if latest.result_identity() != identity {
            return Err(ReportRefusal::InvalidPredecessor);
        }
        Ok(Some(identity.as_bytes()))
    }
}

fn encoded(value: &impl serde::Serialize) -> Result<Vec<u8>, ReportRefusal> {
    serde_json::to_vec(value).map_err(|_| ReportRefusal::UnknownTag)
}

/// The native outcome class; missing evidence never becomes false.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportOutcome {
    /// A temporal truth was computed.
    Evaluated,
    /// The obligation is inactive.
    Inactive,
    /// Its activation cannot yet be established.
    ActivationUnknown,
    /// A required fact is missing.
    Missing,
    /// An admitted fact is refused.
    Refused,
    /// The obligation never activated.
    Unactivated,
    /// A closed window remains indeterminate.
    Indeterminate,
    /// A named evidence counter reached its ceiling.
    Limit,
    /// The work meter stopped the call.
    Meter,
    /// Progress storage reached its limit.
    CoveredPositions,
    /// Aggregate storage reached its limit.
    WindowLimit,
    /// An instance valuation was evicted.
    EvictedValuation,
}

impl ReportOutcome {
    fn tag(self) -> u8 {
        match self {
            Self::Evaluated => 0,
            Self::Inactive => 1,
            Self::ActivationUnknown => 2,
            Self::Missing => 3,
            Self::Refused => 4,
            Self::Unactivated => 5,
            Self::Indeterminate => 6,
            Self::Limit => 7,
            Self::Meter => 8,
            Self::CoveredPositions => 9,
            Self::WindowLimit => 10,
            Self::EvictedValuation => 11,
        }
    }
    fn read(tag: u8) -> Result<Self, ReportRefusal> {
        match tag {
            0 => Ok(Self::Evaluated),
            1 => Ok(Self::Inactive),
            2 => Ok(Self::ActivationUnknown),
            3 => Ok(Self::Missing),
            4 => Ok(Self::Refused),
            5 => Ok(Self::Unactivated),
            6 => Ok(Self::Indeterminate),
            7 => Ok(Self::Limit),
            8 => Ok(Self::Meter),
            9 => Ok(Self::CoveredPositions),
            10 => Ok(Self::WindowLimit),
            11 => Ok(Self::EvictedValuation),
            _ => Err(ReportRefusal::UnknownTag),
        }
    }
}

/// One support use, retaining the trace ordinal and all six row-key facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportSupport {
    /// The ordinal in the admitted trace.
    pub position: u64,
    /// The admitted valuation row, including its six-fact key.
    pub row: ValuationRow,
}

/// An immutable native report. Only evaluation and strict reading construct it.
#[derive(Clone, Debug)]
pub struct Report {
    binding: Vec<Vec<u8>>,
    observation: ObservationRef,
    outcome: ReportOutcome,
    truth: Option<Truth>,
    settlement: Option<Settlement>,
    support: Vec<ReportSupport>,
    identity: Vec<u8>,
    predecessor: Option<Vec<u8>>,
    ledger: Option<EvidenceLedger>,
}

impl PartialEq for Report {
    fn eq(&self, other: &Self) -> bool {
        self.binding == other.binding
            && self.observation == other.observation
            && self.outcome == other.outcome
            && self.truth == other.truth
            && self.settlement == other.settlement
            && self.support == other.support
            && self.identity == other.identity
            && self.predecessor == other.predecessor
    }
}
impl Eq for Report {}

impl Report {
    /// The computed outcome class.
    pub fn outcome(&self) -> ReportOutcome {
        self.outcome
    }
    /// The computed temporal truth, absent for a non-value.
    pub fn truth(&self) -> Option<Truth> {
        self.truth
    }
    /// The computed settlement.
    pub fn settlement(&self) -> Option<Settlement> {
        self.settlement
    }
    /// Support uses in evaluator order, with duplicate rows retained.
    pub fn support(&self) -> &[ReportSupport] {
        &self.support
    }
    /// The echoed observation and revision references.
    pub fn observation(&self) -> &ObservationRef {
        &self.observation
    }
    /// The evaluator's FR-043 result identity, never a content digest.
    pub fn result_identity(&self) -> &[u8] {
        &self.identity
    }
    /// The exact direct predecessor's result identity.
    pub fn predecessor(&self) -> Option<&[u8]> {
        self.predecessor.as_deref()
    }
    /// The evaluator's next-call ledger. A decoded report grants no ledger authority.
    pub fn ledger(&self) -> Option<&EvidenceLedger> {
        self.ledger.as_ref()
    }
    /// The package identity, version, node, occurrence and source-map binding.
    pub fn binding(&self) -> &[Vec<u8>] {
        &self.binding
    }

    /// Write the deterministic, ordered and self-delimiting owner representation.
    pub fn to_owner_bytes(&self) -> Vec<u8> {
        let mut writer = Writer::new();
        for (tag, value) in (1_u8..=5).zip(&self.binding) {
            writer.field(tag, value);
        }
        writer.field(6, self.observation.reference.as_bytes());
        writer.field(7, self.observation.revision.as_bytes());
        writer.field(
            8,
            &[
                self.outcome.tag(),
                truth_tag(self.truth),
                settlement_tag(self.settlement),
            ],
        );
        let mut support = Writer::new();
        support.field(1, &count_bytes(self.support.len()));
        for entry in &self.support {
            let mut row = Writer::new();
            row.field(1, &entry.position.to_le_bytes());
            row.field(2, &row_key(&entry.row));
            support.field(2, &row.finish());
        }
        writer.field(9, &support.finish());
        writer.field(10, &self.identity);
        let mut predecessor = Writer::new();
        match &self.predecessor {
            None => predecessor.field(1, &[0]),
            Some(identity) => {
                predecessor.field(1, &[1]);
                predecessor.field(2, identity);
            }
        }
        writer.field(11, &predecessor.finish());
        writer.finish()
    }

    /// Strictly admit owner bytes against the checked subject and observation.
    ///
    /// # Errors
    /// Returns the closed `ReportRefusal`, with no partial report.
    pub fn from_owner_bytes(
        bytes: &[u8],
        subject: &CheckedSubject<'_>,
        observation: &ObservationRef,
    ) -> Result<Self, ReportRefusal> {
        let mut reader = Reader::new(bytes)?;
        let dimensions = [
            BindingDimension::PackageIdentity,
            BindingDimension::PackageVersion,
            BindingDimension::Clause,
            BindingDimension::Occurrence,
            BindingDimension::SourceMap,
        ];
        for ((tag, expected), dimension) in (1_u8..=5).zip(&subject.binding).zip(dimensions) {
            if reader.field(tag)? != expected {
                return Err(ReportRefusal::BindingMismatch(dimension));
            }
        }
        if reader.field(6)? != observation.reference.as_bytes() {
            return Err(ReportRefusal::BindingMismatch(
                BindingDimension::Observation,
            ));
        }
        if reader.field(7)? != observation.revision.as_bytes() {
            return Err(ReportRefusal::BindingMismatch(BindingDimension::Revision));
        }
        let outcome_bytes = reader.field(8)?;
        let [outcome, truth, settlement] = outcome_bytes else {
            return Err(ReportRefusal::UnknownTag);
        };
        let outcome = ReportOutcome::read(*outcome)?;
        let truth = read_truth(*truth)?;
        let settlement = read_settlement(*settlement)?;
        if (outcome == ReportOutcome::Evaluated) != truth.is_some() {
            return Err(ReportRefusal::UnknownTag);
        }
        let mut rows = Reader::new(reader.field(9)?)?;
        let count = read_u64(rows.field(1)?)?;
        let mut support = Vec::new();
        for _ in 0..count {
            let mut entry = Reader::new(rows.field(2)?)?;
            let position = read_u64(entry.field(1)?)?;
            let key = entry.field(2)?;
            let row = subject
                .evidence
                .rows()
                .iter()
                .find(|row| {
                    u64::try_from(row.position) == Ok(position)
                        && row_key(row) == key
                        && row.revision == observation.revision
                        && row.position < subject.request.positions.len()
                })
                .ok_or(ReportRefusal::SupportNotInTrace)?;
            entry.finish()?;
            support.push(ReportSupport {
                position,
                row: row.clone(),
            });
        }
        rows.finish()?;
        let identity = reader.field(10)?.to_vec();
        let mut previous = Reader::new(reader.field(11)?)?;
        let predecessor = match previous.field(1)? {
            [0] => None,
            [1] => Some(previous.field(2)?.to_vec()),
            _ => return Err(ReportRefusal::UnknownTag),
        };
        previous.finish()?;
        reader.finish()?;
        if predecessor.as_deref() != subject.predecessor()?
            || predecessor.as_deref() == Some(&identity)
        {
            return Err(ReportRefusal::InvalidPredecessor);
        }
        Ok(Self {
            binding: subject.binding.clone(),
            observation: observation.clone(),
            outcome,
            truth,
            settlement,
            support,
            identity,
            predecessor,
            ledger: None,
        })
    }
}

fn count_bytes(count: usize) -> [u8; 8] {
    let mut result = [0; 8];
    for (to, from) in result.iter_mut().zip(count.to_le_bytes()) {
        *to = from;
    }
    result
}
fn read_u64(bytes: &[u8]) -> Result<u64, ReportRefusal> {
    let bytes = bytes.try_into().map_err(|_| ReportRefusal::Truncated)?;
    Ok(u64::from_le_bytes(bytes))
}
fn row_key(row: &ValuationRow) -> Vec<u8> {
    let mut writer = Writer::new();
    // Checked node identities and occurrence records have deterministic owner
    // serialization; they are length-delimited identity members, not artifacts.
    writer.field(
        1,
        &serde_json::to_vec(&row.subject.node).expect("checked node identity encodes"),
    );
    writer.field(
        2,
        &serde_json::to_vec(&row.subject.role).expect("checked role encodes"),
    );
    writer.field(3, &row.subject.ordinal.to_le_bytes());
    writer.field(4, &count_bytes(row.position));
    writer.field(5, row.anchor.as_bytes());
    writer.field(6, row.capture_instance.as_bytes());
    writer.field(7, row.revision.as_bytes());
    writer.field(
        8,
        &serde_json::to_vec(&row.operand).expect("checked operand identity encodes"),
    );
    writer.finish()
}
fn truth_tag(value: Option<Truth>) -> u8 {
    match value {
        None => 0,
        Some(Truth::True) => 1,
        Some(Truth::False) => 2,
        Some(Truth::Pending) => 3,
    }
}
fn read_truth(tag: u8) -> Result<Option<Truth>, ReportRefusal> {
    match tag {
        0 => Ok(None),
        1 => Ok(Some(Truth::True)),
        2 => Ok(Some(Truth::False)),
        3 => Ok(Some(Truth::Pending)),
        _ => Err(ReportRefusal::UnknownTag),
    }
}
fn settlement_tag(value: Option<Settlement>) -> u8 {
    match value {
        None => 0,
        Some(Settlement::ClosedScope) => 1,
        Some(Settlement::DecisiveWitness) => 2,
        Some(Settlement::DecisiveCounterexample) => 3,
        Some(Settlement::Unsettled) => 4,
        Some(Settlement::Unavailable) => 5,
    }
}
fn read_settlement(tag: u8) -> Result<Option<Settlement>, ReportRefusal> {
    match tag {
        0 => Ok(None),
        1 => Ok(Some(Settlement::ClosedScope)),
        2 => Ok(Some(Settlement::DecisiveWitness)),
        3 => Ok(Some(Settlement::DecisiveCounterexample)),
        4 => Ok(Some(Settlement::Unsettled)),
        5 => Ok(Some(Settlement::Unavailable)),
        _ => Err(ReportRefusal::UnknownTag),
    }
}

/// A native report call refused before a report could be constructed.
#[derive(Debug, thiserror::Error)]
pub enum NativeReportError {
    /// Report binding, valuation or predecessor refusal.
    #[error(transparent)]
    Report(#[from] ReportRefusal),
    /// The checked temporal engine refused the call.
    #[error(transparent)]
    Evaluation(#[from] quire_eval::CallFailure),
    /// The checked package cannot supply the temporal engine's view.
    #[error(transparent)]
    Package(#[from] qsl_eval::temporal::evidence::PackageMismatch),
}

/// Evaluate an actual checked occurrence using the one native temporal engine.
/// Valuations come only from the admitted six-bound relation. Request valuation
/// maps are replaced by that relation, including absent/conflicting valuations.
///
/// # Errors
/// Typed binding, predecessor and checked-engine refusals return no report.
pub fn evaluate_v2(
    subject: &CheckedSubject<'_>,
    observation: &ObservationRef,
    meter: &mut Meter,
) -> Result<Report, NativeReportError> {
    let view = temporal_subject(subject.package, &subject.occurrence.node)
        .map_err(|_| ReportRefusal::BindingMismatch(BindingDimension::Clause))?;
    for row in subject.evidence.rows() {
        if row.revision != observation.revision {
            return Err(ReportRefusal::BindingMismatch(BindingDimension::Revision).into());
        }
        if subject.request.anchor.as_deref() != Some(row.anchor.as_str()) {
            return Err(ReportRefusal::SupportNotInTrace.into());
        }
    }
    let predecessor = subject.predecessor()?.map(<[u8]>::to_vec);
    let bare = BarePackage::new(subject.package);
    let package = qsl_eval::temporal::evidence::EvidencePackage::from_package(&bare)?;
    let mut request = subject.request.clone();
    request.correspondence.identity = observation.reference.clone();
    request.correspondence.authority_revision = observation.revision.clone();
    let mut native = Writer::new();
    for row in subject.evidence.rows() {
        let mut entry = Writer::new();
        entry.field(1, &row_key(row));
        entry.field(2, &[u8::from(row.value)]);
        native.field(1, &entry.finish());
    }
    request.native_valuations = native.finish();
    let operands = qsl_eval::temporal::evidence::native_atom_operands(&view);
    request.occurrence = Some(qsl_package::CheckedOccurrence {
        role: subject.occurrence.role.clone(),
        ordinal: subject.occurrence.ordinal,
    });
    for position in &mut request.positions {
        position.valuations.clear();
        position.instance_valuations.clear();
        position.aggregate_samples.clear();
        position.instance_aggregate_samples.clear();
    }
    for (position, input) in request.positions.iter_mut().enumerate() {
        for (atom, operand) in operands.iter().enumerate() {
            let Some(operand) = operand else {
                continue;
            };
            let valuation = match subject
                .evidence
                .value_of(subject.occurrence, position, operand)
            {
                Ok(true) => Valuation::True,
                Ok(false) => Valuation::False,
                Err(_) => Valuation::Missing,
            };
            let atom = u64::try_from(atom).map_err(|_| ReportRefusal::SupportNotInTrace)?;
            if let Some(trigger) = &request.trigger {
                input
                    .instance_valuations
                    .entry(trigger.clone())
                    .or_default()
                    .insert(atom, valuation.clone());
            }
            input.valuations.insert(atom, valuation);
        }
    }
    let result = package.evaluate_temporal_evidence(&subject.occurrence.node, &request, meter)?;
    let outcome = outcome_of(&result);
    let mut support = Vec::new();
    for used in &result.support {
        let atom = usize::try_from(used.atom).map_err(|_| ReportRefusal::SupportNotInTrace)?;
        let operand = operands
            .get(atom)
            .and_then(Option::as_ref)
            .ok_or(ReportRefusal::SupportNotInTrace)?;
        for row in subject
            .evidence
            .rows()
            .iter()
            .filter(|row| row.position == used.position && row.operand == *operand)
        {
            support.push(ReportSupport {
                position: u64::try_from(used.position)
                    .map_err(|_| ReportRefusal::SupportNotInTrace)?,
                row: row.clone(),
            });
        }
    }
    Ok(Report {
        binding: subject.binding.clone(),
        observation: observation.clone(),
        outcome,
        truth: if outcome == ReportOutcome::Evaluated {
            result.truth
        } else {
            None
        },
        settlement: result.settlement,
        support,
        identity: result.result_identity.as_bytes().to_vec(),
        predecessor,
        ledger: Some(result.ledger),
    })
}

fn outcome_of(result: &EvidenceResult) -> ReportOutcome {
    if let Some(incomplete) = &result.incomplete {
        return match incomplete {
            EvidenceIncomplete::Limit(_) => ReportOutcome::Limit,
            EvidenceIncomplete::Meter { .. } => ReportOutcome::Meter,
            EvidenceIncomplete::CoveredPositions(_) => ReportOutcome::CoveredPositions,
            EvidenceIncomplete::WindowLimit(_) => ReportOutcome::WindowLimit,
            EvidenceIncomplete::EvictedValuation { .. } => ReportOutcome::EvictedValuation,
        };
    }
    if let Some(non_value) = &result.non_value {
        return match non_value.kind {
            NonValueKind::Inactive => ReportOutcome::Inactive,
            NonValueKind::ActivationUnknown => ReportOutcome::ActivationUnknown,
            NonValueKind::Missing => ReportOutcome::Missing,
            NonValueKind::Refused => ReportOutcome::Refused,
            NonValueKind::Unactivated => ReportOutcome::Unactivated,
        };
    }
    if result.indeterminate.is_some() {
        ReportOutcome::Indeterminate
    } else {
        ReportOutcome::Evaluated
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! `planner::settle` (QSL FR-529, QSpec FR-488 "Settlement" and FR-492 "Slot
//! outcome" and "Decision order"): settle one planner-local implementation
//! guarantee from its admitted request.
//!
//! The planner reads every premise from the request and nothing else. It
//! never writes `refuted` ([`PlannerValue`] has no such variant), never
//! promotes a `tested` premise, and caps trust at the weakest deciding
//! premise.

use std::collections::BTreeSet;

use qsl_foundation::source::provenance::SourceRegion;
use qsl_foundation::RequestIndex;
use serde_json::{json, Map, Value};

use super::declaration::{
    ClauseFacts, DerivedSlot, ItemRole, PlannerDeclaration, SlotBacking, SlotRole,
};
use super::key::{ExtentClass, FairnessKey, KeyWireError};
use super::request::{
    EnvironmentRecord, PremiseResult, RecordSupplies, SettlementRequest, SlotFill,
};
use crate::outcome::{ItemLabel, OutcomeItem};
use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, PremiseOutcome, SettlementBasis,
    TerminalRecord, TerminalValue, TransferReason, UnavailabilityCause,
};
use qsl_foundation::node_wire::node_key_to_json;
use quire_exact::NodeKey;

use super::compose::{settle_composition, DischargeRecord};
use super::entry::LimitFacts;
use super::request::DEFAULT_MAX_WORK;

/// The planner's work budget (QSpec FR-492 `limits.max_work`, FR-461 setting
/// `planner.max_work`): one unit per slot read, per pool entry compared and
/// per fixed-point iteration checked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Budget {
    /// The units the planner may spend when the request sets none.
    pub max_work: u64,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_work: DEFAULT_MAX_WORK,
        }
    }
}

/// Why a settlement could not run. These are caller defects, not requests
/// the planner refuses: a request from [`super::read_settlement_request`]
/// against the same declaration never hits [`Self::Mismatch`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SettleRefusal {
    /// The request does not belong to the declaration (slot count, roles or
    /// item node differ).
    #[error("the request does not belong to the declaration")]
    Mismatch,
    /// A `fairness-correspondence` slot carries no constraint.
    #[error("the fairness-correspondence slot {index} carries no constraint")]
    SlotWithoutConstraint {
        /// The slot index.
        index: usize,
    },
    /// A composition declaration whose rule is not `assume-guarantee`: only
    /// that rule is planner-local.
    #[error("the composition rule is not `assume-guarantee`")]
    UnsupportedRule,
    /// A fairness constraint could not be written to compare it.
    #[error("a fairness constraint could not be written")]
    Constraint(#[from] KeyWireError),
}

/// The item's FR-360 label: never `refuted` (QSpec FR-488).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlannerValue {
    /// Every needed slot is `proved` or `supplied`.
    Proved,
    /// A slot is neither, or the clause is outside the fragment.
    Inconclusive,
    /// The deciding slot is `unsupported`.
    Unsupported,
    /// The deciding slot is `incomplete`, or the planner's budget ran out.
    Incomplete,
}

impl PlannerValue {
    /// The FR-360 label spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proved => "proved",
            Self::Inconclusive => "inconclusive",
            Self::Unsupported => "unsupported",
            Self::Incomplete => "incomplete",
        }
    }

    /// The FR-243 basis of the label.
    pub const fn basis(self) -> SettlementBasis {
        match self {
            Self::Proved => SettlementBasis::DecisiveWitness,
            Self::Inconclusive => SettlementBasis::Unsettled,
            Self::Unsupported | Self::Incomplete => SettlementBasis::Unavailable,
        }
    }
}

/// A typed cause of a slot outcome or of the item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlannerCause {
    /// An `inconclusive` cause, a planner-only one included.
    Inconclusive(InconclusiveCause),
    /// An `unsupported` cause, or the diagnostic the entry carried instead.
    Unsupported {
        /// The cause.
        cause: UnavailabilityCause,
        /// The entry's FR-272 diagnostic, written in place of the cause.
        diagnostic: Option<Value>,
    },
    /// An `incomplete` cause.
    Incomplete {
        /// The cause.
        cause: IncompleteCause,
        /// The limit a `limit-reached` cause names.
        limit: Option<LimitFacts>,
    },
    /// The planner's own work budget stopped it: `limit-reached{limit:
    /// max_work, value, setting: planner.max_work}`.
    WorkLimit {
        /// The budget.
        value: u64,
    },
}

/// A slot's outcome value (QSpec FR-492 "Slot outcome").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotValue {
    /// The premise result's FR-331 value.
    Label(ItemLabel),
    /// A filled environment record, listed as an assumption.
    Supplied,
    /// An unfilled slot.
    Absent,
    /// A premise supplied only as an assumption (composition).
    Assumption,
}

impl SlotValue {
    /// The wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Label(label) => label_str(label),
            Self::Supplied => "supplied",
            Self::Absent => "absent",
            Self::Assumption => "assumption",
        }
    }

    fn premise_outcome(self) -> PremiseOutcome {
        match self {
            Self::Label(label) => PremiseOutcome::Label(label),
            Self::Supplied | Self::Absent => PremiseOutcome::Absent,
            Self::Assumption => PremiseOutcome::Assumption,
        }
    }

    pub(super) fn settles(self) -> bool {
        matches!(self, Self::Label(ItemLabel::Proved) | Self::Supplied)
    }
}

pub(super) fn label_str(label: ItemLabel) -> &'static str {
    match label {
        ItemLabel::Proved => "proved",
        ItemLabel::Tested => "tested",
        ItemLabel::Refuted => "refuted",
        ItemLabel::Declined => "declined",
        ItemLabel::Unsupported => "unsupported",
        ItemLabel::Incomplete => "incomplete",
        ItemLabel::Inconclusive => "inconclusive",
        ItemLabel::Failed => "failed",
        ItemLabel::Measured => "measured",
        ItemLabel::Pending => "pending",
        ItemLabel::Indeterminate => "indeterminate",
        ItemLabel::Undefined => "undefined",
    }
}

/// One slot's outcome, with the source region of the declaration member that
/// produced the slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotOutcome {
    /// The slot index.
    pub slot: usize,
    /// The slot's role.
    pub role: SlotRole,
    /// The outcome value.
    pub value: SlotValue,
    /// A `proved` slot's certification.
    pub certification: Option<Certification>,
    /// A premise result's extent classification.
    pub extent: Option<ExtentClass>,
    /// A `bounded` extent's bounds, as written.
    pub bounds: Vec<Value>,
    /// The slot's typed cause, when it has one.
    pub cause: Option<PlannerCause>,
    /// The declaration member that produced the slot (QSpec FR-322 source
    /// map).
    pub source: Option<SourceRegion>,
}

/// The extent of a proved guarantee (QSpec FR-488 "Conclusion record").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConclusionExtent {
    /// Every premise is unbounded.
    Unbounded,
    /// The bound of each bounded premise, by slot index.
    Bounded(Vec<(usize, Vec<Value>)>),
}

/// QSpec FR-488 "Conclusion record": one source-bound record of a `proved`
/// guarantee.
#[derive(Clone, Debug)]
pub struct Conclusion {
    /// The checked package's identity.
    pub package: String,
    /// The declaration node.
    pub declaration: NodeKey,
    /// The FR-353 relation node.
    pub relation: Option<NodeKey>,
    /// Every `RustPath` of the relation and the declaration.
    pub sources: Vec<String>,
    /// The clause `P` and `F_P`.
    pub clause: Option<ClauseFacts>,
    /// Every slot's outcome.
    pub slots: Vec<SlotOutcome>,
    /// The environment records used as assumptions.
    pub assumptions: Vec<EnvironmentRecord>,
    /// The proof extent.
    pub extent: ConclusionExtent,
    /// Each proved premise's certification, by slot index.
    pub certifications: Vec<(usize, Certification)>,
}

/// QSpec FR-492 "Planner result".
#[derive(Clone, Debug)]
pub struct PlannerResult {
    /// The item's role and node.
    pub item: (ItemRole, NodeKey),
    /// The FR-360 label.
    pub value: PlannerValue,
    /// The item's cause, on every label but `proved`.
    pub cause: Option<PlannerCause>,
    /// A `proved` item's certification: `trusted` when any deciding premise
    /// is, else `uncertified` when any is, else `certified`.
    pub certification: Option<Certification>,
    /// Every slot's outcome, in slot order.
    pub slots: Vec<SlotOutcome>,
    /// A `proved` guarantee's conclusion record.
    pub conclusion: Option<Conclusion>,
    /// A composition's premise discharge record (QSpec FR-489 "Outputs"): one
    /// entry per discharged slot, also when the item does not settle
    /// `proved`.
    pub discharge: Option<DischargeRecord>,
}

/// Settle `declaration` from `request`.
///
/// # Errors
///
/// [`SettleRefusal`] when the request does not belong to the declaration, a
/// composition's rule is not `assume-guarantee`, or a fairness constraint
/// cannot be written.
pub fn settle(
    declaration: &PlannerDeclaration,
    request: &SettlementRequest,
    budget: Budget,
) -> Result<PlannerResult, SettleRefusal> {
    if request.slots.len() != declaration.slots.len()
        || request.item.node != declaration.node
        || request.item.role != declaration.role
        || request
            .slots
            .iter()
            .zip(&declaration.slots)
            .any(|(slot, derived)| slot.role != derived.role)
    {
        return Err(SettleRefusal::Mismatch);
    }
    if declaration.role == ItemRole::Composition {
        return settle_composition(declaration, request, budget);
    }
    let item = (declaration.role, declaration.node.clone());
    // Before any slot: a clause outside the stuttering-invariant fragment.
    if let Some(operator) = declaration
        .clause
        .as_ref()
        .and_then(|clause| clause.fragment_operator.clone())
    {
        return Ok(PlannerResult {
            item,
            value: PlannerValue::Inconclusive,
            cause: Some(PlannerCause::Inconclusive(
                InconclusiveCause::TransferNotPreserving {
                    reason: TransferReason::Fragment { operator },
                },
            )),
            certification: None,
            slots: Vec::new(),
            conclusion: None,
            discharge: None,
        });
    }

    let max_work = request.max_work.unwrap_or(budget.max_work);
    let supplied = supplied_names(request);
    let mut slots = Vec::with_capacity(declaration.slots.len());
    for (index, (derived, request_slot)) in declaration.slots.iter().zip(&request.slots).enumerate()
    {
        // One work unit per slot read.
        if u64::try_from(index).unwrap_or(u64::MAX) >= max_work {
            return Ok(PlannerResult {
                item,
                value: PlannerValue::Incomplete,
                cause: Some(PlannerCause::WorkLimit { value: max_work }),
                certification: None,
                slots,
                conclusion: None,
                discharge: None,
            });
        }
        slots.push(slot_outcome(
            declaration,
            request,
            index,
            derived,
            &request_slot.fill,
            &supplied,
        )?);
    }

    let (value, cause) = decide(&slots, |_| false);
    let certification = (value == PlannerValue::Proved).then(|| fold_certification(&slots));
    let conclusion =
        (value == PlannerValue::Proved).then(|| conclusion(declaration, request, &slots));
    Ok(PlannerResult {
        item,
        value,
        cause,
        certification,
        slots,
        conclusion,
        discharge: None,
    })
}

/// QSpec FR-492 "Decision order": the first slot, in slot order, whose
/// outcome is neither `proved` nor `supplied` decides the item. A slot
/// `deferred` to another slot (a cycle member waiting on its `fixed-point`
/// slot) is passed over, and decides only when no other slot does.
pub(super) fn decide(
    slots: &[SlotOutcome],
    deferred: impl Fn(&SlotOutcome) -> bool,
) -> (PlannerValue, Option<PlannerCause>) {
    let deciding = slots
        .iter()
        .find(|outcome| !outcome.value.settles() && !deferred(outcome))
        .or_else(|| slots.iter().find(|outcome| !outcome.value.settles()));
    let Some(outcome) = deciding else {
        return (PlannerValue::Proved, None);
    };
    match outcome.value {
        SlotValue::Label(ItemLabel::Unsupported) => {
            (PlannerValue::Unsupported, outcome.cause.clone())
        }
        SlotValue::Label(ItemLabel::Incomplete) => {
            (PlannerValue::Incomplete, outcome.cause.clone())
        }
        other => (
            PlannerValue::Inconclusive,
            Some(PlannerCause::Inconclusive(
                InconclusiveCause::PremiseUnproved {
                    premise: u64::try_from(outcome.slot).unwrap_or(u64::MAX),
                    result: other.premise_outcome(),
                },
            )),
        ),
    }
}

/// The names of the filled environment records.
fn supplied_names(request: &SettlementRequest) -> BTreeSet<&str> {
    request
        .slots
        .iter()
        .filter_map(|slot| match &slot.fill {
            SlotFill::Record(record) => Some(record.name.as_str()),
            _ => None,
        })
        .collect()
}

/// `trusted` when any deciding premise is `trusted`, else `uncertified` when
/// any is, else `certified` (QSpec FR-354).
pub(super) fn fold_certification(slots: &[SlotOutcome]) -> Certification {
    let proved = |wanted| {
        slots.iter().any(|slot| {
            slot.value == SlotValue::Label(ItemLabel::Proved) && slot.certification == Some(wanted)
        })
    };
    if proved(Certification::Trusted) {
        Certification::Trusted
    } else if proved(Certification::Uncertified) {
        Certification::Uncertified
    } else {
        Certification::Certified
    }
}

fn slot_outcome(
    declaration: &PlannerDeclaration,
    request: &SettlementRequest,
    index: usize,
    derived: &DerivedSlot,
    fill: &SlotFill,
    supplied: &BTreeSet<&str>,
) -> Result<SlotOutcome, SettleRefusal> {
    let bare = |value, certification, cause| SlotOutcome {
        slot: index,
        role: derived.role,
        value,
        certification,
        extent: None,
        bounds: Vec::new(),
        cause,
        source: derived.source.clone(),
    };
    Ok(match (&derived.backing, fill) {
        (SlotBacking::Item, SlotFill::Result(result)) => {
            item_outcome(index, derived, result, supplied)
        }
        (SlotBacking::PlannerLocal, _) => {
            let constraint = derived
                .constraint
                .as_ref()
                .ok_or(SettleRefusal::SlotWithoutConstraint { index })?;
            if fairness_implied(declaration, request, derived) {
                let mut outcome = bare(
                    SlotValue::Label(ItemLabel::Proved),
                    Some(Certification::Certified),
                    None,
                );
                outcome.extent = Some(ExtentClass::Unbounded);
                outcome
            } else {
                bare(
                    SlotValue::Label(ItemLabel::Inconclusive),
                    None,
                    Some(PlannerCause::Inconclusive(
                        InconclusiveCause::TransferNotPreserving {
                            reason: TransferReason::Fairness {
                                constraint: constraint.clone(),
                            },
                        },
                    )),
                )
            }
        }
        (SlotBacking::Environment { member: Some(_) }, SlotFill::Record(_)) => {
            bare(SlotValue::Supplied, None, None)
        }
        _ => bare(SlotValue::Absent, None, None),
    })
}

/// A filled item-backed slot's outcome: its premise result's value,
/// certification and extent. A proved `model-claim` result that names an
/// assumption no filled environment record supplies is `inconclusive`.
pub(super) fn item_outcome(
    index: usize,
    derived: &DerivedSlot,
    result: &PremiseResult,
    supplied: &BTreeSet<&str>,
) -> SlotOutcome {
    let value = result.record.value();
    let mut label = ItemLabel::of(value);
    if derived.role == SlotRole::ModelClaim
        && label == ItemLabel::Proved
        && result
            .model_claim_assumptions
            .iter()
            .any(|name| !supplied.contains(name.as_str()))
    {
        label = ItemLabel::Inconclusive;
    }
    let proved = label == ItemLabel::Proved;
    let cause = match value {
        TerminalValue::Inconclusive(cause) => Some(PlannerCause::Inconclusive(cause.clone())),
        TerminalValue::Unsupported(cause) => Some(PlannerCause::Unsupported {
            cause: cause.clone(),
            diagnostic: result.diagnostic.clone(),
        }),
        TerminalValue::Incomplete(cause) => Some(PlannerCause::Incomplete {
            cause: cause.clone(),
            limit: result.limit_reached.clone(),
        }),
        _ => None,
    };
    SlotOutcome {
        slot: index,
        role: derived.role,
        value: SlotValue::Label(label),
        certification: if proved { result.certification } else { None },
        extent: Some(result.item.key.extent),
        bounds: result.item.bounds.clone(),
        cause,
        source: derived.source.clone(),
    }
}

/// QSpec FR-488 "Fairness correspondence": a scheduling premise over the
/// constraint's operation, whose operation has a frame binding, implies the
/// constraint under QSpec FR-380's kind and granularity rule. Only a filled
/// environment record supplies a premise.
fn fairness_implied(
    declaration: &PlannerDeclaration,
    request: &SettlementRequest,
    derived: &DerivedSlot,
) -> bool {
    let (Some(constraint), Some(operation)) = (&derived.constraint, &derived.operation) else {
        return false;
    };
    let framed = declaration
        .slots
        .iter()
        .any(|slot| slot.role == SlotRole::Frame && slot.operation.as_ref() == Some(operation));
    framed
        && declaration
            .slots
            .iter()
            .zip(&request.slots)
            .filter(|(_, filled)| matches!(filled.fill, SlotFill::Record(_)))
            .filter_map(|(slot, _)| match &slot.backing {
                SlotBacking::Environment { member: Some(name) } => Some(name),
                _ => None,
            })
            .filter_map(|name| {
                declaration
                    .environment
                    .iter()
                    .find(|member| member.name == *name)
            })
            .any(|member| member.supplies.implies(operation, constraint))
}

fn conclusion(
    declaration: &PlannerDeclaration,
    request: &SettlementRequest,
    slots: &[SlotOutcome],
) -> Conclusion {
    let assumptions = request
        .slots
        .iter()
        .filter_map(|slot| match &slot.fill {
            SlotFill::Record(record) => Some(record.clone()),
            _ => None,
        })
        .collect();
    let bounded: Vec<(usize, Vec<Value>)> = slots
        .iter()
        .filter(|slot| slot.extent == Some(ExtentClass::Bounded))
        .map(|slot| (slot.slot, slot.bounds.clone()))
        .collect();
    Conclusion {
        package: declaration.package.clone(),
        declaration: declaration.node.clone(),
        relation: declaration.relation_node.clone(),
        sources: declaration.relation_sources.clone(),
        clause: declaration.clause.clone(),
        slots: slots.to_vec(),
        assumptions,
        extent: if bounded.is_empty() {
            ConclusionExtent::Unbounded
        } else {
            ConclusionExtent::Bounded(bounded)
        },
        certifications: slots
            .iter()
            .filter_map(|slot| slot.certification.map(|cert| (slot.slot, cert)))
            .collect(),
    }
}

// ---- Wire --------------------------------------------------------------

fn node_value(node: &NodeKey) -> Result<Value, KeyWireError> {
    Ok(node_key_to_json(node)?)
}

fn region_value(region: &SourceRegion) -> Value {
    json!({
        "authority": region.source().authority(),
        "identity": region.source().identity(),
        "start": region.start(),
        "end": region.end(),
    })
}

/// The wire form of an FR-331 cause the existing outcome writer spells,
/// `{type, ...}`.
fn outcome_cause(value: TerminalValue) -> Value {
    let record = TerminalRecord::from_value(RequestIndex::new(0), value);
    serde_json::to_value(OutcomeItem::from_terminal(&record))
        .ok()
        .and_then(|mut item| item.get_mut("cause").map(Value::take))
        .unwrap_or(Value::Null)
}

impl PlannerCause {
    /// The cause as FR-331 spells it, or the diagnostic that stands in place
    /// of an `unsupported` cause: `(member name, value)`.
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when a constraint cannot be written.
    pub fn to_wire(&self) -> Result<(&'static str, Value), KeyWireError> {
        Ok(match self {
            Self::Inconclusive(InconclusiveCause::PremiseUnproved { premise, result }) => (
                "cause",
                json!({
                    "type": "premise-unproved",
                    "premise": premise,
                    "result": match result {
                        PremiseOutcome::Label(label) => label_str(*label),
                        PremiseOutcome::Absent => "absent",
                        PremiseOutcome::Assumption => "assumption",
                    },
                }),
            ),
            Self::Inconclusive(InconclusiveCause::TransferNotPreserving { reason }) => {
                let mut cause = Map::new();
                cause.insert("type".to_owned(), json!("transfer-not-preserving"));
                match reason {
                    TransferReason::Fragment { operator } => {
                        cause.insert("reason".to_owned(), json!("fragment"));
                        cause.insert("operator".to_owned(), json!(operator));
                    }
                    TransferReason::Fairness { constraint } => {
                        cause.insert("reason".to_owned(), json!("fairness"));
                        cause.insert("constraint".to_owned(), fairness_value(constraint)?);
                    }
                    TransferReason::StutterClosure => {
                        cause.insert("reason".to_owned(), json!("stutter-closure"));
                    }
                }
                ("cause", Value::Object(cause))
            }
            Self::Inconclusive(other) => (
                "cause",
                outcome_cause(TerminalValue::Inconclusive(other.clone())),
            ),
            Self::Unsupported {
                diagnostic: Some(diagnostic),
                ..
            } => ("diagnostic", diagnostic.clone()),
            Self::Unsupported {
                cause,
                diagnostic: None,
            } => (
                "cause",
                outcome_cause(TerminalValue::Unsupported(cause.clone())),
            ),
            Self::Incomplete {
                limit: Some(limit), ..
            } => (
                "cause",
                json!({
                    "type": "limit-reached",
                    "limit": limit.limit,
                    "value": limit.value,
                    "setting": limit.setting,
                }),
            ),
            Self::Incomplete { cause, limit: None } => (
                "cause",
                outcome_cause(TerminalValue::Incomplete(cause.clone())),
            ),
            Self::WorkLimit { value } => (
                "cause",
                json!({
                    "type": "limit-reached",
                    "limit": "max_work",
                    "value": value,
                    "setting": "planner.max_work",
                }),
            ),
        })
    }
}

pub(super) fn fairness_value(
    constraint: &quire_semantic_value::fairness::FairnessConstraint,
) -> Result<Value, KeyWireError> {
    FairnessKey::of(constraint)?.to_value()
}

impl SlotOutcome {
    /// The slot outcome as it is written in the planner result.
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when a member cannot be written.
    pub fn to_wire(&self) -> Result<Value, KeyWireError> {
        let mut slot = Map::new();
        slot.insert("slot".to_owned(), json!(self.slot));
        slot.insert("value".to_owned(), json!(self.value.as_str()));
        if let Some(certification) = self.certification {
            slot.insert("certification".to_owned(), json!(certification.as_str()));
        }
        if let Some(extent) = self.extent {
            slot.insert("extent".to_owned(), json!(extent.as_str()));
            if extent == ExtentClass::Bounded {
                slot.insert("bounds".to_owned(), Value::Array(self.bounds.clone()));
            }
        }
        if let Some(cause) = &self.cause {
            let (name, value) = cause.to_wire()?;
            slot.insert(name.to_owned(), value);
        }
        if let Some(source) = &self.source {
            slot.insert("source".to_owned(), region_value(source));
        }
        Ok(Value::Object(slot))
    }
}

impl Conclusion {
    /// FR-488's conclusion record.
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when a member cannot be written.
    pub fn to_wire(&self) -> Result<Value, KeyWireError> {
        let slots: Result<Vec<Value>, KeyWireError> =
            self.slots.iter().map(SlotOutcome::to_wire).collect();
        let assumptions: Result<Vec<Value>, KeyWireError> =
            self.assumptions.iter().map(record_value).collect();
        let relation = match &self.relation {
            Some(node) => json!({"package": self.package, "relation": node_value(node)?}),
            None => Value::Null,
        };
        let mut clause = Map::new();
        if let Some(facts) = &self.clause {
            clause.insert("node".to_owned(), node_value(&facts.node)?);
            let fairness: Result<Vec<Value>, KeyWireError> =
                facts.fairness.iter().map(fairness_value).collect();
            clause.insert("fairness".to_owned(), Value::Array(fairness?));
            if let Some(span) = &facts.clause_text_span {
                clause.insert("text_span".to_owned(), region_value(span));
            }
        }
        let extent = match &self.extent {
            ConclusionExtent::Unbounded => json!("unbounded"),
            ConclusionExtent::Bounded(bounds) => json!({
                "bounded": bounds
                    .iter()
                    .map(|(slot, bounds)| json!({"slot": slot, "bounds": bounds}))
                    .collect::<Vec<Value>>(),
            }),
        };
        Ok(json!({
            "declaration": node_value(&self.declaration)?,
            "relation": relation,
            "implementation_sources": self.sources,
            "clause": Value::Object(clause),
            "slots": slots?,
            "assumptions": assumptions?,
            "extent": extent,
            "certifications": self
                .certifications
                .iter()
                .map(|(slot, certification)| json!({"slot": slot, "certification": certification.as_str()}))
                .collect::<Vec<Value>>(),
        }))
    }
}

fn record_value(record: &EnvironmentRecord) -> Result<Value, KeyWireError> {
    let supplies = match &record.supplies {
        RecordSupplies::Fairness {
            kind,
            granularity,
            context,
            operation,
        } => json!({"fairness": {
            "kind": match kind {
                quire_semantic_value::fairness::FairnessKind::Weak => "weak",
                quire_semantic_value::fairness::FairnessKind::Strong => "strong",
            },
            "granularity": match granularity {
                quire_semantic_value::fairness::FairnessGranularity::Whole => "whole",
                quire_semantic_value::fairness::FairnessGranularity::Each => "each",
            },
            "context": node_value(context)?,
            "operation": operation,
        }}),
        RecordSupplies::Assumption(node) => json!({"assumption": node_value(node)?}),
    };
    Ok(json!({"name": record.name, "supplier": record.supplier, "supplies": supplies}))
}

impl PlannerResult {
    /// The `quire.planner-result/v1` document of QSpec FR-492 "Planner
    /// result".
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when a member cannot be written.
    pub fn to_wire(&self) -> Result<Value, KeyWireError> {
        let mut result = Map::new();
        result.insert("format".to_owned(), json!("quire.planner-result/v1"));
        result.insert(
            "item".to_owned(),
            json!({"role": self.item.0.as_str(), "node": node_value(&self.item.1)?}),
        );
        result.insert("value".to_owned(), json!(self.value.as_str()));
        result.insert("basis".to_owned(), json!(self.value.basis().as_str()));
        if let Some(cause) = &self.cause {
            let (name, value) = cause.to_wire()?;
            result.insert(name.to_owned(), value);
        }
        if let Some(certification) = self.certification {
            result.insert("certification".to_owned(), json!(certification.as_str()));
        }
        let slots: Result<Vec<Value>, KeyWireError> =
            self.slots.iter().map(SlotOutcome::to_wire).collect();
        result.insert("slots".to_owned(), Value::Array(slots?));
        if let Some(conclusion) = &self.conclusion {
            result.insert("conclusion".to_owned(), conclusion.to_wire()?);
        }
        if let Some(discharge) = &self.discharge {
            result.insert("discharge".to_owned(), discharge.to_wire()?);
        }
        Ok(Value::Object(result))
    }
}

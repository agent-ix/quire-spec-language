// SPDX-License-Identifier: AGPL-3.0-or-later
//! `planner::settle` for an `assume-guarantee` composition (QSL FR-530,
//! QSpec FR-489 "Implication", "Cycles" and "Settlement", FR-492 "Slot
//! outcome"): discharge each premise slot from the guarantee pool, the
//! supplied scheduling constraints, the `discharges` records and the
//! `fixed-point` slot, and write the premise discharge record.
//!
//! The planner decides exactly one entailment, FR-489 "Implication": a
//! supplied guarantee discharges a `rely` or `liveness-dependency` slot when
//! its request item has the slot's peer subject and peer guarantee clause,
//! its result is `proved` and every constraint of its fairness set is implied
//! by a constraint the composition supplies. It never decides that a stronger
//! or different clause discharges a premise, and a result that is not
//! `proved` (an `inconclusive` one included) discharges nothing. It computes
//! no fixed point: a cycle's premises are discharged only by its
//! `fixed-point` slot's `proved` result.

use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};
use serde_json::{json, Map, Value};

use super::declaration::{
    strength_implies, DerivedSlot, PlannerDeclaration, SlotBacking, SlotRole,
};
use super::key::{ExtentClass, FairnessKey, KeyWireError};
use super::request::{Discharger, PremiseResult, SettlementRequest, SlotFill};
use super::settle::{
    decide, fairness_value, fold_certification, item_outcome, Budget, PlannerCause, PlannerResult,
    PlannerValue, SettleRefusal, SlotOutcome, SlotValue,
};
use crate::outcome::ItemLabel;
use crate::proof_result::Certification;

/// The relation by which a discharger implies a premise (QSpec FR-489
/// "Outputs").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Relation {
    /// The slot's own item, or a supplied guarantee of the same subject and
    /// clause, is the premise.
    Identity,
    /// A supplied scheduling constraint implies the premise under QSpec
    /// FR-380.
    FairnessImplication,
    /// An environment record is the premise, listed as an assumption.
    Assumption,
    /// The cycle's `fixed-point` slot holds a `proved` FR-176 result.
    FixedPoint,
}

impl Relation {
    /// The relation's wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::FairnessImplication => "fairness-implication",
            Self::Assumption => "assumption",
            Self::FixedPoint => "fixed-point",
        }
    }
}

/// What discharged a slot.
#[derive(Clone, Debug)]
pub enum DischargeSource {
    /// The slot's own premise result (`component-guarantee`, `fixed-point`).
    OwnResult,
    /// The supplied guarantee at this index of the request's pool.
    Pool(usize),
    /// A supplied scheduling constraint of the composition.
    Supplied(FairnessConstraint),
    /// The filled environment record of this name.
    Environment(String),
    /// The `fixed-point` slot at this index, whose `proved` result the
    /// record carries.
    FixedPoint(usize),
}

/// One discharged slot of the premise discharge record.
#[derive(Clone, Debug)]
pub struct DischargeEntry {
    /// The slot index.
    pub slot: usize,
    /// The slot's role.
    pub role: SlotRole,
    /// What discharged it.
    pub source: DischargeSource,
    /// The subject the discharger names, in the canonical text of the
    /// request item's `subject` member, when it has one.
    pub subject: Option<String>,
    /// The relation by which the discharger implies the premise.
    pub relation: Relation,
    /// The discharger's certification (QSpec FR-354).
    pub certification: Option<Certification>,
    /// The discharger's extent classification.
    pub extent: Option<ExtentClass>,
    /// A `bounded` extent's bounds, as written.
    pub bounds: Vec<Value>,
}

/// QSpec FR-489's premise discharge record: one entry per discharged slot, in
/// slot order. An undischarged slot has no entry; its outcome is in the
/// planner result's slots.
#[derive(Clone, Debug, Default)]
pub struct DischargeRecord {
    /// The entries.
    pub entries: Vec<DischargeEntry>,
}

/// The planner's work budget ran out.
struct Exhausted;

/// QSpec FR-492 `limits.max_work`: one unit per slot read, per pool entry
/// compared and per fixed-point premise checked.
struct Meter {
    spent: u64,
    max: u64,
}

impl Meter {
    fn charge(&mut self) -> Result<(), Exhausted> {
        if self.spent >= self.max {
            return Err(Exhausted);
        }
        self.spent += 1;
        Ok(())
    }
}

/// The parts of a fairness constraint QSpec FR-380 compares.
struct Parts {
    kind: FairnessKind,
    granularity: FairnessGranularity,
    operation: Value,
}

impl Parts {
    /// The parts of `key`, or `None` for a spelling FR-362 does not give.
    #[ix_trace_rs::string_edge]
    fn of(key: &FairnessKey) -> Option<Self> {
        let value = key.to_value().ok()?;
        let kind = match value.get("kind")?.as_str()? {
            "weak" => FairnessKind::Weak,
            "strong" => FairnessKind::Strong,
            _ => return None,
        };
        let granularity = match value.get("granularity")?.as_str()? {
            "whole" => FairnessGranularity::Whole,
            "each" => FairnessGranularity::Each,
            _ => return None,
        };
        Some(Self {
            kind,
            granularity,
            operation: value.get("operation")?.clone(),
        })
    }

    /// QSpec FR-380 "Fairness implication": the same operation, then the
    /// kind and granularity rule.
    fn implies(&self, needed: &Self) -> bool {
        self.operation == needed.operation
            && strength_implies(
                (self.kind, self.granularity),
                (needed.kind, needed.granularity),
            )
    }
}

/// What one slot settled to.
struct Settled {
    outcome: SlotOutcome,
    entry: Option<DischargeEntry>,
}

struct Composition<'a> {
    declaration: &'a PlannerDeclaration,
    request: &'a SettlementRequest,
    /// The parts of the composition's `supplies`, in declaration order.
    supplies: Vec<Parts>,
}

/// Settle an `assume-guarantee` composition from its admitted request.
pub(super) fn settle_composition(
    declaration: &PlannerDeclaration,
    request: &SettlementRequest,
    budget: Budget,
) -> Result<PlannerResult, SettleRefusal> {
    if declaration.rule.as_deref() != Some("assume-guarantee") {
        return Err(SettleRefusal::UnsupportedRule);
    }
    let supplies = declaration
        .supplies
        .iter()
        .map(|constraint| {
            let key = FairnessKey::of(constraint)?;
            Ok(Parts::of(&key))
        })
        .collect::<Result<Vec<_>, KeyWireError>>()?
        .into_iter()
        .flatten()
        .collect();
    let composition = Composition {
        declaration,
        request,
        supplies,
    };
    let max_work = request.max_work.unwrap_or(budget.max_work);
    let mut meter = Meter {
        spent: 0,
        max: max_work,
    };
    let mut slots = Vec::with_capacity(declaration.slots.len());
    let mut entries = Vec::new();
    let mut exhausted = false;
    for (index, derived) in declaration.slots.iter().enumerate() {
        match composition.settle_slot(index, derived, &mut meter) {
            Ok(Settled { outcome, entry }) => {
                slots.push(outcome);
                entries.extend(entry);
            }
            Err(Exhausted) => {
                exhausted = true;
                break;
            }
        }
    }
    let item = (declaration.role, declaration.node.clone());
    let discharge = Some(DischargeRecord { entries });
    if exhausted {
        return Ok(PlannerResult {
            item,
            value: PlannerValue::Incomplete,
            cause: Some(PlannerCause::WorkLimit { value: max_work }),
            certification: None,
            slots,
            conclusion: None,
            discharge,
        });
    }
    let (value, cause) = decide(&slots, |outcome| composition.is_cycle_member(outcome.slot));
    let certification = (value == PlannerValue::Proved).then(|| fold_certification(&slots));
    Ok(PlannerResult {
        item,
        value,
        cause,
        certification,
        slots,
        conclusion: None,
        discharge,
    })
}

impl Composition<'_> {
    /// A `rely` or `liveness-dependency` slot on an accepted cycle: it waits
    /// on its cycle's `fixed-point` slot, which decides in its place.
    fn is_cycle_member(&self, slot: usize) -> bool {
        self.declaration.slots.get(slot).is_some_and(|derived| {
            derived.cycle.is_some()
                && matches!(derived.role, SlotRole::Rely | SlotRole::LivenessDependency)
        })
    }

    fn settle_slot(
        &self,
        index: usize,
        derived: &DerivedSlot,
        meter: &mut Meter,
    ) -> Result<Settled, Exhausted> {
        // One work unit per slot read.
        meter.charge()?;
        let fill = &self.request.slots[index].fill;
        let bare = |value, cause| SlotOutcome {
            slot: index,
            role: derived.role,
            value,
            certification: None,
            extent: None,
            bounds: Vec::new(),
            cause,
            source: derived.source.clone(),
        };
        Ok(match (derived.role, &derived.backing, fill) {
            (SlotRole::ComponentGuarantee, SlotBacking::Item, SlotFill::Result(result)) => {
                self.own_result(index, derived, result)
            }
            (SlotRole::FixedPoint, SlotBacking::Item, SlotFill::Result(result)) => {
                // The planner checks no iteration itself: it reads the one
                // FR-176 result, once per cycle guarantee it names.
                for _ in 0..result.item.key.premises.len().max(1) {
                    meter.charge()?;
                }
                self.own_result(index, derived, result)
            }
            (SlotRole::Rely | SlotRole::LivenessDependency, _, _) => {
                if let Some(cycle) = derived.cycle {
                    self.cycle_member(index, derived, cycle)
                } else {
                    self.discharge_from_pool(index, derived, meter)?
                }
            }
            (SlotRole::Fairness, _, _) => self.discharge_from_supplies(index, derived),
            (
                SlotRole::Environment,
                SlotBacking::Environment { member: Some(_) },
                SlotFill::Record(record),
            ) => Settled {
                entry: Some(DischargeEntry {
                    slot: index,
                    role: derived.role,
                    source: DischargeSource::Environment(record.name.clone()),
                    subject: None,
                    relation: Relation::Assumption,
                    certification: None,
                    extent: None,
                    bounds: Vec::new(),
                }),
                outcome: bare(SlotValue::Supplied, None),
            },
            _ => Settled {
                outcome: bare(SlotValue::Absent, None),
                entry: None,
            },
        })
    }

    /// An item-backed slot settles as its own premise result does.
    fn own_result(&self, index: usize, derived: &DerivedSlot, result: &PremiseResult) -> Settled {
        let outcome = item_outcome(index, derived, result, &std::collections::BTreeSet::new());
        let entry = (outcome.value == SlotValue::Label(ItemLabel::Proved)).then(|| {
            let fixed_point = derived.role == SlotRole::FixedPoint;
            DischargeEntry {
                slot: index,
                role: derived.role,
                source: DischargeSource::OwnResult,
                subject: subject_of(result),
                relation: if fixed_point {
                    Relation::FixedPoint
                } else {
                    Relation::Identity
                },
                certification: outcome.certification,
                extent: outcome.extent,
                bounds: outcome.bounds.clone(),
            }
        });
        Settled { outcome, entry }
    }

    /// QSpec FR-489 "Cycles": a cycle member is discharged only by its
    /// cycle's `fixed-point` slot holding a `proved` result; until then it
    /// waits on that slot.
    fn cycle_member(&self, index: usize, derived: &DerivedSlot, cycle: usize) -> Settled {
        let fixed_point = self
            .declaration
            .slots
            .iter()
            .position(|slot| slot.role == SlotRole::FixedPoint && slot.cycle == Some(cycle));
        let proved = fixed_point.and_then(|at| match &self.request.slots[at].fill {
            SlotFill::Result(result)
                if ItemLabel::of(result.record.value()) == ItemLabel::Proved =>
            {
                Some((at, result))
            }
            _ => None,
        });
        match proved {
            Some((at, result)) => {
                let certification =
                    Some(result.certification.unwrap_or(Certification::Uncertified));
                let extent = Some(result.item.key.extent);
                Settled {
                    outcome: SlotOutcome {
                        slot: index,
                        role: derived.role,
                        value: SlotValue::Label(ItemLabel::Proved),
                        certification,
                        extent,
                        bounds: result.item.bounds.clone(),
                        cause: None,
                        source: derived.source.clone(),
                    },
                    entry: Some(DischargeEntry {
                        slot: index,
                        role: derived.role,
                        source: DischargeSource::FixedPoint(at),
                        subject: derived.premise_subject.clone(),
                        relation: Relation::FixedPoint,
                        certification,
                        extent,
                        bounds: result.item.bounds.clone(),
                    }),
                }
            }
            None => Settled {
                outcome: undischarged(index, derived, SlotValue::Absent),
                entry: None,
            },
        }
    }

    /// QSpec FR-489 "Implication" over the pool, the explicitly named
    /// entries first.
    fn discharge_from_pool(
        &self,
        index: usize,
        derived: &DerivedSlot,
        meter: &mut Meter,
    ) -> Result<Settled, Exhausted> {
        let pool = &self.request.guarantees;
        let named: Vec<usize> = self
            .request
            .discharges
            .iter()
            .filter(|discharge| discharge.slot == index)
            .filter_map(|discharge| match discharge.by {
                Discharger::Guarantee(at) => Some(at),
                Discharger::Environment(_) => None,
            })
            .collect();
        let order = named
            .iter()
            .copied()
            .chain((0..pool.len()).filter(|at| !named.contains(at)));
        for at in order {
            meter.charge()?;
            let entry = &pool[at];
            if !self.discharges(derived, entry) {
                continue;
            }
            let certification = Some(entry.certification.unwrap_or(Certification::Uncertified));
            let extent = Some(entry.item.key.extent);
            return Ok(Settled {
                outcome: SlotOutcome {
                    slot: index,
                    role: derived.role,
                    value: SlotValue::Label(ItemLabel::Proved),
                    certification,
                    extent,
                    bounds: entry.item.bounds.clone(),
                    cause: None,
                    source: derived.source.clone(),
                },
                entry: Some(DischargeEntry {
                    slot: index,
                    role: derived.role,
                    source: DischargeSource::Pool(at),
                    subject: subject_of(entry),
                    relation: Relation::Identity,
                    certification,
                    extent,
                    bounds: entry.item.bounds.clone(),
                }),
            });
        }
        // No pool entry discharges it: a filled environment record named as
        // its only discharger is an assumption, not a discharge.
        let assumed = self.request.discharges.iter().any(|discharge| {
            discharge.slot == index && matches!(discharge.by, Discharger::Environment(_))
        });
        let value = if assumed {
            SlotValue::Assumption
        } else {
            SlotValue::Absent
        };
        Ok(Settled {
            outcome: undischarged(index, derived, value),
            entry: None,
        })
    }

    /// Whether `entry` discharges the `rely` or `liveness-dependency` slot
    /// `derived` (QSpec FR-489 "Implication"). A result that is not `proved`
    /// discharges nothing.
    fn discharges(&self, derived: &DerivedSlot, entry: &PremiseResult) -> bool {
        let (Some(subject), Some(clause)) = (&derived.premise_subject, &derived.peer_clause) else {
            return false;
        };
        let Some(temporal) = &entry.item.key.temporal else {
            return false;
        };
        ItemLabel::of(entry.record.value()) == ItemLabel::Proved
            && temporal.subject == *subject
            && entry.item.key.claim.node() == clause
            && temporal.fairness.iter().all(|key| {
                Parts::of(key).is_some_and(|needed| {
                    self.supplies
                        .iter()
                        .any(|supplied| supplied.implies(&needed))
                })
            })
    }

    /// A `fairness` slot is discharged by a supplied constraint that implies
    /// it under QSpec FR-380.
    fn discharge_from_supplies(&self, index: usize, derived: &DerivedSlot) -> Settled {
        let needed = derived
            .constraint
            .as_ref()
            .and_then(|constraint| FairnessKey::of(constraint).ok())
            .and_then(|key| Parts::of(&key));
        let supplier = needed.and_then(|needed| {
            self.declaration
                .supplies
                .iter()
                .zip(&self.supplies)
                .find(|(_, parts)| parts.implies(&needed))
                .map(|(constraint, _)| constraint.clone())
        });
        match supplier {
            Some(constraint) => Settled {
                outcome: SlotOutcome {
                    slot: index,
                    role: derived.role,
                    value: SlotValue::Label(ItemLabel::Proved),
                    certification: Some(Certification::Certified),
                    extent: Some(ExtentClass::Unbounded),
                    bounds: Vec::new(),
                    cause: None,
                    source: derived.source.clone(),
                },
                entry: Some(DischargeEntry {
                    slot: index,
                    role: derived.role,
                    source: DischargeSource::Supplied(constraint),
                    subject: None,
                    relation: Relation::FairnessImplication,
                    certification: Some(Certification::Certified),
                    extent: Some(ExtentClass::Unbounded),
                    bounds: Vec::new(),
                }),
            },
            None => Settled {
                outcome: undischarged(index, derived, SlotValue::Absent),
                entry: None,
            },
        }
    }
}

fn undischarged(index: usize, derived: &DerivedSlot, value: SlotValue) -> SlotOutcome {
    SlotOutcome {
        slot: index,
        role: derived.role,
        value,
        certification: None,
        extent: None,
        bounds: Vec::new(),
        cause: None,
        source: derived.source.clone(),
    }
}

fn subject_of(result: &PremiseResult) -> Option<String> {
    result
        .item
        .key
        .temporal
        .as_ref()
        .map(|temporal| temporal.subject.clone())
}

// ---- Wire --------------------------------------------------------------

impl DischargeEntry {
    /// The entry as it is written in the premise discharge record.
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when a member cannot be written.
    pub fn to_wire(&self) -> Result<Value, KeyWireError> {
        let by = match &self.source {
            DischargeSource::OwnResult => json!({"slot": self.slot}),
            DischargeSource::Pool(at) => json!({"guarantee": at}),
            DischargeSource::Supplied(constraint) => {
                json!({"supplies": fairness_value(constraint)?})
            }
            DischargeSource::Environment(name) => json!({"environment": name}),
            DischargeSource::FixedPoint(at) => json!({"fixed_point": at}),
        };
        let mut entry = Map::new();
        entry.insert("slot".to_owned(), json!(self.slot));
        entry.insert("role".to_owned(), json!(self.role.as_str()));
        entry.insert("by".to_owned(), by);
        if let Some(subject) = &self.subject {
            entry.insert(
                "subject".to_owned(),
                serde_json::from_str(subject).map_err(|_| KeyWireError)?,
            );
        }
        entry.insert("relation".to_owned(), json!(self.relation.as_str()));
        if let Some(certification) = self.certification {
            entry.insert("certification".to_owned(), json!(certification.as_str()));
        }
        if let Some(extent) = self.extent {
            entry.insert("extent".to_owned(), json!(extent.as_str()));
            if extent == ExtentClass::Bounded {
                entry.insert("bounds".to_owned(), Value::Array(self.bounds.clone()));
            }
        }
        Ok(Value::Object(entry))
    }
}

impl DischargeRecord {
    /// QSpec FR-489's premise discharge record.
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when a member cannot be written.
    pub fn to_wire(&self) -> Result<Value, KeyWireError> {
        let entries: Result<Vec<Value>, KeyWireError> =
            self.entries.iter().map(DischargeEntry::to_wire).collect();
        Ok(json!({"discharges": entries?}))
    }
}

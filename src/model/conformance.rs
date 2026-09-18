// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-151 conformance/redefinition/subsetting axis checking
//! (`quire.model.conformance.variance/v1`,
//! `quire.model.conformance.multiplicity/v1`,
//! `quire.model.conformance.effect/v1` and
//! `quire.model.conformance.refinement/v1`).
//!
//! This module works directly against a caller-constructed
//! [`crate::model::bundle::Bundle`], not [`crate::model::normalize`]'s
//! [`crate::model::normalize::EffectiveView`]: it builds its own
//! [`crate::model::key::EffectiveDeclarationPreimage`]-shaped queries over
//! the bundle's [`RedefinitionRecord`]/[`SubsettingRecord`]s, so field
//! redefinition (already exposed by `normalize`'s phase 4) and operation
//! redefinition (out of scope there, see its module docs) are checked
//! uniformly here.
//!
//! Scope decisions, recorded rather than left implicit:
//!
//! - The `precondition`/`postcondition` axes never refuse (FR-151: contracts
//!   are Liskov-correct "by construction," combined as a disjunction/
//!   conjunction, never checked for an implication) — this module charges
//!   `conformance.axis` for both and never inspects their content.
//! - The refinement obligation (`quire.model.conformance.refinement/v1`)
//!   needs the writing operation's *established* postcondition facts.
//!   FR-146's expression evaluator is out of scope for this rung, so
//!   [`crate::model::bundle::EstablishedFact`] is the caller-stated
//!   simplification `crate::model::bundle` already documents; this module
//!   decides only whether a stated fact discharges a given narrowing.
//! - Exact per-axis work-unit costs (FR-151's `f(T)` formula, the length of
//!   a type's own derivation array) are not reproduced: computing `f(T)`
//!   here would require this module to re-walk `normalize`'s ancestor
//!   graph per axis, and FR-151's own worked examples price authored
//!   operation bodies this rung does not model. Every `conformance.axis`
//!   charge costs a flat one work unit; this is a recorded scope choice,
//!   not silent drift from the spec's numbers.
//! - `resolve_redefinition_target`'s ambiguity ruling (two valid, distinct
//!   inherited targets for the same redefining member) is an interpretation
//!   call: FR-151's own prose motivates it only informally. It is recorded
//!   here, not asserted as unambiguous spec fidelity.

use std::collections::{BTreeMap, HashMap};

use crate::diagnostic::Code;
use crate::model::accounting::{Charge, ChargePoint, Incomplete, Meter};
use crate::model::bundle::{
    Bundle, BundleRecord, EstablishedFact, FieldMemberRecord, GeneralizationRecord, Multiplicity,
    OperationMemberRecord, RedefinitionRecord, SubsettingRecord,
};
use crate::model::key::ProducerKey;
use crate::model::normalize::ModelRefusal;

/// Bounds the proper-descendant walk `type_conforms` performs: an explicit
/// task stack over caller-supplied generalization records, never native
/// recursion, with a visited set for cycle safety and this depth ceiling as
/// a typed `resource_exhausted` refusal rather than an unbounded walk.
const MAX_CONFORMANCE_DEPTH: usize = 128;

/// One failing conformance axis: `{axis, code, cause, detail}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AxisFailure {
    /// The axis name, e.g. `"value-type"`, `"parameter-multiplicity"`.
    pub axis: &'static str,
    /// The stable top-level code.
    pub code: Code,
    /// The FR-151-specific cause tag.
    pub cause: &'static str,
    /// A human-readable detail naming the offending declarations.
    pub detail: String,
}

/// The substantive result of one conformance check, once every axis charge
/// admitted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConformanceOutcome {
    /// Every axis passed.
    Compatible,
    /// At least one axis failed; every failure, in axis order (checking is
    /// exhaustive, never stop-at-first, under the limits).
    Refused(Vec<AxisFailure>),
}

/// The outcome of one conformance check attempt, mirroring
/// [`crate::model::normalize::NormalizeOutcome`]'s three-way shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConformanceCheckOutcome {
    /// Every axis charge was admitted; see the substantive [`ConformanceOutcome`].
    Completed(ConformanceOutcome),
    /// A real defect (a dangling member reference, or a conformance walk
    /// deeper than [`MAX_CONFORMANCE_DEPTH`]) refused the check outright.
    Refused(ModelRefusal),
    /// A `ModelNormalizationLimitsV1` counter was exhausted mid-check.
    Incomplete(Incomplete),
}

/// The outcome of resolving which inherited member a redefining member's
/// redefinition record(s) actually target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RedefinitionTargetOutcome {
    /// Exactly one valid inherited target.
    Resolved(ProducerKey),
    /// Zero or multiple valid inherited targets; every checked record and
    /// its named target, in bundle order.
    Refused {
        /// FR-151's cause tag: always `"redefinition-target"`.
        cause: &'static str,
        /// The record/target pairs considered.
        candidates: Vec<(ProducerKey, ProducerKey)>,
    },
}

struct ConformanceIndex {
    generals_by_specific: HashMap<ProducerKey, Vec<GeneralizationRecord>>,
    fields: HashMap<ProducerKey, FieldMemberRecord>,
    /// A `BTreeMap`, not a `HashMap`: [`check_field_refinement_obligation`]
    /// scans `.values()` for the (assumed unique) writer of a field, and a
    /// `HashMap`'s `RandomState` iteration order made that scan
    /// nondeterministic across runs of the same bundle (finding #3).
    operations: BTreeMap<ProducerKey, OperationMemberRecord>,
    scalars: HashMap<ProducerKey, (i64, i64)>,
    /// Original member key -> its declaring type's key, for fields and
    /// operations alike.
    member_owner: HashMap<ProducerKey, ProducerKey>,
    redefinitions: Vec<RedefinitionRecord>,
}

/// Builds the `specific key -> its generalization records` map every
/// bounded conformance walk in `crate::model` (this module and
/// [`crate::model::dispatch`]) needs. One builder, so the map's shape is a
/// single fact rather than a duplicated field-by-field copy. Keyed on the
/// full [`ProducerKey`], not the display identity alone (PR #140 F2): two
/// generalization records whose `specific` shares a display identity but
/// differs in revision must both index their own distinct ancestor set,
/// never silently overwrite one another.
pub(super) fn generals_by_specific(
    bundle: &Bundle,
) -> HashMap<ProducerKey, Vec<GeneralizationRecord>> {
    let mut generals_by_specific: HashMap<ProducerKey, Vec<GeneralizationRecord>> = HashMap::new();
    for record in &bundle.records {
        if let BundleRecord::Generalization(general) = record {
            generals_by_specific
                .entry(general.specific.clone())
                .or_default()
                .push(general.clone());
        }
    }
    generals_by_specific
}

impl ConformanceIndex {
    fn build(bundle: &Bundle) -> Self {
        let generals_by_specific = generals_by_specific(bundle);
        let mut fields = HashMap::new();
        let mut operations = BTreeMap::new();
        let mut scalars = HashMap::new();
        let mut member_owner = HashMap::new();
        let mut redefinitions = Vec::new();
        for record in &bundle.records {
            match record {
                BundleRecord::ObjectType(_) | BundleRecord::Generalization(_) => {}
                BundleRecord::FieldMember(field) => {
                    member_owner.insert(field.key.clone(), field.owner.clone());
                    fields.insert(field.key.clone(), field.clone());
                }
                BundleRecord::ScalarType(scalar) => {
                    scalars.insert(scalar.key.clone(), (scalar.lower, scalar.upper));
                }
                BundleRecord::OperationMember(operation) => {
                    member_owner.insert(operation.key.clone(), operation.owner.clone());
                    operations.insert(operation.key.clone(), operation.clone());
                }
                BundleRecord::Redefinition(redefinition) => {
                    redefinitions.push(redefinition.clone());
                }
                BundleRecord::Subsetting(_) => {}
                // FR-152 systems-model records are not conformance-checked
                // by this module (crate::model::systems owns them).
                BundleRecord::Component(_)
                | BundleRecord::Endpoint(_)
                | BundleRecord::Relationship(_) => {}
            }
        }
        Self {
            generals_by_specific,
            fields,
            operations,
            scalars,
            member_owner,
            redefinitions,
        }
    }
}

/// Whether `s` conforms to `t`: the same identity, or a chain of supplied
/// generalization records from `s` to `t`. Explicit task stack over
/// caller-supplied records, bounded by [`MAX_CONFORMANCE_DEPTH`] and a
/// visited set, so a cycle or an adversarial chain refuses instead of
/// looping or overflowing a native call stack.
///
/// `pub(super)` so [`crate::model::dispatch`]'s own bounded walks (subtype
/// applicability and dominance) reuse this one implementation rather than a
/// second copy.
pub(super) fn type_conforms(
    generals_by_specific: &HashMap<ProducerKey, Vec<GeneralizationRecord>>,
    s: &ProducerKey,
    t: &ProducerKey,
) -> Result<bool, ModelRefusal> {
    if s == t {
        return Ok(true);
    }
    let mut stack: Vec<ProducerKey> = vec![s.clone()];
    let mut visited: std::collections::HashSet<ProducerKey> = std::collections::HashSet::new();
    let mut steps: usize = 0;
    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        steps += 1;
        if steps > MAX_CONFORMANCE_DEPTH {
            return Err(ModelRefusal {
                code: Code::ResourceExhausted,
                cause: "conformance-depth",
                detail: format!(
                    "conformance check from {} exceeded {MAX_CONFORMANCE_DEPTH} generalization steps",
                    s.identity
                ),
            });
        }
        for general in generals_by_specific.get(&current).into_iter().flatten() {
            if &general.general == t {
                return Ok(true);
            }
            stack.push(general.general.clone());
        }
    }
    Ok(false)
}

/// `quire.model.conformance.multiplicity/v1`: does `from` conform to `to`?
/// `[l1,u1] = to`, `[l2,u2] = from`: conforms exactly when `l1 <= l2` and
/// `u2 <= u1` (`None` is unbounded, greater than every finite value), and
/// `ordered`/`unique` are equal.
pub(super) fn multiplicity_conforms(from: &Multiplicity, to: &Multiplicity) -> bool {
    if from.ordered != to.ordered || from.unique != to.unique {
        return false;
    }
    if from.lower < to.lower {
        return false;
    }
    match (from.upper, to.upper) {
        (Some(u2), Some(u1)) => u2 <= u1,
        (None, Some(_)) => false,
        (Some(_), None) => true,
        (None, None) => true,
    }
}

fn charge_axis(meter: &mut Meter) -> Result<(), Incomplete> {
    meter.charge(Charge::new(ChargePoint::ConformanceAxis))
}

fn missing_member(cause: &'static str, identity: &str, role: &str) -> ModelRefusal {
    ModelRefusal {
        code: Code::DanglingReference,
        cause,
        detail: format!("{role} {identity} is not a declared member of the bundle"),
    }
}

/// Checks a field redefinition's `value-type` and `multiplicity` axes
/// (`quire.model.conformance.variance/v1`, `.../multiplicity/v1`).
pub fn check_field_redefinition(
    bundle: &Bundle,
    record: &RedefinitionRecord,
    meter: &mut Meter,
) -> ConformanceCheckOutcome {
    let index = ConformanceIndex::build(bundle);
    let Some(redefining) = index.fields.get(&record.redefining) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            "unknown-redefining",
            &record.redefining.identity,
            "redefining field",
        ));
    };
    let Some(redefined) = index.fields.get(&record.redefined) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            "unknown-redefined",
            &record.redefined.identity,
            "redefined field",
        ));
    };

    let mut failures = Vec::new();

    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    match type_conforms(
        &index.generals_by_specific,
        &redefining.value_type,
        &redefined.value_type,
    ) {
        Ok(true) => {}
        Ok(false) => failures.push(AxisFailure {
            axis: "value-type",
            code: Code::IllTyped,
            cause: "variance-result",
            detail: format!(
                "{} does not conform to {}",
                redefining.value_type.identity, redefined.value_type.identity
            ),
        }),
        Err(refusal) => return ConformanceCheckOutcome::Refused(refusal),
    }

    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    if !multiplicity_conforms(&redefining.multiplicity, &redefined.multiplicity) {
        failures.push(AxisFailure {
            axis: "multiplicity",
            code: Code::IllTyped,
            cause: "multiplicity-narrowing",
            detail: format!(
                "{:?} does not conform to {:?}",
                redefining.multiplicity, redefined.multiplicity
            ),
        });
    }

    if failures.is_empty() {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible)
    } else {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures))
    }
}

/// Checks a subsetting record's `subsetting-type` and `multiplicity` axes
/// (same rule as field redefinition, different cause on the type axis).
/// Runtime population-membership checking
/// (`binding.subset-value`/`subsetting-violation`) is FR-153 territory, not
/// this static check.
pub fn check_subsetting(
    bundle: &Bundle,
    record: &SubsettingRecord,
    meter: &mut Meter,
) -> ConformanceCheckOutcome {
    let index = ConformanceIndex::build(bundle);
    let Some(subsetting) = index.fields.get(&record.subsetting) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            "unknown-subsetting",
            &record.subsetting.identity,
            "subsetting field",
        ));
    };
    let Some(subsetted) = index.fields.get(&record.subsetted) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            "unknown-subsetted",
            &record.subsetted.identity,
            "subsetted field",
        ));
    };

    let mut failures = Vec::new();

    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    match type_conforms(
        &index.generals_by_specific,
        &subsetting.value_type,
        &subsetted.value_type,
    ) {
        Ok(true) => {}
        Ok(false) => failures.push(AxisFailure {
            axis: "subsetting-type",
            code: Code::IllTyped,
            cause: "subsetting-type",
            detail: format!(
                "{} does not conform to {}",
                subsetting.value_type.identity, subsetted.value_type.identity
            ),
        }),
        Err(refusal) => return ConformanceCheckOutcome::Refused(refusal),
    }

    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    if !multiplicity_conforms(&subsetting.multiplicity, &subsetted.multiplicity) {
        failures.push(AxisFailure {
            axis: "multiplicity",
            code: Code::IllTyped,
            cause: "multiplicity-narrowing",
            detail: format!(
                "{:?} does not conform to {:?}",
                subsetting.multiplicity, subsetted.multiplicity
            ),
        });
    }

    if failures.is_empty() {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible)
    } else {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures))
    }
}

/// Checks an operation redefinition's arity, parameter, result and effect
/// axes. The precondition/postcondition axes never refuse (checked by
/// construction, per the module docs) but are still charged.
pub fn check_operation_redefinition(
    bundle: &Bundle,
    record: &RedefinitionRecord,
    meter: &mut Meter,
) -> ConformanceCheckOutcome {
    let index = ConformanceIndex::build(bundle);
    let Some(redefining) = index.operations.get(&record.redefining) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            "unknown-redefining",
            &record.redefining.identity,
            "redefining operation",
        ));
    };
    let Some(redefined) = index.operations.get(&record.redefined) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            "unknown-redefined",
            &record.redefined.identity,
            "redefined operation",
        ));
    };

    let mut failures = Vec::new();

    // Arity.
    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    let arity_matches = redefining.parameters.len() == redefined.parameters.len();
    if !arity_matches {
        failures.push(AxisFailure {
            axis: "arity",
            code: Code::IllTyped,
            cause: "type-mismatch",
            detail: format!(
                "expected {} parameters, found {}",
                redefined.parameters.len(),
                redefining.parameters.len()
            ),
        });
    } else {
        for (i, (rp, dp)) in redefining
            .parameters
            .iter()
            .zip(redefined.parameters.iter())
            .enumerate()
        {
            let display_index = i + 1; // parameter 0 is the receiver, never checked here.

            if let Err(incomplete) = charge_axis(meter) {
                return ConformanceCheckOutcome::Incomplete(incomplete);
            }
            match type_conforms(&index.generals_by_specific, &dp.value_type, &rp.value_type) {
                Ok(true) => {}
                Ok(false) => failures.push(AxisFailure {
                    axis: "parameter-type",
                    code: Code::IllTyped,
                    cause: "variance-parameter",
                    detail: format!(
                        "parameter {display_index}: expected {} to conform to {}",
                        dp.value_type.identity, rp.value_type.identity
                    ),
                }),
                Err(refusal) => return ConformanceCheckOutcome::Refused(refusal),
            }

            if let Err(incomplete) = charge_axis(meter) {
                return ConformanceCheckOutcome::Incomplete(incomplete);
            }
            if !multiplicity_conforms(&dp.multiplicity, &rp.multiplicity) {
                failures.push(AxisFailure {
                    axis: "parameter-multiplicity",
                    code: Code::IllTyped,
                    cause: "multiplicity-narrowing",
                    detail: format!(
                        "parameter {display_index}: {:?} does not conform to {:?}",
                        dp.multiplicity, rp.multiplicity
                    ),
                });
            }
        }
    }

    // Result type.
    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    match (&redefining.result, &redefined.result) {
        (Some(rr), Some(dr)) => {
            match type_conforms(&index.generals_by_specific, &rr.value_type, &dr.value_type) {
                Ok(true) => {}
                Ok(false) => failures.push(AxisFailure {
                    axis: "result-type",
                    code: Code::IllTyped,
                    cause: "variance-result",
                    detail: format!(
                        "{} does not conform to {}",
                        rr.value_type.identity, dr.value_type.identity
                    ),
                }),
                Err(refusal) => return ConformanceCheckOutcome::Refused(refusal),
            }
        }
        (None, None) => {}
        _ => failures.push(AxisFailure {
            axis: "result-type",
            code: Code::IllTyped,
            cause: "variance-result",
            detail: "one of the redefining/redefined operations has no result".to_owned(),
        }),
    }

    // Result multiplicity.
    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    if let (Some(rr), Some(dr)) = (&redefining.result, &redefined.result) {
        if !multiplicity_conforms(&rr.multiplicity, &dr.multiplicity) {
            failures.push(AxisFailure {
                axis: "result-multiplicity",
                code: Code::IllTyped,
                cause: "multiplicity-narrowing",
                detail: format!(
                    "{:?} does not conform to {:?}",
                    rr.multiplicity, dr.multiplicity
                ),
            });
        }
    }

    // Effect.
    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    for write in &redefining.effect.field_writes {
        let direct = redefined
            .effect
            .field_writes
            .iter()
            .any(|w| w.identity == write.identity);
        let via_redefinition = index.redefinitions.iter().any(|r| {
            r.redefining.identity == write.identity
                && redefined
                    .effect
                    .field_writes
                    .iter()
                    .any(|w| w.identity == r.redefined.identity)
        });
        if !direct && !via_redefinition {
            failures.push(AxisFailure {
                axis: "effect",
                code: Code::IllTyped,
                cause: "effect-escape",
                detail: format!(
                    "write {} is not covered by the redefined effect",
                    write.identity
                ),
            });
        }
    }
    for (create, grants) in [
        (&redefining.effect.creates, &redefined.effect.creates),
        (&redefining.effect.deletes, &redefined.effect.deletes),
    ] {
        for entry in create {
            let mut covered = false;
            for grant in grants {
                match type_conforms(&index.generals_by_specific, entry, grant) {
                    Ok(true) => {
                        covered = true;
                        break;
                    }
                    Ok(false) => {}
                    Err(refusal) => return ConformanceCheckOutcome::Refused(refusal),
                }
            }
            if !covered {
                failures.push(AxisFailure {
                    axis: "effect",
                    code: Code::IllTyped,
                    cause: "effect-escape",
                    detail: format!("{} is not covered by the redefined effect", entry.identity),
                });
            }
        }
    }

    // Precondition/postcondition: checked by construction, never refuses.
    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }

    if failures.is_empty() {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible)
    } else {
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures))
    }
}

/// The FR-151 refinement obligation: a narrowing field redefinition must be
/// established by a fact in the effective postcondition of the exposed
/// operation (own or inherited) that writes the redefined field. See the
/// module docs for the [`EstablishedFact`] scope decision this rests on.
pub fn check_field_refinement_obligation(
    bundle: &Bundle,
    record: &RedefinitionRecord,
) -> Result<ConformanceOutcome, ModelRefusal> {
    let index = ConformanceIndex::build(bundle);
    let Some(redefining) = index.fields.get(&record.redefining) else {
        return Err(missing_member(
            "unknown-redefining",
            &record.redefining.identity,
            "redefining field",
        ));
    };
    let Some(redefined) = index.fields.get(&record.redefined) else {
        return Err(missing_member(
            "unknown-redefined",
            &record.redefined.identity,
            "redefined field",
        ));
    };

    let same_type = redefining.value_type.identity == redefined.value_type.identity;
    let raises_lower = redefining.multiplicity.lower > redefined.multiplicity.lower;
    let single_valued = redefining.multiplicity.upper.is_some_and(|u| u <= 1);

    if same_type && !raises_lower {
        // No narrowing at all (or a narrower upper bound only, which this
        // rung treats under `no-proof-form` below, matching FR-151's
        // "an upper bound on a collection" example).
        if redefining
            .multiplicity
            .upper
            .is_none_or(|redefining_upper| {
                redefined
                    .multiplicity
                    .upper
                    .is_none_or(|redefined_upper| redefining_upper <= redefined_upper)
            })
        {
            return Ok(ConformanceOutcome::Compatible);
        }
    }

    let writer = index.operations.values().find(|operation| {
        operation.effect.field_writes.iter().any(|field| {
            field.identity == record.redefined.identity
                || field.identity == record.redefining.identity
        })
    });
    let Some(writer) = writer else {
        // No exposed operation writes this field: nothing to discharge.
        return Ok(ConformanceOutcome::Compatible);
    };

    let mut facts: Vec<&EstablishedFact> = writer.own_postcondition_facts.iter().collect();
    for redefinition in &index.redefinitions {
        if redefinition.redefined.identity == writer.key.identity
            && redefinition.owner.identity == record.owner.identity
        {
            if let Some(overriding) = index.operations.get(&redefinition.redefining) {
                facts.extend(overriding.own_postcondition_facts.iter());
            }
        }
    }
    let names_field = |key: &ProducerKey| -> bool {
        key.identity == record.redefined.identity || key.identity == record.redefining.identity
    };

    if raises_lower && single_valued {
        let has_presence = facts
            .iter()
            .any(|fact| matches!(fact, EstablishedFact::Presence { field } if names_field(field)));
        return if has_presence {
            Ok(ConformanceOutcome::Compatible)
        } else {
            Ok(ConformanceOutcome::Refused(vec![AxisFailure {
                axis: "refinement",
                code: Code::UndefinedExpression,
                cause: "unproved-refinement",
                detail: format!(
                    "{} narrows the multiplicity of {} with no establishing presence fact (obligation field-presence)",
                    record.redefining.identity, record.redefined.identity
                ),
            }]))
        };
    }

    if raises_lower && !single_valued {
        return Ok(ConformanceOutcome::Refused(vec![AxisFailure {
            axis: "refinement",
            code: Code::UndefinedExpression,
            cause: "unproved-refinement",
            detail: format!(
                "{} narrows a collection upper bound, which no FR-146 fact form expresses (obligation no-proof-form)",
                record.redefining.identity
            ),
        }]));
    }

    // Value type changed: field-domain via scalar interval containment, or
    // no-proof-form when either type is not a known scalar (an object type
    // narrowing, per FR-151's own example).
    match (
        index.scalars.get(&redefining.value_type),
        index.scalars.get(&redefined.value_type),
    ) {
        (Some(&(narrow_lower, narrow_upper)), Some(_)) => {
            let interval = facts.iter().find_map(|fact| match fact {
                EstablishedFact::Interval { field, lower, upper } if names_field(field) => {
                    Some((*lower, *upper))
                }
                _ => None,
            });
            match interval {
                Some((lower, upper)) if lower >= narrow_lower && upper <= narrow_upper => {
                    Ok(ConformanceOutcome::Compatible)
                }
                Some((lower, upper)) => Ok(ConformanceOutcome::Refused(vec![AxisFailure {
                    axis: "refinement",
                    code: Code::UndefinedExpression,
                    cause: "unproved-refinement",
                    detail: format!(
                        "established interval [{lower}, {upper}] is not contained in [{narrow_lower}, {narrow_upper}] (obligation field-domain)"
                    ),
                }])),
                None => Ok(ConformanceOutcome::Refused(vec![AxisFailure {
                    axis: "refinement",
                    code: Code::UndefinedExpression,
                    cause: "unproved-refinement",
                    detail: format!(
                        "no establishing interval fact for {} (obligation field-domain)",
                        record.redefining.identity
                    ),
                }])),
            }
        }
        _ => Ok(ConformanceOutcome::Refused(vec![AxisFailure {
            axis: "refinement",
            code: Code::UndefinedExpression,
            cause: "unproved-refinement",
            detail: format!(
                "{} narrows an object-typed domain, which no FR-146 fact form expresses (obligation no-proof-form)",
                record.redefining.identity
            ),
        }])),
    }
}

/// Resolves which of `redefining`'s stated redefinition records name a
/// genuinely inherited target: a member declared on a proper ancestor of
/// `owner`, never `owner` itself. Zero or several distinct valid targets
/// refuse `redefinition-target` naming every candidate — never an arbitrary
/// pick among them.
pub fn resolve_redefinition_target(
    bundle: &Bundle,
    owner: &ProducerKey,
    redefining: &ProducerKey,
) -> Result<RedefinitionTargetOutcome, ModelRefusal> {
    let index = ConformanceIndex::build(bundle);
    let mut candidates: Vec<(ProducerKey, ProducerKey)> = Vec::new();
    let mut valid: Vec<ProducerKey> = Vec::new();

    for record in &index.redefinitions {
        if record.owner.identity != owner.identity
            || record.redefining.identity != redefining.identity
        {
            continue;
        }
        candidates.push((record.key.clone(), record.redefined.clone()));
        let Some(target_owner) = index.member_owner.get(&record.redefined) else {
            continue;
        };
        if target_owner == owner {
            continue; // declared directly on `owner`, not inherited.
        }
        if type_conforms(&index.generals_by_specific, owner, target_owner)? {
            valid.push(record.redefined.clone());
        }
    }

    let mut distinct: Vec<ProducerKey> = Vec::new();
    for target in &valid {
        if !distinct
            .iter()
            .any(|existing| existing.identity == target.identity)
        {
            distinct.push(target.clone());
        }
    }

    match distinct.len() {
        1 => Ok(RedefinitionTargetOutcome::Resolved(distinct.remove(0))),
        _ => Ok(RedefinitionTargetOutcome::Refused {
            cause: "redefinition-target",
            candidates,
        }),
    }
}

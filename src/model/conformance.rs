// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-151 conformance/redefinition/subsetting axis checking
//! (`quire.model.conformance.variance/v1`,
//! `quire.model.conformance.multiplicity/v1`,
//! `quire.model.conformance.effect/v1` and
//! `quire.model.conformance.refinement/v1`).
//!
//! This module works directly against a caller-constructed
//! [`crate::model::domain_package::DomainPackage`], not [`crate::model::normalize`]'s
//! [`crate::model::normalize::EffectiveView`]: it builds its own
//! [`crate::model::key::EffectiveDeclarationPreimage`]-shaped queries over
//! the domain package's field/operation members' own inline `redefines`/
//! `subsets` properties (`model-complete.md`:161/162), so field
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
//!   `crate::model` has no FR-146 expression parser, so a postcondition
//!   clause is not parsed from source: the caller states one accepted
//!   single-relation guard form directly, as
//!   [`crate::model::domain_package::PostconditionClause`]. What a clause actually
//!   establishes is not caller-trusted, though — [`check_field_refinement_obligation`]
//!   rebuilds the small typed guard tree each clause describes and runs it
//!   through `crate::value`'s own FR-146 fact-derivation primitive
//!   (`established_field_fact`), the identical guard-fact propagation a real
//!   checked postcondition's `Definedness::walk` already uses, then decides
//!   discharge from what that derivation actually proves.
//! - Exact per-axis work-unit costs (FR-151's `f(T)` formula, the length of
//!   a type's own derivation array) are not reproduced: computing `f(T)`
//!   here would require this module to re-walk `normalize`'s ancestor
//!   graph per axis, and FR-151's own worked examples price authored
//!   operation bodies this rung does not model. Every `conformance.axis`
//!   charge costs a flat one work unit; this is a recorded scope choice,
//!   not silent drift from the spec's numbers.
//! - `resolve_redefinition_target` resolves a redefining member's own single
//!   inline `redefines` property (`model-complete.md`:162) against exactly
//!   the one target it names: either that target is a genuinely inherited
//!   member and the call resolves, or it is not and the call refuses
//!   `redefinition-target` with [`RedefinitionTargetOutcome::Refused`]'s
//!   `candidate` naming the `(redefining, stated target)` pair. There is no
//!   "several distinct valid targets" shape to rule on: a member names at
//!   most one `redefines` target, never a set of candidates to choose among.
//!
//!   TC-196 R07's other shape — two distinct redefining members (e.g. `B/z`
//!   and `B/z2`) contending for the identical single inherited target — is
//!   *not* checked by `resolve_redefinition_target`: nothing in `src/`
//!   called it, so per-redefiner queries here could never see the sibling
//!   that contends with them. For **field** members that shape is instead
//!   detected where a model actually normalizes through it, `normalize`'s
//!   own phase 4 (`apply_redefinitions`'s undominated-edges branch), which
//!   already has every sibling redefiner of a contended target in view.
//!   `normalize`'s phase 4 is field-only, though (see its own module doc),
//!   so the identical shape for **operation** members is priced — a
//!   contested operation target's `normalize.conflict-check` charge is not
//!   skipped — but detected and resolved nowhere at all: no dominance
//!   search decides which competing operation redefiner wins, and no
//!   refusal reports an undominated operation contest the way
//!   `derivation-conflict`/`redefinition-target` do for fields. This
//!   predates this PR and is outside QSL #145's own scope. Remaining work:
//!   #173.
#![allow(
    clippy::large_enum_variant,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline"
)]
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::ControlFlow;

use crate::diagnostic::Code;
use crate::model::accounting::{Charge, ChargePoint, Incomplete, Meter};
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, FieldMemberRecord, Multiplicity, OperationMemberRecord,
    PostconditionClause, ValueTypeRef,
};
use crate::model::key::DeclarationKey;
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
use crate::model::population::redefinition_reaches;
use crate::value::{
    established_field_fact, Connective, Established, Integer, IntegerInterval, Location, Node,
    NodeKind, OrderedKind, OrderingOperator, Origin, ProvedInterval, Value, ValueType,
};

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
    pub cause: ModelRefusalCause,
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
/// own inline `redefines` property (`model-complete.md`:162) actually
/// targets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RedefinitionTargetOutcome {
    /// A valid inherited target.
    Resolved(DeclarationKey),
    /// `redefining`'s own `redefines` names no genuinely inherited member:
    /// either it names a member declared directly on `redefining`'s own
    /// owner rather than a proper ancestor, or the owner does not actually
    /// specialize the named target's declaring type.
    Refused {
        /// FR-151's cause tag.
        cause: ModelRefusalCause,
        /// `redefining`'s own `(redefining key, stated target)` pair —
        /// `None` when `redefining` declares no `redefines` at all. Its
        /// inline `redefines` property (`model-complete.md`:162) is
        /// singular, so this is at most one candidate.
        candidate: Option<(DeclarationKey, DeclarationKey)>,
    },
}

struct ConformanceIndex {
    generals_by_specific: HashMap<DeclarationKey, Vec<DeclarationKey>>,
    fields: HashMap<DeclarationKey, FieldMemberRecord>,
    /// A `BTreeMap`, not a `HashMap`: [`check_field_refinement_obligation`]
    /// scans `.values()` for the (assumed unique) writer of a field, and a
    /// `HashMap`'s `RandomState` iteration order made that scan
    /// nondeterministic across runs of the same domain package (finding #3).
    operations: BTreeMap<DeclarationKey, OperationMemberRecord>,
    scalars: HashMap<DeclarationKey, (i64, i64)>,
    /// Original member key -> its declaring type's key, for fields and
    /// operations alike.
    member_owner: HashMap<DeclarationKey, DeclarationKey>,
}

/// Builds the `specific key -> its declared supertypes[]` map every bounded
/// conformance walk in `crate::model` (this module and
/// [`crate::model::dispatch`]) needs. One builder, so the map's shape is a
/// single fact rather than a duplicated field-by-field copy. Keyed on the
/// full [`DeclarationKey`], not the display identity alone (PR #140 F2): two
/// object types whose key shares a display identity but differs in revision
/// must both index their own distinct ancestor set, never silently overwrite
/// one another. `supertypes` is an inline property of the object type itself
/// (`model-complete.md`:155).
pub(super) fn generals_by_specific(
    domain_package: &DomainPackage,
) -> HashMap<DeclarationKey, Vec<DeclarationKey>> {
    let mut generals_by_specific: HashMap<DeclarationKey, Vec<DeclarationKey>> = HashMap::new();
    for record in &domain_package.records {
        if let DomainPackageRecord::ObjectType(object_type) = record {
            if !object_type.supertypes.is_empty() {
                generals_by_specific
                    .entry(object_type.key.clone())
                    .or_default()
                    .extend(object_type.supertypes.iter().cloned());
            }
        }
    }
    generals_by_specific
}

impl ConformanceIndex {
    fn build(domain_package: &DomainPackage) -> Self {
        let generals_by_specific = generals_by_specific(domain_package);
        let mut fields = HashMap::new();
        let mut operations = BTreeMap::new();
        let mut scalars = HashMap::new();
        let mut member_owner = HashMap::new();
        for record in &domain_package.records {
            match record {
                DomainPackageRecord::ObjectType(_) => {}
                DomainPackageRecord::FieldMember(field) => {
                    member_owner.insert(field.key.clone(), field.owner.clone());
                    fields.insert(field.key.clone(), field.clone());
                }
                DomainPackageRecord::ScalarType(scalar) => {
                    scalars.insert(scalar.key.clone(), (scalar.lower, scalar.upper));
                }
                DomainPackageRecord::OperationMember(operation) => {
                    member_owner.insert(operation.key.clone(), operation.owner.clone());
                    operations.insert(operation.key.clone(), operation.clone());
                }
                // FR-152 systems-model records are not conformance-checked
                // by this module (crate::model::systems owns them); FR-153
                // population declarations are not conformance-checked here
                // either (crate::model::population owns them).
                DomainPackageRecord::Component(_)
                | DomainPackageRecord::Endpoint(_)
                | DomainPackageRecord::Relationship(_)
                | DomainPackageRecord::Allocation(_)
                | DomainPackageRecord::Population(_) => {}
            }
        }
        Self {
            generals_by_specific,
            fields,
            operations,
            scalars,
            member_owner,
        }
    }
}

/// The one bounded (explicit stack, visited set, [`MAX_CONFORMANCE_DEPTH`]
/// ceiling) proper-descendant DFS [`type_conforms`] needs. Previously also
/// shared with a phase-4 dominance helper (`ancestor_closure`, QSL #145)
/// that collected a full ancestor set rather than breaking on a target
/// match; that helper is gone (QSL #145 — phase 4
/// now derives each owner's ancestor set from `crate::model::normalize`'s
/// own phase-3 paths instead of a second, separately bounded walk here),
/// so `visit` only ever breaks or continues
/// with `()` now, but the shape is kept generic in case a future caller
/// needs a different break payload.
///
/// Calls `visit(current, general)` once per direct-generalization edge
/// discovered, in DFS pre-order: `current` is the node being expanded,
/// `general` its direct generalization. `visit` returns
/// [`ControlFlow::Break`] to stop the walk immediately —
/// [`type_conforms`]'s early exit on a target match — or
/// [`ControlFlow::Continue`] to keep walking and push `general` onto the
/// stack. The walk's own early termination is returned as this function's
/// `Ok` payload; a `Break` short-circuits before any further nodes are
/// popped.
///
/// `label` is the refusal detail's own subject phrase (`"conformance check
/// from"`).
fn walk_ancestors<B>(
    generals_by_specific: &HashMap<DeclarationKey, Vec<DeclarationKey>>,
    s: &DeclarationKey,
    label: &str,
    mut visit: impl FnMut(&DeclarationKey, &DeclarationKey) -> ControlFlow<B>,
) -> Result<ControlFlow<B>, ModelRefusal> {
    let mut stack: Vec<DeclarationKey> = vec![s.clone()];
    let mut visited: HashSet<DeclarationKey> = HashSet::new();
    let mut steps: usize = 0;
    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        steps += 1;
        if steps > MAX_CONFORMANCE_DEPTH {
            return Err(ModelRefusal {
                code: Code::ResourceExhausted,
                cause: ModelRefusalCause::ConformanceDepth { from: s.clone() },
                detail: format!(
                    "{label} {} exceeded {MAX_CONFORMANCE_DEPTH} generalization steps",
                    s.node
                ),
            });
        }
        for general in generals_by_specific.get(&current).into_iter().flatten() {
            match visit(&current, general) {
                ControlFlow::Break(value) => return Ok(ControlFlow::Break(value)),
                ControlFlow::Continue(()) => stack.push(general.clone()),
            }
        }
    }
    Ok(ControlFlow::Continue(()))
}

/// Whether `s` conforms to `t`: the same identity, or a chain of supplied
/// generalization records from `s` to `t`. Delegates to [`walk_ancestors`]
/// for the bounded (explicit stack, visited set, [`MAX_CONFORMANCE_DEPTH`]
/// ceiling) DFS itself, breaking as soon as `t` is found so a cycle or an
/// adversarial chain refuses instead of looping or overflowing a native
/// call stack.
///
/// `pub(super)` so [`crate::model::dispatch`]'s own bounded walks (subtype
/// applicability and dominance) reuse this one implementation rather than a
/// second copy.
pub(super) fn type_conforms(
    generals_by_specific: &HashMap<DeclarationKey, Vec<DeclarationKey>>,
    s: &DeclarationKey,
    t: &DeclarationKey,
) -> Result<bool, ModelRefusal> {
    if s == t {
        return Ok(true);
    }
    let found = walk_ancestors(
        generals_by_specific,
        s,
        "conformance check from",
        |_current, general| {
            if general == t {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )?;
    Ok(matches!(found, ControlFlow::Break(())))
}

/// [`type_conforms`], lifted to [`ValueTypeRef`]: a native value type
/// conforms only to itself (QSL declares no generalization among native
/// value types), and a package value type conforms exactly as
/// [`type_conforms`] already decides. A native and a package value type
/// never conform to one another.
pub(super) fn value_type_conforms(
    generals_by_specific: &HashMap<DeclarationKey, Vec<DeclarationKey>>,
    s: &ValueTypeRef,
    t: &ValueTypeRef,
) -> Result<bool, ModelRefusal> {
    match (s, t) {
        (ValueTypeRef::Native(a), ValueTypeRef::Native(b)) => Ok(a == b),
        (ValueTypeRef::Package(s_key), ValueTypeRef::Package(t_key)) => {
            type_conforms(generals_by_specific, s_key, t_key)
        }
        _ => Ok(false),
    }
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

fn missing_member(cause: ModelRefusalCause, identity: &str, role: &str) -> ModelRefusal {
    ModelRefusal {
        code: Code::DanglingReference,
        cause,
        detail: format!("{role} {identity} is not a declared member of the domain package"),
    }
}

/// Checks a field redefinition's `value-type` and `multiplicity` axes
/// (`quire.model.conformance.variance/v1`, `.../multiplicity/v1`).
pub fn check_field_redefinition(
    domain_package: &DomainPackage,
    redefining_key: &DeclarationKey,
    redefined_key: &DeclarationKey,
    meter: &mut Meter,
) -> ConformanceCheckOutcome {
    let index = ConformanceIndex::build(domain_package);
    let Some(redefining) = index.fields.get(redefining_key) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            ModelRefusalCause::UnknownRedefining {
                member: redefining_key.clone(),
            },
            &redefining_key.node,
            "redefining field",
        ));
    };
    let Some(redefined) = index.fields.get(redefined_key) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            ModelRefusalCause::UnknownRedefined {
                member: redefined_key.clone(),
            },
            &redefined_key.node,
            "redefined field",
        ));
    };

    let mut failures = Vec::new();

    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    match value_type_conforms(
        &index.generals_by_specific,
        &redefining.value_type,
        &redefined.value_type,
    ) {
        Ok(true) => {}
        Ok(false) => failures.push(AxisFailure {
            axis: "value-type",
            code: Code::IllTyped,
            cause: ModelRefusalCause::VarianceResult,
            detail: format!(
                "{} does not conform to {}",
                redefining.value_type, redefined.value_type
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
            cause: ModelRefusalCause::MultiplicityNarrowing {
                from: redefining.multiplicity,
                to: redefined.multiplicity,
            },
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

/// Checks a subsetting pair's `subsetting-type` and `multiplicity` axes
/// (same rule as field redefinition, different cause on the type axis).
/// Runtime population-membership checking
/// (`binding.subset-value`/`subsetting-violation`) is FR-153 territory, not
/// this static check.
pub fn check_subsetting(
    domain_package: &DomainPackage,
    subsetting_key: &DeclarationKey,
    subsetted_key: &DeclarationKey,
    meter: &mut Meter,
) -> ConformanceCheckOutcome {
    let index = ConformanceIndex::build(domain_package);
    let Some(subsetting) = index.fields.get(subsetting_key) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            ModelRefusalCause::UnknownSubsetting {
                member: subsetting_key.clone(),
            },
            &subsetting_key.node,
            "subsetting field",
        ));
    };
    let Some(subsetted) = index.fields.get(subsetted_key) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            ModelRefusalCause::UnknownSubsetted {
                member: subsetted_key.clone(),
            },
            &subsetted_key.node,
            "subsetted field",
        ));
    };

    let mut failures = Vec::new();

    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    match value_type_conforms(
        &index.generals_by_specific,
        &subsetting.value_type,
        &subsetted.value_type,
    ) {
        Ok(true) => {}
        Ok(false) => failures.push(AxisFailure {
            axis: "subsetting-type",
            code: Code::IllTyped,
            cause: ModelRefusalCause::SubsettingType {
                subsetting: subsetting.value_type.clone(),
                subsetted: subsetted.value_type.clone(),
            },
            detail: format!(
                "{} does not conform to {}",
                subsetting.value_type, subsetted.value_type
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
            cause: ModelRefusalCause::MultiplicityNarrowing {
                from: subsetting.multiplicity,
                to: subsetted.multiplicity,
            },
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
    domain_package: &DomainPackage,
    redefining_key: &DeclarationKey,
    redefined_key: &DeclarationKey,
    meter: &mut Meter,
) -> ConformanceCheckOutcome {
    let index = ConformanceIndex::build(domain_package);
    let Some(redefining) = index.operations.get(redefining_key) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            ModelRefusalCause::UnknownRedefining {
                member: redefining_key.clone(),
            },
            &redefining_key.node,
            "redefining operation",
        ));
    };
    let Some(redefined) = index.operations.get(redefined_key) else {
        return ConformanceCheckOutcome::Refused(missing_member(
            ModelRefusalCause::UnknownRedefined {
                member: redefined_key.clone(),
            },
            &redefined_key.node,
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
            cause: ModelRefusalCause::TypeMismatch,
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
            match value_type_conforms(&index.generals_by_specific, &dp.value_type, &rp.value_type) {
                Ok(true) => {}
                Ok(false) => failures.push(AxisFailure {
                    axis: "parameter-type",
                    code: Code::IllTyped,
                    cause: ModelRefusalCause::VarianceParameter {
                        index: display_index,
                        declared: dp.value_type.clone(),
                        redefined: rp.value_type.clone(),
                    },
                    detail: format!(
                        "parameter {display_index}: expected {} to conform to {}",
                        dp.value_type, rp.value_type
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
                    cause: ModelRefusalCause::MultiplicityNarrowing {
                        from: dp.multiplicity,
                        to: rp.multiplicity,
                    },
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
            match value_type_conforms(&index.generals_by_specific, &rr.value_type, &dr.value_type) {
                Ok(true) => {}
                Ok(false) => failures.push(AxisFailure {
                    axis: "result-type",
                    code: Code::IllTyped,
                    cause: ModelRefusalCause::VarianceResult,
                    detail: format!("{} does not conform to {}", rr.value_type, dr.value_type),
                }),
                Err(refusal) => return ConformanceCheckOutcome::Refused(refusal),
            }
        }
        (None, None) => {}
        _ => failures.push(AxisFailure {
            axis: "result-type",
            code: Code::IllTyped,
            cause: ModelRefusalCause::VarianceResult,
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
                cause: ModelRefusalCause::MultiplicityNarrowing {
                    from: rr.multiplicity,
                    to: dr.multiplicity,
                },
                detail: format!(
                    "{:?} does not conform to {:?}",
                    rr.multiplicity, dr.multiplicity
                ),
            });
        }
    }

    // Effect. A write is covered when it is granted directly or reaches a
    // granted field through the redefinition chain
    // ([`redefinition_reaches`]; QSL #171) -- not just one hop, since
    // model-complete.md:56/:64 make a multi-hop chain like `C.x -> B.x ->
    // A.x` legal with no direct `C.x -> A.x` record.
    if let Err(incomplete) = charge_axis(meter) {
        return ConformanceCheckOutcome::Incomplete(incomplete);
    }
    for write in &redefining.effect.modifies {
        let covered = redefinition_reaches(&domain_package.records, write, |candidate| {
            redefined.effect.modifies.contains(candidate)
        });
        if !covered {
            failures.push(AxisFailure {
                axis: "effect",
                code: Code::IllTyped,
                cause: ModelRefusalCause::EffectEscape {
                    field: write.clone(),
                },
                detail: format!(
                    "write {} is not covered by the redefined effect",
                    write.node
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
                    cause: ModelRefusalCause::EffectEscape {
                        field: entry.clone(),
                    },
                    detail: format!("{} is not covered by the redefined effect", entry.node),
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

/// `self`, bound as local slot 0, for every synthetic guard tree
/// [`self_field_node`] builds — the one variable a [`PostconditionClause`]
/// ever names.
const CLAUSE_SELF_SLOT: usize = 0;

fn clause_location() -> Location {
    Location {
        origin: Origin::Expression,
        path: Vec::new(),
    }
}

/// The synthetic `self.<field>` projection node every [`PostconditionClause`]
/// guard is built over: `self` at [`CLAUSE_SELF_SLOT`], projected at stable
/// path step zero (the one field a clause ever names, so the step index
/// itself carries no further meaning), declared at `value_type`.
fn self_field_node(value_type: ValueType) -> Node {
    let self_node = Node {
        kind: NodeKind::Local(CLAUSE_SELF_SLOT),
        value_type: ValueType::Integer,
        location: clause_location(),
    };
    Node {
        kind: NodeKind::Field {
            operand: Box::new(self_node),
            index: 0,
            optional: false,
        },
        value_type,
        location: clause_location(),
    }
}

/// `domain`'s value type: `None` (a non-scalar, or unknown, domain) is the
/// unbounded `ValueType::Integer` fallback, per FR-146's rule that a
/// projection onto the field a narrowing redefinition redefines carries the
/// declared facts of the *redefined* parent member, never the narrowing
/// type — so callers seed this from `redefined`'s own declared scalar
/// bounds, not `redefining`'s. `Some((lower, upper))` is the closed interval
/// type, or a [`ModelRefusal`] when a domain package's `ScalarTypeRecord` is
/// malformed (its own lower greater than its upper) — a real defect in
/// caller-supplied domain package data, refused rather than panicked on.
fn field_domain_type(domain: Option<(i64, i64)>) -> Result<ValueType, ModelRefusal> {
    match domain {
        Some((lower, upper)) => {
            let interval =
                IntegerInterval::new(Integer::from(lower), Integer::from(upper)).map_err(|_| {
                    ModelRefusal {
                        code: Code::InvalidModelBinding,
                        // FR-272's `invalid_model_binding` cause list is
                        // closed; there is no dedicated scalar-domain
                        // variant, so this is the catalogued
                        // `malformed-declaration` (its own payload: "the IR
                        // node identity and invalid member path" — here the
                        // scalar type's own declaration).
                        cause: ModelRefusalCause::MalformedDeclaration,
                        detail: format!(
                            "a scalar type's declared domain has lower {lower} greater than its upper {upper}"
                        ),
                    }
                })?;
            Ok(ValueType::Int(interval))
        }
        None => Ok(ValueType::Integer),
    }
}

/// The synthetic `present(self.<field>)` guard a [`PostconditionClause::Presence`]
/// clause describes. Always unbounded (`ValueType::Integer`): presence
/// never depends on a scalar domain, so this can never fail.
fn presence_condition() -> Node {
    Node {
        kind: NodeKind::Present(Box::new(self_field_node(ValueType::Integer))),
        value_type: ValueType::Boolean,
        location: clause_location(),
    }
}

/// The synthetic `self.<field> <operator> <literal>` guard a
/// [`PostconditionClause::Comparison`] clause describes.
fn comparison_condition(
    domain: Option<(i64, i64)>,
    operator: OrderingOperator,
    literal: i64,
) -> Result<Node, ModelRefusal> {
    let field = self_field_node(field_domain_type(domain)?);
    let literal_node = Node {
        kind: NodeKind::Literal(Value::Integer(Integer::from(literal))),
        value_type: ValueType::Integer,
        location: clause_location(),
    };
    Ok(Node {
        kind: NodeKind::Order(
            operator,
            OrderedKind::Integers,
            Box::new(field),
            Box::new(literal_node),
        ),
        value_type: ValueType::Boolean,
        location: clause_location(),
    })
}

/// Derives what `clauses` together actually establish about the field they
/// all name, seeding each synthetic guard's declared domain from `domain`
/// (the redefined parent member's own scalar bounds, when it has one). The
/// effective postcondition is a conjunction (see the module docs), so every
/// clause's guard is folded into one `Connective::And` tree and
/// [`established_field_fact`] runs once over that tree — not once per
/// clause with only the first surviving result kept — so a two-clause bound
/// such as `cs >= 0` and `cs <= 5` is proved together instead of only
/// whichever clause happened to be checked first. `Err` when `domain`
/// itself is malformed (see [`field_domain_type`]).
fn established_facts(
    clauses: &[&PostconditionClause],
    domain: Option<(i64, i64)>,
) -> Result<Established, ModelRefusal> {
    let condition = |clause: &&PostconditionClause| -> Result<Node, ModelRefusal> {
        match clause {
            PostconditionClause::Presence { .. } => Ok(presence_condition()),
            PostconditionClause::Comparison {
                operator, literal, ..
            } => comparison_condition(domain, *operator, *literal),
        }
    };
    let mut conditions = clauses.iter().map(condition);
    let Some(first) = conditions.next() else {
        return Ok(Established::default());
    };
    let mut folded = first?;
    for next in conditions {
        folded = Node {
            kind: NodeKind::Connective(Connective::And, Box::new(folded), Box::new(next?)),
            value_type: ValueType::Boolean,
            location: clause_location(),
        };
    }
    Ok(established_field_fact(&folded, CLAUSE_SELF_SLOT))
}

/// A human-readable `[lower, upper]` rendering of a [`ProvedInterval`], with
/// an unbounded end spelled out rather than omitted.
fn format_interval(interval: &ProvedInterval) -> String {
    let lower = interval
        .lower
        .as_ref()
        .map_or_else(|| "unbounded".to_owned(), Integer::to_string);
    let upper = interval
        .upper
        .as_ref()
        .map_or_else(|| "unbounded".to_owned(), Integer::to_string);
    format!("[{lower}, {upper}]")
}

/// The FR-151 refinement obligation: a narrowing field redefinition must be
/// established by a fact in the effective postcondition of the exposed
/// operation (own or inherited) that writes the redefined field. See the
/// module docs for the [`PostconditionClause`] scope decision this rests on.
pub fn check_field_refinement_obligation(
    domain_package: &DomainPackage,
    redefining_key: &DeclarationKey,
    redefined_key: &DeclarationKey,
) -> Result<ConformanceOutcome, ModelRefusal> {
    let index = ConformanceIndex::build(domain_package);
    let Some(redefining) = index.fields.get(redefining_key) else {
        return Err(missing_member(
            ModelRefusalCause::UnknownRedefining {
                member: redefining_key.clone(),
            },
            &redefining_key.node,
            "redefining field",
        ));
    };
    let Some(redefined) = index.fields.get(redefined_key) else {
        return Err(missing_member(
            ModelRefusalCause::UnknownRedefined {
                member: redefined_key.clone(),
            },
            &redefined_key.node,
            "redefined field",
        ));
    };

    let same_type = redefining.value_type == redefined.value_type;
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
        operation
            .effect
            .modifies
            .iter()
            .any(|field| field.node == redefined_key.node || field.node == redefining_key.node)
    });
    let Some(writer) = writer else {
        // No exposed operation writes this field: nothing to discharge.
        return Ok(ConformanceOutcome::Compatible);
    };

    let mut clauses: Vec<&PostconditionClause> = writer.own_postcondition_clauses.iter().collect();
    for operation in index.operations.values() {
        let Some(target) = &operation.redefines else {
            continue;
        };
        if target.node == writer.key.node && operation.owner.node == redefining.owner.node {
            clauses.extend(operation.own_postcondition_clauses.iter());
        }
    }
    let names_field = |key: &DeclarationKey| -> bool {
        key.node == redefined_key.node || key.node == redefining_key.node
    };

    // FR-146's own rule: a projection onto the field a narrowing redefinition
    // redefines carries the declared facts of the redefined PARENT member,
    // never the narrowing type — so every synthetic guard below is seeded
    // from `redefined`'s own declared scalar bounds, not `redefining`'s.
    let domain = redefined
        .value_type
        .as_package()
        .and_then(|key| index.scalars.get(key))
        .copied();
    let field_clauses: Vec<&PostconditionClause> = clauses
        .iter()
        .filter(|clause| names_field(clause.field()))
        .copied()
        .collect();
    let established = established_facts(&field_clauses, domain)?;

    if raises_lower && single_valued {
        return if established.presence {
            Ok(ConformanceOutcome::Compatible)
        } else {
            Ok(ConformanceOutcome::Refused(vec![AxisFailure {
                axis: "refinement",
                code: Code::UndefinedExpression,
                cause: ModelRefusalCause::UnprovedRefinement,
                detail: format!(
                    "{} narrows the multiplicity of {} with no establishing presence fact (obligation field-presence)",
                    redefining_key.node, redefined_key.node
                ),
            }]))
        };
    }

    if raises_lower && !single_valued {
        return Ok(ConformanceOutcome::Refused(vec![AxisFailure {
            axis: "refinement",
            code: Code::UndefinedExpression,
            cause: ModelRefusalCause::UnprovedRefinement,
            detail: format!(
                "{} narrows a collection upper bound, which no FR-146 fact form expresses (obligation no-proof-form)",
                redefining_key.node
            ),
        }]));
    }

    // Value type changed: field-domain via scalar interval containment, or
    // no-proof-form when either type is not a known scalar (an object type
    // narrowing, per FR-151's own example).
    match (
        redefining
            .value_type
            .as_package()
            .and_then(|key| index.scalars.get(key)),
        redefined
            .value_type
            .as_package()
            .and_then(|key| index.scalars.get(key)),
    ) {
        (Some(&(narrow_lower, narrow_upper)), Some(_)) => {
            let interval = established.interval.clone();
            let lower_bound = Integer::from(narrow_lower);
            let upper_bound = Integer::from(narrow_upper);
            match interval {
                Some(proved) => {
                    let contained = match (&proved.lower, &proved.upper) {
                        (Some(lower), Some(upper)) => *lower >= lower_bound && *upper <= upper_bound,
                        _ => false,
                    };
                    if contained {
                        Ok(ConformanceOutcome::Compatible)
                    } else {
                        Ok(ConformanceOutcome::Refused(vec![AxisFailure {
                            axis: "refinement",
                            code: Code::UndefinedExpression,
                            cause: ModelRefusalCause::UnprovedRefinement,
                            detail: format!(
                                "established interval {} is not contained in [{narrow_lower}, {narrow_upper}] (obligation field-domain)",
                                format_interval(&proved)
                            ),
                        }]))
                    }
                }
                None => Ok(ConformanceOutcome::Refused(vec![AxisFailure {
                    axis: "refinement",
                    code: Code::UndefinedExpression,
                    cause: ModelRefusalCause::UnprovedRefinement,
                    detail: format!(
                        "no establishing interval fact for {} (obligation field-domain)",
                        redefining_key.node
                    ),
                }])),
            }
        }
        _ => Ok(ConformanceOutcome::Refused(vec![AxisFailure {
            axis: "refinement",
            code: Code::UndefinedExpression,
            cause: ModelRefusalCause::UnprovedRefinement,
            detail: format!(
                "{} narrows an object-typed domain, which no FR-146 fact form expresses (obligation no-proof-form)",
                redefining_key.node
            ),
        }])),
    }
}

/// Resolves whether `redefining`'s own inline `redefines` property
/// (`model-complete.md`:162) names a genuinely inherited target: a member
/// declared on a proper ancestor of `redefining`'s own owner (read from the
/// index, never a caller-supplied claim), not on that owner itself.
///
/// This covers R07's *first* shape only: one redefining member queried in
/// isolation, whose own stated `redefines` resolves or does not. It cannot
/// see sibling redefiners of the same target (querying `B/z` alone has no
/// visibility into `B/z2`), so it does not — and cannot — detect R07's
/// *second* shape, several distinct members all redefining one shared
/// inherited target. That contention check runs where the real boundary can
/// see every redefiner at once: `normalize.rs`'s phase 4
/// (`apply_redefinitions`), not here. This function is currently unwired
/// from `src/`'s pipeline (like its `conformance.rs` siblings); QSL #165
/// composes it into the real pipeline's `conformance.axis` accounting.
pub fn resolve_redefinition_target(
    domain_package: &DomainPackage,
    redefining: &DeclarationKey,
) -> Result<RedefinitionTargetOutcome, ModelRefusal> {
    let index = ConformanceIndex::build(domain_package);

    let own_redefines = index
        .fields
        .get(redefining)
        .and_then(|field| field.redefines.clone())
        .or_else(|| {
            index
                .operations
                .get(redefining)
                .and_then(|operation| operation.redefines.clone())
        });
    let Some(target) = own_redefines else {
        return Ok(RedefinitionTargetOutcome::Refused {
            cause: ModelRefusalCause::RedefinitionTarget,
            candidate: None,
        });
    };

    let owner = index.member_owner.get(redefining).expect(
        "redefining names a declared field or operation member, indexed by ConformanceIndex::build under its own owner",
    );
    if let Some(target_owner) = index.member_owner.get(&target) {
        if target_owner != owner && type_conforms(&index.generals_by_specific, owner, target_owner)?
        {
            return Ok(RedefinitionTargetOutcome::Resolved(target));
        }
    }

    Ok(RedefinitionTargetOutcome::Refused {
        cause: ModelRefusalCause::RedefinitionTarget,
        candidate: Some((redefining.clone(), target)),
    })
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-151 dispatch linking (`quire.model.dispatch.single/v1`): candidate
//! family enumeration, per-subtype applicability, dominance and the linked
//! dispatch table.
//!
//! Scope decisions, recorded rather than left implicit:
//!
//! - This module performs **link-time linking only**. It does not implement
//!   `dispatch.select` (evaluating a receiver and choosing among a linked
//!   table at runtime) or precondition evaluation at call time (TC-196
//!   D01/D06): both need FR-146's expression evaluator, out of scope for
//!   `crate::model` — the same boundary [`crate::model::conformance`]'s
//!   module docs record.
//! - It does not build the FR-146 call graph or detect a `definition-cycle`
//!   through a dispatch edge (TC-196 D08): that needs the call graph itself,
//!   which nothing in this crate constructs. A caller with a real call graph
//!   can still use this module's linked table as one input to that check.
//! - `generalizationClosure` is not a [`crate::model::domain_package::DomainPackage`] or
//!   `DomainPackageRef` field: adding one would perturb every effective-view
//!   digest this rung already ships. [`link_dispatch`] takes it as an
//!   explicit parameter instead (TC-196 D05).
//! - Work-unit costs for `dispatch.candidate`/`dispatch.dominance` do not
//!   reproduce FR-151's exact per-type `f(T)` derivation-length formula, for
//!   the same reason [`crate::model::conformance`] does not for
//!   `conformance.axis`: computing it here would mean re-walking
//!   `normalize`'s derivation-fact accounting from this module. Every
//!   `dispatch.subtype`/`dispatch.candidate`/`dispatch.dominance` charge
//!   costs a flat one work unit; `dispatch.candidate` additionally sizes the
//!   real `dispatch_candidates` high-water counter, one per (subtype,
//!   candidate) charge in enumeration order, which is exact.
//! - Subtypes are enumerated in the order [`crate::model::normalize`]'s
//!   [`crate::model::normalize::EffectiveView`] already ships them:
//!   ascending by effective type identity. This module depends on that view
//!   only for ordering (never for redefinition/effect data, which it reads
//!   from the [`crate::model::domain_package::DomainPackage`] directly, mirroring
//!   `conformance`'s independence from phase 4's exposure machinery).
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::{HashMap, HashSet};

use crate::diagnostic::Code;
use crate::model::accounting::{Charge, ChargePoint, Incomplete, LimitKind, Meter};
use crate::model::conformance::{generals_by_specific, type_conforms};
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, OperationMemberRecord, RedefinitionRecord,
};
use crate::model::key::DeclarationKey;
use crate::model::normalize::{EffectiveView, ModelRefusal, ModelRefusalCause};

/// Bounds the family-closure walk `link_dispatch` performs over
/// caller-supplied redefinition records: an explicit task stack, never
/// native recursion, with a visited set and this depth ceiling as a typed
/// `resource_exhausted` refusal.
const MAX_DISPATCH_DEPTH: usize = 128;

/// Whether a [`crate::model::domain_package::DomainPackageRef`]'s generalization graph
/// is closed. See the module docs: this is not a `DomainPackage` field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneralizationClosure {
    /// Every generalization edge relevant to this operation is admitted.
    Closed,
    /// The graph is not known to be closed; linking cannot proceed.
    Open,
}

/// One strict dominance edge `dominant.owner` narrower than
/// `dominated.owner`, reported alongside an ambiguous subtype.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DominancePair {
    /// The dominating candidate's own original key.
    pub dominant: DeclarationKey,
    /// The dominated candidate's own original key.
    pub dominated: DeclarationKey,
}

/// One subtype that failed to link: `ambiguous_dispatch`/`no-applicable`
/// (zero applicable candidates) or `ambiguous_dispatch`/`multiple-undominated`
/// (several undominated applicable candidates).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubtypeRefusal {
    /// The subtype this refusal is about.
    pub subtype: DeclarationKey,
    /// Always `Code::AmbiguousDispatch` (FR-151-AC-3 names it explicitly).
    pub code: Code,
    /// FR-151's cause tag: [`ModelRefusalCause::NoApplicable`] or
    /// [`ModelRefusalCause::MultipleUndominated`].
    pub cause: ModelRefusalCause,
    /// Every applicable candidate's own original key, in enumeration order.
    pub candidates: Vec<DeclarationKey>,
    /// Every strict dominance pair found among the applicable candidates
    /// (empty for `no-applicable`).
    pub dominance_pairs: Vec<DominancePair>,
}

/// A linked dispatch table: one entry per subtype, naming the unique
/// undominated candidate linked for it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchTable {
    entries: Vec<(DeclarationKey, DeclarationKey)>,
}

impl DispatchTable {
    /// Every `(subtype, linked candidate)` entry, in subtype enumeration
    /// order.
    pub fn entries(&self) -> &[(DeclarationKey, DeclarationKey)] {
        &self.entries
    }

    /// The candidate linked for `subtype`, if this table has an entry for it.
    /// Matches on the full [`DeclarationKey`], never the display identity
    /// alone (finding #2): two subtypes sharing an identity but differing
    /// in revision must resolve independently, not "first identity match
    /// wins."
    pub fn linked_for(&self, subtype: &DeclarationKey) -> Option<&DeclarationKey> {
        self.entries
            .iter()
            .find(|(s, _)| s == subtype)
            .map(|(_, candidate)| candidate)
    }
}

/// `incomplete_population`/`unclosed-method-set`: an open generalization
/// closure has no dispatch table (TC-196 D05). Not a refusal — reported
/// before any dispatch charge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnclosedMethodSet {
    /// Always `Code::IncompletePopulation`.
    pub code: Code,
    /// Always [`ModelRefusalCause::UnclosedMethodSet`].
    pub cause: ModelRefusalCause,
    /// The operation this incomplete linking attempt was for.
    pub operation: DeclarationKey,
}

/// The substantive result of one successful linking pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DispatchLinkOutcome {
    /// Every subtype linked to a unique undominated candidate.
    Linked(DispatchTable),
    /// At least one subtype failed to link; every failing subtype, in
    /// enumeration order, and no dispatch table (linking is exhaustive: it
    /// still charges and reports every other subtype's outcome internally,
    /// but produces a table only when every subtype links).
    Ambiguous(Vec<SubtypeRefusal>),
}

/// The outcome of one [`link_dispatch`] attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkCheckOutcome {
    /// Every dispatch charge was admitted; see the substantive
    /// [`DispatchLinkOutcome`].
    Completed(DispatchLinkOutcome),
    /// A real defect (a dangling operation reference, or a family/dominance
    /// walk deeper than [`MAX_DISPATCH_DEPTH`]) refused the check outright.
    Refused(ModelRefusal),
    /// A `ModelNormalizationLimitsV1` counter was exhausted mid-link.
    Incomplete(Incomplete),
    /// The generalization closure was not [`GeneralizationClosure::Closed`].
    OpenClosure(UnclosedMethodSet),
}

struct DispatchIndex {
    operations: HashMap<DeclarationKey, OperationMemberRecord>,
    redefinitions: Vec<RedefinitionRecord>,
    generals_by_specific:
        HashMap<DeclarationKey, Vec<crate::model::domain_package::SupertypeRecord>>,
    /// D05 (`model-complete.md:156`, FR-151 "Dispatch rules": "For every
    /// effective type `S` conforming to `T`... that is not abstract"):
    /// object-type keys declared `abstract`, so [`link_dispatch`]'s subtype
    /// enumeration excludes them from ever becoming a dispatch target.
    abstract_types: HashSet<DeclarationKey>,
}

impl DispatchIndex {
    fn build(domain_package: &DomainPackage) -> Self {
        let mut operations = HashMap::new();
        let mut redefinitions = Vec::new();
        let mut abstract_types = HashSet::new();
        for record in &domain_package.records {
            match record {
                DomainPackageRecord::OperationMember(operation) => {
                    operations.insert(operation.key.clone(), operation.clone());
                }
                DomainPackageRecord::Redefinition(redefinition) => {
                    redefinitions.push(redefinition.clone());
                }
                DomainPackageRecord::ObjectType(object) => {
                    if object.abstract_type {
                        abstract_types.insert(object.key.clone());
                    }
                }
                DomainPackageRecord::FieldMember(_)
                | DomainPackageRecord::Supertype(_)
                | DomainPackageRecord::ScalarType(_)
                | DomainPackageRecord::Subsetting(_)
                | DomainPackageRecord::Component(_)
                | DomainPackageRecord::Endpoint(_)
                | DomainPackageRecord::Relationship(_)
                | DomainPackageRecord::Population(_) => {}
            }
        }
        Self {
            operations,
            redefinitions,
            generals_by_specific: generals_by_specific(domain_package),
            abstract_types,
        }
    }
}

/// The family of `original`: `original` itself together with every
/// redefining operation reaching it, by any chain of redefinition records,
/// sorted ascending by [`DeclarationKey`] for deterministic reporting.
/// Bounded task stack, never native recursion, over caller-supplied records.
fn build_family(
    index: &DispatchIndex,
    original: &DeclarationKey,
) -> Result<Vec<DeclarationKey>, ModelRefusal> {
    let mut family = vec![original.clone()];
    let mut frontier: Vec<DeclarationKey> = vec![original.clone()];
    let mut visited: HashSet<DeclarationKey> = HashSet::new();
    visited.insert(original.clone());
    let mut steps: usize = 0;
    while let Some(target) = frontier.pop() {
        steps += 1;
        if steps > MAX_DISPATCH_DEPTH {
            return Err(ModelRefusal {
                code: Code::ResourceExhausted,
                cause: ModelRefusalCause::DispatchFamilyDepth {
                    original: original.clone(),
                },
                detail: format!(
                    "dispatch family for {} exceeded {MAX_DISPATCH_DEPTH} redefinition steps",
                    original.node
                ),
            });
        }
        for redefinition in &index.redefinitions {
            if redefinition.redefined == target && visited.insert(redefinition.redefining.clone()) {
                family.push(redefinition.redefining.clone());
                frontier.push(redefinition.redefining.clone());
            }
        }
    }
    family.sort();
    Ok(family)
}

/// Whether `p` (by its owner) strictly dominates `q`: `p`'s owner is a
/// proper descendant of `q`'s owner.
fn dominates(
    generals_by_specific: &HashMap<
        DeclarationKey,
        Vec<crate::model::domain_package::SupertypeRecord>,
    >,
    p_owner: &DeclarationKey,
    q_owner: &DeclarationKey,
) -> Result<bool, ModelRefusal> {
    if p_owner == q_owner {
        return Ok(false);
    }
    type_conforms(generals_by_specific, p_owner, q_owner)
}

/// Links `original`'s dispatch family across every effective type `view`
/// admits, per `quire.model.dispatch.single/v1`. See the module docs for
/// this pass's exact scope (link-time only; no runtime selection, no
/// call-graph cycle detection).
pub fn link_dispatch(
    domain_package: &DomainPackage,
    view: &EffectiveView,
    original: &DeclarationKey,
    closure: GeneralizationClosure,
    meter: &mut Meter,
) -> LinkCheckOutcome {
    if matches!(closure, GeneralizationClosure::Open) {
        return LinkCheckOutcome::OpenClosure(UnclosedMethodSet {
            code: Code::IncompletePopulation,
            cause: ModelRefusalCause::UnclosedMethodSet,
            operation: original.clone(),
        });
    }

    let index = DispatchIndex::build(domain_package);
    let Some(receiver_operation) = index.operations.get(original) else {
        return LinkCheckOutcome::Refused(ModelRefusal {
            code: Code::DanglingReference,
            cause: ModelRefusalCause::UnknownOriginal {
                original: original.clone(),
            },
            detail: format!("{} is not a declared operation member", original.node),
        });
    };
    let receiver_type = receiver_operation.owner.clone();

    let family = match build_family(&index, original) {
        Ok(family) => family,
        Err(refusal) => return LinkCheckOutcome::Refused(refusal),
    };
    let mut candidates: Vec<DeclarationKey> = family
        .into_iter()
        .filter(|member| index.operations.get(member).is_some_and(|op| op.has_body))
        .collect();
    candidates.sort();

    // Every effective type conforming to the receiver type, in ascending
    // effective-identity order (the view's own order): type-level entries
    // only, filtered by domain-package-level conformance to `receiver_type`.
    let mut subtypes: Vec<DeclarationKey> = Vec::new();
    for entry in &view.declarations {
        if entry.preimage.owner_effective_type.is_some() {
            continue; // a member entry, not a type entry.
        }
        let candidate_subtype = &entry.preimage.original;
        if index.abstract_types.contains(candidate_subtype) {
            continue; // D05: an abstract subtype is never a dispatch target.
        }
        match type_conforms(
            &index.generals_by_specific,
            candidate_subtype,
            &receiver_type,
        ) {
            Ok(true) => subtypes.push(candidate_subtype.clone()),
            Ok(false) => {}
            Err(refusal) => return LinkCheckOutcome::Refused(refusal),
        }
    }

    let mut candidate_charges: u64 = 0;
    let mut table: Vec<(DeclarationKey, DeclarationKey)> = Vec::new();
    let mut refusals: Vec<SubtypeRefusal> = Vec::new();

    for subtype in &subtypes {
        if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::DispatchSubtype)) {
            return LinkCheckOutcome::Incomplete(incomplete);
        }

        let mut applicable: Vec<DeclarationKey> = Vec::new();
        for candidate in &candidates {
            candidate_charges += 1;
            if let Err(incomplete) = meter.charge(
                Charge::new(ChargePoint::DispatchCandidate)
                    .size(LimitKind::DispatchCandidates, candidate_charges),
            ) {
                return LinkCheckOutcome::Incomplete(incomplete);
            }
            let Some(candidate_record) = index.operations.get(candidate) else {
                return LinkCheckOutcome::Refused(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: ModelRefusalCause::UnknownCandidate {
                        candidate: candidate.clone(),
                    },
                    detail: format!("{} is not a declared operation member", candidate.node),
                });
            };
            let candidate_owner = &candidate_record.owner;
            match type_conforms(&index.generals_by_specific, subtype, candidate_owner) {
                Ok(true) => applicable.push(candidate.clone()),
                Ok(false) => {}
                Err(refusal) => return LinkCheckOutcome::Refused(refusal),
            }
        }

        if applicable.is_empty() {
            refusals.push(SubtypeRefusal {
                subtype: subtype.clone(),
                code: Code::AmbiguousDispatch,
                cause: ModelRefusalCause::NoApplicable,
                candidates: Vec::new(),
                dominance_pairs: Vec::new(),
            });
            continue;
        }

        if applicable.len() == 1 {
            table.push((subtype.clone(), applicable[0].clone()));
            continue;
        }

        if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::DispatchDominance)) {
            return LinkCheckOutcome::Incomplete(incomplete);
        }
        let mut dominance_pairs: Vec<DominancePair> = Vec::new();
        let mut dominated: HashSet<DeclarationKey> = HashSet::new();
        for p in &applicable {
            let Some(p_record) = index.operations.get(p) else {
                return LinkCheckOutcome::Refused(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: ModelRefusalCause::UnknownCandidate {
                        candidate: p.clone(),
                    },
                    detail: format!("{} is not a declared operation member", p.node),
                });
            };
            let p_owner = &p_record.owner;
            for q in &applicable {
                if p == q {
                    continue;
                }
                let Some(q_record) = index.operations.get(q) else {
                    return LinkCheckOutcome::Refused(ModelRefusal {
                        code: Code::DanglingReference,
                        cause: ModelRefusalCause::UnknownCandidate {
                            candidate: q.clone(),
                        },
                        detail: format!("{} is not a declared operation member", q.node),
                    });
                };
                let q_owner = &q_record.owner;
                match dominates(&index.generals_by_specific, p_owner, q_owner) {
                    Ok(true) => {
                        dominance_pairs.push(DominancePair {
                            dominant: p.clone(),
                            dominated: q.clone(),
                        });
                        dominated.insert(q.clone());
                    }
                    Ok(false) => {}
                    Err(refusal) => return LinkCheckOutcome::Refused(refusal),
                }
            }
        }
        let undominated: Vec<DeclarationKey> = applicable
            .iter()
            .filter(|c| !dominated.contains(*c))
            .cloned()
            .collect();

        if undominated.len() == 1 {
            table.push((subtype.clone(), undominated[0].clone()));
        } else {
            refusals.push(SubtypeRefusal {
                subtype: subtype.clone(),
                code: Code::AmbiguousDispatch,
                cause: ModelRefusalCause::MultipleUndominated,
                candidates: applicable,
                dominance_pairs,
            });
        }
    }

    if refusals.is_empty() {
        LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(DispatchTable {
            entries: table,
        }))
    } else {
        LinkCheckOutcome::Completed(DispatchLinkOutcome::Ambiguous(refusals))
    }
}

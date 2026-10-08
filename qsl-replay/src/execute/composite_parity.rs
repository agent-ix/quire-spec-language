// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-358: [`replay_composite_parity`] and [`settle_verified_shadow`], the
//! replay facade's two entries for a composite `bounded_shadow`
//! equality-parity claim.
//!
//! Both recompile the request's package, select the claimed node, derive its
//! operands' positions and bounds, and tie the obligation identity before
//! anything settles ([`prepare`]). The falsified entry then evaluates the
//! language's exact equality on the operands CG decoded and compares it with
//! the retained shadow result; the verified entry reads no native
//! observation and tells `Proved` from `Tested` by what the harness covered.
//! Neither selects a `Boolean` function or calls [`super::replay`], and no
//! row settles `Refuted`.

use std::collections::BTreeMap;

use qsl_foundation::bound::DomainKey;
use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::WireNodeId;
use quire_exact::IntegerInterval;
use quire_exact::{plan_equality, planned_equality, Meter, Outcome, ScalarLimits, Value};
use quire_semantic_value::declaration::EqualityOperator;

use super::argument::{self, Stopped};
use super::composite_domain::derive;
use super::composite_site::{locate, Operand, Site};
use super::operator_parity::ScalarIdentityMismatch;
use super::parity_identity::{
    parity_obligation, BoundEntries, Domain, ParityArgument, ParityPreimage,
};
use super::scalar_site::operand_identities;
use super::{consumed, exceeded_before_call, recompile, request_limits, Recompiled, ReplayRefusal};
use crate::bounds::ReplayLimits;
use crate::composite::{
    CompositeEvidence, CompositeIdentity, CompositeParityClaim, CompositeParityReport,
    CompositeParityResult, EqualityOutcome, FalsifiedParity, IncompleteStage,
    NativeParityObservation, Refinement, VerifiedShadow, VerifiedShadowReport,
    VerifiedShadowResult,
};
use crate::identity::ObligationIdentity;
use crate::request::{ReplayRequest, ReplayRequestWire};
use crate::scalar::{OperandIdentity, OperandRefusal, ScalarAgreement, ScalarClaim, ScalarOutcome};
use crate::witness::WitnessValue;

/// What the common steps (FR-358 steps 1 to 6) settled about a request and
/// its claim.
struct Prepared {
    request: ReplayRequest,
    compiled: Recompiled,
    operands: [Operand; 2],
    /// Whether every derived position is covered by the harness bounds.
    covered: bool,
}

/// FR-358's common steps, in order, stopping at the first refusal.
fn prepare(
    wire: ReplayRequestWire,
    claim: &CompositeParityClaim,
    replay_limits: ReplayLimits,
) -> Result<Prepared, ReplayRefusal> {
    let request = ReplayRequest::decode(wire, replay_limits)?;
    let limits = request_limits(request.stage_limits(), replay_limits)?;
    let compiled = recompile(&request, &limits)?;
    let site = locate(
        &compiled,
        request.selected_function(),
        claim.node,
        &claim.occurrence,
        claim.operator,
    )?;
    let package = compiled.checked.package();
    let parameters = distinct_parameters(&site);
    let limit = limits.spine.checking.nodes();
    let mut positions = derive(&parameters, package, limit)?;
    positions.declare(request.declared_domains())?;
    let harness = positions.harness(&claim.harness_bounds)?;
    identity_tie(&request, &compiled, &site, claim)?;
    let covered = positions.covered(&harness);
    Ok(Prepared {
        request,
        compiled,
        operands: site.operands,
        covered,
    })
}

/// The distinct parameter operands of the claimed node, ascending by node id,
/// each with its declared type.
fn distinct_parameters(site: &Site) -> Vec<(WireNodeId, &quire_exact::ValueType)> {
    let mut parameters: BTreeMap<WireNodeId, &quire_exact::ValueType> = BTreeMap::new();
    for operand in &site.operands {
        if let Operand::Parameter { node, value_type } = operand {
            parameters.insert(*node, value_type);
        }
    }
    parameters.into_iter().collect()
}

/// FR-358 step 6, the one call site of the identity tie. The claimed node's
/// operands are checked against the recompiled package position by position
/// (a parameter operand must be the node's child at its position), then
/// ADR-013 O-09's one parity preimage is built (operator application at the
/// node, one argument per operand: a parameter's domain is its harness bounds,
/// a literal's has no free position) and recomputed through [`parity_obligation`];
/// a digest that differs from the request's, or a preimage the encoder
/// refuses, refuses `ScalarIdentity` and never yields an identity.
pub(super) fn identity_tie(
    request: &ReplayRequest,
    compiled: &Recompiled,
    site: &Site,
    claim: &CompositeParityClaim,
) -> Result<(), ReplayRefusal> {
    let mismatch = |cause: ScalarIdentityMismatch| ReplayRefusal::ScalarIdentity(Box::new(cause));
    let preimage = parity_preimage(compiled, site, claim)?;
    let recomputed = parity_obligation(&preimage)
        .map_err(|error| mismatch(ScalarIdentityMismatch::Encoding(error)))?;
    if recomputed == request.obligation_identity() {
        Ok(())
    } else {
        Err(mismatch(ScalarIdentityMismatch::Obligation {
            claimed: request.obligation_identity(),
            recomputed,
        }))
    }
}

/// The O-09 parity preimage of `claim` at `site`, after the membership
/// checks against the recompiled package.
pub(super) fn parity_preimage(
    compiled: &Recompiled,
    site: &Site,
    claim: &CompositeParityClaim,
) -> Result<ParityPreimage, ReplayRefusal> {
    let mismatch = |cause: ScalarIdentityMismatch| ReplayRefusal::ScalarIdentity(Box::new(cause));
    let graph = compiled.checked.package().graph().semantic_graph();
    let node = graph
        .resolve_wire(claim.node)
        .and_then(|key| graph.node(key))
        .ok_or_else(|| fault("claimed-node-is-in-the-graph"))?;
    let identities = operand_identities(node, claim.node).map_err(mismatch)?;
    let mut arguments = Vec::with_capacity(2);
    for (position, (operand, identity)) in site.operands.iter().zip(identities).enumerate() {
        let domain = match (operand, identity) {
            (
                Operand::Parameter {
                    node: parameter, ..
                },
                OperandIdentity::GraphChild(found),
            ) => {
                if found != *parameter {
                    return Err(mismatch(ScalarIdentityMismatch::OperandChild {
                        node: claim.node,
                        position,
                        claimed: *parameter,
                        found: Some(found),
                    }));
                }
                let bounds = claim
                    .harness_bounds
                    .iter()
                    .filter(|bound| match bound.domain() {
                        DomainKey::Node { node, .. } => node == parameter,
                        DomainKey::Population { .. } => false,
                    })
                    .cloned()
                    .collect();
                Domain::Bounds(
                    BoundEntries::new(bounds)
                        .map_err(|error| mismatch(ScalarIdentityMismatch::Encoding(error)))?,
                )
            }
            (
                Operand::Parameter {
                    node: parameter, ..
                },
                OperandIdentity::InlineLiteral,
            ) => {
                return Err(mismatch(ScalarIdentityMismatch::OperandChild {
                    node: claim.node,
                    position,
                    claimed: *parameter,
                    found: None,
                }))
            }
            (Operand::Literal { value, .. }, OperandIdentity::InlineLiteral) => {
                let Value::Integer(value) = value else {
                    return Err(mismatch(ScalarIdentityMismatch::NotInlineLiteral {
                        node: claim.node,
                        position,
                    }));
                };
                Domain::Range(IntegerInterval::spanning(value.clone(), value.clone()))
            }
            // A composite literal is a graph node of its own: it ranges over
            // exactly its value, so it names no free position.
            (Operand::Literal { .. }, OperandIdentity::GraphChild(_)) => Domain::Bounds(
                BoundEntries::new(Vec::new())
                    .map_err(|error| mismatch(ScalarIdentityMismatch::Encoding(error)))?,
            ),
        };
        arguments.push(ParityArgument { identity, domain });
    }
    Ok(ParityPreimage {
        node: claim.node,
        occurrence: site.occurrence.origin().clone(),
        obligation_kind: claim.obligation_kind.clone(),
        arguments,
    })
}

/// FR-358: replay a falsified composite `bounded_shadow` item.
///
/// `wire` is the original proving context: its `package_id`, package
/// reference, dependencies and byte provision, the enclosing function of the
/// claimed node as `selected_function`, the `obligation_identity` CG minted
/// and the proving run's declared domains; its `source` is not read. `claim`
/// names the equality node and the harness bounds, and `falsified` carries
/// the operands CG decoded from the falsifying draw with the retained shadow
/// comparison, the driver's native observation and the refinement evidence.
///
/// `replay_limits` is `replay.input_bytes`, as [`crate::replay`] takes it.
/// The common steps refuse first, with the full claim carried
/// ([`CompositeParityReport::claim`]). Then the first matching row settles:
/// F-1 a refinement disagreement, F-2 a native stop, F-3 an operand failing
/// admission, F-4 an accounting limit hit while admitting, F-5 QSL's own
/// exact-evaluation limit, F-6 a refinement ceiling and F-7 the comparison.
pub fn replay_composite_parity(
    wire: ReplayRequestWire,
    claim: CompositeParityClaim,
    falsified: FalsifiedParity,
    replay_limits: ReplayLimits,
) -> CompositeParityReport {
    let obligation = ObligationIdentity::from_digest(wire.obligation_identity);
    let identity = CompositeIdentity::new(
        obligation,
        &claim,
        CompositeEvidence::Falsified(Box::new(falsified.clone())),
    );
    let result = match prepare(wire, &claim, replay_limits)
        .and_then(|prepared| falsified_rows(&prepared, &claim, &identity, falsified))
    {
        Ok(result) => result,
        Err(refusal) => CompositeParityResult::Refused(Box::new(refusal)),
    };
    CompositeParityReport::new(identity, result)
}

fn falsified_rows(
    prepared: &Prepared,
    claim: &CompositeParityClaim,
    identity: &CompositeIdentity,
    falsified: FalsifiedParity,
) -> Result<CompositeParityResult, ReplayRefusal> {
    // F-1.
    if falsified.refinement == Refinement::Disagreed {
        return Ok(CompositeParityResult::Disagreed);
    }
    // F-2: before admission, so an artifact fault is never hidden.
    match &falsified.native {
        NativeParityObservation::Incomplete(_) | NativeParityObservation::ExecutionFault(_) => {
            return Ok(CompositeParityResult::GeneratedFault {
                native: falsified.native,
            })
        }
        NativeParityObservation::Completed(_) | NativeParityObservation::Refused(_) => {}
    }
    // F-3 and F-4.
    let admitted = match admit(prepared, &falsified.operands)? {
        Admission::Admitted(admitted) => admitted,
        Admission::Refused(index) => {
            return Ok(CompositeParityResult::RefusedInput(OperandRefusal {
                index,
            }))
        }
        Admission::Stopped(incomplete) => {
            return Ok(CompositeParityResult::Incomplete {
                stage: IncompleteStage::Admission(incomplete),
            })
        }
    };
    // F-5.
    let [left, right] = &admitted;
    let (exact, charges) = match exact_equality(left, right, claim.operator, claim.limits)? {
        Exact::Outcome(exact, charges) => (exact, charges),
        Exact::Stopped(incomplete) => {
            return Ok(CompositeParityResult::Incomplete {
                stage: IncompleteStage::ExactEvaluation(incomplete),
            })
        }
    };
    // F-6.
    if falsified.refinement == Refinement::CeilingReached {
        return Ok(CompositeParityResult::Incomplete {
            stage: IncompleteStage::RefinementCeiling,
        });
    }
    // F-7.
    Ok(if exact == falsified.shadow {
        CompositeParityResult::Agrees {
            agreement: ScalarAgreement::new(
                ScalarClaim::CompositeEquality(Box::new(identity.clone())),
                ScalarOutcome::Equality(exact),
            ),
            charges,
        }
    } else {
        CompositeParityResult::Diverged {
            exact,
            shadow: falsified.shadow,
            charges,
        }
    })
}

/// What admitting the two operands found.
enum Admission {
    Admitted([Value; 2]),
    /// The operand at this position failed admission.
    Refused(usize),
    /// An accounting limit of the request stopped the admission.
    Stopped(Box<quire_exact::Incomplete>),
}

/// Admit each operand against its domain through FR-098's conversion and
/// S6a admission, under the request's accounting limits: a parameter operand
/// against its declared type, a literal operand when it equals the
/// package's literal under the kernel's equality relation.
fn admit(prepared: &Prepared, operands: &[WitnessValue; 2]) -> Result<Admission, ReplayRefusal> {
    let package = prepared.compiled.checked.package();
    let limits = prepared.request.accounting_limits();
    let mut admitted = Vec::with_capacity(2);
    for (index, (operand, witness)) in prepared.operands.iter().zip(operands).enumerate() {
        let declared = std::slice::from_ref(operand.value_type());
        let mut converted = match argument::convert_arguments(
            package,
            declared,
            std::slice::from_ref(witness),
            &limits,
        ) {
            Ok(converted) => converted,
            Err(Stopped::Refusal(refusal)) => match *refusal {
                ReplayRefusal::Input(_) => return Ok(Admission::Refused(index)),
                other => return Err(other),
            },
            Err(Stopped::Limit(incomplete)) => return Ok(Admission::Stopped(incomplete)),
        };
        if let Some(incomplete) = exceeded_before_call(&converted, &limits) {
            return Ok(Admission::Stopped(Box::new(incomplete)));
        }
        let value = converted
            .values
            .pop()
            .ok_or_else(|| fault("converted-operand"))?;
        if let Operand::Literal { value: literal, .. } = operand {
            let mut meter = Meter::new(argument::UNLIMITED);
            match planned_equality(&value, literal, &mut meter) {
                Outcome::Completed(true) => {}
                Outcome::Completed(false)
                | Outcome::Undefined(_)
                | Outcome::Refused(_)
                | Outcome::Incomplete(_) => return Ok(Admission::Refused(index)),
            }
        }
        admitted.push(value);
    }
    let [left, right] = <[Value; 2]>::try_from(admitted).map_err(|_| fault("two-operands"))?;
    Ok(Admission::Admitted([left, right]))
}

fn fault(invariant: &'static str) -> ReplayRefusal {
    ReplayRefusal::Fault(InternalFault::new("replay", invariant))
}

/// QSL's exact equality.
enum Exact {
    Outcome(EqualityOutcome, ScalarLimits),
    Stopped(Box<quire_exact::Incomplete>),
}

/// The kernel's equality relation on `left` and `right` under `limits`: the
/// verdict under `operator` (`NotEqual` negates `Equal`, with the same pair
/// count) and the occurrence-pair count, counted with no early exit after an
/// unequal pair (QSpec FR-149).
fn exact_equality(
    left: &Value,
    right: &Value,
    operator: EqualityOperator,
    limits: ScalarLimits,
) -> Result<Exact, ReplayRefusal> {
    let mut meter = Meter::new(limits);
    let verdict = match planned_equality(left, right, &mut meter) {
        Outcome::Completed(verdict) => verdict,
        Outcome::Incomplete(incomplete) => return Ok(Exact::Stopped(Box::new(incomplete))),
        Outcome::Undefined(_) | Outcome::Refused(_) => {
            return Err(fault("admitted-operands-compare"))
        }
    };
    let pairs = plan_equality(left, right)
        .map_err(|_| fault("admitted-operands-plan"))?
        .pair_events()
        .to_u64()
        .ok_or_else(|| fault("pair-count-fits-u64"))?;
    let equal = match operator {
        EqualityOperator::Equal => verdict,
        EqualityOperator::NotEqual => !verdict,
    };
    Ok(Exact::Outcome(
        EqualityOutcome {
            equal,
            pair_count: pairs,
        },
        consumed(&meter),
    ))
}

/// FR-358: settle a verified composite `bounded_shadow` item.
///
/// `wire`, `claim` and the common steps are as [`replay_composite_parity`]'s;
/// this entry takes no native observation and no operands. `verified` carries
/// the SUCCESS count of the verified run and the refinement evidence. The
/// first matching row settles: V-1 a refinement disagreement (`Failed`), V-2
/// a refinement ceiling (`Incomplete`), V-3 no SUCCESS check (the vacuous
/// `Proved { 0 }`), V-4 an exhaustive refinement with every derived position
/// covered (`Proved`) and V-5 otherwise (`Tested`, never promoted).
pub fn settle_verified_shadow(
    wire: ReplayRequestWire,
    claim: CompositeParityClaim,
    verified: VerifiedShadow,
    replay_limits: ReplayLimits,
) -> VerifiedShadowReport {
    let obligation = ObligationIdentity::from_digest(wire.obligation_identity);
    let identity =
        CompositeIdentity::new(obligation, &claim, CompositeEvidence::Verified(verified));
    let result = match prepare(wire, &claim, replay_limits) {
        Ok(prepared) => verified_rows(&prepared, verified),
        Err(refusal) => VerifiedShadowResult::Refused(Box::new(refusal)),
    };
    VerifiedShadowReport::new(identity, result)
}

fn verified_rows(prepared: &Prepared, verified: VerifiedShadow) -> VerifiedShadowResult {
    match verified.refinement {
        Refinement::Disagreed => VerifiedShadowResult::Disagreed,
        Refinement::CeilingReached => VerifiedShadowResult::Incomplete {
            stage: IncompleteStage::RefinementCeiling,
        },
        Refinement::Exhausted | Refinement::NotExhausted if verified.success_checks == 0 => {
            VerifiedShadowResult::Vacuous
        }
        Refinement::Exhausted if prepared.covered => VerifiedShadowResult::Proved {
            success_checks: verified.success_checks,
        },
        Refinement::Exhausted | Refinement::NotExhausted => VerifiedShadowResult::Tested,
    }
}

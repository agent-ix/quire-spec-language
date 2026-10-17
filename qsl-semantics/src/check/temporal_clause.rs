// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-326: the `TemporalTrace` family's S3 check of one `temporal` clause
//! (ADR-012 §12.2 Check row; ADR-014 §3 TR-3 and TR-4, §5 A-2 and A-3).
//!
//! A clause is checked under the one temporal profile its unit selects
//! (`profile.rs`'s [`TemporalProfile`] catalog). Each temporal operator's
//! interval is built into a validated [`TemporalInterval`]:
//!
//! - under a bounded profile every operator needs a closed interval, so an
//!   operator with none refuses `unsupported_construct`/`expression-form`
//!   at the operator and an `[a,*]` interval refuses the same way at the
//!   interval; the clause then has a horizon, the greatest sum of interval
//!   upper bounds along a path of the formula, computed with checked
//!   arithmetic;
//! - under `quire.temporal.infinite-trace/v1` an operator with no interval is
//!   unbounded (`None`), a closed or `[a,*]` interval is admitted, and the
//!   clause has no horizon;
//! - under every profile `a > b` refuses `ill_typed`/`type-mismatch` at the
//!   interval.
//!
//! The clause's requirement record is `temporal-satisfaction` over its
//! extent: the `over` parameter's, each `on each` parameter's and each
//! capture's type positions, plus, under infinite-trace, the formula itself
//! as one infinite-trace domain no finite bound can stand for.
//!
//! This module checks the interval structure FR-326 states. A `holds`
//! condition, the activation's `when`, a capture's value and the fairness
//! constraints are FR-123's, and the emitted node is FR-337's: until those
//! land, [`not_emitted`] refuses a clause that checks, so it is never
//! compiled into a package that silently lacks it.

use std::collections::BTreeMap;

use qsl_forms::{
    ActivationForm, IntervalForm, IntervalUpper, ParameterForm, TemporalClauseForm,
    TemporalNodeForm, TemporalNodeId, TemporalOperator,
};
use qsl_foundation::bound::{DomainKey, DomainKind, IntervalKey, TemporalInterval, TemporalUpper};
use qsl_foundation::diagnostic::{LimitExceeded, Locus, StageFailure, Staged};
use qsl_foundation::digest::{ByteDigest, WireNodeId};
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::source_map::SourceMap;
use qsl_foundation::{InternalFault, Setting, Span};
use quire_exact::ValueType;
use quire_semantic_value::location::{Location, Origin};

use super::profile::TemporalProfile;
use super::refusal::{CheckCause, CheckRefusal, TemporalCause};
use super::Scope;
use crate::check::Capability;
use crate::family::requirements::{classify_domains, ClassifyFailure};
use crate::family::requirements::{ClaimExtent, Requirements};
use crate::family::{CheckContext, CheckOutcome, FamilyContract};

/// The stage name this module's faults report.
const STAGE: &str = "check.temporal";

/// One header profile the `TemporalTrace` catalog holds (FR-110).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TemporalSelection {
    /// The profile the header selected.
    pub profile: TemporalProfile,
    /// The span of the whole `profile` declaration.
    pub span: Span,
}

/// A `temporal` clause as the assembler hands it to the check: its form and
/// the span of the whole declaration, which a clause-level refusal names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalDeclaration {
    /// The clause as S2 built it.
    pub form: TemporalClauseForm,
    /// The span of the whole declaration.
    pub span: Span,
}

/// What [`TemporalTraceFamily::check`] reads of its unit (ADR-012 §2
/// `Declarations`): the type scope, the unit's temporal profile selections
/// and the source its regions resolve in.
pub struct TemporalDeclarations<'a> {
    pub(crate) scope: &'a Scope,
    pub(crate) selections: &'a [TemporalSelection],
    pub(crate) source: &'a RawSourceRef,
    pub(crate) embedding: Option<&'a SourceMap>,
    /// The type positions a requirement's extent walk may visit.
    pub(crate) position_limit: u64,
}

/// The `TemporalTrace` family's `temporal` clause production (ADR-012
/// §15.2, FR-326).
pub struct TemporalTraceFamily;

/// One temporal operator of a checked formula.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedOperator {
    /// The operator's node in the clause's formula.
    pub node: TemporalNodeId,
    /// Which operator.
    pub operator: TemporalOperator,
    /// Its interval: `Some` under a bounded profile and on an interval
    /// operator under infinite-trace, `None` on an unbounded operator and on
    /// a connective (ADR-014 TR-3).
    pub interval: Option<TemporalInterval>,
}

/// The `temporal-satisfaction` claim of a clause (ADR-014 §4), before the
/// checked nodes it is keyed by exist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalClaim {
    kind: Capability,
    /// The unbounded type positions of the clause's parameters, by the
    /// parameter's index (`over` first) and the child-index path.
    parameter_domains: BTreeMap<(usize, Vec<u32>), DomainKind>,
    /// Whether the formula is itself an infinite-trace domain.
    infinite_trace: bool,
}

impl TemporalClaim {
    /// The capability kind the claim requests.
    pub fn kind(&self) -> Capability {
        self.kind
    }

    /// The claim's requirement record (FR-326 "Requirement record"), keyed
    /// by the clause's checked node and each parameter's, in the order the
    /// clause declares them.
    pub fn requirements(
        &self,
        clause: WireNodeId,
        parameters: &[WireNodeId],
    ) -> Result<Requirements, InternalFault> {
        let mut domains = BTreeMap::new();
        for ((index, path), kind) in &self.parameter_domains {
            let node = parameters
                .get(*index)
                .ok_or_else(|| InternalFault::new(STAGE, "parameter-node-keyed"))?;
            domains.insert(
                DomainKey::Node {
                    node: *node,
                    path: path.clone(),
                },
                *kind,
            );
        }
        if self.infinite_trace {
            domains.insert(
                DomainKey::Node {
                    node: clause,
                    path: Vec::new(),
                },
                DomainKind::InfiniteTrace,
            );
        }
        Ok(Requirements::new(
            self.kind,
            ClaimExtent::from_domains(domains),
        ))
    }
}

/// A temporal clause checked by [`TemporalTraceFamily::check`] (FR-326
/// "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTemporalClause {
    name: String,
    span: Span,
    profile: TemporalProfile,
    profile_ref: DefinitionRef,
    clock_binding: WireNodeId,
    operators: Vec<CheckedOperator>,
    horizon: Option<u64>,
    claim: TemporalClaim,
}

impl CheckedTemporalClause {
    /// The declared clause name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The span of the clause's declaration.
    pub fn span(&self) -> Span {
        self.span
    }

    /// The one profile the clause was checked under.
    pub fn profile(&self) -> TemporalProfile {
        self.profile
    }

    /// Every operator node of the formula (connectives included), in arena
    /// order, each with its interval when it is a temporal operator.
    pub fn operators(&self) -> &[CheckedOperator] {
        &self.operators
    }

    /// The formula's horizon (ADR-014 TR-4): `Some` under a bounded
    /// profile, `None` under infinite-trace.
    pub fn horizon(&self) -> Option<u64> {
        self.horizon
    }

    /// The QSpec FR-255 key of the interval `node` carries, under the
    /// clause's profile and clock binding; `None` when `node` carries no
    /// finite interval.
    pub fn interval_key(&self, node: TemporalNodeId) -> Option<IntervalKey> {
        let operator = self
            .operators
            .iter()
            .find(|operator| operator.node == node)?;
        operator
            .interval?
            .key(self.profile_ref.clone(), self.clock_binding)
    }
}

impl FamilyContract for TemporalTraceFamily {
    type Form = TemporalDeclaration;
    type Checked = CheckedTemporalClause;
    type Cause = CheckRefusal;
    type Declarations<'a> = TemporalDeclarations<'a>;
    type Claim = TemporalClaim;

    fn check<'a>(
        form: &TemporalDeclaration,
        cx: &mut CheckContext<'a, TemporalDeclarations<'a>>,
    ) -> CheckOutcome<CheckedTemporalClause, CheckRefusal> {
        check_clause(form, cx.declarations()).map(Staged::new)
    }

    fn requirements(checked: &CheckedTemporalClause) -> Vec<TemporalClaim> {
        vec![checked.claim.clone()]
    }
}

fn refuse(cause: TemporalCause) -> StageFailure<CheckRefusal> {
    StageFailure::Refused(refusal(cause))
}

fn refusal(cause: TemporalCause) -> CheckRefusal {
    CheckRefusal {
        // A clause has no expression tree for a `Location` path to walk;
        // `region::DeclarationRegions::refusal_region` reads a `Temporal`
        // cause's own span directly, so this `Location` is never resolved.
        location: Location::root(Origin::Expression),
        cause: CheckCause::Temporal(Box::new(cause)),
    }
}

fn fault(invariant: &'static str) -> StageFailure<CheckRefusal> {
    StageFailure::Fault(InternalFault::new(STAGE, invariant))
}

/// Whether `operator` is a temporal operator, which an interval may
/// qualify, rather than a connective.
const fn is_temporal(operator: TemporalOperator) -> bool {
    match operator {
        TemporalOperator::Eventually
        | TemporalOperator::Always
        | TemporalOperator::Once
        | TemporalOperator::Historically
        | TemporalOperator::Until
        | TemporalOperator::Release
        | TemporalOperator::Since
        | TemporalOperator::Triggered => true,
        TemporalOperator::Not
        | TemporalOperator::And
        | TemporalOperator::Or
        | TemporalOperator::Implies => false,
    }
}

/// The clock binding's node id. Recorded exception to keying on the string
/// itself: `IntervalKey::clock_binding` is a `WireNodeId` (QSpec FR-255's
/// structural clock-binding node, fixed by ADR-014 and `qsl_foundation::bound`),
/// a 32-byte id that cannot hold the `clock` string, so the id is the digest of
/// the string's length-prefixed bytes. Equal strings bind equally. FR-233's
/// `model-steps`/`model-time` binding node replaces it.
fn clock_binding(text: &str) -> WireNodeId {
    let mut preimage = Vec::with_capacity(text.len() + 8);
    preimage.extend_from_slice(&u64::try_from(text.len()).unwrap_or(u64::MAX).to_be_bytes());
    preimage.extend_from_slice(text.as_bytes());
    WireNodeId::from_digest(ByteDigest::of(&preimage).as_bytes())
}

/// A temporal operator's interval built from its form: the validated
/// interval, or the refusal that `a > b` gives under every profile.
fn build_interval(interval: &IntervalForm) -> Result<TemporalInterval, TemporalCause> {
    let upper = match interval.upper {
        IntervalUpper::Finite(upper) => TemporalUpper::Finite(upper),
        IntervalUpper::Open => TemporalUpper::Open,
    };
    TemporalInterval::new(interval.lower, upper).map_err(|inverted| {
        TemporalCause::InvertedInterval {
            lower: inverted.lower,
            upper: inverted.upper,
            span: interval.span,
        }
    })
}

/// The parameters whose types the clause's extent ranges over: `over`, the
/// `on each` parameter and each capture, in that order.
fn parameters(form: &TemporalClauseForm) -> Vec<&ParameterForm> {
    let mut parameters = vec![&form.over];
    if let ActivationForm::Each { parameter, .. } = &form.activation {
        parameters.push(parameter);
    }
    parameters.extend(form.captures.iter().map(|capture| &capture.parameter));
    parameters
}

/// The region of `span` as a located stage limit.
fn located(
    limit: LimitExceeded,
    declarations: &TemporalDeclarations<'_>,
    span: Span,
) -> StageFailure<CheckRefusal> {
    let region = super::region::region(declarations.source, declarations.embedding, span);
    StageFailure::Limit(limit.at(region.map(Locus::Region)))
}

fn check_clause(
    declaration: &TemporalDeclaration,
    declarations: &TemporalDeclarations<'_>,
) -> Result<CheckedTemporalClause, StageFailure<CheckRefusal>> {
    let form = &declaration.form;
    let profile = match declarations.selections {
        [only] => only.profile,
        found => {
            return Err(refuse(TemporalCause::ProfileSelection {
                found: found
                    .iter()
                    .map(|selection| selection.profile.identity())
                    .collect(),
                span: declaration.span,
            }))
        }
    };
    let profile_ref =
        DefinitionRef::new(super::profile::CLAUSE_PROFILE_AUTHORITY, profile.identity())
            .map_err(|_| fault("profile-identity-valid"))?;

    let mut types = Vec::new();
    for parameter in parameters(form) {
        let resolved = super::type_form::resolve_form(declarations.scope, &parameter.value_type)
            .map_err(|error| {
                refuse(TemporalCause::ParameterType {
                    parameter: parameter.name.name.clone(),
                    fault: error.fault,
                    span: error.span,
                })
            })?;
        types.push(resolved);
    }

    let bounded = profile.is_bounded();
    let mut operators = Vec::new();
    let mut refusals: Vec<TemporalCause> = Vec::new();
    for (id, node) in form.formula.iter() {
        let TemporalNodeForm::Operator(operator) = node else {
            continue;
        };
        let interval = match (&operator.interval, is_temporal(operator.operator)) {
            (Some(_), false) => return Err(fault("connective-carries-no-interval")),
            (None, true) if bounded => {
                refusals.push(TemporalCause::MissingInterval {
                    span: operator.operator_span,
                });
                None
            }
            (None, _) => None,
            (Some(written), true) => match build_interval(written) {
                Err(cause) => {
                    refusals.push(cause);
                    None
                }
                Ok(interval) => {
                    if bounded && interval.upper() == TemporalUpper::Open {
                        refusals.push(TemporalCause::OpenInterval { span: written.span });
                    }
                    Some(interval)
                }
            },
        };
        operators.push(CheckedOperator {
            node: id,
            operator: operator.operator,
            interval,
        });
    }
    if let Some(first) = refusals.into_iter().min_by_key(|cause| cause.span().start) {
        return Err(refuse(first));
    }

    let horizon = if bounded {
        Some(formula_horizon(declaration, declarations)?)
    } else {
        None
    };

    let roots: Vec<&ValueType> = types.iter().collect();
    let parameter_domains = classify_domains(
        &roots,
        declarations.scope.types(),
        declarations.position_limit,
    )
    .map_err(|failure| match failure {
        ClassifyFailure::Limit(limit) => located(limit, declarations, declaration.span),
        ClassifyFailure::Fault(fault) => StageFailure::Fault(fault),
    })?;

    Ok(CheckedTemporalClause {
        name: form.name.name.clone(),
        span: declaration.span,
        profile,
        profile_ref,
        clock_binding: clock_binding(&form.clock.text),
        operators,
        horizon,
        claim: TemporalClaim {
            kind: Capability::TemporalSatisfaction,
            parameter_domains,
            infinite_trace: !bounded,
        },
    })
}

/// A bounded formula's horizon (ADR-014 TR-4): the greatest sum of interval
/// upper bounds along a path from the root. Each node's operands follow it
/// in the arena, so one reverse pass computes every reach from its
/// operands'. A sum past `u64` is a work-budget stage limit located at the
/// operator whose interval overflowed.
fn formula_horizon(
    declaration: &TemporalDeclaration,
    declarations: &TemporalDeclarations<'_>,
) -> Result<u64, StageFailure<CheckRefusal>> {
    let formula = &declaration.form.formula;
    let nodes = formula.nodes();
    let mut reach = vec![0_u64; nodes.len()];
    for (index, node) in nodes.iter().enumerate().rev() {
        let TemporalNodeForm::Operator(operator) = node else {
            continue;
        };
        let mut deepest = 0_u64;
        for operand in &operator.operands {
            if operand.index() <= index {
                return Err(fault("operands-follow-operator"));
            }
            deepest = deepest.max(
                *reach
                    .get(operand.index())
                    .ok_or_else(|| fault("operand-in-arena"))?,
            );
        }
        let own = match operator.interval.map(|interval| interval.upper) {
            Some(IntervalUpper::Finite(upper)) => upper,
            Some(IntervalUpper::Open) | None => 0,
        };
        let Some(total) = deepest.checked_add(own) else {
            let limit = LimitExceeded::new(
                Setting::S3WorkUnits,
                u64::MAX,
                u128::from(deepest) + u128::from(own),
            );
            return Err(located(limit, declarations, operator.operator_span));
        };
        *reach
            .get_mut(index)
            .ok_or_else(|| fault("operator-in-arena"))? = total;
    }
    reach
        .get(formula.root().index())
        .copied()
        .ok_or_else(|| fault("root-in-arena"))
}

/// The refusal for a clause that checked but that no S4 emission writes yet
/// (FR-337): refused rather than compiled and silently missing from the
/// package.
pub fn not_emitted(checked: &CheckedTemporalClause) -> CheckRefusal {
    refusal(TemporalCause::NotEmitted {
        name: checked.name.clone(),
        span: checked.span,
    })
}

#[cfg(test)]
mod tests;

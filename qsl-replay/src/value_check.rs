// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-483 and FR-484: the native evaluation of one assignment for the
//! `value-validity` check arm.
//!
//! The arm decides a claim over a checked package by evaluating the claim's
//! function at assignments of its parameters through the S6a seam
//! (`CheckedPackage::call`), the same evaluator that replays a counterexample
//! (FR-098). This module owns that evaluation, so the arm in `qsl-analyze`
//! and the replay of the arm's counterexample run one code path over one
//! package:
//!
//! * [`ValueTarget::select`] reads the claim's function and occurrence and
//!   refuses a claim that selects no form ([`Selection::NoForm`]);
//! * [`ValueTarget::evaluate`] evaluates one [`Assignment`] and reports a
//!   [`Verdict`];
//! * [`ValueTarget::replay`] re-evaluates a violating assignment and returns
//!   a [`ReproducedViolation`] only when the same violation recurs. That
//!   token is the only evidence [`crate::TerminalRecord::refuted_by_assignment`]
//!   accepts, so an arm cannot report `refuted` for a candidate its native
//!   replay did not reproduce (FR-483 "Counterexample and evidence kind").
//!
//! Two claim forms are evaluated:
//!
//! * `clause`: a `Boolean` function declaration's `declaration` occurrence.
//!   The goal holds at an assignment when the function completes `true`; it
//!   is violated when the function completes `false` or has no value
//!   ([`Violation::undefined`] names the innermost undefined occurrence, which
//!   lies in the callee's body when the undefinedness is inside a function
//!   application, because the evaluator reports the innermost location).
//! * `scalar-operation`: a `value-validity` requirement record the function
//!   owns. The goal is violated at an assignment exactly when the call stops
//!   undefined at the record's own site. The evaluator reports no occurrence
//!   it merely passed through, so an assignment that completes, or stops
//!   elsewhere, shows neither that the occurrence was reached nor that it was
//!   defined there; [`Verdict::NotDecisive`] says so, and the arm never turns
//!   it into a proof.

use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{Capability, NodeTag};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::{CallFailure, FamilyOutcome};
use quire_exact::{
    Cancel, CancelCause, Incomplete, Integer, IntegerInterval, Meter, NodeKey, Origin, Outcome,
    Role, Undefined, Value, ValueType,
};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::location::Location;

use crate::limits::AccountingLimits;
use crate::spine::kernel_undefined_reason;

/// The claim form a target selects (QSpec FR-484 "Forms").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClaimForm {
    /// A Boolean clause: a `Boolean` function declaration's own occurrence.
    Clause,
    /// A scalar operation application of a function body.
    ScalarOperation,
}

impl ClaimForm {
    /// The form's wire spelling (QSpec FR-484 "Forms").
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clause => "clause",
            Self::ScalarOperation => "scalar-operation",
        }
    }
}

/// Why a claim selects no evaluable target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selection {
    /// The claim's node and occurrence select none of the forms, or select a
    /// node the package does not hold: `declined` `invalid-input` (QSpec
    /// FR-483 "The check input").
    NoForm,
    /// The package lacks a node the form requires: `declined`
    /// `incomplete-input`.
    MissingNode,
    /// A broken invariant of the checked package; the item settles `failed`.
    Fault(InternalFault),
}

/// What of a parameter's domain the arm cannot enumerate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedDomain {
    /// A parameter whose declared type is neither `Boolean` nor an integer
    /// type. The arm substitutes the `integer` domain kind only.
    ParameterType,
}

/// The values one parameter ranges over.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParameterDomain {
    /// `Boolean`: `false` and `true`.
    Boolean,
    /// An integer type's inclusive bounds, restricted by the item's bounds.
    /// `None` when the restriction left no value.
    Integers(Option<IntegerInterval>),
    /// The mathematical `Integer`, which a bound must restrict.
    Unbounded,
    /// A type the arm does not enumerate.
    Unsupported(UnsupportedDomain),
}

/// One parameter of the claim's function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetParameter {
    node: NodeKey,
    domain: ParameterDomain,
}

impl TargetParameter {
    /// The parameter's node.
    pub fn node(&self) -> NodeKey {
        self.node.clone()
    }

    /// The parameter's wire node id.
    pub fn wire(&self) -> NodeKey {
        self.node.clone()
    }

    /// The values the parameter ranges over.
    pub fn domain(&self) -> &ParameterDomain {
        &self.domain
    }
}

/// One assignment of a target's parameters, in declared order.
pub type Assignment = Vec<Value>;

/// Where an evaluation was undefined, and why.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndefinedAt {
    /// The `{node, occurrence}` of the innermost undefined subexpression.
    pub locus: OccurrenceKey,
    /// The catalogued reason the native evaluator reports.
    pub reason: &'static str,
}

/// A violation of the goal at one assignment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Violation {
    /// `Some` when the goal had no value there (QSpec FR-484 "Clause and
    /// function application"); `None` when it was defined and false.
    pub undefined: Option<UndefinedAt>,
}

/// Why an evaluation stopped before a verdict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Stop {
    /// A scalar accounting counter was denied.
    Meter(Incomplete),
    /// The caller cancelled, or its deadline ended the run.
    Cancelled(CancelCause),
}

/// What evaluating one assignment showed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Verdict {
    /// The goal holds at the assignment.
    Holds,
    /// The goal is false or undefined at the assignment.
    Violated(Violation),
    /// The evaluation neither violated the claim nor showed it held: a
    /// `scalar-operation` assignment that completed or stopped at another
    /// occurrence.
    NotDecisive,
    /// The evaluation stopped.
    Stopped(Stop),
    /// The evaluator refused: the result is defined but not admitted.
    Refused,
    /// The goal is undefined at the assignment but the package locates the
    /// undefined subexpression at no single `expression` occurrence, so the
    /// violation cannot be named: neither a refutation nor a pass.
    Unlocated,
}

/// A violation the native replay reproduced: the only evidence of a
/// `refuted` arm outcome.
#[derive(Clone, Debug)]
pub struct ReproducedViolation {
    assignment: Assignment,
    violation: Violation,
}

impl ReproducedViolation {
    /// The violating assignment, in the target's parameter order.
    pub fn assignment(&self) -> &[Value] {
        &self.assignment
    }

    /// The violation the replay reproduced.
    pub fn violation(&self) -> &Violation {
        &self.violation
    }
}

/// A claim's function, form and parameter domains over one checked package.
pub struct ValueTarget<'p> {
    package: &'p CheckedPackage,
    name: QualifiedName,
    form: ClaimForm,
    claim: OccurrenceKey,
    parameters: Vec<TargetParameter>,
    limits: AccountingLimits,
}

impl<'p> ValueTarget<'p> {
    /// The target of the claim at `claim` of the function named `function`.
    ///
    /// A `Boolean` function's node at occurrence role `declaration` or
    /// `claim` is a `clause`; an
    /// occurrence a `value-validity` requirement record of the function keys
    /// is a `scalar-operation`. Every other occurrence selects no form.
    ///
    /// # Errors
    ///
    /// [`Selection`] names why the claim selects no evaluable target.
    pub fn select(
        package: &'p CheckedPackage,
        function: &str,
        claim: &OccurrenceKey,
    ) -> Result<Self, Selection> {
        let graph = package.graph();
        let fault = |invariant| Selection::Fault(InternalFault::new("value-check", invariant));
        let callable = graph.callable(function).ok_or(Selection::NoForm)?;
        let name = QualifiedName::unqualified(function)
            .map_err(|_| fault("function-name-is-an-identifier"))?;
        let semantic = graph.semantic_graph();
        let parameters = semantic
            .node(callable.identity.clone())
            .and_then(|node| node.function_parameters())
            .ok_or_else(|| fault("callable-identity-is-a-function-node"))?;
        if parameters.len() != callable.parameters.len() {
            return Err(fault("function-node-parameters-match-signature"));
        }
        // FR-484 "Forms": a clause's occurrence role is `claim` or
        // `declaration`. Both select the one clause a `Boolean` function
        // is, so an item that names the function's node by either role
        // selects it; the claim keeps the role the item named.
        let is_clause_key = [Role::new("declaration"), Role::new("claim")]
            .into_iter()
            .any(|role| {
                *claim == OccurrenceKey::new(callable.identity.clone(), Origin::new(role, 0))
            });
        let form = if is_clause_key {
            if *callable.result != ValueType::Boolean {
                return Err(Selection::NoForm);
            }
            ClaimForm::Clause
        } else {
            let owned = graph.requirements().get(claim).is_some_and(|record| {
                record.requirements().kind() == Capability::ValueValidity
                    && record.owner() == Some(function)
            });
            if !owned {
                return Err(Selection::NoForm);
            }
            semantic
                .node(claim.node().clone())
                .ok_or(Selection::MissingNode)?;
            graph
                .occurrence(claim.node().clone(), claim.origin())
                .ok_or(Selection::MissingNode)?;
            ClaimForm::ScalarOperation
        };
        let parameters = parameters
            .into_iter()
            .zip(callable.parameters)
            .map(|(node, (_, value_type))| TargetParameter {
                node,
                domain: match value_type {
                    ValueType::Boolean => ParameterDomain::Boolean,
                    ValueType::Int(interval) => ParameterDomain::Integers(Some(interval.clone())),
                    ValueType::Integer => ParameterDomain::Unbounded,
                    _ => ParameterDomain::Unsupported(UnsupportedDomain::ParameterType),
                },
            })
            .collect();
        Ok(Self {
            package,
            name,
            form,
            claim: claim.clone(),
            parameters,
            limits: AccountingLimits::default(),
        })
    }

    /// This target with the integer parameter at `node` restricted to
    /// `interval` (a `ProofBound` of kind `integer-range`, QSpec FR-483
    /// "The check input"). A restriction that leaves no value empties the
    /// parameter's domain. A bound on a parameter that is not an integer one
    /// changes nothing.
    #[must_use]
    pub fn bounded(mut self, node: NodeKey, interval: &IntegerInterval) -> Self {
        for parameter in &mut self.parameters {
            if parameter.node != node {
                continue;
            }
            parameter.domain =
                match std::mem::replace(&mut parameter.domain, ParameterDomain::Boolean) {
                    ParameterDomain::Integers(declared) => ParameterDomain::Integers(
                        declared.and_then(|declared| intersect(&declared, interval)),
                    ),
                    ParameterDomain::Unbounded => ParameterDomain::Integers(Some(interval.clone())),
                    other @ (ParameterDomain::Boolean | ParameterDomain::Unsupported(_)) => other,
                };
        }
        self
    }

    /// This target evaluating under `limits`, the call's accounting limits.
    #[must_use]
    pub fn with_limits(mut self, limits: AccountingLimits) -> Self {
        self.limits = limits;
        self
    }

    /// The claim's form.
    pub fn form(&self) -> ClaimForm {
        self.form
    }

    /// The claim's `{node, occurrence}`, the locus of every `where`.
    pub fn claim(&self) -> &OccurrenceKey {
        &self.claim
    }

    /// The function's parameters, in declared order.
    pub fn parameters(&self) -> &[TargetParameter] {
        &self.parameters
    }

    /// Evaluate the claim at `assignment`.
    ///
    /// # Errors
    ///
    /// An [`InternalFault`] when the evaluator breaks one of its invariants
    /// or `assignment` does not fit the target's parameters.
    #[deny(clippy::wildcard_enum_match_arm)]
    pub fn evaluate(
        &self,
        assignment: &[Value],
        cancel: &Cancel,
    ) -> Result<Verdict, InternalFault> {
        let mut meter = Meter::new(self.limits.0).with_cancel(cancel.clone());
        let evaluation = match self.package.call(
            &self.name,
            assignment.to_vec(),
            &ObjectEnvironment::default().with_unresolved_references(),
            &mut meter,
        ) {
            Ok(evaluation) => evaluation,
            Err(CallFailure::Cancelled(cause)) => {
                return Ok(Verdict::Stopped(Stop::Cancelled(cause)));
            }
            Err(CallFailure::Fault(fault)) => return Err(fault),
            Err(_) => {
                return Err(InternalFault::new(
                    "value-check",
                    "assignment-is-admitted-by-the-call",
                ))
            }
        };
        if let Some(cause) = cancel.tripped() {
            return Ok(Verdict::Stopped(Stop::Cancelled(cause)));
        }
        Ok(match (evaluation.outcome, self.form) {
            (
                FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true))),
                ClaimForm::Clause,
            ) => Verdict::Holds,
            (
                FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false))),
                ClaimForm::Clause,
            ) => Verdict::Violated(Violation { undefined: None }),
            (FamilyOutcome::Evaluated(Outcome::Completed(_)), ClaimForm::ScalarOperation) => {
                Verdict::NotDecisive
            }
            (FamilyOutcome::Evaluated(Outcome::Completed(_)), ClaimForm::Clause) => {
                return Err(InternalFault::new(
                    "value-check",
                    "boolean-function-completes-a-boolean",
                ))
            }
            (FamilyOutcome::Evaluated(Outcome::Undefined(reason)), form) => {
                let at = evaluation
                    .location
                    .as_ref()
                    .and_then(|location| self.locus_of(location));
                match (form, at) {
                    (ClaimForm::Clause, Some(locus)) => Verdict::Violated(Violation {
                        undefined: Some(UndefinedAt {
                            locus,
                            reason: undefined_reason(reason),
                        }),
                    }),
                    (ClaimForm::Clause, None) => Verdict::Unlocated,
                    (ClaimForm::ScalarOperation, Some(locus)) if locus == self.claim => {
                        Verdict::Violated(Violation {
                            undefined: Some(UndefinedAt {
                                locus,
                                reason: undefined_reason(reason),
                            }),
                        })
                    }
                    (ClaimForm::ScalarOperation, _) => Verdict::NotDecisive,
                }
            }
            (FamilyOutcome::Evaluated(Outcome::Refused(_)), _) => Verdict::Refused,
            (FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)), _) => {
                Verdict::Stopped(Stop::Meter(incomplete))
            }
            (FamilyOutcome::FamilyEvaluated(_), _) => Verdict::Refused,
            #[cfg(seam_probe_replay_downstream)]
            (FamilyOutcome::__SeamProbe, _) => {
                unreachable!("never constructed outside the probe build")
            }
        })
    }

    /// Whether `assignment` is one of the target's assignments: one value
    /// per parameter, each of the parameter's type and within its domain.
    pub fn admits(&self, assignment: &[Value]) -> bool {
        assignment.len() == self.parameters.len()
            && assignment
                .iter()
                .zip(&self.parameters)
                .all(|(value, parameter)| match (value, &parameter.domain) {
                    (Value::Boolean(_), ParameterDomain::Boolean) => true,
                    (Value::Integer(n), ParameterDomain::Integers(Some(interval))) => {
                        n >= interval.lower() && n <= interval.upper()
                    }
                    _ => false,
                })
    }

    /// Replay `assignment` through the native evaluator: the violation it
    /// reproduces, or `None` when the evaluator does not violate the goal at
    /// it, or violates it differently from `expected` (QSpec FR-483: the
    /// verdict and cause are compared, never the counterexample).
    ///
    /// # Errors
    ///
    /// As [`Self::evaluate`].
    pub fn replay(
        &self,
        assignment: &[Value],
        expected: &Violation,
        cancel: &Cancel,
    ) -> Result<Option<ReproducedViolation>, InternalFault> {
        if !self.admits(assignment) {
            return Ok(None);
        }
        Ok(match self.evaluate(assignment, cancel)? {
            Verdict::Violated(found) if found == *expected => Some(ReproducedViolation {
                assignment: assignment.to_vec(),
                violation: found,
            }),
            Verdict::Violated(_)
            | Verdict::Holds
            | Verdict::NotDecisive
            | Verdict::Stopped(_)
            | Verdict::Refused
            | Verdict::Unlocated => None,
        })
    }

    /// The `{node, occurrence}` the evaluator's `location` stands for: the
    /// one `expression` occurrence of an operation node the package locates
    /// there. A `value` node (a literal, or a parameter, which the package
    /// also records at the body's root) is a leaf, and no evaluation is
    /// undefined at a leaf, so leaves are not candidates. `None` when no
    /// candidate or more than one remains, so a locus is never a guess
    /// (QSpec FR-484: the innermost undefined subexpression).
    fn locus_of(&self, location: &Location) -> Option<OccurrenceKey> {
        let graph = self.package.graph();
        let expression = Role::new("expression");
        let mut found = graph
            .occurrences()
            .filter(|(node, origin, at)| {
                *at == location
                    && *origin.role() == expression
                    && graph
                        .semantic_graph()
                        .node(node.clone())
                        .is_some_and(|n| n.node_tag() != NodeTag::Value)
            })
            .map(|(node, origin, _)| OccurrenceKey::new(node.clone(), origin));
        let only = found.next()?;
        found.next().is_none().then_some(only)
    }
}

fn undefined_reason(reason: Undefined) -> &'static str {
    kernel_undefined_reason(reason)
}

/// `a` restricted to `b`, or `None` when they share no value.
fn intersect(a: &IntegerInterval, b: &IntegerInterval) -> Option<IntegerInterval> {
    let lower = if a.lower() >= b.lower() {
        a.lower()
    } else {
        b.lower()
    };
    let upper = if a.upper() <= b.upper() {
        a.upper()
    } else {
        b.upper()
    };
    IntegerInterval::new(lower.clone(), upper.clone()).ok()
}

/// The integer values of `interval`, in ascending order.
pub fn integers(interval: &IntegerInterval) -> impl Iterator<Item = Integer> {
    let upper = interval.upper().clone();
    let mut next = Some(interval.lower().clone());
    std::iter::from_fn(move || {
        let current = next.take()?;
        if current > upper {
            return None;
        }
        next = Some(current.add(&Integer::one()));
        Some(current)
    })
}

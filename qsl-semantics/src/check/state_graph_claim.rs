// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-165: the `StateGraph` family's S3 check of one state-graph claim
//! (ADR-022 SG-1 to SG-4; ADR-012 §12.2 Check row; ADR-014 §5 A-3).
//!
//! The FR-091 assembler resolves a [`StateGraphClaimForm`]'s profile alias and
//! its `over` object type into a [`StateGraphDeclaration`].
//! [`StateGraphFamily::check`] then:
//!
//! - refuses, at the earliest offending span, a fairness constraint, a
//!   temporal operator in a predicate, and a predicate read of an operation
//!   anchor (`self`), a result or a pre-state, each
//!   `unsupported_construct`/`expression-form` (ADR-022 GM-3, GM-7);
//! - checks each predicate as a Boolean state clause over the `over`
//!   parameter, through the clause checker invariants use (FR-104), so a
//!   predicate that type-checks is exactly an invariant body that does;
//! - records the claim's form ([`StateGraphForm`]) beside its requirement
//!   record ([`StateGraphClaim::requirements`]).
//!
//! The checked claim is retained on the [`CheckedGraph`](super::CheckedGraph);
//! no S4 emission writes a state-graph claim node yet, so the claim has no
//! node identity and its requirement record is keyed by a node the caller
//! supplies.

use std::collections::BTreeMap;

use qsl_forms::{
    ExprNode, Expression, ExpressionForm, StateClauseKind, StateGraphClaimForm,
    StateGraphClaimKind, StateGraphPredicateForm,
};
use qsl_foundation::bound::{DomainKey, DomainKind};
use qsl_foundation::diagnostic::{StageFailure, Staged};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::selection::ProfileSelection;
use qsl_foundation::Span;
use quire_exact::{EffectiveId, ValueType};
use quire_semantic_value::location::{Location, Origin};

use super::ir::{CheckedBody, CheckedNode};
use super::refusal::{CheckCause, CheckRefusal, PredicateRole, StateGraphCause};
use super::state_clause::{check_clause, ClauseDeclarations, ClauseView, PopulationDomain};
use super::Capability;
use crate::family::{CheckContext, CheckOutcome, ClaimExtent, FamilyContract, Requirements};

/// A state-graph claim as the assembler hands it to the check: its form, its
/// `using` alias resolved to one of the unit's profile selections and its
/// `over` type resolved to an object type of an admitted domain package.
#[derive(Clone, Debug)]
pub struct StateGraphDeclaration {
    /// The claim as S2 built it.
    pub form: StateGraphClaimForm,
    /// The profile selection the claim's `using` alias resolved to.
    pub selection: ProfileSelection,
    /// The object type the `over` parameter ranges over; `None` for a claim
    /// with no `over` binding.
    pub context: Option<EffectiveId>,
    /// The population the `over` type is a member of, when exactly one is.
    pub context_population: Option<PopulationDomain>,
}

/// The `StateGraph` family's claim production (ADR-012 §15.2, FR-165).
pub struct StateGraphFamily;

/// A checked Boolean state predicate of a claim (FR-165 "Outputs").
#[derive(Debug)]
pub enum CheckedStatePredicate {
    /// The `from` of an `always possible` claim written without one:
    /// the constant `true` (ADR-022 SG-2).
    ConstantTrue,
    /// A written predicate, checked as an invariant body is.
    Checked(CheckedPredicateBody),
}

/// A written predicate after the clause checker: its Boolean body over the
/// slots the `over` parameter binds.
#[derive(Debug)]
pub struct CheckedPredicateBody {
    parameters: Vec<(String, ValueType)>,
    body: CheckedBody,
}

impl CheckedPredicateBody {
    /// The checked Boolean body.
    pub fn body(&self) -> CheckedNode<'_> {
        self.body.root()
    }

    /// The slots the body is typed over: the `over` parameter first.
    pub fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }
}

impl CheckedStatePredicate {
    /// Whether the predicate is the constant `true` an omitted `from` stands
    /// for.
    pub fn is_constant_true(&self) -> bool {
        matches!(self, Self::ConstantTrue)
    }

    /// The checked body of a written predicate.
    pub fn body(&self) -> Option<CheckedNode<'_>> {
        match self {
            Self::ConstantTrue => None,
            Self::Checked(checked) => Some(checked.body()),
        }
    }
}

/// The form a claim checked to (ADR-022 SG-1 to SG-3).
#[derive(Debug)]
pub enum StateGraphForm {
    /// SG-1 `possible`.
    Possible {
        /// `P`.
        target: CheckedStatePredicate,
    },
    /// SG-2 `always possible`.
    AlwaysPossible {
        /// `Q`: the written `from`, or the constant `true`.
        from: CheckedStatePredicate,
        /// `P`.
        target: CheckedStatePredicate,
    },
    /// SG-3 `unique path`.
    UniquePath {
        /// `X`.
        from: CheckedStatePredicate,
        /// `Y`.
        to: CheckedStatePredicate,
    },
}

/// The `over (x: T)` parameter of a checked claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OverParameter {
    /// The parameter's name.
    pub name: String,
    /// The object type it ranges over.
    pub object: EffectiveId,
}

/// The `temporal-satisfaction` claim of a state-graph claim (ADR-022 SG-4),
/// before the checked node it is keyed by exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGraphClaim {
    kind: Capability,
}

impl StateGraphClaim {
    /// The capability kind the claim requests.
    pub fn kind(&self) -> Capability {
        self.kind
    }

    /// The claim's requirement record (FR-165 "Requirement record"):
    /// `temporal-satisfaction` over an unbounded extent whose one domain names
    /// the claim's checked node. A finite prefix of paths never decides
    /// whether some path exists past it, so the extent is the same unbounded
    /// kind an infinite-trace formula has.
    pub fn requirements(&self, claim: WireNodeId) -> Requirements {
        let domains = BTreeMap::from([(
            DomainKey::Node {
                node: claim,
                path: Vec::new(),
            },
            DomainKind::InfiniteTrace,
        )]);
        Requirements::new(self.kind, ClaimExtent::from_domains(domains))
    }
}

/// A state-graph claim checked by [`StateGraphFamily::check`] (FR-165
/// "Outputs").
#[derive(Debug)]
pub struct CheckedStateGraphClaim {
    name: String,
    span: Span,
    over: Option<OverParameter>,
    form: StateGraphForm,
    claim: StateGraphClaim,
    /// The package's running expression-node total after this claim.
    pub(crate) nodes_used: u64,
}

impl CheckedStateGraphClaim {
    /// The declared claim name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The span of the claim's declaration.
    pub fn span(&self) -> Span {
        self.span
    }

    /// The `over` binding: `Some` when written, `None` when absent. S3 does
    /// not instantiate it; FR-125 does.
    pub fn over(&self) -> Option<&OverParameter> {
        self.over.as_ref()
    }

    /// The form the claim checked to, with its checked predicates.
    pub fn form(&self) -> &StateGraphForm {
        &self.form
    }

    /// The claim's requirement, to key by the claim's checked node.
    pub fn claim(&self) -> &StateGraphClaim {
        &self.claim
    }
}

impl FamilyContract for StateGraphFamily {
    type Form = StateGraphDeclaration;
    type Checked = CheckedStateGraphClaim;
    type Cause = CheckRefusal;
    type Declarations<'a> = ClauseDeclarations<'a>;
    type Claim = StateGraphClaim;

    fn check<'a>(
        form: &StateGraphDeclaration,
        cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
    ) -> CheckOutcome<CheckedStateGraphClaim, CheckRefusal> {
        let declarations = cx.declarations();
        check_claim(form, declarations, cx).map(Staged::new)
    }

    fn requirements(checked: &CheckedStateGraphClaim) -> Vec<StateGraphClaim> {
        vec![checked.claim.clone()]
    }
}

fn refuse(cause: StateGraphCause) -> StageFailure<CheckRefusal> {
    StageFailure::Refused(refusal(cause))
}

fn refusal(cause: StateGraphCause) -> CheckRefusal {
    CheckRefusal {
        // The cause names its own span; `region::DeclarationRegions::
        // refusal_region` reads it directly, so this `Location` is never
        // resolved.
        location: Location::root(Origin::Expression),
        cause: CheckCause::StateGraph(Box::new(cause)),
    }
}

/// The refusal for a claim that checks but that no S4 emission writes yet
/// (FR-165): refused rather than compiled and silently missing from the
/// package.
pub(crate) fn not_emitted(checked: &CheckedStateGraphClaim) -> CheckRefusal {
    refusal(StateGraphCause::NotEmitted {
        name: checked.name.clone(),
        span: checked.span,
    })
}

/// The predicates of a claim form, each with its role, in source order.
pub(crate) fn predicates(
    kind: &StateGraphClaimKind,
) -> Vec<(PredicateRole, &StateGraphPredicateForm)> {
    match kind {
        StateGraphClaimKind::Possible { target } => vec![(PredicateRole::Target, target)],
        StateGraphClaimKind::AlwaysPossible { from, target } => from
            .iter()
            .map(|from| (PredicateRole::From, from))
            .chain([(PredicateRole::Target, target)])
            .collect(),
        StateGraphClaimKind::UniquePath { from, to } => {
            vec![(PredicateRole::From, from), (PredicateRole::To, to)]
        }
    }
}

/// The spans of every read in `operand` of the caller-side anchors a state
/// predicate may not read: `self`, `result` and `pre(e)`.
fn anchor_reads(operand: &ExpressionForm) -> Vec<Span> {
    let mut found = Vec::new();
    let spans = &operand.spans;
    let mut pending = vec![(operand.expression.root(), Some(spans.root()))];
    while let Some((node, located)) = pending.pop() {
        if matches!(
            node.node(),
            ExprNode::SelfRef | ExprNode::Result | ExprNode::Pre(_)
        ) {
            found.extend(located.and_then(|id| spans.span(id)));
        }
        for (index, child) in node.children().into_iter().enumerate() {
            pending.push((child, located.and_then(|id| spans.child(id, index))));
        }
    }
    found
}

/// The first refusal in source order among the claim's fairness constraints,
/// temporal operators and anchor reads.
fn syntactic_refusal(form: &StateGraphClaimForm) -> Option<StateGraphCause> {
    let fairness = form
        .fairness
        .iter()
        .map(|constraint| StateGraphCause::Fairness {
            span: constraint.span,
        });
    let predicate_causes = predicates(&form.kind)
        .into_iter()
        .flat_map(|(_, predicate)| {
            let operators = predicate.temporal_operators.iter().map(|operator| {
                StateGraphCause::TemporalOperator {
                    span: operator.span,
                }
            });
            let reads = predicate
                .operands
                .iter()
                .flat_map(anchor_reads)
                .map(|span| StateGraphCause::ReadsBeyondState { span });
            operators.chain(reads).collect::<Vec<_>>()
        });
    fairness
        .chain(predicate_causes)
        .filter_map(|cause| Some((cause.span()?.start, cause)))
        .min_by_key(|(start, _)| *start)
        .map(|(_, cause)| cause)
}

/// The span of the node `location` reaches in `operand`.
fn span_at(operand: &ExpressionForm, location: &Location) -> Option<Span> {
    let spans = &operand.spans;
    let mut node = spans.root();
    for index in location.path() {
        node = spans.child(node, index)?;
    }
    spans.span(node)
}

/// The view the clause checker reads of a predicate operand under the claim's
/// `over` binding.
fn view_of<'v>(declaration: &'v StateGraphDeclaration, body: &'v Expression) -> ClauseView<'v> {
    ClauseView {
        kind: StateClauseKind::Invariant,
        complete_value: super::state_clause::is_complete_value(&declaration.selection),
        context: declaration.context,
        receiver: declaration
            .form
            .over
            .as_ref()
            .zip(declaration.context)
            .map(|(over, context)| (over.name.name.as_str(), ValueType::Reference(context))),
        operation: None,
        context_population: declaration.context_population,
        frame_population: None,
        body,
    }
}

/// The first name of a predicate's operands that resolves to nothing: a name
/// refusal outranks a temporal operator or fairness constraint, so a claim
/// that holds both reports the name. Only a missing name counts, since an
/// operand read alone may lack the context its operator supplies; a name a
/// `let` of the predicate binds, and `self` (refused as an anchor read),
/// resolve.
fn unresolved_name<'a>(
    declaration: &StateGraphDeclaration,
    declarations: &ClauseDeclarations<'a>,
    cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
) -> Option<StateGraphCause> {
    let mut earliest: Option<(usize, StateGraphCause)> = None;
    for (role, predicate) in predicates(&declaration.form.kind) {
        for operand in &predicate.operands {
            let view = view_of(declaration, &operand.expression);
            let Err(inner) =
                check_clause(&view, declarations, &mut cx.reborrow(declarations.scope))
            else {
                continue;
            };
            let CheckCause::MissingName(name) = &inner.cause else {
                continue;
            };
            if name == "self" || predicate.bindings.contains(name) {
                continue;
            }
            let span = span_at(operand, &inner.location)
                .or_else(|| operand.spans.span(operand.spans.root()))
                .unwrap_or(declaration.form.span);
            if earliest
                .as_ref()
                .is_none_or(|(start, _)| span.start < *start)
            {
                earliest = Some((
                    span.start,
                    StateGraphCause::Predicate {
                        role,
                        span,
                        cause: Box::new(inner.cause),
                    },
                ));
            }
        }
    }
    earliest.map(|(_, cause)| cause)
}

fn check_predicate<'a>(
    role: PredicateRole,
    predicate: &StateGraphPredicateForm,
    declaration: &StateGraphDeclaration,
    declarations: &ClauseDeclarations<'a>,
    nodes_used: u64,
    cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
) -> Result<(CheckedStatePredicate, u64), StageFailure<CheckRefusal>> {
    // The syntactic refusals ran first: what reaches here is one operand.
    let [operand] = predicate.operands.as_slice() else {
        return Err(StageFailure::Fault(qsl_foundation::InternalFault::new(
            STAGE,
            "predicate-has-one-operand",
        )));
    };
    let body: &Expression = &operand.expression;
    // Each predicate continues from the nodes the claim's earlier ones used,
    // so the node limit applies to the claim's cumulative total.
    let declarations = &ClauseDeclarations {
        scope: declarations.scope,
        signatures: declarations.signatures,
        dispatch_tables: declarations.dispatch_tables,
        models: declarations.models,
        checking_limits: declarations.checking_limits,
        location: declarations.location,
        nodes_used,
        cancel: declarations.cancel,
        staged: declarations.staged,
    };
    let view = view_of(declaration, body);
    let typed = check_clause(&view, declarations, &mut cx.reborrow(declarations.scope)).map_err(
        |inner| {
            let span = span_at(operand, &inner.location)
                .or_else(|| operand.spans.span(operand.spans.root()))
                .unwrap_or(declaration.form.span);
            refuse(StateGraphCause::Predicate {
                role,
                span,
                cause: Box::new(inner.cause),
            })
        },
    )?;
    Ok((
        CheckedStatePredicate::Checked(CheckedPredicateBody {
            parameters: typed.parameters,
            body: typed.body,
        }),
        typed.nodes_used,
    ))
}

/// The stage name this module's faults report.
const STAGE: &str = "check.state-graph";

fn check_claim<'a>(
    declaration: &StateGraphDeclaration,
    declarations: &ClauseDeclarations<'a>,
    cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
) -> Result<CheckedStateGraphClaim, StageFailure<CheckRefusal>> {
    let form = &declaration.form;
    if let Some(cause) = syntactic_refusal(form) {
        let cause = unresolved_name(declaration, declarations, cx).unwrap_or(cause);
        return Err(refuse(cause));
    }
    let mut nodes_used = declarations.nodes_used;
    let mut check = |role, predicate: &StateGraphPredicateForm| {
        let (checked, used) =
            check_predicate(role, predicate, declaration, declarations, nodes_used, cx)?;
        nodes_used = used;
        Ok::<_, StageFailure<CheckRefusal>>(checked)
    };
    let checked_form = match &form.kind {
        StateGraphClaimKind::Possible { target } => StateGraphForm::Possible {
            target: check(PredicateRole::Target, target)?,
        },
        StateGraphClaimKind::AlwaysPossible { from, target } => StateGraphForm::AlwaysPossible {
            from: match from {
                Some(from) => check(PredicateRole::From, from)?,
                None => CheckedStatePredicate::ConstantTrue,
            },
            target: check(PredicateRole::Target, target)?,
        },
        StateGraphClaimKind::UniquePath { from, to } => StateGraphForm::UniquePath {
            from: check(PredicateRole::From, from)?,
            to: check(PredicateRole::To, to)?,
        },
    };
    Ok(CheckedStateGraphClaim {
        name: form.name.name.clone(),
        span: form.span,
        over: form
            .over
            .as_ref()
            .zip(declaration.context)
            .map(|(over, object)| OverParameter {
                name: over.name.name.clone(),
                object,
            }),
        form: checked_form,
        claim: StateGraphClaim {
            kind: Capability::TemporalSatisfaction,
        },
        nodes_used,
    })
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-218 "Choice and repeat decisions": the S3 check of each `choice`'s and
//! `repeat`'s guards, and of a `choice`'s partition.
//!
//! A guard here is a Boolean formula over literals alone: one that reads a
//! name needs the binder scopes and the decision-fact rules of FR-218,
//! which are not implemented, and `protocol_clause::content` refuses it
//! `unsupported_construct`/`not-yet-implemented`, as it refuses a non-empty
//! `visible` list. With no visible fact there is one valuation, so a
//! `choice`'s guards are exhaustive and exclusive exactly when one of them
//! is true: none true is a gap (`missing-arm`), two or more an overlap
//! (`duplicate-arm`). Each guard, invariant and variant is checked through
//! the one clause checker (FR-104) over no names.

use qsl_forms::{ExpressionForm, ProtocolDeclarationForm};
use qsl_foundation::Span;
use quire_semantic_value::location::{Location, Origin};

use super::check::Scope;
use super::ir::CheckedBody;
use super::protocol_clause::{constant_formula, constant_integer, ProtocolNodeId};
use super::protocol_obligation::{type_body, Goal};
use super::refusal::{ArmFault, CheckCause, CheckRefusal, ProtocolAnchorCause};
use super::state_clause::ClauseDeclarations;
use crate::family::CheckContext;

/// One `case` of a checked `choice`.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedCase {
    /// The case label.
    pub name: String,
    /// The checked guard.
    pub guard: CheckedBody,
}

/// One checked `choice` (FR-218 "Outputs": the owner and visible facts of
/// each `choice`).
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedChoice {
    /// The `choice` node.
    pub node: ProtocolNodeId,
    /// The index of the owning role among the protocol's roles.
    pub owner: usize,
    /// The checked `visible` entries, in source order.
    pub visible: Vec<CheckedBody>,
    /// The cases, in source order.
    pub cases: Vec<CheckedCase>,
}

/// A `repeat`'s checked `invariant` and `variant`.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedLoopProof {
    /// The invariant.
    pub invariant: CheckedBody,
    /// The variant; only a termination claim reads it.
    pub variant: CheckedBody,
}

/// One checked `repeat` (FR-218 "Constructs": the maximum, invariant and
/// variant it has).
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedRepeat {
    /// The `repeat` node.
    pub node: ProtocolNodeId,
    /// The index of the owning role among the protocol's roles.
    pub owner: usize,
    /// The checked `visible` entries, in source order.
    pub visible: Vec<CheckedBody>,
    /// The iteration maximum, when one is written.
    pub max: Option<u64>,
    /// The loop proof, when one is written.
    pub loop_proof: Option<CheckedLoopProof>,
    /// The checked `while` guard.
    pub guard: CheckedBody,
}

/// Every checked `choice` and `repeat` of a protocol, each in source order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CheckedDecisions {
    /// The choices.
    pub choices: Vec<CheckedChoice>,
    /// The repeats.
    pub repeats: Vec<CheckedRepeat>,
}

/// What [`check`] found.
pub(crate) enum Decisions {
    /// Every decision checked, with the package's running node total.
    Checked(CheckedDecisions, u64),
    /// Refusals, in source order.
    Refused(Vec<CheckRefusal>),
    /// A part this check does not read: `protocol_clause::content` refuses
    /// it, so there is nothing to add.
    Uncovered,
}

fn refusal(cause: ProtocolAnchorCause) -> CheckRefusal {
    CheckRefusal {
        // A protocol has no expression tree for a `Location` path to walk;
        // `region::DeclarationRegions::refusal_region` reads the cause's own
        // span.
        location: Location::root(Origin::Expression),
        cause: CheckCause::ProtocolAnchor(Box::new(cause)),
    }
}

struct Checker<'c, 'a> {
    declarations: &'c ClauseDeclarations<'a>,
    nodes: u64,
    refusals: Vec<(Span, CheckRefusal)>,
    uncovered: bool,
}

impl Checker<'_, '_> {
    /// `form` checked over no names against `goal`; `None` when it is not a
    /// form this check reads, or when it refuses.
    fn expression(
        &mut self,
        form: &ExpressionForm,
        goal: Goal<'_>,
        cx: &mut CheckContext<'_, Scope>,
    ) -> Option<CheckedBody> {
        let mut nodes = self.nodes;
        let typed = type_body(form, &[], goal, self.declarations, &mut nodes, cx);
        self.nodes = nodes;
        match typed {
            Ok(body) => Some(body),
            Err(refused) => {
                let span = form.spans.at(&[]).unwrap_or(Span { start: 0, end: 0 });
                self.refusals.push((span, refused));
                None
            }
        }
    }

    /// A guard: a constant formula, checked as a Boolean.
    fn guard(
        &mut self,
        form: &ExpressionForm,
        cx: &mut CheckContext<'_, Scope>,
    ) -> Option<CheckedBody> {
        if constant_formula(form).is_none() {
            self.uncovered = true;
            return None;
        }
        self.expression(form, Goal::Boolean, cx)
    }
}

/// The index of the role `name` among `protocol`'s roles.
fn role_index(protocol: &ProtocolDeclarationForm, name: &str) -> Option<usize> {
    protocol.roles.iter().position(|role| role.name.name == name)
}

/// Checks every `choice` and `repeat` of `protocol` (FR-218 "Choice and
/// repeat decisions").
pub(crate) fn check(
    protocol: &ProtocolDeclarationForm,
    declarations: &ClauseDeclarations<'_>,
    cx: &mut CheckContext<'_, ClauseDeclarations<'_>>,
) -> Decisions {
    let mut checker = Checker {
        declarations,
        nodes: declarations.nodes_used,
        refusals: Vec::new(),
        uncovered: false,
    };
    let mut cx = cx.reborrow(declarations.scope);
    let mut decisions = CheckedDecisions::default();

    for choice in &protocol.choices {
        if !choice.visible.is_empty() {
            checker.uncovered = true;
            continue;
        }
        let mut cases = Vec::with_capacity(choice.cases.len());
        let mut values = Vec::with_capacity(choice.cases.len());
        for case in &choice.cases {
            values.push(constant_formula(&case.guard));
            if let Some(guard) = checker.guard(&case.guard, &mut cx) {
                cases.push(CheckedCase {
                    name: case.name.name.clone(),
                    guard,
                });
            }
        }
        let Some(values) = values.into_iter().collect::<Option<Vec<bool>>>() else {
            continue;
        };
        let covering: Vec<&str> = choice
            .cases
            .iter()
            .zip(&values)
            .filter(|(_, value)| **value)
            .map(|(case, _)| case.name.name.as_str())
            .collect();
        let (failed, involved) = match covering.len() {
            0 => (
                vec![ArmFault::MissingArm],
                choice
                    .cases
                    .iter()
                    .map(|case| case.name.name.clone())
                    .collect(),
            ),
            1 => (Vec::new(), Vec::new()),
            _ => (
                vec![ArmFault::DuplicateArm],
                covering.iter().map(|name| (*name).to_owned()).collect(),
            ),
        };
        if !failed.is_empty() {
            let span = choice.name.span;
            checker.refusals.push((
                span,
                refusal(ProtocolAnchorCause::Exhaustiveness {
                    choice: choice.name.name.clone(),
                    cases: involved,
                    failed,
                    span,
                }),
            ));
            continue;
        }
        let Some(owner) = role_index(protocol, &choice.owner.name) else {
            // `protocol_clause::content` refuses the owner.
            checker.uncovered = true;
            continue;
        };
        if cases.len() != choice.cases.len() {
            continue;
        }
        decisions.choices.push(CheckedChoice {
            node: ProtocolNodeId(choice.declaration),
            owner,
            visible: Vec::new(),
            cases,
        });
    }

    for repeat in &protocol.repeats {
        if !repeat.visible.is_empty() {
            checker.uncovered = true;
            continue;
        }
        let guard = checker.guard(&repeat.guard, &mut cx);
        let loop_proof = match &repeat.loop_proof {
            None => Some(None),
            Some(proof) => {
                let invariant = checker.guard(&proof.invariant, &mut cx);
                let variant = if constant_integer(&proof.variant) {
                    checker.expression(&proof.variant, Goal::Infer, &mut cx)
                } else {
                    checker.uncovered = true;
                    None
                };
                match (invariant, variant) {
                    (Some(invariant), Some(variant)) => {
                        Some(Some(CheckedLoopProof { invariant, variant }))
                    }
                    _ => None,
                }
            }
        };
        let Some(owner) = role_index(protocol, &repeat.owner.name) else {
            // `protocol_clause::content` refuses the owner.
            checker.uncovered = true;
            continue;
        };
        let (Some(guard), Some(loop_proof)) = (guard, loop_proof) else {
            continue;
        };
        decisions.repeats.push(CheckedRepeat {
            node: ProtocolNodeId(repeat.declaration),
            owner,
            visible: Vec::new(),
            max: repeat.max.as_ref().map(|max| max.value),
            loop_proof,
            guard,
        });
    }

    if !checker.refusals.is_empty() {
        checker.refusals.sort_by_key(|(span, _)| span.start);
        return Decisions::Refused(
            checker
                .refusals
                .into_iter()
                .map(|(_, refusal)| refusal)
                .collect(),
        );
    }
    if checker.uncovered {
        return Decisions::Uncovered;
    }
    Decisions::Checked(decisions, checker.nodes)
}

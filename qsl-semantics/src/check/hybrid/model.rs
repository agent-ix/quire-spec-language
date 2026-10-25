// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-447 "Modes and transitions", "Initial states", "Continuous variables
//! and flows", "The clock line" and "Invariants, guards, resets and
//! expression forms": the S3 check of one `hybrid` declaration.

use std::collections::BTreeMap;

use qsl_forms::{BinaryOperator, ExprId, ExprNode, ExpressionForm, HybridForm, SpanId};
use qsl_foundation::selection::ProfileSelection;
use qsl_foundation::Span;
use quire_exact::{Identifier, Integer};
use quire_semantic_value::location::{Location, Origin};

use super::callee::Callees;
use super::cause::MalformedReason;
use super::term::{lower, lower_at, Binder, Env, Kind, Lowered, Meter, Stop};
use super::{
    CheckedHybridModel, CheckedMode, CheckedTransition, CompareOp, ContinuousVariable, HybridCause,
    HybridModelIndex, HybridTerm, InitialEntry, ModeIndex, ResetConstraint, VariableClass,
    VariableIndex,
};
use crate::check::refusal::{CheckCause, CheckRefusal, CheckingStage, StageLimitCause};

/// A `hybrid` declaration as the assembler hands it to the check: its form,
/// and the profile selection its `using` alias resolved to.
#[derive(Clone, Debug)]
pub struct HybridDeclaration {
    /// The declaration as S2 built it.
    pub form: HybridForm,
    /// The unit's profile selection the `using` alias names.
    pub selection: ProfileSelection,
}

/// What the check of a unit's hybrid declarations produced.
pub(in crate::check) struct HybridChecked {
    /// The models that checked clean, in source order.
    pub(in crate::check) models: Vec<CheckedHybridModel>,
    /// Each clean model's position by declared name, for the claims over it.
    pub(in crate::check) index: BTreeMap<String, HybridModelIndex>,
    /// Every refusal, in source order.
    pub(in crate::check) refusals: Vec<CheckRefusal>,
}

/// Check each declaration. A declaration that refuses produces no model, so
/// `models` index a clean prefix only when no declaration refused; a caller
/// that finds any refusal refuses the package.
pub(in crate::check) fn check_models(
    declarations: &[HybridDeclaration],
    callees: &Callees<'_>,
    node_limit: u64,
    nodes_used: &mut u64,
) -> HybridChecked {
    let mut checked = HybridChecked {
        models: Vec::new(),
        index: BTreeMap::new(),
        refusals: Vec::new(),
    };
    let mut names: BTreeMap<&str, Span> = BTreeMap::new();
    let mut meter = Meter {
        used: nodes_used,
        limit: node_limit,
    };
    for declaration in declarations {
        let form = &declaration.form;
        let mut stops = Vec::new();
        match names.get(form.name.name.as_str()) {
            Some(first) => stops.push(Stop::Cause(HybridCause::DuplicateHybrid {
                name: form.name.name.clone(),
                first: *first,
                span: form.name.span,
            })),
            None => {
                names.insert(form.name.name.as_str(), form.name.span);
            }
        }
        let model = check_one(form, callees, &mut meter, &mut stops);
        let limit_reached = stops.iter().any(|stop| matches!(stop, Stop::Limit { .. }));
        let clean = stops.is_empty();
        checked.refusals.extend(stops.into_iter().map(refusal));
        if let (Some(model), true) = (model, clean) {
            checked.index.insert(
                form.name.name.clone(),
                HybridModelIndex::new(checked.models.len()),
            );
            checked.models.push(model);
        }
        if limit_reached {
            break;
        }
    }
    checked
}

/// The refusal `stop` stands for.
pub(super) fn into_refusal(stop: Stop) -> CheckRefusal {
    refusal(stop)
}

fn refusal(stop: Stop) -> CheckRefusal {
    let cause = match stop {
        Stop::Cause(cause) => CheckCause::Hybrid(Box::new(cause)),
        Stop::Limit {
            kind,
            limit,
            actual,
        } => CheckCause::ResourceExhausted(Box::new(StageLimitCause {
            stage: CheckingStage::Typing,
            kind,
            limit,
            actual,
            region: None,
        })),
    };
    CheckRefusal {
        location: Location::root(Origin::Expression),
        cause,
    }
}

fn identifier(name: &str, span: Span, stops: &mut Vec<Stop>) -> Option<Identifier> {
    match Identifier::new(name) {
        Ok(identifier) => Some(identifier),
        Err(_) => {
            stops.push(Stop::Cause(HybridCause::Malformed {
                reason: MalformedReason::InvalidName,
                span,
            }));
            None
        }
    }
}

/// The span of an expression form's root.
fn root_span(form: &ExpressionForm) -> Span {
    form.spans
        .span(form.spans.root())
        .unwrap_or(Span { start: 0, end: 0 })
}

/// `form` lowered to a Boolean term, else refused `non-boolean-root` at its
/// root.
fn boolean(
    form: &ExpressionForm,
    env: &Env<'_>,
    meter: &mut Meter<'_>,
    stops: &mut Vec<Stop>,
) -> Option<HybridTerm> {
    match lower(form, env, meter) {
        Ok(lowered) if lowered.kind == Kind::Boolean => Some(lowered.term),
        Ok(_) => {
            stops.push(Stop::Cause(HybridCause::NonBoolean {
                span: root_span(form),
            }));
            None
        }
        Err(stop) => {
            stops.push(stop);
            None
        }
    }
}

fn check_one(
    form: &HybridForm,
    callees: &Callees<'_>,
    meter: &mut Meter<'_>,
    stops: &mut Vec<Stop>,
) -> Option<CheckedHybridModel> {
    let name = identifier(&form.name.name, form.name.span, stops);

    // Modes: each name once, resolvable by transitions and initial entries.
    let mut mode_names: BTreeMap<&str, (ModeIndex, Span)> = BTreeMap::new();
    let mut mode_identifiers = Vec::with_capacity(form.modes.len());
    for (position, mode) in form.modes.iter().enumerate() {
        match mode_names.get(mode.name.name.as_str()) {
            Some((_, first)) => stops.push(Stop::Cause(HybridCause::DuplicateMode {
                name: mode.name.name.clone(),
                first: *first,
                span: mode.name.span,
            })),
            None => {
                mode_names.insert(
                    mode.name.name.as_str(),
                    (ModeIndex::new(position), mode.name.span),
                );
            }
        }
        mode_identifiers.push(identifier(&mode.name.name, mode.name.span, stops));
    }

    // Continuous variables: the left sides of the equations, in order of
    // first appearance across the modes.
    let mut variables: Vec<ContinuousVariable> = Vec::new();
    let mut by_name: BTreeMap<String, VariableIndex> = BTreeMap::new();
    for mode in &form.modes {
        for equation in &mode.equations {
            let written = equation.variable.name.as_str();
            if equation.variable.is_qualified() {
                stops.push(Stop::Cause(HybridCause::Malformed {
                    reason: MalformedReason::QualifiedEquationName,
                    span: equation.variable.span,
                }));
                continue;
            }
            if by_name.contains_key(written) {
                continue;
            }
            if let Some(variable) = identifier(written, equation.variable.span, stops) {
                by_name.insert(written.to_owned(), VariableIndex::new(variables.len()));
                variables.push(ContinuousVariable {
                    name: variable,
                    class: VariableClass::Hybrid,
                });
            }
        }
    }
    let env = Env {
        variables: &by_name,
        reset: false,
        claim: None,
        callees,
    };
    let reset_env = Env {
        variables: &by_name,
        reset: true,
        claim: None,
        callees,
    };

    // Each mode: invariant, exactly one rate per variable, transitions.
    let mut every_rate_is_one = vec![true; variables.len()];
    let mut modes = Vec::with_capacity(form.modes.len());
    let mut complete = true;
    for (position, mode) in form.modes.iter().enumerate() {
        let invariant = boolean(&mode.invariant, &env, meter, stops);
        let mut equation_of: BTreeMap<&str, Span> = BTreeMap::new();
        let mut rates: Vec<Option<HybridTerm>> = vec![None; variables.len()];
        for equation in &mode.equations {
            let written = equation.variable.name.as_str();
            let Some(index) = by_name.get(written).copied() else {
                continue;
            };
            if let Some(first) = equation_of.get(written) {
                complete = false;
                stops.push(Stop::Cause(HybridCause::DuplicateEquation {
                    variable: written.to_owned(),
                    first: *first,
                    span: equation.variable.span,
                }));
                continue;
            }
            equation_of.insert(written, equation.variable.span);
            if !is_literal_one(&equation.rate) {
                if let Some(one) = every_rate_is_one.get_mut(index.get()) {
                    *one = false;
                }
            }
            match lower(&equation.rate, &env, meter) {
                Ok(Lowered {
                    term,
                    kind: Kind::Real,
                    ..
                }) => {
                    if let Some(slot) = rates.get_mut(index.get()) {
                        *slot = Some(term);
                    }
                }
                Ok(_) => {
                    complete = false;
                    stops.push(Stop::Cause(HybridCause::TypeMismatch {
                        span: root_span(&equation.rate),
                    }));
                }
                Err(stop) => {
                    complete = false;
                    stops.push(stop);
                }
            }
        }
        for variable in variables.iter() {
            if !equation_of.contains_key(variable.name.as_str()) {
                complete = false;
                stops.push(Stop::Cause(HybridCause::MissingEquation {
                    mode: mode.name.name.clone(),
                    variable: variable.name.as_str().to_owned(),
                    span: mode.name.span,
                }));
            }
        }

        let mut transitions = Vec::with_capacity(mode.transitions.len());
        for transition in &mode.transitions {
            let target = match mode_names.get(transition.target.name.as_str()) {
                Some((target, _)) => Some(*target),
                None => {
                    stops.push(Stop::Cause(HybridCause::MissingMode {
                        name: transition.target.name.clone(),
                        span: transition.target.span,
                    }));
                    None
                }
            };
            let guard = boolean(&transition.guard, &env, meter, stops);
            let resets = reset(&transition.reset, &by_name, &reset_env, meter, stops);
            if let (Some(target), Some(guard), Some(resets)) = (target, guard, resets) {
                transitions.push(CheckedTransition {
                    target,
                    guard,
                    resets,
                });
            }
        }
        let name = mode_identifiers.get(position).cloned().flatten();
        let rates: Option<Vec<HybridTerm>> = rates.into_iter().collect();
        if let (Some(name), Some(invariant), Some(rates)) = (name, invariant, rates) {
            modes.push(CheckedMode {
                name,
                invariant,
                rates,
                transitions,
            });
        }
    }

    // Initial states.
    let mut initial = Vec::new();
    let mut initial_ok = true;
    match &form.initial {
        None => initial.push(InitialEntry {
            mode: ModeIndex::new(0),
            condition: HybridTerm::Boolean(true),
        }),
        Some(clause) => {
            for entry in &clause.entries {
                let mode = mode_names
                    .get(entry.mode.name.as_str())
                    .map(|(mode, _)| *mode);
                if mode.is_none() {
                    initial_ok = false;
                    stops.push(Stop::Cause(HybridCause::MissingMode {
                        name: entry.mode.name.clone(),
                        span: entry.mode.span,
                    }));
                }
                let condition = boolean(&entry.condition, &env, meter, stops);
                match (mode, condition) {
                    (Some(mode), Some(condition)) => initial.push(InitialEntry { mode, condition }),
                    _ => initial_ok = false,
                }
            }
        }
    }

    // The clock line: a variable is a clock when its rate is the numeric
    // literal `1` in every mode (ADR-026 HY-1).
    if complete {
        for (variable, one) in variables.iter_mut().zip(&every_rate_is_one) {
            if *one {
                variable.class = VariableClass::Clock;
            }
        }
        if !variables.is_empty()
            && variables
                .iter()
                .all(|variable| variable.class == VariableClass::Clock)
        {
            stops.push(Stop::Cause(HybridCause::RateOneHybrid {
                name: form.name.name.clone(),
                span: form.name.span,
            }));
        }
    }

    let all_modes = modes.len() == form.modes.len();
    match name {
        Some(name) if complete && initial_ok && all_modes => Some(CheckedHybridModel {
            name,
            variables,
            initial,
            modes,
        }),
        _ => None,
    }
}

/// Whether the expression is the numeric literal `1`, as written: `2 - 1`
/// and `rational(2, 2)` are not (FR-447 "The clock line").
fn is_literal_one(rate: &ExpressionForm) -> bool {
    matches!(rate.expression.root_node(), ExprNode::Integer(value) if *value == Integer::one())
}

/// One reset conjunct's standing, for the duplicate rule: the variable's
/// earliest conjunct and its `=` conjunct, if any.
struct Conjuncts {
    first: Span,
    equals: Option<Span>,
}

enum Walk {
    Visit(ExprId, SpanId, bool),
    Unbind,
}

/// A transition's reset (FR-447 "Invariants, guards, resets and expression
/// forms"): `true`, or a conjunction, possibly under `let` binders, of
/// conjuncts `x ~ e`. Refusals go to `stops`; the result is `None` when any
/// was recorded.
fn reset(
    form: &ExpressionForm,
    variables: &BTreeMap<String, VariableIndex>,
    env: &Env<'_>,
    meter: &mut Meter<'_>,
    stops: &mut Vec<Stop>,
) -> Option<Vec<ResetConstraint>> {
    let spans = &form.spans;
    let whole = root_span(form);
    let start = stops.len();
    let mut constraints: Vec<ResetConstraint> = Vec::new();
    let mut seen: BTreeMap<VariableIndex, Conjuncts> = BTreeMap::new();
    let mut bound: Vec<Binder> = Vec::new();
    let mut walk = vec![Walk::Visit(form.expression.root_id(), spans.root(), true)];
    let malformed = |reason, span| Stop::Cause(HybridCause::Malformed { reason, span });
    while let Some(step) = walk.pop() {
        let (id, at, top) = match step {
            Walk::Unbind => {
                bound.pop();
                continue;
            }
            Walk::Visit(id, at, top) => (id, at, top),
        };
        let span = spans.span(at).unwrap_or(whole);
        let child = |index: usize| spans.child(at, index).unwrap_or(at);
        match form.expression.at(id).node() {
            // `reset { true }` keeps every variable; `true` is no conjunct.
            ExprNode::Boolean(true) if top => {}
            ExprNode::Let { name, value, body } => {
                match lower_at(form, (*value, child(0)), &bound, env, meter) {
                    Ok(lowered) => {
                        bound.push((name.clone(), lowered));
                        walk.push(Walk::Unbind);
                        walk.push(Walk::Visit(*body, child(1), top));
                    }
                    Err(stop) => stops.push(stop),
                }
            }
            ExprNode::Binary {
                operator: BinaryOperator::And,
                left,
                right,
            } => {
                walk.push(Walk::Visit(*right, child(1), false));
                walk.push(Walk::Visit(*left, child(0), false));
            }
            ExprNode::Binary {
                operator,
                left,
                right,
            } => {
                let relation = match operator {
                    BinaryOperator::Equal => CompareOp::Eq,
                    BinaryOperator::NotEqual => CompareOp::Ne,
                    BinaryOperator::Less => CompareOp::Lt,
                    BinaryOperator::LessOrEqual => CompareOp::Le,
                    BinaryOperator::GreaterOrEqual => CompareOp::Ge,
                    BinaryOperator::Greater => CompareOp::Gt,
                    _ => {
                        stops.push(malformed(MalformedReason::ResetNotConjunction, span));
                        continue;
                    }
                };
                let variable = match form.expression.at(*left).node() {
                    ExprNode::Name(name)
                        if !bound.iter().any(|(binder, _)| binder == &name.name) =>
                    {
                        variables
                            .get(&name.name)
                            .copied()
                            .map(|index| (name.name.as_str(), index))
                    }
                    _ => None,
                };
                let Some((name, index)) = variable else {
                    stops.push(malformed(MalformedReason::ResetConjunctNotVariable, span));
                    continue;
                };
                let lowered = lower_at(form, (*right, child(1)), &bound, env, meter);
                let value = match lowered {
                    Ok(Lowered {
                        term,
                        kind: Kind::Real,
                        ..
                    }) => term,
                    Ok(_) => {
                        stops.push(Stop::Cause(HybridCause::TypeMismatch {
                            span: spans.span(child(1)).unwrap_or(span),
                        }));
                        continue;
                    }
                    Err(stop) => {
                        stops.push(stop);
                        continue;
                    }
                };
                let conflict = match seen.get_mut(&index) {
                    None => {
                        seen.insert(
                            index,
                            Conjuncts {
                                first: span,
                                equals: (relation == CompareOp::Eq).then_some(span),
                            },
                        );
                        None
                    }
                    Some(entry) => {
                        let first = entry
                            .equals
                            .or((relation == CompareOp::Eq).then_some(entry.first));
                        if relation == CompareOp::Eq && entry.equals.is_none() {
                            entry.equals = Some(span);
                        }
                        first
                    }
                };
                if let Some(first) = conflict {
                    stops.push(Stop::Cause(HybridCause::DuplicateReset {
                        variable: name.to_owned(),
                        first,
                        span,
                    }));
                    continue;
                }
                constraints.push(ResetConstraint {
                    variable: index,
                    relation,
                    value,
                });
            }
            _ => stops.push(malformed(MalformedReason::ResetNotConjunction, span)),
        }
    }
    (stops.len() == start).then_some(constraints)
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Clock-reading declarations": a function or predicate of the
//! unit's own source whose parameter or result is clock-bearing.
//!
//! Such a declaration has no checked node. [`check_declaration`] checks its
//! body by "Clock constraints and guards": a clock parameter is read only as
//! the clock operand of an atomic clock constraint whose bound is a time
//! quantity literal or a quantity parameter, as an argument for a clock
//! parameter of another clock-reading declaration, or as a branch of a
//! place expression; and a declaration whose result is `Clock` has a place
//! expression as its body.
//!
//! [`Expansion`] expands a call where a precondition reads it. The
//! arguments are read at the call site: a clock argument as the places it
//! can denote, each with the data condition that selects it, and a data
//! argument as the call-site node it is. The body is then read with its
//! parameters bound to those arguments, into the disjunctive form the
//! precondition's guard takes: each data literal is a call-site node, so
//! the recorded guard holds atomic clock constraints over the receiver's
//! clocks only.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{BinaryOperator, ExprId, ExprNode, Expression, NameForm};
use qsl_foundation::Span;
use quire_exact::Rational;

use super::{timed, ClockConstraint, ClockId, ClockOp, Clocks, Dnf, Located, PartBudget};
use crate::check::assemble::{ClockReadingDeclaration, ClockSlot};
use crate::check::refusal::{CheckCause, TemporalCause, TimedConstruct};

/// The places a place expression can denote, each with the call-site data
/// literals that select it.
pub(super) type Places = Vec<(Vec<(ExprId, bool)>, ClockId)>;

/// The unit's clock-reading declarations, by name.
pub(crate) struct Declarations<'r> {
    by_name: BTreeMap<&'r str, &'r ClockReadingDeclaration>,
    /// The name of every clock field of an admitted object type, which a
    /// body reads as `self.x`.
    clock_fields: BTreeSet<String>,
}

impl<'r> Declarations<'r> {
    pub(crate) fn new(
        declarations: &'r [ClockReadingDeclaration],
        clock_fields: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            by_name: declarations
                .iter()
                .map(|declaration| (declaration.function.name.as_str(), declaration))
                .collect(),
            clock_fields: clock_fields.into_iter().collect(),
        }
    }

    /// The declaration a written call name names: an unqualified name of
    /// the unit's own source.
    pub(super) fn named(&self, name: &NameForm) -> Option<&'r ClockReadingDeclaration> {
        if name.segments.len() > 1 {
            return None;
        }
        self.by_name.get(name.name.as_str()).copied()
    }

    /// Whether node `id` of `expression` is a call of a clock-reading
    /// declaration.
    pub(super) fn is_call(&self, expression: &Expression, id: ExprId) -> bool {
        matches!(
            expression.get(id).map(|node| node.node()),
            Some(ExprNode::Call { name, .. }) if self.named(name).is_some()
        )
    }
}

/// The comparison a binary operator spells, when it is one.
fn comparison(operator: BinaryOperator) -> Option<ClockOp> {
    ClockOp::of(operator)
}

/// The slot of `parameter` in `declaration`, when it names one.
fn parameter_slot(declaration: &ClockReadingDeclaration, name: &str) -> Option<ClockSlot> {
    declaration
        .function
        .parameters
        .iter()
        .position(|(parameter, _)| parameter == name)
        .and_then(|index| declaration.parameters.get(index).copied())
}

/// The parameter a body node names, when it is a bare name of one.
fn parameter<'d>(
    declaration: &'d ClockReadingDeclaration,
    node: &ExprNode,
) -> Option<(&'d str, ClockSlot)> {
    let ExprNode::Name(name) = node else {
        return None;
    };
    if name.segments.len() > 1 {
        return None;
    }
    let slot = parameter_slot(declaration, &name.name)?;
    let (written, _) = declaration
        .function
        .parameters
        .iter()
        .find(|(parameter, _)| *parameter == name.name)?;
    Some((written.as_str(), slot))
}

/// One declaration's body and the span of each of its nodes.
struct Body<'d> {
    declaration: &'d ClockReadingDeclaration,
    spans: Vec<Span>,
}

impl<'d> Body<'d> {
    fn of(declaration: &'d ClockReadingDeclaration) -> Self {
        let function = &declaration.function;
        let spans = function
            .spans()
            .and_then(|spans| spans.body.of_nodes(&function.body))
            .unwrap_or_default();
        Self { declaration, spans }
    }

    fn expression(&self) -> &'d Expression {
        &self.declaration.function.body
    }

    fn node(&self, id: ExprId) -> Option<&'d ExprNode> {
        self.expression().get(id).map(|node| node.node())
    }

    fn span(&self, id: ExprId) -> Span {
        self.spans
            .get(id.index())
            .copied()
            .or_else(|| {
                self.declaration
                    .function
                    .spans()
                    .map(|spans| spans.declaration)
            })
            .unwrap_or(Span { start: 0, end: 0 })
    }

    /// Whether node `id` is a place expression of this body: a clock
    /// parameter, a clock field of `self`, an `if` whose branches are place
    /// expressions, or a call of a declaration whose result is `Clock`.
    fn is_place(&self, id: ExprId, declarations: &Declarations<'_>) -> bool {
        match self.node(id) {
            Some(node @ ExprNode::Name(_)) => {
                matches!(
                    parameter(self.declaration, node),
                    Some((_, ClockSlot::Clock))
                )
            }
            Some(ExprNode::Field { operand, field }) => {
                matches!(self.node(*operand), Some(ExprNode::SelfRef))
                    && declarations.clock_fields.contains(field)
            }
            Some(ExprNode::If {
                then, otherwise, ..
            }) => self.is_place(*then, declarations) && self.is_place(*otherwise, declarations),
            Some(ExprNode::Call { name, .. }) => declarations
                .named(name)
                .is_some_and(|called| called.result == ClockSlot::Clock),
            _ => false,
        }
    }

    /// Whether node `id` is shaped as a bound: a time quantity literal, a
    /// negated one, or a data parameter.
    fn is_bound(&self, id: ExprId) -> bool {
        match self.node(id) {
            Some(ExprNode::Quantity(_)) => true,
            Some(ExprNode::Negate(operand)) => {
                matches!(self.node(*operand), Some(ExprNode::Quantity(_)))
            }
            Some(node @ ExprNode::Name(_)) => {
                matches!(
                    parameter(self.declaration, node),
                    Some((_, ClockSlot::Data))
                )
            }
            _ => false,
        }
    }
}

/// Where a body node is read.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Position {
    /// As a place: a clock operand, a clock argument or a place branch.
    Place,
    /// Anywhere else.
    Other,
}

/// FR-230's check of one clock-reading declaration's body.
///
/// # Errors
///
/// `ill_typed`/`type-mismatch` at a `Clock`-result body that is no place
/// expression; `clock-comparison` at a comparison of two clocks;
/// `negative-clock-constant` or `clock-bound-not-constant` at a bound that
/// is not a time quantity literal or a quantity parameter;
/// `clock-read-outside-constraint` at any other read of a clock parameter,
/// including an equation that sets a data value from one.
pub(crate) fn check_declaration(
    declaration: &ClockReadingDeclaration,
    declarations: &Declarations<'_>,
) -> Result<(), TemporalCause> {
    let body = Body::of(declaration);
    let root = body.expression().root_id();
    let start = if declaration.result == ClockSlot::Clock {
        if !body.is_place(root, declarations) {
            return Err(TemporalCause::NotAPlaceExpression {
                span: body.span(root),
            });
        }
        Position::Place
    } else {
        Position::Other
    };
    let mut pending = vec![(root, start)];
    while let Some((id, position)) = pending.pop() {
        let Some(node) = body.node(id) else {
            continue;
        };
        match node {
            ExprNode::Name(_) => {
                if let Some((_, ClockSlot::Clock | ClockSlot::Bearing)) =
                    parameter(declaration, node)
                {
                    if position != Position::Place {
                        return Err(TemporalCause::ClockReadOutsideConstraint {
                            span: body.span(id),
                        });
                    }
                }
            }
            ExprNode::Binary {
                operator,
                left,
                right,
            } if comparison(*operator).is_some()
                && (body.is_place(*left, declarations) || body.is_place(*right, declarations)) =>
            {
                let (place, bound) = if body.is_place(*left, declarations) {
                    (*left, *right)
                } else {
                    (*right, *left)
                };
                if body.is_place(bound, declarations) {
                    return Err(TemporalCause::ClockComparison {
                        span: body.span(id),
                    });
                }
                if !body.is_bound(bound) {
                    // An equation that sets a data value from a clock reads
                    // the clock outside a constraint; an order comparison
                    // with a non-constant bound has no constant.
                    return Err(if *operator == BinaryOperator::Equal {
                        TemporalCause::ClockReadOutsideConstraint {
                            span: body.span(place),
                        }
                    } else {
                        TemporalCause::ClockBoundNotConstant {
                            span: body.span(bound),
                        }
                    });
                }
                if let Some(ExprNode::Negate(_)) = body.node(bound) {
                    return Err(TemporalCause::NegativeClockConstant {
                        span: body.span(bound),
                    });
                }
                pending.push((place, Position::Place));
                pending.push((bound, Position::Other));
            }
            ExprNode::If {
                condition,
                then,
                otherwise,
            } if position == Position::Place => {
                pending.push((*condition, Position::Other));
                pending.push((*then, Position::Place));
                pending.push((*otherwise, Position::Place));
            }
            ExprNode::Field { operand, field }
                if position != Position::Place
                    && matches!(body.node(*operand), Some(ExprNode::SelfRef))
                    && declarations.clock_fields.contains(field) =>
            {
                return Err(TemporalCause::ClockReadOutsideConstraint {
                    span: body.span(id),
                });
            }
            ExprNode::Call { name, arguments } => {
                let called = declarations.named(name);
                if position != Position::Place
                    && called.is_some_and(|called| called.result == ClockSlot::Clock)
                {
                    return Err(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(id),
                    });
                }
                for (index, argument) in arguments.iter().enumerate() {
                    let slot = called
                        .and_then(|called| called.parameters.get(index).copied())
                        .unwrap_or(ClockSlot::Data);
                    let read = if slot == ClockSlot::Data {
                        Position::Other
                    } else {
                        if !body.is_place(*argument, declarations) {
                            return Err(TemporalCause::NotAPlaceExpression {
                                span: body.span(*argument),
                            });
                        }
                        Position::Place
                    };
                    pending.push((*argument, read));
                }
            }
            _ => pending.extend(
                node.children()
                    .into_iter()
                    .map(|child| (child, Position::Other)),
            ),
        }
    }
    Ok(())
}

/// A clock-reading declaration's argument, read at the call site.
#[derive(Clone)]
enum Argument {
    /// The places a clock argument can denote.
    Places(Places),
    /// A data argument: the call-site node it is, when it is one, and its
    /// value as a constant time quantity, or why it is none.
    Value {
        node: Option<ExprId>,
        constant: Result<Rational, TemporalCause>,
    },
}

/// The expansion of the calls of one precondition, at one call site.
pub(super) struct Expansion<'e, 'f, 'c> {
    pub(super) located: &'e Located<'f>,
    pub(super) clocks: &'e Clocks<'c>,
    pub(super) declarations: &'e Declarations<'e>,
    /// The object type `self` ranges over at the call site.
    pub(super) context: quire_exact::EffectiveId,
}

/// The not-yet-checked refusal of a clock-reading construct this build
/// does not expand, at `span`.
fn unexpanded(span: Span) -> CheckCause {
    CheckCause::TimedNotChecked {
        construct: TimedConstruct::Clock,
        span,
    }
}

/// The bodies being expanded, outermost first, to refuse recursion.
type Stack = Vec<String>;

impl<'e, 'f, 'c> Expansion<'e, 'f, 'c> {
    /// The places call-site node `id` can denote when it is a place
    /// expression: `self.x`, an `if` whose branches are place expressions,
    /// or a call of a declaration whose result is `Clock`. `None` when it
    /// is none.
    pub(super) fn site_places(
        &self,
        id: ExprId,
        budget: &mut PartBudget,
    ) -> Result<Option<Places>, CheckCause> {
        if let Some(clock) = self.located.clock(id) {
            return Ok(Some(vec![(Vec::new(), clock)]));
        }
        match self.located.node(id) {
            Some(ExprNode::If {
                condition,
                then,
                otherwise,
            }) => {
                if self.located.reads_clock(*condition) {
                    return Err(timed(self.located.outside(*condition)));
                }
                let (Some(then), Some(otherwise)) = (
                    self.site_places(*then, budget)?,
                    self.site_places(*otherwise, budget)?,
                ) else {
                    return Ok(None);
                };
                let mut places = guarded(then, (*condition, true));
                places.extend(guarded(otherwise, (*condition, false)));
                budget.charge(places.len())?;
                Ok(Some(places))
            }
            Some(ExprNode::Call { name, arguments }) => {
                let Some(declaration) = self.declarations.named(name) else {
                    return Ok(None);
                };
                if declaration.result != ClockSlot::Clock {
                    return Ok(None);
                }
                let arguments = self.site_arguments(declaration, arguments, budget)?;
                let body = Body::of(declaration);
                let mut stack = Vec::new();
                self.places(
                    &body,
                    &arguments,
                    body.expression().root_id(),
                    id,
                    &mut stack,
                    budget,
                )
                .map(Some)
            }
            _ => Ok(None),
        }
    }

    /// The disjunctive forms of the call at call-site node `id` of a
    /// clock-reading predicate, read positively and negated.
    pub(super) fn predicate(
        &self,
        id: ExprId,
        budget: &mut PartBudget,
    ) -> Result<Option<(Dnf, Dnf)>, CheckCause> {
        let Some(ExprNode::Call { name, arguments }) = self.located.node(id) else {
            return Ok(None);
        };
        let Some(declaration) = self.declarations.named(name) else {
            return Ok(None);
        };
        if declaration.result == ClockSlot::Clock {
            // A place expression: the comparison that reads it expands it.
            return Ok(None);
        }
        let arguments = self.site_arguments(declaration, arguments, budget)?;
        let body = Body::of(declaration);
        let mut stack = Vec::new();
        self.boolean(
            &body,
            &arguments,
            body.expression().root_id(),
            id,
            &mut stack,
            budget,
        )
        .map(Some)
    }

    /// The arguments of a call at the call site, by parameter name.
    fn site_arguments(
        &self,
        declaration: &ClockReadingDeclaration,
        arguments: &[ExprId],
        budget: &mut PartBudget,
    ) -> Result<BTreeMap<String, Argument>, CheckCause> {
        let mut bound = BTreeMap::new();
        for ((name, _), (slot, argument)) in declaration.function.parameters.iter().zip(
            declaration
                .parameters
                .iter()
                .copied()
                .zip(arguments.iter().copied()),
        ) {
            let span = self.located.span(argument);
            let read = match slot {
                ClockSlot::Clock => match self.site_places(argument, budget)? {
                    Some(places) => Argument::Places(places),
                    None => {
                        return Err(timed(TemporalCause::NotAPlaceExpression { span }));
                    }
                },
                ClockSlot::Bearing => return Err(unexpanded(span)),
                ClockSlot::Data => {
                    if self.located.reads_clock(argument) {
                        return Err(timed(self.located.outside(argument)));
                    }
                    Argument::Value {
                        node: Some(argument),
                        constant: self.located.constant(argument, self.clocks),
                    }
                }
            };
            bound.insert(name.clone(), read);
        }
        Ok(bound)
    }

    /// The arguments of a call written in `body`, by parameter name of the
    /// called declaration.
    #[allow(clippy::too_many_arguments)] // one call site's whole reading state
    fn body_arguments<'d>(
        &self,
        body: &Body<'d>,
        environment: &BTreeMap<String, Argument>,
        called: &ClockReadingDeclaration,
        arguments: &[ExprId],
        site: ExprId,
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<BTreeMap<String, Argument>, CheckCause> {
        let mut bound = BTreeMap::new();
        for ((name, _), (slot, argument)) in called.function.parameters.iter().zip(
            called
                .parameters
                .iter()
                .copied()
                .zip(arguments.iter().copied()),
        ) {
            let read = match slot {
                ClockSlot::Clock => Argument::Places(self.places(
                    body,
                    environment,
                    argument,
                    site,
                    stack,
                    budget,
                )?),
                ClockSlot::Bearing => return Err(unexpanded(self.located.span(site))),
                ClockSlot::Data => match body.node(argument) {
                    Some(node @ ExprNode::Name(_)) => match parameter(body.declaration, node)
                        .and_then(|(name, _)| environment.get(name))
                    {
                        Some(passed) => passed.clone(),
                        None => return Err(unexpanded(self.located.span(site))),
                    },
                    Some(ExprNode::Quantity(quantity)) => Argument::Value {
                        node: None,
                        constant: self.clocks.quantity(quantity),
                    },
                    _ => return Err(unexpanded(self.located.span(site))),
                },
            };
            bound.insert(name.clone(), read);
        }
        Ok(bound)
    }

    /// The places body node `id` denotes under `environment`.
    fn places<'d>(
        &self,
        body: &Body<'d>,
        environment: &BTreeMap<String, Argument>,
        id: ExprId,
        site: ExprId,
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<Places, CheckCause> {
        let site_span = self.located.span(site);
        match body.node(id) {
            Some(node @ ExprNode::Name(_)) => match parameter(body.declaration, node)
                .and_then(|(name, _)| environment.get(name))
            {
                Some(Argument::Places(places)) => Ok(places.clone()),
                _ => Err(unexpanded(site_span)),
            },
            Some(ExprNode::Field { operand, field })
                if matches!(body.node(*operand), Some(ExprNode::SelfRef)) =>
            {
                let clock = self
                    .clocks
                    .model
                    .clock_field(self.context, field)
                    .and_then(|key| {
                        self.clocks
                            .fields
                            .iter()
                            .position(|clock| &clock.field == key)
                    })
                    .map(ClockId)
                    .ok_or(timed(TemporalCause::NotAPlaceExpression {
                        span: site_span,
                    }))?;
                Ok(vec![(Vec::new(), clock)])
            }
            Some(ExprNode::If {
                condition,
                then,
                otherwise,
            }) => {
                let (selected, rejected) =
                    self.boolean(body, environment, *condition, site, stack, budget)?;
                let then = self.places(body, environment, *then, site, stack, budget)?;
                let otherwise = self.places(body, environment, *otherwise, site, stack, budget)?;
                let mut places = Vec::new();
                for (condition, branch) in [(selected, then), (rejected, otherwise)] {
                    for (literals, clocks) in condition {
                        if !clocks.is_empty() {
                            // A place selected by a clock constraint has no
                            // data condition.
                            return Err(unexpanded(site_span));
                        }
                        for (selecting, clock) in &branch {
                            places.push((
                                literals.iter().chain(selecting).copied().collect(),
                                *clock,
                            ));
                        }
                    }
                }
                budget.charge(places.len())?;
                Ok(places)
            }
            Some(ExprNode::Call { name, arguments }) => {
                let called = self
                    .declarations
                    .named(name)
                    .filter(|called| called.result == ClockSlot::Clock)
                    .ok_or(unexpanded(site_span))?;
                let inner = self.enter(called, stack, site_span)?;
                let arguments =
                    self.body_arguments(body, environment, called, arguments, site, stack, budget)?;
                let called_body = Body::of(called);
                let places = self.places(
                    &called_body,
                    &arguments,
                    called_body.expression().root_id(),
                    site,
                    stack,
                    budget,
                );
                stack.truncate(inner);
                places
            }
            _ => Err(timed(TemporalCause::NotAPlaceExpression {
                span: site_span,
            })),
        }
    }

    /// Push `called` on the expansion stack, refusing recursion; returns
    /// the stack's length before the push.
    fn enter(
        &self,
        called: &ClockReadingDeclaration,
        stack: &mut Stack,
        span: Span,
    ) -> Result<usize, CheckCause> {
        let name = &called.function.name;
        if stack.contains(name) {
            return Err(unexpanded(span));
        }
        let depth = stack.len();
        stack.push(name.clone());
        Ok(depth)
    }

    /// The constant body node `id` denotes as a bound.
    fn bound(
        &self,
        body: &Body<'_>,
        environment: &BTreeMap<String, Argument>,
        id: ExprId,
        site: ExprId,
    ) -> Result<Rational, CheckCause> {
        match body.node(id) {
            Some(ExprNode::Quantity(quantity)) => self.clocks.quantity(quantity).map_err(timed),
            Some(node @ ExprNode::Name(_)) => match parameter(body.declaration, node)
                .and_then(|(name, _)| environment.get(name))
            {
                Some(Argument::Value { constant, .. }) => constant.clone().map_err(timed),
                _ => Err(unexpanded(self.located.span(site))),
            },
            _ => Err(timed(TemporalCause::ClockBoundNotConstant {
                span: self.located.span(site),
            })),
        }
    }

    /// The disjunctive forms of Boolean body node `id` under `environment`,
    /// read positively and negated.
    fn boolean<'d>(
        &self,
        body: &Body<'d>,
        environment: &BTreeMap<String, Argument>,
        id: ExprId,
        site: ExprId,
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<(Dnf, Dnf), CheckCause> {
        let site_span = self.located.span(site);
        let truth = || vec![(Vec::new(), Vec::new())];
        match body.node(id) {
            Some(ExprNode::Boolean(true)) => Ok((truth(), Vec::new())),
            Some(ExprNode::Boolean(false)) => Ok((Vec::new(), truth())),
            Some(ExprNode::Not(operand)) => {
                let (positive, negative) =
                    self.boolean(body, environment, *operand, site, stack, budget)?;
                Ok((negative, positive))
            }
            Some(ExprNode::Binary {
                operator:
                    operator @ (BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies),
                left,
                right,
            }) => {
                let (lp, ln) = self.boolean(body, environment, *left, site, stack, budget)?;
                let (rp, rn) = self.boolean(body, environment, *right, site, stack, budget)?;
                Ok(match operator {
                    BinaryOperator::And => (budget.product(&lp, &rp)?, [ln, rn].concat()),
                    BinaryOperator::Or => ([lp, rp].concat(), budget.product(&ln, &rn)?),
                    _ => ([ln, rp].concat(), budget.product(&lp, &rn)?),
                })
            }
            Some(ExprNode::Binary {
                operator,
                left,
                right,
            }) if comparison(*operator).is_some() => {
                let Some(op) = comparison(*operator) else {
                    return Err(unexpanded(site_span));
                };
                let (place, bound, op) = if body.is_place(*left, self.declarations) {
                    (*left, *right, op)
                } else if body.is_place(*right, self.declarations) {
                    (*right, *left, op.mirrored())
                } else {
                    return Err(unexpanded(site_span));
                };
                let places = self.places(body, environment, place, site, stack, budget)?;
                let bound = self.bound(body, environment, bound, site)?;
                Ok(constraints(places, op, &bound, site_span))
            }
            Some(node @ ExprNode::Name(_)) => match parameter(body.declaration, node)
                .and_then(|(name, _)| environment.get(name))
            {
                Some(Argument::Value {
                    node: Some(argument),
                    ..
                }) => Ok((
                    vec![(vec![(*argument, true)], Vec::new())],
                    vec![(vec![(*argument, false)], Vec::new())],
                )),
                _ => Err(unexpanded(site_span)),
            },
            Some(ExprNode::Call { name, arguments }) => {
                let called = self
                    .declarations
                    .named(name)
                    .filter(|called| called.result != ClockSlot::Clock)
                    .ok_or(unexpanded(site_span))?;
                let inner = self.enter(called, stack, site_span)?;
                let arguments =
                    self.body_arguments(body, environment, called, arguments, site, stack, budget)?;
                let called_body = Body::of(called);
                let read = self.boolean(
                    &called_body,
                    &arguments,
                    called_body.expression().root_id(),
                    site,
                    stack,
                    budget,
                );
                stack.truncate(inner);
                read
            }
            _ => Err(unexpanded(site_span)),
        }
    }
}

/// `places`, each selected also by `literal`.
fn guarded(places: Places, literal: (ExprId, bool)) -> Places {
    places
        .into_iter()
        .map(|(mut literals, clock)| {
            literals.insert(0, literal);
            (literals, clock)
        })
        .collect()
}

/// The disjunctive forms of `place ~ bound` over `places`, read positively
/// and negated. The places' selecting conditions are exclusive and
/// exhaustive (each place expression's branches partition its conditions),
/// so the negation is the same places with each constraint negated.
pub(super) fn constraints(places: Places, op: ClockOp, bound: &Rational, span: Span) -> (Dnf, Dnf) {
    let constraint = |clock: ClockId, op: ClockOp| ClockConstraint {
        clock,
        op,
        bound: bound.clone(),
        span,
    };
    let positive = places
        .iter()
        .map(|(literals, clock)| (literals.clone(), vec![constraint(*clock, op)]))
        .collect();
    let negative = places
        .iter()
        .flat_map(|(literals, clock)| {
            op.negated()
                .into_iter()
                .map(move |negated| (literals.clone(), vec![constraint(*clock, negated)]))
        })
        .collect();
    (positive, negative)
}

/// The span of the first call of a clock-reading declaration in
/// `function`'s body, which no clock rule admits there.
pub(super) fn first_call(
    declarations: &Declarations<'_>,
    function: &qsl_forms::FunctionDeclaration,
) -> Option<Span> {
    let spans = function
        .spans()
        .and_then(|spans| spans.body.of_nodes(&function.body))?;
    function
        .body
        .iter()
        .filter(|node| declarations.is_call(&function.body, node.id()))
        .filter_map(|node| spans.get(node.id().index()).copied())
        .min_by_key(|span| span.start)
}

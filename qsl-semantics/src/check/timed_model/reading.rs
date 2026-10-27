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
//! can denote, each with the data condition that selects it; `self` for an
//! object parameter as the receiver; and a data argument as the call-site
//! term it is. The body is then read with its parameters bound to those
//! arguments, into the disjunctive form the precondition's guard takes:
//! each data predicate of the body is read at the call, its parameters
//! replaced by their arguments, so the recorded guard holds atomic clock
//! constraints over the receiver's clocks only.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use qsl_forms::{
    BinaryOperator, ExprId, ExprNode, Expression, ExpressionForm, ExpressionSpans, NameForm,
    TypeFormHead,
};
use qsl_foundation::Span;
use quire_exact::{EffectiveId, Rational};

use super::{
    timed, ClockConstraint, ClockId, ClockOp, Clocks, DataLiteral, Dnf, Located, PartBudget,
};
use crate::check::assemble::{ClockReadingDeclaration, ClockSlot};
use crate::check::lowering::AdmittedModel;
use crate::check::lowering::PlaceStep;
use crate::check::refusal::{CheckCause, TemporalCause, TimedConstruct};
use quire_checked::key::DeclarationKey;

/// The places a place expression can denote, each with the data literals
/// that select it.
pub(super) type Places = Vec<(Vec<(DataLiteral, bool)>, Place)>;

/// A clock place relative to the receiver.
#[derive(Clone, Debug)]
pub(super) struct Place {
    /// The reference fields walked from the receiver to the object holding
    /// the clock; none for the receiver's own clock.
    pub(super) via: Vec<PlaceStep>,
    /// The clock.
    pub(super) clock: ClockId,
}

/// The unit's clock-reading declarations, by name.
pub(crate) struct Declarations<'r> {
    by_name: BTreeMap<&'r str, &'r ClockReadingDeclaration>,
    /// The admitted models, whose object types hold the clock fields a body
    /// reads as `self.x` or `o.x`.
    models: &'r [AdmittedModel],
}

impl<'r> Declarations<'r> {
    pub(crate) fn new(
        declarations: &'r [ClockReadingDeclaration],
        models: &'r [AdmittedModel],
    ) -> Self {
        Self {
            by_name: declarations
                .iter()
                .map(|declaration| (declaration.function.name.as_str(), declaration))
                .collect(),
            models,
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
            Some(ExprNode::Call { name, arguments: _ }) if self.named(name).is_some()
        )
    }

    /// What the path `steps` below `root` reaches. From an object
    /// parameter, the leading reference fields select the object the rest
    /// of the path is read on (FR-230-AC-10's `o.peer.x`).
    fn path_kind(&self, root: &PathRoot, steps: &[PlaceStep]) -> PathKind {
        let reached: Vec<(Vec<PlaceStep>, &[PlaceStep])> = match root {
            PathRoot::Object(object) => self
                .models
                .iter()
                .flat_map(|model| {
                    let (rest, holder) = match object {
                        Some(object) => {
                            let (walked, holder) = model.through_references(*object, steps);
                            (&steps[walked..], Some(holder))
                        }
                        None => (steps, None),
                    };
                    model
                        .clock_places()
                        .iter()
                        .filter(|place| {
                            holder.is_none_or(|holder| model.conforms(holder, place.owner))
                        })
                        .map(|place| (place.path.clone(), rest))
                        .collect::<Vec<_>>()
                })
                .collect(),
            PathRoot::Record(model, key) => self
                .models
                .get(*model)
                .map(|model| {
                    model
                        .record_clock_paths(key)
                        .into_iter()
                        .map(|path| (path, steps))
                        .collect()
                })
                .unwrap_or_default(),
        };
        if reached
            .iter()
            .any(|(place, rest)| place.as_slice() == *rest)
        {
            PathKind::Clock
        } else if reached
            .iter()
            .any(|(place, rest)| crate::check::lowering::strictly_below(place, rest))
        {
            PathKind::Bearing
        } else {
            PathKind::Data
        }
    }

    /// The package record type a parameter's written type names, with the
    /// index of its model, when it names one that holds a clock.
    fn record_type(&self, written: &qsl_forms::TypeForm) -> Option<(usize, DeclarationKey)> {
        let TypeFormHead::Name(name) = &written.head else {
            return None;
        };
        self.models.iter().enumerate().find_map(|(index, model)| {
            let key = model.record_named(name)?;
            (!model.record_clock_paths(key).is_empty()).then(|| (index, key.clone()))
        })
    }

    /// The object type a parameter's written type names, with its model,
    /// when it names one.
    fn object_type(
        &self,
        written: &qsl_forms::TypeForm,
    ) -> Option<(&'r AdmittedModel, EffectiveId)> {
        let TypeFormHead::Name(name) = &written.head else {
            return None;
        };
        self.models
            .iter()
            .find_map(|model| Some((model, model.object_type_named(name)?)))
    }
}

/// Where a place path of a body starts.
enum PathRoot {
    /// An object: `self` (`None`, any object type) or an object parameter.
    Object(Option<EffectiveId>),
    /// A record parameter, by its model's index and its record type.
    Record(usize, DeclarationKey),
}

/// What a place path reaches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PathKind {
    /// A clock place.
    Clock,
    /// A value holding clock places below it.
    Bearing,
    /// A data value.
    Data,
}

/// A declaration's parameter: its name, how it is read, and the object
/// type or clock-bearing package record type it is typed by, when it is
/// one.
struct Parameter<'d> {
    name: &'d str,
    slot: ClockSlot,
    object: Option<EffectiveId>,
    record: Option<(usize, DeclarationKey)>,
}

/// The parameter a body node names, when it is a bare name of one.
fn parameter<'d>(
    declarations: &Declarations<'_>,
    declaration: &'d ClockReadingDeclaration,
    node: &ExprNode,
) -> Option<Parameter<'d>> {
    let ExprNode::Name(name) = node else {
        return None;
    };
    if name.segments.len() > 1 {
        return None;
    }
    let index = declaration
        .function
        .parameters
        .iter()
        .position(|(parameter, _)| *parameter == name.name)?;
    let (written, type_form) = declaration.function.parameters.get(index)?;
    Some(Parameter {
        name: written.as_str(),
        slot: declaration.parameters.get(index).copied()?,
        object: declarations
            .object_type(type_form)
            .map(|(_, object)| object),
        record: declarations.record_type(type_form),
    })
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

    /// The parameter body node `id` names, when it is a bare name of one.
    fn parameter(&self, declarations: &Declarations<'_>, id: ExprId) -> Option<Parameter<'d>> {
        parameter(declarations, self.declaration, self.node(id)?)
    }

    /// The place path at `id`: the field steps of `r.f.g…` and the node `r`
    /// they start from, `self` or a parameter. `None` when `id` is no field
    /// path.
    fn path(&self, id: ExprId) -> Option<(ExprId, Vec<PlaceStep>)> {
        let mut steps = Vec::new();
        let mut at = id;
        while let Some(ExprNode::Field { operand, field }) = self.node(at) {
            steps.push(PlaceStep::Field(field.clone()));
            at = *operand;
        }
        if steps.is_empty() {
            return None;
        }
        steps.reverse();
        Some((at, steps))
    }

    /// What the place path at `id` reaches, when it is one from `self`, an
    /// object parameter or a clock-bearing record parameter.
    fn path_kind(&self, id: ExprId, declarations: &Declarations<'_>) -> Option<PathKind> {
        let (root, steps) = self.path(id)?;
        let root = match self.node(root)? {
            ExprNode::SelfRef => PathRoot::Object(None),
            ExprNode::Name(_) => {
                let parameter = self.parameter(declarations, root)?;
                match (parameter.object, parameter.record) {
                    (Some(object), _) => PathRoot::Object(Some(object)),
                    (None, Some((model, key))) => PathRoot::Record(model, key),
                    (None, None) => return None,
                }
            }
            _ => return None,
        };
        Some(declarations.path_kind(&root, &steps))
    }

    /// Whether node `id` reads a clock place directly: `self.x`, `o.x` of an
    /// object parameter or `r.x` of a record parameter, through record
    /// fields.
    fn is_clock_field(&self, id: ExprId, declarations: &Declarations<'_>) -> bool {
        self.path_kind(id, declarations) == Some(PathKind::Clock)
    }

    /// Whether node `id` can be the argument for a clock-bearing parameter:
    /// a clock-bearing parameter, or a place path to a clock-bearing value.
    fn is_bearing_place(&self, id: ExprId, declarations: &Declarations<'_>) -> bool {
        match self.node(id) {
            Some(ExprNode::Name(_)) => self
                .parameter(declarations, id)
                .is_some_and(|parameter| parameter.slot == ClockSlot::Bearing),
            Some(ExprNode::Field { .. }) => {
                self.path_kind(id, declarations) == Some(PathKind::Bearing)
            }
            _ => false,
        }
    }

    /// Whether node `id` is a place expression of this body: a clock
    /// parameter, a clock field of `self` or of an object parameter, an `if`
    /// whose branches are place expressions, or a call of a declaration
    /// whose result is `Clock`.
    fn is_place(&self, id: ExprId, declarations: &Declarations<'_>) -> bool {
        match self.node(id) {
            Some(ExprNode::Name(_)) => self
                .parameter(declarations, id)
                .is_some_and(|parameter| parameter.slot == ClockSlot::Clock),
            Some(ExprNode::Field { .. }) => self.is_clock_field(id, declarations),
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
    fn is_bound(&self, id: ExprId, declarations: &Declarations<'_>) -> bool {
        match self.node(id) {
            Some(ExprNode::Quantity(_)) => true,
            Some(ExprNode::Negate(operand)) => {
                matches!(self.node(*operand), Some(ExprNode::Quantity(_)))
            }
            Some(ExprNode::Name(_)) => self
                .parameter(declarations, id)
                .is_some_and(|parameter| parameter.slot == ClockSlot::Data),
            _ => false,
        }
    }

    /// The first node under `id` that reads a clock: a clock or
    /// clock-bearing parameter, a clock field, or a call of a clock-reading
    /// declaration. `None` when the subtree is data.
    fn first_clock_read(&self, id: ExprId, declarations: &Declarations<'_>) -> Option<ExprId> {
        let mut pending = vec![id];
        let mut found: Option<ExprId> = None;
        while let Some(id) = pending.pop() {
            let Some(node) = self.node(id) else {
                continue;
            };
            let reads = match node {
                ExprNode::Name(_) => self
                    .parameter(declarations, id)
                    .is_some_and(|parameter| parameter.slot != ClockSlot::Data),
                ExprNode::Field { .. } => match self.path_kind(id, declarations) {
                    Some(PathKind::Clock | PathKind::Bearing) => true,
                    // A data field of a clock-bearing value reads no clock.
                    Some(PathKind::Data) => continue,
                    None => false,
                },
                ExprNode::Call { name, .. } => declarations.named(name).is_some(),
                _ => false,
            };
            if reads {
                let earlier =
                    found.is_none_or(|first| self.span(id).start < self.span(first).start);
                if earlier {
                    found = Some(id);
                }
                continue;
            }
            pending.extend(node.children());
        }
        found
    }
}

/// The bare-name nodes under `root` that name one of `names` free of every
/// binder above them in the subtree: the occurrences of a parameter.
pub(super) fn free_occurrences<'n>(
    expression: &Expression,
    root: ExprId,
    names: &BTreeSet<&'n str>,
) -> Vec<(ExprId, &'n str)> {
    let mut found = Vec::new();
    let mut pending: Vec<(ExprId, BTreeSet<&str>)> = vec![(root, BTreeSet::new())];
    while let Some((id, shadowed)) = pending.pop() {
        let Some(node) = expression.get(id).map(|node| node.node()) else {
            continue;
        };
        if let ExprNode::Name(name) = node {
            if name.segments.len() == 1 {
                if let Some(parameter) = names.get(name.name.as_str()) {
                    if !shadowed.contains(*parameter) {
                        found.push((id, *parameter));
                    }
                }
            }
            continue;
        }
        // The children outside each binder's scope: a `let` value, a
        // query's or accumulation's source, an accumulation's identity, a
        // sequence's bounds and a case's scrutinee.
        let outside: Vec<ExprId> = match node {
            ExprNode::Let { value, .. } => vec![*value],
            ExprNode::Query { source, .. }
            | ExprNode::Count { source, .. }
            | ExprNode::Sum { source, .. } => vec![*source],
            ExprNode::Accumulate {
                source, identity, ..
            } => std::iter::once(*source).chain(*identity).collect(),
            ExprNode::Seq { low, high, .. } => vec![*low, *high],
            ExprNode::Case(case) => vec![case.scrutinee],
            _ => Vec::new(),
        };
        let mut inner = shadowed.clone();
        for binder in node.binders() {
            if let Some(parameter) = names.get(binder) {
                inner.insert(*parameter);
            }
        }
        for child in node.children() {
            let scope = if outside.contains(&child) {
                shadowed.clone()
            } else {
                inner.clone()
            };
            pending.push((child, scope));
        }
    }
    found
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
/// `clock-read-outside-constraint` at any other read of a clock parameter
/// or clock field, including an equation that sets a data value from one.
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
                let clock = body
                    .parameter(declarations, id)
                    .is_some_and(|parameter| parameter.slot != ClockSlot::Data);
                if clock && position != Position::Place {
                    return Err(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(id),
                    });
                }
            }
            ExprNode::Binary {
                operator,
                left,
                right,
            } if ClockOp::of(*operator).is_some()
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
                if !body.is_bound(bound, declarations) {
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
            ExprNode::Field { .. } if body.path_kind(id, declarations).is_some() => {
                // A clock or clock-bearing value is read only as a place; a
                // data field of one is a data read.
                if body.path_kind(id, declarations) != Some(PathKind::Data)
                    && position != Position::Place
                {
                    return Err(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(id),
                    });
                }
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
                    let place = match slot {
                        ClockSlot::Data => None,
                        ClockSlot::Clock => Some(body.is_place(*argument, declarations)),
                        ClockSlot::Bearing => Some(body.is_bearing_place(*argument, declarations)),
                    };
                    let read = match place {
                        None => Position::Other,
                        Some(true) => Position::Place,
                        Some(false) => {
                            return Err(TemporalCause::NotAPlaceExpression {
                                span: body.span(*argument),
                            })
                        }
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
    /// A place path below the receiver: `self` (no steps) for an object
    /// parameter, or a clock-bearing record value for a record parameter.
    Path(Vec<PlaceStep>),
    /// A data argument: the call-site term it is, and its value as a
    /// constant time quantity, or why it is none.
    Value {
        term: DataLiteral,
        constant: Result<Rational, TemporalCause>,
    },
}

/// The expansion of the calls of one precondition, at one call site.
pub(super) struct Expansion<'e, 'f, 'c> {
    pub(super) located: &'e Located<'f>,
    pub(super) clocks: &'e Clocks<'c>,
    pub(super) declarations: &'e Declarations<'e>,
    /// The object type `self` ranges over at the call site.
    pub(super) context: EffectiveId,
}

/// The not-yet-checked refusal of `construct`, which this build does not
/// expand, at `span`.
fn not_checked(construct: TimedConstruct, span: Span) -> CheckCause {
    CheckCause::TimedNotChecked { construct, span }
}

/// The bodies being expanded, outermost first, to refuse recursion.
type Stack = Vec<String>;

/// A body reading in progress: the body, its parameters' arguments, and
/// the call-site node the outermost call is.
struct Frame<'b, 'd> {
    body: &'b Body<'d>,
    environment: &'b BTreeMap<String, Argument>,
    site: ExprId,
}

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
            return Ok(Some(vec![(
                Vec::new(),
                Place {
                    via: Vec::new(),
                    clock,
                },
            )]));
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
                let mut places = guarded(then, (DataLiteral::Site(*condition), true));
                places.extend(guarded(otherwise, (DataLiteral::Site(*condition), false)));
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
                let environment = self.site_arguments(declaration, arguments, budget)?;
                let body = Body::of(declaration);
                let frame = Frame {
                    body: &body,
                    environment: &environment,
                    site: id,
                };
                let mut stack = vec![declaration.function.name.clone()];
                self.places(&frame, body.expression().root_id(), &mut stack, budget)
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
        let environment = self.site_arguments(declaration, arguments, budget)?;
        let body = Body::of(declaration);
        let frame = Frame {
            body: &body,
            environment: &environment,
            site: id,
        };
        let mut stack = vec![declaration.function.name.clone()];
        self.boolean(&frame, body.expression().root_id(), &mut stack, budget)
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
        for ((name, type_form), (slot, argument)) in declaration.function.parameters.iter().zip(
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
                ClockSlot::Bearing => Argument::Path(self.site_record(type_form, argument)?),
                ClockSlot::Data => {
                    if self.located.reads_clock(argument) {
                        return Err(timed(self.located.outside(argument)));
                    }
                    if let Some(path) = self.site_object(type_form, argument) {
                        // `self`, or an object `self` refers to.
                        Argument::Path(path)
                    } else {
                        Argument::Value {
                            term: DataLiteral::Site(argument),
                            constant: self.located.constant(argument, self.clocks),
                        }
                    }
                }
            };
            bound.insert(name.clone(), read);
        }
        Ok(bound)
    }

    /// The reference fields a call-site argument for an object parameter of
    /// type `type_form` walks from `self`: none for `self`, `[peer]` for
    /// `self.peer`, each a required single reference to an object of the
    /// parameter's type. `None` for any other argument, which is data.
    fn site_object(
        &self,
        type_form: &qsl_forms::TypeForm,
        argument: ExprId,
    ) -> Option<Vec<PlaceStep>> {
        let (model, expected) = self.declarations.object_type(type_form)?;
        let path = match self.located.node(argument)? {
            ExprNode::SelfRef => Vec::new(),
            _ => super::self_path(self.located.form, argument)?,
        };
        let (walked, holder) = model.through_references(self.context, &path);
        (walked == path.len() && model.conforms(holder, expected)).then_some(path)
    }

    /// The record path a call-site argument for a clock-bearing parameter of
    /// type `type_form` denotes: a place path `self.f…` to a value of that
    /// record type.
    fn site_record(
        &self,
        type_form: &qsl_forms::TypeForm,
        argument: ExprId,
    ) -> Result<Vec<PlaceStep>, CheckCause> {
        let span = self.located.span(argument);
        let not_a_place = || timed(TemporalCause::NotAPlaceExpression { span });
        let Some((_, expected)) = self.declarations.record_type(type_form) else {
            // A clock-bearing type of the unit's own source: no place of the
            // model holds one.
            return Err(not_a_place());
        };
        let path = super::self_path(self.located.form, argument).ok_or_else(not_a_place)?;
        if self.clocks.model.record_at(self.context, &path) != Some(&expected) {
            return Err(not_a_place());
        }
        Ok(path)
    }

    /// The receiver path the place path at body node `root` starts from:
    /// none for `self`, an object or record parameter's argument path.
    fn prefix(&self, frame: &Frame<'_, '_>, root: ExprId) -> Result<Vec<PlaceStep>, CheckCause> {
        let body = frame.body;
        match body.node(root) {
            Some(ExprNode::SelfRef) => Ok(Vec::new()),
            Some(ExprNode::Name(_)) => match body
                .parameter(self.declarations, root)
                .and_then(|parameter| frame.environment.get(parameter.name))
            {
                Some(Argument::Path(prefix)) => Ok(prefix.clone()),
                // A clock of another object: a place through a reference
                // (FR-230-AC-10's `peer_clock`).
                Some(Argument::Value { .. }) => Err(not_checked(
                    TimedConstruct::ReferencedClock,
                    body.span(root),
                )),
                _ => Err(timed(TemporalCause::NotAPlaceExpression {
                    span: body.span(root),
                })),
            },
            _ => Err(timed(TemporalCause::NotAPlaceExpression {
                span: body.span(root),
            })),
        }
    }

    /// The arguments of a call written in a body, by parameter name of the
    /// called declaration.
    fn body_arguments(
        &self,
        frame: &Frame<'_, '_>,
        called: &ClockReadingDeclaration,
        arguments: &[ExprId],
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<BTreeMap<String, Argument>, CheckCause> {
        let body = frame.body;
        let mut bound = BTreeMap::new();
        for ((name, type_form), (slot, argument)) in called.function.parameters.iter().zip(
            called
                .parameters
                .iter()
                .copied()
                .zip(arguments.iter().copied()),
        ) {
            let read = match slot {
                ClockSlot::Clock => Argument::Places(self.places(frame, argument, stack, budget)?),
                ClockSlot::Bearing => {
                    if let Some(passed) = body
                        .parameter(self.declarations, argument)
                        .and_then(|parameter| frame.environment.get(parameter.name))
                    {
                        passed.clone()
                    } else {
                        let (root, steps) = body.path(argument).ok_or_else(|| {
                            timed(TemporalCause::NotAPlaceExpression {
                                span: body.span(argument),
                            })
                        })?;
                        let mut path = self.prefix(frame, root)?;
                        path.extend(steps);
                        let expected = self.declarations.record_type(type_form);
                        if expected.as_ref().map(|(_, key)| key)
                            != self.clocks.model.record_at(self.context, &path)
                        {
                            return Err(timed(TemporalCause::NotAPlaceExpression {
                                span: body.span(argument),
                            }));
                        }
                        Argument::Path(path)
                    }
                }
                ClockSlot::Data => {
                    if let Some(passed) = body
                        .parameter(self.declarations, argument)
                        .and_then(|parameter| frame.environment.get(parameter.name))
                    {
                        passed.clone()
                    } else if matches!(body.node(argument), Some(ExprNode::SelfRef))
                        && self.declarations.object_type(type_form).is_some()
                    {
                        Argument::Path(Vec::new())
                    } else {
                        let constant = match body.node(argument) {
                            Some(ExprNode::Quantity(quantity)) => self.clocks.quantity(quantity),
                            Some(ExprNode::Negate(operand))
                                if matches!(body.node(*operand), Some(ExprNode::Quantity(_))) =>
                            {
                                Err(TemporalCause::NegativeClockConstant {
                                    span: body.span(argument),
                                })
                            }
                            _ => Err(TemporalCause::ClockBoundNotConstant {
                                span: body.span(argument),
                            }),
                        };
                        Argument::Value {
                            term: self.formed(frame, argument)?,
                            constant,
                        }
                    }
                }
            };
            bound.insert(name.clone(), read);
        }
        Ok(bound)
    }

    /// Body node `id`, a data term, read at the call site: its parameters
    /// replaced by their arguments, located at the outermost call.
    ///
    /// # Errors
    ///
    /// `clock-read-outside-constraint` at the first clock read under `id`.
    fn formed(&self, frame: &Frame<'_, '_>, id: ExprId) -> Result<DataLiteral, CheckCause> {
        let body = frame.body;
        if let Some(read) = body.first_clock_read(id, self.declarations) {
            return Err(timed(TemporalCause::ClockReadOutsideConstraint {
                span: body.span(read),
            }));
        }
        // A bare data parameter is its argument.
        if let Some(Argument::Value { term, .. }) = body
            .parameter(self.declarations, id)
            .and_then(|parameter| frame.environment.get(parameter.name))
        {
            return Ok(term.clone());
        }
        let site_span = self.located.span(frame.site);
        let names: BTreeSet<&str> = frame.environment.keys().map(String::as_str).collect();
        let mut trees: Vec<(ExprId, Expression)> = Vec::new();
        for (occurrence, name) in free_occurrences(body.expression(), id, &names) {
            match frame.environment.get(name) {
                Some(Argument::Value { term, .. }) => trees.push((occurrence, self.term(term)?)),
                // `self.f…`: the receiver's value at the path.
                Some(Argument::Path(path)) => {
                    let mut tree = Expression::self_ref();
                    for step in path {
                        // An argument path names a single record or object,
                        // never a collection slot.
                        let PlaceStep::Field(field) = step else {
                            return Err(timed(TemporalCause::NotAPlaceExpression {
                                span: body.span(occurrence),
                            }));
                        };
                        tree = Expression::field(tree, field.clone());
                    }
                    trees.push((occurrence, tree));
                }
                // A clock parameter read as data was refused above.
                Some(Argument::Places(_)) | None => {
                    return Err(timed(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(occurrence),
                    }))
                }
            }
        }
        let grafts: BTreeMap<ExprId, &Expression> = trees
            .iter()
            .map(|(occurrence, tree)| (*occurrence, tree))
            .collect();
        let expression = body
            .expression()
            .graft(id, &grafts)
            .ok_or_else(|| timed(self.located.outside(frame.site)))?;
        let spans = ExpressionSpans::at_point(&expression, site_span.start)
            .map_err(|_| timed(self.located.outside(frame.site)))?;
        Ok(DataLiteral::Formed(Rc::new(ExpressionForm {
            expression,
            spans,
        })))
    }

    /// The expression a data term denotes.
    fn term(&self, term: &DataLiteral) -> Result<Expression, CheckCause> {
        match term {
            DataLiteral::Site(id) => self
                .located
                .form
                .cut(*id, &BTreeMap::new())
                .map(|form| form.expression)
                .ok_or_else(|| timed(self.located.outside(*id))),
            DataLiteral::Formed(form) | DataLiteral::Element(form, ..) => {
                Ok(form.expression.clone())
            }
        }
    }

    /// The places body node `id` denotes.
    fn places(
        &self,
        frame: &Frame<'_, '_>,
        id: ExprId,
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<Places, CheckCause> {
        let body = frame.body;
        let not_a_place = || {
            timed(TemporalCause::NotAPlaceExpression {
                span: body.span(id),
            })
        };
        match body.node(id) {
            Some(ExprNode::Name(_)) => match body
                .parameter(self.declarations, id)
                .and_then(|parameter| frame.environment.get(parameter.name))
            {
                Some(Argument::Places(places)) => Ok(places.clone()),
                _ => Err(not_a_place()),
            },
            Some(ExprNode::Field { .. }) => {
                let (root, steps) = body.path(id).ok_or_else(not_a_place)?;
                let mut path = self.prefix(frame, root)?;
                path.extend(steps);
                // A clock of another object: the leading reference fields
                // select the object holding it (FR-230-AC-10's `peer`).
                let (walked, holder) = self.clocks.model.through_references(self.context, &path);
                let (via, rest) = path.split_at(walked);
                let clock = self
                    .clocks
                    .fields
                    .iter()
                    .position(|clock| {
                        clock.path == rest && self.clocks.model.conforms(holder, clock.owner)
                    })
                    .map(ClockId)
                    .ok_or_else(not_a_place)?;
                Ok(vec![(
                    Vec::new(),
                    Place {
                        via: via.to_vec(),
                        clock,
                    },
                )])
            }
            Some(ExprNode::If {
                condition,
                then,
                otherwise,
            }) => {
                // The condition is data: a clock read there selects no place.
                let (selected, rejected) = self.boolean(frame, *condition, stack, budget)?;
                if selected
                    .iter()
                    .chain(&rejected)
                    .any(|(_, clocks)| !clocks.is_empty())
                {
                    return Err(timed(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(*condition),
                    }));
                }
                let then = self.places(frame, *then, stack, budget)?;
                let otherwise = self.places(frame, *otherwise, stack, budget)?;
                let mut places = Vec::new();
                for (condition, branch) in [(selected, then), (rejected, otherwise)] {
                    for (literals, _) in condition {
                        for (selecting, place) in &branch {
                            places.push((
                                literals.iter().chain(selecting).cloned().collect(),
                                place.clone(),
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
                    .ok_or_else(not_a_place)?;
                self.call(
                    frame,
                    id,
                    called,
                    arguments,
                    stack,
                    budget,
                    |inner, root, stack, budget| self.places(inner, root, stack, budget),
                )
            }
            _ => Err(not_a_place()),
        }
    }

    /// Read the call at body node `id` of `called`, with `read` over the
    /// called body, refusing recursion.
    #[allow(clippy::too_many_arguments)] // one call's whole reading state
    fn call<T>(
        &self,
        frame: &Frame<'_, '_>,
        id: ExprId,
        called: &ClockReadingDeclaration,
        arguments: &[ExprId],
        stack: &mut Stack,
        budget: &mut PartBudget,
        read: impl FnOnce(&Frame<'_, '_>, ExprId, &mut Stack, &mut PartBudget) -> Result<T, CheckCause>,
    ) -> Result<T, CheckCause> {
        let name = &called.function.name;
        if stack.contains(name) {
            // FR-230 does not say what a measured recursive clock-reading
            // declaration expands to (STD-493 item 19); an unmeasured one is
            // refused before expansion (`unmeasured_cycle`).
            return Err(not_checked(
                TimedConstruct::RecursiveClockReading,
                frame.body.span(id),
            ));
        }
        let environment = self.body_arguments(frame, called, arguments, stack, budget)?;
        let called_body = Body::of(called);
        let inner = Frame {
            body: &called_body,
            environment: &environment,
            site: frame.site,
        };
        stack.push(name.clone());
        let result = read(&inner, called_body.expression().root_id(), stack, budget);
        stack.pop();
        result
    }

    /// The constant body node `id` denotes as a bound.
    fn bound(&self, frame: &Frame<'_, '_>, id: ExprId) -> Result<Rational, CheckCause> {
        let body = frame.body;
        match body.node(id) {
            Some(ExprNode::Quantity(quantity)) => self.clocks.quantity(quantity).map_err(timed),
            Some(ExprNode::Name(_)) => match body
                .parameter(self.declarations, id)
                .and_then(|parameter| frame.environment.get(parameter.name))
            {
                Some(Argument::Value { constant, .. }) => constant.clone().map_err(timed),
                _ => Err(timed(TemporalCause::ClockBoundNotConstant {
                    span: body.span(id),
                })),
            },
            Some(ExprNode::Negate(_)) => Err(timed(TemporalCause::NegativeClockConstant {
                span: body.span(id),
            })),
            _ => Err(timed(TemporalCause::ClockBoundNotConstant {
                span: body.span(id),
            })),
        }
    }

    /// The disjunctive forms of Boolean body node `id`, read positively and
    /// negated: a Boolean combination of atomic clock constraints, calls of
    /// clock-reading predicates and data predicates.
    fn boolean(
        &self,
        frame: &Frame<'_, '_>,
        id: ExprId,
        stack: &mut Stack,
        budget: &mut PartBudget,
    ) -> Result<(Dnf, Dnf), CheckCause> {
        let body = frame.body;
        let truth = || vec![(Vec::new(), Vec::new())];
        match body.node(id) {
            Some(ExprNode::Boolean(true)) => Ok((truth(), Vec::new())),
            Some(ExprNode::Boolean(false)) => Ok((Vec::new(), truth())),
            Some(ExprNode::Not(operand)) => {
                let (positive, negative) = self.boolean(frame, *operand, stack, budget)?;
                Ok((negative, positive))
            }
            Some(ExprNode::Binary {
                operator:
                    operator @ (BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies),
                left,
                right,
            }) if body.first_clock_read(id, self.declarations).is_some() => {
                let (lp, ln) = self.boolean(frame, *left, stack, budget)?;
                let (rp, rn) = self.boolean(frame, *right, stack, budget)?;
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
            }) if body.is_place(*left, self.declarations)
                || body.is_place(*right, self.declarations) =>
            {
                let Some(op) = ClockOp::of(*operator) else {
                    return Err(timed(TemporalCause::ClockReadOutsideConstraint {
                        span: body.span(id),
                    }));
                };
                let (place, bound, op) = if body.is_place(*left, self.declarations) {
                    (*left, *right, op)
                } else {
                    (*right, *left, op.mirrored())
                };
                let places = self.places(frame, place, stack, budget)?;
                let bound = self.bound(frame, bound)?;
                Ok(constraints(
                    places,
                    op,
                    &bound,
                    self.located.span(frame.site),
                ))
            }
            Some(ExprNode::Call { name, arguments })
                if self
                    .declarations
                    .named(name)
                    .is_some_and(|called| called.result != ClockSlot::Clock) =>
            {
                let Some(called) = self.declarations.named(name) else {
                    return Err(timed(self.located.outside(frame.site)));
                };
                self.call(
                    frame,
                    id,
                    called,
                    arguments,
                    stack,
                    budget,
                    |inner, root, stack, budget| self.boolean(inner, root, stack, budget),
                )
            }
            // A data predicate: read at the call site.
            _ => {
                let literal = self.formed(frame, id)?;
                Ok((
                    vec![(vec![(literal.clone(), true)], Vec::new())],
                    vec![(vec![(literal, false)], Vec::new())],
                ))
            }
        }
    }
}

/// `places`, each selected also by `literal`.
fn guarded(places: Places, literal: (DataLiteral, bool)) -> Places {
    places
        .into_iter()
        .map(|(mut literals, clock)| {
            literals.insert(0, literal.clone());
            (literals, clock)
        })
        .collect()
}

/// The disjunctive forms of `place ~ bound` over `places`, read positively
/// and negated. The places' selecting conditions are exclusive and
/// exhaustive (each place expression's branches partition its conditions),
/// so the negation is the same places with each constraint negated.
pub(super) fn constraints(places: Places, op: ClockOp, bound: &Rational, span: Span) -> (Dnf, Dnf) {
    let constraint = |place: &Place, op: ClockOp| ClockConstraint {
        clock: place.clock,
        via: place.via.clone(),
        slots: Vec::new(),
        op,
        bound: bound.clone(),
        span,
    };
    let positive = places
        .iter()
        .map(|(literals, place)| (literals.clone(), vec![constraint(place, op)]))
        .collect();
    let negative = places
        .iter()
        .flat_map(|(literals, place)| {
            op.negated()
                .into_iter()
                .map(move |negated| (literals.clone(), vec![constraint(place, negated)]))
        })
        .collect();
    (positive, negative)
}

/// The span of the first call of a clock-reading declaration in
/// `expression`, which no clock rule admits there.
pub(super) fn first_call(
    declarations: &Declarations<'_>,
    expression: &Expression,
    spans: &ExpressionSpans,
) -> Option<Span> {
    let spans = spans.of_nodes(expression)?;
    expression
        .iter()
        .filter(|node| declarations.is_call(expression, node.id()))
        .filter_map(|node| spans.get(node.id().index()).copied())
        .min_by_key(|span| span.start)
}

/// The region site a clock-reading declaration's name is registered under,
/// which locates a refusal of the declaration as a whole.
pub(crate) fn site(name: &str) -> String {
    format!("clock-reading {name}")
}

/// The first cycle of calls among `declarations` (FR-146's call graph over
/// them) with a member written without `decreases`, as the member names
/// from the first member back to it, and the name of its first member: a
/// recursive declaration with no measure (FR-091-AC-30 `missing-measure`).
pub(crate) fn unmeasured_cycle(
    declarations: &[ClockReadingDeclaration],
) -> Option<(Vec<String>, &str)> {
    let index: BTreeMap<&str, usize> = declarations
        .iter()
        .enumerate()
        .map(|(position, declaration)| (declaration.function.name.as_str(), position))
        .collect();
    let calls: Vec<Vec<usize>> = declarations
        .iter()
        .map(|declaration| {
            let body = &declaration.function.body;
            let mut called: Vec<usize> = body
                .iter()
                .filter_map(|node| match node.node() {
                    ExprNode::Call { name, .. } if name.segments.len() == 1 => {
                        index.get(name.name.as_str()).copied()
                    }
                    _ => None,
                })
                .collect();
            called.sort_unstable();
            called.dedup();
            called
        })
        .collect();
    for (start, declaration) in declarations.iter().enumerate() {
        // The shortest call path from `start` back to itself.
        let mut previous: BTreeMap<usize, usize> = BTreeMap::new();
        let mut pending = std::collections::VecDeque::from([start]);
        let mut closing = None;
        'walk: while let Some(at) = pending.pop_front() {
            for &next in calls.get(at).into_iter().flatten() {
                if next == start {
                    closing = Some(at);
                    break 'walk;
                }
                if previous.contains_key(&next) {
                    continue;
                }
                previous.insert(next, at);
                pending.push_back(next);
            }
        }
        let Some(mut at) = closing else {
            continue;
        };
        let mut members = vec![start];
        let mut back = Vec::new();
        while at != start {
            back.push(at);
            at = previous.get(&at).copied().unwrap_or(start);
        }
        members.extend(back.into_iter().rev());
        members.push(start);
        let unmeasured = members.iter().any(|member| {
            declarations
                .get(*member)
                .is_some_and(|member| member.function.measure.is_none())
        });
        if unmeasured {
            let names = members
                .iter()
                .filter_map(|member| declarations.get(*member))
                .map(|member| member.function.name.clone())
                .collect();
            return Some((names, declaration.function.name.as_str()));
        }
    }
    None
}

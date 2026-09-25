// SPDX-License-Identifier: AGPL-3.0-or-later
//! The typing loop (QSL-228): [`Typer::check_as`] and [`Typer::infer`] type
//! an expression on an explicit heap stack, not the host stack.
//!
//! A form whose operands are still to type is a [`Frame`] on that stack.
//! The loop either descends into the next operand ([`Goal`]) or hands the
//! node it just typed to the frame below, which types its next operand or
//! finishes its own node. The host stack a check needs is therefore the same
//! at every nesting depth: an expression nested to the maximum checking
//! depth checks, and one nested deeper refuses on the depth limit, on a
//! small thread in a debug build.
//!
//! Operands are typed in the order the recursive typer before QSL-228 typed
//! them, and every depth check, node charge, binding, formed unit and
//! refusal happens at the same point of that order, so every checked tree,
//! slot and refusal is that typer's. A nesting level `Typer::enter` entered
//! is left by a [`Frame::Leave`] once the form above it is typed.

use std::borrow::Cow;

use super::{
    catalogued_step, coerce, contains_pre_eligible_read, contextual, ineligible, is_integer,
    mismatch, node, refuse, LocalKind, Typer,
};
use crate::check::family::Application;
use crate::check::ir::{Connective, Node, NodeKind, RecordSlot, Slot, Visit};
use crate::check::refusal::{CheckCause, CheckRefusal, Location, WrongSnapshotCause};
use crate::check::DispatchOperation;
use crate::value::declaration::{EqualityOperand, EqualityOperator, FieldDeclaration};
use qsl_forms::{
    Accumulation, BinaryOperator, BinderQuery, ClauseKind, Expression, FieldInitializer,
};
use qsl_foundation::absence::AbsenceMode;
use quire_exact::{
    ArithmeticOperator, CollectionType, NodeKey, OrderingOperator, Value, ValueType,
};

/// An operand's expected type or hint: borrowed from the declarations or
/// the caller, or owned when an operand typed before it formed it.
type Want<'r> = Cow<'r, ValueType>;

/// The type a condition, a connective's operand, a filter, a quantifier or
/// a count predicate is checked against.
static BOOLEAN: ValueType = ValueType::Boolean;

fn boolean<'r>() -> Want<'r> {
    Cow::Borrowed(&BOOLEAN)
}

/// Where a typed operand is admitted.
enum Expected<'r> {
    /// Where this type is expected ([`Typer::check_as`]): an integer is
    /// coerced into it.
    Checked(Want<'r>),
    /// As a dispatch call's argument of this declared parameter type:
    /// identical, or a reference upcast.
    Argument(Want<'r>),
}

impl Expected<'_> {
    fn into_type(self) -> ValueType {
        match self {
            Self::Checked(required) | Self::Argument(required) => required.into_owned(),
        }
    }
}

/// How a conditional's branches or a `let`'s body is typed.
enum Branches<'r> {
    /// Admitted where expected, like the conditional or `let` itself.
    Expected(Expected<'r>),
    /// Inferred, with this hint for a contextual literal.
    Inferred(Option<Want<'r>>),
}

impl<'r> Branches<'r> {
    /// The goal of typing `expression`, a branch or body, at `location`.
    fn goal<'e>(&self, expression: &'e Expression, location: Location) -> Goal<'e, 'r> {
        match self {
            Self::Expected(Expected::Checked(required)) => {
                Goal::Expect(expression, Expected::Checked(required.clone()), location)
            }
            Self::Expected(Expected::Argument(required)) => {
                Goal::Expect(expression, Expected::Argument(required.clone()), location)
            }
            Self::Inferred(hint) => Goal::Infer(expression, hint.clone(), location),
        }
    }
}

/// Typing one sub-expression.
enum Goal<'e, 'r> {
    /// Type it where a type is expected.
    Expect(&'e Expression, Expected<'r>, Location),
    /// Infer its type ([`Typer::infer`]), with a hint for a contextual
    /// literal.
    Infer(&'e Expression, Option<Want<'r>>, Location),
}

/// What the loop does next.
#[allow(
    clippy::large_enum_variant,
    reason = "Step is a per-iteration return value, never stored in a Vec; boxing its typed Node would add an allocation per checked node"
)]
enum Step<'e, 'r> {
    /// Type a sub-expression.
    Descend(Goal<'e, 'r>),
    /// Hand this typed node to the frame below.
    Typed(Node),
}

/// A form waiting on the node of one of its operands.
enum Frame<'e, 'r> {
    /// Leave the nesting level entered for the form above this frame.
    Leave,
    /// Admit the typed node where this type is expected ([`coerce`]).
    Coerce(Want<'r>),
    /// Admit the typed node as a dispatch argument of this parameter type,
    /// refusing at this location.
    Upcast(Want<'r>, Location),
    If(Box<IfFrame<'e, 'r>>),
    Let(Box<LetFrame<'e, 'r>>),
    Connective(Box<ConnectiveFrame<'e>>),
    Peer(Box<PeerFrame<'e, 'r>>),
    Unary(Box<UnaryFrame<'r>>),
    /// `e.f`: the field name and the form's location.
    Field(&'e str, Location),
    Attribute(Box<AttributeFrame<'e>>),
    Application(Box<ApplicationFrame<'e, 'r>>),
    Record(Box<RecordFrame<'e, 'r>>),
    Collection(Box<CollectionFrame<'e>>),
    Query(Box<QueryFrame<'e>>),
    Accumulate(Box<AccumulateFrame<'e>>),
    Tally(Box<TallyFrame<'e>>),
    Contains(Box<ContainsFrame<'e>>),
    Lookup(Box<LookupFrame<'e>>),
    Dispatch(Box<DispatchFrame<'e>>),
    DispatchArguments(Box<DispatchArgumentsFrame<'e, 'r>>),
}

/// `if condition then then else otherwise`.
struct IfFrame<'e, 'r> {
    branches: Branches<'r>,
    location: Location,
    stage: IfStage<'e>,
}

#[allow(
    clippy::large_enum_variant,
    reason = "this stage lives inside an already boxed Frame; boxing its Node payload would add a second allocation per node"
)]
enum IfStage<'e> {
    Condition {
        then: &'e Expression,
        otherwise: &'e Expression,
    },
    Then {
        condition: Node,
        otherwise: &'e Expression,
    },
    Otherwise {
        condition: Node,
        then: Node,
    },
}

/// `let name = value in body`.
struct LetFrame<'e, 'r> {
    branches: Branches<'r>,
    name: &'e str,
    body: &'e Expression,
    location: Location,
    /// The typed value and the slot it is bound to, once typed.
    value: Option<(Node, Slot)>,
}

/// Which operand of a pair is being typed.
enum Pair<'e, T> {
    /// The first; the other is still to type, at this location.
    First(&'e Expression, Location),
    /// The second; the first is typed.
    Second(T),
}

/// `left and right`, `left or right` or `left implies right`.
struct ConnectiveFrame<'e> {
    connective: Connective,
    location: Location,
    pair: Pair<'e, Node>,
}

/// An arithmetic, ordering or equality operator: the non-contextual
/// operand is typed first, so a literal can take its peer's type.
struct PeerFrame<'e, 'r> {
    form: PeerForm<'e, 'r>,
    /// Whether the right operand is typed first.
    right_first: bool,
    location: Location,
}

/// A binary operator typed as an operand pair.
enum PeerOperator {
    Arithmetic(ArithmeticOperator),
    Ordering(OrderingOperator),
    Equality(EqualityOperator),
}

#[allow(
    clippy::large_enum_variant,
    reason = "this stage lives inside an already boxed Frame; boxing its Node payload would add a second allocation per node"
)]
enum PeerForm<'e, 'r> {
    Arithmetic {
        operator: ArithmeticOperator,
        hint: Option<Want<'r>>,
        pair: Pair<'e, Node>,
    },
    Ordering {
        operator: OrderingOperator,
        pair: Pair<'e, Node>,
    },
    Equality {
        operator: EqualityOperator,
        pair: Pair<'e, (Node, EqualityOperand)>,
        /// The target and location of the operand being typed, when it is
        /// a non-collection `convert<T>(e)` compared as converted.
        converted: Option<(ValueType, Location)>,
    },
}

/// A form over one operand whose node it finishes once the operand is
/// typed.
struct UnaryFrame<'r> {
    form: Unary<'r>,
    location: Location,
}

enum Unary<'r> {
    Negate(Option<Want<'r>>),
    Not,
    Present,
    Value,
    Deref,
    Pre,
    Flatten,
    Size,
    Convert(ValueType),
    AllInstances(ValueType),
}

/// `deref(r).f`, its `deref(r)` operand at `operand_location`.
struct AttributeFrame<'e> {
    field: &'e str,
    operand_location: Location,
    location: Location,
}

/// A function or tuple-constructor call.
struct ApplicationFrame<'e, 'r> {
    application: Application<'r>,
    arguments: &'e [Expression],
    location: Location,
    typed: Vec<Node>,
}

/// A record literal, admitting its initializers in source order.
struct RecordFrame<'e, 'r> {
    key: NodeKey,
    declared: &'r [FieldDeclaration],
    fields: &'e [(String, FieldInitializer)],
    /// The next initializer to admit.
    next: usize,
    /// The next value initializer's location child index.
    child: usize,
    supplied: Vec<Option<RecordSlot>>,
    /// The declared field the operand being typed fills.
    awaiting: Option<usize>,
    location: Location,
}

/// A collection literal, its elements checked against its element type.
struct CollectionFrame<'e> {
    collection_type: CollectionType,
    elements: &'e [Expression],
    location: Location,
    typed: Vec<Node>,
}

/// A one-binder query `q(binder in source: body)`.
struct QueryFrame<'e> {
    query: BinderQuery,
    binder: &'e str,
    body: &'e Expression,
    location: Location,
    /// The typed source, the binder's slot and the source's collection
    /// type, once the source is typed.
    source: Option<(Node, Slot, Box<CollectionType>)>,
}

/// `fold<A>(acc, x in c: step, identity: i)` or `reduce<A>(acc, x in c:
/// step)`.
struct AccumulateFrame<'e> {
    form: Accumulation,
    value_type: ValueType,
    accumulator: &'e str,
    binder: &'e str,
    step: &'e Expression,
    identity: Option<&'e Expression>,
    location: Location,
    stage: AccumulateStage,
}

#[allow(
    clippy::large_enum_variant,
    reason = "this stage lives inside an already boxed Frame; boxing its Node payload would add a second allocation per node"
)]
enum AccumulateStage {
    Source,
    Step {
        source: Node,
        accumulator: Slot,
        binder: Slot,
        source_type: Box<CollectionType>,
    },
    Identity(Fold),
}

/// A `fold`/`reduce` whose source and step are typed.
struct Fold {
    source: Node,
    step: Node,
    accumulator: Slot,
    binder: Slot,
    source_type: Box<CollectionType>,
    /// Whether the step is in the FR-145 set-and-bag catalog.
    catalogued: bool,
}

/// A counting or summing query.
#[derive(Clone, Copy)]
enum Tally {
    /// `count<N>(x in c: p)`.
    Count,
    /// `sum<N>(x in c: e)`.
    Sum,
}

/// `count<N>(x in c: p)` or `sum<N>(x in c: e)`.
struct TallyFrame<'e> {
    tally: Tally,
    value_type: ValueType,
    binder: &'e str,
    body: &'e Expression,
    location: Location,
    /// The typed source and the binder's slot, once the source is typed.
    source: Option<(Node, Slot)>,
}

/// `contains(c, v)`.
struct ContainsFrame<'e> {
    item: &'e Expression,
    location: Location,
    /// The typed collection and its element type, once typed.
    collection: Option<(Node, ValueType)>,
}

/// `lookup<T>(p, r) absent m`.
struct LookupFrame<'e> {
    target: ValueType,
    reference: &'e Expression,
    absence: AbsenceMode,
    location: Location,
    population: Option<Node>,
}

/// `receiver.member(args)`, its receiver being typed.
struct DispatchFrame<'e> {
    member: &'e str,
    arguments: &'e [Expression],
    location: Location,
}

/// `receiver.member(args)`, its receiver typed and resolved to an
/// operation, its arguments being typed.
struct DispatchArgumentsFrame<'e, 'r> {
    receiver: Node,
    operation_index: usize,
    operation: &'r DispatchOperation,
    arguments: &'e [Expression],
    location: Location,
    typed: Vec<Node>,
}

/// `(first, second)` in source order: `second` is the left operand when
/// the right one was typed first.
fn ordered<T>(right_first: bool, first: T, second: T) -> (T, T) {
    if right_first {
        (second, first)
    } else {
        (first, second)
    }
}

impl<'a> Typer<'a> {
    /// Type `expression` where `required` is expected. A conditional or `let`
    /// passes the requirement into its branches or body.
    pub(crate) fn check_as(
        &mut self,
        expression: &Expression,
        required: &ValueType,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        self.run(Goal::Expect(
            expression,
            Expected::Checked(Cow::Borrowed(required)),
            location.clone(),
        ))
    }

    /// Type `expression`, with `hint` as the expected type of a contextual
    /// literal.
    pub(crate) fn infer(
        &mut self,
        expression: &Expression,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        self.run(Goal::Infer(
            expression,
            hint.map(Cow::Borrowed),
            location.clone(),
        ))
    }

    /// The loop: descend into the next goal, or hand the node just typed to
    /// the frame below; the node no frame waits on is the root's.
    fn run<'e, 'r>(&mut self, goal: Goal<'e, 'r>) -> Result<Node, CheckRefusal>
    where
        'a: 'r,
    {
        let mut frames: Vec<Frame<'e, 'r>> = Vec::new();
        let mut step = Step::Descend(goal);
        loop {
            step = match step {
                Step::Descend(goal) => self.descend(goal, &mut frames)?,
                Step::Typed(typed) => match frames.pop() {
                    None => return Ok(typed),
                    Some(frame) => self.accept(frame, typed, &mut frames)?,
                },
            };
        }
    }

    /// Start `goal`: a form with no operand to type is typed now; any other
    /// pushes its frame and descends into its first operand.
    fn descend<'e, 'r>(
        &mut self,
        goal: Goal<'e, 'r>,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal>
    where
        'a: 'r,
    {
        match goal {
            Goal::Expect(expression, expected, location) => {
                self.expect(expression, expected, location, frames)
            }
            Goal::Infer(expression, hint, location) => {
                self.enter(&location)?;
                frames.push(Frame::Leave);
                self.infer_form(expression, hint, location, frames)
            }
        }
    }

    /// Start typing `expression` where `expected`: a conditional or `let`
    /// passes the expectation into its branches or body; any other form is
    /// inferred, hinted with the expected type, and then admitted.
    fn expect<'e, 'r>(
        &mut self,
        expression: &'e Expression,
        expected: Expected<'r>,
        location: Location,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal>
    where
        'a: 'r,
    {
        match expression {
            Expression::If {
                condition,
                then,
                otherwise,
            } => {
                self.enter(&location)?;
                frames.push(Frame::Leave);
                Ok(Self::conditional(
                    Branches::Expected(expected),
                    (condition, then, otherwise),
                    location,
                    frames,
                ))
            }
            Expression::Let { name, value, body } => {
                self.enter(&location)?;
                frames.push(Frame::Leave);
                Ok(Self::binding(
                    Branches::Expected(expected),
                    (name, value, body),
                    location,
                    frames,
                ))
            }
            _ => {
                let hint = match expected {
                    Expected::Checked(required) => {
                        frames.push(Frame::Coerce(required.clone()));
                        required
                    }
                    Expected::Argument(required) => {
                        frames.push(Frame::Upcast(required.clone(), location.clone()));
                        required
                    }
                };
                Ok(Step::Descend(Goal::Infer(expression, Some(hint), location)))
            }
        }
    }

    /// Start `if condition then then else otherwise`.
    fn conditional<'e, 'r>(
        branches: Branches<'r>,
        (condition, then, otherwise): (&'e Expression, &'e Expression, &'e Expression),
        location: Location,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Step<'e, 'r> {
        let condition_location = location.child(0);
        frames.push(Frame::If(Box::new(IfFrame {
            branches,
            location,
            stage: IfStage::Condition { then, otherwise },
        })));
        Step::Descend(Goal::Expect(
            condition,
            Expected::Checked(boolean()),
            condition_location,
        ))
    }

    /// Start `let name = value in body`.
    fn binding<'e, 'r>(
        branches: Branches<'r>,
        (name, value, body): (&'e str, &'e Expression, &'e Expression),
        location: Location,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Step<'e, 'r> {
        let value_location = location.child(0);
        frames.push(Frame::Let(Box::new(LetFrame {
            branches,
            name,
            body,
            location,
            value: None,
        })));
        Step::Descend(Goal::Infer(value, None, value_location))
    }

    /// Start a form over one operand, `goal`.
    fn unary<'e, 'r>(
        form: Unary<'r>,
        goal: Goal<'e, 'r>,
        location: Location,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Step<'e, 'r> {
        frames.push(Frame::Unary(Box::new(UnaryFrame { form, location })));
        Step::Descend(goal)
    }

    /// FR-065's dispatch seam over [`Expression`] (ADR-012 §4.3): start
    /// inferring `expression`'s type, its nesting level already entered.
    ///
    /// **`Expression::Call` is thin (FR-065-CON-3).** Its arm makes one call
    /// into [`Application::resolve`] -- `Value`'s application check -- and
    /// holds no name, arity or argument-type logic of its own. Each argument
    /// is then checked against [`Application::parameter`] under this typer's
    /// clause kind, and the node is built by [`Application::finish`].
    /// FR-065-AC-4 states the verdicts that check gives; FR-065-AC-5 states
    /// that a call gets the same verdict from a declaration body, a
    /// `decreases` measure and a clause expression.
    ///
    /// `Expression::Call` is `Value`'s variant of the one [`Expression`]
    /// enum: its owning family is the family whose check its arm calls
    /// (ADR-012 §4.3).
    ///
    /// `#[deny(...)]` (FR-063's residual paragraph, carried into the
    /// QSL-25 implementation by owner ruling): a future change that wants
    /// to delete an arm from this `match` cannot restore exhaustiveness
    /// with a `_ => ...` catch-all -- that is the exact "escape hatch"
    /// closed here, not merely by convention; what this attribute forbids
    /// is silently absorbing a *future* removed arm behind a catch-all
    /// instead of deleting the corresponding variant.
    #[deny(clippy::wildcard_enum_match_arm)]
    #[deny(clippy::match_wildcard_for_single_variants)]
    fn infer_form<'e, 'r>(
        &mut self,
        expression: &'e Expression,
        hint: Option<Want<'r>>,
        location: Location,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal>
    where
        'a: 'r,
    {
        let typed = match expression {
            Expression::Boolean(value) => node(
                NodeKind::Literal(Value::Boolean(*value)),
                ValueType::Boolean,
                &location,
            ),
            Expression::Integer(value) => node(
                NodeKind::Literal(Value::Integer(value.clone())),
                ValueType::Integer,
                &location,
            ),
            Expression::Rational(numerator, denominator) => {
                self.rational_literal(numerator, denominator, hint.as_deref(), &location)?
            }
            Expression::Name(name) => self.name(name, &location)?,
            Expression::Let { name, value, body } => {
                return Ok(Self::binding(
                    Branches::Inferred(hint),
                    (name, value, body),
                    location,
                    frames,
                ));
            }
            Expression::If {
                condition,
                then,
                otherwise,
            } => {
                return Ok(Self::conditional(
                    Branches::Inferred(hint),
                    (condition, then, otherwise),
                    location,
                    frames,
                ));
            }
            Expression::Binary {
                operator,
                left,
                right,
            } => return self.binary(*operator, (left, right), hint, location, frames),
            Expression::Negate(operand) => {
                let goal = Goal::Infer(operand, None, location.child(0));
                return Ok(Self::unary(Unary::Negate(hint), goal, location, frames));
            }
            Expression::Not(operand) => {
                let goal = Goal::Expect(operand, Expected::Checked(boolean()), location.child(0));
                return Ok(Self::unary(Unary::Not, goal, location, frames));
            }
            Expression::Field { operand, field } => {
                let operand_location = location.child(0);
                if let Expression::Deref(reference) = &**operand {
                    self.enter(&operand_location)?;
                    let reference_location = operand_location.child(0);
                    frames.push(Frame::Attribute(Box::new(AttributeFrame {
                        field,
                        operand_location,
                        location,
                    })));
                    frames.push(Frame::Leave);
                    return Ok(Step::Descend(Goal::Infer(
                        reference,
                        None,
                        reference_location,
                    )));
                }
                frames.push(Frame::Field(field, location));
                return Ok(Step::Descend(Goal::Infer(operand, None, operand_location)));
            }
            Expression::Present(operand) => {
                let goal = Goal::Infer(operand, None, location.child(0));
                return Ok(Self::unary(Unary::Present, goal, location, frames));
            }
            Expression::Value(operand) => {
                let goal = Goal::Infer(operand, None, location.child(0));
                return Ok(Self::unary(Unary::Value, goal, location, frames));
            }
            Expression::Deref(operand) => {
                // Only `deref(r).f` is a value.
                let goal = Goal::Infer(operand, None, location.child(0));
                return Ok(Self::unary(Unary::Deref, goal, location, frames));
            }
            Expression::Pre(operand) => {
                // FR-153/FR-042 (FR-208 applies FR-042 to invariants and
                // preconditions, naming this same cause explicitly): `pre(...)`
                // is legal only in an operation's postcondition -- every other
                // clause kind (a function body, its measure, an invariant, a
                // precondition, or a bare `check_expression` call) refuses it
                // here, before looking at `operand` at all, as
                // `wrong_snapshot`/`forbidden-pre-read`. `wrong-anchor` is a
                // different, catalogued cause for a different case this
                // checker cannot see: a postcondition it does admit, evaluated
                // at runtime over a population with no attached pre anchor
                // (`evaluate.rs`'s `select_anchor`) -- never a `pre(...)`
                // written in the wrong clause.
                if self.clause_kind != ClauseKind::Postcondition {
                    return Err(refuse(
                        &location,
                        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead),
                    ));
                }
                // FR-042's Behavior clause: "Pre is refused ... on bare
                // parameters/constants/captures". `operand` must contain, in
                // its own syntax, at least one form `pre(...)`'s anchor can
                // actually act on -- see `contains_pre_eligible_read`'s own
                // doc for exactly which forms count and why a bare `Name`
                // (a parameter, or a `let` bound outside this very
                // `pre(...)`) never does.
                if !contains_pre_eligible_read(operand) {
                    return Err(refuse(
                        &location,
                        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead),
                    ));
                }
                // FR-042-AC-3's `let s = self in pre(s.version)` analogue:
                // an eligible read this operand does contain must not take
                // its population operand from a `let` local captured before
                // this `pre(...)` was reached -- see
                // `contains_captured_pre_alias`'s own doc.
                if self.contains_captured_pre_alias(operand, self.slots) {
                    return Err(refuse(
                        &location,
                        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead),
                    ));
                }
                // FR-153: `pre(e)` is identity-typed; only the anchor that
                // `allInstances`/`lookup` read underneath it changes.
                let goal = Goal::Infer(operand, hint, location.child(0));
                return Ok(Self::unary(Unary::Pre, goal, location, frames));
            }
            Expression::Call { name, arguments } => {
                let application = Application::resolve(self, name, arguments.len(), &location)?;
                return Ok(Self::application(
                    Box::new(ApplicationFrame {
                        application,
                        arguments,
                        location,
                        typed: Vec::with_capacity(arguments.len()),
                    }),
                    frames,
                ));
            }
            Expression::Record { name, fields } => {
                let (key, declared) = self.record_declaration(name, &location)?;
                return Self::record_step(
                    Box::new(RecordFrame {
                        key,
                        declared,
                        fields,
                        next: 0,
                        child: 0,
                        supplied: declared.iter().map(|_| None).collect(),
                        awaiting: None,
                        location,
                    }),
                    frames,
                );
            }
            Expression::Collection { kind, elements } => {
                let collection_type = Self::collection_type(*kind, hint.as_deref(), &location)?;
                return Ok(Self::collection(
                    Box::new(CollectionFrame {
                        collection_type,
                        elements,
                        location,
                        typed: Vec::with_capacity(elements.len()),
                    }),
                    frames,
                ));
            }
            Expression::Convert { target, operand } => {
                let target = self.declared_target(target, &location)?;
                let goal = Goal::Infer(operand, None, location.child(0));
                return Ok(Self::unary(Unary::Convert(target), goal, location, frames));
            }
            Expression::Query {
                query,
                binder,
                source,
                body,
            } => {
                let source_location = location.child(0);
                frames.push(Frame::Query(Box::new(QueryFrame {
                    query: *query,
                    binder,
                    body,
                    location,
                    source: None,
                })));
                return Ok(Step::Descend(Goal::Infer(source, None, source_location)));
            }
            Expression::Flatten(source) => {
                let goal = Goal::Infer(source, None, location.child(0));
                return Ok(Self::unary(Unary::Flatten, goal, location, frames));
            }
            Expression::Accumulate {
                accumulator_type_span: _,
                form,
                accumulator_type,
                accumulator,
                binder,
                source,
                step,
                identity,
            } => {
                let value_type = self.type_named(accumulator_type, &location)?;
                let source_location = location.child(0);
                frames.push(Frame::Accumulate(Box::new(AccumulateFrame {
                    form: *form,
                    value_type,
                    accumulator,
                    binder,
                    step,
                    identity: identity.as_deref(),
                    location,
                    stage: AccumulateStage::Source,
                })));
                return Ok(Step::Descend(Goal::Infer(source, None, source_location)));
            }
            Expression::Count {
                result_type_span: _,
                result_type,
                binder,
                source,
                predicate,
            } => {
                return self.tally(
                    Tally::Count,
                    (result_type, binder),
                    (source, predicate),
                    location,
                    frames,
                )
            }
            Expression::Sum {
                result_type_span: _,
                result_type,
                binder,
                source,
                summand,
            } => {
                return self.tally(
                    Tally::Sum,
                    (result_type, binder),
                    (source, summand),
                    location,
                    frames,
                )
            }
            Expression::Size(operand) => {
                let goal = Goal::Infer(operand, None, location.child(0));
                return Ok(Self::unary(Unary::Size, goal, location, frames));
            }
            Expression::Contains { collection, item } => {
                let collection_location = location.child(0);
                frames.push(Frame::Contains(Box::new(ContainsFrame {
                    item,
                    location,
                    collection: None,
                })));
                return Ok(Step::Descend(Goal::Infer(
                    collection,
                    None,
                    collection_location,
                )));
            }
            Expression::AllInstances { target, population } => {
                let target = self.population_target(target, &location)?;
                let goal = Goal::Infer(population, None, location.child(0));
                return Ok(Self::unary(
                    Unary::AllInstances(target),
                    goal,
                    location,
                    frames,
                ));
            }
            Expression::Lookup {
                target,
                population,
                reference,
                absence,
            } => {
                let target = self.population_target(target, &location)?;
                let population_location = location.child(0);
                frames.push(Frame::Lookup(Box::new(LookupFrame {
                    target,
                    reference,
                    absence: *absence,
                    location,
                    population: None,
                })));
                return Ok(Step::Descend(Goal::Infer(
                    population,
                    None,
                    population_location,
                )));
            }
            Expression::Dispatch {
                receiver,
                member,
                arguments,
            } => {
                self.dispatch_admitted(&location)?;
                let receiver_location = location.child(0);
                let frame = Frame::Dispatch(Box::new(DispatchFrame {
                    member,
                    arguments,
                    location,
                }));
                if let Expression::Deref(inner) = &**receiver {
                    self.enter(&receiver_location)?;
                    frames.push(frame);
                    frames.push(Frame::Leave);
                    let inner_location = receiver_location.child(0);
                    return Ok(Step::Descend(Goal::Infer(inner, None, inner_location)));
                }
                frames.push(frame);
                return Ok(Step::Descend(Goal::Infer(
                    receiver,
                    None,
                    receiver_location,
                )));
            }
            // FR-063/S2 seam (QSL-143): no arm for `Expression::__SeamProbe`
            // under `--cfg seam_probe` alone -- this match is deliberately
            // non-exhaustive (`E0004`) in `xtask seam-probe`'s build of
            // `qsl-semantics`, the seam probe's evidence for "the check seam
            // over the parsed form enum" (ADR-012 §5.1 row S2). Do not add a
            // catch-all to make it compile (this function's own
            // `#[deny(clippy::wildcard_enum_match_arm)]` already forbids
            // that for every other arm).
            //
            // The arm below exists only in the probe's build of the crates
            // that depend on this one (`--cfg seam_probe --cfg
            // seam_probe_downstream`), the same shape `FamilyKind::
            // catalog_code_prefix`'s own doc explains.
            #[cfg(seam_probe_downstream)]
            Expression::__SeamProbe => unreachable!("never constructed outside the probe build"),
        };
        Ok(Step::Typed(typed))
    }

    /// Start `left op right`: a connective checks both operands Boolean;
    /// any other operator types its operands as a pair.
    fn binary<'e, 'r>(
        &mut self,
        operator: BinaryOperator,
        (left, right): (&'e Expression, &'e Expression),
        hint: Option<Want<'r>>,
        location: Location,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal>
    where
        'a: 'r,
    {
        let (left_location, right_location) = (location.child(0), location.child(1));
        let peer = match operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide => PeerOperator::Arithmetic(match operator {
                BinaryOperator::Add => ArithmeticOperator::Add,
                BinaryOperator::Subtract => ArithmeticOperator::Subtract,
                BinaryOperator::Multiply => ArithmeticOperator::Multiply,
                BinaryOperator::Divide => ArithmeticOperator::Divide,
                _ => return Err(ineligible(&location)),
            }),
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                PeerOperator::Equality(if operator == BinaryOperator::Equal {
                    EqualityOperator::Equal
                } else {
                    EqualityOperator::NotEqual
                })
            }
            BinaryOperator::Less
            | BinaryOperator::LessOrEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterOrEqual => PeerOperator::Ordering(match operator {
                BinaryOperator::Less => OrderingOperator::Less,
                BinaryOperator::LessOrEqual => OrderingOperator::LessOrEqual,
                BinaryOperator::Greater => OrderingOperator::Greater,
                BinaryOperator::GreaterOrEqual => OrderingOperator::GreaterOrEqual,
                _ => return Err(ineligible(&location)),
            }),
            BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies => {
                let connective = match operator {
                    BinaryOperator::And => Connective::And,
                    BinaryOperator::Or => Connective::Or,
                    BinaryOperator::Implies => Connective::Implies,
                    _ => return Err(ineligible(&location)),
                };
                frames.push(Frame::Connective(Box::new(ConnectiveFrame {
                    connective,
                    location,
                    pair: Pair::First(right, right_location),
                })));
                return Ok(Step::Descend(Goal::Expect(
                    left,
                    Expected::Checked(boolean()),
                    left_location,
                )));
            }
        };
        let right_first = contextual(left) && !contextual(right);
        let ((first, first_location), (second, second_location)) =
            ordered(right_first, (left, left_location), (right, right_location));
        let (form, goal) = match peer {
            PeerOperator::Arithmetic(operator) => (
                PeerForm::Arithmetic {
                    operator,
                    hint,
                    pair: Pair::First(second, second_location),
                },
                Goal::Infer(first, None, first_location),
            ),
            PeerOperator::Ordering(operator) => (
                PeerForm::Ordering {
                    operator,
                    pair: Pair::First(second, second_location),
                },
                Goal::Infer(first, None, first_location),
            ),
            PeerOperator::Equality(operator) => {
                let mut converted = None;
                let goal = self.equality_operand(&mut converted, first, None, first_location)?;
                (
                    PeerForm::Equality {
                        operator,
                        pair: Pair::First(second, second_location),
                        converted,
                    },
                    goal,
                )
            }
        };
        frames.push(Frame::Peer(Box::new(PeerFrame {
            form,
            right_first,
            location,
        })));
        Ok(Step::Descend(goal))
    }

    /// Start one equality operand, hinted with its typed peer's type: a
    /// non-collection `convert<T>(e)` is compared as converted, its own
    /// nesting level entered here and `converted` set to its target; a
    /// contextual operand with a typed peer is checked as the peer's type;
    /// any other operand is inferred.
    fn equality_operand<'e, 'r>(
        &mut self,
        converted: &mut Option<(ValueType, Location)>,
        expression: &'e Expression,
        peer: Option<Want<'r>>,
        location: Location,
    ) -> Result<Goal<'e, 'r>, CheckRefusal> {
        if let Expression::Convert { target, operand } = expression {
            let target = self.resolve_type(target, &location)?;
            if !matches!(target, ValueType::Collection(_)) {
                self.enter(&location)?;
                self.check_declared_type(&target, &location)?;
                let operand_location = location.child(0);
                *converted = Some((target, location));
                return Ok(Goal::Infer(operand, None, operand_location));
            }
        }
        Ok(match peer {
            Some(peer) if contextual(expression) => {
                Goal::Expect(expression, Expected::Checked(peer), location)
            }
            peer => Goal::Infer(expression, peer, location),
        })
    }

    /// A typed equality operand, the equality operand it compares as, and
    /// the type its peer is hinted with. A converted operand leaves the
    /// nesting level [`Self::equality_operand`] entered for it; its IEEE
    /// source is refused.
    fn equality_typed(
        &mut self,
        typed: Node,
        converted: Option<(ValueType, Location)>,
    ) -> Result<(Node, EqualityOperand, ValueType), CheckRefusal> {
        match converted {
            Some((target, location)) => {
                self.leave();
                if matches!(typed.value_type, ValueType::Float(_)) {
                    return Err(mismatch(&location));
                }
                let source = typed.value_type.clone();
                Ok((
                    typed,
                    EqualityOperand::converted(source, target.clone()),
                    target,
                ))
            }
            None => {
                let value_type = typed.value_type.clone();
                Ok((
                    typed,
                    EqualityOperand::typed(value_type.clone()),
                    value_type,
                ))
            }
        }
    }

    /// Check a call's next argument, or build the call once every argument
    /// is checked.
    fn application<'e, 'r>(
        frame: Box<ApplicationFrame<'e, 'r>>,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Step<'e, 'r> {
        let index = frame.typed.len();
        let arguments = frame.arguments;
        match (arguments.get(index), frame.application.parameter(index)) {
            (Some(argument), Some(parameter)) => {
                let location = frame.location.child(index);
                frames.push(Frame::Application(frame));
                Step::Descend(Goal::Expect(
                    argument,
                    Expected::Checked(Cow::Borrowed(parameter)),
                    location,
                ))
            }
            _ => {
                let ApplicationFrame {
                    application,
                    location,
                    typed,
                    ..
                } = *frame;
                Step::Typed(application.finish(typed, &location))
            }
        }
    }

    /// Admit a record literal's initializers in source order up to the next
    /// value to check, or build the record once every initializer is
    /// admitted.
    fn record_step<'e, 'r>(
        mut frame: Box<RecordFrame<'e, 'r>>,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal> {
        let fields = frame.fields;
        while let Some((field, initializer)) = fields.get(frame.next) {
            frame.next += 1;
            let field_location = match initializer {
                FieldInitializer::Value(_) => {
                    let at = frame.location.child(frame.child);
                    frame.child += 1;
                    at
                }
                FieldInitializer::Null => frame.location.clone(),
            };
            let admitted = Self::record_field(
                frame.declared,
                &mut frame.supplied,
                field,
                initializer,
                &field_location,
            )?;
            if let Some((index, value_type, expression)) = admitted {
                frame.awaiting = Some(index);
                frames.push(Frame::Record(frame));
                return Ok(Step::Descend(Goal::Expect(
                    expression,
                    Expected::Checked(Cow::Borrowed(value_type)),
                    field_location,
                )));
            }
        }
        let RecordFrame {
            key,
            declared,
            supplied,
            location,
            ..
        } = *frame;
        Self::record(key, declared, supplied, &location).map(Step::Typed)
    }

    /// Check a collection literal's next element, or build the literal once
    /// every element is checked.
    fn collection<'e, 'r>(
        frame: Box<CollectionFrame<'e>>,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Step<'e, 'r> {
        let index = frame.typed.len();
        let elements = frame.elements;
        match elements.get(index) {
            Some(element) => {
                let location = frame.location.child(index);
                let element_type = frame.collection_type.element().clone();
                frames.push(Frame::Collection(frame));
                Step::Descend(Goal::Expect(
                    element,
                    Expected::Checked(Cow::Owned(element_type)),
                    location,
                ))
            }
            None => {
                let CollectionFrame {
                    collection_type,
                    location,
                    typed,
                    ..
                } = *frame;
                let value_type = ValueType::collection(collection_type.clone());
                Step::Typed(node(
                    NodeKind::Collection {
                        collection_type,
                        elements: typed,
                    },
                    value_type,
                    &location,
                ))
            }
        }
    }

    /// Start `count<N>(x in c: p)` or `sum<N>(x in c: e)`: `N` names an
    /// integer type.
    fn tally<'e, 'r>(
        &self,
        tally: Tally,
        (result_type, binder): (&str, &'e str),
        (source, body): (&'e Expression, &'e Expression),
        location: Location,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal> {
        let value_type = self.type_named(result_type, &location)?;
        if !is_integer(&value_type) {
            return Err(mismatch(&location));
        }
        let source_location = location.child(0);
        frames.push(Frame::Tally(Box::new(TallyFrame {
            tally,
            value_type,
            binder,
            body,
            location,
            source: None,
        })));
        Ok(Step::Descend(Goal::Infer(source, None, source_location)))
    }

    /// Check a dispatch call's next argument, or build the call once every
    /// argument is checked.
    fn dispatch_arguments<'e, 'r>(
        frame: Box<DispatchArgumentsFrame<'e, 'r>>,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Step<'e, 'r> {
        let index = frame.typed.len();
        let (arguments, operation) = (frame.arguments, frame.operation);
        match (arguments.get(index), operation.parameters.get(index)) {
            (Some(argument), Some(parameter)) => {
                let location = frame.location.child(index + 1);
                frames.push(Frame::DispatchArguments(frame));
                Step::Descend(Goal::Expect(
                    argument,
                    Expected::Argument(Cow::Borrowed(parameter)),
                    location,
                ))
            }
            _ => {
                let DispatchArgumentsFrame {
                    receiver,
                    operation_index,
                    location,
                    typed,
                    ..
                } = *frame;
                Step::Typed(node(
                    NodeKind::Dispatch {
                        receiver: Box::new(receiver),
                        table: operation.table,
                        operation: operation_index,
                        arguments: typed,
                    },
                    operation.result.clone(),
                    &location,
                ))
            }
        }
    }

    /// Hand the node `typed` to `frame`, which types its next operand or
    /// finishes its own node.
    fn accept<'e, 'r>(
        &mut self,
        frame: Frame<'e, 'r>,
        typed: Node,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal>
    where
        'a: 'r,
    {
        match frame {
            Frame::Leave => {
                self.leave();
                Ok(Step::Typed(typed))
            }
            Frame::Coerce(required) => coerce(typed, &required).map(Step::Typed),
            Frame::Upcast(required, location) => {
                self.upcast(typed, &required, &location).map(Step::Typed)
            }
            Frame::If(frame) => Self::accept_if(frame, typed, frames),
            Frame::Let(frame) => self.accept_let(frame, typed, frames),
            Frame::Connective(frame) => Ok(Self::accept_connective(frame, typed, frames)),
            Frame::Peer(frame) => self.accept_peer(frame, typed, frames),
            Frame::Unary(frame) => self.accept_unary(*frame, typed).map(Step::Typed),
            Frame::Field(field, location) => self.field(typed, field, &location).map(Step::Typed),
            Frame::Attribute(frame) => self
                .attribute(typed, frame.field, &frame.operand_location, &frame.location)
                .map(Step::Typed),
            Frame::Application(mut frame) => {
                frame.typed.push(typed);
                Ok(Self::application(frame, frames))
            }
            Frame::Record(mut frame) => {
                if let Some(index) = frame.awaiting.take() {
                    if let Some(slot) = frame.supplied.get_mut(index) {
                        *slot = Some(RecordSlot::Present(Box::new(typed)));
                    }
                }
                Self::record_step(frame, frames)
            }
            Frame::Collection(mut frame) => {
                frame.typed.push(typed);
                Ok(Self::collection(frame, frames))
            }
            Frame::Query(frame) => self.accept_query(frame, typed, frames),
            Frame::Accumulate(frame) => self.accept_accumulate(frame, typed, frames),
            Frame::Tally(frame) => self.accept_tally(frame, typed, frames),
            Frame::Contains(frame) => self.accept_contains(frame, typed, frames),
            Frame::Lookup(mut frame) => match frame.population.take() {
                None => {
                    Self::lookup_population(&typed)?;
                    let reference_location = frame.location.child(1);
                    let reference = frame.reference;
                    frame.population = Some(typed);
                    frames.push(Frame::Lookup(frame));
                    Ok(Step::Descend(Goal::Infer(
                        reference,
                        None,
                        reference_location,
                    )))
                }
                Some(population) => self
                    .lookup(
                        &frame.target,
                        population,
                        typed,
                        frame.absence,
                        &frame.location,
                    )
                    .map(Step::Typed),
            },
            Frame::Dispatch(frame) => {
                let DispatchFrame {
                    member,
                    arguments,
                    location,
                } = *frame;
                let (operation_index, operation) =
                    self.dispatch_operation(&typed, member, arguments.len(), &location)?;
                Ok(Self::dispatch_arguments(
                    Box::new(DispatchArgumentsFrame {
                        receiver: typed,
                        operation_index,
                        operation,
                        arguments,
                        location,
                        typed: Vec::with_capacity(arguments.len()),
                    }),
                    frames,
                ))
            }
            Frame::DispatchArguments(mut frame) => {
                frame.typed.push(typed);
                Ok(Self::dispatch_arguments(frame, frames))
            }
        }
    }

    fn accept_if<'e, 'r>(
        frame: Box<IfFrame<'e, 'r>>,
        typed: Node,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal> {
        let IfFrame {
            branches,
            location,
            stage,
        } = *frame;
        match stage {
            IfStage::Condition { then, otherwise } => {
                let goal = branches.goal(then, location.child(1));
                frames.push(Frame::If(Box::new(IfFrame {
                    branches,
                    location,
                    stage: IfStage::Then {
                        condition: typed,
                        otherwise,
                    },
                })));
                Ok(Step::Descend(goal))
            }
            IfStage::Then {
                condition,
                otherwise,
            } => {
                let goal = branches.goal(otherwise, location.child(2));
                frames.push(Frame::If(Box::new(IfFrame {
                    branches,
                    location,
                    stage: IfStage::Otherwise {
                        condition,
                        then: typed,
                    },
                })));
                Ok(Step::Descend(goal))
            }
            IfStage::Otherwise { condition, then } => {
                let otherwise = typed;
                let value_type = match branches {
                    Branches::Expected(expected) => expected.into_type(),
                    Branches::Inferred(_) => {
                        if then.value_type == otherwise.value_type {
                            then.value_type.clone()
                        } else if is_integer(&then.value_type) && is_integer(&otherwise.value_type)
                        {
                            ValueType::Integer
                        } else {
                            return Err(mismatch(&otherwise.location));
                        }
                    }
                };
                Ok(Step::Typed(node(
                    NodeKind::If {
                        condition: Box::new(condition),
                        then: Box::new(then),
                        otherwise: Box::new(otherwise),
                    },
                    value_type,
                    &location,
                )))
            }
        }
    }

    fn accept_let<'e, 'r>(
        &mut self,
        mut frame: Box<LetFrame<'e, 'r>>,
        typed: Node,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal> {
        match frame.value.take() {
            None => {
                let slot = self.bind(
                    frame.name,
                    typed.value_type.clone(),
                    &frame.location,
                    LocalKind::Bound,
                )?;
                let goal = frame.branches.goal(frame.body, frame.location.child(1));
                frame.value = Some((typed, slot));
                frames.push(Frame::Let(frame));
                Ok(Step::Descend(goal))
            }
            Some((value, slot)) => {
                self.unbind(1);
                let LetFrame {
                    branches, location, ..
                } = *frame;
                let body = typed;
                let value_type = match branches {
                    Branches::Expected(expected) => expected.into_type(),
                    Branches::Inferred(_) => body.value_type.clone(),
                };
                Ok(Step::Typed(node(
                    NodeKind::Let {
                        slot,
                        value: Box::new(value),
                        body: Box::new(body),
                    },
                    value_type,
                    &location,
                )))
            }
        }
    }

    fn accept_connective<'e, 'r>(
        frame: Box<ConnectiveFrame<'e>>,
        typed: Node,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Step<'e, 'r> {
        let ConnectiveFrame {
            connective,
            location,
            pair,
        } = *frame;
        match pair {
            Pair::First(right, right_location) => {
                frames.push(Frame::Connective(Box::new(ConnectiveFrame {
                    connective,
                    location,
                    pair: Pair::Second(typed),
                })));
                Step::Descend(Goal::Expect(
                    right,
                    Expected::Checked(boolean()),
                    right_location,
                ))
            }
            Pair::Second(left) => Step::Typed(node(
                NodeKind::Connective(connective, Box::new(left), Box::new(typed)),
                ValueType::Boolean,
                &location,
            )),
        }
    }

    fn accept_peer<'e, 'r>(
        &mut self,
        frame: Box<PeerFrame<'e, 'r>>,
        typed: Node,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal> {
        let PeerFrame {
            form,
            right_first,
            location,
        } = *frame;
        match form {
            PeerForm::Arithmetic {
                operator,
                hint,
                pair,
            } => match pair {
                Pair::First(second, second_location) => {
                    let peer = typed.value_type.clone();
                    frames.push(Frame::Peer(Box::new(PeerFrame {
                        form: PeerForm::Arithmetic {
                            operator,
                            hint,
                            pair: Pair::Second(typed),
                        },
                        right_first,
                        location,
                    })));
                    Ok(Step::Descend(Goal::Infer(
                        second,
                        Some(Cow::Owned(peer)),
                        second_location,
                    )))
                }
                Pair::Second(first) => {
                    let (left, right) = ordered(right_first, first, typed);
                    self.arithmetic(operator, left, right, hint.as_deref(), &location)
                        .map(Step::Typed)
                }
            },
            PeerForm::Ordering { operator, pair } => match pair {
                Pair::First(second, second_location) => {
                    let peer = typed.value_type.clone();
                    frames.push(Frame::Peer(Box::new(PeerFrame {
                        form: PeerForm::Ordering {
                            operator,
                            pair: Pair::Second(typed),
                        },
                        right_first,
                        location,
                    })));
                    Ok(Step::Descend(Goal::Infer(
                        second,
                        Some(Cow::Owned(peer)),
                        second_location,
                    )))
                }
                Pair::Second(first) => {
                    let (left, right) = ordered(right_first, first, typed);
                    self.ordering(operator, left, right, &location)
                        .map(Step::Typed)
                }
            },
            PeerForm::Equality {
                operator,
                pair,
                converted,
            } => {
                let (typed, compared, peer) = self.equality_typed(typed, converted)?;
                match pair {
                    Pair::First(second, second_location) => {
                        let mut converted = None;
                        let goal = self.equality_operand(
                            &mut converted,
                            second,
                            Some(Cow::Owned(peer)),
                            second_location,
                        )?;
                        frames.push(Frame::Peer(Box::new(PeerFrame {
                            form: PeerForm::Equality {
                                operator,
                                pair: Pair::Second((typed, compared)),
                                converted,
                            },
                            right_first,
                            location,
                        })));
                        Ok(Step::Descend(goal))
                    }
                    Pair::Second(first) => {
                        let (left, right) = ordered(right_first, first, (typed, compared));
                        self.equality(operator, left, right, &location)
                            .map(Step::Typed)
                    }
                }
            }
        }
    }

    fn accept_unary(&mut self, frame: UnaryFrame<'_>, operand: Node) -> Result<Node, CheckRefusal> {
        let UnaryFrame { form, location } = frame;
        match form {
            Unary::Negate(hint) => self.negate(operand, hint.as_deref(), &location),
            Unary::Not => Ok(node(
                NodeKind::Not(Box::new(operand)),
                ValueType::Boolean,
                &location,
            )),
            Unary::Present => {
                if !matches!(operand.value_type, ValueType::Option(_)) {
                    return Err(mismatch(&location));
                }
                Ok(node(
                    NodeKind::Present(Box::new(operand)),
                    ValueType::Boolean,
                    &location,
                ))
            }
            Unary::Value => {
                let ValueType::Option(payload) = operand.value_type.clone() else {
                    return Err(mismatch(&location));
                };
                Ok(node(
                    NodeKind::Value(Box::new(operand)),
                    *payload,
                    &location,
                ))
            }
            Unary::Deref => Err(mismatch(&location)),
            Unary::Pre => {
                let value_type = operand.value_type.clone();
                Ok(node(
                    NodeKind::Pre(Box::new(operand)),
                    value_type,
                    &location,
                ))
            }
            Unary::Flatten => self.flatten(operand, &location),
            Unary::Size => {
                self.element_type(&operand)?;
                Ok(node(
                    NodeKind::Size(Box::new(operand)),
                    ValueType::Integer,
                    &location,
                ))
            }
            Unary::Convert(target) => self.convert(&target, operand, &location),
            Unary::AllInstances(target) => Self::all_instances(&target, operand, &location),
        }
    }

    fn accept_query<'e, 'r>(
        &mut self,
        mut frame: Box<QueryFrame<'e>>,
        typed: Node,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal> {
        match frame.source.take() {
            None => {
                let (slot, source_type) =
                    self.bind_element(&typed, frame.binder, &frame.location)?;
                let body_location = frame.location.child(1);
                let goal = match frame.query {
                    BinderQuery::Map | BinderQuery::FlatMap => {
                        Goal::Infer(frame.body, None, body_location)
                    }
                    BinderQuery::Filter | BinderQuery::Forall | BinderQuery::Exists => {
                        Goal::Expect(frame.body, Expected::Checked(boolean()), body_location)
                    }
                };
                frame.source = Some((typed, slot, source_type));
                frames.push(Frame::Query(frame));
                Ok(Step::Descend(goal))
            }
            Some((source, slot, source_type)) => self
                .query(
                    frame.query,
                    (slot, source_type),
                    source,
                    typed,
                    &frame.location,
                )
                .map(Step::Typed),
        }
    }

    fn accept_accumulate<'e, 'r>(
        &mut self,
        mut frame: Box<AccumulateFrame<'e>>,
        typed: Node,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal> {
        match std::mem::replace(&mut frame.stage, AccumulateStage::Source) {
            AccumulateStage::Source => {
                let (accumulator, binder, source_type) = self.bind_accumulation(
                    (frame.form, frame.identity.is_some()),
                    &frame.value_type,
                    (frame.accumulator, frame.binder),
                    &typed,
                    &frame.location,
                )?;
                let goal = Goal::Infer(
                    frame.step,
                    Some(Cow::Owned(frame.value_type.clone())),
                    frame.location.child(1),
                );
                frame.stage = AccumulateStage::Step {
                    source: typed,
                    accumulator,
                    binder,
                    source_type,
                };
                frames.push(Frame::Accumulate(frame));
                Ok(Step::Descend(goal))
            }
            AccumulateStage::Step {
                source,
                accumulator,
                binder,
                source_type,
            } => {
                let raw = typed;
                self.unbind(2);
                let catalogued = catalogued_step(&raw, accumulator, &frame.value_type);
                let step = coerce(raw, &frame.value_type)?;
                let fold = Fold {
                    source,
                    step,
                    accumulator,
                    binder,
                    source_type,
                    catalogued,
                };
                match frame.identity {
                    Some(identity) => {
                        let goal = Goal::Expect(
                            identity,
                            Expected::Checked(Cow::Owned(frame.value_type.clone())),
                            frame.location.child(2),
                        );
                        frame.stage = AccumulateStage::Identity(fold);
                        frames.push(Frame::Accumulate(frame));
                        Ok(Step::Descend(goal))
                    }
                    None => Self::fold(*frame, fold, None).map(Step::Typed),
                }
            }
            AccumulateStage::Identity(fold) => {
                Self::fold(*frame, fold, Some(typed)).map(Step::Typed)
            }
        }
    }

    /// A `fold`/`reduce` node over its typed source, step and identity: an
    /// unordered source needs a catalogued step.
    fn fold(
        frame: AccumulateFrame<'_>,
        fold: Fold,
        identity: Option<Node>,
    ) -> Result<Node, CheckRefusal> {
        if !fold.source_type.kind().is_ordered() && !fold.catalogued {
            return Err(ineligible(&frame.location));
        }
        Ok(node(
            NodeKind::Fold {
                accumulator: fold.accumulator,
                binder: fold.binder,
                source: Box::new(fold.source),
                step: Box::new(fold.step),
                identity: identity.map(Box::new),
            },
            frame.value_type,
            &frame.location,
        ))
    }

    fn accept_tally<'e, 'r>(
        &mut self,
        mut frame: Box<TallyFrame<'e>>,
        typed: Node,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal> {
        match frame.source.take() {
            None => {
                let element = self.element_type(&typed)?;
                let slot = self.bind(frame.binder, element, &frame.location, LocalKind::Bound)?;
                let body_location = frame.location.child(1);
                let goal = match frame.tally {
                    Tally::Count => {
                        Goal::Expect(frame.body, Expected::Checked(boolean()), body_location)
                    }
                    Tally::Sum => Goal::Infer(frame.body, None, body_location),
                };
                frame.source = Some((typed, slot));
                frames.push(Frame::Tally(frame));
                Ok(Step::Descend(goal))
            }
            Some((source, slot)) => {
                self.unbind(1);
                let body = typed;
                let visit = match frame.tally {
                    Tally::Count => Visit::Count,
                    Tally::Sum => {
                        if !is_integer(&body.value_type) {
                            return Err(mismatch(&body.location));
                        }
                        Visit::Sum
                    }
                };
                let TallyFrame {
                    value_type,
                    location,
                    ..
                } = *frame;
                Ok(Step::Typed(node(
                    NodeKind::Query {
                        visit,
                        slot,
                        source: Box::new(source),
                        body: Box::new(body),
                    },
                    value_type,
                    &location,
                )))
            }
        }
    }

    fn accept_contains<'e, 'r>(
        &mut self,
        mut frame: Box<ContainsFrame<'e>>,
        typed: Node,
        frames: &mut Vec<Frame<'e, 'r>>,
    ) -> Result<Step<'e, 'r>, CheckRefusal> {
        match frame.collection.take() {
            None => {
                let element = self.element_type(&typed)?;
                let goal = Goal::Expect(
                    frame.item,
                    Expected::Checked(Cow::Owned(element.clone())),
                    frame.location.child(1),
                );
                frame.collection = Some((typed, element));
                frames.push(Frame::Contains(frame));
                Ok(Step::Descend(goal))
            }
            Some((collection, element)) => self
                .contains(collection, typed, element, &frame.location)
                .map(Step::Typed),
        }
    }
}

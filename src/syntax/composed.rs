// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-035: edition-selected syntax only; no model, profile or execution admission.
pub(crate) mod arena;

use super::{ClauseKind, ExprId, ExprKind, ModelImport, ParsedUnit};
use crate::{Source, Span, Spanned};

/// Source-selected syntax, separated from the historical checked-package path.
#[derive(Clone, Debug)]
pub enum NativeUnit {
    /// The original syntax shape consumed by the historical compiler APIs.
    Historical(ParsedUnit),
    /// Composed syntax; this cannot be passed off as a historical checked package.
    Composed(ComposedUnit),
}

/// The composed grammar's explicit edition selection.
pub const EDITION: &str = "1-draft";

/// A model-qualified declaration reference, retaining both original token spans.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualifiedName {
    /// The referenced model's imported alias, not yet resolved to a model artifact.
    pub model: Spanned<String>,
    /// The declaration or type name selected within that model.
    pub name: Spanned<String>,
}

/// Authored parameter type syntax, before resolving imports or checking domains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParameterType {
    /// The built-in boolean parameter type; the span covers its keyword spelling.
    Boolean(Span),
    /// A parameter type given by a qualified model type name.
    Model(QualifiedName),
}

/// A named, typed binder introduced by activation, capture, invocation or protocol syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    /// The bound identifier, in scope over the parameter's associated body.
    pub name: Spanned<String>,
    /// The parameter's declared type.
    pub ty: ParameterType,
    /// The complete parameter declaration's source range.
    pub span: Span,
}

/// A named value bound once at declaration or activation time and reused across a body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capture {
    /// The captured value's declared name and type.
    pub parameter: Parameter,
    /// Expression computing the captured value at binding time.
    pub value: ExprId,
    /// The complete capture declaration's source range.
    pub span: Span,
}

/// Activation syntax, including the trigger binder's separately scoped guard.
/// No runtime trigger, clock or captured value is established by this record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Activation {
    /// Activates exactly once, at declaration processing, with no per-occurrence trigger.
    Origin {
        /// The `on origin` clause's source range.
        span: Span,
    },
    /// Activates once per occurrence of a trigger parameter, optionally filtered by a guard.
    Each {
        /// The per-occurrence binder that activation reruns over.
        trigger: Parameter,
        /// Optional condition restricting which trigger occurrences activate.
        guard: Option<ExprId>,
        /// The complete `on each` clause's source range.
        span: Span,
    },
}

/// Unsigned interval spellings; ordering, units and bounds require admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Interval {
    /// Original spelling of the interval's inclusive lower bound.
    pub lower: Spanned<String>,
    /// Original spelling of the interval's inclusive upper bound.
    pub upper: Spanned<String>,
    /// The complete bracketed interval's source range.
    pub span: Span,
}

/// One top-level named declaration: a predicate, state clause, temporal formula or protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration {
    /// The declaration's authored name.
    pub name: Spanned<String>,
    /// The selected admission/checking profile for this declaration.
    pub profile: Spanned<String>,
    /// The declaration's syntax-level category and its associated content.
    pub kind: DeclarationKind,
    /// The complete declaration's source range.
    pub span: Span,
}

/// The syntax shape carried by a declaration, keyed by its introducing keyword.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeclarationKind {
    /// A named, parameterized boolean expression declaration.
    Predicate {
        /// The predicate's formal parameters.
        parameters: Vec<Parameter>,
        /// The declared boolean result type's source span.
        result: Span,
        /// The predicate's defining expression.
        body: ExprId,
    },
    /// An invariant, precondition or postcondition over a model context.
    State {
        /// Which clause category: invariant, precondition or postcondition.
        kind: ClauseKind,
        /// The qualified model type the clause applies to.
        context: QualifiedName,
        /// The operation name for pre/post clauses; absent for invariants.
        operation: Option<Spanned<String>>,
        /// The clause's defining boolean expression.
        body: ExprId,
    },
    /// A temporal-logic formula evaluated over a clocked, activated input.
    Temporal {
        /// The bound occurrence value the temporal formula observes.
        input: Parameter,
        /// The named clock this temporal declaration is evaluated against.
        clock: Spanned<String>,
        /// When new evaluation instances of this formula begin.
        activation: Box<Activation>,
        /// Values captured once at activation time, in scope over the formula.
        captures: Vec<Capture>,
        /// Handle to the formula's root node in the unit's temporal arena.
        formula: TemporalId,
    },
    /// A choreographed protocol declaration.
    Protocol(Box<Protocol>),
}

/// Common value expressions reuse the historical syntax shapes and parser.
/// New forms remain explicit syntax variants and cannot enter historical checking.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueKind {
    /// A value expression shape reused unchanged from the historical grammar.
    Shared(ExprKind),
    /// A call to a named predicate or declared function with argument expressions.
    Invoke {
        /// The invoked declaration's name.
        name: Spanned<String>,
        /// Argument expressions supplied at the call site.
        arguments: Vec<ExprId>,
    },
    /// A rational-number literal spelled as a numerator/denominator pair.
    Rational {
        /// Signed constant spelling; normalization and numeric bounds are deferred.
        numerator: Spanned<String>,
        /// Signed constant spelling of the denominator; normalization and numeric bounds are deferred.
        denominator: Spanned<String>,
    },
    /// A division or remainder operation between two value expressions.
    Product {
        /// Selected product operator.
        op: Spanned<ProductOp>,
        /// Left operand expression.
        left: ExprId,
        /// Right operand expression.
        right: ExprId,
    },
    /// A collection-size query bound to a declared domain type.
    Size {
        /// The qualified domain type the size query is checked against.
        domain: QualifiedName,
        /// The collection expression being measured.
        argument: ExprId,
    },
    /// A membership test of one value within a collection.
    Contains {
        /// The collection expression being tested.
        collection: ExprId,
        /// The candidate member expression.
        member: ExprId,
    },
    /// A filter, map, count or sum comprehension over a domain expression.
    Query {
        /// Selected comprehension operator.
        op: Spanned<QueryOp>,
        /// Required for count/sum, absent for filter/map in the selected grammar.
        result: Option<QualifiedName>,
        /// Name bound to each element of `domain` within `body`.
        binder: Spanned<String>,
        /// The collection expression iterated over.
        domain: ExprId,
        /// The per-element expression evaluated for each bound element.
        body: ExprId,
    },
}

/// Admitted product-level syntax operators, before operand type checking.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductOp {
    /// Division, spelled `/`.
    Slash,
    /// Remainder, spelled `mod`.
    Mod,
}

/// Admitted comprehension operators over a domain expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryOp {
    /// Selects the subset of domain elements for which the body holds.
    Filter,
    /// Transforms each domain element through the body expression.
    Map,
    /// Counts domain elements for which the body holds, checked against `result`.
    Count,
    /// Sums the body's numeric value over the domain, checked against `result`.
    Sum,
}

/// A flat value-expression node in a composed unit's expression arena.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expression {
    /// The expression's untyped syntax shape.
    pub kind: ValueKind,
    /// The complete expression's source range.
    pub span: Span,
    /// Original operator/builtin/call token, distinct from the full expression region.
    pub operator_span: Option<Span>,
}

/// A handle local to one composed unit's flat temporal arena.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TemporalId(pub(crate) usize);

/// Admitted temporal-logic operators; interval and operand well-formedness are checked later.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalOp {
    /// Right-associative temporal implication.
    Implies,
    /// Temporal disjunction.
    Or,
    /// Temporal conjunction.
    And,
    /// Temporal negation.
    Not,
    /// Holds at some future instant within the given interval.
    Eventually,
    /// Holds at every future instant within the given interval.
    Always,
    /// Held at some past instant within the given interval.
    Once,
    /// Held at every past instant within the given interval.
    Historically,
    /// The left operand holds until the right operand holds, within the given interval.
    Until,
    /// Dual of Until: the right operand holds up to and including when the left operand holds.
    Release,
    /// The right operand held at some past instant, with the left holding ever since, within the given interval.
    Since,
    /// Dual of Since: the left operand has held continuously since the right operand last held, within the given interval.
    Triggered,
}

/// The temporal-formula node shape, evaluated over a clocked instant sequence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalKind {
    /// A temporal constant, including beyond an authoritative execution boundary.
    Constant(bool),
    /// A native value expression at an evaluation instant; distinct from a constant.
    Holds(ExprId),
    /// A parenthesized subformula, kept distinct from its unwrapped shape.
    Group(TemporalId),
    /// A unary temporal operator applied to a subformula, optionally windowed by an interval.
    Unary {
        /// Selected unary temporal operator.
        op: Spanned<TemporalOp>,
        /// Optional bounding interval; absent only for `not`.
        interval: Option<Interval>,
        /// The operand subformula.
        argument: TemporalId,
    },
    /// A binary temporal operator relating two subformulas, windowed by an interval.
    Binary {
        /// Selected binary temporal operator.
        op: Spanned<TemporalOp>,
        /// Bounding interval for the relation.
        interval: Option<Interval>,
        /// Left operand subformula.
        left: TemporalId,
        /// Right operand subformula.
        right: TemporalId,
    },
}

/// A flat temporal-formula node in a composed unit's temporal arena.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Temporal {
    /// The node's untyped temporal syntax shape.
    pub kind: TemporalKind,
    /// The complete subformula's source range.
    pub span: Span,
}

/// A named participant bound to a model type within a protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Role {
    /// The role's authored name, referenced from events and requirements.
    pub name: Spanned<String>,
    /// The qualified model type this role is bound to.
    pub model: QualifiedName,
    /// The complete role declaration's source range.
    pub span: Span,
}

/// A named relationship type declared for use within a protocol's `related by` clauses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Relationship {
    /// The relationship's authored name.
    pub name: Spanned<String>,
    /// The qualified relationship type this name is bound to.
    pub model: QualifiedName,
    /// The complete relationship declaration's source range.
    pub span: Span,
}

/// A channel's message delivery ordering discipline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ordering {
    /// No ordering guarantee between messages on the channel.
    Unordered(Span),
    /// First-in-first-out ordering keyed by a per-message expression.
    Fifo {
        /// The per-message binder the ordering key is computed over.
        parameter: Parameter,
        /// Expression computing the ordering key for each message.
        key: ExprId,
        /// The complete `fifo by` clause's source range.
        span: Span,
    },
}

/// A named, directed message channel between two protocol roles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Channel {
    /// The channel's authored name, referenced from send/receive events.
    pub name: Spanned<String>,
    /// The sending role's name.
    pub from: Spanned<String>,
    /// The receiving role's name.
    pub to: Spanned<String>,
    /// The qualified type of message the channel carries.
    pub carries: QualifiedName,
    /// The channel's delivery ordering discipline.
    pub ordering: Ordering,
    /// Interval bounding message delivery latency.
    pub delivery: Interval,
    /// The complete channel declaration's source range.
    pub span: Span,
}

/// Structural node paths are distinct from model-qualified references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeRef {
    /// Dotted path segments identifying the referenced control node.
    pub path: Vec<Spanned<String>>,
    /// The complete node reference's source range.
    pub span: Span,
}

/// A qualified reference to one operation declared on a model context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    /// The qualified model type the operation is declared on.
    pub context: QualifiedName,
    /// The operation's name within that context.
    pub name: Spanned<String>,
}

/// Where a compensation's effect becomes irreversible, if ever.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommitBoundary {
    /// The effect is never committed; compensation remains available indefinitely.
    Never(Span),
    /// The control node past which the effect is committed and can no longer be compensated.
    Node(NodeRef),
}

/// A registered compensating action for one protocol effect, including its retry and recovery logic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Compensation {
    /// The compensation's authored name, referenced from `event ... for` clauses.
    pub name: Spanned<String>,
    /// The control node identifying the effect this compensation reverses.
    pub effect: NodeRef,
    /// Binder receiving the forwarded effect value at registration time.
    pub forward: Parameter,
    /// The role responsible for attempting the compensating operation.
    pub role: Spanned<String>,
    /// The compensating operation to invoke on the role's model.
    pub operation: Operation,
    /// The selected profile used to invoke the compensating operation.
    pub profile: Spanned<String>,
    /// The named clock the compensation's timing is measured against.
    pub clock: Spanned<String>,
    /// Values captured once at registration time, in scope over the whole compensation.
    pub registration_captures: Vec<Capture>,
    /// Per-occurrence binder for the event that activates this compensation.
    pub trigger: Parameter,
    /// Condition restricting which trigger occurrences activate the compensation.
    pub guard: ExprId,
    /// Values captured once at activation time, in scope over retry and recovery.
    pub activation_captures: Vec<Capture>,
    /// Interval bounding how long the compensating operation may be attempted.
    pub within: Interval,
    /// Original spelling of the maximum number of compensation attempts.
    pub attempts: Spanned<String>,
    /// The qualified type recording one compensation attempt's outcome.
    pub attempt_type: QualifiedName,
    /// Binder for the earlier of two attempts being compared by `retry`.
    pub earlier: Parameter,
    /// Binder for the later of two attempts being compared by `retry`.
    pub later: Parameter,
    /// Expression deciding whether to retry, given the earlier and later attempts.
    pub retry: ExprId,
    /// Where the compensated effect becomes irreversible.
    pub commit: CommitBoundary,
    /// Binder for the final attempt outcome passed to `recover`.
    pub recovery: Parameter,
    /// Expression computing the compensation's recovery value.
    pub recover: ExprId,
    /// The complete compensation declaration's source range.
    pub span: Span,
}

/// A protocol-level obligation: a referenced temporal formula or a registered compensation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolRequirement {
    /// A reference to a temporal declaration that the protocol's run must satisfy.
    Temporal {
        /// The referenced temporal declaration's name.
        name: Spanned<String>,
        /// The complete `requires temporal` clause's source range.
        span: Span,
    },
    /// A registered compensating action available to the protocol's run.
    Compensation(Box<Compensation>),
}

/// A complete choreographed protocol: participants, channels, obligations and control flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Protocol {
    /// The bound occurrence value that activates new protocol instances.
    pub input: Parameter,
    /// When new protocol instances begin.
    pub activation: Activation,
    /// Values captured once at activation time, in scope over the whole protocol.
    pub captures: Vec<Capture>,
    /// Participants bound to model types within this protocol.
    pub roles: Vec<Role>,
    /// Relationship types available to this protocol's `related by` clauses.
    pub relationships: Vec<Relationship>,
    /// Message channels available for send/receive events.
    pub channels: Vec<Channel>,
    /// Temporal obligations and registered compensations for this protocol.
    pub requirements: Vec<ProtocolRequirement>,
    /// Handle to the protocol's root control node.
    pub run: ControlId,
    /// The protocol's completion clause.
    pub finish: Finish,
}

/// A handle local to one composed unit's flat control arena.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlId(pub(crate) usize);

/// One guarded alternative of a `choice` control node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Case {
    /// The case's authored name.
    pub name: Spanned<String>,
    /// Condition selecting this case among its choice's alternatives.
    pub guard: ExprId,
    /// Handle to the control node executed when this case is selected.
    pub control: ControlId,
    /// The complete case's source range.
    pub span: Span,
}

/// One concurrently executed arm of a `parallel` control node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Branch {
    /// The branch's authored name.
    pub name: Spanned<String>,
    /// Handle to the control node executed by this branch.
    pub control: ControlId,
    /// The complete branch's source range.
    pub span: Span,
}

/// An assertion that two objects stand in a declared relationship, attached to an event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Related {
    /// The declared relationship type being asserted.
    pub relationship: Spanned<String>,
    /// Expression for the relationship's source object.
    pub from: ExprId,
    /// Expression for the relationship's target object.
    pub to: ExprId,
    /// The complete `related by` clause's source range.
    pub span: Span,
}

/// Event production kinds retain distinct references; parsing does not establish
/// delivery, attempt, successful-effect or compensation identity correspondence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventKind {
    /// Sends a message on a named channel.
    Send {
        /// The channel the message is sent on.
        channel: Spanned<String>,
    },
    /// Receives a message on a named channel, matched to its originating send.
    Receive {
        /// The channel the message is received on.
        channel: Spanned<String>,
        /// The control node identifying the matched send event.
        send: NodeRef,
    },
    /// A role attempts an operation, checked against declared contracts.
    Attempt {
        /// The role attempting the operation.
        role: Spanned<String>,
        /// The operation being attempted.
        operation: Operation,
        /// Named contracts the attempt is checked against.
        contracts: Vec<Spanned<String>>,
    },
    /// The successful effect of a prior attempt.
    Effect {
        /// The control node identifying the attempt this effect follows.
        attempt: NodeRef,
    },
    /// A domain event raised by a role, optionally tied to a compensation.
    Event {
        /// The role raising the event.
        role: Spanned<String>,
        /// The compensation this event is associated with, if any.
        compensation: Option<NodeRef>,
    },
}

/// One control-flow leaf producing a domain event, with its binder and constraint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    /// The event's production shape.
    pub kind: EventKind,
    /// Binder for the produced event's value.
    pub parameter: Parameter,
    /// Relationship assertions attached to this event.
    pub related: Vec<Related>,
    /// Expression the produced event value must satisfy.
    pub constraint: ExprId,
}

/// Authored control structure. Child handles are flat arena references;
/// declaration order neither resolves names nor supplies causal/closure facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlKind {
    /// Executes child control nodes one after another, in order.
    Sequence(Vec<ControlId>),
    /// One role selects a single alternative among guarded cases.
    Choice {
        /// The role making the selection.
        role: Spanned<String>,
        /// Values visible to the selecting role when choosing a case.
        visible: Vec<ExprId>,
        /// The guarded alternatives available for selection.
        cases: Vec<Case>,
    },
    /// Executes branches concurrently, joining on named completion points.
    Parallel {
        /// The concurrently executed branches.
        branches: Vec<Branch>,
        /// Names of the completion points all branches must reach before joining.
        join: Vec<Spanned<String>>,
    },
    /// Repeats a body control node while a guard holds, up to a maximum count.
    Repeat {
        /// The role deciding whether to continue repeating.
        role: Spanned<String>,
        /// Values visible to the role when deciding to continue.
        visible: Vec<ExprId>,
        /// Original spelling of the maximum repetition count.
        maximum: Spanned<String>,
        /// Condition evaluated before each repetition.
        guard: ExprId,
        /// Control node executed on each repetition.
        body: ControlId,
        /// Control node executed once the maximum count is reached.
        exhausted: ControlId,
    },
    /// Waits for a matching event after a node, within a bounded interval, with a timeout branch.
    Await {
        /// The control node whose completion this wait is measured from.
        after: NodeRef,
        /// The selected profile used to evaluate the wait.
        profile: Spanned<String>,
        /// The named clock the wait's timing is measured against.
        clock: Spanned<String>,
        /// Interval bounding how long the wait may take.
        within: Interval,
        /// The matched event control node awaited.
        event: ControlId,
        /// Control node executed once the awaited event occurs.
        then: ControlId,
        /// Control node executed if the interval elapses without the event occurring.
        timeout: ControlId,
    },
    /// A leaf control node producing one domain event.
    Event(Event),
    /// Asserts a boolean expression at this point in the control flow.
    Check {
        /// The selected profile used to evaluate the check.
        profile: Spanned<String>,
        /// The asserted boolean expression.
        expression: ExprId,
    },
    /// Marks the point past which an effect becomes irreversible for a compensation.
    Commit {
        /// The role performing the commit.
        role: Spanned<String>,
        /// Binder for the committed value.
        parameter: Parameter,
        /// Expression the committed value must satisfy.
        constraint: ExprId,
    },
}

/// A flat control-flow node in a composed unit's control arena.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Control {
    /// The control node's authored name.
    pub name: Spanned<String>,
    /// The node's untyped control-flow syntax shape.
    pub kind: ControlKind,
    /// The complete control node's source range.
    pub span: Span,
}

/// A protocol's completion clause, computing its final result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finish {
    /// The finish clause's authored name.
    pub name: Spanned<String>,
    /// Binder for the value the protocol completes with.
    pub parameter: Parameter,
    /// Expression the completion value must satisfy.
    pub constraint: ExprId,
    /// The complete finish clause's source range.
    pub span: Span,
}

/// Source-owned, flat syntax arenas. Handles never identify declarations in another unit.
#[derive(Clone, Debug)]
pub struct ComposedUnit {
    pub(crate) source: Source,
    pub(crate) language: Spanned<String>,
    pub(crate) edition: Spanned<String>,
    pub(crate) profiles: Vec<ModelImport>,
    pub(crate) models: Vec<ModelImport>,
    pub(crate) declarations: Vec<Declaration>,
    pub(crate) expressions: Vec<Expression>,
    pub(crate) temporal: Vec<Temporal>,
    pub(crate) controls: Vec<Control>,
}

impl ComposedUnit {
    /// Exact source from which this unit was parsed.
    pub fn source(&self) -> &Source {
        &self.source
    }
    /// Authored language selection and its original string-literal region.
    pub fn language(&self) -> &Spanned<String> {
        &self.language
    }
    /// Authored edition selection and its original string-literal region.
    pub fn edition(&self) -> &Spanned<String> {
        &self.edition
    }
    /// Unresolved profile selections in source order. They reuse the import tuple:
    /// `package` holds the profile definition name, not a model package selection.
    pub fn profiles(&self) -> &[ModelImport] {
        &self.profiles
    }
    /// Unresolved model imports, kept separate from semantic profile selections.
    pub fn models(&self) -> &[ModelImport] {
        &self.models
    }
    /// Declarations in authored order; duplicates and binding validity are unresolved.
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }
    /// Value handles are local to this unit, as are temporal and control handles.
    pub fn expression(&self, id: ExprId) -> Option<&Expression> {
        self.expressions.get(id.0)
    }
    /// Flat expression arena; each child handle is local to this unit.
    pub fn expressions(&self) -> &[Expression] {
        &self.expressions
    }
    /// IDs are local to this unit; out-of-range handles are rejected.
    pub fn temporal(&self, id: TemporalId) -> Option<&Temporal> {
        self.temporal.get(id.0)
    }
    /// Flat temporal-formula arena; each child handle is local to this unit.
    pub fn temporal_nodes(&self) -> &[Temporal] {
        &self.temporal
    }
    /// IDs are local to this unit; out-of-range handles are rejected.
    pub fn control(&self, id: ControlId) -> Option<&Control> {
        self.controls.get(id.0)
    }
    /// Flat control-flow arena; each child handle is local to this unit.
    pub fn controls(&self) -> &[Control] {
        &self.controls
    }
}

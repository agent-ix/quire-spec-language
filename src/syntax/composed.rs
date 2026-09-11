// SPDX-License-Identifier: AGPL-3.0-only
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
    pub model: Spanned<String>,
    pub name: Spanned<String>,
}

/// Authored parameter type syntax, before resolving imports or checking domains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParameterType {
    Boolean(Span),
    Model(QualifiedName),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    pub name: Spanned<String>,
    pub ty: ParameterType,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capture {
    pub parameter: Parameter,
    pub value: ExprId,
    pub span: Span,
}

/// Activation syntax, including the trigger binder's separately scoped guard.
/// No runtime trigger, clock or captured value is established by this record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Activation {
    Origin {
        span: Span,
    },
    Each {
        trigger: Parameter,
        guard: Option<ExprId>,
        span: Span,
    },
}

/// Unsigned interval spellings; ordering, units and bounds require admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Interval {
    pub lower: Spanned<String>,
    pub upper: Spanned<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Declaration {
    pub name: Spanned<String>,
    pub profile: Spanned<String>,
    pub kind: DeclarationKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeclarationKind {
    Predicate {
        parameters: Vec<Parameter>,
        result: Span,
        body: ExprId,
    },
    State {
        kind: ClauseKind,
        context: QualifiedName,
        operation: Option<Spanned<String>>,
        body: ExprId,
    },
    Temporal {
        input: Parameter,
        clock: Spanned<String>,
        activation: Box<Activation>,
        captures: Vec<Capture>,
        formula: TemporalId,
    },
    Protocol(Box<Protocol>),
}

/// Common value expressions reuse the historical syntax shapes and parser.
/// New forms remain explicit syntax variants and cannot enter historical checking.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueKind {
    Shared(ExprKind),
    Invoke {
        name: Spanned<String>,
        arguments: Vec<ExprId>,
    },
    Rational {
        /// Signed constant spelling; normalization and numeric bounds are deferred.
        numerator: Spanned<String>,
        denominator: Spanned<String>,
    },
    Product {
        op: Spanned<ProductOp>,
        left: ExprId,
        right: ExprId,
    },
    Size {
        domain: QualifiedName,
        argument: ExprId,
    },
    Contains {
        collection: ExprId,
        member: ExprId,
    },
    Query {
        op: Spanned<QueryOp>,
        /// Required for count/sum, absent for filter/map in the selected grammar.
        result: Option<QualifiedName>,
        binder: Spanned<String>,
        domain: ExprId,
        body: ExprId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductOp {
    Slash,
    Mod,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryOp {
    Filter,
    Map,
    Count,
    Sum,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expression {
    pub kind: ValueKind,
    pub span: Span,
    /// Original operator/builtin/call token, distinct from the full expression region.
    pub operator_span: Option<Span>,
}

/// A handle local to one composed unit's flat temporal arena.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TemporalId(pub(crate) usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalOp {
    Implies,
    Or,
    And,
    Not,
    Eventually,
    Always,
    Once,
    Historically,
    Until,
    Release,
    Since,
    Triggered,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalKind {
    /// A temporal constant, including beyond an authoritative execution boundary.
    Constant(bool),
    /// A native value expression at an evaluation instant; distinct from a constant.
    Holds(ExprId),
    Group(TemporalId),
    Unary {
        op: Spanned<TemporalOp>,
        interval: Option<Interval>,
        argument: TemporalId,
    },
    Binary {
        op: Spanned<TemporalOp>,
        interval: Option<Interval>,
        left: TemporalId,
        right: TemporalId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Temporal {
    pub kind: TemporalKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Role {
    pub name: Spanned<String>,
    pub model: QualifiedName,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Relationship {
    pub name: Spanned<String>,
    pub model: QualifiedName,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ordering {
    Unordered(Span),
    Fifo {
        parameter: Parameter,
        key: ExprId,
        span: Span,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Channel {
    pub name: Spanned<String>,
    pub from: Spanned<String>,
    pub to: Spanned<String>,
    pub carries: QualifiedName,
    pub ordering: Ordering,
    pub delivery: Interval,
    pub span: Span,
}

/// Structural node paths are distinct from model-qualified references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeRef {
    pub path: Vec<Spanned<String>>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    pub context: QualifiedName,
    pub name: Spanned<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommitBoundary {
    Never(Span),
    Node(NodeRef),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Compensation {
    pub name: Spanned<String>,
    pub effect: NodeRef,
    pub forward: Parameter,
    pub role: Spanned<String>,
    pub operation: Operation,
    pub profile: Spanned<String>,
    pub clock: Spanned<String>,
    pub registration_captures: Vec<Capture>,
    pub trigger: Parameter,
    pub guard: ExprId,
    pub activation_captures: Vec<Capture>,
    pub within: Interval,
    pub attempts: Spanned<String>,
    pub attempt_type: QualifiedName,
    pub earlier: Parameter,
    pub later: Parameter,
    pub retry: ExprId,
    pub commit: CommitBoundary,
    pub recovery: Parameter,
    pub recover: ExprId,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolRequirement {
    Temporal { name: Spanned<String>, span: Span },
    Compensation(Box<Compensation>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Protocol {
    pub input: Parameter,
    pub activation: Activation,
    pub captures: Vec<Capture>,
    pub roles: Vec<Role>,
    pub relationships: Vec<Relationship>,
    pub channels: Vec<Channel>,
    pub requirements: Vec<ProtocolRequirement>,
    pub run: ControlId,
    pub finish: Finish,
}

/// A handle local to one composed unit's flat control arena.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlId(pub(crate) usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Case {
    pub name: Spanned<String>,
    pub guard: ExprId,
    pub control: ControlId,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Branch {
    pub name: Spanned<String>,
    pub control: ControlId,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Related {
    pub relationship: Spanned<String>,
    pub from: ExprId,
    pub to: ExprId,
    pub span: Span,
}

/// Event production kinds retain distinct references; parsing does not establish
/// delivery, attempt, successful-effect or compensation identity correspondence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventKind {
    Send {
        channel: Spanned<String>,
    },
    Receive {
        channel: Spanned<String>,
        send: NodeRef,
    },
    Attempt {
        role: Spanned<String>,
        operation: Operation,
        contracts: Vec<Spanned<String>>,
    },
    Effect {
        attempt: NodeRef,
    },
    Event {
        role: Spanned<String>,
        compensation: Option<NodeRef>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub kind: EventKind,
    pub parameter: Parameter,
    pub related: Vec<Related>,
    pub constraint: ExprId,
}

/// Authored control structure. Child handles are flat arena references;
/// declaration order neither resolves names nor supplies causal/closure facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlKind {
    Sequence(Vec<ControlId>),
    Choice {
        role: Spanned<String>,
        visible: Vec<ExprId>,
        cases: Vec<Case>,
    },
    Parallel {
        branches: Vec<Branch>,
        join: Vec<Spanned<String>>,
    },
    Repeat {
        role: Spanned<String>,
        visible: Vec<ExprId>,
        maximum: Spanned<String>,
        guard: ExprId,
        body: ControlId,
        exhausted: ControlId,
    },
    Await {
        after: NodeRef,
        profile: Spanned<String>,
        clock: Spanned<String>,
        within: Interval,
        event: ControlId,
        then: ControlId,
        timeout: ControlId,
    },
    Event(Event),
    Check {
        profile: Spanned<String>,
        expression: ExprId,
    },
    Commit {
        role: Spanned<String>,
        parameter: Parameter,
        constraint: ExprId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Control {
    pub name: Spanned<String>,
    pub kind: ControlKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finish {
    pub name: Spanned<String>,
    pub parameter: Parameter,
    pub constraint: ExprId,
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
    pub fn source(&self) -> &Source {
        &self.source
    }
    pub fn language(&self) -> &Spanned<String> {
        &self.language
    }
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
    pub fn expressions(&self) -> &[Expression] {
        &self.expressions
    }
    pub fn temporal(&self, id: TemporalId) -> Option<&Temporal> {
        self.temporal.get(id.0)
    }
    pub fn temporal_nodes(&self) -> &[Temporal] {
        &self.temporal
    }
    pub fn control(&self, id: ControlId) -> Option<&Control> {
        self.controls.get(id.0)
    }
    pub fn controls(&self) -> &[Control] {
        &self.controls
    }
}

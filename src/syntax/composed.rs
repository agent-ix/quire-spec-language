// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-035: edition-selected syntax only; no model, profile or execution admission.
pub(crate) mod arena;

use super::{ClauseKind, ExprId, ExprKind, ModelImport, ParsedUnit};
use qsl_foundation::{Source, Span, Spanned};

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

/// A named, typed binder introduced by activation, capture or invocation syntax.
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

/// One top-level named declaration: a predicate, state clause or temporal formula.
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

/// A qualified reference to one operation declared on a model context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    /// The qualified model type the operation is declared on.
    pub context: QualifiedName,
    /// The operation's name within that context.
    pub name: Spanned<String>,
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
    /// Value handles are local to this unit, as are temporal handles.
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
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S2 parsed value-expression and declaration forms (ADR-011 §6.2
//! module map, layer-2 `forms`), as a typed tree over source names.
//!
//! Names are unresolved source spellings: a parameter, `let` or binder name,
//! a qualified enum member `E::m`, or a qualified call target. A location in
//! a refusal is the path of child indices from the declaration root, each
//! index numbered as [`Expression::children`] lists the children.

use qsl_foundation::absence::AbsenceMode;
use qsl_foundation::Span;
use quire_exact::{CollectionKind, Integer};

use super::spans::{DeclarationSpans, SpansMismatch};

/// A builtin type keyword a [`TypeForm`] can be headed by, other than a
/// collection kind (see [`TypeFormHead::Collection`]). Closed: `check`'s
/// resolution matches it exhaustively (FR-091-CON-2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinType {
    /// `Boolean`.
    Boolean,
    /// `Integer`: unbounded.
    Integer,
    /// `Int[lower, upper]`.
    Int,
    /// `Rational[numerator lower, numerator upper; denominator lower,
    /// denominator upper]`.
    Rational,
    /// `Decimal[lower, upper; min scale, max scale; rounding mode]`.
    Decimal,
    /// `Float32`, carrying no width payload: FR-091-OQ-4 puts the rounding
    /// mode in the floating type, so the head names the keyword only.
    Float32,
    /// `Float64`; see [`Self::Float32`].
    Float64,
    /// `Text[min, max; profile]`.
    Text,
    /// `Option<T>`.
    Option,
    /// `Reference<T>`.
    Reference,
}

/// A [`TypeForm`]'s head.
#[derive(Clone, Debug)]
pub enum TypeFormHead {
    /// A builtin keyword type.
    Builtin(BuiltinType),
    /// A collection type (`Sequence`, `Set`, `Bag`, `OrderedSet`), carried
    /// as the kernel [`CollectionKind`] the same way
    /// [`Expression::Collection`] carries it (ADR-011 §6.1: layer 2 depends
    /// on "1, F, K"; ADR-013 OQ-A).
    Collection(CollectionKind),
    /// `Population<T>[N]` (FR-153): its one argument is the named element
    /// type `T`, its one bound the declared maximum `N`.
    Population,
    /// A qualified name that `check` resolves against a declared alias,
    /// record or tuple, enum or model object type.
    Name(String),
}

/// A declared type as spelled in source (ADR-011 §1's stage table:
/// "Unchecked syntactic forms per family, each with a span. No semantic
/// identity."; §2.2 row E2: "Declared bounds and extents carried as
/// syntax").
///
/// It carries no `ValueType` and no `NodeKey` (FR-091-AC-11). [`Self::head`]
/// is a builtin keyword, a kernel collection kind, `Population` or a name.
/// [`Self::arguments`] are this same syntactic type recursively (an element
/// or payload type). [`Self::bounds`] are declared bound or extent literals,
/// spelled exactly as written (e.g. `Int[0, 10]`'s `"0"`, `"10"`;
/// `Text[1, 100; nfc]`'s `"nfc"`); FR-091-OQ-9 is open on whether they
/// become kernel values. `check` resolves a type form to the kernel
/// `ValueType` at E3 (ADR-013 O-14/C-26), and declaration identity is
/// minted over that resolved type, never over this spelling.
#[derive(Clone, Debug)]
pub struct TypeForm {
    /// The type's head.
    pub head: TypeFormHead,
    /// Type arguments in source order (e.g. `Sequence<Int[0,10]>`'s element,
    /// `Option<T>`'s payload, or `Population<M::A>[3]`'s named element type,
    /// which the kernel `ValueType::Population` does not itself carry).
    pub arguments: Vec<TypeForm>,
    /// Declared bound or extent literals, spelled exactly as written, in
    /// source order.
    pub bounds: Vec<String>,
    /// The source span.
    pub span: Span,
}

impl TypeForm {
    /// A type form headed by `head`, with no arguments or bounds yet.
    pub fn new(head: TypeFormHead, span: Span) -> Self {
        Self {
            head,
            arguments: Vec::new(),
            bounds: Vec::new(),
            span,
        }
    }

    /// A builtin keyword type with no arguments or bounds yet, e.g.
    /// `Boolean`.
    pub fn builtin(builtin: BuiltinType, span: Span) -> Self {
        Self::new(TypeFormHead::Builtin(builtin), span)
    }

    /// A collection type of `kind` with no element argument or bounds yet.
    pub fn collection(kind: CollectionKind, span: Span) -> Self {
        Self::new(TypeFormHead::Collection(kind), span)
    }

    /// A bare qualified name, e.g. a record, tuple, enum or model object
    /// type's own declared name.
    pub fn name(name: impl Into<String>, span: Span) -> Self {
        Self::new(TypeFormHead::Name(name.into()), span)
    }

    /// This type's own declared type arguments, in source order.
    #[must_use]
    pub fn with_arguments(mut self, arguments: Vec<TypeForm>) -> Self {
        self.arguments = arguments;
        self
    }

    /// This type's own declared bound or extent literals, spelled exactly as
    /// written, in source order.
    #[must_use]
    pub fn with_bounds(mut self, bounds: Vec<String>) -> Self {
        self.bounds = bounds;
        self
    }
}

/// A binary operator of the expression grammar.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BinaryOperator {
    /// `+`.
    Add,
    /// `-`.
    Subtract,
    /// `*`.
    Multiply,
    /// `/`.
    Divide,
    /// `=`.
    Equal,
    /// `!=`.
    NotEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
    /// `and`.
    And,
    /// `or`.
    Or,
    /// `implies`.
    Implies,
}

/// A record field initializer `f: e` or `f: null`.
#[derive(Clone, Debug)]
pub enum FieldInitializer {
    /// `f: e`.
    Value(Expression),
    /// `f: null`.
    Null,
}

/// A one-binder collection query form.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BinderQuery {
    /// `map(x in c: e)`, also spelled `collect`.
    Map,
    /// `filter(x in c: p)`.
    Filter,
    /// `flatMap(x in c: e)`: exactly `flatten(map(x in c: e))`.
    FlatMap,
    /// `forall(x in c: p)`.
    Forall,
    /// `exists(x in c: p)`.
    Exists,
}

/// An accumulating collection form.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Accumulation {
    /// `fold<A>(acc, x in c: step, identity: i)`.
    Fold,
    /// `reduce<A>(acc, x in c: step)`.
    Reduce,
}

/// The clause a checked declaration is. FR-151's dispatch-call restriction
/// (`quire.model.dispatch.single/v1`) gates a dispatched
/// `receiver.member(args)` call on this context, not on syntax alone: it
/// checks inside an invariant, precondition or postcondition, and is refused
/// `ill_typed`/`operator-ineligible` inside a function or operation body
/// (TC-196 D07).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClauseKind {
    /// A model invariant clause.
    Invariant,
    /// An operation's precondition clause.
    Precondition,
    /// An operation's postcondition clause.
    Postcondition,
    /// A function body, an operation body, or a `decreases` measure.
    Body,
}

/// The [`ClauseKind`] a [`FunctionDeclaration::clause`] entry may declare.
/// [`ClauseKind::Postcondition`] has no variant here: `pre(...)` legality
/// belongs to `check`'s `CheckedGraph::check_postcondition_expression`'s own
/// `pre_anchor`/population wiring, which no `PackageDeclarations::functions`
/// entry ever has, so the type itself rules the case out instead of a
/// runtime check on an otherwise-valid `ClauseKind` value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeclaredClauseKind {
    /// A model invariant clause.
    Invariant,
    /// An operation's precondition clause.
    Precondition,
    /// A function body, an operation body, or a `decreases` measure.
    Body,
}

impl From<DeclaredClauseKind> for ClauseKind {
    fn from(kind: DeclaredClauseKind) -> Self {
        match kind {
            DeclaredClauseKind::Invariant => Self::Invariant,
            DeclaredClauseKind::Precondition => Self::Precondition,
            DeclaredClauseKind::Body => Self::Body,
        }
    }
}

/// One value expression.
#[derive(Clone, Debug)]
pub enum Expression {
    /// `true` or `false`.
    Boolean(bool),
    /// An integer literal.
    Integer(Integer),
    /// `rational(n, d)`; it takes its unique expected `Rational[..]` type.
    Rational(Integer, Integer),
    /// A parameter, `let` or binder name, or a qualified enum member.
    Name(String),
    /// `let name = value in body`.
    Let {
        /// The bound name.
        name: String,
        /// The initializer, evaluated once.
        value: Box<Expression>,
        /// The scope of the binding.
        body: Box<Expression>,
    },
    /// `if condition then then else otherwise`.
    If {
        /// The Boolean condition.
        condition: Box<Expression>,
        /// Taken when the condition is true.
        then: Box<Expression>,
        /// Taken when the condition is false.
        otherwise: Box<Expression>,
    },
    /// `left op right`.
    Binary {
        /// The operator.
        operator: BinaryOperator,
        /// The left operand.
        left: Box<Expression>,
        /// The right operand.
        right: Box<Expression>,
    },
    /// Unary `-e`.
    Negate(Box<Expression>),
    /// `not e`.
    Not(Box<Expression>),
    /// `e.f`.
    Field {
        /// The record, or `deref(r)`, operand.
        operand: Box<Expression>,
        /// The field or attribute name.
        field: String,
    },
    /// `present(e)`.
    Present(Box<Expression>),
    /// `value(e)`.
    Value(Box<Expression>),
    /// `deref(r)`; only an attribute projection `deref(r).f` is a value.
    Deref(Box<Expression>),
    /// A call of a qualified name: a function, tuple constructor, or another
    /// declaration or undeclared name that the checker refuses.
    Call {
        /// The qualified call target.
        name: String,
        /// Arguments in source order.
        arguments: Vec<Expression>,
    },
    /// `R { f: e, ... }` in source order.
    Record {
        /// The record declaration name.
        name: String,
        /// Field initializers in source order.
        fields: Vec<(String, FieldInitializer)>,
    },
    /// `sequence[..]`, `set[..]`, `bag[..]` or `orderedSet[..]`; it takes its
    /// unique expected collection type.
    Collection {
        /// The literal's kind.
        kind: CollectionKind,
        /// Element expressions in source order.
        elements: Vec<Expression>,
    },
    /// `convert<T>(e)`.
    Convert {
        /// The target type.
        target: TypeForm,
        /// The converted operand.
        operand: Box<Expression>,
    },
    /// A one-binder query `q(binder in source: body)`.
    Query {
        /// The form.
        query: BinderQuery,
        /// The binder name.
        binder: String,
        /// The collection operand.
        source: Box<Expression>,
        /// The body, predicate or mapped expression.
        body: Box<Expression>,
    },
    /// `flatten(source)`.
    Flatten(Box<Expression>),
    /// `fold<A>(acc, x in c: step, identity: i)` or `reduce<A>(acc, x in c:
    /// step)`; the identity is kept whichever form is written so its presence
    /// is checked.
    Accumulate {
        /// Fold or reduce.
        form: Accumulation,
        /// The qualified name of the accumulator type `A`.
        accumulator_type: String,
        /// The accumulator name.
        accumulator: String,
        /// The element binder name.
        binder: String,
        /// The collection operand.
        source: Box<Expression>,
        /// The step.
        step: Box<Expression>,
        /// The `identity:` expression, when written.
        identity: Option<Box<Expression>>,
    },
    /// `count<N>(x in c: p)`.
    Count {
        /// The qualified name of the result type `N`.
        result_type: String,
        /// The binder name.
        binder: String,
        /// The collection operand.
        source: Box<Expression>,
        /// The predicate.
        predicate: Box<Expression>,
    },
    /// `sum<N>(x in c: e)`.
    Sum {
        /// The qualified name of the result type `N`.
        result_type: String,
        /// The binder name.
        binder: String,
        /// The collection operand.
        source: Box<Expression>,
        /// The summand.
        summand: Box<Expression>,
    },
    /// `size(c)`.
    Size(Box<Expression>),
    /// `contains(c, v)`.
    Contains {
        /// The collection operand.
        collection: Box<Expression>,
        /// The searched value.
        item: Box<Expression>,
    },
    /// `allInstances<T>(p)` (FR-153): every current member of `p` whose
    /// most-specific type conforms to `T`.
    AllInstances {
        /// The queried type `T`.
        target: TypeForm,
        /// The population operand `p`.
        population: Box<Expression>,
    },
    /// `lookup<T>(p, r) absent m` (FR-153): `r`'s presence in `p`, per `m`.
    Lookup {
        /// The queried type `T`.
        target: TypeForm,
        /// The population operand `p`.
        population: Box<Expression>,
        /// The reference operand `r`.
        reference: Box<Expression>,
        /// The absence mode `m`.
        absence: AbsenceMode,
    },
    /// `receiver.member(args)` (FR-151): a dispatched call, resolved at
    /// check time to the receiver's static type's exposed effective
    /// operation, and at link time to the receiver's most-specific runtime
    /// type. Only checks inside an invariant, precondition or postcondition
    /// clause (TC-196 D07); a call in a function or operation body is
    /// `ill_typed`/`operator-ineligible`.
    Dispatch {
        /// `self`, a `deref(...)` result, or another `Reference<T>` value.
        receiver: Box<Expression>,
        /// The unqualified member name.
        member: String,
        /// Arguments in source order.
        arguments: Vec<Expression>,
    },
    /// `pre(e)` (FR-153): `e`, evaluated with every `allInstances`/`lookup`
    /// underneath it reading its population operand's invocation pre state
    /// instead of the ambient post state. A caller-side anchor operation,
    /// valid only where a checked declaration's postcondition body admits
    /// it; `e`'s own type is unchanged.
    Pre(Box<Expression>),
}

impl Expression {
    /// The direct subexpressions in location-index order.
    pub fn children(&self) -> Vec<&Expression> {
        match self {
            Self::Boolean(_) | Self::Integer(_) | Self::Rational(..) | Self::Name(_) => Vec::new(),
            Self::Let { value, body, .. } => vec![value, body],
            Self::If {
                condition,
                then,
                otherwise,
            } => vec![condition, then, otherwise],
            Self::Binary { left, right, .. } => vec![left, right],
            Self::Negate(operand)
            | Self::Not(operand)
            | Self::Present(operand)
            | Self::Value(operand)
            | Self::Deref(operand)
            | Self::Flatten(operand)
            | Self::Size(operand)
            | Self::Field { operand, .. }
            | Self::Convert { operand, .. }
            | Self::Pre(operand) => vec![operand],
            Self::Call { arguments, .. } => arguments.iter().collect(),
            Self::Record { fields, .. } => fields
                .iter()
                .filter_map(|(_, initializer)| match initializer {
                    FieldInitializer::Value(expression) => Some(expression),
                    FieldInitializer::Null => None,
                })
                .collect(),
            Self::Collection { elements, .. } => elements.iter().collect(),
            Self::Query { source, body, .. } => vec![source, body],
            Self::Accumulate {
                source,
                step,
                identity,
                ..
            } => {
                let mut children: Vec<&Expression> = vec![source, step];
                children.extend(identity.as_deref());
                children
            }
            Self::Count {
                source, predicate, ..
            } => vec![source, predicate],
            Self::Sum {
                source, summand, ..
            } => vec![source, summand],
            Self::Contains { collection, item } => vec![collection, item],
            Self::AllInstances { population, .. } => vec![population],
            Self::Lookup {
                population,
                reference,
                ..
            } => vec![population, reference],
            Self::Dispatch {
                receiver,
                arguments,
                ..
            } => {
                let mut children: Vec<&Expression> = vec![receiver];
                children.extend(arguments);
                children
            }
        }
    }

    /// Moves every direct subexpression that has children of its own onto
    /// `stack`, leaving a childless placeholder or an empty `Vec` behind.
    /// Childless subexpressions are never pushed, so a node whose operands
    /// are all childless allocates nothing: a boxed one stays in place and a
    /// list element is dropped as its `Vec` drains, neither drop recursing.
    fn detach_children(&mut self, stack: &mut Vec<Expression>) {
        fn take(boxed: &mut Expression, stack: &mut Vec<Expression>) {
            if !boxed.is_childless() {
                stack.push(std::mem::replace(boxed, Expression::Boolean(false)));
            }
        }
        match self {
            Self::Boolean(_) | Self::Integer(_) | Self::Rational(..) | Self::Name(_) => {}
            Self::Negate(operand)
            | Self::Not(operand)
            | Self::Present(operand)
            | Self::Value(operand)
            | Self::Deref(operand)
            | Self::Flatten(operand)
            | Self::Size(operand)
            | Self::Field { operand, .. }
            | Self::Convert { operand, .. }
            | Self::AllInstances {
                population: operand,
                ..
            }
            | Self::Pre(operand) => take(operand, stack),
            Self::Let {
                value: first,
                body: second,
                ..
            }
            | Self::Binary {
                left: first,
                right: second,
                ..
            }
            | Self::Query {
                source: first,
                body: second,
                ..
            }
            | Self::Count {
                source: first,
                predicate: second,
                ..
            }
            | Self::Sum {
                source: first,
                summand: second,
                ..
            }
            | Self::Contains {
                collection: first,
                item: second,
            }
            | Self::Lookup {
                population: first,
                reference: second,
                ..
            } => {
                take(first, stack);
                take(second, stack);
            }
            Self::If {
                condition,
                then,
                otherwise,
            } => {
                take(condition, stack);
                take(then, stack);
                take(otherwise, stack);
            }
            Self::Accumulate {
                source,
                step,
                identity,
                ..
            } => {
                take(source, stack);
                take(step, stack);
                if let Some(identity) = identity {
                    take(identity, stack);
                }
            }
            Self::Call { arguments, .. }
            | Self::Collection {
                elements: arguments,
                ..
            } => stack.extend(arguments.drain(..).filter(|a| !a.is_childless())),
            Self::Dispatch {
                receiver,
                arguments,
                ..
            } => {
                take(receiver, stack);
                stack.extend(arguments.drain(..).filter(|a| !a.is_childless()));
            }
            Self::Record { fields, .. } => {
                stack.extend(
                    fields
                        .drain(..)
                        .filter_map(|(_, initializer)| match initializer {
                            FieldInitializer::Value(expression) => Some(expression),
                            FieldInitializer::Null => None,
                        })
                        .filter(|e| !e.is_childless()),
                );
            }
        }
    }

    /// Whether this form has no subexpressions at all.
    fn is_childless(&self) -> bool {
        match self {
            Self::Boolean(_) | Self::Integer(_) | Self::Rational(..) | Self::Name(_) => true,
            Self::Call { arguments, .. }
            | Self::Collection {
                elements: arguments,
                ..
            } => arguments.is_empty(),
            Self::Record { fields, .. } => fields.is_empty(),
            Self::Let { .. }
            | Self::If { .. }
            | Self::Binary { .. }
            | Self::Negate(_)
            | Self::Not(_)
            | Self::Field { .. }
            | Self::Present(_)
            | Self::Value(_)
            | Self::Deref(_)
            | Self::Convert { .. }
            | Self::Query { .. }
            | Self::Flatten(_)
            | Self::Accumulate { .. }
            | Self::Count { .. }
            | Self::Sum { .. }
            | Self::Size(_)
            | Self::Contains { .. }
            | Self::AllInstances { .. }
            | Self::Lookup { .. }
            | Self::Dispatch { .. }
            | Self::Pre(_) => false,
        }
    }
}

/// Drops an expression of any nesting depth in constant native stack: each
/// subexpression that has children of its own is detached onto a heap stack
/// first, so every node's own drop sees only childless operands.
impl Drop for Expression {
    fn drop(&mut self) {
        let mut stack = Vec::new();
        self.detach_children(&mut stack);
        while let Some(mut expression) = stack.pop() {
            expression.detach_children(&mut stack);
        }
    }
}

/// `function name using V(parameters): result pure [decreases(measure)] {
/// body }`.
///
/// Also used for a checked FR-151 dispatch candidate's own body or effective
/// precondition, so both share the FR-146 call-graph and termination
/// machinery: [`clause_kind`](Self::clause_kind) then reads
/// [`ClauseKind::Precondition`] instead of the default
/// [`ClauseKind::Body`], since a dispatched call is admitted inside a
/// precondition but not inside an operation body (TC-196 D06/D07/D08).
///
/// `clause_kind` and `callable_by_name` are private fields, read through
/// [`Self::clause_kind`] and [`Self::callable_by_name`]: neither is a bare
/// mutable field a caller can set independently of the other, and a
/// synthesized FR-151 dispatch candidate body or precondition clause is
/// never itself reachable through an ordinary named [`Expression::Call`]
/// (TC-196 D07's own restriction would otherwise be reachable by calling a
/// candidate directly instead of dispatching to it). Build one with
/// [`Self::new`] (an ordinary named function, name-callable) or
/// [`Self::clause`] (an invariant/precondition/postcondition clause, or a
/// synthesized dispatch candidate, never name-callable).
#[derive(Clone, Debug)]
pub struct FunctionDeclaration {
    /// The declared name.
    pub name: String,
    /// Parameters in order.
    pub parameters: Vec<(String, TypeForm)>,
    /// The declared result type.
    pub result: TypeForm,
    /// The `decreases` measure, when written.
    pub measure: Option<Expression>,
    /// The body.
    pub body: Expression,
    /// The clause this declaration's body is checked as. Every ordinary
    /// named function is [`ClauseKind::Body`].
    clause_kind: ClauseKind,
    /// Whether an ordinary named [`Expression::Call`] elsewhere in the same
    /// package may resolve to this declaration. `false` for every
    /// FR-151 synthesized function (TC-196 D07's bypass:
    /// closing the clause-kind restriction off syntax alone still leaves a
    /// candidate's body or precondition callable by plain name unless this
    /// is also `false`).
    callable_by_name: bool,
    /// The form's byte spans (FR-091-AC-10), when it was read from a source
    /// unit. A declaration built by hand or synthesized (FR-151) has none.
    spans: Option<DeclarationSpans>,
}

impl FunctionDeclaration {
    /// An ordinary named function: a real, name-callable declaration checked
    /// as [`ClauseKind::Body`].
    pub fn new(
        name: impl Into<String>,
        parameters: Vec<(String, TypeForm)>,
        result: TypeForm,
        measure: Option<Expression>,
        body: Expression,
    ) -> Self {
        Self {
            name: name.into(),
            parameters,
            result,
            measure,
            body,
            clause_kind: ClauseKind::Body,
            callable_by_name: true,
            spans: None,
        }
    }

    /// A declaration checked as `clause_kind`, never reachable through an
    /// ordinary named [`Expression::Call`]: an invariant or precondition
    /// clause, or a synthesized FR-151 dispatch candidate
    /// body or effective precondition. `clause_kind` is
    /// [`DeclaredClauseKind`], not [`ClauseKind`]: admitting
    /// `ClauseKind::Postcondition` here would let any caller assembling a
    /// package hand an ordinary function `pre(...)` legality it never
    /// earned — `pre(...)` is legal only behind a real postcondition's own
    /// `pre_anchor`/population wiring
    /// (`check`'s `CheckedGraph::check_postcondition_expression`, a standalone
    /// expression check outside `PackageDeclarations::functions` entirely),
    /// which no package function has — so `DeclaredClauseKind` leaves that
    /// case unrepresentable rather than accepting it and refusing later.
    pub fn clause(
        name: impl Into<String>,
        parameters: Vec<(String, TypeForm)>,
        result: TypeForm,
        measure: Option<Expression>,
        body: Expression,
        clause_kind: DeclaredClauseKind,
    ) -> Self {
        Self {
            name: name.into(),
            parameters,
            result,
            measure,
            body,
            clause_kind: clause_kind.into(),
            callable_by_name: false,
            spans: None,
        }
    }

    /// The clause this declaration's body is checked as:
    /// [`ClauseKind::Body`] for [`Self::new`], the declared kind for
    /// [`Self::clause`].
    pub fn clause_kind(&self) -> ClauseKind {
        self.clause_kind
    }

    /// Whether an ordinary named [`Expression::Call`] may resolve to this
    /// declaration: `true` for [`Self::new`], `false` for [`Self::clause`].
    pub fn callable_by_name(&self) -> bool {
        self.callable_by_name
    }

    /// This declaration read from a source unit, with its form's spans
    /// (FR-091-AC-10). Refused when the body spans do not have the body's
    /// shape, or the measure spans the measure's.
    pub fn with_spans(mut self, spans: DeclarationSpans) -> Result<Self, SpansMismatch> {
        if !spans.body.fits(&self.body) {
            return Err(SpansMismatch::Body);
        }
        match (&spans.measure, &self.measure) {
            (None, None) => {}
            (Some(measure_spans), Some(measure)) if measure_spans.fits(measure) => {}
            (Some(_), Some(_) | None) | (None, Some(_)) => return Err(SpansMismatch::Measure),
        }
        self.spans = Some(spans);
        Ok(self)
    }

    /// The form's byte spans, when it was read from a source unit.
    pub fn spans(&self) -> Option<&DeclarationSpans> {
        self.spans.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// TC-169 / FR-067-AC-9 / FR-067-CON-3: every variant, field and
    /// method of the parsed-form types is the fixed set below. An exhaustive match with every field named
    /// (no `..`) fails to compile the moment a variant or a field is added,
    /// removed or renamed — E0004 (non-exhaustive match) for a variant,
    /// E0026/E0027 (unknown/missing field) for a field — so this test is
    /// itself the shape check, not just evidence run under it.
    ///
    /// Two residual limits, both covered in practice by the root crate's
    /// live callers rather than by this test: binding a field with `_`
    /// names it but does not check its *type* (TC-169 step 4's stated
    /// scope), and `FunctionDeclaration::clause` is never called here.
    /// `check::checked_dispatch`'s `checked_dispatch_operation` exercises
    /// both, at its own `FunctionDeclaration::clause` call sites — a
    /// type-shape change on a bound-`_` field, or a `clause` signature
    /// change, still fails to compile there.
    #[trace("TC-169", "FR-067-AC-9", "FR-067-CON-3")]
    #[test]
    fn every_variant_and_field_of_the_moved_types_is_unchanged() {
        fn binary_operator(value: BinaryOperator) -> &'static str {
            match value {
                BinaryOperator::Add => "Add",
                BinaryOperator::Subtract => "Subtract",
                BinaryOperator::Multiply => "Multiply",
                BinaryOperator::Divide => "Divide",
                BinaryOperator::Equal => "Equal",
                BinaryOperator::NotEqual => "NotEqual",
                BinaryOperator::Less => "Less",
                BinaryOperator::LessOrEqual => "LessOrEqual",
                BinaryOperator::Greater => "Greater",
                BinaryOperator::GreaterOrEqual => "GreaterOrEqual",
                BinaryOperator::And => "And",
                BinaryOperator::Or => "Or",
                BinaryOperator::Implies => "Implies",
            }
        }
        assert_eq!(binary_operator(BinaryOperator::Add), "Add");

        fn binder_query(value: BinderQuery) -> &'static str {
            match value {
                BinderQuery::Map => "Map",
                BinderQuery::Filter => "Filter",
                BinderQuery::FlatMap => "FlatMap",
                BinderQuery::Forall => "Forall",
                BinderQuery::Exists => "Exists",
            }
        }
        assert_eq!(binder_query(BinderQuery::Map), "Map");

        fn accumulation(value: Accumulation) -> &'static str {
            match value {
                Accumulation::Fold => "Fold",
                Accumulation::Reduce => "Reduce",
            }
        }
        assert_eq!(accumulation(Accumulation::Fold), "Fold");

        fn clause_kind(value: ClauseKind) -> &'static str {
            match value {
                ClauseKind::Invariant => "Invariant",
                ClauseKind::Precondition => "Precondition",
                ClauseKind::Postcondition => "Postcondition",
                ClauseKind::Body => "Body",
            }
        }
        assert_eq!(clause_kind(ClauseKind::Body), "Body");

        fn declared_clause_kind(value: DeclaredClauseKind) -> &'static str {
            match value {
                DeclaredClauseKind::Invariant => "Invariant",
                DeclaredClauseKind::Precondition => "Precondition",
                DeclaredClauseKind::Body => "Body",
            }
        }
        assert_eq!(declared_clause_kind(DeclaredClauseKind::Body), "Body");
        assert_eq!(
            ClauseKind::from(DeclaredClauseKind::Precondition),
            ClauseKind::Precondition
        );

        fn field_initializer(value: &FieldInitializer) -> &'static str {
            match value {
                FieldInitializer::Value(_) => "Value",
                FieldInitializer::Null => "Null",
            }
        }
        assert_eq!(
            field_initializer(&FieldInitializer::Value(Expression::Boolean(true))),
            "Value"
        );
        assert_eq!(field_initializer(&FieldInitializer::Null), "Null");

        fn function_declaration_fields(value: &FunctionDeclaration) -> &'static str {
            let FunctionDeclaration {
                name: _,
                parameters: _,
                result: _,
                measure: _,
                body: _,
                clause_kind: _,
                callable_by_name: _,
                spans: _,
            } = value;
            "FunctionDeclaration"
        }
        let declaration = FunctionDeclaration::new(
            "f",
            Vec::new(),
            TypeForm::builtin(crate::BuiltinType::Boolean, Span { start: 0, end: 0 }),
            None,
            Expression::Boolean(true),
        );
        assert_eq!(
            function_declaration_fields(&declaration),
            "FunctionDeclaration"
        );

        fn expression(value: &Expression) -> &'static str {
            match value {
                Expression::Boolean(_) => "Boolean",
                Expression::Integer(_) => "Integer",
                Expression::Rational(_, _) => "Rational",
                Expression::Name(_) => "Name",
                Expression::Let {
                    name: _,
                    value: _,
                    body: _,
                } => "Let",
                Expression::If {
                    condition: _,
                    then: _,
                    otherwise: _,
                } => "If",
                Expression::Binary {
                    operator: _,
                    left: _,
                    right: _,
                } => "Binary",
                Expression::Negate(_) => "Negate",
                Expression::Not(_) => "Not",
                Expression::Field {
                    operand: _,
                    field: _,
                } => "Field",
                Expression::Present(_) => "Present",
                Expression::Value(_) => "Value",
                Expression::Deref(_) => "Deref",
                Expression::Call {
                    name: _,
                    arguments: _,
                } => "Call",
                Expression::Record { name: _, fields: _ } => "Record",
                Expression::Collection {
                    kind: _,
                    elements: _,
                } => "Collection",
                Expression::Convert {
                    target: _,
                    operand: _,
                } => "Convert",
                Expression::Query {
                    query: _,
                    binder: _,
                    source: _,
                    body: _,
                } => "Query",
                Expression::Flatten(_) => "Flatten",
                Expression::Accumulate {
                    form: _,
                    accumulator_type: _,
                    accumulator: _,
                    binder: _,
                    source: _,
                    step: _,
                    identity: _,
                } => "Accumulate",
                Expression::Count {
                    result_type: _,
                    binder: _,
                    source: _,
                    predicate: _,
                } => "Count",
                Expression::Sum {
                    result_type: _,
                    binder: _,
                    source: _,
                    summand: _,
                } => "Sum",
                Expression::Size(_) => "Size",
                Expression::Contains {
                    collection: _,
                    item: _,
                } => "Contains",
                Expression::AllInstances {
                    target: _,
                    population: _,
                } => "AllInstances",
                Expression::Lookup {
                    target: _,
                    population: _,
                    reference: _,
                    absence: _,
                } => "Lookup",
                Expression::Dispatch {
                    receiver: _,
                    member: _,
                    arguments: _,
                } => "Dispatch",
                Expression::Pre(_) => "Pre",
            }
        }
        assert_eq!(expression(&Expression::Boolean(true)), "Boolean");
        assert_eq!(expression(&declaration.body), "Boolean");

        // `Expression::children` (the one method besides construction, per
        // this module's own doc) walks direct subexpressions.
        let nested = Expression::Not(Box::new(Expression::Boolean(false)));
        assert_eq!(nested.children().len(), 1);
    }

    /// Nesting depth for the drop tests: well past the ~21,700 levels at
    /// which a recursive drop overflows a 2 MiB debug thread.
    const LEVELS: usize = 100_000;

    fn leaf(name: &str) -> Expression {
        Expression::Name(name.to_owned())
    }

    fn type_form() -> TypeForm {
        TypeForm::name("T", Span { start: 0, end: 0 })
    }

    /// Builds `LEVELS` nested expressions with `wrap` on a 2 MiB thread,
    /// checks the depth reached, and drops the tree on that same thread.
    fn drop_on_small_stack(wrap: fn(Expression, usize) -> Expression) {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                let mut expression = leaf("bottom");
                for level in 0..LEVELS {
                    expression = wrap(expression, level);
                }
                let mut depth = 0;
                let mut node = &expression;
                while let Some(child) = node.children().into_iter().find(|c| !c.is_childless()) {
                    depth += 1;
                    node = child;
                }
                assert_eq!(depth, LEVELS - 1, "the tree is nested LEVELS deep");
                drop(expression);
            })
            .expect("spawn the drop thread")
            .join()
            .expect("dropping a deep expression does not overflow the stack");
    }

    #[test]
    fn deep_single_operand_forms_drop_on_a_small_stack() {
        drop_on_small_stack(|inner, level| {
            let operand = Box::new(inner);
            match level % 11 {
                0 => Expression::Negate(operand),
                1 => Expression::Not(operand),
                2 => Expression::Present(operand),
                3 => Expression::Value(operand),
                4 => Expression::Deref(operand),
                5 => Expression::Flatten(operand),
                6 => Expression::Size(operand),
                7 => Expression::Pre(operand),
                8 => Expression::Field {
                    operand,
                    field: "f".to_owned(),
                },
                9 => Expression::Convert {
                    target: type_form(),
                    operand,
                },
                _ => Expression::AllInstances {
                    target: type_form(),
                    population: operand,
                },
            }
        });
    }

    #[test]
    fn deep_two_operand_forms_drop_on_a_small_stack() {
        drop_on_small_stack(|inner, level| {
            // Alternate which operand carries the nesting.
            let (first, second) = if level % 2 == 0 {
                (Box::new(inner), Box::new(leaf("x")))
            } else {
                (Box::new(leaf("x")), Box::new(inner))
            };
            match (level / 2) % 7 {
                0 => Expression::Let {
                    name: "x".to_owned(),
                    value: first,
                    body: second,
                },
                1 => Expression::Binary {
                    operator: BinaryOperator::And,
                    left: first,
                    right: second,
                },
                2 => Expression::Query {
                    query: BinderQuery::Map,
                    binder: "x".to_owned(),
                    source: first,
                    body: second,
                },
                3 => Expression::Count {
                    result_type: "N".to_owned(),
                    binder: "x".to_owned(),
                    source: first,
                    predicate: second,
                },
                4 => Expression::Sum {
                    result_type: "N".to_owned(),
                    binder: "x".to_owned(),
                    source: first,
                    summand: second,
                },
                5 => Expression::Contains {
                    collection: first,
                    item: second,
                },
                _ => Expression::Lookup {
                    target: type_form(),
                    population: first,
                    reference: second,
                    absence: AbsenceMode::Refused,
                },
            }
        });
    }

    #[test]
    fn deep_if_drops_on_a_small_stack() {
        drop_on_small_stack(|inner, level| {
            let mut operands = [leaf("c"), leaf("t"), leaf("e")];
            operands[level % 3] = inner;
            let [condition, then, otherwise] = operands;
            Expression::If {
                condition: Box::new(condition),
                then: Box::new(then),
                otherwise: Box::new(otherwise),
            }
        });
    }

    #[test]
    fn deep_accumulate_drops_on_a_small_stack() {
        drop_on_small_stack(|inner, level| {
            let mut operands = [leaf("c"), leaf("s"), leaf("i")];
            operands[level % 3] = inner;
            let [source, step, identity] = operands;
            Expression::Accumulate {
                form: Accumulation::Fold,
                accumulator_type: "A".to_owned(),
                accumulator: "a".to_owned(),
                binder: "x".to_owned(),
                source: Box::new(source),
                step: Box::new(step),
                identity: Some(Box::new(identity)),
            }
        });
    }

    #[test]
    fn deep_argument_lists_drop_on_a_small_stack() {
        drop_on_small_stack(|inner, level| match level % 4 {
            0 => Expression::Call {
                name: "f".to_owned(),
                arguments: vec![leaf("a"), inner],
            },
            1 => Expression::Collection {
                kind: CollectionKind::Sequence,
                elements: vec![inner, leaf("a")],
            },
            2 => Expression::Dispatch {
                receiver: Box::new(inner),
                member: "m".to_owned(),
                arguments: vec![leaf("a")],
            },
            _ => Expression::Dispatch {
                receiver: Box::new(leaf("r")),
                member: "m".to_owned(),
                arguments: vec![inner],
            },
        });
    }

    #[test]
    fn deep_record_fields_drop_on_a_small_stack() {
        drop_on_small_stack(|inner, _| Expression::Record {
            name: "R".to_owned(),
            fields: vec![
                ("a".to_owned(), FieldInitializer::Null),
                ("b".to_owned(), FieldInitializer::Value(inner)),
            ],
        });
    }
}

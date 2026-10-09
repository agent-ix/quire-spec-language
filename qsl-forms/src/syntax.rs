// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S2 parsed value-expression and declaration forms (ADR-011 §6.2
//! module map, layer-2 `forms`), as typed trees over source names. An
//! expression is an arena of nodes, each naming its children by id
//! (ADR-030 D-4.2).
//!
//! Names are unresolved source spellings: a parameter, `let` or binder name,
//! a qualified enum member `E::m`, or a qualified call target. A location in
//! a refusal is the path of child indices from the declaration root, each
//! index numbered as [`ExprNode::children`] lists the children.

use std::convert::Infallible;
use std::fmt;
use std::marker::PhantomData;
use std::ops::ControlFlow;

use qsl_foundation::absence::AbsenceMode;
use qsl_foundation::Span;
use quire_exact::{CollectionKind, Integer};

use super::spans::{DeclarationSpans, ExpressionSpans, SpansMismatch};

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeFormHead {
    /// A builtin keyword type.
    Builtin(BuiltinType),
    /// A collection type (`Sequence`, `Set`, `Bag`, `OrderedSet`), carried
    /// as the kernel [`CollectionKind`] the same way
    /// [`ExprNode::Collection`] carries it (ADR-011 §6.1: layer 2 depends
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
///
/// A type nests as deep as its source writes it (`Option<Option<...>>`), so
/// its `Clone`, `PartialEq`, `Debug` and `Drop` run on an explicit heap
/// stack rather than recursing once per level (ADR-030).
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

/// Clones a [`TypeForm`] bottom up: each exit rebuilds its node from the
/// clones of its arguments, which its children's exits left on `built`.
struct CloneTypes<'t> {
    built: Vec<TypeForm>,
    source: PhantomData<&'t TypeForm>,
    #[cfg(test)]
    gauge: crate::syntax::stack_peak::Gauge,
}

impl<'t> quire_walk::Walk for CloneTypes<'t> {
    type Node = &'t TypeForm;
    type Frame = &'t TypeForm;
    type Stop = Infallible;

    fn enter(
        &mut self,
        node: &'t TypeForm,
        children: &mut quire_walk::Children<'_, &'t TypeForm>,
    ) -> ControlFlow<Infallible, &'t TypeForm> {
        #[cfg(test)]
        self.gauge.enter(node.arguments.len());
        children.extend(&node.arguments);
        ControlFlow::Continue(node)
    }

    fn exit(&mut self, node: &'t TypeForm) -> ControlFlow<Infallible> {
        #[cfg(test)]
        {
            self.gauge.exit();
            crate::syntax::stack_peak::note("type_form_clone_built", self.built.len());
        }
        let arguments = self
            .built
            .split_off(self.built.len() - node.arguments.len());
        self.built.push(TypeForm {
            head: node.head.clone(),
            arguments,
            bounds: node.bounds.clone(),
            span: node.span,
        });
        ControlFlow::Continue(())
    }
}

impl Clone for TypeForm {
    fn clone(&self) -> Self {
        let mut clone = CloneTypes {
            built: Vec::new(),
            source: PhantomData,
            #[cfg(test)]
            gauge: crate::syntax::stack_peak::Gauge::new("type_form_clone"),
        };
        match quire_walk::walk(&mut clone, self) {
            ControlFlow::Continue(()) => {}
            ControlFlow::Break(never) => match never {},
        }
        clone.built.pop().expect("the root's exit leaves its clone")
    }
}

/// Compares two [`TypeForm`]s node by node, stopping at the first pair
/// that differs.
struct CompareTypes<'t> {
    forms: PhantomData<&'t TypeForm>,
    #[cfg(test)]
    gauge: crate::syntax::stack_peak::Gauge,
}

impl<'t> quire_walk::Walk for CompareTypes<'t> {
    type Node = (&'t TypeForm, &'t TypeForm);
    type Frame = ();
    type Stop = ();

    fn enter(
        &mut self,
        (left, right): (&'t TypeForm, &'t TypeForm),
        children: &mut quire_walk::Children<'_, (&'t TypeForm, &'t TypeForm)>,
    ) -> ControlFlow<()> {
        if left.head != right.head
            || left.bounds != right.bounds
            || left.span != right.span
            || left.arguments.len() != right.arguments.len()
        {
            return ControlFlow::Break(());
        }
        #[cfg(test)]
        self.gauge.enter(left.arguments.len());
        children.extend(left.arguments.iter().zip(&right.arguments));
        ControlFlow::Continue(())
    }

    fn exit(&mut self, (): ()) -> ControlFlow<()> {
        #[cfg(test)]
        self.gauge.exit();
        ControlFlow::Continue(())
    }
}

impl PartialEq for TypeForm {
    fn eq(&self, other: &Self) -> bool {
        quire_walk::walk(
            &mut CompareTypes {
                forms: PhantomData,
                #[cfg(test)]
                gauge: crate::syntax::stack_peak::Gauge::new("type_form_compare"),
            },
            (self, other),
        )
        .is_continue()
    }
}

impl Eq for TypeForm {}

/// Writes a [`TypeForm`] in the shape `#[derive(Debug)]` gives it:
/// `TypeForm { head: .., arguments: [..], bounds: [..], span: .. }`.
struct FormatTypes<'f, 'g, 't> {
    out: &'f mut fmt::Formatter<'g>,
    source: PhantomData<&'t TypeForm>,
    #[cfg(test)]
    gauge: crate::syntax::stack_peak::Gauge,
}

impl<'t> quire_walk::Walk for FormatTypes<'_, '_, 't> {
    /// The node, and whether an earlier sibling was written before it.
    type Node = (&'t TypeForm, bool);
    type Frame = &'t TypeForm;
    type Stop = fmt::Error;

    fn enter(
        &mut self,
        (node, after_sibling): (&'t TypeForm, bool),
        children: &mut quire_walk::Children<'_, (&'t TypeForm, bool)>,
    ) -> ControlFlow<fmt::Error, &'t TypeForm> {
        let written = (|| {
            if after_sibling {
                self.out.write_str(", ")?;
            }
            write!(self.out, "TypeForm {{ head: {:?}, arguments: [", node.head)
        })();
        if let Err(error) = written {
            return ControlFlow::Break(error);
        }
        #[cfg(test)]
        self.gauge.enter(node.arguments.len());
        children.extend(
            node.arguments
                .iter()
                .enumerate()
                .map(|(index, argument)| (argument, index > 0)),
        );
        ControlFlow::Continue(node)
    }

    fn exit(&mut self, node: &'t TypeForm) -> ControlFlow<fmt::Error> {
        #[cfg(test)]
        self.gauge.exit();
        match write!(
            self.out,
            "], bounds: {:?}, span: {:?} }}",
            node.bounds, node.span
        ) {
            Ok(()) => ControlFlow::Continue(()),
            Err(error) => ControlFlow::Break(error),
        }
    }
}

impl fmt::Debug for TypeForm {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match quire_walk::walk(
            &mut FormatTypes {
                out,
                source: PhantomData,
                #[cfg(test)]
                gauge: crate::syntax::stack_peak::Gauge::new("type_form_format"),
            },
            (self, false),
        ) {
            ControlFlow::Continue(()) => Ok(()),
            ControlFlow::Break(error) => Err(error),
        }
    }
}

impl Drop for TypeForm {
    /// Moves every descendant onto one heap stack and drops each with its
    /// arguments already taken, so dropping a deep type never recurses.
    fn drop(&mut self) {
        let mut pending = std::mem::take(&mut self.arguments);
        while let Some(mut form) = pending.pop() {
            pending.append(&mut form.arguments);
        }
    }
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldInitializer {
    /// `f: e`, the value named by its id in the record's arena.
    Value(ExprId),
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

/// The typed index of one node of an [`Expression`] arena.
///
/// An id names a node of the arena that minted it: every node a
/// [`ExprNode`] names as a child, and the root, are ids of the node's own
/// arena. Ids are positions, so they stay valid in a clone of that arena.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExprId(usize);

impl ExprId {
    /// The node's position in its arena: the number of nodes stored before
    /// it.
    pub fn index(self) -> usize {
        self.0
    }
}

impl fmt::Debug for ExprId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// One node of a value expression: its payload, with each subexpression
/// named by its [`ExprId`] in the same [`Expression`] arena.
///
/// `#[cfg(seam_probe)]` adds one further probe-only variant (ADR-012 §5.1
/// S2, FR-063): under `--cfg seam_probe`, every closed `match` over
/// this type outside this module becomes non-exhaustive (`E0004`) unless
/// it has its own probe arm. Never constructed outside the probe build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExprNode {
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
        value: ExprId,
        /// The scope of the binding.
        body: ExprId,
    },
    /// `if condition then then else otherwise`.
    If {
        /// The Boolean condition.
        condition: ExprId,
        /// Taken when the condition is true.
        then: ExprId,
        /// Taken when the condition is false.
        otherwise: ExprId,
    },
    /// `left op right`.
    Binary {
        /// The operator.
        operator: BinaryOperator,
        /// The left operand.
        left: ExprId,
        /// The right operand.
        right: ExprId,
    },
    /// Unary `-e`.
    Negate(ExprId),
    /// `not e`.
    Not(ExprId),
    /// `e.f`.
    Field {
        /// The record, or `deref(r)`, operand.
        operand: ExprId,
        /// The field or attribute name.
        field: String,
    },
    /// `present(e)`.
    Present(ExprId),
    /// `value(e)`.
    Value(ExprId),
    /// `deref(r)`; only an attribute projection `deref(r).f` is a value.
    Deref(ExprId),
    /// A call of a qualified name: a function, tuple constructor, or another
    /// declaration or undeclared name that the checker refuses.
    Call {
        /// The qualified call target.
        name: String,
        /// Arguments in source order.
        arguments: Vec<ExprId>,
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
        elements: Vec<ExprId>,
    },
    /// `convert<T>(e)`.
    Convert {
        /// The target type.
        target: TypeForm,
        /// The converted operand.
        operand: ExprId,
    },
    /// A one-binder query `q(binder in source: body)`.
    Query {
        /// The form.
        query: BinderQuery,
        /// The binder name.
        binder: String,
        /// The collection operand.
        source: ExprId,
        /// The body, predicate or mapped expression.
        body: ExprId,
    },
    /// `flatten(source)`.
    Flatten(ExprId),
    /// `fold<A>(acc, x in c: step, identity: i)` or `reduce<A>(acc, x in c:
    /// step)`; the identity is kept whichever form is written so its presence
    /// is checked.
    Accumulate {
        /// Fold or reduce.
        form: Accumulation,
        /// The qualified name of the accumulator type `A`.
        accumulator_type: String,
        /// The span of the accumulator type's name; empty when the form
        /// was not read from a source unit.
        accumulator_type_span: Span,
        /// The accumulator name.
        accumulator: String,
        /// The element binder name.
        binder: String,
        /// The collection operand.
        source: ExprId,
        /// The step.
        step: ExprId,
        /// The `identity:` expression, when written.
        identity: Option<ExprId>,
    },
    /// `count<N>(x in c: p)`.
    Count {
        /// The qualified name of the result type `N`.
        result_type: String,
        /// The span of the result type's name; empty when the form was not
        /// read from a source unit.
        result_type_span: Span,
        /// The binder name.
        binder: String,
        /// The collection operand.
        source: ExprId,
        /// The predicate.
        predicate: ExprId,
    },
    /// `sum<N>(x in c: e)`.
    Sum {
        /// The qualified name of the result type `N`.
        result_type: String,
        /// The span of the result type's name; empty when the form was not
        /// read from a source unit.
        result_type_span: Span,
        /// The binder name.
        binder: String,
        /// The collection operand.
        source: ExprId,
        /// The summand.
        summand: ExprId,
    },
    /// `size(c)`.
    Size(ExprId),
    /// `contains(c, v)`.
    Contains {
        /// The collection operand.
        collection: ExprId,
        /// The searched value.
        item: ExprId,
    },
    /// `allInstances<T>(p)` (FR-153): every current member of `p` whose
    /// most-specific type conforms to `T`.
    AllInstances {
        /// The queried type `T`.
        target: TypeForm,
        /// The population operand `p`.
        population: ExprId,
    },
    /// `lookup<T>(p, r) absent m` (FR-153): `r`'s presence in `p`, per `m`.
    Lookup {
        /// The queried type `T`.
        target: TypeForm,
        /// The population operand `p`.
        population: ExprId,
        /// The reference operand `r`.
        reference: ExprId,
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
        receiver: ExprId,
        /// The unqualified member name.
        member: String,
        /// Arguments in source order.
        arguments: Vec<ExprId>,
    },
    /// `pre(e)` (FR-153): `e`, evaluated with every `allInstances`/`lookup`
    /// underneath it reading its population operand's invocation pre state
    /// instead of the ambient post state. A caller-side anchor operation,
    /// valid only where a checked declaration's postcondition body admits
    /// it; `e`'s own type is unchanged.
    Pre(ExprId),
    /// `self` (FR-102, ADR-012 §15.2): the state clause's current object.
    /// Owned by `ProtocolClause`, which reads the clause's own observation
    /// to give it a value; `Value`'s evaluator never meets this node
    /// (legal only inside a state clause).
    SelfRef,
    /// `result` (FR-102, ADR-012 §15.2): an operation postcondition's
    /// result value. Owned by `ProtocolClause`, for the same reason as
    /// [`Self::SelfRef`]. Nothing here checks that it appears only in a
    /// postcondition; S3 does (FR-104).
    Result,
    /// `reaches(source, target, edge)` (FR-102, ADR-012 §15.2): whether
    /// `target` is reachable from `source` by following `edge` in the
    /// state clause's own observation. Owned by `StateModel`, which reads
    /// object fields (§15.2's "Field reads, `deref`, `reaches` and object
    /// identity equality"); legal only inside a state clause (FR-104).
    /// `edge` is one member name and its span: a qualified spelling names
    /// nothing more than the operands' own type, which S3 resolves, so a
    /// multi-segment edge refuses at S2 (FR-102-AC-3).
    Reaches {
        /// The source reference.
        source: ExprId,
        /// The target reference.
        target: ExprId,
        /// The edge member name, as spelled.
        edge: String,
        /// The span of the edge member name.
        edge_span: Span,
    },
    /// FR-063/S2: exists only so `--cfg seam_probe` makes every
    /// match over `ExprNode` outside this module non-exhaustive. Never
    /// constructed outside the probe build.
    #[cfg(seam_probe)]
    __SeamProbe,
}

impl ExprNode {
    /// The direct subexpressions, in location-index order: a location's
    /// child index `i` names the `i`th of them.
    pub fn children(&self) -> Vec<ExprId> {
        match self {
            Self::Boolean(_)
            | Self::Integer(_)
            | Self::Rational(..)
            | Self::Name(_)
            | Self::SelfRef
            | Self::Result => Vec::new(),
            Self::Let { value, body, .. } => vec![*value, *body],
            Self::If {
                condition,
                then,
                otherwise,
            } => vec![*condition, *then, *otherwise],
            Self::Binary { left, right, .. } => vec![*left, *right],
            Self::Negate(operand)
            | Self::Not(operand)
            | Self::Present(operand)
            | Self::Value(operand)
            | Self::Deref(operand)
            | Self::Flatten(operand)
            | Self::Size(operand)
            | Self::Field { operand, .. }
            | Self::Convert { operand, .. }
            | Self::Pre(operand) => vec![*operand],
            Self::Call { arguments, .. } => arguments.clone(),
            Self::Record { fields, .. } => fields
                .iter()
                .filter_map(|(_, initializer)| match initializer {
                    FieldInitializer::Value(expression) => Some(*expression),
                    FieldInitializer::Null => None,
                })
                .collect(),
            Self::Collection { elements, .. } => elements.clone(),
            Self::Query { source, body, .. } => vec![*source, *body],
            Self::Accumulate {
                source,
                step,
                identity,
                ..
            } => {
                let mut children = vec![*source, *step];
                children.extend(*identity);
                children
            }
            Self::Count {
                source, predicate, ..
            } => vec![*source, *predicate],
            Self::Sum {
                source, summand, ..
            } => vec![*source, *summand],
            Self::Contains { collection, item } => vec![*collection, *item],
            Self::AllInstances { population, .. } => vec![*population],
            Self::Lookup {
                population,
                reference,
                ..
            } => vec![*population, *reference],
            Self::Dispatch {
                receiver,
                arguments,
                ..
            } => {
                let mut children = vec![*receiver];
                children.extend(arguments.iter().copied());
                children
            }
            Self::Reaches { source, target, .. } => vec![*source, *target],
            #[cfg(seam_probe)]
            Self::__SeamProbe => unreachable!("never constructed outside the probe build"),
        }
    }

    /// The names this node binds, in binding order: a `let`'s name, a
    /// query's, count's or sum's binder, or an accumulation's accumulator
    /// then its binder. Empty for every other node.
    pub fn binders(&self) -> Vec<&str> {
        match self {
            Self::Let { name: binder, .. }
            | Self::Query { binder, .. }
            | Self::Count { binder, .. }
            | Self::Sum { binder, .. } => vec![binder],
            Self::Accumulate {
                accumulator,
                binder,
                ..
            } => vec![accumulator, binder],
            Self::Boolean(_)
            | Self::Integer(_)
            | Self::Rational(..)
            | Self::Name(_)
            | Self::SelfRef
            | Self::Result
            | Self::If { .. }
            | Self::Binary { .. }
            | Self::Negate(_)
            | Self::Not(_)
            | Self::Present(_)
            | Self::Value(_)
            | Self::Deref(_)
            | Self::Flatten(_)
            | Self::Size(_)
            | Self::Field { .. }
            | Self::Convert { .. }
            | Self::Pre(_)
            | Self::Call { .. }
            | Self::Record { .. }
            | Self::Collection { .. }
            | Self::Contains { .. }
            | Self::AllInstances { .. }
            | Self::Lookup { .. }
            | Self::Dispatch { .. }
            | Self::Reaches { .. } => Vec::new(),
            #[cfg(seam_probe)]
            Self::__SeamProbe => unreachable!("never constructed outside the probe build"),
        }
    }

    /// The name this node reads and the names it binds, writable, with no
    /// child id in reach.
    fn spellings_mut(&mut self) -> Spellings<'_> {
        match self {
            Self::Name(name) => Spellings {
                reference: Some(name),
                binders: Vec::new(),
            },
            Self::Let { name: binder, .. }
            | Self::Query { binder, .. }
            | Self::Count { binder, .. }
            | Self::Sum { binder, .. } => Spellings {
                reference: None,
                binders: vec![binder],
            },
            Self::Accumulate {
                accumulator,
                binder,
                ..
            } => Spellings {
                reference: None,
                binders: vec![accumulator, binder],
            },
            Self::Boolean(_)
            | Self::Integer(_)
            | Self::Rational(..)
            | Self::SelfRef
            | Self::Result
            | Self::If { .. }
            | Self::Binary { .. }
            | Self::Negate(_)
            | Self::Not(_)
            | Self::Present(_)
            | Self::Value(_)
            | Self::Deref(_)
            | Self::Flatten(_)
            | Self::Size(_)
            | Self::Field { .. }
            | Self::Convert { .. }
            | Self::Pre(_)
            | Self::Call { .. }
            | Self::Record { .. }
            | Self::Collection { .. }
            | Self::Contains { .. }
            | Self::AllInstances { .. }
            | Self::Lookup { .. }
            | Self::Dispatch { .. }
            | Self::Reaches { .. } => Spellings {
                reference: None,
                binders: Vec::new(),
            },
            #[cfg(seam_probe)]
            Self::__SeamProbe => unreachable!("never constructed outside the probe build"),
        }
    }

    /// Apply `shift` to every child id this node names.
    fn shift_children(&mut self, shift: impl Fn(ExprId) -> ExprId) {
        let one = |id: &mut ExprId| *id = shift(*id);
        match self {
            Self::Boolean(_)
            | Self::Integer(_)
            | Self::Rational(..)
            | Self::Name(_)
            | Self::SelfRef
            | Self::Result => {}
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
            | Self::Pre(operand) => one(operand),
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
            }
            | Self::Reaches {
                source: first,
                target: second,
                ..
            } => {
                one(first);
                one(second);
            }
            Self::If {
                condition,
                then,
                otherwise,
            } => {
                one(condition);
                one(then);
                one(otherwise);
            }
            Self::Accumulate {
                source,
                step,
                identity,
                ..
            } => {
                one(source);
                one(step);
                if let Some(identity) = identity {
                    one(identity);
                }
            }
            Self::Call { arguments, .. }
            | Self::Collection {
                elements: arguments,
                ..
            } => arguments.iter_mut().for_each(one),
            Self::Dispatch {
                receiver,
                arguments,
                ..
            } => {
                one(receiver);
                arguments.iter_mut().for_each(one);
            }
            Self::Record { fields, .. } => {
                for (_, initializer) in fields {
                    if let FieldInitializer::Value(value) = initializer {
                        one(value);
                    }
                }
            }
            #[cfg(seam_probe)]
            Self::__SeamProbe => unreachable!("never constructed outside the probe build"),
        }
    }
}

/// One value expression: an arena of [`ExprNode`]s, each stored after its
/// children, with the root last (ADR-030 D-4.2, FR-257).
///
/// Children are named by [`ExprId`], so `Clone`, `PartialEq` and `Drop`
/// run over one flat vector and never recurse, however deep the tree, and
/// [`fmt::Debug`] renders the tree over the walker toolkit's explicit
/// stack. A bottom-up computation is one forward loop over [`Self::iter`].
///
/// Build one with [`ExpressionBuilder`], or compose one from subtrees with
/// the constructors below (`Expression::binary`, `Expression::name`, ...).
/// Every constructor keeps the invariant that each node names only nodes
/// stored before it and the root is the last node.
#[derive(Clone, Eq, PartialEq)]
pub struct Expression {
    /// Never empty: the root is the last node.
    nodes: Vec<ExprNode>,
}

/// A node of an [`Expression`] together with the arena it lives in, so its
/// children can be reached.
#[derive(Clone, Copy)]
pub struct ExprRef<'a> {
    tree: &'a Expression,
    id: ExprId,
    node: &'a ExprNode,
}

impl<'a> ExprRef<'a> {
    /// This node's id in its arena.
    pub fn id(self) -> ExprId {
        self.id
    }

    /// The node's payload.
    pub fn node(self) -> &'a ExprNode {
        self.node
    }

    /// The arena this node lives in.
    pub fn tree(self) -> &'a Expression {
        self.tree
    }

    /// The node `id` names in this node's arena.
    ///
    /// # Panics
    ///
    /// When `id` is from another arena and past this one's last node.
    pub fn at(self, id: ExprId) -> ExprRef<'a> {
        self.tree.at(id)
    }

    /// The direct subexpressions, in location-index order
    /// ([`ExprNode::children`]).
    pub fn children(self) -> Vec<ExprRef<'a>> {
        self.node
            .children()
            .into_iter()
            .map(|id| self.at(id))
            .collect()
    }
}

impl fmt::Debug for ExprRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        render(self.tree, self.id, f)
    }
}

impl Expression {
    /// A tree of the one childless `node`.
    fn single(node: ExprNode) -> Self {
        Self { nodes: vec![node] }
    }

    /// `true` or `false`.
    pub fn boolean(value: bool) -> Self {
        Self::single(ExprNode::Boolean(value))
    }

    /// An integer literal.
    pub fn integer(value: impl Into<Integer>) -> Self {
        Self::single(ExprNode::Integer(value.into()))
    }

    /// `rational(numerator, denominator)`.
    pub fn rational(numerator: Integer, denominator: Integer) -> Self {
        Self::single(ExprNode::Rational(numerator, denominator))
    }

    /// A name.
    pub fn name(name: impl Into<String>) -> Self {
        Self::single(ExprNode::Name(name.into()))
    }

    /// `self`.
    pub fn self_ref() -> Self {
        Self::single(ExprNode::SelfRef)
    }

    /// `result`.
    pub fn result() -> Self {
        Self::single(ExprNode::Result)
    }

    /// `let name = value in body`.
    pub fn let_in(name: impl Into<String>, value: Self, body: Self) -> Self {
        let name = name.into();
        Self::compose([value, body], |[value, body]| ExprNode::Let {
            name,
            value,
            body,
        })
    }

    /// `if condition then then else otherwise`.
    pub fn if_then_else(condition: Self, then: Self, otherwise: Self) -> Self {
        Self::compose(
            [condition, then, otherwise],
            |[condition, then, otherwise]| ExprNode::If {
                condition,
                then,
                otherwise,
            },
        )
    }

    /// `left operator right`.
    pub fn binary(operator: BinaryOperator, left: Self, right: Self) -> Self {
        Self::compose([left, right], |[left, right]| ExprNode::Binary {
            operator,
            left,
            right,
        })
    }

    /// `-operand`.
    pub fn negate(operand: Self) -> Self {
        Self::compose([operand], |[operand]| ExprNode::Negate(operand))
    }

    /// `not operand`.
    pub fn logical_not(operand: Self) -> Self {
        Self::compose([operand], |[operand]| ExprNode::Not(operand))
    }

    /// `operand.field`.
    pub fn field(operand: Self, field: impl Into<String>) -> Self {
        let field = field.into();
        Self::compose([operand], |[operand]| ExprNode::Field { operand, field })
    }

    /// `present(operand)`.
    pub fn present(operand: Self) -> Self {
        Self::compose([operand], |[operand]| ExprNode::Present(operand))
    }

    /// `value(operand)`.
    pub fn value(operand: Self) -> Self {
        Self::compose([operand], |[operand]| ExprNode::Value(operand))
    }

    /// `deref(operand)`.
    pub fn deref(operand: Self) -> Self {
        Self::compose([operand], |[operand]| ExprNode::Deref(operand))
    }

    /// `pre(operand)`.
    pub fn pre(operand: Self) -> Self {
        Self::compose([operand], |[operand]| ExprNode::Pre(operand))
    }

    /// `flatten(operand)`.
    pub fn flatten(operand: Self) -> Self {
        Self::compose([operand], |[operand]| ExprNode::Flatten(operand))
    }

    /// `size(operand)`.
    pub fn size(operand: Self) -> Self {
        Self::compose([operand], |[operand]| ExprNode::Size(operand))
    }

    /// `name(arguments)`.
    pub fn call(name: impl Into<String>, arguments: Vec<Self>) -> Self {
        let name = name.into();
        let mut nodes = Vec::new();
        let arguments = arguments
            .into_iter()
            .map(|argument| append(&mut nodes, argument))
            .collect();
        nodes.push(ExprNode::Call { name, arguments });
        Self { nodes }
    }

    /// `name { field: value, ... }`, where a `None` value is `null`.
    pub fn record(name: impl Into<String>, fields: Vec<(String, Option<Self>)>) -> Self {
        let name = name.into();
        let mut nodes = Vec::new();
        let fields = fields
            .into_iter()
            .map(|(field, value)| {
                let initializer = match value {
                    Some(value) => FieldInitializer::Value(append(&mut nodes, value)),
                    None => FieldInitializer::Null,
                };
                (field, initializer)
            })
            .collect();
        nodes.push(ExprNode::Record { name, fields });
        Self { nodes }
    }

    /// `kind[elements]`.
    pub fn collection(kind: CollectionKind, elements: Vec<Self>) -> Self {
        let mut nodes = Vec::new();
        let elements = elements
            .into_iter()
            .map(|element| append(&mut nodes, element))
            .collect();
        nodes.push(ExprNode::Collection { kind, elements });
        Self { nodes }
    }

    /// `convert<target>(operand)`.
    pub fn convert(target: TypeForm, operand: Self) -> Self {
        Self::compose([operand], |[operand]| ExprNode::Convert { target, operand })
    }

    /// `query(binder in source: body)`.
    pub fn query(query: BinderQuery, binder: impl Into<String>, source: Self, body: Self) -> Self {
        let binder = binder.into();
        Self::compose([source, body], |[source, body]| ExprNode::Query {
            query,
            binder,
            source,
            body,
        })
    }

    /// `fold<A>(accumulator, binder in source: step, identity: i)` or
    /// `reduce<A>(accumulator, binder in source: step)`, `A` being
    /// `accumulator_type`.
    pub fn accumulate(
        form: Accumulation,
        accumulator_type: DeclaredName,
        accumulator: impl Into<String>,
        binder: impl Into<String>,
        source: Self,
        step: Self,
        identity: Option<Self>,
    ) -> Self {
        let mut nodes = Vec::new();
        let source = append(&mut nodes, source);
        let step = append(&mut nodes, step);
        let identity = identity.map(|identity| append(&mut nodes, identity));
        nodes.push(ExprNode::Accumulate {
            form,
            accumulator_type: accumulator_type.name,
            accumulator_type_span: accumulator_type.span,
            accumulator: accumulator.into(),
            binder: binder.into(),
            source,
            step,
            identity,
        });
        Self { nodes }
    }

    /// `count<N>(binder in source: predicate)`, `N` being `result_type`.
    pub fn count(
        result_type: DeclaredName,
        binder: impl Into<String>,
        source: Self,
        predicate: Self,
    ) -> Self {
        let binder = binder.into();
        Self::compose([source, predicate], |[source, predicate]| ExprNode::Count {
            result_type: result_type.name,
            result_type_span: result_type.span,
            binder,
            source,
            predicate,
        })
    }

    /// `sum<N>(binder in source: summand)`, `N` being `result_type`.
    pub fn sum(
        result_type: DeclaredName,
        binder: impl Into<String>,
        source: Self,
        summand: Self,
    ) -> Self {
        let binder = binder.into();
        Self::compose([source, summand], |[source, summand]| ExprNode::Sum {
            result_type: result_type.name,
            result_type_span: result_type.span,
            binder,
            source,
            summand,
        })
    }

    /// `contains(collection, item)`.
    pub fn contains(collection: Self, item: Self) -> Self {
        Self::compose([collection, item], |[collection, item]| {
            ExprNode::Contains { collection, item }
        })
    }

    /// `allInstances<target>(population)`.
    pub fn all_instances(target: TypeForm, population: Self) -> Self {
        Self::compose([population], |[population]| ExprNode::AllInstances {
            target,
            population,
        })
    }

    /// `lookup<target>(population, reference) absent absence`.
    pub fn lookup(
        target: TypeForm,
        population: Self,
        reference: Self,
        absence: AbsenceMode,
    ) -> Self {
        Self::compose([population, reference], |[population, reference]| {
            ExprNode::Lookup {
                target,
                population,
                reference,
                absence,
            }
        })
    }

    /// `receiver.member(arguments)`.
    pub fn dispatch(receiver: Self, member: impl Into<String>, arguments: Vec<Self>) -> Self {
        let mut nodes = Vec::new();
        let receiver = append(&mut nodes, receiver);
        let arguments = arguments
            .into_iter()
            .map(|argument| append(&mut nodes, argument))
            .collect();
        nodes.push(ExprNode::Dispatch {
            receiver,
            member: member.into(),
            arguments,
        });
        Self { nodes }
    }

    /// `reaches(source, target, edge)`.
    pub fn reaches(source: Self, target: Self, edge: impl Into<String>, edge_span: Span) -> Self {
        let edge = edge.into();
        Self::compose([source, target], |[source, target]| ExprNode::Reaches {
            source,
            target,
            edge,
            edge_span,
        })
    }

    /// The tree whose root `node` takes `children`, each appended in order.
    fn compose<const N: usize>(
        children: [Self; N],
        node: impl FnOnce([ExprId; N]) -> ExprNode,
    ) -> Self {
        let mut nodes = Vec::with_capacity(children.iter().map(Self::len).sum::<usize>() + 1);
        let ids = children.map(|child| append(&mut nodes, child));
        nodes.push(node(ids));
        Self { nodes }
    }

    /// The root node.
    pub fn root(&self) -> ExprRef<'_> {
        self.at(ExprId(self.nodes.len().saturating_sub(1)))
    }

    /// The root's id: the last node's.
    pub fn root_id(&self) -> ExprId {
        ExprId(self.nodes.len().saturating_sub(1))
    }

    /// The root node's payload.
    pub fn root_node(&self) -> &ExprNode {
        self.root().node()
    }

    /// The node `id` names, or `None` when `id` is past this arena's last
    /// node.
    pub fn get(&self, id: ExprId) -> Option<ExprRef<'_>> {
        self.nodes.get(id.0).map(|node| ExprRef {
            tree: self,
            id,
            node,
        })
    }

    /// The node `id` names.
    ///
    /// # Panics
    ///
    /// When `id` is from another arena and past this one's last node.
    pub fn at(&self, id: ExprId) -> ExprRef<'_> {
        ExprRef {
            tree: self,
            id,
            node: &self.nodes[id.0],
        }
    }

    /// The number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Always `false`: an expression has at least its root.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// A copy of this tree with each node's payload rewritten by `respell`,
    /// which is handed each node (children first) and a copy of its payload
    /// to change in place: a name, a binder, an operator. The tree's shape
    /// never changes: a rewrite that names other children than the node did
    /// refuses the whole copy with [`ShapeChanged`], naming that node.
    pub fn respelled(
        &self,
        mut respell: impl FnMut(ExprRef<'_>, &mut ExprNode),
    ) -> Result<Self, ShapeChanged> {
        let nodes = self
            .iter()
            .map(|node| {
                let mut copy = node.node().clone();
                respell(node, &mut copy);
                if copy.children() == node.node().children() {
                    Ok(copy)
                } else {
                    Err(ShapeChanged(node.id()))
                }
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { nodes })
    }

    /// A copy of this tree with the names its nodes read and bind
    /// respelled by `respell`, which is handed each node's id (children
    /// first) and that node's [`Spellings`]. It reaches no child id, so the
    /// copy always has this tree's shape.
    pub fn respell_names(&self, mut respell: impl FnMut(ExprId, Spellings<'_>)) -> Self {
        let nodes = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                let mut copy = node.clone();
                respell(ExprId(index), copy.spellings_mut());
                copy
            })
            .collect();
        Self { nodes }
    }

    /// Every node with its id, each after its children, the root last.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = ExprRef<'_>> + '_ {
        self.nodes.iter().enumerate().map(|(index, node)| ExprRef {
            tree: self,
            id: ExprId(index),
            node,
        })
    }
}

/// Append `tree`'s nodes to `nodes`, shifting every child id past the nodes
/// already there, and return the id of `tree`'s root.
fn append(nodes: &mut Vec<ExprNode>, tree: Expression) -> ExprId {
    let offset = nodes.len();
    nodes.extend(tree.nodes.into_iter().map(|mut node| {
        node.shift_children(|id| ExprId(id.0 + offset));
        node
    }));
    ExprId(nodes.len().saturating_sub(1))
}

impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        render(self, self.root_id(), f)
    }
}

/// Render the subtree under `root` as nested text: each node's payload,
/// then its children in brackets. Runs on the walker toolkit's explicit
/// stack, so a deep tree formats in constant native stack.
fn render(tree: &Expression, root: ExprId, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    struct Render<'t, 'f, 'g> {
        tree: &'t Expression,
        f: &'f mut fmt::Formatter<'g>,
        #[cfg(test)]
        gauge: crate::syntax::stack_peak::Gauge,
    }
    impl quire_walk::Walk for Render<'_, '_, '_> {
        /// A node to render, and whether it is its parent's first child.
        type Node = (ExprId, bool);
        /// Whether the node opened a child list to close on exit.
        type Frame = bool;
        type Stop = fmt::Error;

        fn enter(
            &mut self,
            (id, first): (ExprId, bool),
            children: &mut quire_walk::Children<'_, (ExprId, bool)>,
        ) -> ControlFlow<fmt::Error, bool> {
            let write = |result: fmt::Result| match result {
                Ok(()) => ControlFlow::Continue(()),
                Err(error) => ControlFlow::Break(error),
            };
            if !first {
                write(self.f.write_str(", "))?;
            }
            let Some(node) = self.tree.get(id) else {
                return ControlFlow::Break(fmt::Error);
            };
            write(write!(self.f, "{id:?} {:?}", node.node()))?;
            let ids = node.node().children();
            let opened = !ids.is_empty();
            if opened {
                write(self.f.write_str(" ["))?;
            }
            #[cfg(test)]
            self.gauge.enter(ids.len());
            children.extend(
                ids.into_iter()
                    .enumerate()
                    .map(|(index, child)| (child, index == 0)),
            );
            ControlFlow::Continue(opened)
        }

        fn exit(&mut self, opened: bool) -> ControlFlow<fmt::Error> {
            #[cfg(test)]
            self.gauge.exit();
            if opened {
                if let Err(error) = self.f.write_str("]") {
                    return ControlFlow::Break(error);
                }
            }
            ControlFlow::Continue(())
        }
    }
    let mut render = Render {
        tree,
        f,
        #[cfg(test)]
        gauge: crate::syntax::stack_peak::Gauge::new("expression_debug"),
    };
    match quire_walk::walk(&mut render, (root, true)) {
        ControlFlow::Continue(()) => Ok(()),
        ControlFlow::Break(error) => Err(error),
    }
}

/// The names one expression node writes, each writable, and no child id:
/// what [`Expression::respell_names`] hands its closure.
#[derive(Debug)]
pub struct Spellings<'n> {
    /// The name a `Name` node reads; `None` for every other node.
    pub reference: Option<&'n mut String>,
    /// The names the node binds, in [`ExprNode::binders`] order.
    pub binders: Vec<&'n mut String>,
}

/// [`Expression::respelled`]'s rewrite of this node named other children
/// than the node has.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShapeChanged(pub ExprId);

impl fmt::Display for ShapeChanged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a respelling changed the children of expression node {}",
            self.0.index()
        )
    }
}

impl std::error::Error for ShapeChanged {}

/// Builds an [`Expression`] node by node, children first. Every node but
/// the root is the child of exactly one later node, so what it builds is a
/// tree.
#[derive(Clone, Debug, Default)]
pub struct ExpressionBuilder {
    nodes: Vec<ExprNode>,
    /// Whether each stored node is already some later node's child.
    claimed: Vec<bool>,
}

/// Why an [`ExpressionBuilder`] refused a node or a build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TreeRefusal {
    /// A pushed node named a child the builder has not stored.
    UnknownChild(ExprId),
    /// A pushed node named a child that another node, or this node
    /// already, names.
    ClaimedTwice(ExprId),
    /// The build found this node, not the last, with no parent.
    Unclaimed(ExprId),
    /// The build found no node to be the root.
    Empty,
}

impl fmt::Display for TreeRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownChild(id) => {
                write!(f, "expression node {} is not built yet", id.index())
            }
            Self::ClaimedTwice(id) => {
                write!(f, "expression node {} already has a parent", id.index())
            }
            Self::Unclaimed(id) => {
                write!(
                    f,
                    "expression node {} has no parent and is not the root",
                    id.index()
                )
            }
            Self::Empty => f.write_str("an expression needs at least its root"),
        }
    }
}

impl std::error::Error for TreeRefusal {}

impl ExpressionBuilder {
    /// An empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store `node` after every node already stored and return its id.
    /// Refused when `node` names a child this builder has not stored, or
    /// one that already has a parent.
    pub fn push(&mut self, node: ExprNode) -> Result<ExprId, TreeRefusal> {
        let children = node.children();
        for (position, child) in children.iter().enumerate() {
            match self.claimed.get(child.0) {
                None => return Err(TreeRefusal::UnknownChild(*child)),
                Some(true) => return Err(TreeRefusal::ClaimedTwice(*child)),
                Some(false) if children[..position].contains(child) => {
                    return Err(TreeRefusal::ClaimedTwice(*child))
                }
                Some(false) => {}
            }
        }
        for child in &children {
            self.claimed[child.0] = true;
        }
        let next = self.nodes.len();
        self.nodes.push(node);
        self.claimed.push(false);
        Ok(ExprId(next))
    }

    /// Copy every node of `tree` in, after every node already stored, and
    /// return the id of its root, which has no parent yet.
    pub fn graft(&mut self, tree: Expression) -> ExprId {
        let root = append(&mut self.nodes, tree);
        self.claimed.resize(self.nodes.len(), true);
        if let Some(last) = self.claimed.last_mut() {
            *last = false;
        }
        root
    }

    /// The expression whose root is the last node stored. Refused when no
    /// node is stored, or when a node other than the last has no parent.
    pub fn build(self) -> Result<Expression, TreeRefusal> {
        let Some((_, rest)) = self.claimed.split_last() else {
            return Err(TreeRefusal::Empty);
        };
        if let Some(orphan) = rest.iter().position(|claimed| !claimed) {
            return Err(TreeRefusal::Unclaimed(ExprId(orphan)));
        }
        Ok(Expression { nodes: self.nodes })
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
/// never itself reachable through an ordinary named [`ExprNode::Call`]
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
    /// Whether this is a `function` or a `predicate` declaration
    /// (FR-091 "Predicate form").
    kind: DeclarationKind,
    /// Whether an ordinary named [`ExprNode::Call`] elsewhere in the same
    /// package may resolve to this declaration. `false` for every
    /// FR-151 synthesized function (TC-196 D07's bypass:
    /// closing the clause-kind restriction off syntax alone still leaves a
    /// candidate's body or precondition callable by plain name unless this
    /// is also `false`).
    callable_by_name: bool,
    /// The form's byte spans (FR-091-AC-10), when it was read from a source
    /// unit. A declaration built by hand or synthesized (FR-151) has none.
    spans: Option<DeclarationSpans>,
    /// The `using` alias as written, when the declaration was read from a
    /// source unit (FR-091-AC-2). A declaration built by hand has none.
    using: Option<UsingAlias>,
}

/// The declaration kind of a [`FunctionDeclaration`] (FR-091 "Predicate
/// form"): every declaration other than a `predicate` is a `Function`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationKind {
    /// A `function` declaration, or a synthesized function or clause.
    Function,
    /// A `predicate` declaration: a function with a `Boolean` result and no
    /// measure.
    Predicate,
}

/// A declaration's `using` alias as written, with the span of the alias
/// (FR-091-AC-2). S2 resolves nothing: the assembler resolves it to one of
/// the unit's profile selections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsingAlias {
    /// The alias spelling.
    pub alias: String,
    /// The span of the alias.
    pub span: Span,
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
            kind: DeclarationKind::Function,
            callable_by_name: true,
            spans: None,
            using: None,
        }
    }

    /// This declaration with declaration kind `kind`: a `predicate`
    /// declaration is a function whose kind is [`DeclarationKind::Predicate`].
    #[must_use]
    pub fn with_kind(mut self, kind: DeclarationKind) -> Self {
        self.kind = kind;
        self
    }

    /// Whether this is a `function` or a `predicate` declaration.
    pub fn kind(&self) -> DeclarationKind {
        self.kind
    }

    /// A declaration checked as `clause_kind`, never reachable through an
    /// ordinary named [`ExprNode::Call`]: an invariant or precondition
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
            kind: DeclarationKind::Function,
            callable_by_name: false,
            spans: None,
            using: None,
        }
    }

    /// This declaration with its `using` alias as written (FR-091-AC-2).
    #[must_use]
    pub fn with_using(mut self, using: UsingAlias) -> Self {
        self.using = Some(using);
        self
    }

    /// The `using` alias as written, when the declaration was read from a
    /// source unit.
    pub fn using(&self) -> Option<&UsingAlias> {
        self.using.as_ref()
    }

    /// The clause this declaration's body is checked as:
    /// [`ClauseKind::Body`] for [`Self::new`], the declared kind for
    /// [`Self::clause`].
    pub fn clause_kind(&self) -> ClauseKind {
        self.clause_kind
    }

    /// Whether an ordinary named [`ExprNode::Call`] may resolve to this
    /// declaration: `true` for [`Self::new`], `false` for [`Self::clause`].
    pub fn callable_by_name(&self) -> bool {
        self.callable_by_name
    }

    /// This declaration read from a source unit, with its form's spans
    /// (FR-091-AC-10). Refused when the body spans do not have the body's
    /// shape, the measure spans the measure's, or either lies outside the
    /// declaration's span.
    pub fn with_spans(mut self, spans: DeclarationSpans) -> Result<Self, SpansMismatch> {
        spans.fit(&self.body, self.measure.as_ref())?;
        self.spans = Some(spans);
        Ok(self)
    }

    /// The form's byte spans, when it was read from a source unit and they
    /// still fit its body and measure. `body` and `measure` are public, so
    /// an edit after [`Self::with_spans`] can change the tree's shape; the
    /// spans are then withheld rather than let a path reach the wrong node.
    pub fn spans(&self) -> Option<&DeclarationSpans> {
        self.spans
            .as_ref()
            .filter(|spans| spans.fit(&self.body, self.measure.as_ref()).is_ok())
    }
}

/// A declared name and the span of its identifier token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredName {
    /// The name as written.
    pub name: String,
    /// The span of the name's identifier.
    pub span: Span,
}

/// `type Name = T;` (FR-091 "Alias form").
#[derive(Clone, Debug)]
pub struct AliasForm {
    /// The declared name.
    pub name: DeclaredName,
    /// The aliased type.
    pub target: TypeForm,
}

/// One `f: T;` or `f: T?;` field of a record form.
#[derive(Clone, Debug)]
pub struct RecordFieldForm {
    /// The field name.
    pub name: String,
    /// The field's declared type.
    pub type_form: TypeForm,
    /// Whether the field is written with the optional marker `?`.
    pub optional: bool,
}

/// `record Name { f: T; ... }` (FR-091 "Record form").
#[derive(Clone, Debug)]
pub struct RecordForm {
    /// The declared name.
    pub name: DeclaredName,
    /// The fields in source order.
    pub fields: Vec<RecordFieldForm>,
}

/// `tuple Name(T, ...);` (FR-091 "Tuple form").
#[derive(Clone, Debug)]
pub struct TupleForm {
    /// The declared name.
    pub name: DeclaredName,
    /// The element types in source order.
    pub elements: Vec<TypeForm>,
}

/// One member of an [`EnumForm`]: its case name, and its display string as
/// spelled when `= "text"` is written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumMemberForm {
    /// The case name and the span of its identifier.
    pub case: DeclaredName,
    /// The display string exactly as spelled, quotes included, with the
    /// span of the string literal; `None` when none is written. It enters
    /// no identity and no comparison (QSpec FR-141).
    pub display: Option<(String, Span)>,
}

/// `enum Name { A, B = "text" }` and `ordered enum Name { .. }` (FR-091
/// "Enum form").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumForm {
    /// The declared name.
    pub name: DeclaredName,
    /// Whether the declaration is written `ordered enum`.
    pub ordered: bool,
    /// The members in source order.
    pub members: Vec<EnumMemberForm>,
}

/// A qualified name as written, with `::` separators, and its span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameForm {
    /// The name as written.
    pub name: String,
    /// The span of the name.
    pub span: Span,
}

/// The operator that precedes a dimension term.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TermOperator {
    /// `*`.
    Multiply,
    /// `/`.
    Divide,
}

/// One term of a derived dimension: `Name` or `Name^-2` (FR-091 "Dimension
/// form").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionTermForm {
    /// The operator before the term; `None` for the first term.
    pub operator: Option<TermOperator>,
    /// The dimension's qualified name, as written.
    pub name: NameForm,
    /// The exponent when `^` is written, with the span of the signed
    /// integer.
    pub exponent: Option<(Integer, Span)>,
}

/// `dimension Name;` (a base dimension, no terms) or `dimension Name = T
/// * U / V;` (FR-091 "Dimension form").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionForm {
    /// The declared name.
    pub name: DeclaredName,
    /// The terms in source order; empty when no `=` is written.
    pub terms: Vec<DimensionTermForm>,
}

/// Which spelling an [`ExactNumberForm`] was written in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactNumberKind {
    /// `rational(numerator, denominator)`.
    Rational,
    /// `decimal(coefficient, scale)`.
    Decimal,
}

/// An exact number as written, neither reduced nor checked (FR-091 "Unit
/// form"): the assembler reduces it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactNumberForm {
    /// The spelling kind.
    pub kind: ExactNumberKind,
    /// The numerator, or the coefficient.
    pub first: Integer,
    /// The denominator, or the scale.
    pub second: Integer,
    /// The span of the whole exact number.
    pub span: Span,
}

/// `unit name : Dimension = scale [* target] [+ offset];` (FR-091 "Unit
/// form").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitForm {
    /// The declared name.
    pub name: DeclaredName,
    /// The dimension's qualified name after `:`.
    pub dimension: NameForm,
    /// The scale, the exact number after `=`.
    pub scale: ExactNumberForm,
    /// The target unit's qualified name after `*`, when written.
    pub target: Option<NameForm>,
    /// The offset, the exact number after `+`, when written.
    pub offset: Option<ExactNumberForm>,
}

/// A [`StateClauseForm`]'s kind (FR-102 "Outputs"): read from the leading
/// token alone, `invariant` gives [`Self::Invariant`], `pre` gives
/// [`Self::Precondition`] and `post` gives [`Self::Postcondition`]. An
/// invariant's `at current` observation is carried as this kind itself: the
/// grammar admits no other observation for an invariant, so no separate
/// observation member is added.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StateClauseKind {
    /// `invariant N using p on M::T at current { e }`.
    Invariant,
    /// `pre N using p on M::T::op { e }`.
    Precondition,
    /// `post N using p on M::T::op { e }`.
    Postcondition,
}

/// `invariant N using p on M::T at current { e }`, `pre N using p on
/// M::T::op { e }` or `post N using p on M::T::op { e }` (FR-102 "Outputs",
/// ADR-012 §15.2: owned by `ProtocolClause`). S2 keeps the `using` alias,
/// the model context and the operation member spelled and spanned,
/// unresolved: it resolves none of them, and neither does it check where
/// the clause may appear; the assembler and S3 do (FR-104).
#[derive(Clone, Debug)]
pub struct StateClauseForm {
    /// Invariant, precondition or postcondition.
    pub kind: StateClauseKind,
    /// The declared clause name.
    pub name: DeclaredName,
    /// The `using` alias.
    pub profile: UsingAlias,
    /// The `model-name` (`M::T`) context, as spelled.
    pub context: NameForm,
    /// The operation member name, for a `pre` or `post` clause; `None` for
    /// an invariant.
    pub operation: Option<DeclaredName>,
    /// The body.
    pub body: Expression,
    /// The form's byte spans (FR-091-AC-10 applied to a state clause): the
    /// whole declaration's, and one per node of the body. A state clause
    /// declares no `decreases` measure, so `measure` is always `None`.
    pub spans: DeclarationSpans,
}

/// One segment of a `NodeReference`, exactly as written: its text and the
/// span of that segment alone (FR-112 "Outputs"). `Main::Applied` has two
/// segments, `Tried` has one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorSegment {
    /// The segment's text, unjoined from its neighbors.
    pub text: String,
    /// The span of this segment alone.
    pub span: Span,
}

/// A protocol node reference's segments in source order, with the span of
/// the whole reference (FR-112 "Outputs"). S2 keeps the segments exactly as
/// written: it does not join them into one qualified string, drop a
/// segment, or read a segment as a display name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorForm {
    /// The reference's segments, in source order.
    pub segments: Vec<AnchorSegment>,
    /// The span of the whole reference.
    pub span: Span,
}

/// One named control that lexically encloses a scoped anchor's reference
/// (FR-112 "Outputs"): a `sequence`, `choice`, `parallel`, `branch`,
/// `case` or `repeat`'s own declared name, with the span of that name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeName {
    /// The enclosing control's declared name.
    pub name: String,
    /// The span of that name's identifier.
    pub span: Span,
}

/// The index of one [`ScopeEntry`] in its protocol's
/// [`ProtocolDeclarationForm::scopes`] arena. `None` in a `scope` field is
/// the protocol's empty top-level scope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScopeId(pub(crate) usize);

impl ScopeId {
    /// This scope's index in [`ProtocolDeclarationForm::scopes`].
    pub fn index(self) -> usize {
        self.0
    }
}

/// One named control's scope: its own name and the scope that encloses it.
/// A chain of named controls shares each enclosing entry, so a protocol of
/// any nesting depth stores each scope once rather than copying the whole
/// enclosing path into every anchor, declaration and binder (ADR-030).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeEntry {
    /// The control whose name opens this scope.
    pub name: ScopeName,
    /// The enclosing scope; `None` when this scope sits directly in the
    /// protocol's top level.
    pub parent: Option<ScopeId>,
}

/// Which reference position a [`ScopedAnchorForm`] fills (FR-112
/// "Outputs").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnchorSite {
    /// The `NodeReference` after `of` in a `receive` event node.
    ReceiveOf,
    /// The `NodeReference` after `of` in an `effect` event node.
    EffectOf,
    /// The `NodeReference` after `for` in an `event` node.
    EventFor,
    /// The `NodeReference` after `after` in an `await` control.
    AwaitAfter,
    /// The `NodeReference` after `for` in a `compensate` declaration.
    CompensateFor,
    /// The `NodeReference` after `commit` in a `compensate` declaration,
    /// when it is not `never`.
    CompensateCommit,
}

/// One protocol node reference and the lexical control scope it is written
/// in (FR-112, ADR-012 §12.2 Form row). A scoped anchor is a clause of its
/// construct with its own refusal causes (FR-113), so it is a typed
/// subnode, not a string kept inside the construct's form. It is
/// represented only inside S2 and S3: it has no `CheckedClauseKind`
/// variant and no checked-package/v2 node. S2 records the reference and
/// where it was written; it resolves nothing (FR-113 is S3's job).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopedAnchorForm {
    /// Which reference position this anchor fills.
    pub site: AnchorSite,
    /// The reference itself, as written.
    pub anchor: AnchorForm,
    /// The innermost named control enclosing the reference;
    /// [`ProtocolDeclarationForm::scope_names`] lists the whole path,
    /// outermost first, from the protocol's `run` control. `None` for a
    /// reference in a protocol-level requirement (a `compensate`
    /// declaration).
    pub scope: Option<ScopeId>,
    /// The channel the owning `receive` event node is written `via`, for a
    /// `receive-of` anchor (FR-113's channel-mismatch check); `None` for
    /// every other site.
    pub channel: Option<String>,
}

/// The kind of static protocol node a [`ProtocolNodeDeclaration`] names, or
/// a [`ScopedAnchorForm`] resolves to (FR-113 Inputs and its wrong-kind
/// table). Each variant's [`Self::label`] is the construct's own keyword,
/// the word FR-113's refusals name a target's kind by.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolNodeKind {
    /// A `send` event node.
    Send,
    /// A `receive` event node.
    Receive,
    /// An `attempt` event node.
    Attempt,
    /// An `effect` event node.
    Effect,
    /// An `event` event node.
    Event,
    /// A bare `commit` node inside a control tree.
    Commit,
    /// The protocol's `finish` node.
    Finish,
    /// A top-level `compensate` template.
    CompensateTemplate,
    /// A `sequence` structural control.
    Sequence,
    /// A `choice` structural control.
    Choice,
    /// A `parallel` structural control.
    Parallel,
    /// A `repeat` structural control.
    Repeat,
    /// A `branch` of a `parallel`.
    Branch,
    /// A `case` of a `choice`.
    Case,
    /// An `await` structural control.
    Await,
    /// A `check` control.
    Check,
}

impl ProtocolNodeKind {
    /// The construct's own keyword (FR-113's refusals name a target's kind
    /// by this word).
    pub fn label(self) -> &'static str {
        match self {
            Self::Send => "send",
            Self::Receive => "receive",
            Self::Attempt => "attempt",
            Self::Effect => "effect",
            Self::Event => "event",
            Self::Commit => "commit",
            Self::Finish => "finish",
            Self::CompensateTemplate => "compensate",
            Self::Sequence => "sequence",
            Self::Choice => "choice",
            Self::Parallel => "parallel",
            Self::Repeat => "repeat",
            Self::Branch => "branch",
            Self::Case => "case",
            Self::Await => "await",
            Self::Check => "check",
        }
    }
}

/// One static protocol node a scope declares directly (FR-113 Inputs: "for
/// each scope ..., the names of the static nodes it declares directly,
/// with their spans"). `scope` follows the same convention as
/// [`ScopedAnchorForm::scope`]: the named controls that enclose this
/// declaration, outermost first, empty for a top-level declaration (the
/// `run` control itself, the `finish` node and each `compensate`
/// template). A `send` or `receive` event node also carries the channel it
/// is written `via`, for FR-113's channel-mismatch check; every other kind
/// carries `None`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolNodeDeclaration {
    /// The kind of node this declaration is.
    pub kind: ProtocolNodeKind,
    /// The declared name and its span.
    pub name: DeclaredName,
    /// The innermost named control enclosing this declaration; `None` at
    /// the top level.
    pub scope: Option<ScopeId>,
    /// The channel a `send` or `receive` is written `via`; `None` for
    /// every other kind.
    pub channel: Option<String>,
}

/// A binder's syntactic role (FR-113 "Refusals": a record binder, a
/// capture, a compensation trigger and a retry or recovery parameter all
/// share the one no-shadowing rule, QSpec `shared-grammar.md`: "the same
/// no-shadowing rule applies to binders and captures").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinderKind {
    /// The protocol's own `over (p)` input parameter, visible everywhere in
    /// the protocol (FR-113 "Refusals"; composed lane: `BinderKind::Input`).
    Input,
    /// The protocol's own `activation on each (p) [when (...)]` parameter,
    /// when written (`on origin` binds nothing); visible everywhere in the
    /// protocol (FR-113 "Refusals").
    ActivationParameter,
    /// The `as (x: T)` of an event node (`send`, `receive`, `attempt`,
    /// `effect` or `event`), a `commit`, a `finish` or a `compensate`
    /// declaration's own bound parameter.
    RecordBinder,
    /// A `capture p = e;`, at the protocol's top level or inside a
    /// `compensate` declaration.
    Capture,
    /// A `compensate` declaration's `activate first (p)` trigger parameter.
    Trigger,
    /// One of a `compensate` declaration's `retry (a, b)` parameters.
    RetryParameter,
    /// A `compensate` declaration's `recover (p)` parameter.
    RecoveryParameter,
}

/// One binder FR-113's no-shadowing rule checks: its own declared name and
/// the scope it is visible in. `scope` follows the same convention
/// [`ScopedAnchorForm::scope`] and [`ProtocolNodeDeclaration::scope`] do:
/// the named controls enclosing the binder, outermost first, empty at the
/// protocol's top level. A binder inside a `compensate` declaration is
/// scoped by that declaration's own name (the same way a structural
/// control's children are scoped by its name); the `compensate`
/// declaration's own record binder is scoped one level shallower, at the
/// empty top-level scope its own declaration lives in (FR-113-AC-5: a
/// nested `capture` shadows it).
///
/// `scope` locates a binder for FR-114's later binding pass; it plays no
/// part in FR-113's own no-shadowing check. QSpec `shared-grammar.md`
/// requires every binder "unique in their enclosing declaration" -- the
/// checked protocol as a whole, not any one lexical scope inside it -- so
/// the check (`qsl-semantics`' `shadow_refusals`) treats every binder name
/// in the protocol as one flat namespace: a `finish` binder may shadow a
/// `run`-tree binder, two different `compensate` declarations' binders of
/// one name collide, and sibling `case`/`branch` binders of one name
/// collide, the same as the composed lane's own `DuplicateBinder` checker
/// (`src/linking/composed/scopes.rs`). Only a *different* protocol
/// declaration is a separate "enclosing declaration": two protocols may
/// reuse a binder name (FR-113 "Refusals").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinderForm {
    /// The binder's syntactic role.
    pub kind: BinderKind,
    /// The declared name and its span.
    pub name: DeclaredName,
    /// The innermost named control enclosing the binder; `None` at the
    /// protocol's top level.
    pub scope: Option<ScopeId>,
    /// The binder's declared type (`x: T`'s `T`), as spelled: S3 resolves
    /// it against the package scope.
    pub value_type: TypeForm,
}

/// One `role R on M::T;` or `role R each M::T from ...;` declaration of a
/// protocol: its declared name and the object type it is `on`
/// (or replicated `each` over), spelled and unresolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleForm {
    /// The declared role name and its span.
    pub name: DeclaredName,
    /// The `M::T` object type the role is `on` or `each`, as spelled.
    pub context: NameForm,
}

/// A part of a protocol declaration that is neither a static node (a
/// [`ProtocolNodeDeclaration`]), a binder, a role, nor an attempt's own
/// operation and `contracts` list. S2 records only that it is
/// present and where, so S3 can tell a protocol it checks in full from one
/// holding content it does not check yet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolConstructKind {
    /// A `capture p = e;` at the protocol's top level.
    Capture,
    /// An `activation on each (p) [when (...)]` (rather than `on origin`).
    ActivationEach,
    /// A replicated `role R each M::T from ... max N lifetime ...;`.
    ReplicatedRole,
    /// A `relationship r = M::R;` declaration.
    Relationship,
    /// A `channel C from R to R carries T ...;` declaration.
    Channel,
    /// A `requires temporal T;` requirement.
    Requirement,
    /// A `related by r (e, e)` clause of an event node.
    Related,
}

impl ProtocolConstructKind {
    /// The construct's own keyword phrase, for a refusal naming it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::ActivationEach => "activation on each",
            Self::ReplicatedRole => "role each",
            Self::Relationship => "relationship",
            Self::Channel => "channel",
            Self::Requirement => "requires temporal",
            Self::Related => "related by",
        }
    }
}

/// One [`ProtocolConstructKind`] occurrence and its span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolConstructForm {
    /// Which construct this is.
    pub kind: ProtocolConstructKind,
    /// The construct's own span.
    pub span: Span,
}

/// The `{ e }` body block of an event node, a `commit`, a `check` or the
/// `finish` node, recorded by the index of the static declaration
/// that owns it. `constant` is `Some(b)` exactly when the body is the bare
/// Boolean literal `b` (`{ true }` or `{ false }`), `None` for any other
/// body; S2 builds no expression tree for it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolBodyForm {
    /// The index into [`ProtocolDeclarationForm::declarations`] of the node
    /// that owns this body.
    pub owner: usize,
    /// The literal the body consists of, when it is a bare Boolean literal.
    pub constant: Option<bool>,
    /// The span of the body's expression.
    pub span: Span,
}

/// One `attempt`'s `on M::T::op` operation name and `contracts [...]` list
/// (FR-114 "Inputs"), captured alongside its own `ProtocolNodeKind::Attempt`
/// static declaration. S2 keeps the context and operation member spelled
/// and unresolved, the same contract [`StateClauseForm::operation`] keeps
/// (FR-102): the assembler resolves the operation (FR-114 "Behavior",
/// mirroring FR-104's own resolution of a state clause's operation), and S3
/// checks each `contracts` entry against the resolved anchor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttemptForm {
    /// The index into [`ProtocolDeclarationForm::declarations`] of this
    /// attempt's own static declaration (`ProtocolNodeKind::Attempt`), so a
    /// later stage locates the declaration this operation and these
    /// contracts belong to without a second name lookup.
    pub declaration: usize,
    /// The role the attempt is performed `by`, as spelled.
    pub role: DeclaredName,
    /// The `M::T` context of `on M::T::op`, as spelled.
    pub context: NameForm,
    /// The operation member name `op`, as spelled.
    pub operation: DeclaredName,
    /// The `contracts [...]` list, in source order; empty when written
    /// `contracts []`.
    pub contracts: Vec<DeclaredName>,
}

/// `protocol Name using p over (params) activation { ... run Control
/// Finish }` (FR-112 "Outputs", ADR-012 §12.2). S2 builds the scoped
/// anchors and the declaration collection FR-113 resolves them against,
/// and the parts S3 needs to tell a protocol it checks in full
/// from one it does not: the `using` alias, the roles, each body block, and
/// every other construct present ([`Self::constructs`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolDeclarationForm {
    /// The declared protocol name.
    pub name: DeclaredName,
    /// The profile alias the protocol is written `using`.
    pub using: UsingAlias,
    /// Every role declaration, in source order.
    pub roles: Vec<RoleForm>,
    /// Every body block of an event node, `commit`, `check` or `finish`, in
    /// source order. Bodies inside a `compensate` template, a `case`'s
    /// `when`, a `repeat`'s blocks or a `choice`'s visibility are not
    /// recorded: each of those constructs is itself a static declaration S3
    /// reads by kind.
    pub bodies: Vec<ProtocolBodyForm>,
    /// Every capture, `activation on each`, replicated role,
    /// relationship, channel, requirement and `related by` clause, in
    /// source order.
    pub constructs: Vec<ProtocolConstructForm>,
    /// Every named control's scope, each entered once, in source order of
    /// the controls that open them. Anchors, declarations and binders name
    /// their scope by index here.
    pub scopes: Vec<ScopeEntry>,
    /// Every scoped anchor the declaration holds, in source order of their
    /// references (FR-112-AC-1).
    pub scoped_anchors: Vec<ScopedAnchorForm>,
    /// Every static node the declaration's scopes declare directly, in
    /// source order (FR-113 Inputs).
    pub declarations: Vec<ProtocolNodeDeclaration>,
    /// Every binder the declaration holds, in source order (FR-113
    /// "Refusals" binder no-shadowing rule).
    pub binders: Vec<BinderForm>,
    /// Every `attempt`'s operation name and `contracts` list, in source
    /// order (FR-114 "Inputs").
    pub attempts: Vec<AttemptForm>,
}

impl ProtocolDeclarationForm {
    /// The names of the named controls that make up `scope`, outermost
    /// first; empty for the top level (`None`).
    ///
    /// # Panics
    ///
    /// Panics if `scope` is not an index of this protocol's
    /// [`Self::scopes`].
    pub fn scope_names(&self, scope: Option<ScopeId>) -> Vec<&ScopeName> {
        let mut names = Vec::new();
        let mut current = scope;
        // Each entry's parent was entered before it, so the chain ends
        // within `scopes.len()` steps; the bound keeps a hand-built cycle
        // from looping.
        while let Some(id) = current.filter(|_| names.len() < self.scopes.len()) {
            let entry = &self.scopes[id.0];
            names.push(&entry.name);
            current = entry.parent;
        }
        names.reverse();
        names
    }
}

/// A temporal operator: a connective or a unary or binary temporal operator
/// of a `temporal` clause's formula (FR-325). Closed: S3 matches it
/// exhaustively.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalOperator {
    /// `not`.
    Not,
    /// `and`.
    And,
    /// `or`.
    Or,
    /// `implies`.
    Implies,
    /// `eventually`.
    Eventually,
    /// `always`.
    Always,
    /// `once`.
    Once,
    /// `historically`.
    Historically,
    /// `until`.
    Until,
    /// `release`.
    Release,
    /// `since`.
    Since,
    /// `triggered`.
    Triggered,
}

/// The upper bound of an [`IntervalForm`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntervalUpper {
    /// `[a, b]`: the bound `b`.
    Finite(u64),
    /// `[a, *]`.
    Open,
}

/// An operator's `[a, b]` or `[a, *]` interval, as written: S2 builds an
/// interval with `a > b` and refuses nothing about it, because whether an
/// interval is admitted, required or well-ordered is S3's decision
/// (FR-325, FR-326).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntervalForm {
    /// The lower bound `a`.
    pub lower: u64,
    /// The upper bound.
    pub upper: IntervalUpper,
    /// The span of the whole bracketed interval.
    pub span: Span,
}

/// The index of one node of a [`TemporalFormulaForm`] arena. Minted only by
/// the arena that holds the node.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TemporalNodeId(pub(crate) usize);

impl TemporalNodeId {
    /// The node's position in its arena.
    pub fn index(self) -> usize {
        self.0
    }
}

/// An expression with the span of each of its nodes: a `holds(e)` operand,
/// an activation's `when (e)` condition or a capture's value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpressionForm {
    /// The expression.
    pub expression: Expression,
    /// The span of each expression node.
    pub spans: ExpressionSpans,
}

/// One operator node of a temporal formula (FR-325 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalOperatorForm {
    /// Which operator.
    pub operator: TemporalOperator,
    /// The operator's interval; `None` when it is written with none.
    pub interval: Option<IntervalForm>,
    /// The operands in source order: one for a unary operator, two for a
    /// binary one.
    pub operands: Vec<TemporalNodeId>,
    /// The span of the whole operator application, operands included.
    pub span: Span,
    /// The span of the operator's own token.
    pub operator_span: Span,
}

/// One node of a temporal formula.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalNodeForm {
    /// `true` or `false`.
    Constant {
        /// The constant.
        value: bool,
        /// The constant's span.
        span: Span,
    },
    /// `holds(e)`.
    Holds {
        /// The state condition.
        condition: ExpressionForm,
        /// The span of the whole `holds(e)`.
        span: Span,
    },
    /// An operator applied to its operands.
    Operator(TemporalOperatorForm),
}

/// A temporal formula: an arena of nodes naming their operands by id, so
/// building, comparing, cloning and dropping one never recurses however
/// deep the formula (ADR-030 D-4.2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalFormulaForm {
    nodes: Vec<TemporalNodeForm>,
    root: TemporalNodeId,
}

impl TemporalFormulaForm {
    pub(crate) fn new(nodes: Vec<TemporalNodeForm>, root: TemporalNodeId) -> Self {
        Self { nodes, root }
    }

    /// The formula's root node.
    pub fn root(&self) -> TemporalNodeId {
        self.root
    }

    /// The node `id` names, or `None` when `id` is not this arena's.
    pub fn node(&self, id: TemporalNodeId) -> Option<&TemporalNodeForm> {
        self.nodes.get(id.0)
    }

    /// Every node, in arena order.
    pub fn nodes(&self) -> &[TemporalNodeForm] {
        &self.nodes
    }
}

/// A `( name : Type )` parameter as written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterForm {
    /// The parameter name.
    pub name: DeclaredName,
    /// The declared type, as spelled.
    pub value_type: TypeForm,
}

/// A temporal clause's activation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActivationForm {
    /// `on origin`.
    Origin,
    /// `on each (p: T) [when (e)]`.
    Each {
        /// The activation parameter.
        parameter: ParameterForm,
        /// The `when` condition, when written.
        when: Option<ExpressionForm>,
    },
}

/// A fairness constraint's kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FairnessKind {
    /// `weak`.
    Weak,
    /// `strong`.
    Strong,
}

/// A fairness constraint's granularity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FairnessGranularity {
    /// `whole`.
    Whole,
    /// `each`.
    Each,
}

/// One `fair [kind] [granularity] Operation;` constraint, as written
/// (FR-325 "Outputs"). The operation stays a spelled, unresolved name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FairnessConstraintForm {
    /// The constraint's kind; `None` when none is written.
    pub kind: Option<FairnessKind>,
    /// The operation name.
    pub operation: NameForm,
    /// The granularity; `None` when none is written.
    pub granularity: Option<FairnessGranularity>,
    /// The span of the whole constraint.
    pub span: Span,
}

/// One `capture name: Type = value;` of a temporal clause, as written
/// (FR-325 "Outputs"): S2 resolves neither the type nor the value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureForm {
    /// The captured name and its declared type.
    pub parameter: ParameterForm,
    /// The value expression.
    pub value: ExpressionForm,
    /// The span of the whole capture, `;` included.
    pub span: Span,
}

/// A `temporal` clause (FR-325, ADR-011 M-3b, ADR-012 TemporalTrace
/// family). S2 resolves nothing and selects no meaning: the profile alias,
/// the `over` type and each fairness operation stay spelled, and an
/// operator's interval is never judged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalClauseForm {
    /// The declared clause name.
    pub name: DeclaredName,
    /// The `using` alias.
    pub profile: UsingAlias,
    /// The `over (p: T)` parameter.
    pub over: ParameterForm,
    /// The activation.
    pub activation: ActivationForm,
    /// The fairness constraints in source order.
    pub fairness: Vec<FairnessConstraintForm>,
    /// The captures in source order.
    pub captures: Vec<CaptureForm>,
    /// The formula.
    pub formula: TemporalFormulaForm,
}

/// One `Value` parsed declaration form (FR-091 "What a `Value` parsed form
/// carries").
#[derive(Clone, Debug)]
pub enum DeclarationForm {
    /// A `function` declaration, boxed: it is several times the size of
    /// the other forms.
    Function(Box<FunctionDeclaration>),
    /// A `type` alias declaration.
    Alias(AliasForm),
    /// A `record` declaration.
    Record(RecordForm),
    /// A `tuple` declaration.
    Tuple(TupleForm),
    /// An `enum` or `ordered enum` declaration.
    Enum(EnumForm),
    /// A `dimension` declaration.
    Dimension(DimensionForm),
    /// A `unit` declaration.
    Unit(UnitForm),
    /// An `invariant`, `pre` or `post` state clause, boxed for the same
    /// reason as [`Self::Function`] (FR-102).
    StateClause(Box<StateClauseForm>),
    /// A `protocol` declaration (FR-112).
    Protocol(ProtocolDeclarationForm),
    /// A `temporal` clause (FR-325), boxed for the same reason as
    /// [`Self::Function`].
    Temporal(Box<TemporalClauseForm>),
}

/// The longest each explicit heap stack grew in this thread, by name, so a
/// test can show each stack grows by a constant per CST node S1 charged.
#[cfg(test)]
pub(crate) mod stack_peak {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    thread_local! {
        static PEAKS: RefCell<BTreeMap<&'static str, usize>> =
            const { RefCell::new(BTreeMap::new()) };
    }

    /// Record that `stack` holds `len` entries.
    pub(crate) fn note(stack: &'static str, len: usize) {
        PEAKS.with(|peaks| {
            let mut peaks = peaks.borrow_mut();
            let peak = peaks.entry(stack).or_insert(0);
            *peak = (*peak).max(len);
        });
    }

    /// Counts a `quire_walk` walk's task stack from the walker's side:
    /// entering a node trades its enter task for its frame and adds one
    /// task per child, and exiting drops the frame.
    pub(crate) struct Gauge {
        stack: &'static str,
        live: usize,
    }

    impl Gauge {
        /// A gauge for a walk that starts with its root's enter task.
        pub(crate) fn new(stack: &'static str) -> Self {
            Self { stack, live: 1 }
        }

        /// A node was entered and named `children` children.
        pub(crate) fn enter(&mut self, children: usize) {
            self.live += children;
            note(self.stack, self.live);
        }

        /// A node was exited.
        pub(crate) fn exit(&mut self) {
            self.live -= 1;
        }
    }

    /// The longest `stack` grew since the last call, which resets it.
    pub(crate) fn take(stack: &'static str) -> usize {
        PEAKS.with(|peaks| peaks.borrow_mut().remove(stack).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// Nesting depth for the deep-arena tests (ADR-030 D-6).
    const LEVELS: usize = 100_000;

    /// Run `run` on a thread with a 512 KiB stack.
    fn on_small_stack(run: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(run)
            .expect("spawn a 512 KiB thread")
            .join()
            .expect("the arena's traits do not overflow a 512 KiB stack");
    }

    /// The depth of the tree: one forward loop over the arena, each node's
    /// depth one more than its deepest child's.
    fn depth(tree: &Expression) -> usize {
        let mut depths: Vec<usize> = Vec::with_capacity(tree.len());
        for node in tree.iter() {
            let deepest = node
                .node()
                .children()
                .into_iter()
                .map(|child| depths[child.index()])
                .max()
                .unwrap_or(0);
            depths.push(deepest + 1);
        }
        depths.last().copied().unwrap_or(0)
    }

    /// A 100,000-deep tree, alternating single-, two- and many-operand
    /// forms, clones, compares equal to its clone, formats for debug and
    /// drops on a 512 KiB stack.
    #[trace("TC-724", "FR-257-AC-1")]
    #[test]
    fn a_deep_arena_clones_compares_formats_and_drops_on_a_small_stack() {
        on_small_stack(|| {
            let mut builder = ExpressionBuilder::new();
            let mut inner = builder.push(ExprNode::Name("bottom".to_owned())).unwrap();
            for level in 1..LEVELS {
                let mut leaf = || builder.push(ExprNode::Name("x".to_owned())).unwrap();
                let node = match level % 5 {
                    0 => ExprNode::Not(inner),
                    1 => ExprNode::Binary {
                        operator: BinaryOperator::Add,
                        left: inner,
                        right: leaf(),
                    },
                    2 => ExprNode::Let {
                        name: "v".to_owned(),
                        value: leaf(),
                        body: inner,
                    },
                    3 => ExprNode::Call {
                        name: "f".to_owned(),
                        arguments: vec![leaf(), inner],
                    },
                    _ => ExprNode::Record {
                        name: "R".to_owned(),
                        fields: vec![
                            ("a".to_owned(), FieldInitializer::Null),
                            ("b".to_owned(), FieldInitializer::Value(inner)),
                        ],
                    },
                };
                inner = builder.push(node).unwrap();
            }
            let tree = builder.build().expect("the builder holds one tree");
            assert_eq!(depth(&tree), LEVELS);
            let copy = tree.clone();
            assert_eq!(copy, tree);
            let rendered = format!("{tree:?}");
            assert!(rendered.starts_with(&format!("#{} ", tree.len() - 1)));
            assert_eq!(rendered.matches("Name(\"bottom\")").count(), 1);
            drop(copy);
            drop(tree);
        });
    }

    /// Composing subtrees appends each one after the nodes already there and
    /// shifts its ids, so every child still names its own subtree.
    #[trace("TC-724", "FR-257-AC-1")]
    #[test]
    fn composed_subtrees_keep_their_children() {
        let left = Expression::binary(
            BinaryOperator::Subtract,
            Expression::name("a"),
            Expression::name("b"),
        );
        let tree = Expression::binary(
            BinaryOperator::Add,
            left,
            Expression::logical_not(Expression::name("c")),
        );
        assert_eq!(
            format!("{tree:?}"),
            "#5 Binary { operator: Add, left: #2, right: #4 } \
             [#2 Binary { operator: Subtract, left: #0, right: #1 } \
             [#0 Name(\"a\"), #1 Name(\"b\")], #4 Not(#3) [#3 Name(\"c\")]]"
        );
        let built = {
            let mut builder = ExpressionBuilder::new();
            let a = builder.push(ExprNode::Name("a".to_owned())).unwrap();
            let b = builder.push(ExprNode::Name("b".to_owned())).unwrap();
            let left = builder
                .push(ExprNode::Binary {
                    operator: BinaryOperator::Subtract,
                    left: a,
                    right: b,
                })
                .unwrap();
            let c = builder.graft(Expression::logical_not(Expression::name("c")));
            builder
                .push(ExprNode::Binary {
                    operator: BinaryOperator::Add,
                    left,
                    right: c,
                })
                .unwrap();
            builder.build().unwrap()
        };
        assert_eq!(built, tree);
    }

    /// A builder refuses a node that names a child it has not stored, so an
    /// arena never holds an id past its own nodes.
    #[trace("TC-724", "FR-257-AC-1")]
    #[test]
    fn a_builder_refuses_an_unknown_child() {
        let foreign = Expression::logical_not(Expression::name("x")).root_id();
        let mut builder = ExpressionBuilder::new();
        assert_eq!(
            builder.push(ExprNode::Not(foreign)),
            Err(TreeRefusal::UnknownChild(foreign))
        );
        assert_eq!(builder.build(), Err(TreeRefusal::Empty));
    }

    /// A builder refuses a child named by two parents, or twice by one, and
    /// a build that would leave a node other than the root with no parent:
    /// what it builds is always one tree.
    #[trace("TC-724", "FR-257-AC-1")]
    #[test]
    fn a_builder_builds_only_one_tree() {
        let mut builder = ExpressionBuilder::new();
        let x = builder.push(ExprNode::Name("x".to_owned())).unwrap();
        builder.push(ExprNode::Not(x)).unwrap();
        assert_eq!(
            builder.push(ExprNode::Negate(x)),
            Err(TreeRefusal::ClaimedTwice(x))
        );
        let y = builder.push(ExprNode::Name("y".to_owned())).unwrap();
        assert_eq!(
            builder.push(ExprNode::Binary {
                operator: BinaryOperator::Add,
                left: y,
                right: y,
            }),
            Err(TreeRefusal::ClaimedTwice(y))
        );
        assert_eq!(builder.build(), Err(TreeRefusal::Unclaimed(ExprId(1))));
    }

    /// `respell_names` rewrites the names a tree reads and binds, in
    /// binding order, and keeps every child where it was.
    #[trace("TC-724", "FR-257-AC-1")]
    #[test]
    fn respell_names_rewrites_references_and_binders() {
        let tree = Expression::let_in(
            "v",
            Expression::name("x"),
            Expression::binary(
                BinaryOperator::Add,
                Expression::name("v"),
                Expression::integer(1_i64),
            ),
        );
        let renamed = tree.respell_names(|_, spellings| {
            if let Some(name) = spellings.reference {
                name.push('1');
            }
            for binder in spellings.binders {
                binder.push('2');
            }
        });
        assert_eq!(
            renamed,
            Expression::let_in(
                "v2",
                Expression::name("x1"),
                Expression::binary(
                    BinaryOperator::Add,
                    Expression::name("v1"),
                    Expression::integer(1_i64)
                ),
            )
        );
        assert_eq!(renamed.root_node().binders(), ["v2"]);
    }

    /// `respelled` rewrites each node's payload in place and keeps the
    /// tree's shape: a rewrite that renames a name lands, and one that
    /// points a node at another child refuses the copy.
    #[trace("TC-724", "FR-257-AC-1")]
    #[test]
    fn respelled_rewrites_payloads_and_keeps_the_shape() {
        let tree = Expression::binary(
            BinaryOperator::Add,
            Expression::name("x"),
            Expression::name("y"),
        );
        let renamed = tree
            .respelled(|_, node| {
                if let ExprNode::Name(name) = node {
                    name.push('1');
                }
            })
            .expect("a rename keeps the shape");
        assert_eq!(
            renamed,
            Expression::binary(
                BinaryOperator::Add,
                Expression::name("x1"),
                Expression::name("y1"),
            )
        );
        let rewired = tree.respelled(|_, node| {
            if let ExprNode::Binary { left, right, .. } = node {
                std::mem::swap(left, right);
            }
        });
        assert_eq!(rewired, Err(ShapeChanged(tree.root_id())));
    }
}

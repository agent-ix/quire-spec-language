// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S2 parsed value-expression and declaration forms (ADR-011 §6.2
//! module map: `value::expression::syntax` moves to layer-2 `forms`, M-3a),
//! as a typed tree over source names.
//!
//! Names are unresolved source spellings: a parameter, `let` or binder name,
//! a qualified enum member `E::m`, or a qualified call target. A location in
//! a refusal is the path of child indices from the declaration root, each
//! index numbered as [`Expression::children`] lists the children.

use crate::absence::AbsenceMode;
use crate::value::ValueType;
use quire_exact::{CollectionKind, Integer};

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
/// belongs to `CheckedPackage::check_postcondition_expression`'s own
/// `pre_anchor`/population wiring, which no `PackageDeclarations::functions`
/// entry ever has, so the type itself rules the case out instead of a
/// runtime check on an otherwise-valid `ClauseKind` value.
///
/// [`CheckedPackage::check_postcondition_expression`]: crate::value::expression::CheckedPackage::check_postcondition_expression
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
        target: ValueType,
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
        target: ValueType,
        /// The population operand `p`.
        population: Box<Expression>,
    },
    /// `lookup<T>(p, r) absent m` (FR-153): `r`'s presence in `p`, per `m`.
    Lookup {
        /// The queried type `T`.
        target: ValueType,
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
}

/// `function name using V(parameters): result pure [decreases(measure)] {
/// body }`.
///
/// Also used for a checked FR-151 dispatch candidate's own body or effective
/// precondition, so both share the FR-146 call-graph and termination
/// machinery: [`clause_kind`](Self::clause) then reads
/// [`ClauseKind::Precondition`] instead of the default
/// [`ClauseKind::Body`], since a dispatched call is admitted inside a
/// precondition but not inside an operation body (TC-196 D06/D07/D08).
///
/// `clause_kind` and `callable_by_name` are crate-private: neither is a bare
/// mutable field a caller can set independently of the other, and a
/// synthesized FR-151 dispatch candidate body or precondition clause is
/// never itself reachable through an ordinary named [`Expression::Call`]
/// (TC-196 D07's own restriction would otherwise be reachable by calling a
/// candidate directly instead of dispatching to it). Build one with
/// [`Self::new`] (an ordinary named function, name-callable) or
/// [`Self::clause`] (an invariant/precondition/postcondition clause, or a
/// crate-internal synthesized dispatch candidate, never name-callable).
#[derive(Clone, Debug)]
pub struct FunctionDeclaration {
    /// The declared name.
    pub name: String,
    /// Parameters in order.
    pub parameters: Vec<(String, ValueType)>,
    /// The declared result type.
    pub result: ValueType,
    /// The `decreases` measure, when written.
    pub measure: Option<Expression>,
    /// The body.
    pub body: Expression,
    /// The clause this declaration's body is checked as. Every ordinary
    /// named function is [`ClauseKind::Body`].
    pub(crate) clause_kind: ClauseKind,
    /// Whether an ordinary named [`Expression::Call`] elsewhere in the same
    /// package may resolve to this declaration. `false` for every
    /// crate-internal FR-151 synthesized function (TC-196 D07's bypass:
    /// closing the clause-kind restriction off syntax alone still leaves a
    /// candidate's body or precondition callable by plain name unless this
    /// is also `false`).
    pub(crate) callable_by_name: bool,
}

impl FunctionDeclaration {
    /// An ordinary named function: a real, name-callable declaration checked
    /// as [`ClauseKind::Body`].
    pub fn new(
        name: impl Into<String>,
        parameters: Vec<(String, ValueType)>,
        result: ValueType,
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
        }
    }

    /// A declaration checked as `clause_kind`, never reachable through an
    /// ordinary named [`Expression::Call`]: an invariant or precondition
    /// clause, or (crate-internal) a synthesized FR-151 dispatch candidate
    /// body or effective precondition. `clause_kind` is
    /// [`DeclaredClauseKind`], not [`ClauseKind`]: admitting
    /// `ClauseKind::Postcondition` here would let any caller assembling a
    /// package hand an ordinary function `pre(...)` legality it never
    /// earned — `pre(...)` is legal only behind a real postcondition's own
    /// `pre_anchor`/population wiring
    /// ([`CheckedPackage::check_postcondition_expression`], a standalone
    /// expression check outside `PackageDeclarations::functions` entirely),
    /// which no package function has — so `DeclaredClauseKind` leaves that
    /// case unrepresentable rather than accepting it and refusing later.
    ///
    /// [`CheckedPackage::check_postcondition_expression`]: crate::value::expression::CheckedPackage::check_postcondition_expression
    pub fn clause(
        name: impl Into<String>,
        parameters: Vec<(String, ValueType)>,
        result: ValueType,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// TC-169 / FR-067-AC-9 / FR-067-CON-3: this move relocates the eight
    /// types' defining module only, adding, removing or renaming no
    /// variant, field or method. An exhaustive match with every field named
    /// (no `..`) fails to compile the moment a variant or a field is added,
    /// removed or renamed — E0004 (non-exhaustive match) for a variant,
    /// E0026/E0027 (unknown/missing field) for a field — so this test is
    /// itself the shape check, not just evidence run under it.
    ///
    /// Two residual limits, both covered in practice by this crate's own
    /// live callers rather than by this test: binding a field with `_`
    /// names it but does not check its *type* (TC-169 step 4's stated
    /// scope), and `FunctionDeclaration::clause` is never called here.
    /// `src/model/checked_dispatch.rs:851,888,916` exercises both today —
    /// a type-shape change on a bound-`_` field, or a `clause` signature
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
            } = value;
            "FunctionDeclaration"
        }
        let declaration = FunctionDeclaration::new(
            "f",
            Vec::new(),
            ValueType::Boolean,
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

        // `Expression::children` (the one method besides construction this
        // requirement's move must leave callable, per this module's own doc)
        // still exists and still walks direct subexpressions.
        let nested = Expression::Not(Box::new(Expression::Boolean(false)));
        assert_eq!(nested.children().len(), 1);
    }

    /// TC-169 step 1 / FR-067-AC-9: `value::expression::syntax` is absent
    /// from the module tree, checked directly against this repository's
    /// file-per-module convention (`value::expression::mod`'s own `mod X;`
    /// declarations map 1:1 to `src/value/expression/X.rs`), rather than
    /// only through the public-API `compile_fail` doctest on this crate's
    /// `forms` module doc, which cannot by itself distinguish "absent" from
    /// "still present but private".
    #[trace("TC-169", "FR-067-AC-9")]
    #[test]
    fn value_expression_syntax_is_absent_from_the_module_tree() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(
            !manifest_dir.join("src/value/expression/syntax.rs").exists(),
            "src/value/expression/syntax.rs still exists on disk"
        );
        let mod_rs = std::fs::read_to_string(manifest_dir.join("src/value/expression/mod.rs"))
            .expect("src/value/expression/mod.rs exists");
        let declares_syntax_module = mod_rs
            .lines()
            .any(|line| line.trim() == "mod syntax;" || line.trim() == "pub mod syntax;");
        assert!(
            !declares_syntax_module,
            "value::expression::mod still declares a syntax submodule"
        );
    }
}

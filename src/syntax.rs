// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-002: located syntax and parser ceilings, before model linking or evaluation.
use crate::source::{Source, Span, Spanned};

pub mod composed;

/// Exact admitted language identifier.
pub const LANGUAGE: &str = "ix:native";
/// Exact admitted syntax edition.
pub const EDITION: &str = "0-draft";
/// Exact admitted profile label; parsing alone does not establish executable support.
pub const PROFILE: &str = "state-finite/0-draft";

/// Caller limits may lower the implementation ceilings, never disable them.
/// Equality compares the requested capacities, so a resource-only configuration
/// change remains visible in retained build provenance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Inclusive input-content ceiling, clamped to 1 MiB.
    pub source_bytes: usize,
    /// Historical parsing: maximum non-comment tokens. Complete parsing:
    /// maximum retained CST leaves. Clamped to 100,000.
    pub tokens: usize,
    /// Maximum syntax nodes, clamped to 50,000. The historical path counts
    /// expressions; the composed path also counts declarations, parameters,
    /// captures, activation/interval records and protocol binding/control records.
    pub nodes: usize,
    /// Maximum delimiter or recursive parser nesting, clamped to 64.
    pub nesting: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            source_bytes: crate::source::MAX_SOURCE_BYTES,
            tokens: 100_000,
            nodes: 50_000,
            nesting: 64,
        }
    }
}

impl Limits {
    pub(crate) fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            source_bytes: self.source_bytes.min(hard.source_bytes),
            tokens: self.tokens.min(hard.tokens),
            nodes: self.nodes.min(hard.nodes),
            nesting: self.nesting.min(hard.nesting),
        }
    }
}

/// Expression handle local to one historical or composed unit; not a cross-unit identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExprId(pub(crate) usize);

/// Admitted unary syntax operators; type validity is a later phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    /// Logical negation, spelled not.
    Not,
    /// Numeric negation, spelled minus.
    Negate,
}

/// Admitted binary syntax operators, before operand type checking.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    /// Right-associative logical implication.
    Implies,
    /// Logical disjunction.
    Or,
    /// Logical conjunction.
    And,
    /// Equality comparison.
    Equal,
    /// Inequality comparison.
    NotEqual,
    /// Strict less-than comparison.
    Less,
    /// Non-strict less-than comparison.
    LessEqual,
    /// Strict greater-than comparison.
    Greater,
    /// Non-strict greater-than comparison.
    GreaterEqual,
    /// Addition.
    Add,
    /// Subtraction.
    Subtract,
    /// Multiplication.
    Multiply,
    /// Integer division, spelled div.
    Divide,
    /// Integer remainder, spelled rem.
    Remainder,
}

impl BinaryOp {
    /// Native token spelling for this operator.
    pub fn text(self) -> &'static str {
        match self {
            Self::Implies => "implies",
            Self::Or => "or",
            Self::And => "and",
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "div",
            Self::Remainder => "rem",
        }
    }
    pub(crate) fn power(self) -> u8 {
        match self {
            Self::Implies => 1,
            Self::Or => 2,
            Self::And => 3,
            Self::Equal
            | Self::NotEqual
            | Self::Less
            | Self::LessEqual
            | Self::Greater
            | Self::GreaterEqual => 4,
            Self::Add | Self::Subtract => 5,
            Self::Multiply | Self::Divide | Self::Remainder => 6,
        }
    }
    pub(crate) fn comparison(self) -> bool {
        self.power() == 4
    }
}

/// Closed set of admitted single-argument builtin syntax forms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Builtin {
    /// Option presence query.
    Present,
    /// Option value selection.
    Value,
    /// Reference dereference.
    Deref,
    /// Collection size query.
    Size,
    /// Pre-state expression selection.
    Pre,
}

impl Builtin {
    /// Native token spelling for this builtin.
    pub fn text(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Value => "value",
            Self::Deref => "deref",
            Self::Size => "size",
            Self::Pre => "pre",
        }
    }
}

/// Untyped expression shape with child handles into flat unit storage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExprKind {
    /// Explicit parentheses; the Expr span includes their delimiters.
    Group {
        /// Parenthesized expression.
        inner: ExprId,
    },
    /// Boolean literal.
    Boolean(bool),
    /// Original nonnegative decimal integer spelling; negation is a unary node.
    Integer(String),
    /// Decoded Unicode string value; the raw spelling remains in the source span.
    Text(String),
    /// Unresolved name reference.
    Name(Spanned<String>),
    /// Context object reference.
    SelfValue,
    /// Invocation result reference.
    ResultValue,
    /// Unresolved model-qualified enumeration variant.
    EnumValue {
        /// Imported model alias.
        model: Spanned<String>,
        /// Enumeration name.
        name: Spanned<String>,
        /// Variant name.
        variant: Spanned<String>,
    },
    /// Field selection on an expression.
    Field {
        /// Receiver expression.
        base: ExprId,
        /// Selected field name.
        name: Spanned<String>,
    },
    /// Unary operator application.
    Unary {
        /// Selected operator.
        op: UnaryOp,
        /// Operand expression.
        argument: ExprId,
    },
    /// Binary operator application.
    Binary {
        /// Selected operator.
        op: BinaryOp,
        /// Left operand.
        left: ExprId,
        /// Right operand.
        right: ExprId,
    },
    /// Single-argument builtin application.
    Call {
        /// Selected builtin.
        builtin: Builtin,
        /// Argument expression.
        argument: ExprId,
    },
    /// Local immutable binding and its body expression.
    Let {
        /// Binding name.
        name: Spanned<String>,
        /// Initializer expression.
        value: ExprId,
        /// Expression in which the binding is in scope.
        body: ExprId,
    },
    /// Conditional expression.
    If {
        /// Branch condition.
        condition: ExprId,
        /// Selected expression when the condition holds.
        then_value: ExprId,
        /// Selected expression when the condition does not hold.
        else_value: ExprId,
    },
    /// Finite-domain quantifier syntax; domain validity requires later typing.
    Quantifier {
        /// True for forall, false for exists.
        universal: bool,
        /// Bound element name.
        name: Spanned<String>,
        /// Domain expression.
        domain: ExprId,
        /// Predicate expression.
        predicate: ExprId,
    },
    /// Directed reachability syntax over one named field.
    Reaches {
        /// Starting object expression.
        start: ExprId,
        /// Target object expression.
        target: ExprId,
        /// Traversed field name.
        field: Spanned<String>,
    },
}

/// Flat expression node with its exact original source region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expr {
    /// Untyped syntax shape.
    pub kind: ExprKind,
    /// Complete expression range, including grouping delimiters.
    pub span: Span,
}

/// Model import syntax; labels are retained without resolving a model artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelImport {
    /// Local model alias.
    pub alias: Spanned<String>,
    /// Decoded package label.
    pub package: Spanned<String>,
    /// Decoded version label.
    pub version: Spanned<String>,
    /// Decoded digest label; parser success does not validate a model digest.
    pub digest: Spanned<String>,
    /// Complete import declaration range.
    pub span: Span,
}

/// Syntax-level clause category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseKind {
    /// Current-state object invariant.
    Invariant,
    /// Operation precondition.
    Precondition,
    /// Operation postcondition.
    Postcondition,
}

/// Located declaration, before checking model context or expression type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Clause {
    /// Invariant, precondition or postcondition syntax.
    pub kind: ClauseKind,
    /// Authored declaration name.
    pub name: Spanned<String>,
    /// Unresolved imported model alias.
    pub model: Spanned<String>,
    /// Unresolved context type name.
    pub context: Spanned<String>,
    /// Operation name for pre/post clauses; absent for invariants.
    pub operation: Option<Spanned<String>>,
    /// Root expression handle into the containing ParsedUnit.
    pub expression: ExprId,
    /// Complete declaration source range.
    pub span: Span,
}

/// Flat syntax storage avoids recursive destruction of adversarial operator chains.
#[derive(Clone, Debug)]
pub struct ParsedUnit {
    pub(crate) source: Source,
    pub(crate) language: Spanned<String>,
    pub(crate) edition: Spanned<String>,
    pub(crate) imports: Vec<ModelImport>,
    pub(crate) clauses: Vec<Clause>,
    pub(crate) expressions: Vec<Expr>,
}

impl ParsedUnit {
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
    /// Unresolved model imports in source order.
    pub fn imports(&self) -> &[ModelImport] {
        &self.imports
    }
    /// Located declarations in source order.
    pub fn clauses(&self) -> &[Clause] {
        &self.clauses
    }
    /// IDs are local to this unit; out-of-range handles are rejected.
    pub fn expression(&self, id: ExprId) -> Option<&Expr> {
        self.expressions.get(id.0)
    }
    /// Flat expression arena; each child handle is local to this unit.
    pub fn expressions(&self) -> &[Expr] {
        &self.expressions
    }
}

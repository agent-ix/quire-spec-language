// SPDX-License-Identifier: AGPL-3.0-only
use crate::source::{Source, Span, Spanned};

pub const LANGUAGE: &str = "ix:native";
pub const EDITION: &str = "0-draft";
pub const PROFILE: &str = "state-finite/0-draft";

/// Caller limits may lower the implementation ceilings, never disable them.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub source_bytes: usize,
    pub tokens: usize,
    pub nodes: usize,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExprId(pub(crate) usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    Not,
    Negate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    Implies,
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

impl BinaryOp {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Builtin {
    Present,
    Value,
    Deref,
    Size,
    Pre,
}

impl Builtin {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExprKind {
    Group {
        inner: ExprId,
    },
    Boolean(bool),
    Integer(String),
    Text(String),
    Name(Spanned<String>),
    SelfValue,
    ResultValue,
    EnumValue {
        model: Spanned<String>,
        name: Spanned<String>,
        variant: Spanned<String>,
    },
    Field {
        base: ExprId,
        name: Spanned<String>,
    },
    Unary {
        op: UnaryOp,
        argument: ExprId,
    },
    Binary {
        op: BinaryOp,
        left: ExprId,
        right: ExprId,
    },
    Call {
        builtin: Builtin,
        argument: ExprId,
    },
    Let {
        name: Spanned<String>,
        value: ExprId,
        body: ExprId,
    },
    If {
        condition: ExprId,
        then_value: ExprId,
        else_value: ExprId,
    },
    Quantifier {
        universal: bool,
        name: Spanned<String>,
        domain: ExprId,
        predicate: ExprId,
    },
    Reaches {
        start: ExprId,
        target: ExprId,
        field: Spanned<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelImport {
    pub alias: Spanned<String>,
    pub package: Spanned<String>,
    pub version: Spanned<String>,
    pub digest: Spanned<String>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseKind {
    Invariant,
    Precondition,
    Postcondition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Clause {
    pub kind: ClauseKind,
    pub name: Spanned<String>,
    pub model: Spanned<String>,
    pub context: Spanned<String>,
    pub operation: Option<Spanned<String>>,
    pub expression: ExprId,
    pub span: Span,
}

/// Flat syntax storage avoids recursive destruction of adversarial operator chains.
#[derive(Clone, Debug)]
pub struct ParsedUnit {
    pub(crate) source: Source,
    pub(crate) imports: Vec<ModelImport>,
    pub(crate) clauses: Vec<Clause>,
    pub(crate) expressions: Vec<Expr>,
}

impl ParsedUnit {
    pub fn source(&self) -> &Source {
        &self.source
    }
    pub fn imports(&self) -> &[ModelImport] {
        &self.imports
    }
    pub fn clauses(&self) -> &[Clause] {
        &self.clauses
    }
    /// IDs are local to this unit; out-of-range handles are rejected.
    pub fn expression(&self, id: ExprId) -> Option<&Expr> {
        self.expressions.get(id.0)
    }
    pub fn expressions(&self) -> &[Expr] {
        &self.expressions
    }
}

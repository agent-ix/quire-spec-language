// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 "Canonical printing": the `QF_LIA` terms the encoding builds and
//! the one printer that writes them.
//!
//! A [`Term`] is built through the constructors below. The Boolean constants
//! stand for "no condition": a total expression's definedness is `true` and
//! a construct that cannot fail has the failure `false`, so the constructors
//! drop them. They fold no
//! other term: a literal read in place of a constant stays in the term it
//! stands in (FR-315 "Canonical printing").
//! [`Script::render`] prints with single spaces, no comments, integer
//! literals in decimal with negatives as `(- n)`, and each command on its own
//! line ending in a line feed.

use std::ops::Not;

use quire_exact::Integer;

use crate::smt::SmtEncodingRefusal;

/// An SMT sort the encoding declares.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sort {
    /// `Bool`.
    Bool,
    /// `Int`.
    Int,
}

impl Sort {
    const fn spelling(self) -> &'static str {
        match self {
            Self::Bool => "Bool",
            Self::Int => "Int",
        }
    }
}

/// An ordering comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Comparison {
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
}

impl Comparison {
    const fn spelling(self) -> &'static str {
        match self {
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
        }
    }
}

/// A linear term of `QF_LIA`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Term {
    /// A Boolean literal, printed as written and never folded.
    Bool(bool),
    /// "No condition" (`true`) or "no failure" (`false`): the definedness of
    /// a total expression and the failure of a construct that cannot fail.
    /// The constructors fold it away; an empty conjunction or disjunction
    /// prints as it does.
    Cond(bool),
    /// An integer literal.
    Int(Integer),
    /// A declared constant, by its printed symbol.
    Const(String),
    /// `(not t)`.
    Not(Box<Term>),
    /// `(and t ...)`, with at least two operands.
    And(Vec<Term>),
    /// `(or t ...)`, with at least two operands.
    Or(Vec<Term>),
    /// `(=> l r)`.
    Implies(Box<Term>, Box<Term>),
    /// `(= l r)`.
    Equal(Box<Term>, Box<Term>),
    /// `(<= l r)` and its siblings.
    Compare(Comparison, Box<Term>, Box<Term>),
    /// `(+ t ...)`, with at least two operands.
    Add(Vec<Term>),
    /// `(- l r)`.
    Subtract(Box<Term>, Box<Term>),
    /// `(- t)`.
    Negate(Box<Term>),
    /// `(* n t)` for a literal `n`.
    Scale(Integer, Box<Term>),
    /// `(mod t n)` for a literal `n` other than zero.
    Modulo(Box<Term>, Integer),
    /// `(ite c t e)`.
    If(Box<Term>, Box<Term>, Box<Term>),
}

impl Term {
    /// The condition constant `value`: "no condition" or "no failure".
    pub const fn truth(value: bool) -> Self {
        Self::Cond(value)
    }

    /// The Boolean literal `value`, which the printer keeps.
    pub const fn literal(value: bool) -> Self {
        Self::Bool(value)
    }

    /// The integer literal `value`.
    pub fn int(value: i64) -> Self {
        Self::Int(Integer::from(value))
    }

    /// The declared constant `symbol`.
    pub fn constant(symbol: impl Into<String>) -> Self {
        Self::Const(symbol.into())
    }

    /// Whether this term is the Boolean constant `value`.
    pub fn is_truth(&self, value: bool) -> bool {
        *self == Self::Cond(value)
    }

    /// The integer literal this term is, if it is one.
    pub fn as_literal(&self) -> Option<&Integer> {
        match self {
            Self::Int(value) => Some(value),
            _ => None,
        }
    }

    /// The conjunction of `terms`: `false` when one is `false`, `true` when
    /// none remains and `true` dropped.
    pub fn all(terms: impl IntoIterator<Item = Self>) -> Self {
        let mut kept = Vec::new();
        for term in terms {
            match term {
                Self::Cond(true) => {}
                Self::Cond(false) => return Self::Cond(false),
                other => kept.push(other),
            }
        }
        match kept.len() {
            0 => Self::Cond(true),
            1 => kept.remove(0),
            _ => Self::And(kept),
        }
    }

    /// The disjunction of `terms`: `true` when one is `true`, `false` when
    /// none remains and `false` dropped.
    pub fn any(terms: impl IntoIterator<Item = Self>) -> Self {
        let mut kept = Vec::new();
        for term in terms {
            match term {
                Self::Cond(false) => {}
                Self::Cond(true) => return Self::Cond(true),
                other => kept.push(other),
            }
        }
        match kept.len() {
            0 => Self::Cond(false),
            1 => kept.remove(0),
            _ => Self::Or(kept),
        }
    }

    /// `self => other`.
    #[must_use]
    pub fn implies(self, other: Self) -> Self {
        match (&self, &other) {
            (Self::Cond(false), _) | (_, Self::Cond(true)) => Self::Cond(true),
            (Self::Cond(true), _) => other,
            (_, Self::Cond(false)) => self.not(),
            _ => Self::Implies(Box::new(self), Box::new(other)),
        }
    }

    /// `self = other`.
    #[must_use]
    pub fn equal(self, other: Self) -> Self {
        Self::Equal(Box::new(self), Box::new(other))
    }

    /// `self op other`.
    #[must_use]
    pub fn compare(self, op: Comparison, other: Self) -> Self {
        Self::Compare(op, Box::new(self), Box::new(other))
    }

    /// `self + other`.
    #[must_use]
    pub fn plus(self, other: Self) -> Self {
        Self::Add(vec![self, other])
    }

    /// `self - other`.
    #[must_use]
    pub fn minus(self, other: Self) -> Self {
        Self::Subtract(Box::new(self), Box::new(other))
    }

    /// `-self`.
    #[must_use]
    pub fn negated(self) -> Self {
        Self::Negate(Box::new(self))
    }

    /// `self * other` when one operand is a literal; `None` otherwise, since
    /// the product of two non-literals is not linear.
    pub fn times(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Self::Int(factor), term) | (term, Self::Int(factor)) => {
                Some(Self::Scale(factor, Box::new(term)))
            }
            _ => None,
        }
    }

    /// `self mod divisor` for a literal `divisor` other than zero.
    #[must_use]
    pub fn modulo(self, divisor: Integer) -> Self {
        Self::Modulo(Box::new(self), divisor)
    }

    /// `if self then then else otherwise`, folding a constant condition.
    #[must_use]
    pub fn select(self, then: Self, otherwise: Self) -> Self {
        match self {
            Self::Cond(true) => then,
            Self::Cond(false) => otherwise,
            condition => Self::If(Box::new(condition), Box::new(then), Box::new(otherwise)),
        }
    }

    /// `lower <= self <= upper`.
    #[must_use]
    pub fn within(self, lower: &Integer, upper: &Integer) -> Self {
        Self::all([
            Self::Int(lower.clone()).compare(Comparison::LessOrEqual, self.clone()),
            self.compare(Comparison::LessOrEqual, Self::Int(upper.clone())),
        ])
    }

    /// The term's canonical printing.
    pub fn text(&self) -> String {
        let mut out = String::new();
        self.print(&mut out);
        out
    }

    fn print(&self, out: &mut String) {
        match self {
            Self::Bool(value) | Self::Cond(value) => {
                out.push_str(if *value { "true" } else { "false" });
            }
            Self::Int(value) => print_integer(value, out),
            Self::Const(symbol) => out.push_str(symbol),
            Self::Not(inner) => apply("not", [inner.as_ref()], out),
            Self::And(terms) => apply("and", terms, out),
            Self::Or(terms) => apply("or", terms, out),
            Self::Implies(left, right) => apply("=>", [left.as_ref(), right.as_ref()], out),
            Self::Equal(left, right) => apply("=", [left.as_ref(), right.as_ref()], out),
            Self::Compare(op, left, right) => {
                apply(op.spelling(), [left.as_ref(), right.as_ref()], out);
            }
            Self::Add(terms) => apply("+", terms, out),
            Self::Subtract(left, right) => apply("-", [left.as_ref(), right.as_ref()], out),
            Self::Negate(inner) => apply("-", [inner.as_ref()], out),
            Self::Scale(factor, inner) => {
                out.push_str("(* ");
                print_integer(factor, out);
                out.push(' ');
                inner.print(out);
                out.push(')');
            }
            Self::Modulo(inner, divisor) => {
                out.push_str("(mod ");
                inner.print(out);
                out.push(' ');
                print_integer(divisor, out);
                out.push(')');
            }
            Self::If(condition, then, otherwise) => apply(
                "ite",
                [condition.as_ref(), then.as_ref(), otherwise.as_ref()],
                out,
            ),
        }
    }
}

/// `not self`, folding a condition constant.
impl std::ops::Not for Term {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Self::Cond(value) => Self::Cond(!value),
            other => Self::Not(Box::new(other)),
        }
    }
}

fn print_integer(value: &Integer, out: &mut String) {
    let text = value.to_string();
    match text.strip_prefix('-') {
        Some(magnitude) => {
            out.push_str("(- ");
            out.push_str(magnitude);
            out.push(')');
        }
        None => out.push_str(&text),
    }
}

fn apply<'t>(head: &str, operands: impl IntoIterator<Item = &'t Term>, out: &mut String) {
    out.push('(');
    out.push_str(head);
    for operand in operands {
        out.push(' ');
        operand.print(out);
    }
    out.push(')');
}

/// A printable SMT-LIB symbol for `name`: itself when it is a simple symbol,
/// quoted with `|` otherwise.
///
/// # Errors
///
/// [`SmtEncodingRefusal::ConstantName`] when `name` holds a character a
/// quoted symbol cannot hold.
///
/// `#[string_edge]`: classifies the characters of a name from the package, to
/// print it, and gates no semantics on the text.
#[ix_trace_rs::string_edge]
pub fn symbol(name: &str) -> Result<String, SmtEncodingRefusal> {
    let simple = |character: char| {
        character.is_ascii_alphanumeric() || "~!@$%^&*_-+=<>.?/".contains(character)
    };
    if !name.is_empty()
        && name.chars().all(simple)
        && !name.starts_with(|first: char| first.is_ascii_digit())
    {
        return Ok(name.to_owned());
    }
    if name.is_empty() || name.contains(['|', '\\']) {
        return Err(SmtEncodingRefusal::ConstantName {
            name: name.to_owned(),
        });
    }
    Ok(format!("|{name}|"))
}

/// The most bytes of a canonical script (FR-315 "Size limits").
pub const MAX_SCRIPT_BYTES: usize = 1 << 28;

/// A `QF_LIA` script: the declarations and the assertions of one query, kept
/// in their printed form so that its size is known as it grows.
#[derive(Clone, Debug)]
pub struct Script {
    text: String,
    declared: usize,
    exceeded: bool,
}

impl Default for Script {
    fn default() -> Self {
        Self::new()
    }
}

impl Script {
    /// A script with its `(set-logic QF_LIA)` command.
    pub fn new() -> Self {
        Self {
            text: String::from("(set-logic QF_LIA)\n"),
            declared: 0,
            exceeded: false,
        }
    }

    fn push(&mut self, command: &str) {
        if self.text.len().saturating_add(command.len()) > MAX_SCRIPT_BYTES {
            self.exceeded = true;
        } else {
            self.text.push_str(command);
        }
    }

    /// Declare the constant `symbol` of `sort`, after the ones declared so
    /// far. Declarations precede every assertion.
    pub fn declare(&mut self, symbol: &str, sort: Sort) {
        self.declared += 1;
        self.push(&format!("(declare-const {symbol} {})\n", sort.spelling()));
    }

    /// Assert `term` after the assertions so far.
    pub fn assert(&mut self, term: &Term) {
        self.assert_text(&term.text());
    }

    /// Assert the already printed term `printed`.
    pub fn assert_text(&mut self, printed: &str) {
        if self.exceeded {
            return;
        }
        self.push(&format!("(assert {printed})\n"));
    }

    /// The number of constants declared so far.
    pub const fn declared(&self) -> usize {
        self.declared
    }

    /// Whether a command would have passed [`MAX_SCRIPT_BYTES`].
    pub const fn exceeded(&self) -> bool {
        self.exceeded
    }

    /// The canonical printing: `(set-logic QF_LIA)`, the declarations in the
    /// order given, one `assert` per assertion, then `(check-sat)`.
    pub fn render(mut self) -> Vec<u8> {
        self.text.push_str("(check-sat)\n");
        self.text.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn text(term: &Term) -> String {
        let mut out = String::new();
        term.print(&mut out);
        out
    }

    /// FR-315 "canonical printing": negative integers print as `(- n)`.
    #[trace("TC-900", "FR-315-AC-1")]
    #[test]
    fn negative_literals_print_as_a_negation() {
        assert_eq!(text(&Term::int(-3)), "(- 3)");
        assert_eq!(text(&Term::int(3)), "3");
    }

    /// Conjunction drops `true` and short-circuits on `false`, and nested
    /// conjunctions stay nested.
    #[trace("TC-900", "FR-315-AC-1")]
    #[test]
    fn conjunctions_fold_constants_and_stay_nested() {
        let a = Term::constant("a");
        let b = Term::constant("b");
        assert_eq!(Term::all([Term::truth(true), a.clone()]), a);
        assert!(Term::all([a.clone(), Term::truth(false)]).is_truth(false));
        let nested = Term::all([Term::all([a.clone(), b.clone()]), a.clone()]);
        assert_eq!(text(&nested), "(and (and a b) a)");
        assert_eq!(text(&Term::any([a, b])), "(or a b)");
    }

    /// A quoted symbol is used only when the name needs it, and a name a
    /// quoted symbol cannot hold is refused.
    #[trace("TC-900", "FR-315-AC-3")]
    #[test]
    fn symbols_quote_only_when_needed() {
        assert_eq!(symbol("s0.c.value").as_deref(), Ok("s0.c.value"));
        assert_eq!(symbol("s0.a b.value").as_deref(), Ok("|s0.a b.value|"));
        assert!(symbol("a|b").is_err());
    }

    /// The script prints the logic first and `(check-sat)` last, each
    /// command on its own line.
    #[trace("TC-900", "FR-315-AC-1")]
    #[test]
    fn a_script_prints_in_canonical_order() {
        let mut script = Script::new();
        script.declare("x", Sort::Int);
        script.assert(&Term::constant("x").equal(Term::int(0)));
        assert_eq!(
            String::from_utf8(script.render()).expect("UTF-8"),
            "(set-logic QF_LIA)\n(declare-const x Int)\n(assert (= x 0))\n(check-sat)\n"
        );
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-314's Alethe proof checker (ADR-018 PC-6, LA-3): reads one part's
//! refutation as Alethe commands and checks it against the SMT-LIB query
//! the core encoded for that part.
//!
//! [`read_query`] reads the query's declarations and assertions;
//! [`read_proof`] reads the proof into numbered commands, or finds it
//! malformed (not a sequence of well-formed Alethe commands). A [`Proof`]
//! then answers FR-314's proof rules in order: [`Proof::invalid_step`], the
//! first command whose rule is in the checked set and whose conclusion does
//! not follow, or whose premises are not earlier visible commands;
//! [`Proof::not_refutation`], a last command that does not conclude the
//! empty clause; and [`Proof::outcome`], the first command whose rule is
//! outside the checked set ([`ProofOutcome::Unverifiable`]).
//!
//! Commands are numbered from 0 in the proof's flattened text order, every
//! command (`assume`, `step`, `anchor`, `define-fun`) and every command
//! inside a subproof, its closing step included, taking the next number.
//!
//! Terms are compared syntactically after the `:named`, `define-fun` and
//! `let` abbreviations are expanded, modulo the orientation of equalities
//! (the Alethe specification's "implicit reordering of equalities"): a
//! binary `=` is held with its arguments in a canonical order. A step's
//! conclusion is also kept as written, because in a non-empty context its
//! equality is oriented.
//!
//! An anchor's `:args` extend the context `Γ`; `refl`, `trans` and `cong`
//! are checked under `subst(Γ)`, up to the renaming of bound variables, and
//! `bind` against the subproof it closes. A rule that requires an empty
//! context, used in a non-empty one, is not decided, and neither is a
//! subproof closed by a rule other than `subproof` or `bind`, or a
//! `resolution` whose pivots the bounded search cannot settle.

#![allow(
    clippy::mutable_key_type,
    reason = "a term holds quire_exact::Integer, whose lazy BigInt cache takes no part in Eq, Ord or Hash"
)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;
use std::str::FromStr;

use quire_exact::{Integer, Rational};

/// The deepest nesting of parentheses the reader admits; a deeper proof or
/// query is malformed rather than read recursively without bound.
const MAX_DEPTH: usize = 512;

/// An S-expression as the SMT-LIB and Alethe readers see it.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Sexpr {
    Atom(Rc<str>),
    List(Vec<Sexpr>),
}

/// Read every top-level S-expression of `text`; `None` when it does not
/// parse (an unbalanced parenthesis, an unterminated string or quoted
/// symbol, or nesting deeper than [`MAX_DEPTH`]).
fn read_sexprs(text: &str) -> Option<Vec<Sexpr>> {
    let mut stack: Vec<Vec<Sexpr>> = vec![Vec::new()];
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        match c {
            '(' => {
                if stack.len() > MAX_DEPTH {
                    return None;
                }
                stack.push(Vec::new());
            }
            ')' => {
                let list = stack.pop()?;
                stack.last_mut()?.push(Sexpr::List(list));
            }
            ';' => {
                for (_, next) in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            '"' | '|' => {
                let close = c;
                let mut end = None;
                while let Some((at, next)) = chars.next() {
                    if next == close {
                        // `""` inside a string is an escaped quote.
                        if close == '"' && chars.peek().is_some_and(|(_, after)| *after == '"') {
                            chars.next();
                            continue;
                        }
                        end = Some(at + next.len_utf8());
                        break;
                    }
                }
                let end = end?;
                stack
                    .last_mut()?
                    .push(Sexpr::Atom(text.get(start..end)?.into()));
            }
            _ => {
                let mut end = start + c.len_utf8();
                while let Some(&(at, next)) = chars.peek() {
                    if next.is_whitespace() || matches!(next, '(' | ')' | ';' | '"' | '|') {
                        break;
                    }
                    end = at + next.len_utf8();
                    chars.next();
                }
                stack
                    .last_mut()?
                    .push(Sexpr::Atom(text.get(start..end)?.into()));
            }
        }
    }
    if stack.len() != 1 {
        return None;
    }
    stack.pop()
}

/// A term of a query or proof, after its `:named` and `define-fun` proxies
/// are expanded.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum Term {
    /// A symbol: a constant, `true`, `false`, or a bound variable.
    Symbol(Rc<str>),
    /// A numeric literal; `real` for a decimal or rational literal, which
    /// Alethe sorts `Real` (an integer numeral is `Int`).
    Number { value: Rational, real: bool },
    /// An application of a function symbol to its arguments, in source
    /// order, except that a binary `=` holds its arguments in canonical
    /// order (implicit reordering of equalities).
    App(Rc<str>, Vec<Rc<Term>>),
    /// A binder (`forall`, `exists`, `lambda`, `choice`): its sorted
    /// variables, each with its sort's text, and its body.
    Binder {
        quantifier: Rc<str>,
        vars: Vec<(Rc<str>, Rc<str>)>,
        body: Rc<Term>,
    },
    /// A form this checker reads only syntactically (an indexed identifier,
    /// a sort ascription).
    Other(Rc<str>),
}

/// An interpreted SMT-LIB operator, which a rule or the sort check reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Op {
    Not,
    And,
    Or,
    Implies,
    Xor,
    Eq,
    Distinct,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    Abs,
    ToReal,
    ToInt,
    IsInt,
    Ite,
}

/// The interpreted operator an application's head spells; `None` for an
/// uninterpreted function symbol. `#[string_edge]`: the one place the
/// SMT-LIB operator spellings a query or proof carries are read.
#[ix_trace_rs::string_edge]
fn op(head: &str) -> Option<Op> {
    Some(match head {
        "not" => Op::Not,
        "and" => Op::And,
        "or" => Op::Or,
        "=>" => Op::Implies,
        "xor" => Op::Xor,
        "=" => Op::Eq,
        "distinct" => Op::Distinct,
        "<" => Op::Lt,
        "<=" => Op::Le,
        ">" => Op::Gt,
        ">=" => Op::Ge,
        "+" => Op::Add,
        "-" => Op::Sub,
        "*" => Op::Mul,
        "/" => Op::Div,
        "div" => Op::IntDiv,
        "mod" => Op::Mod,
        "abs" => Op::Abs,
        "to_real" => Op::ToReal,
        "to_int" => Op::ToInt,
        "is_int" => Op::IsInt,
        "ite" => Op::Ite,
        _ => return None,
    })
}

/// A binder's quantifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Quantifier {
    Forall,
    Exists,
    Choice,
    Lambda,
}

/// The quantifier a binder's keyword spells. `#[string_edge]`: reads the
/// SMT-LIB binder keyword.
#[ix_trace_rs::string_edge]
fn quantifier(keyword: &str) -> Option<Quantifier> {
    match keyword {
        "forall" => Some(Quantifier::Forall),
        "exists" => Some(Quantifier::Exists),
        "choice" => Some(Quantifier::Choice),
        "lambda" => Some(Quantifier::Lambda),
        _ => None,
    }
}

/// The Boolean constant a symbol spells, if it is `true` or `false`.
/// `#[string_edge]`: reads the SMT-LIB constant spelling.
#[ix_trace_rs::string_edge]
fn boolean_literal(symbol: &str) -> Option<bool> {
    match symbol {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

impl Term {
    fn app(head: &str, args: Vec<Rc<Term>>) -> Rc<Term> {
        let mut args = args;
        if op(head) == Some(Op::Eq) && args.len() == 2 && args[0] > args[1] {
            args.swap(0, 1);
        }
        Rc::new(Term::App(head.into(), args))
    }

    fn not(term: &Rc<Term>) -> Rc<Term> {
        Term::app("not", vec![term.clone()])
    }

    /// The arguments of an application of `head`, or `None`.
    fn args_of(&self, head: &str) -> Option<&[Rc<Term>]> {
        match self {
            Term::App(name, args) if &**name == head => Some(args),
            _ => None,
        }
    }

    /// The operand of `(not t)`.
    fn negated(&self) -> Option<&Rc<Term>> {
        match self.args_of("not")? {
            [operand] => Some(operand),
            _ => None,
        }
    }

    fn is_symbol(&self, name: &str) -> bool {
        matches!(self, Term::Symbol(symbol) if &**symbol == name)
    }
}

/// A literal read with its leading negations merged, as resolution reads
/// pivots: its base formula and whether an odd number of `not`s precede it.
fn polarity(literal: &Rc<Term>) -> (&Rc<Term>, bool) {
    let mut base = literal;
    let mut negative = false;
    while let Some(operand) = base.negated() {
        base = operand;
        negative = !negative;
    }
    (base, negative)
}

/// A `define-fun` with parameters: its parameter names and its body.
type Function = (Vec<Rc<str>>, Rc<Term>);

/// A conclusion clause.
type Clause = Rc<[Rc<Term>]>;

/// The proxies a query or proof defines: `:named` annotations and
/// `define-fun` commands without parameters, and the `define-fun` commands
/// with parameters, which an application expands.
#[derive(Default)]
struct Names {
    terms: HashMap<Rc<str>, Rc<Term>>,
    functions: HashMap<Rc<str>, Function>,
    /// The variables of the binders being read, innermost last: a bound
    /// variable shadows a proxy of its name.
    bound: Vec<Rc<str>>,
}

impl Names {
    /// Read `sexpr` as a term, recording each `:named` proxy it introduces.
    /// `#[string_edge]`: reads an S-expression of the query or proof text into a term.
    #[ix_trace_rs::string_edge]
    fn term(&mut self, sexpr: &Sexpr) -> Option<Rc<Term>> {
        match sexpr {
            Sexpr::Atom(atom) => Some(self.atom(atom)),
            Sexpr::List(items) => {
                let (head, rest) = items.split_first()?;
                let Sexpr::Atom(head) = head else {
                    return Some(Rc::new(Term::Other(render(sexpr).into())));
                };
                match &**head {
                    "!" => {
                        let (body, attributes) = rest.split_first()?;
                        let term = self.term(body)?;
                        let mut attributes = attributes.iter();
                        while let Some(key) = attributes.next() {
                            let Sexpr::Atom(key) = key else {
                                return None;
                            };
                            if !key.starts_with(':') {
                                return None;
                            }
                            if &**key == ":named" {
                                let Some(Sexpr::Atom(name)) = attributes.next() else {
                                    return None;
                                };
                                self.terms.insert(name.clone(), term.clone());
                            } else if attributes.clone().next().is_some_and(
                                |value| !matches!(value, Sexpr::Atom(v) if v.starts_with(':')),
                            ) {
                                attributes.next();
                            }
                        }
                        Some(term)
                    }
                    "forall" | "exists" | "lambda" | "choice" => {
                        let [Sexpr::List(declared), body] = rest else {
                            return None;
                        };
                        let vars = sorted_vars(declared)?;
                        let depth = self.bound.len();
                        self.bound.extend(vars.iter().map(|(name, _)| name.clone()));
                        let body = self.term(body);
                        self.bound.truncate(depth);
                        Some(Rc::new(Term::Binder {
                            quantifier: head.clone(),
                            vars,
                            body: body?,
                        }))
                    }
                    "let" => {
                        // `let` is read as the substitution it abbreviates.
                        let [Sexpr::List(bindings), body] = rest else {
                            return None;
                        };
                        let mut map = HashMap::new();
                        for binding in bindings {
                            let Sexpr::List(pair) = binding else {
                                return None;
                            };
                            let [Sexpr::Atom(name), value] = pair.as_slice() else {
                                return None;
                            };
                            map.insert(name.clone(), self.term(value)?);
                        }
                        let depth = self.bound.len();
                        self.bound.extend(map.keys().cloned());
                        let body = self.term(body);
                        self.bound.truncate(depth);
                        Some(substitute(&body?, &map))
                    }
                    "_" | "as" => Some(Rc::new(Term::Other(render(sexpr).into()))),
                    _ => {
                        let args = rest
                            .iter()
                            .map(|arg| self.term(arg))
                            .collect::<Option<Vec<_>>>()?;
                        // `(- n)` of a literal is the literal `-n`, as the
                        // query printer writes negatives.
                        if &**head == "-" {
                            if let [arg] = args.as_slice() {
                                if let Term::Number { value, real } = &**arg {
                                    return Some(Rc::new(Term::Number {
                                        value: value.neg(),
                                        real: *real,
                                    }));
                                }
                            }
                        }
                        if &**head == "/" {
                            if let [numerator, denominator] = args.as_slice() {
                                if let (
                                    Term::Number { value: n, .. },
                                    Term::Number { value: d, .. },
                                ) = (&**numerator, &**denominator)
                                {
                                    if let Some(value) = n.div(d) {
                                        return Some(Rc::new(Term::Number { value, real: true }));
                                    }
                                }
                            }
                        }
                        if !self.bound.contains(head) {
                            if let Some((parameters, body)) = self.functions.get(head) {
                                if parameters.len() != args.len() {
                                    return None;
                                }
                                let map = parameters.iter().cloned().zip(args).collect();
                                return Some(substitute(body, &map));
                            }
                        }
                        Some(Term::app(head, args))
                    }
                }
            }
        }
    }

    fn atom(&self, atom: &Rc<str>) -> Rc<Term> {
        if self.bound.contains(atom) {
            return Rc::new(Term::Symbol(atom.clone()));
        }
        if let Some(term) = self.terms.get(atom) {
            return term.clone();
        }
        if let Some(value) = number(atom) {
            return Rc::new(value);
        }
        Rc::new(Term::Symbol(atom.clone()))
    }
}

/// `((x S) ...)`: each variable with its sort's text.
fn sorted_vars(declared: &[Sexpr]) -> Option<Vec<(Rc<str>, Rc<str>)>> {
    declared
        .iter()
        .map(|var| match var {
            Sexpr::List(pair) => match pair.as_slice() {
                [Sexpr::Atom(name), sort] => Some((name.clone(), render(sort).into())),
                _ => None,
            },
            Sexpr::Atom(_) => None,
        })
        .collect()
}

/// The symbols that occur free in `term`.
fn free_symbols(term: &Term, bound: &mut Vec<Rc<str>>, free: &mut HashSet<Rc<str>>) {
    match term {
        Term::Symbol(name) => {
            if !bound.contains(name) {
                free.insert(name.clone());
            }
        }
        Term::App(_, args) => {
            for arg in args {
                free_symbols(arg, bound, free);
            }
        }
        Term::Binder { vars, body, .. } => {
            let depth = bound.len();
            bound.extend(vars.iter().map(|(name, _)| name.clone()));
            free_symbols(body, bound, free);
            bound.truncate(depth);
        }
        Term::Number { .. } | Term::Other(_) => {}
    }
}

fn is_free_in(name: &str, term: &Term) -> bool {
    let mut free = HashSet::new();
    free_symbols(term, &mut Vec::new(), &mut free);
    free.contains(name)
}

/// `term` with each free symbol of `map`'s domain replaced by its image,
/// renaming a bound variable that an image would capture.
fn substitute(term: &Rc<Term>, map: &HashMap<Rc<str>, Rc<Term>>) -> Rc<Term> {
    if map.is_empty() {
        return term.clone();
    }
    match &**term {
        Term::Symbol(name) => map.get(name).cloned().unwrap_or_else(|| term.clone()),
        Term::App(head, args) => {
            Term::app(head, args.iter().map(|arg| substitute(arg, map)).collect())
        }
        Term::Binder {
            quantifier,
            vars,
            body,
        } => {
            let mut inner = map.clone();
            for (name, _) in vars {
                inner.remove(name);
            }
            let mut captured = HashSet::new();
            for image in inner.values() {
                free_symbols(image, &mut Vec::new(), &mut captured);
            }
            let mut renamed = Vec::with_capacity(vars.len());
            let mut avoid = captured.clone();
            free_symbols(body, &mut Vec::new(), &mut avoid);
            for (name, sort) in vars {
                if captured.contains(name) {
                    let mut fresh = format!("{name}'");
                    while avoid.contains(fresh.as_str()) {
                        fresh.push('\'');
                    }
                    let fresh: Rc<str> = fresh.into();
                    avoid.insert(fresh.clone());
                    inner.insert(name.clone(), Rc::new(Term::Symbol(fresh.clone())));
                    renamed.push((fresh, sort.clone()));
                } else {
                    renamed.push((name.clone(), sort.clone()));
                }
            }
            Rc::new(Term::Binder {
                quantifier: quantifier.clone(),
                vars: renamed,
                body: substitute(body, &inner),
            })
        }
        Term::Number { .. } | Term::Other(_) => term.clone(),
    }
}

/// Whether `left` and `right` are equal up to the renaming of bound
/// variables (and the orientation of equalities).
fn alpha_equal(left: &Term, right: &Term) -> bool {
    fn equal(left: &Term, right: &Term, pairs: &mut Vec<(Rc<str>, Rc<str>)>) -> bool {
        match (left, right) {
            (Term::Symbol(x), Term::Symbol(y)) => {
                match pairs.iter().rev().find(|(a, b)| a == x || b == y) {
                    Some((a, b)) => a == x && b == y,
                    None => x == y,
                }
            }
            (Term::App(f, xs), Term::App(g, ys)) => {
                f == g
                    && xs.len() == ys.len()
                    && (xs.iter().zip(ys).all(|(x, y)| equal(x, y, pairs))
                        || (op(f) == Some(Op::Eq)
                            && xs.len() == 2
                            && equal(&xs[0], &ys[1], pairs)
                            && equal(&xs[1], &ys[0], pairs)))
            }
            (
                Term::Binder {
                    quantifier: q,
                    vars: xs,
                    body: a,
                },
                Term::Binder {
                    quantifier: r,
                    vars: ys,
                    body: b,
                },
            ) => {
                if q != r
                    || xs.len() != ys.len()
                    || xs.iter().zip(ys).any(|((_, s), (_, t))| s != t)
                {
                    return false;
                }
                let depth = pairs.len();
                pairs.extend(
                    xs.iter()
                        .zip(ys)
                        .map(|((x, _), (y, _))| (x.clone(), y.clone())),
                );
                let same = equal(a, b, pairs);
                pairs.truncate(depth);
                same
            }
            _ => left == right,
        }
    }
    equal(left, right, &mut Vec::new())
}

/// A numeric literal: an integer numeral (optionally negative), a decimal,
/// or a rational `p/q`.
fn number(atom: &str) -> Option<Term> {
    let (negative, digits) = match atom.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, atom),
    };
    if digits.is_empty() || !digits.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let sign = |value: Rational| if negative { value.neg() } else { value };
    if let Some((numerator, denominator)) = digits.split_once('/') {
        let n = integer(numerator)?;
        let d = integer(denominator)?;
        let value = Rational::new(n, d).ok()?;
        return Some(Term::Number {
            value: sign(value),
            real: true,
        });
    }
    if let Some((whole, fraction)) = digits.split_once('.') {
        let scale = u64::try_from(fraction.len()).ok()?;
        let joined = format!("{whole}{fraction}");
        let value = Rational::from_integer(integer(&joined)?).divided_by_power_of_ten(scale);
        return Some(Term::Number {
            value: sign(value),
            real: true,
        });
    }
    Some(Term::Number {
        value: sign(Rational::from_integer(integer(digits)?)),
        real: false,
    })
}

fn integer(digits: &str) -> Option<Integer> {
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Integer::from_str(digits).ok()
}

/// `sexpr` printed back with single spaces, for a form read only
/// syntactically.
fn render(sexpr: &Sexpr) -> String {
    match sexpr {
        Sexpr::Atom(atom) => atom.to_string(),
        Sexpr::List(items) => {
            let inner: Vec<String> = items.iter().map(render).collect();
            format!("({})", inner.join(" "))
        }
    }
}

/// The query a part's proof refutes, as the checker reads it.
#[derive(Debug)]
pub(crate) struct Query {
    /// The asserted terms, which a top-level `assume` may introduce.
    assertions: HashSet<Rc<Term>>,
    /// The constants declared `Int`, for `la_generic`'s strengthening.
    integers: HashSet<Rc<str>>,
    /// Every declared constant's sort.
    sorts: HashMap<Rc<str>, Sort>,
    /// The query's `:named` proxies, which stay valid in the proof.
    names: HashMap<Rc<str>, Rc<Term>>,
}

/// Read `bytes` as an SMT-LIB script: its `declare-const` commands and its
/// assertions. `None` when it does not parse.
/// `#[string_edge]`: reads the SMT-LIB script's command keywords and sorts.
#[ix_trace_rs::string_edge]
pub(crate) fn read_query(bytes: &[u8]) -> Option<Query> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut names = Names::default();
    let mut assertions = HashSet::new();
    let mut integers = HashSet::new();
    let mut sorts = HashMap::new();
    for command in read_sexprs(text)? {
        let Sexpr::List(items) = &command else {
            return None;
        };
        let Some(Sexpr::Atom(head)) = items.first() else {
            return None;
        };
        match (&**head, &items[1..]) {
            ("declare-const", [Sexpr::Atom(name), Sexpr::Atom(sort)]) => {
                if &**sort == "Int" {
                    integers.insert(name.clone());
                }
                sorts.insert(name.clone(), Sort::named(sort));
            }
            ("assert", [term]) => {
                assertions.insert(names.term(term)?);
            }
            _ => {}
        }
    }
    Some(Query {
        assertions,
        integers,
        sorts,
        names: names.terms,
    })
}

/// A term's sort, as far as the checker types terms.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Sort {
    Bool,
    Int,
    Real,
    /// Any other sort, by its text.
    Named(Rc<str>),
}

impl Sort {
    /// `#[string_edge]`: reads a sort's SMT-LIB spelling.
    #[ix_trace_rs::string_edge]
    fn named(text: &str) -> Self {
        match text {
            "Bool" => Self::Bool,
            "Int" => Self::Int,
            "Real" => Self::Real,
            other => Self::Named(other.into()),
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Real)
    }
}

/// Why a term has no sort.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Unsorted {
    /// An operand has a sort its operator does not take: the term is
    /// ill-sorted, so no rule may read it.
    IllSorted,
    /// A symbol or operator the checker does not type.
    Unknown,
}

/// The sort of `term` over `query`'s declarations and the variables of
/// `scope`, innermost last.
fn sort_of(term: &Term, query: &Query, scope: &mut Vec<(Rc<str>, Sort)>) -> Result<Sort, Unsorted> {
    match term {
        Term::Symbol(name) => {
            if let Some((_, sort)) = scope.iter().rev().find(|(bound, _)| bound == name) {
                return Ok(sort.clone());
            }
            match boolean_literal(name) {
                Some(_) => Ok(Sort::Bool),
                None => query.sorts.get(name).cloned().ok_or(Unsorted::Unknown),
            }
        }
        Term::Number { real, .. } => Ok(if *real { Sort::Real } else { Sort::Int }),
        Term::App(head, args) => {
            let sorts = args
                .iter()
                .map(|arg| sort_of(arg, query, scope))
                .collect::<Result<Vec<_>, _>>()?;
            application_sort(head, &sorts)
        }
        Term::Binder {
            quantifier,
            vars,
            body,
        } => {
            let depth = scope.len();
            scope.extend(
                vars.iter()
                    .map(|(name, sort)| (name.clone(), Sort::named(sort))),
            );
            let body = sort_of(body, query, scope);
            scope.truncate(depth);
            let body = body?;
            match self::quantifier(quantifier) {
                Some(Quantifier::Forall | Quantifier::Exists) if body == Sort::Bool => {
                    Ok(Sort::Bool)
                }
                Some(Quantifier::Forall | Quantifier::Exists) => Err(Unsorted::IllSorted),
                Some(Quantifier::Choice) => match vars.as_slice() {
                    [(_, sort)] if body == Sort::Bool => Ok(Sort::named(sort)),
                    [_] => Err(Unsorted::IllSorted),
                    _ => Err(Unsorted::Unknown),
                },
                _ => Err(Unsorted::Unknown),
            }
        }
        Term::Other(_) => Err(Unsorted::Unknown),
    }
}

/// The sort of `head` applied to operands of `sorts`.
fn application_sort(head: &str, sorts: &[Sort]) -> Result<Sort, Unsorted> {
    let all = |wanted: &dyn Fn(&Sort) -> bool| sorts.iter().all(wanted);
    let numeric = || {
        if sorts.contains(&Sort::Real) {
            Sort::Real
        } else {
            Sort::Int
        }
    };
    let ill = Err(Unsorted::IllSorted);
    let Some(op) = op(head) else {
        return Err(Unsorted::Unknown);
    };
    match op {
        Op::Not => match sorts {
            [Sort::Bool] => Ok(Sort::Bool),
            _ => ill,
        },
        Op::And | Op::Or | Op::Implies | Op::Xor => {
            if all(&|sort| *sort == Sort::Bool) {
                Ok(Sort::Bool)
            } else {
                ill
            }
        }
        Op::Eq | Op::Distinct => match sorts {
            [first, rest @ ..] if !rest.is_empty() => {
                let agree = rest
                    .iter()
                    .all(|sort| sort == first || (sort.is_numeric() && first.is_numeric()));
                if agree {
                    Ok(Sort::Bool)
                } else {
                    ill
                }
            }
            _ => ill,
        },
        Op::Lt | Op::Le | Op::Gt | Op::Ge => {
            if sorts.len() >= 2 && all(&Sort::is_numeric) {
                Ok(Sort::Bool)
            } else {
                ill
            }
        }
        Op::Add | Op::Mul | Op::Sub => {
            if !sorts.is_empty() && all(&Sort::is_numeric) {
                Ok(numeric())
            } else {
                ill
            }
        }
        Op::Div => {
            if sorts.len() >= 2 && all(&Sort::is_numeric) {
                Ok(Sort::Real)
            } else {
                ill
            }
        }
        Op::IntDiv | Op::Mod => {
            if sorts.len() >= 2 && all(&|sort| *sort == Sort::Int) {
                Ok(Sort::Int)
            } else {
                ill
            }
        }
        Op::Abs => match sorts {
            [Sort::Int] => Ok(Sort::Int),
            _ => ill,
        },
        Op::ToReal => match sorts {
            [Sort::Int] => Ok(Sort::Real),
            _ => ill,
        },
        Op::ToInt => match sorts {
            [Sort::Real] => Ok(Sort::Int),
            _ => ill,
        },
        Op::IsInt => match sorts {
            [Sort::Real] => Ok(Sort::Bool),
            _ => ill,
        },
        Op::Ite => match sorts {
            [Sort::Bool, then, otherwise] if then == otherwise => Ok(then.clone()),
            [Sort::Bool, then, otherwise] if then.is_numeric() && otherwise.is_numeric() => {
                Ok(Sort::Real)
            }
            _ => ill,
        },
    }
}

/// FR-314's sort test of the formulas a command states: each is
/// `Bool`-sorted, `Fails` when one is ill-sorted or not `Bool` and
/// `Undecided` when one has a symbol or operator the checker does not type.
fn formulas_sorted<'t>(
    formulas: impl IntoIterator<Item = &'t Rc<Term>>,
    query: &Query,
    context: &[Entry],
) -> Check {
    let mut scope: Vec<_> = context
        .iter()
        .map(|entry| (entry.name.clone(), Sort::named(&entry.sort)))
        .collect();
    for formula in formulas {
        match sort_of(formula, query, &mut scope) {
            Ok(Sort::Bool) => {}
            Ok(_) | Err(Unsorted::IllSorted) => return Check::Fails,
            Err(Unsorted::Unknown) => return Check::Undecided,
        }
    }
    Check::Holds
}

/// A checked or unchecked Alethe rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rule {
    Refl,
    Trans,
    Cong,
    EqReflexive,
    EqTransitive,
    EqCongruent,
    EqCongruentPred,
    Resolution,
    Contraction,
    Reordering,
    Tautology,
    NotNot,
    And,
    NotOr,
    Or,
    NotAnd,
    Implies,
    NotImplies1,
    NotImplies2,
    Equiv1,
    Equiv2,
    NotEquiv1,
    NotEquiv2,
    Ite1,
    Ite2,
    NotIte1,
    NotIte2,
    AndPos,
    AndNeg,
    OrPos,
    OrNeg,
    ImpliesPos,
    ImpliesNeg1,
    ImpliesNeg2,
    EquivPos1,
    EquivPos2,
    EquivNeg1,
    EquivNeg2,
    ItePos1,
    ItePos2,
    IteNeg1,
    IteNeg2,
    False,
    NotSymm,
    LaGeneric,
    LaDisequality,
    LaTotality,
    LaTautology,
    LaMultPos,
    LaMultNeg,
    PolySimp,
    PolySimpRel,
    Bind,
    Subproof,
}

/// FR-314's checked Alethe rules, by name. `resolution` and
/// `th_resolution` are one rule (the Alethe specification: the names differ
/// only in which solver component emitted them).
const CHECKED: &[(&str, Rule)] = &[
    ("refl", Rule::Refl),
    ("trans", Rule::Trans),
    ("cong", Rule::Cong),
    ("eq_reflexive", Rule::EqReflexive),
    ("eq_transitive", Rule::EqTransitive),
    ("eq_congruent", Rule::EqCongruent),
    ("eq_congruent_pred", Rule::EqCongruentPred),
    ("resolution", Rule::Resolution),
    ("th_resolution", Rule::Resolution),
    ("contraction", Rule::Contraction),
    ("reordering", Rule::Reordering),
    ("tautology", Rule::Tautology),
    ("not_not", Rule::NotNot),
    ("and", Rule::And),
    ("not_or", Rule::NotOr),
    ("or", Rule::Or),
    ("not_and", Rule::NotAnd),
    ("implies", Rule::Implies),
    ("not_implies1", Rule::NotImplies1),
    ("not_implies2", Rule::NotImplies2),
    ("equiv1", Rule::Equiv1),
    ("equiv2", Rule::Equiv2),
    ("not_equiv1", Rule::NotEquiv1),
    ("not_equiv2", Rule::NotEquiv2),
    ("ite1", Rule::Ite1),
    ("ite2", Rule::Ite2),
    ("not_ite1", Rule::NotIte1),
    ("not_ite2", Rule::NotIte2),
    ("and_pos", Rule::AndPos),
    ("and_neg", Rule::AndNeg),
    ("or_pos", Rule::OrPos),
    ("or_neg", Rule::OrNeg),
    ("implies_pos", Rule::ImpliesPos),
    ("implies_neg1", Rule::ImpliesNeg1),
    ("implies_neg2", Rule::ImpliesNeg2),
    ("equiv_pos1", Rule::EquivPos1),
    ("equiv_pos2", Rule::EquivPos2),
    ("equiv_neg1", Rule::EquivNeg1),
    ("equiv_neg2", Rule::EquivNeg2),
    ("ite_pos1", Rule::ItePos1),
    ("ite_pos2", Rule::ItePos2),
    ("ite_neg1", Rule::IteNeg1),
    ("ite_neg2", Rule::IteNeg2),
    ("false", Rule::False),
    ("not_symm", Rule::NotSymm),
    ("la_generic", Rule::LaGeneric),
    ("la_disequality", Rule::LaDisequality),
    ("la_totality", Rule::LaTotality),
    ("la_tautology", Rule::LaTautology),
    ("la_mult_pos", Rule::LaMultPos),
    ("la_mult_neg", Rule::LaMultNeg),
    ("bind", Rule::Bind),
    ("subproof", Rule::Subproof),
    ("poly_simp", Rule::PolySimp),
    ("poly_simp_rel", Rule::PolySimpRel),
];

/// The checked rule `name` names, or `None` for an unchecked rule
/// (`hole`, `lia_generic` and every rule FR-314 does not list).
#[ix_trace_rs::string_edge]
fn checked_rule(name: &str) -> Option<Rule> {
    CHECKED
        .iter()
        .find(|(checked, _)| *checked == name)
        .map(|(_, rule)| *rule)
}

/// One numbered command of a proof.
#[derive(Debug)]
struct Command {
    kind: CommandKind,
    /// Whether every premise and discharged assumption names a command
    /// earlier in a scope still open at this command.
    premises_visible: bool,
    /// The context `Γ` of the command: the arguments of every enclosing
    /// anchor, outermost first.
    context: Rc<[Entry]>,
}

/// One element of a context: a fixed variable, or a variable mapped to a
/// term.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Entry {
    name: Rc<str>,
    sort: Rc<str>,
    value: Option<Rc<Term>>,
}

/// `subst(Γ)`: a fixed variable maps to itself, shadowing an earlier
/// mapping; a mapping composes with the substitution before it, so
/// `subst([x ↦ 7, x ↦ g(x)]) = {x ↦ g(7)}`.
fn substitution(context: &[Entry]) -> HashMap<Rc<str>, Rc<Term>> {
    let mut map = HashMap::new();
    for entry in context {
        match &entry.value {
            None => {
                map.remove(&entry.name);
            }
            Some(value) => {
                let value = substitute(value, &map);
                map.insert(entry.name.clone(), value);
            }
        }
    }
    map
}

#[derive(Debug)]
enum CommandKind {
    /// `(assume id φ)`; `local` inside a subproof.
    Assume { term: Rc<Term>, local: bool },
    /// `(step id (cl …) :rule r …)`.
    Step(Box<Step>),
    /// `(anchor :step id [:args …])`.
    Anchor,
    /// `(define-fun …)`.
    Define,
}

/// An equality as written, its sides in their source order: the
/// conclusion of a step with a non-empty context is oriented.
type Oriented = Option<(Rc<Term>, Rc<Term>)>;

#[derive(Debug)]
struct Step {
    clause: Vec<Rc<Term>>,
    /// The clause's single literal, when it is an equality, as written.
    oriented: Oriented,
    rule: Rc<str>,
    /// Each premise's conclusion clause.
    premises: Vec<Rc<[Rc<Term>]>>,
    /// Each premise's single equality as written.
    premises_oriented: Vec<Oriented>,
    args: Vec<Rc<Term>>,
    /// The source order of the two relation atoms in a `poly_simp_rel`
    /// conclusion. The general term representation canonicalizes `=`; this
    /// narrow view retains orientation for exact scale checks.
    poly_relations: Option<PolyRelations>,
    /// For a step that closes a subproof: the subproof's assumptions, in
    /// the order of `:discharge`, whether `:discharge` names exactly the
    /// subproof's assumptions, and the clause of its last command.
    closes: Option<Subproof>,
}

#[derive(Debug)]
struct PolyRelations {
    left_operator: Rc<str>,
    left: (Rc<Term>, Rc<Term>),
    right_operator: Rc<str>,
    right: (Rc<Term>, Rc<Term>),
}

#[derive(Debug)]
struct Subproof {
    discharged: Vec<Rc<Term>>,
    discharges_exactly: bool,
    last: Option<Rc<[Rc<Term>]>>,
    /// The last command's single equality as written.
    last_oriented: Oriented,
    /// The anchor's own arguments.
    arguments: Vec<Entry>,
}

/// A part's proof, read into numbered commands.
#[derive(Debug)]
pub(crate) struct Proof {
    commands: Vec<Command>,
}

/// What a proof that no rule rejects shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProofOutcome {
    /// Every command is checked: the proof refutes its query.
    Refutes,
    /// The command numbered `step` is the first whose rule is unchecked.
    Unverifiable { step: u64 },
}

/// What reading a proof gives.
pub(crate) enum Reading {
    /// The proof's numbered commands.
    Read(Proof),
    /// The proof is not a sequence of well-formed Alethe commands.
    Malformed,
    /// A charge was refused, and the read stopped.
    Stopped,
}

/// An open subproof while the proof is read.
struct Frame {
    closing: Rc<str>,
    /// The anchor's arguments.
    arguments: Vec<Entry>,
    /// The ids defined in this subproof, removed from view when it closes.
    defined: Vec<Rc<str>>,
    assumptions: Vec<(Rc<str>, Rc<Term>)>,
    last: Option<(Rc<[Rc<Term>]>, Oriented)>,
}

/// The context the open `frames` give.
fn context_of(frames: &[Frame]) -> Rc<[Entry]> {
    frames
        .iter()
        .flat_map(|frame| frame.arguments.iter().cloned())
        .collect()
}

/// An anchor's `:args`: each `(x S)` fixes `x`, each `(:= (x S) t)` maps it.
/// `#[string_edge]`: reads an anchor's `:args` text.
#[ix_trace_rs::string_edge]
fn anchor_arguments(value: Option<&Sexpr>, names: &mut Names) -> Option<Vec<Entry>> {
    let Some(value) = value else {
        return Some(Vec::new());
    };
    let Sexpr::List(items) = value else {
        return None;
    };
    items
        .iter()
        .map(|item| {
            let Sexpr::List(parts) = item else {
                return None;
            };
            match parts.as_slice() {
                [Sexpr::Atom(name), sort] => Some(Entry {
                    name: name.clone(),
                    sort: render(sort).into(),
                    value: None,
                }),
                [Sexpr::Atom(assign), Sexpr::List(var), term] if &**assign == ":=" => {
                    let [Sexpr::Atom(name), sort] = var.as_slice() else {
                        return None;
                    };
                    Some(Entry {
                        name: name.clone(),
                        sort: render(sort).into(),
                        value: Some(names.term(term)?),
                    })
                }
                _ => None,
            }
        })
        .collect()
}

/// The single literal of `(cl l)` as written, when it is an equality.
/// `#[string_edge]`: reads a clause's single equality as written in the proof text.
#[ix_trace_rs::string_edge]
fn oriented_literal(clause: &Sexpr, names: &mut Names) -> Oriented {
    let Sexpr::List(items) = clause else {
        return None;
    };
    let [_, Sexpr::List(literal)] = items.as_slice() else {
        return None;
    };
    let [Sexpr::Atom(head), left, right] = literal.as_slice() else {
        return None;
    };
    if &**head != "=" {
        return None;
    }
    Some((names.term(left)?, names.term(right)?))
}

/// Read `bytes` as an Alethe proof of `query`, numbering its commands.
/// `charge` is called with the edge count of each command (one, plus one per
/// `:premises` and `:discharge` entry) before the command is read, and stops
/// the read when it returns `false`.
/// `#[string_edge]`: reads the Alethe proof's command keywords and attributes.
#[ix_trace_rs::string_edge]
pub(crate) fn read_proof(
    bytes: &[u8],
    query: &Query,
    charge: &mut dyn FnMut(u64) -> bool,
) -> Reading {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Reading::Malformed;
    };
    let Some(sexprs) = read_sexprs(text) else {
        return Reading::Malformed;
    };
    let mut names = Names {
        terms: query.names.clone(),
        ..Names::default()
    };
    let mut defined: HashSet<Rc<str>> = HashSet::new();
    // Each visible id's conclusion clause, and its equality as written.
    let mut visible: HashMap<Rc<str>, (Clause, Oriented)> = HashMap::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut commands = Vec::with_capacity(sexprs.len());
    for sexpr in &sexprs {
        let Sexpr::List(items) = sexpr else {
            return Reading::Malformed;
        };
        let Some(Sexpr::Atom(head)) = items.first() else {
            return Reading::Malformed;
        };
        let attributes = |from: usize| attributes(items.get(from..).unwrap_or_default());
        let context = context_of(&frames);
        let cost = match &**head {
            "step" => match attributes(3) {
                Some(attrs) => {
                    1 + list_len(attrs.get(":premises")) + list_len(attrs.get(":discharge"))
                }
                None => return Reading::Malformed,
            },
            _ => 1,
        };
        if !charge(cost) {
            return Reading::Stopped;
        }
        match (&**head, &items[1..]) {
            ("assume", [Sexpr::Atom(id), term]) => {
                let Some(term) = names.term(term) else {
                    return Reading::Malformed;
                };
                if !defined.insert(id.clone()) {
                    return Reading::Malformed;
                }
                let clause: Rc<[Rc<Term>]> = Rc::from(vec![term.clone()]);
                let local = !frames.is_empty();
                if let Some(frame) = frames.last_mut() {
                    frame.defined.push(id.clone());
                    frame.assumptions.push((id.clone(), term.clone()));
                    frame.last = Some((clause.clone(), None));
                }
                visible.insert(id.clone(), (clause, None));
                commands.push(Command {
                    kind: CommandKind::Assume { term, local },
                    premises_visible: true,
                    context,
                });
            }
            ("anchor", _) => {
                let Some(attrs) = attributes(1) else {
                    return Reading::Malformed;
                };
                let Some(Sexpr::Atom(closing)) = attrs.get(":step") else {
                    return Reading::Malformed;
                };
                let Some(arguments) = anchor_arguments(attrs.get(":args"), &mut names) else {
                    return Reading::Malformed;
                };
                frames.push(Frame {
                    closing: closing.clone(),
                    arguments,
                    defined: Vec::new(),
                    assumptions: Vec::new(),
                    last: None,
                });
                commands.push(Command {
                    kind: CommandKind::Anchor,
                    premises_visible: true,
                    context,
                });
            }
            ("define-fun", [Sexpr::Atom(name), Sexpr::List(parameters), _sort, body]) => {
                let Some(parameters) = sorted_vars(parameters) else {
                    return Reading::Malformed;
                };
                let depth = names.bound.len();
                names
                    .bound
                    .extend(parameters.iter().map(|(parameter, _)| parameter.clone()));
                let body = names.term(body);
                names.bound.truncate(depth);
                let Some(body) = body else {
                    return Reading::Malformed;
                };
                if parameters.is_empty() {
                    names.terms.insert(name.clone(), body);
                } else {
                    let parameters = parameters.into_iter().map(|(name, _)| name).collect();
                    names.functions.insert(name.clone(), (parameters, body));
                }
                commands.push(Command {
                    kind: CommandKind::Define,
                    premises_visible: true,
                    context,
                });
            }
            ("step", [Sexpr::Atom(id), clause, ..]) => {
                let Some(attrs) = attributes(3) else {
                    return Reading::Malformed;
                };
                let oriented = oriented_literal(clause, &mut names);
                let poly_relations = ordered_poly_relations(clause, &mut names);
                let Some(clause) = read_clause(clause, &mut names) else {
                    return Reading::Malformed;
                };
                let Some(Sexpr::Atom(rule)) = attrs.get(":rule") else {
                    return Reading::Malformed;
                };
                let Some(premise_ids) = id_list(attrs.get(":premises")) else {
                    return Reading::Malformed;
                };
                let Some(discharge_ids) = id_list(attrs.get(":discharge")) else {
                    return Reading::Malformed;
                };
                let args = match attrs.get(":args") {
                    None => Vec::new(),
                    Some(Sexpr::List(args)) => {
                        let Some(args) = args
                            .iter()
                            .map(|arg| names.term(arg))
                            .collect::<Option<Vec<_>>>()
                        else {
                            return Reading::Malformed;
                        };
                        args
                    }
                    Some(Sexpr::Atom(_)) => return Reading::Malformed,
                };
                if !defined.insert(id.clone()) {
                    return Reading::Malformed;
                }
                // A step whose id another open subproof closes is out of
                // place.
                if frames
                    .iter()
                    .rev()
                    .skip(1)
                    .any(|frame| frame.closing == *id)
                {
                    return Reading::Malformed;
                }
                let mut premises_visible = true;
                let (premises, premises_oriented): (Vec<_>, Vec<_>) = premise_ids
                    .iter()
                    .filter_map(|premise| {
                        let found = visible.get(premise).cloned();
                        premises_visible &= found.is_some();
                        found
                    })
                    .unzip();
                let clause: Rc<[Rc<Term>]> = Rc::from(clause);
                let closes = if frames.last().is_some_and(|frame| frame.closing == *id) {
                    let Some(frame) = frames.pop() else {
                        return Reading::Malformed;
                    };
                    for gone in &frame.defined {
                        visible.remove(gone);
                    }
                    let assumed: HashMap<&Rc<str>, &Rc<Term>> = frame
                        .assumptions
                        .iter()
                        .map(|(id, term)| (id, term))
                        .collect();
                    let mut discharged = Vec::with_capacity(discharge_ids.len());
                    for id in &discharge_ids {
                        match assumed.get(id) {
                            Some(term) => discharged.push((*term).clone()),
                            None => premises_visible = false,
                        }
                    }
                    let named: HashSet<&Rc<str>> = discharge_ids.iter().collect();
                    let discharges_exactly = named.len() == discharge_ids.len()
                        && named.len() == assumed.len()
                        && assumed.keys().all(|id| named.contains(id));
                    let (last, last_oriented) = frame.last.unzip();
                    Some(Subproof {
                        discharged,
                        discharges_exactly,
                        last,
                        last_oriented: last_oriented.flatten(),
                        arguments: frame.arguments,
                    })
                } else {
                    if !discharge_ids.is_empty() {
                        premises_visible = false;
                    }
                    None
                };
                if let Some(frame) = frames.last_mut() {
                    frame.defined.push(id.clone());
                    frame.last = Some((clause.clone(), oriented.clone()));
                }
                visible.insert(id.clone(), (clause.clone(), oriented.clone()));
                // A closing step stands in the context outside its subproof.
                let context = if closes.is_some() {
                    context_of(&frames)
                } else {
                    context
                };
                commands.push(Command {
                    kind: CommandKind::Step(Box::new(Step {
                        clause: clause.to_vec(),
                        oriented,
                        rule: rule.clone(),
                        premises,
                        premises_oriented,
                        args,
                        poly_relations,
                        closes,
                    })),
                    premises_visible,
                    context,
                });
            }
            _ => return Reading::Malformed,
        }
    }
    if !frames.is_empty() {
        return Reading::Malformed;
    }
    Reading::Read(Proof { commands })
}

/// The keyword attributes of a command: each `:key` with the value after
/// it, or `None` when the next item is itself a keyword. `None` when an item
/// that should be a keyword is not one, or a keyword repeats.
fn attributes(items: &[Sexpr]) -> Option<Attributes<'_>> {
    let mut map = BTreeMap::new();
    let mut at = 0;
    while let Some(key) = items.get(at) {
        let Sexpr::Atom(key) = key else {
            return None;
        };
        if !key.starts_with(':') {
            return None;
        }
        at += 1;
        let value = match items.get(at) {
            Some(value) if !matches!(value, Sexpr::Atom(next) if next.starts_with(':')) => {
                at += 1;
                Some(value)
            }
            _ => None,
        };
        if map.insert(&**key, value).is_some() {
            return None;
        }
    }
    Some(Attributes(map))
}

/// A command's keyword attributes.
struct Attributes<'s>(BTreeMap<&'s str, Option<&'s Sexpr>>);

impl<'s> Attributes<'s> {
    /// The value of `key`, if it has one.
    fn get(&self, key: &str) -> Option<&'s Sexpr> {
        self.0.get(key).copied().flatten()
    }
}

fn list_len(value: Option<&Sexpr>) -> u64 {
    match value {
        Some(Sexpr::List(items)) => u64::try_from(items.len()).unwrap_or(u64::MAX),
        _ => 0,
    }
}

/// The ids of a `:premises` or `:discharge` list; `None` when it is not a
/// list of symbols.
fn id_list(value: Option<&Sexpr>) -> Option<Vec<Rc<str>>> {
    match value {
        None => Some(Vec::new()),
        Some(Sexpr::List(items)) => items
            .iter()
            .map(|item| match item {
                Sexpr::Atom(id) => Some(id.clone()),
                Sexpr::List(_) => None,
            })
            .collect(),
        Some(Sexpr::Atom(_)) => None,
    }
}

/// A step's `(cl l1 … ln)`.
/// `#[string_edge]`: reads a `(cl ...)` clause from the proof text.
#[ix_trace_rs::string_edge]
fn read_clause(sexpr: &Sexpr, names: &mut Names) -> Option<Vec<Rc<Term>>> {
    let Sexpr::List(items) = sexpr else {
        return None;
    };
    let (Sexpr::Atom(head), literals) = items.split_first()? else {
        return None;
    };
    if &**head != "cl" {
        return None;
    }
    literals.iter().map(|literal| names.term(literal)).collect()
}

/// The two relation atoms of a `poly_simp_rel` conclusion, retaining the
/// source orientation that `Term::app` intentionally canonicalizes away.
fn ordered_poly_relations(sexpr: &Sexpr, names: &mut Names) -> Option<PolyRelations> {
    fn relation(sexpr: &Sexpr, names: &mut Names) -> Option<(Rc<str>, (Rc<Term>, Rc<Term>))> {
        let Sexpr::List(parts) = sexpr else {
            return None;
        };
        let [Sexpr::Atom(operator), left, right] = parts.as_slice() else {
            return None;
        };
        matches!(&**operator, "=" | "<" | "<=" | ">" | ">=").then_some(())?;
        Some((operator.clone(), (names.term(left)?, names.term(right)?)))
    }
    let Sexpr::List(parts) = sexpr else {
        return None;
    };
    let [Sexpr::Atom(cl), literal] = parts.as_slice() else {
        return None;
    };
    if &**cl != "cl" {
        return None;
    }
    let Sexpr::List(equality) = literal else {
        return None;
    };
    let [Sexpr::Atom(operator), left, right] = equality.as_slice() else {
        return None;
    };
    if &**operator != "=" {
        return None;
    }
    let (left_operator, left) = relation(left, names)?;
    let (right_operator, right) = relation(right, names)?;
    Some(PolyRelations {
        left_operator,
        left,
        right_operator,
        right,
    })
}

impl Proof {
    /// The number of commands.
    pub(crate) fn len(&self) -> usize {
        self.commands.len()
    }

    /// FR-314's `ProofStepInvalid` rule over every command, in order.
    pub(crate) fn invalid_step(
        &self,
        query: &Query,
        charge: &mut impl FnMut(u64) -> bool,
    ) -> Result<Option<u64>, ()> {
        self.first(query, Check::Fails, charge)
    }

    /// FR-314's last-command rule: the index of a last command that does
    /// not conclude the empty clause.
    pub(crate) fn not_refutation(&self) -> Option<u64> {
        let last = self.commands.last()?;
        let refutes = matches!(&last.kind, CommandKind::Step(step) if step.clause.is_empty());
        (!refutes).then(|| u64::try_from(self.commands.len() - 1).unwrap_or(u64::MAX))
    }

    /// The outcome of a proof no rule rejects: `Unverifiable` at the first
    /// command this checker does not decide.
    pub(crate) fn outcome(
        &self,
        query: &Query,
        charge: &mut impl FnMut(u64) -> bool,
    ) -> Result<ProofOutcome, ()> {
        match self.first(query, Check::Undecided, charge)? {
            Some(step) => Ok(ProofOutcome::Unverifiable { step }),
            None => Ok(ProofOutcome::Refutes),
        }
    }

    fn first(
        &self,
        query: &Query,
        wanted: Check,
        charge: &mut impl FnMut(u64) -> bool,
    ) -> Result<Option<u64>, ()> {
        for (command, index) in self.commands.iter().zip(0_u64..) {
            if command.judge(query, charge)? == wanted {
                return Ok(Some(index));
            }
        }
        Ok(None)
    }
}

/// What the checker finds of one command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Check {
    /// The command follows by its rule.
    Holds,
    /// The command's rule is checked and its conclusion does not follow,
    /// or a premise is not an earlier visible command.
    Fails,
    /// The command's rule is outside the checked set, or this checker does
    /// not decide it here.
    Undecided,
}

type Polynomial = std::collections::BTreeMap<Vec<Rc<Term>>, Integer>;

/// Exact integer-polynomial normalisation used only by the two admitted
/// cvc5 polynomial rules. `None` is an unsupported or non-Int shape.
fn polynomial(
    term: &Term,
    query: &Query,
    charge: &mut impl FnMut(u64) -> bool,
) -> Result<Option<Polynomial>, ()> {
    fn add_into(
        into: &mut Polynomial,
        from: Polynomial,
        factor: &Integer,
        charge: &mut impl FnMut(u64) -> bool,
    ) -> Result<(), ()> {
        if !charge(u64::try_from(from.len()).unwrap_or(u64::MAX).max(1)) {
            return Err(());
        }
        for (monomial, coefficient) in from {
            let coefficient = coefficient.mul(factor);
            let value = into
                .get(&monomial)
                .cloned()
                .unwrap_or_else(Integer::zero)
                .add(&coefficient);
            if value.is_zero() {
                into.remove(&monomial);
            } else {
                into.insert(monomial, value);
            }
        }
        Ok(())
    }
    if !charge(1) {
        return Err(());
    }
    match term {
        Term::Number { value, real: false } if value.is_integer() => {
            let mut result = Polynomial::new();
            result.insert(Vec::new(), value.numerator().clone());
            Ok(Some(result))
        }
        Term::Number { .. } => Ok(None),
        Term::Symbol(_) if sort_of(term, query, &mut Vec::new()) == Ok(Sort::Int) => {
            let mut result = Polynomial::new();
            result.insert(vec![Rc::new(term.clone())], Integer::one());
            Ok(Some(result))
        }
        Term::Symbol(_) | Term::Binder { .. } | Term::Other(_) => Ok(None),
        Term::App(name, args) if &**name == "+" && !args.is_empty() => {
            let mut result = Polynomial::new();
            for arg in args {
                let Some(part) = polynomial(arg, query, charge)? else {
                    return Ok(None);
                };
                add_into(&mut result, part, &Integer::one(), charge)?;
            }
            Ok(Some(result))
        }
        Term::App(name, args) if &**name == "-" && !args.is_empty() => {
            let Some(first) = args.first() else {
                return Ok(None);
            };
            let Some(mut result) = polynomial(first, query, charge)? else {
                return Ok(None);
            };
            if args.len() == 1 {
                for value in result.values_mut() {
                    *value = value.neg();
                }
            } else {
                for arg in &args[1..] {
                    let Some(part) = polynomial(arg, query, charge)? else {
                        return Ok(None);
                    };
                    add_into(&mut result, part, &Integer::from(-1_i64), charge)?;
                }
            }
            Ok(Some(result))
        }
        Term::App(name, args) if &**name == "*" && !args.is_empty() => {
            let mut result = Polynomial::from([(Vec::new(), Integer::one())]);
            for arg in args {
                let Some(part) = polynomial(arg, query, charge)? else {
                    return Ok(None);
                };
                let work = result.len().saturating_mul(part.len());
                if !charge(u64::try_from(work).unwrap_or(u64::MAX).max(1)) {
                    return Err(());
                }
                let mut product = Polynomial::new();
                for (left, a) in &result {
                    for (right, b) in &part {
                        let mut monomial = left.clone();
                        monomial.extend(right.iter().cloned());
                        monomial.sort();
                        let coefficient = a.mul(b);
                        let total = product
                            .get(&monomial)
                            .cloned()
                            .unwrap_or_else(Integer::zero)
                            .add(&coefficient);
                        if total.is_zero() {
                            product.remove(&monomial);
                        } else {
                            product.insert(monomial, total);
                        }
                    }
                }
                result = product;
            }
            Ok(Some(result))
        }
        Term::App(_, _) => Ok(None),
    }
}

fn polynomial_equality(
    term: &Term,
    query: &Query,
    charge: &mut impl FnMut(u64) -> bool,
) -> Result<Option<(Polynomial, Polynomial)>, ()> {
    let Some([left, right]) = term.args_of("=").and_then(two) else {
        return Ok(None);
    };
    let Some(left_poly) = polynomial(left, query, charge)? else {
        return Ok(None);
    };
    let Some(right_poly) = polynomial(right, query, charge)? else {
        return Ok(None);
    };
    Ok(Some((left_poly, right_poly)))
}

fn polynomial_rule(
    step: &Step,
    rule: Rule,
    query: &Query,
    charge: &mut impl FnMut(u64) -> bool,
) -> Result<Check, ()> {
    let clause = match step.clause.as_slice() {
        [literal] => literal,
        _ => return Ok(Check::Undecided),
    };
    let (left, right) = if rule == Rule::PolySimp {
        if !step.premises.is_empty() {
            return Ok(Check::Fails);
        }
        let Some((left, right)) = polynomial_equality(clause, query, charge)? else {
            return Ok(Check::Undecided);
        };
        (left, right)
    } else {
        if step.premises.len() != 1 || step.premises[0].len() != 1 {
            return Ok(Check::Undecided);
        }
        let Some((premise_left, premise_right)) =
            polynomial_equality(&step.premises[0][0], query, charge)?
        else {
            return Ok(Check::Undecided);
        };
        if !polynomial_equal(&premise_left, &premise_right, charge)? {
            return Ok(Check::Fails);
        }
        let Some((left_op, left, right_op, right)) =
            relation_polynomial(step.poly_relations.as_ref(), query, charge)?
        else {
            return Ok(Check::Undecided);
        };
        if relation_scale(&premise_left, &left, charge)?.is_none()
            || relation_scale(&premise_right, &right, charge)?.is_none()
        {
            return Ok(Check::Fails);
        }
        if left_op != right_op {
            let reversed = matches!(
                (left_op, right_op),
                ("<", ">") | ("<=", ">=") | (">", "<") | (">=", "<=")
            );
            return Ok(
                if reversed && relation_scale(&left, &right, charge)? == Some(false) {
                    Check::Undecided
                } else {
                    Check::Fails
                },
            );
        }
        return Ok(match relation_scale(&left, &right, charge)? {
            Some(true) => Check::Holds,
            Some(false) if left_op == "=" => Check::Undecided,
            Some(false) => Check::Fails,
            None => Check::Fails,
        });
    };
    Ok(if polynomial_equal(&left, &right, charge)? {
        Check::Holds
    } else {
        Check::Fails
    })
}

fn relation_polynomial(
    relations: Option<&PolyRelations>,
    query: &Query,
    charge: &mut impl FnMut(u64) -> bool,
) -> Result<Option<(&'static str, Polynomial, &'static str, Polynomial)>, ()> {
    let Some(relations) = relations else {
        return Ok(None);
    };
    let left_op = match &*relations.left_operator {
        "=" => "=",
        "<" => "<",
        "<=" => "<=",
        ">" => ">",
        ">=" => ">=",
        _ => return Ok(None),
    };
    let right_op = match &*relations.right_operator {
        "=" => "=",
        "<" => "<",
        "<=" => "<=",
        ">" => ">",
        ">=" => ">=",
        _ => return Ok(None),
    };
    let (ll, lr) = &relations.left;
    let (rl, rr) = &relations.right;
    let Some(mut first) = polynomial(ll, query, charge)? else {
        return Ok(None);
    };
    let Some(second) = polynomial(lr, query, charge)? else {
        return Ok(None);
    };
    add_polynomial(&mut first, second, &Integer::from(-1_i64), charge)?;
    let Some(mut third) = polynomial(rl, query, charge)? else {
        return Ok(None);
    };
    let Some(fourth) = polynomial(rr, query, charge)? else {
        return Ok(None);
    };
    add_polynomial(&mut third, fourth, &Integer::from(-1_i64), charge)?;
    Ok(Some((left_op, first, right_op, third)))
}

/// Whether `right` is a positive exact-rational multiple of `left`. A
/// negative exact multiple is reported separately so the caller can leave
/// a semantically valid but unadmitted relation Unverifiable.
fn relation_scale(
    left: &Polynomial,
    right: &Polynomial,
    charge: &mut impl FnMut(u64) -> bool,
) -> Result<Option<bool>, ()> {
    if !charge(
        u64::try_from(left.len().saturating_add(right.len()))
            .unwrap_or(u64::MAX)
            .max(1),
    ) {
        return Err(());
    }
    if left.is_empty() || right.is_empty() {
        return Ok((left.is_empty() && right.is_empty()).then_some(true));
    }
    let Some((basis, coefficient)) = left.iter().find(|(_, coefficient)| !coefficient.is_zero())
    else {
        return Ok(None);
    };
    let Some(target) = right.get(basis) else {
        return Ok(None);
    };
    let Some(scale) =
        Rational::from_integer(target.clone()).div(&Rational::from_integer(coefficient.clone()))
    else {
        return Ok(None);
    };
    let exact = left.iter().all(|(monomial, value)| {
        right.get(monomial).is_some_and(|target| {
            scale.mul(&Rational::from_integer(value.clone()))
                == Rational::from_integer(target.clone())
        })
    }) && right.keys().all(|monomial| left.contains_key(monomial));
    Ok(exact.then_some(!scale.numerator().is_negative()))
}

fn polynomial_equal(
    left: &Polynomial,
    right: &Polynomial,
    charge: &mut impl FnMut(u64) -> bool,
) -> Result<bool, ()> {
    if !charge(
        u64::try_from(left.len().saturating_add(right.len()))
            .unwrap_or(u64::MAX)
            .max(1),
    ) {
        return Err(());
    }
    Ok(left == right)
}

fn add_polynomial(
    into: &mut Polynomial,
    from: Polynomial,
    factor: &Integer,
    charge: &mut impl FnMut(u64) -> bool,
) -> Result<(), ()> {
    if !charge(u64::try_from(from.len()).unwrap_or(u64::MAX).max(1)) {
        return Err(());
    }
    for (monomial, coefficient) in from {
        let total = into
            .get(&monomial)
            .cloned()
            .unwrap_or_else(Integer::zero)
            .add(&coefficient.mul(factor));
        if total.is_zero() {
            into.remove(&monomial);
        } else {
            into.insert(monomial, total);
        }
    }
    Ok(())
}

impl From<bool> for Check {
    fn from(holds: bool) -> Self {
        if holds {
            Self::Holds
        } else {
            Self::Fails
        }
    }
}

impl Command {
    /// FR-314's test of one command.
    fn judge(&self, query: &Query, charge: &mut impl FnMut(u64) -> bool) -> Result<Check, ()> {
        if !self.premises_visible {
            return Ok(Check::Fails);
        }
        Ok(match &self.kind {
            CommandKind::Assume { term, local } => {
                if !(*local || query.assertions.contains(term)) {
                    return Ok(Check::Fails);
                }
                formulas_sorted([term], query, &self.context)
            }
            CommandKind::Anchor | CommandKind::Define => Check::Holds,
            // A formula the checker cannot type leaves the step at most
            // `Undecided`, still `Fails` when its rule shows it wrong.
            CommandKind::Step(step) => match formulas_sorted(&step.clause, query, &self.context) {
                Check::Holds => step.judge(&self.context, query, charge)?,
                Check::Undecided => match step.judge(&self.context, query, charge)? {
                    Check::Fails => Check::Fails,
                    Check::Holds | Check::Undecided => Check::Undecided,
                },
                Check::Fails => Check::Fails,
            },
        })
    }
}

/// The literals of a clause as a set.
fn set(clause: &[Rc<Term>]) -> HashSet<&Rc<Term>> {
    clause.iter().collect()
}

/// A clause of exactly the literals `expected`, as sets.
fn is_clause(clause: &[Rc<Term>], expected: &[Rc<Term>]) -> bool {
    set(clause) == set(expected)
}

impl Step {
    /// FR-314's test of the step in `context`.
    fn judge(
        &self,
        context: &[Entry],
        query: &Query,
        charge: &mut impl FnMut(u64) -> bool,
    ) -> Result<Check, ()> {
        let Some(rule) = checked_rule(&self.rule) else {
            return Ok(Check::Undecided);
        };
        if matches!(rule, Rule::PolySimp | Rule::PolySimpRel) {
            return polynomial_rule(self, rule, query, charge);
        }
        Ok(match (&self.closes, rule) {
            (Some(subproof), Rule::Subproof) if subproof.arguments.is_empty() => {
                self.follows(Rule::Subproof, query).into()
            }
            (Some(subproof), Rule::Bind) => self.binds(subproof, context).into(),
            // A subproof closed by another rule, or `subproof` over a
            // context, is not decided here.
            (Some(_), _) => Check::Undecided,
            (None, Rule::Subproof | Rule::Bind) => Check::Fails,
            (None, rule) if !context.is_empty() => match rule {
                Rule::Refl | Rule::Trans | Rule::Cong => {
                    self.follows_in(rule, &substitution(context)).into()
                }
                // A rule that requires an empty context.
                _ => Check::Undecided,
            },
            (None, Rule::Resolution) => resolution(&self.premises, &self.clause, &self.args),
            (None, rule) => self.follows(rule, query).into(),
        })
    }

    /// `refl`, `trans` and `cong` in a non-empty context, whose substitution
    /// is `subst`: the conclusion's equality is oriented, its left side the
    /// original term, and `refl` holds when `subst` takes the left side to
    /// the right up to the renaming of bound variables.
    fn follows_in(&self, rule: Rule, subst: &HashMap<Rc<str>, Rc<Term>>) -> bool {
        let Some((left, right)) = &self.oriented else {
            return false;
        };
        if self.clause.len() != 1 {
            return false;
        }
        let premises: Option<Vec<(Rc<Term>, Rc<Term>)>> = self
            .premises
            .iter()
            .zip(&self.premises_oriented)
            .map(|(clause, oriented)| (clause.len() == 1).then(|| oriented.clone()).flatten())
            .collect();
        let Some(premises) = premises else {
            return false;
        };
        let renames = |from: &Rc<Term>, to: &Rc<Term>| alpha_equal(&substitute(from, subst), to);
        match rule {
            Rule::Refl => premises.is_empty() && renames(left, right),
            Rule::Trans => {
                let mut at = left.clone();
                for (from, to) in &premises {
                    if *from != at {
                        return false;
                    }
                    at = to.clone();
                }
                !premises.is_empty() && at == *right
            }
            Rule::Cong => {
                let (Term::App(f, xs), Term::App(g, ys)) = (&**left, &**right) else {
                    return false;
                };
                if f != g || xs.len() != ys.len() {
                    return false;
                }
                argument_orders(f, xs).iter().any(|xs| {
                    argument_orders(g, ys).iter().any(|ys| {
                        let mut next = premises.iter().peekable();
                        for (x, y) in xs.iter().zip(ys) {
                            if next.peek().is_some_and(|(from, to)| from == x && to == y) {
                                next.next();
                            } else if !renames(x, y) {
                                return false;
                            }
                        }
                        next.next().is_none()
                    })
                })
            }
            _ => false,
        }
    }

    /// `bind` (Alethe rule 18): the subproof's anchor fixes `y1 … yn` and
    /// maps `x1 ↦ y1 … xn ↦ yn`, its last step concludes `φ ≈ φ'`, and this
    /// step concludes `Q x1 … xn. φ ≈ Q y1 … yn. φ'`, with no `yi` free in
    /// the left side or named in the outer context.
    fn binds(&self, subproof: &Subproof, outer: &[Entry]) -> bool {
        let Some((left, right)) = &self.oriented else {
            return false;
        };
        let (
            Term::Binder {
                quantifier: q,
                vars: xs,
                body: phi,
            },
            Term::Binder {
                quantifier: r,
                vars: ys,
                body: phi_prime,
            },
        ) = (&**left, &**right)
        else {
            return false;
        };
        let n = xs.len();
        let arguments = &subproof.arguments;
        let fixed_then_mapped = n > 0
            && ys.len() == n
            && arguments.len() == 2 * n
            && arguments[..n].iter().zip(ys).all(|(entry, (y, sort))| {
                entry.value.is_none() && entry.name == *y && entry.sort == *sort
            })
            && arguments[n..]
                .iter()
                .zip(xs.iter().zip(ys))
                .all(|(entry, ((x, sort), (y, _)))| {
                    entry.name == *x
                        && entry.sort == *sort
                        && entry
                            .value
                            .as_deref()
                            .is_some_and(|value| value.is_symbol(y))
                });
        let fresh = ys
            .iter()
            .all(|(y, _)| !is_free_in(y, left) && !outer.iter().any(|entry| entry.name == *y));
        let inner = subproof
            .last_oriented
            .as_ref()
            .is_some_and(|(from, to)| from == phi && to == phi_prime);
        q == r
            && matches!(&**q, "forall" | "exists")
            && self.clause.len() == 1
            && self.premises.is_empty()
            && fixed_then_mapped
            && fresh
            && inner
    }

    /// The single premise's clause, for a one-premise rule.
    fn premise(&self) -> Option<&[Rc<Term>]> {
        match self.premises.as_slice() {
            [premise] => Some(premise),
            _ => None,
        }
    }

    /// The single literal of the single premise.
    fn premise_unit(&self) -> Option<&Rc<Term>> {
        match self.premise()? {
            [literal] => Some(literal),
            _ => None,
        }
    }

    /// The argument `k` as an index.
    /// The one integer argument `k` of `and`, `not_or`, `and_pos` and
    /// `or_neg`, which the Alethe rules require.
    fn index_arg(&self) -> Option<usize> {
        match self.args.as_slice() {
            [arg] => match &**arg {
                Term::Number { value, real: false } if value.is_integer() => value
                    .numerator()
                    .to_u64()
                    .and_then(|k| usize::try_from(k).ok()),
                _ => None,
            },
            _ => None,
        }
    }

    #[allow(clippy::too_many_lines)] // one arm per checked rule, as the rule table lists them
    fn follows(&self, rule: Rule, query: &Query) -> bool {
        let clause = self.clause.as_slice();
        let no_premises = self.premises.is_empty();
        let not = Term::not;
        match rule {
            Rule::Subproof => self.closes.as_ref().is_some_and(|subproof| {
                let Some(last) = &subproof.last else {
                    return false;
                };
                let mut expected: Vec<Rc<Term>> = subproof.discharged.iter().map(not).collect();
                expected.extend(last.iter().cloned());
                subproof.discharges_exactly && is_clause(clause, &expected)
            }),
            // Decided in `check`, against its subproof.
            Rule::Bind => false,
            Rule::Refl => match clause {
                [literal] => literal
                    .args_of("=")
                    .is_some_and(|args| matches!(args, [left, right] if alpha_equal(left, right))),
                _ => false,
            },
            Rule::EqReflexive => no_premises && self.follows(Rule::Refl, query),
            Rule::Trans => match clause {
                [literal] => {
                    let Some([first, last]) = literal.args_of("=").and_then(two) else {
                        return false;
                    };
                    let Some(equalities) = unit_equalities(&self.premises) else {
                        return false;
                    };
                    chain(first, last, &equalities) || chain(last, first, &equalities)
                }
                _ => false,
            },
            Rule::Cong => match clause {
                [literal] => {
                    let Some([left, right]) = literal.args_of("=").and_then(two) else {
                        return false;
                    };
                    let Some(equalities) = unit_equalities(&self.premises) else {
                        return false;
                    };
                    congruent(left, right, &equalities) || congruent(right, left, &equalities)
                }
                _ => false,
            },
            Rule::EqTransitive => {
                let Some((last, hypotheses)) = clause.split_last() else {
                    return false;
                };
                let Some([first, end]) = last.args_of("=").and_then(two) else {
                    return false;
                };
                let Some(equalities) = negated_equalities(hypotheses) else {
                    return false;
                };
                no_premises && (chain(first, end, &equalities) || chain(end, first, &equalities))
            }
            Rule::EqCongruent | Rule::EqCongruentPred => {
                let Some((last, hypotheses)) = clause.split_last() else {
                    return false;
                };
                let Some(equalities) = negated_equalities(hypotheses) else {
                    return false;
                };
                let Some([left, right]) = last.args_of("=").and_then(two) else {
                    return false;
                };
                no_premises
                    && (congruent_pairwise(left, right, &equalities)
                        || congruent_pairwise(right, left, &equalities))
            }
            // Decided three ways in `check`.
            Rule::Resolution => false,
            Rule::Contraction => self.premise().is_some_and(|premise| {
                let mut seen = HashSet::new();
                let distinct: Vec<&Rc<Term>> = premise
                    .iter()
                    .filter(|literal| seen.insert(*literal))
                    .collect();
                clause.len() == distinct.len() && clause.iter().zip(distinct).all(|(a, b)| a == b)
            }),
            Rule::Reordering => self.premise().is_some_and(|premise| {
                let mut left: Vec<&Rc<Term>> = premise.iter().collect();
                let mut right: Vec<&Rc<Term>> = clause.iter().collect();
                left.sort();
                right.sort();
                left == right
            }),
            Rule::Tautology => {
                matches!(clause, [only] if only.is_symbol("true"))
                    && self.premise().is_some_and(|premise| {
                        premise.iter().any(|a| {
                            let (base_a, negative_a) = polarity(a);
                            premise.iter().any(|b| {
                                let (base_b, negative_b) = polarity(b);
                                base_a == base_b && negative_a != negative_b
                            })
                        })
                    })
            }
            Rule::NotNot => {
                no_premises
                    && matches!(clause, [first, second]
                        if first.negated().and_then(|t| t.negated()).and_then(|t| t.negated()) == Some(second))
            }
            Rule::And | Rule::NotOr => {
                let head = if rule == Rule::And { "and" } else { "or" };
                let Some(unit) = self.premise_unit() else {
                    return false;
                };
                let operand = if rule == Rule::And {
                    Some(unit)
                } else {
                    unit.negated()
                };
                let Some(conjuncts) = operand.and_then(|t| t.args_of(head)) else {
                    return false;
                };
                let Some(k) = self.index_arg() else {
                    return false;
                };
                let [literal] = clause else {
                    return false;
                };
                let pick = |conjunct: &Rc<Term>| {
                    if rule == Rule::And {
                        conjunct.clone()
                    } else {
                        not(conjunct)
                    }
                };
                conjuncts.get(k).is_some_and(|c| pick(c) == *literal)
            }
            Rule::Or => self
                .premise_unit()
                .and_then(|unit| unit.args_of("or"))
                .is_some_and(|disjuncts| clause == disjuncts),
            Rule::NotAnd => self
                .premise_unit()
                .and_then(|unit| unit.negated())
                .and_then(|operand| operand.args_of("and"))
                .is_some_and(|conjuncts| {
                    clause.len() == conjuncts.len()
                        && clause.iter().zip(conjuncts).all(|(l, c)| *l == not(c))
                }),
            Rule::Implies
            | Rule::NotImplies1
            | Rule::NotImplies2
            | Rule::Equiv1
            | Rule::Equiv2
            | Rule::NotEquiv1
            | Rule::NotEquiv2
            | Rule::Ite1
            | Rule::Ite2
            | Rule::NotIte1
            | Rule::NotIte2 => {
                let Some(unit) = self.premise_unit() else {
                    return false;
                };
                let (negative, operand) = match unit.negated() {
                    Some(operand) => (true, operand),
                    None => (false, unit),
                };
                let expected = match (rule, negative) {
                    (Rule::Implies, false) => operand
                        .args_of("=>")
                        .and_then(two)
                        .map(|[a, b]| vec![not(a), b.clone()]),
                    (Rule::NotImplies1, true) => operand
                        .args_of("=>")
                        .and_then(two)
                        .map(|[a, _]| vec![a.clone()]),
                    (Rule::NotImplies2, true) => operand
                        .args_of("=>")
                        .and_then(two)
                        .map(|[_, b]| vec![not(b)]),
                    (Rule::Equiv1, false) => equiv(operand).map(|[a, b]| vec![not(a), b.clone()]),
                    (Rule::Equiv2, false) => equiv(operand).map(|[a, b]| vec![a.clone(), not(b)]),
                    (Rule::NotEquiv1, true) => {
                        equiv(operand).map(|[a, b]| vec![a.clone(), b.clone()])
                    }
                    (Rule::NotEquiv2, true) => equiv(operand).map(|[a, b]| vec![not(a), not(b)]),
                    (Rule::Ite1, false) => ite(operand).map(|[c, _, e]| vec![c.clone(), e.clone()]),
                    (Rule::Ite2, false) => ite(operand).map(|[c, t, _]| vec![not(c), t.clone()]),
                    (Rule::NotIte1, true) => ite(operand).map(|[c, _, e]| vec![c.clone(), not(e)]),
                    (Rule::NotIte2, true) => ite(operand).map(|[c, t, _]| vec![not(c), not(t)]),
                    _ => None,
                };
                expected.is_some_and(|expected| clause == expected.as_slice())
            }
            Rule::AndPos
            | Rule::AndNeg
            | Rule::OrPos
            | Rule::OrNeg
            | Rule::ImpliesPos
            | Rule::ImpliesNeg1
            | Rule::ImpliesNeg2
            | Rule::EquivPos1
            | Rule::EquivPos2
            | Rule::EquivNeg1
            | Rule::EquivNeg2
            | Rule::ItePos1
            | Rule::ItePos2
            | Rule::IteNeg1
            | Rule::IteNeg2 => no_premises && self.tautology(rule),
            Rule::False => {
                no_premises
                    && matches!(clause, [only] if only.negated().is_some_and(|t| t.is_symbol("false")))
            }
            Rule::NotSymm => self.premise_unit().is_some_and(|unit| {
                // Equalities are held in canonical order, so the symmetric
                // literal is the same term.
                matches!(clause, [literal] if literal == unit
                    && unit.negated().is_some_and(|eq| eq.args_of("=").is_some()))
            }),
            Rule::LaGeneric => {
                no_premises && la_generic(clause, &self.args, query).unwrap_or(false)
            }
            Rule::LaTautology => {
                no_premises
                    && match clause {
                        [literal] => la_trivial(literal, query).unwrap_or(false),
                        [_, _] => {
                            let one = Rc::new(Term::Number {
                                value: Rational::from_integer(Integer::one()),
                                real: true,
                            });
                            la_generic(clause, &[one.clone(), one], query).unwrap_or(false)
                        }
                        _ => false,
                    }
            }
            Rule::LaDisequality => {
                no_premises
                    && match clause {
                        [eq, first, second] => {
                            eq.args_of("=").and_then(two).is_some_and(|[a, b]| {
                                let le = |x: &Rc<Term>, y: &Rc<Term>| {
                                    not(&Term::app("<=", vec![x.clone(), y.clone()]))
                                };
                                (*first == le(a, b) && *second == le(b, a))
                                    || (*first == le(b, a) && *second == le(a, b))
                            })
                        }
                        _ => false,
                    }
            }
            Rule::LaTotality => {
                no_premises
                    && match clause {
                        [first, second] => {
                            first.args_of("<=").and_then(two).is_some_and(|[a, b]| {
                                *second == Term::app("<=", vec![b.clone(), a.clone()])
                            })
                        }
                        _ => false,
                    }
            }
            Rule::LaMultPos | Rule::LaMultNeg => {
                no_premises && la_mult(clause, rule == Rule::LaMultPos)
            }
            // Dispatched from `judge`, where polynomial work is budgeted.
            Rule::PolySimp | Rule::PolySimpRel => false,
        }
    }

    /// The tautologies with no premise, by their shapes in the Alethe
    /// specification.
    fn tautology(&self, rule: Rule) -> bool {
        let clause = self.clause.as_slice();
        let not = Term::not;
        let Some((first, rest)) = clause.split_first() else {
            return false;
        };
        let k = self.index_arg();
        match rule {
            Rule::AndPos => match (first.negated().and_then(|t| t.args_of("and")), rest) {
                (Some(conjuncts), [literal]) => match k {
                    Some(k) => conjuncts.get(k) == Some(literal),
                    None => false,
                },
                _ => false,
            },
            Rule::AndNeg => first.args_of("and").is_some_and(|conjuncts| {
                rest.len() == conjuncts.len()
                    && rest.iter().zip(conjuncts).all(|(l, c)| *l == not(c))
            }),
            Rule::OrPos => first
                .negated()
                .and_then(|t| t.args_of("or"))
                .is_some_and(|disjuncts| rest == disjuncts),
            Rule::OrNeg => match (first.args_of("or"), rest) {
                (Some(disjuncts), [literal]) => match k {
                    Some(k) => disjuncts.get(k).is_some_and(|d| not(d) == *literal),
                    None => false,
                },
                _ => false,
            },
            Rule::ImpliesPos => first
                .negated()
                .and_then(|t| t.args_of("=>"))
                .and_then(two)
                .is_some_and(|[a, b]| rest == [not(a), b.clone()]),
            Rule::ImpliesNeg1 => first
                .args_of("=>")
                .and_then(two)
                .is_some_and(|[a, _]| rest == [a.clone()]),
            Rule::ImpliesNeg2 => first
                .args_of("=>")
                .and_then(two)
                .is_some_and(|[_, b]| rest == [not(b)]),
            Rule::EquivPos1 => first
                .negated()
                .and_then(|t| equiv(t))
                .is_some_and(|[a, b]| oriented(rest, &[a.clone(), not(b)], &[b.clone(), not(a)])),
            Rule::EquivPos2 => first
                .negated()
                .and_then(|t| equiv(t))
                .is_some_and(|[a, b]| oriented(rest, &[not(a), b.clone()], &[not(b), a.clone()])),
            Rule::EquivNeg1 => equiv(first)
                .is_some_and(|[a, b]| oriented(rest, &[not(a), not(b)], &[not(b), not(a)])),
            Rule::EquivNeg2 => equiv(first).is_some_and(|[a, b]| {
                oriented(rest, &[a.clone(), b.clone()], &[b.clone(), a.clone()])
            }),
            Rule::ItePos1 => first
                .negated()
                .and_then(|t| ite(t))
                .is_some_and(|[c, _, e]| rest == [c.clone(), e.clone()]),
            Rule::ItePos2 => first
                .negated()
                .and_then(|t| ite(t))
                .is_some_and(|[c, t, _]| rest == [not(c), t.clone()]),
            Rule::IteNeg1 => ite(first).is_some_and(|[c, _, e]| rest == [c.clone(), not(e)]),
            Rule::IteNeg2 => ite(first).is_some_and(|[c, t, _]| rest == [not(c), not(t)]),
            _ => false,
        }
    }
}

/// The equalities of premises that are each one unit equality, in order.
fn unit_equalities(premises: &[Rc<[Rc<Term>]>]) -> Option<Vec<(Rc<Term>, Rc<Term>)>> {
    premises
        .iter()
        .map(|premise| match &**premise {
            [unit] => unit
                .args_of("=")
                .and_then(two)
                .map(|[a, b]| (a.clone(), b.clone())),
            _ => None,
        })
        .collect()
}

/// The equalities of literals that are each a negated equality, in order.
fn negated_equalities(literals: &[Rc<Term>]) -> Option<Vec<(Rc<Term>, Rc<Term>)>> {
    literals
        .iter()
        .map(|literal| {
            literal
                .negated()
                .and_then(|equality| equality.args_of("="))
                .and_then(two)
                .map(|[a, b]| (a.clone(), b.clone()))
        })
        .collect()
}

/// The two arguments of a binary application.
fn two(args: &[Rc<Term>]) -> Option<[&Rc<Term>; 2]> {
    match args {
        [a, b] => Some([a, b]),
        _ => None,
    }
}

/// The two sides of an equivalence `(= φ1 φ2)`, in either orientation the
/// canonical order gives them.
fn equiv(term: &Rc<Term>) -> Option<[&Rc<Term>; 2]> {
    term.args_of("=").and_then(two)
}

/// Whether `literals` is `first`, or `second` (the same literals with the
/// equivalence's sides exchanged, which the canonical order of `=` may have
/// swapped).
fn oriented(literals: &[Rc<Term>], first: &[Rc<Term>], second: &[Rc<Term>]) -> bool {
    literals == first || literals == second
}

/// The three operands of `(ite c t e)`.
fn ite(term: &Rc<Term>) -> Option<[&Rc<Term>; 3]> {
    match term.args_of("ite")? {
        [c, t, e] => Some([c, t, e]),
        _ => None,
    }
}

/// Whether the equalities, each used once in order and in either
/// orientation, chain `from` to `to`.
fn chain(from: &Rc<Term>, to: &Rc<Term>, equalities: &[(Rc<Term>, Rc<Term>)]) -> bool {
    if equalities.is_empty() {
        return false;
    }
    let mut at = from.clone();
    for (a, b) in equalities {
        if *a == at {
            at = b.clone();
        } else if *b == at {
            at = a.clone();
        } else {
            return false;
        }
    }
    at == *to
}

/// `cong`: `left` and `right` apply one function symbol, and each pair of
/// arguments is equal or is, in order, the next of `equalities` (in either
/// orientation); every equality is used.
fn congruent(left: &Rc<Term>, right: &Rc<Term>, equalities: &[(Rc<Term>, Rc<Term>)]) -> bool {
    let (Term::App(f, xs), Term::App(g, ys)) = (&**left, &**right) else {
        return false;
    };
    if f != g || xs.len() != ys.len() {
        return false;
    }
    argument_orders(f, xs).iter().any(|xs| {
        argument_orders(g, ys).iter().any(|ys| {
            let mut next = equalities.iter();
            let mut pending = next.next();
            for (x, y) in xs.iter().zip(ys) {
                if let Some((a, b)) = pending {
                    if (a == x && b == y) || (a == y && b == x) {
                        pending = next.next();
                        continue;
                    }
                }
                if x != y {
                    return false;
                }
            }
            pending.is_none()
        })
    })
}

/// The orders in which `cong` may read the arguments of an application of
/// `head`. A binary `=` holds its arguments in canonical order, but a proof's
/// premises follow the source order of the arguments, and an equality is
/// symmetric (the Alethe specification's implicit reordering of equalities),
/// so both orders are read; every other application has its one order.
fn argument_orders(head: &str, args: &[Rc<Term>]) -> Vec<Vec<Rc<Term>>> {
    let mut orders = vec![args.to_vec()];
    if op(head) == Some(Op::Eq) && args.len() == 2 {
        orders.push(args.iter().rev().cloned().collect());
    }
    orders
}

/// `eq_congruent`: each argument pair is equal or is one of the negated
/// equalities, which are the argument pairs in order.
fn congruent_pairwise(
    left: &Rc<Term>,
    right: &Rc<Term>,
    equalities: &[(Rc<Term>, Rc<Term>)],
) -> bool {
    let (Term::App(f, xs), Term::App(g, ys)) = (&**left, &**right) else {
        return false;
    };
    f == g
        && xs.len() == ys.len()
        && xs.len() == equalities.len()
        && xs
            .iter()
            .zip(ys)
            .zip(equalities)
            .all(|((x, y), (a, b))| (a == x && b == y) || (a == y && b == x))
}

/// The most pivot choices [`resolution`] explores for a step that names no
/// pivots; past it the step is not decided.
const MAX_RESOLUTION_NODES: usize = 1 << 12;

/// `clause` resolved with `next` on the pivot `(base, negative)`: the
/// literals of `clause` of that polarity and those of `next` of the other
/// are removed, and the rest kept once each.
fn resolve(
    clause: &[Rc<Term>],
    next: &[Rc<Term>],
    (base, negative): (&Rc<Term>, bool),
) -> Vec<Rc<Term>> {
    let mut kept: Vec<Rc<Term>> = Vec::with_capacity(clause.len() + next.len());
    let keep = |literal: &Rc<Term>, removed: bool, kept: &mut Vec<Rc<Term>>| {
        if !removed && !kept.contains(literal) {
            kept.push(literal.clone());
        }
    };
    for literal in clause {
        let (b, n) = polarity(literal);
        keep(literal, b == base && n == negative, &mut kept);
    }
    for literal in next {
        let (b, n) = polarity(literal);
        keep(literal, b == base && n != negative, &mut kept);
    }
    kept
}

/// `resolution` and `th_resolution` (Alethe rules 6 and 7): the conclusion
/// is, as a set of literals, the clause a chain of binary resolutions over
/// the premises in order derives. Leading negations are merged when a
/// pivot is matched.
///
/// A step whose `:args` name the pivots (`p1 b1 … pk bk`, `bi` `true` when
/// `pi` occurs in the clause so far and `(not pi)` in the next premise) is
/// checked on exactly those. A step with no `:args` holds when some choice
/// of pivots derives its conclusion; the search stops undecided past
/// [`MAX_RESOLUTION_NODES`].
fn resolution(premises: &[Rc<[Rc<Term>]>], conclusion: &[Rc<Term>], args: &[Rc<Term>]) -> Check {
    let Some((first, rest)) = premises.split_first() else {
        return Check::Fails;
    };
    if rest.is_empty() {
        return Check::Fails;
    }
    let wanted = set(conclusion);
    if !args.is_empty() {
        if args.len() != 2 * rest.len() {
            return Check::Fails;
        }
        let mut clause = first.to_vec();
        for (next, pivot) in rest.iter().zip(args.chunks(2)) {
            let [term, flag] = pivot else {
                return Check::Fails;
            };
            let positive = if flag.is_symbol("true") {
                true
            } else if flag.is_symbol("false") {
                false
            } else {
                return Check::Fails;
            };
            let (base, negative) = polarity(term);
            let (left, right) = (negative != !positive, negative == !positive);
            let occurs = |clause: &[Rc<Term>], negative: bool| {
                clause.iter().any(|literal| {
                    let (b, n) = polarity(literal);
                    b == base && n == negative
                })
            };
            if !occurs(&clause, left) || !occurs(next, right) {
                return Check::Fails;
            }
            clause = resolve(&clause, next, (base, left));
        }
        return (set(&clause) == wanted).into();
    }
    let mut budget = MAX_RESOLUTION_NODES;
    search(first.to_vec(), rest, &wanted, &mut budget)
}

fn search(
    clause: Vec<Rc<Term>>,
    rest: &[Rc<[Rc<Term>]>],
    wanted: &HashSet<&Rc<Term>>,
    budget: &mut usize,
) -> Check {
    let Some((next, later)) = rest.split_first() else {
        return (set(&clause) == *wanted).into();
    };
    let mut pivots: Vec<(&Rc<Term>, bool)> = Vec::new();
    for literal in &clause {
        let (base, negative) = polarity(literal);
        let complemented = next.iter().any(|other| {
            let (b, n) = polarity(other);
            b == base && n != negative
        });
        if complemented && !pivots.contains(&(base, negative)) {
            pivots.push((base, negative));
        }
    }
    let mut undecided = false;
    for pivot in pivots {
        if *budget == 0 {
            return Check::Undecided;
        }
        *budget -= 1;
        match search(resolve(&clause, next, pivot), later, wanted, budget) {
            Check::Holds => return Check::Holds,
            Check::Undecided => undecided = true,
            Check::Fails => {}
        }
    }
    if undecided {
        Check::Undecided
    } else {
        Check::Fails
    }
}

/// A linear combination: each atom's coefficient, and the constant.
#[derive(Clone, Debug)]
struct Linear {
    terms: BTreeMap<Rc<Term>, Rational>,
    constant: Rational,
}

impl Default for Linear {
    fn default() -> Self {
        Self::constant(Rational::from_integer(Integer::zero()))
    }
}

impl Linear {
    fn constant(value: Rational) -> Self {
        Self {
            terms: BTreeMap::new(),
            constant: value,
        }
    }

    fn scaled(&self, factor: &Rational) -> Self {
        Self {
            terms: self
                .terms
                .iter()
                .map(|(atom, c)| (atom.clone(), c.mul(factor)))
                .collect(),
            constant: self.constant.mul(factor),
        }
    }

    fn plus(&mut self, other: &Self) {
        for (atom, c) in &other.terms {
            let sum = self
                .terms
                .get(atom)
                .map_or_else(|| c.clone(), |existing| existing.add(c));
            if sum.is_zero() {
                self.terms.remove(atom);
            } else {
                self.terms.insert(atom.clone(), sum);
            }
        }
        self.constant = self.constant.add(&other.constant);
    }

    fn is_constant(&self) -> bool {
        self.terms.is_empty()
    }
}

/// `term` as a linear combination of atoms, or `None` when it is not
/// linear (a product of two non-constant terms, a division by a
/// non-constant).
fn linear(term: &Rc<Term>) -> Option<Linear> {
    match &**term {
        Term::Number { value, .. } => Some(Linear::constant(value.clone())),
        Term::App(head, args) => match (&**head, args.as_slice()) {
            ("+", args) if !args.is_empty() => {
                let mut sum = Linear::default();
                for arg in args {
                    sum.plus(&linear(arg)?);
                }
                Some(sum)
            }
            ("-", [only]) => {
                Some(linear(only)?.scaled(&Rational::from_integer(Integer::one()).neg()))
            }
            ("-", [first, rest @ ..]) => {
                let mut sum = linear(first)?;
                let minus = Rational::from_integer(Integer::one()).neg();
                for arg in rest {
                    sum.plus(&linear(arg)?.scaled(&minus));
                }
                Some(sum)
            }
            ("*", args) if !args.is_empty() => {
                let mut factor = Rational::from_integer(Integer::one());
                let mut variable: Option<Linear> = None;
                for arg in args {
                    let part = linear(arg)?;
                    if part.is_constant() {
                        factor = factor.mul(&part.constant);
                    } else if variable.is_none() {
                        variable = Some(part);
                    } else {
                        return None;
                    }
                }
                Some(match variable {
                    Some(variable) => variable.scaled(&factor),
                    None => Linear::constant(factor),
                })
            }
            ("/", [numerator, denominator]) => {
                let d = linear(denominator)?;
                if !d.is_constant() {
                    return None;
                }
                let inverse = Rational::from_integer(Integer::one()).div(&d.constant)?;
                Some(linear(numerator)?.scaled(&inverse))
            }
            _ => Some(atom(term)),
        },
        _ => Some(atom(term)),
    }
}

fn atom(term: &Rc<Term>) -> Linear {
    let mut terms = BTreeMap::new();
    terms.insert(term.clone(), Rational::from_integer(Integer::one()));
    Linear {
        terms,
        constant: Rational::from_integer(Integer::zero()),
    }
}

/// The relation of a normalized inequality `s ⋈ d`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Relation {
    Greater,
    GreaterEqual,
    Equal,
}

/// Steps 1 to 3 of `la_generic` over one literal: the negation of the
/// literal as `s ⋈ d`. `None` for a literal that is not a linear
/// (in)equality, or a positive equality, which `la_generic` never carries.
fn normalized(literal: &Rc<Term>) -> Option<(Linear, Relation)> {
    let (relation, left, right) = match literal.negated() {
        // Step 2: `¬(s1 ⋈ s2)` is `s1 ⋈ s2`, `<` and `<=` turned around.
        Some(atom) => {
            let (head, [s1, s2]) = comparison(atom)?;
            match head {
                Op::Lt => (Relation::Greater, s2, s1),
                Op::Le => (Relation::GreaterEqual, s2, s1),
                Op::Gt => (Relation::Greater, s1, s2),
                Op::Ge => (Relation::GreaterEqual, s1, s2),
                Op::Eq => (Relation::Equal, s1, s2),
                _ => return None,
            }
        }
        // Step 1: a positive literal is negated.
        None => {
            let (head, [s1, s2]) = comparison(literal)?;
            match head {
                Op::Gt => (Relation::GreaterEqual, s2, s1),
                Op::Ge => (Relation::Greater, s2, s1),
                Op::Lt => (Relation::GreaterEqual, s1, s2),
                Op::Le => (Relation::Greater, s1, s2),
                _ => return None,
            }
        }
    };
    // Step 3: `s ⋈ d`, `s` the left side's atoms less the right side's and
    // `d` the right constant less the left.
    let mut s = linear(left)?;
    let right = linear(right)?;
    s.plus(&right.scaled(&Rational::from_integer(Integer::one()).neg()));
    let d = s.constant.neg();
    s.constant = Rational::from_integer(Integer::zero());
    Some((
        Linear {
            terms: s.terms,
            constant: d,
        },
        relation,
    ))
}

/// A comparison's operator and its two operands.
fn comparison(term: &Rc<Term>) -> Option<(Op, [&Rc<Term>; 2])> {
    match &**term {
        Term::App(head, args) => {
            let [a, b] = args.as_slice() else {
                return None;
            };
            let op = op(head)?;
            matches!(op, Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Eq).then_some((op, [a, b]))
        }
        _ => None,
    }
}

/// Whether `s` takes only integer values: every atom an integer-sorted
/// constant of `query` with an integer coefficient, so integer strengthening
/// (Alethe `la_generic` step 4) is sound for it.
fn integer_sorted(s: &Linear, query: &Query) -> bool {
    s.terms.iter().all(|(atom, coefficient)| {
        coefficient.is_integer()
            && match &**atom {
                Term::Symbol(name) => query.integers.contains(name),
                _ => false,
            }
    })
}

/// `⌊d⌋`.
fn floor(d: &Rational) -> Integer {
    let Ok(denominator) = quire_exact::NonZeroInteger::try_from(d.denominator()) else {
        return Integer::zero();
    };
    d.numerator().div_mod_floor(denominator).0
}

/// The `la_generic` check of `clause` with the coefficients `args`.
/// `None` when a literal or coefficient cannot be read.
fn la_generic(clause: &[Rc<Term>], args: &[Rc<Term>], query: &Query) -> Option<bool> {
    if clause.len() != args.len() || clause.is_empty() {
        return Some(false);
    }
    let one = Rational::from_integer(Integer::one());
    let mut sum = Linear::default();
    let mut relations = Vec::with_capacity(clause.len());
    for (literal, coefficient) in clause.iter().zip(args) {
        let Term::Number { value: a, .. } = &**coefficient else {
            return None;
        };
        let (mut s, mut relation) = normalized(literal)?;
        // Step 4: integer strengthening.
        if integer_sorted(&s, query) {
            let d = s.constant.clone();
            match relation {
                Relation::Greater => {
                    relation = Relation::GreaterEqual;
                    s.constant = if d.is_integer() {
                        d.add(&one)
                    } else {
                        Rational::from_integer(floor(&d)).add(&one)
                    };
                }
                Relation::GreaterEqual => {
                    if !d.is_integer() {
                        s.constant = Rational::from_integer(floor(&d)).add(&one);
                    }
                }
                Relation::Equal => {}
            }
        }
        // Step 5: scale by `a`, or `|a|` for an inequality.
        let factor = if relation == Relation::Equal || !a.numerator().is_negative() {
            a.clone()
        } else {
            a.neg()
        };
        let scaled = s.scaled(&factor);
        let mut terms = scaled.terms;
        terms.retain(|_, c| !c.is_zero());
        let mut part = Linear {
            terms,
            constant: Rational::from_integer(Integer::zero()),
        };
        // The constant is the right-hand side; keep it apart from the
        // atoms' sum.
        part.constant = scaled.constant;
        sum.plus(&part);
        relations.push(relation);
    }
    if !sum.terms.values().all(Rational::is_zero) {
        return Some(false);
    }
    let relation = if relations.iter().all(|r| *r == relations[0]) {
        relations[0]
    } else if relations.contains(&Relation::Greater) {
        Relation::Greater
    } else {
        Relation::GreaterEqual
    };
    let zero = Rational::from_integer(Integer::zero());
    Some(match relation {
        Relation::Greater => sum.constant >= zero,
        Relation::GreaterEqual | Relation::Equal => sum.constant > zero,
    })
}

/// `la_tautology`'s unit form: after steps 1 to 3 the negated literal is
/// `0 ⋈ d` and is false.
fn la_trivial(literal: &Rc<Term>, _query: &Query) -> Option<bool> {
    let (s, relation) = normalized(literal)?;
    if !s.terms.values().all(Rational::is_zero) {
        return Some(false);
    }
    let zero = Rational::from_integer(Integer::zero());
    Some(match relation {
        Relation::Greater => s.constant >= zero,
        Relation::GreaterEqual => s.constant > zero,
        Relation::Equal => s.constant != zero,
    })
}

/// `la_mult_pos` and `la_mult_neg` over one literal: `(=> (and (> t1 0) R)
/// R')` with `R` a comparison of `t2` and `t3` (or a negated equality) and
/// `R'` the same comparison of `t1*t2` and `t1*t3`, inverted for
/// `la_mult_neg`.
fn la_mult(clause: &[Rc<Term>], positive: bool) -> bool {
    let [literal] = clause else {
        return false;
    };
    let Some([premise, conclusion]) = literal.args_of("=>").and_then(two) else {
        return false;
    };
    let Some([sign, relation]) = premise.args_of("and").and_then(two) else {
        return false;
    };
    let Some((sign_head, [t1, z])) = comparison(sign) else {
        return false;
    };
    let expected_sign = if positive { Op::Gt } else { Op::Lt };
    if sign_head != expected_sign || !matches!(&**z, Term::Number { value, .. } if value.is_zero())
    {
        return false;
    }
    let product = |t: &Rc<Term>| Term::app("*", vec![t1.clone(), t.clone()]);
    if let (Some(inner), Some(outer)) = (relation.negated(), conclusion.negated()) {
        let (Some((Op::Eq, [t2, t3])), Some((Op::Eq, [p2, p3]))) =
            (comparison(inner), comparison(outer))
        else {
            return false;
        };
        return (*p2 == product(t2) && *p3 == product(t3))
            || (*p2 == product(t3) && *p3 == product(t2));
    }
    let (Some((head, [t2, t3])), Some((outer_head, [p2, p3]))) =
        (comparison(relation), comparison(conclusion))
    else {
        return false;
    };
    let inverted = match (positive, head) {
        (true, head) => head,
        (false, Op::Lt) => Op::Gt,
        (false, Op::Le) => Op::Ge,
        (false, Op::Gt) => Op::Lt,
        (false, Op::Ge) => Op::Le,
        (false, head) => head,
    };
    if outer_head != inverted {
        return false;
    }
    if head == Op::Eq {
        (*p2 == product(t2) && *p3 == product(t3)) || (*p2 == product(t3) && *p3 == product(t2))
    } else {
        *p2 == product(t2) && *p3 == product(t3)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::{read_proof, read_query, ProofOutcome, Query, Reading};

    const A: &str = "(> x 0)";

    fn query() -> Query {
        read_query(
            b"(set-logic QF_LIA)\n(declare-const x Int)\n(declare-const y Int)\n\
              (assert (> x 0))\n(assert (< x 1))\n(assert (= y (+ x 1)))\n(check-sat)\n",
        )
        .expect("the query reads")
    }

    /// The two admitted cvc5 polynomial rules check the captured row-1
    /// normalization, while unsupported arithmetic remains undecided.
    #[trace("TC-893", "FR-314-AC-18", "FR-314-AC-19", "FR-314-AC-20")]
    #[test]
    fn polynomial_rules_check_int_normalization_only() {
        let q = query();
        let t28 = "(step t28 (cl (= (* 1 (- (+ 1 x) y)) (* 1 (- x (+ (- 1) y))))) :rule poly_simp)";
        let t29 =
            "(step t29 (cl (= (= (+ 1 x) y) (= x (+ (- 1) y)))) :rule poly_simp_rel :premises (t28))";
        let checked = format!("{t28}{t29}");
        let Reading::Read(proof) = read_proof(checked.as_bytes(), &q, &mut |_| true) else {
            panic!("captured polynomial proof reads");
        };
        assert_eq!(proof.invalid_step(&q, &mut |_| true), Ok(None));

        let nonlinear = "(step t0 (cl (= (* x y) (* y x))) :rule poly_simp)";
        let Reading::Read(proof) = read_proof(nonlinear.as_bytes(), &q, &mut |_| true) else {
            panic!("nonlinear polynomial step reads");
        };
        assert_eq!(proof.invalid_step(&q, &mut |_| true), Ok(None));

        let negative_equality = "(step p0 (cl (= (+ x (- y)) (+ x (- y)))) :rule poly_simp)\
             (step p1 (cl (= (= x y) (= (- x) (- y)))) :rule poly_simp_rel :premises (p0))";
        let Reading::Read(proof) = read_proof(negative_equality.as_bytes(), &q, &mut |_| true)
        else {
            panic!("negative equality relation reads");
        };
        assert_eq!(
            proof.outcome(&q, &mut |_| true),
            Ok(ProofOutcome::Unverifiable { step: 1 })
        );

        let negative_same_order = "(step p0 (cl (= (+ x (- y)) (+ x (- y)))) :rule poly_simp)\
             (step p1 (cl (= (< x y) (< (- x) (- y)))) :rule poly_simp_rel :premises (p0))";
        let Reading::Read(proof) = read_proof(negative_same_order.as_bytes(), &q, &mut |_| true)
        else {
            panic!("same-order negative relation reads");
        };
        assert_eq!(proof.invalid_step(&q, &mut |_| true), Ok(Some(1)));

        let negative_reversed_order = "(step p0 (cl (= (+ x (- y)) (+ x (- y)))) :rule poly_simp)\
             (step p1 (cl (= (< x y) (> (- x) (- y)))) :rule poly_simp_rel :premises (p0))";
        let Reading::Read(proof) =
            read_proof(negative_reversed_order.as_bytes(), &q, &mut |_| true)
        else {
            panic!("reversed negative relation reads");
        };
        assert_eq!(
            proof.outcome(&q, &mut |_| true),
            Ok(ProofOutcome::Unverifiable { step: 1 })
        );

        let false_step = "(step t0 (cl (= (* 2 x) x)) :rule poly_simp)";
        let Reading::Read(proof) = read_proof(false_step.as_bytes(), &q, &mut |_| true) else {
            panic!("false polynomial step reads");
        };
        assert_eq!(proof.invalid_step(&q, &mut |_| true), Ok(Some(0)));

        let real_query = read_query(
            b"(set-logic QF_LRA)\n(declare-const r Real)\n(assert (= r r))\n(check-sat)\n",
        )
        .expect("real query reads");
        let real_step = "(step t0 (cl (= (* 1.0 r) r)) :rule poly_simp)";
        let Reading::Read(proof) = read_proof(real_step.as_bytes(), &real_query, &mut |_| true)
        else {
            panic!("unsupported real step reads");
        };
        assert_eq!(
            proof.outcome(&real_query, &mut |_| true),
            Ok(ProofOutcome::Unverifiable { step: 0 })
        );

        let Reading::Read(proof) = read_proof(t28.as_bytes(), &q, &mut |_| true) else {
            panic!("captured polynomial step reads");
        };
        assert_eq!(proof.invalid_step(&q, &mut |_| false), Err(()));
    }

    /// What FR-314's proof rules say of `proof`: `Err("malformed")`,
    /// `Err("empty")`, `Err("invalid N")`, `Err("short N")`, or the outcome.
    fn verdict(proof: &str) -> Result<ProofOutcome, String> {
        let query = query();
        let read = read_proof(proof.as_bytes(), &query, &mut |_| true);
        let Reading::Read(proof) = read else {
            return Err("malformed".to_owned());
        };
        if proof.len() == 0 {
            return Err("empty".to_owned());
        }
        if let Some(index) = proof.invalid_step(&query, &mut |_| true).expect("no stop") {
            return Err(format!("invalid {index}"));
        }
        if let Some(index) = proof.not_refutation() {
            return Err(format!("short {index}"));
        }
        Ok(proof.outcome(&query, &mut |_| true).expect("no stop"))
    }

    /// FR-314's command numbering and its subproof, malformed and
    /// not-refutation vectors, over a refinement part's proof reader.
    #[trace("TC-893")]
    #[test]
    fn commands_number_in_flattened_text_order() {
        let premise_later = format!(
            "(assume a0 {A})(anchor :step t2)(assume t2.a0 {A})\
             (step t2.t1 (cl) :rule resolution :premises (t2.t2))\
             (step t2.t2 (cl {A}) :rule hole)\
             (step t2 (cl (not {A})) :rule subproof :discharge (t2.a0))"
        );
        assert_eq!(verdict(&premise_later), Err("invalid 3".to_owned()));
        let renamed = premise_later
            .replace("t2.a0", "t0.t8")
            .replace("t2.t1", "t0.t7")
            .replace("t2.t2", "t0.t1")
            .replace("a0 ", "t9 ")
            .replace("t2", "t0");
        assert_eq!(verdict(&renamed), Err("invalid 3".to_owned()));
        assert_eq!(
            verdict(&format!(
                "(assume a0 {A})(anchor :step t2)(assume t2.a0 {A})\
                 (step t2.t1 (cl {A}) :rule hole)\
                 (step t2 (cl (not {A}) {A}) :rule subproof :discharge (t2.a0))"
            )),
            Err("short 4".to_owned())
        );
        assert_eq!(
            verdict(&format!(
                "(assume a0 {A})(anchor :step t2)(assume t2.a0 {A})\
                 (step t2.t1 (cl) :rule hole)\
                 (step t2 (cl (not {A})) :rule subproof :discharge (t2.a0))\
                 (step t3 (cl) :rule resolution :premises (a0 t2))"
            )),
            Ok(ProofOutcome::Unverifiable { step: 3 })
        );
        assert_eq!(verdict(""), Err("empty".to_owned()));
        for malformed in [
            format!("(assume a0 {A}"),
            format!("(assume a0 {A})(assume a0 {A})"),
            format!("(assume a0 {A})(anchor :step t2)(assume t2.a0 {A})"),
        ] {
            assert_eq!(
                verdict(&malformed),
                Err("malformed".to_owned()),
                "{malformed}"
            );
        }
        assert!(matches!(
            read_proof(&[0xff], &query(), &mut |_| true),
            Reading::Malformed
        ));
    }

    /// A refutation by checked rules refutes; each rule checks its own
    /// conclusion, so a wrong coefficient, a pivot that is not one, or an
    /// assumption that is no assertion is invalid.
    #[trace("TC-893")]
    #[test]
    fn checked_rules_check_their_conclusions() {
        let refutation = "(assume a0 (> x 0))(assume a1 (< x 1))\
             (step t0 (cl (not (> x 0)) (not (< x 1))) :rule la_generic :args (1 1))\
             (step t1 (cl) :rule resolution :premises (t0 a0 a1))";
        assert_eq!(verdict(refutation), Ok(ProofOutcome::Refutes));
        // Coefficients that leave an atom uncancelled prove nothing.
        let wrong = refutation.replace(":args (1 1)", ":args (1 2)");
        assert_eq!(verdict(&wrong), Err("invalid 2".to_owned()));
        let false_pivot = "(assume a0 (> x 0))(assume a1 (< x 1))\
             (step t1 (cl) :rule resolution :premises (a0 a1))";
        assert_eq!(verdict(false_pivot), Err("invalid 2".to_owned()));
        let unasserted = "(assume a0 (> x 5))(step t1 (cl) :rule hole :premises (a0))";
        assert_eq!(verdict(unasserted), Err("invalid 0".to_owned()));
        // An assertion's equality is assumed in either orientation.
        assert_eq!(
            verdict("(assume a0 (= (+ x 1) y))(step t1 (cl) :rule hole :premises (a0))"),
            Ok(ProofOutcome::Unverifiable { step: 1 })
        );
        let tautology = "(assume a0 (> x 0))\
             (step t0 (cl (> x 0) (not (> x 0))) :rule hole)\
             (step t1 (cl true) :rule tautology :premises (t0))\
             (step t2 (cl false) :rule tautology :premises (a0))";
        assert_eq!(verdict(tautology), Err("invalid 3".to_owned()));
    }

    /// `resolution` checks its conclusion exactly: named pivots are
    /// matched as given, unnamed ones are searched for, and a clause other
    /// than the one the chain derives is invalid.
    #[trace("TC-893")]
    #[test]
    fn resolution_matches_its_pivots_exactly() {
        let refutation = "(assume a0 (> x 0))(assume a1 (< x 1))\
             (step t0 (cl (not (> x 0)) (not (< x 1))) :rule la_generic :args (1 1))\
             (step t1 (cl) :rule resolution :premises (t0 a0 a1) \
             :args ((> x 0) false (< x 1) false))";
        assert_eq!(verdict(refutation), Ok(ProofOutcome::Refutes));
        let wrong_flag = refutation.replace("(> x 0) false", "(> x 0) true");
        assert_eq!(verdict(&wrong_flag), Err("invalid 3".to_owned()));
        let short_args = refutation.replace(" (< x 1) false", "");
        assert_eq!(verdict(&short_args), Err("invalid 3".to_owned()));
        let premises = "(step h0 (cl p q) :rule hole)(step h1 (cl (not p) (not q)) :rule hole)";
        for (conclusion, expected) in [
            ("(cl q (not q))", "short 2"),
            ("(cl p (not p))", "short 2"),
            ("(cl p (not q))", "invalid 2"),
            ("(cl q (not q) r)", "invalid 2"),
        ] {
            let proof =
                format!("{premises}(step t0 {conclusion} :rule resolution :premises (h0 h1))");
            assert_eq!(verdict(&proof), Err(expected.to_owned()), "{conclusion}");
        }
    }

    /// `and`, `not_or`, `and_pos` and `or_neg` take their index argument.
    #[trace("TC-893")]
    #[test]
    fn conjunct_rules_take_their_index() {
        let premise = "(step h0 (cl (and p q)) :rule hole)";
        assert_eq!(
            verdict(&format!(
                "{premise}(step t0 (cl q) :rule and :premises (h0) :args (1))"
            )),
            Err("short 1".to_owned())
        );
        for args in ["", ":args (0)", ":args (2)", ":args (1 1)"] {
            assert_eq!(
                verdict(&format!(
                    "{premise}(step t0 (cl q) :rule and :premises (h0) {args})"
                )),
                Err("invalid 1".to_owned()),
                "{args}"
            );
        }
        assert_eq!(
            verdict("(step t0 (cl (not (and p q)) q) :rule and_pos)"),
            Err("invalid 0".to_owned())
        );
    }

    /// A `define-fun` with parameters is expanded where it is applied.
    #[trace("TC-893")]
    #[test]
    fn a_defined_function_expands_at_its_application() {
        let proof = "(define-fun f ((z Int)) Bool (> z 0))\
             (assume a0 (f x))(assume a1 (< x 1))\
             (step t0 (cl (not (f x)) (not (< x 1))) :rule la_generic :args (1 1))\
             (step t1 (cl) :rule resolution :premises (t0 a0 a1))";
        assert_eq!(verdict(proof), Ok(ProofOutcome::Refutes));
        let wrong = proof.replace("(assume a0 (f x))", "(assume a0 (f y))");
        assert_eq!(verdict(&wrong), Err("invalid 1".to_owned()));
    }

    /// An anchor's context: `refl` and `cong` under `subst(Γ)`, and `bind`
    /// closing the subproof (the Alethe specification's Example 4).
    #[trace("TC-893")]
    #[test]
    fn an_anchor_context_renames_a_bound_variable() {
        let renaming = "(anchor :step t5 :args ((y Int) (:= (x Int) y)))\
             (step t3 (cl (= x y)) :rule refl)\
             (step t4 (cl (= (> x 0) (> y 0))) :rule cong :premises (t3))\
             (step t5 (cl (= (forall ((x Int)) (> x 0)) (forall ((y Int)) (> y 0)))) :rule bind)";
        assert_eq!(verdict(renaming), Err("short 3".to_owned()));
        // `refl` is checked under the context's substitution.
        let unrenamed = renaming.replace("(cl (= x y)) :rule refl", "(cl (= x x)) :rule refl");
        assert_eq!(verdict(&unrenamed), Err("invalid 1".to_owned()));
        // The conclusion's equality is oriented in a non-empty context.
        let reversed = renaming.replace("(cl (= x y)) :rule refl", "(cl (= y x)) :rule refl");
        assert_eq!(verdict(&reversed), Err("invalid 1".to_owned()));
        // `bind` binds the variables the anchor fixes and maps.
        let other = renaming.replace("(forall ((y Int)) (> y 0))", "(forall ((z Int)) (> z 0))");
        assert_eq!(verdict(&other), Err("invalid 3".to_owned()));
        let free = renaming.replace("(forall ((x Int)) (> x 0))", "(forall ((x Int)) (> y 0))");
        assert_eq!(verdict(&free), Err("invalid 3".to_owned()));
        // A rule that needs an empty context is not decided inside one.
        let context_free = format!(
            "{}(step t6 (cl) :rule hole)",
            renaming.replace(
                "(step t4 (cl (= (> x 0) (> y 0))) :rule cong :premises (t3))",
                "(step t2 (cl (not (not (not true))) true) :rule not_not)\
                 (step t4 (cl (= (> x 0) (> y 0))) :rule cong :premises (t3))"
            )
        );
        assert_eq!(
            verdict(&context_free),
            Ok(ProofOutcome::Unverifiable { step: 2 })
        );
    }

    /// The numerals' forgery: `0`, `1` and `2` read as Booleans through
    /// `equiv_neg1` and `equiv_neg2`, with `la_tautology`'s disequalities,
    /// would refute any query from no assumption.
    pub(super) fn numeral_forgery() -> String {
        let mut proof = String::new();
        for (a, b, id) in [("0", "1", "p"), ("0", "2", "q"), ("1", "2", "r")] {
            proof.push_str(&format!(
                "(step {id}d (cl (not (= {a} {b}))) :rule la_tautology)\n\
                 (step {id}n (cl (= {a} {b}) (not {a}) (not {b})) :rule equiv_neg1)\n\
                 (step {id}p (cl (= {a} {b}) {a} {b}) :rule equiv_neg2)\n\
                 (step {id}1 (cl (not {a}) (not {b})) :rule resolution :premises ({id}n {id}d))\n\
                 (step {id}2 (cl {a} {b}) :rule resolution :premises ({id}p {id}d))\n"
            ));
        }
        proof.push_str(
            "(step s1 (cl 0 (not 2)) :rule resolution :premises (p2 r1))\n\
             (step s2 (cl 0) :rule resolution :premises (q2 s1))\n\
             (step s3 (cl 2 (not 0)) :rule resolution :premises (r2 p1))\n\
             (step s4 (cl (not 0)) :rule resolution :premises (q1 s3))\n\
             (step s5 (cl) :rule resolution :premises (s2 s4))\n",
        );
        proof
    }

    /// SR-1788001 FND-001: a literal or formula operand that is not
    /// `Bool`-sorted is `ProofStepInvalid`, so the numerals' forgery fails at
    /// its first `equiv_neg1`, and the other propositional shapes over an
    /// `Int` term fail too.
    #[trace("TC-893")]
    #[test]
    fn an_ill_sorted_literal_is_invalid() {
        assert_eq!(verdict(&numeral_forgery()), Err("invalid 1".to_owned()));
        for step in [
            "(step t0 (cl (= 0 1) (not 0) (not 1)) :rule equiv_neg1)",
            "(step t0 (cl (= 0 1) 0 1) :rule equiv_neg2)",
            "(step t0 (cl (not (not (not x))) x) :rule not_not)",
            "(step t0 (cl (not (ite (> x 0) x y)) (> x 0) y) :rule ite1)",
            "(step t0 (cl (not (and x y)) x) :rule and :args (0))",
            "(step t0 (cl (not (= x y)) (not x) y) :rule eq_congruent_pred)",
        ] {
            assert_eq!(verdict(step), Err("invalid 0".to_owned()), "{step}");
        }
    }

    /// SR-1788001 FND-002: integer strengthening needs integral
    /// coefficients, so `(* (/ 1 2) x) > 0` is not strengthened to `≥ 1`
    /// and the refutation of the satisfiable `x > 0 ∧ x < 2` fails at it.
    #[trace("TC-893")]
    #[test]
    fn strengthening_needs_integral_coefficients() {
        let query = read_query(
            b"(set-logic QF_LIA)\n(declare-const x Int)\n\
              (assert (> x 0))\n(assert (< x 2))\n(check-sat)\n",
        )
        .expect("the query reads");
        let invalid = |proof: &str| {
            let Reading::Read(proof) = read_proof(proof.as_bytes(), &query, &mut |_| true) else {
                panic!("the proof reads");
            };
            proof.invalid_step(&query, &mut |_| true).expect("no stop")
        };
        assert_eq!(
            invalid(
                "(assume a0 (> x 0))\n(assume a1 (< x 2))\n\
                 (step t0 (cl (not (> x 0)) (> (* (/ 1 2) x) 0)) :rule la_generic :args (1 2))\n\
                 (step t1 (cl (not (> (* (/ 1 2) x) 0)) (not (< x 2))) :rule la_generic :args (2 1))\n\
                 (step t2 (cl) :rule resolution :premises (a0 t0 t1 a1))\n"
            ),
            Some(3)
        );
        assert_eq!(
            invalid("(step t0 (cl (not (> (* (/ 1 2) x) 0)) (not (< x 2))) :rule la_tautology)\n"),
            Some(0)
        );
        // An integral coefficient is still strengthened.
        assert_eq!(
            invalid(
                "(step t0 (cl (not (> (* 2 x) 0)) (not (< x 1))) :rule la_generic :args (1 2))\n"
            ),
            None
        );
    }

    /// `cong` over a binary `=`: the equality holds its arguments in
    /// canonical order, but cvc5's premises follow the source order of the
    /// arguments, so a `cong` whose application is `=` is checked in either
    /// orientation of each side (the Alethe implicit reordering of
    /// equalities), with a reflexive argument omitted from the premises, and
    /// a premise in either orientation.
    #[trace("TC-893")]
    #[test]
    fn cong_over_an_equality_reads_its_arguments_in_source_order() {
        let premises = "(step t0 (cl (= y (+ y 1))) :rule hole)\
             (step t1 (cl (= x (+ x 1))) :rule hole)\
             (step t2 (cl (= y (+ x 1))) :rule hole)";
        let conclusion = |text: &str, ids: &str| {
            format!("{premises}(step t3 (cl {text}) :rule cong :premises ({ids}))")
        };
        let accepted = |proof: String| verdict(&proof) == Err("short 3".to_owned());
        // Both arguments differ, the sides' canonical orders disagree.
        assert!(accepted(conclusion(
            "(= (= y x) (= (+ y 1) (+ x 1)))",
            "t0 t1"
        )));
        // The conclusion's orientation and each premise's do not matter.
        assert!(accepted(conclusion(
            "(= (= (+ x 1) (+ y 1)) (= x y))",
            "t1 t0"
        )));
        // A reflexive argument is omitted from the premises.
        assert!(accepted(conclusion("(= (= y y) (= (+ y 1) y))", "t0")));
        // The premises must still match the arguments in some orientation.
        assert!(!accepted(conclusion(
            "(= (= y x) (= (+ y 1) (+ x 1)))",
            "t0 t2"
        )));
        assert!(!accepted(conclusion(
            "(= (= y x) (= (+ y 1) (+ x 1)))",
            "t0"
        )));
    }

    /// cvc5's shape of a `cong` over a shared reflexive term: the premise is
    /// the equality of two `define-fun` proxies, the other premise is the
    /// `refl` of a `:named` reflexive term, and the conclusion is a proxy.
    #[trace("TC-893")]
    #[test]
    fn cong_accepts_proxy_conclusion_with_a_named_refl_premise() {
        let proof = "(define-fun @p_67 () Int y)(define-fun @p_45 () Int (+ y 1))\
             (define-fun @p_71 () Bool (= (= @p_67 x) (= @p_45 x)))\
             (step t34 (cl (= @p_67 @p_45)) :rule hole)\
             (step t35 (cl (! (= x x) :named @p_65)) :rule refl)\
             (step t36 (cl @p_71) :rule cong :premises (t34 t35))";
        assert_eq!(verdict(proof), Err("short 5".to_owned()));
        let wrong = proof.replace("(= @p_45 x)))", "(= @p_45 y)))");
        assert_eq!(verdict(&wrong), Err("invalid 5".to_owned()));
    }

    /// Only `=` is symmetric: `cong` over any other head reads its arguments
    /// in source order, so swapped premises or a swapped side are refused,
    /// while an n-ary application and an `=` of more than two operands are
    /// read argument by argument with a reflexive operand omitted.
    #[trace("TC-893")]
    #[test]
    fn cong_reorders_only_the_arguments_of_an_equality() {
        let premises = "(step t0 (cl (= y (+ y 1))) :rule hole)\
             (step t1 (cl (= x (+ x 1))) :rule hole)";
        let accepted = |text: &str, ids: &str| {
            let proof = format!("{premises}(step t2 (cl (= {text})) :rule cong :premises ({ids}))");
            match verdict(&proof) {
                Err(reason) if reason == "short 2" => true,
                Err(reason) if reason == "invalid 2" => false,
                other => panic!("{text} {ids}: {other:?}"),
            }
        };
        for head in ["-", "<", "+"] {
            let same = format!("({head} y x) ({head} (+ y 1) (+ x 1))");
            assert!(accepted(&same, "t0 t1"), "{head}");
            assert!(!accepted(&same, "t1 t0"), "{head}");
            let swapped = format!("({head} x y) ({head} (+ y 1) (+ x 1))");
            assert!(!accepted(&swapped, "t0 t1"), "{head}");
            assert!(!accepted(&swapped, "t1 t0"), "{head}");
        }
        // N-ary: the reflexive operand is omitted from the premises.
        assert!(accepted("(+ y 1 x) (+ (+ y 1) 1 (+ x 1))", "t0 t1"));
        assert!(!accepted("(+ y 1 x) (+ (+ y 1) 1 (+ x 1))", "t1 t0"));
        // An `=` of three operands is not reordered.
        assert!(accepted("(= y x 1) (= (+ y 1) (+ x 1) 1)", "t0 t1"));
        assert!(!accepted("(= y x 1) (= (+ y 1) (+ x 1) 1)", "t1 t0"));
    }
}

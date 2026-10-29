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
//! Terms are compared syntactically after the `:named` and `define-fun`
//! proxies are expanded, and modulo the orientation of equalities (the
//! Alethe specification's "implicit reordering of equalities"): a binary
//! `=` is held with its arguments in a canonical order.
//!
//! Not checked yet, and so `Unverifiable` wherever a proof uses them
//! (FR-314's checked set names them; each is remaining work): a subproof
//! whose anchor carries a context (`:args`), every command inside it and the
//! `bind` rule that closes it, and a `define-fun` with parameters.

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
    /// A form this checker reads only syntactically (a binder, an indexed
    /// identifier).
    Other(Rc<str>),
}

impl Term {
    fn app(head: &str, args: Vec<Rc<Term>>) -> Rc<Term> {
        let mut args = args;
        if head == "=" && args.len() == 2 && args[0] > args[1] {
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

/// The proxies a query or proof defines: `:named` annotations and
/// `define-fun` commands without parameters.
#[derive(Default)]
struct Names {
    terms: HashMap<Rc<str>, Rc<Term>>,
}

impl Names {
    /// Read `sexpr` as a term, recording each `:named` proxy it introduces.
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
                    "forall" | "exists" | "let" | "lambda" | "choice" | "_" | "as" => {
                        Some(Rc::new(Term::Other(render(sexpr).into())))
                    }
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
                        Some(Term::app(head, args))
                    }
                }
            }
        }
    }

    fn atom(&self, atom: &Rc<str>) -> Rc<Term> {
        if let Some(term) = self.terms.get(atom) {
            return term.clone();
        }
        if let Some(value) = number(atom) {
            return Rc::new(value);
        }
        Rc::new(Term::Symbol(atom.clone()))
    }
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
    /// The query's `:named` proxies, which stay valid in the proof.
    names: HashMap<Rc<str>, Rc<Term>>,
}

/// Read `bytes` as an SMT-LIB script: its `declare-const` commands and its
/// assertions. `None` when it does not parse.
pub(crate) fn read_query(bytes: &[u8]) -> Option<Query> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut names = Names::default();
    let mut assertions = HashSet::new();
    let mut integers = HashSet::new();
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
        names: names.terms,
    })
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
    /// Whether the command lies in a subproof whose anchor carries a
    /// context, which this checker does not read.
    in_context: bool,
}

#[derive(Debug)]
enum CommandKind {
    /// `(assume id φ)`; `local` inside a subproof.
    Assume { term: Rc<Term>, local: bool },
    /// `(step id (cl …) :rule r …)`.
    Step(Step),
    /// `(anchor :step id [:args …])`; `context` when it carries `:args`.
    Anchor { context: bool },
    /// `(define-fun …)`; `parameters` when it takes any.
    Define { parameters: bool },
}

#[derive(Debug)]
struct Step {
    clause: Vec<Rc<Term>>,
    rule: Rc<str>,
    /// Each premise's conclusion clause.
    premises: Vec<Rc<[Rc<Term>]>>,
    args: Vec<Rc<Term>>,
    /// For a step that closes a subproof: the subproof's assumptions, in
    /// the order of `:discharge`, whether `:discharge` names exactly the
    /// subproof's assumptions, and the clause of its last command.
    closes: Option<Subproof>,
}

#[derive(Debug)]
struct Subproof {
    discharged: Vec<Rc<Term>>,
    discharges_exactly: bool,
    last: Option<Rc<[Rc<Term>]>>,
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
    context: bool,
    /// The ids defined in this subproof, removed from view when it closes.
    defined: Vec<Rc<str>>,
    assumptions: Vec<(Rc<str>, Rc<Term>)>,
    last: Option<Rc<[Rc<Term>]>>,
}

/// Read `bytes` as an Alethe proof of `query`, numbering its commands.
/// `charge` is called with the edge count of each command (one, plus one per
/// `:premises` and `:discharge` entry) before the command is read, and stops
/// the read when it returns `false`.
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
    };
    let mut defined: HashSet<Rc<str>> = HashSet::new();
    // Each visible id's conclusion clause.
    let mut visible: HashMap<Rc<str>, Rc<[Rc<Term>]>> = HashMap::new();
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
        let in_context = frames.iter().any(|frame| frame.context);
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
                    frame.last = Some(clause.clone());
                }
                visible.insert(id.clone(), clause);
                commands.push(Command {
                    kind: CommandKind::Assume { term, local },
                    premises_visible: true,
                    in_context,
                });
            }
            ("anchor", _) => {
                let Some(attrs) = attributes(1) else {
                    return Reading::Malformed;
                };
                let Some(Sexpr::Atom(closing)) = attrs.get(":step") else {
                    return Reading::Malformed;
                };
                let context = attrs.has(":args");
                frames.push(Frame {
                    closing: closing.clone(),
                    context,
                    defined: Vec::new(),
                    assumptions: Vec::new(),
                    last: None,
                });
                commands.push(Command {
                    kind: CommandKind::Anchor { context },
                    premises_visible: true,
                    in_context,
                });
            }
            ("define-fun", [Sexpr::Atom(name), Sexpr::List(parameters), _sort, body]) => {
                if parameters.is_empty() {
                    let Some(term) = names.term(body) else {
                        return Reading::Malformed;
                    };
                    names.terms.insert(name.clone(), term);
                }
                commands.push(Command {
                    kind: CommandKind::Define {
                        parameters: !parameters.is_empty(),
                    },
                    premises_visible: true,
                    in_context,
                });
            }
            ("step", [Sexpr::Atom(id), clause, ..]) => {
                let Some(attrs) = attributes(3) else {
                    return Reading::Malformed;
                };
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
                let premises: Vec<Rc<[Rc<Term>]>> = premise_ids
                    .iter()
                    .filter_map(|premise| {
                        let found = visible.get(premise).cloned();
                        premises_visible &= found.is_some();
                        found
                    })
                    .collect();
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
                    Some(Subproof {
                        discharged,
                        discharges_exactly,
                        last: frame.last,
                    })
                } else {
                    if !discharge_ids.is_empty() {
                        premises_visible = false;
                    }
                    None
                };
                if let Some(frame) = frames.last_mut() {
                    frame.defined.push(id.clone());
                    frame.last = Some(clause.clone());
                }
                visible.insert(id.clone(), clause.clone());
                commands.push(Command {
                    kind: CommandKind::Step(Step {
                        clause: clause.to_vec(),
                        rule: rule.clone(),
                        premises,
                        args,
                        closes,
                    }),
                    premises_visible,
                    in_context,
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

    fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
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

impl Proof {
    /// The number of commands.
    pub(crate) fn len(&self) -> usize {
        self.commands.len()
    }

    /// FR-314's `ProofStepInvalid` rule over every command, in order.
    pub(crate) fn invalid_step(&self, query: &Query) -> Option<u64> {
        self.commands
            .iter()
            .zip(0_u64..)
            .find(|(command, _)| !command.valid(query))
            .map(|(_, index)| index)
    }

    /// FR-314's last-command rule: the index of a last command that does
    /// not conclude the empty clause.
    pub(crate) fn not_refutation(&self) -> Option<u64> {
        let last = self.commands.last()?;
        let refutes = matches!(&last.kind, CommandKind::Step(step) if step.clause.is_empty());
        (!refutes).then(|| u64::try_from(self.commands.len() - 1).unwrap_or(u64::MAX))
    }

    /// FR-314's `Unverifiable` rule: the first command this checker does
    /// not check.
    pub(crate) fn first_unchecked(&self) -> Option<u64> {
        self.commands
            .iter()
            .zip(0_u64..)
            .find(|(command, _)| !command.checked())
            .map(|(_, index)| index)
    }

    /// The outcome of a proof no rule rejects.
    pub(crate) fn outcome(&self) -> ProofOutcome {
        match self.first_unchecked() {
            Some(step) => ProofOutcome::Unverifiable { step },
            None => ProofOutcome::Refutes,
        }
    }
}

impl Command {
    /// Whether this checker checks the command's rule.
    fn checked(&self) -> bool {
        if self.in_context {
            return false;
        }
        match &self.kind {
            CommandKind::Assume { .. } => true,
            CommandKind::Anchor { context } => !context,
            CommandKind::Define { parameters } => !parameters,
            CommandKind::Step(step) => match checked_rule(&step.rule) {
                None | Some(Rule::Bind) => false,
                // A closing step of a subproof is checked as `subproof`
                // only.
                Some(Rule::Subproof) => step.closes.is_some(),
                Some(_) => step.closes.is_none(),
            },
        }
    }

    /// FR-314's `ProofStepInvalid` test of one command: its premises are
    /// earlier visible commands, and, for a checked rule, its conclusion
    /// follows by the rule.
    fn valid(&self, query: &Query) -> bool {
        if !self.premises_visible {
            return false;
        }
        if !self.checked() {
            return true;
        }
        match &self.kind {
            CommandKind::Assume { term, local } => *local || query.assertions.contains(term),
            CommandKind::Anchor { .. } | CommandKind::Define { .. } => true,
            CommandKind::Step(step) => match checked_rule(&step.rule) {
                Some(rule) => step.follows(rule, query),
                None => true,
            },
        }
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
    fn index_arg(&self) -> Option<Option<usize>> {
        match self.args.as_slice() {
            [] => Some(None),
            [arg] => match &**arg {
                Term::Number { value, real: false } => value
                    .numerator()
                    .to_u64()
                    .and_then(|k| usize::try_from(k).ok())
                    .map(Some),
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
            Rule::Bind => false,
            Rule::Refl => match clause {
                [literal] => literal
                    .args_of("=")
                    .is_some_and(|args| matches!(args, [left, right] if left == right)),
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
            Rule::Resolution => resolves(&self.premises, clause),
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
                match k {
                    Some(k) => conjuncts.get(k).is_some_and(|c| pick(c) == *literal),
                    None => conjuncts.iter().any(|c| pick(c) == *literal),
                }
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
                    Some(Some(k)) => conjuncts.get(k) == Some(literal),
                    Some(None) => conjuncts.contains(literal),
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
                    Some(Some(k)) => disjuncts.get(k).is_some_and(|d| not(d) == *literal),
                    Some(None) => disjuncts.iter().any(|d| not(d) == *literal),
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

/// `resolution`: a chain of binary resolutions over the premises in order
/// derives a clause whose literals all lie in `conclusion`. Each step
/// resolves on a pivot whose base formula occurs with opposite polarities
/// (leading negations merged) in the clause so far and the next premise,
/// preferring a pivot that does not occur in the conclusion. Deriving a
/// subset of the conclusion is sound: the conclusion follows by weakening.
fn resolves(premises: &[Rc<[Rc<Term>]>], conclusion: &[Rc<Term>]) -> bool {
    let Some((first, rest)) = premises.split_first() else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let wanted = set(conclusion);
    let mut current: Vec<Rc<Term>> = first.to_vec();
    for next in rest {
        let mut candidates = Vec::new();
        for (i, left) in current.iter().enumerate() {
            let (base_left, negative_left) = polarity(left);
            for (j, right) in next.iter().enumerate() {
                let (base_right, negative_right) = polarity(right);
                if base_left == base_right && negative_left != negative_right {
                    candidates.push((i, j));
                }
            }
        }
        let pick = candidates
            .iter()
            .find(|(i, j)| !wanted.contains(&current[*i]) && !wanted.contains(&next[*j]))
            .or_else(|| candidates.first());
        let Some(&(i, j)) = pick else {
            return false;
        };
        let pivot_left = current[i].clone();
        let pivot_right = next[j].clone();
        let mut resolved: Vec<Rc<Term>> = current
            .into_iter()
            .filter(|literal| *literal != pivot_left)
            .collect();
        resolved.extend(
            next.iter()
                .filter(|literal| **literal != pivot_right)
                .cloned(),
        );
        current = resolved;
    }
    current.iter().all(|literal| wanted.contains(literal))
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
                "<" => (Relation::Greater, s2, s1),
                "<=" => (Relation::GreaterEqual, s2, s1),
                ">" => (Relation::Greater, s1, s2),
                ">=" => (Relation::GreaterEqual, s1, s2),
                "=" => (Relation::Equal, s1, s2),
                _ => return None,
            }
        }
        // Step 1: a positive literal is negated.
        None => {
            let (head, [s1, s2]) = comparison(literal)?;
            match head {
                ">" => (Relation::GreaterEqual, s2, s1),
                ">=" => (Relation::Greater, s2, s1),
                "<" => (Relation::GreaterEqual, s1, s2),
                "<=" => (Relation::Greater, s1, s2),
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
fn comparison(term: &Rc<Term>) -> Option<(&str, [&Rc<Term>; 2])> {
    match &**term {
        Term::App(head, args) => {
            let [a, b] = args.as_slice() else {
                return None;
            };
            matches!(&**head, "<" | "<=" | ">" | ">=" | "=").then_some((&**head, [a, b]))
        }
        _ => None,
    }
}

/// Whether every atom of `s` is an integer-sorted constant of `query`.
fn integer_sorted(s: &Linear, query: &Query) -> bool {
    s.terms.keys().all(|atom| match &**atom {
        Term::Symbol(name) => query.integers.contains(name),
        _ => false,
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
    let expected_sign = if positive { ">" } else { "<" };
    if sign_head != expected_sign || !matches!(&**z, Term::Number { value, .. } if value.is_zero())
    {
        return false;
    }
    let product = |t: &Rc<Term>| Term::app("*", vec![t1.clone(), t.clone()]);
    if let (Some(inner), Some(outer)) = (relation.negated(), conclusion.negated()) {
        let (Some(("=", [t2, t3])), Some(("=", [p2, p3]))) = (comparison(inner), comparison(outer))
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
        (false, "<") => ">",
        (false, "<=") => ">=",
        (false, ">") => "<",
        (false, ">=") => "<=",
        (false, head) => head,
    };
    if outer_head != inverted {
        return false;
    }
    if head == "=" {
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
        if let Some(index) = proof.invalid_step(&query) {
            return Err(format!("invalid {index}"));
        }
        if let Some(index) = proof.not_refutation() {
            return Err(format!("short {index}"));
        }
        Ok(proof.outcome())
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
}

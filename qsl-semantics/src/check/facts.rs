// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 static definedness: stable paths, guard facts, interval proofs and
//! the call sites termination checking consumes.
//!
//! Facts are only ever added by the accepted guard forms: `present(p)` for a
//! stable path `p`, and an integer ordering or equality between a stable
//! integer path (or `size(p)`) and an integer literal. A comparison of two
//! non-literal operands yields no fact. The walk and every guard-fact helper
//! it calls run on the walker toolkit, and each path's facts are persistent
//! (one trie path more than its parent's), so no depth of typed expression is
//! a limit and no chain of nested guards copies facts per level (ADR-030
//! D-1).

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::marker::PhantomData;
use std::ops::ControlFlow;
use std::rc::Rc;

mod persistent;

use self::persistent::PersistentMap;
use super::check::DispatchOperation;
use super::ir::{
    Arithmetic, CheckedLiteral, CheckedNode, Connective, DispatchTable, NodeKind, Observation,
    OrderedKind, Slot, Visit,
};
use super::observation::Observations;
use super::refusal::{
    CheckCause, CheckRefusal, InvalidDispatchDeclaration, Obligation, ProvedInterval,
};
use quire_exact::{ArithmeticOperator, OrderingOperator};
use quire_exact::{CollectionType, Value, ValueType};
use quire_exact::{Integer, IntegerInterval};
use quire_semantic_value::declaration::{EqualityOperator, FieldRef};
use quire_semantic_value::location::Location;

/// One step of a stable path.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum Step {
    Field(usize),
    Value,
    /// A model attribute read in a state clause (FR-104), with the
    /// observation it reads at: a fact about one observation of a path
    /// never discharges an obligation at another.
    Attribute(FieldRef, Observation),
}

/// A parameter, `let` or binder root followed by field projections and
/// guarded `value` steps.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct StablePath {
    pub(crate) root: Slot,
    pub(crate) steps: Vec<Step>,
}

/// What an interval fact is about.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum Subject {
    Value(StablePath),
    Size(StablePath),
}

/// The facts proved on one reachable path. Each is a persistent map, so a
/// path's facts are its parent's plus one entry at the cost of one trie
/// path, and a chain of nested guards shares them all.
#[derive(Clone, Default)]
struct Facts {
    intervals: PersistentMap<Subject, ProvedInterval>,
    present: PersistentMap<StablePath, ()>,
    nonzero: PersistentMap<StablePath, ()>,
}

/// How a call argument relates to the caller's parameters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArgumentShape {
    /// Exactly parameter `p`.
    Parameter(Slot),
    /// `p - k` for a literal `k >= 1`.
    Decremented(Slot),
    /// A nonempty stable projection path rooted at parameter `p`.
    Projection(Slot),
    /// Anything else.
    Other,
}

/// Whether a call-graph edge is an ordinary named call or an FR-151 dispatch
/// edge (TC-196 D08). A cycle containing a dispatch edge is refused
/// `definition-cycle` before the measure logic runs; an ordinary cycle keeps
/// its existing measure-decrease obligations unaffected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EdgeKind {
    Ordinary,
    Dispatch,
}

/// One reachable call.
#[derive(Clone, Debug)]
pub(crate) struct CallSite {
    pub(crate) callee: usize,
    pub(crate) location: Location,
    pub(crate) arguments: Vec<ArgumentShape>,
    pub(crate) kind: EdgeKind,
}

/// One end of an extended integer interval.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum End {
    NegativeInfinity,
    Finite(Integer),
    PositiveInfinity,
}

impl End {
    fn negate(&self) -> Self {
        match self {
            Self::NegativeInfinity => Self::PositiveInfinity,
            Self::Finite(value) => Self::Finite(value.neg()),
            Self::PositiveInfinity => Self::NegativeInfinity,
        }
    }

    fn sign(&self) -> std::cmp::Ordering {
        match self {
            Self::NegativeInfinity => std::cmp::Ordering::Less,
            Self::Finite(value) => value.cmp(&Integer::zero()),
            Self::PositiveInfinity => std::cmp::Ordering::Greater,
        }
    }

    /// The sum of two ends of the same side; the side decides `inf - inf`.
    fn add(&self, other: &Self, lower: bool) -> Self {
        match (self, other) {
            (Self::Finite(left), Self::Finite(right)) => Self::Finite(left.add(right)),
            (Self::NegativeInfinity, _) | (_, Self::NegativeInfinity) if lower => {
                Self::NegativeInfinity
            }
            (Self::PositiveInfinity, _) | (_, Self::PositiveInfinity) if !lower => {
                Self::PositiveInfinity
            }
            (Self::NegativeInfinity, _) | (_, Self::NegativeInfinity) => Self::NegativeInfinity,
            (Self::PositiveInfinity, _) | (_, Self::PositiveInfinity) => Self::PositiveInfinity,
        }
    }

    /// The product, with `0 * inf = 0`.
    fn mul(&self, other: &Self) -> Self {
        use std::cmp::Ordering::{Equal, Greater, Less};
        match (self, other) {
            (Self::Finite(left), Self::Finite(right)) => Self::Finite(left.mul(right)),
            _ => match (self.sign(), other.sign()) {
                (Equal, _) | (_, Equal) => Self::Finite(Integer::zero()),
                (Less, Less) | (Greater, Greater) => Self::PositiveInfinity,
                (Less, Greater) | (Greater, Less) => Self::NegativeInfinity,
            },
        }
    }
}

/// An extended integer interval, `lower <= upper` unless empty.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Interval {
    lower: End,
    upper: End,
}

impl Interval {
    fn unbounded() -> Self {
        Self {
            lower: End::NegativeInfinity,
            upper: End::PositiveInfinity,
        }
    }

    fn point(value: &Integer) -> Self {
        Self {
            lower: End::Finite(value.clone()),
            upper: End::Finite(value.clone()),
        }
    }

    fn of_domain(domain: &IntegerInterval) -> Self {
        Self {
            lower: End::Finite(domain.lower().clone()),
            upper: End::Finite(domain.upper().clone()),
        }
    }

    fn of_proved(proved: &ProvedInterval) -> Self {
        Self {
            lower: proved
                .lower
                .clone()
                .map_or(End::NegativeInfinity, End::Finite),
            upper: proved
                .upper
                .clone()
                .map_or(End::PositiveInfinity, End::Finite),
        }
    }

    fn proved(&self) -> ProvedInterval {
        let finite = |end: &End| match end {
            End::Finite(value) => Some(value.clone()),
            End::NegativeInfinity | End::PositiveInfinity => None,
        };
        ProvedInterval {
            lower: finite(&self.lower),
            upper: finite(&self.upper),
        }
    }

    fn is_empty(&self) -> bool {
        self.lower > self.upper
    }

    fn add(&self, other: &Self) -> Self {
        Self {
            lower: self.lower.add(&other.lower, true),
            upper: self.upper.add(&other.upper, false),
        }
    }

    fn negate(&self) -> Self {
        Self {
            lower: self.upper.negate(),
            upper: self.lower.negate(),
        }
    }

    fn mul(&self, other: &Self) -> Self {
        let products = [
            self.lower.mul(&other.lower),
            self.lower.mul(&other.upper),
            self.upper.mul(&other.lower),
            self.upper.mul(&other.upper),
        ];
        let lower = products.iter().min().cloned();
        let upper = products.iter().max().cloned();
        Self {
            lower: lower.unwrap_or(End::NegativeInfinity),
            upper: upper.unwrap_or(End::PositiveInfinity),
        }
    }

    fn intersect(&self, other: &Self) -> Self {
        Self {
            lower: self.lower.clone().max(other.lower.clone()),
            upper: self.upper.clone().min(other.upper.clone()),
        }
    }

    fn hull(&self, other: &Self) -> Self {
        Self {
            lower: self.lower.clone().min(other.lower.clone()),
            upper: self.upper.clone().max(other.upper.clone()),
        }
    }

    fn within(&self, outer: &Self) -> bool {
        outer.lower <= self.lower && self.upper <= outer.upper
    }

    fn excludes_zero(&self) -> bool {
        let zero = End::Finite(Integer::zero());
        self.lower > zero || self.upper < zero
    }

    fn magnitude(&self) -> End {
        let lower = self.lower.negate();
        lower.max(self.upper.clone())
    }
}

fn declared(value_type: &ValueType) -> Interval {
    match value_type {
        ValueType::Int(domain) => Interval::of_domain(domain),
        _ => Interval::unbounded(),
    }
}

fn cardinality(collection: &CollectionType) -> Interval {
    match collection.bound() {
        Some(bound) => Interval {
            lower: End::Finite(Integer::from(bound.minimum())),
            upper: End::Finite(Integer::from(bound.maximum())),
        },
        // An unbounded collection (ADR-014 §2) holds any finite count.
        None => Interval {
            lower: End::Finite(Integer::zero()),
            upper: End::PositiveInfinity,
        },
    }
}

/// The interval of every accumulated prefix of a `sum` over `size`
/// summands each in `summand`, in every visit order: the empty prefix when
/// the source may be empty, and every prefix of `k` summands for
/// `1 <= k <= max`, whose sum lies in `[k * lower, k * upper]`.
fn prefix_sums(summand: &Interval, size: &Interval) -> Interval {
    let zero = End::Finite(Integer::zero());
    let one = End::Finite(Integer::one());
    let longest = size.upper.clone();
    let shortest = if size.lower <= zero {
        zero.clone()
    } else {
        one.clone()
    };
    if longest < one {
        return Interval::point(&Integer::zero());
    }
    let reach = |end: &End| {
        let candidates = [shortest.mul(end), one.mul(end), longest.mul(end)];
        let lower = candidates
            .iter()
            .min()
            .cloned()
            .unwrap_or(End::NegativeInfinity);
        let upper = candidates
            .iter()
            .max()
            .cloned()
            .unwrap_or(End::PositiveInfinity);
        (lower, upper)
    };
    let (low_lower, low_upper) = reach(&summand.lower);
    let (high_lower, high_upper) = reach(&summand.upper);
    let mut proved = Interval {
        lower: low_lower.min(high_lower),
        upper: low_upper.max(high_upper),
    };
    if shortest == zero {
        proved = proved.hull(&Interval::point(&Integer::zero()));
    }
    proved
}

/// Join two outcomes of a branch point: keep common interval subjects as
/// their hull, and the common presence and nonzero facts.
fn join(left: Option<Rc<Facts>>, right: Option<Rc<Facts>>) -> Option<Rc<Facts>> {
    match (left, right) {
        (None, other) | (other, None) => other,
        (Some(left), Some(right)) if Rc::ptr_eq(&left, &right) => Some(left),
        (Some(left), Some(right)) => Some(Rc::new(Facts {
            intervals: left
                .intervals
                .intersect_with(&right.intervals, &|left, right| {
                    Some(
                        Interval::of_proved(left)
                            .hull(&Interval::of_proved(right))
                            .proved(),
                    )
                }),
            present: left
                .present
                .intersect_with(&right.present, &|_, _| Some(())),
            nonzero: left
                .nonzero
                .intersect_with(&right.nonzero, &|_, _| Some(())),
        })),
    }
}

/// The relation a guard asserts between a subject and a literal.
#[derive(Clone, Copy)]
enum Relation {
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Equal,
    NotEqual,
}

impl Relation {
    fn negated(self) -> Self {
        match self {
            Self::Less => Self::GreaterOrEqual,
            Self::LessOrEqual => Self::Greater,
            Self::Greater => Self::LessOrEqual,
            Self::GreaterOrEqual => Self::Less,
            Self::Equal => Self::NotEqual,
            Self::NotEqual => Self::Equal,
        }
    }

    /// `k R s` as `s R' k`.
    fn flipped(self) -> Self {
        match self {
            Self::Less => Self::Greater,
            Self::LessOrEqual => Self::GreaterOrEqual,
            Self::Greater => Self::Less,
            Self::GreaterOrEqual => Self::LessOrEqual,
            Self::Equal | Self::NotEqual => self,
        }
    }
}

/// The definedness walker of one declaration.
pub(crate) struct Definedness<'a> {
    pub(crate) calls: Vec<CallSite>,
    parameters: usize,
    dispatch_tables: &'a [DispatchTable],
    dispatch_operations: &'a [DispatchOperation],
    /// A state clause's model reads and their observations (FR-104), or
    /// `None` outside a state clause. Only a state clause extends stable
    /// paths through model attribute reads and `pre(e)`, each attribute
    /// step keyed by its observation.
    observations: Option<&'a Observations>,
    /// The stable path each `let` slot aliases, resolved when the `let` was
    /// walked. A slot is bound by exactly one `let` and read only inside its
    /// body, so one table serves every path of the walk and no `let` copies
    /// the facts of the paths above it.
    aliases: BTreeMap<Slot, StablePath>,
}

impl<'a> Definedness<'a> {
    pub(crate) fn new(
        parameters: usize,
        dispatch_tables: &'a [DispatchTable],
        dispatch_operations: &'a [DispatchOperation],
    ) -> Self {
        Self {
            calls: Vec::new(),
            parameters,
            dispatch_tables,
            dispatch_operations,
            observations: None,
            aliases: BTreeMap::new(),
        }
    }

    /// Check a state clause body whose model reads observe `observations`
    /// (FR-104 "Definedness facts").
    pub(crate) fn with_observations(mut self, observations: &'a Observations) -> Self {
        self.observations = Some(observations);
        self
    }

    /// Check every obligation on a path that can execute.
    pub(crate) fn check(&mut self, root: CheckedNode<'_>) -> Result<(), CheckRefusal> {
        self.aliases.clear();
        self.walk(root, Facts::default())
    }

    /// The stable path `node` reads under `facts`, if it reads one: the
    /// projection chain is followed to its root on a loop, then its steps
    /// are applied root first.
    fn stable_path(&self, node: CheckedNode<'_>, facts: &Facts) -> Option<StablePath> {
        // The steps from `node` down to the root, outermost first.
        let mut steps = Vec::new();
        let mut current = node;
        let root = loop {
            match current.kind() {
                NodeKind::Local(slot) => break *slot,
                NodeKind::Field { operand, index, .. } => {
                    steps.push(PathStep::Field(*index));
                    current = current.at(*operand);
                }
                NodeKind::Value(operand) => {
                    steps.push(PathStep::Value);
                    current = current.at(*operand);
                }
                // FR-104: in a state clause a stable path also takes the
                // steps `deref(value(p)).f` and `self.f` take, each keyed by
                // the observation it reads at; `pre(e)` retags no step
                // itself, since the reads under it already carry `pre`.
                NodeKind::Attribute {
                    reference, field, ..
                } => {
                    let observation = self.observations?.of_read(current.location())?;
                    steps.push(PathStep::Attribute(Step::Attribute(
                        field.clone(),
                        observation,
                    )));
                    current = current.at(*reference);
                }
                NodeKind::Pre(operand) if self.observations.is_some() => {
                    current = current.at(*operand);
                }
                _ => return None,
            }
        };
        let mut path = self.aliases.get(&root).cloned().unwrap_or(StablePath {
            root,
            steps: Vec::new(),
        });
        for step in steps.into_iter().rev() {
            match step {
                PathStep::Field(index) => path.steps.push(Step::Field(index)),
                PathStep::Value => {
                    if !facts.present.contains(&path) {
                        return None;
                    }
                    path.steps.push(Step::Value);
                }
                PathStep::Attribute(step) => path.steps.push(step),
            }
        }
        Some(path)
    }

    fn subject(&self, node: CheckedNode<'_>, facts: &Facts) -> Option<Subject> {
        let mut node = node;
        while let NodeKind::Coerce(operand, _) = node.kind() {
            node = node.at(*operand);
        }
        match node.kind() {
            NodeKind::Size(operand) => self
                .stable_path(node.at(*operand), facts)
                .map(Subject::Size),
            _ if matches!(node.value_type(), ValueType::Integer | ValueType::Int(_)) => {
                self.stable_path(node, facts).map(Subject::Value)
            }
            _ => None,
        }
    }

    /// The interval `node` lies in under `facts`, evaluated bottom up on the
    /// walker toolkit.
    fn interval(&self, node: CheckedNode<'_>, facts: &Facts) -> Interval {
        let mut walk = IntervalWalk {
            definedness: self,
            facts,
            values: Vec::new(),
            nodes: PhantomData,
        };
        let ControlFlow::Continue(()) = quire_walk::walk(&mut walk, node);
        walk.values.pop().unwrap_or_else(Interval::unbounded)
    }

    /// The integer literal `node` is, through any `-` and coercions.
    fn literal(node: CheckedNode<'_>) -> Option<Integer> {
        let mut node = node;
        let mut negated = false;
        loop {
            match node.kind() {
                NodeKind::Literal(CheckedLiteral(Value::Integer(value))) => {
                    return Some(if negated { value.neg() } else { value.clone() });
                }
                NodeKind::Negate(operand) => {
                    negated = !negated;
                    node = node.at(*operand);
                }
                NodeKind::Coerce(operand, _) => node = node.at(*operand),
                _ => return None,
            }
        }
    }

    /// Apply `subject R k` to `facts`, or `None` when it is unsatisfiable.
    fn relate(
        &self,
        subject: CheckedNode<'_>,
        relation: Relation,
        literal: &Integer,
        facts: &Rc<Facts>,
    ) -> Option<Rc<Facts>> {
        let Some(key) = self.subject(subject, facts) else {
            return Some(Rc::clone(facts));
        };
        let current = self.interval(subject, facts);
        let one = Integer::one();
        let refined = match relation {
            Relation::Less => current.intersect(&Interval {
                lower: End::NegativeInfinity,
                upper: End::Finite(literal.sub(&one)),
            }),
            Relation::LessOrEqual => current.intersect(&Interval {
                lower: End::NegativeInfinity,
                upper: End::Finite(literal.clone()),
            }),
            Relation::Greater => current.intersect(&Interval {
                lower: End::Finite(literal.add(&one)),
                upper: End::PositiveInfinity,
            }),
            Relation::GreaterOrEqual => current.intersect(&Interval {
                lower: End::Finite(literal.clone()),
                upper: End::PositiveInfinity,
            }),
            Relation::Equal => current.intersect(&Interval::point(literal)),
            Relation::NotEqual => {
                let mut refined = current;
                let point = End::Finite(literal.clone());
                if refined.lower == point {
                    refined.lower = End::Finite(literal.add(&one));
                }
                if refined.upper == point {
                    refined.upper = End::Finite(literal.sub(&one));
                }
                refined
            }
        };
        if refined.is_empty() {
            return None;
        }
        let mut facts = Facts::clone(facts);
        if let (Relation::NotEqual, true, Subject::Value(path)) =
            (relation, literal.is_zero(), &key)
        {
            facts.nonzero.insert(path.clone(), ());
        }
        facts.intervals.insert(key, refined.proved());
        Some(Rc::new(facts))
    }

    fn comparison(
        &self,
        left: CheckedNode<'_>,
        right: CheckedNode<'_>,
        relation: Relation,
        facts: &Rc<Facts>,
    ) -> Outcomes {
        let (subject, relation, literal) = match (Self::literal(left), Self::literal(right)) {
            (None, Some(literal)) => (left, relation, literal),
            (Some(literal), None) => (right, relation.flipped(), literal),
            (Some(_), Some(_)) | (None, None) => {
                return (Some(Rc::clone(facts)), Some(Rc::clone(facts)));
            }
        };
        (
            self.relate(subject, relation, &literal, facts),
            self.relate(subject, relation.negated(), &literal, facts),
        )
    }

    /// The facts on the true and false outcomes of a reachable condition,
    /// on the walker toolkit: a connective's right operand is evaluated
    /// under the facts its left operand's outcome establishes.
    fn outcomes(&self, condition: CheckedNode<'_>, facts: &Rc<Facts>) -> Outcomes {
        let mut walk = OutcomeWalk {
            definedness: self,
            results: Vec::new(),
            nodes: PhantomData,
        };
        let ControlFlow::Continue(()) = quire_walk::walk(
            &mut walk,
            OutcomeNode::Evaluate(condition, Rc::clone(facts)),
        );
        walk.results.pop().unwrap_or_default()
    }

    /// The outcomes of a condition that is not a negation, `pre` or
    /// connective: a literal, or a guard form that adds a fact.
    fn guard(&self, condition: CheckedNode<'_>, facts: &Rc<Facts>) -> Outcomes {
        match condition.kind() {
            NodeKind::Literal(CheckedLiteral(Value::Boolean(true))) => {
                (Some(Rc::clone(facts)), None)
            }
            NodeKind::Literal(CheckedLiteral(Value::Boolean(false))) => {
                (None, Some(Rc::clone(facts)))
            }
            NodeKind::Present(operand) => match self.stable_path(condition.at(*operand), facts) {
                Some(path) => {
                    let mut when_true = Facts::clone(facts);
                    when_true.present.insert(path, ());
                    (Some(Rc::new(when_true)), Some(Rc::clone(facts)))
                }
                None => (Some(Rc::clone(facts)), Some(Rc::clone(facts))),
            },
            NodeKind::Order(operator, OrderedKind::Integers, left, right) => {
                let relation = match operator {
                    OrderingOperator::Less => Relation::Less,
                    OrderingOperator::LessOrEqual => Relation::LessOrEqual,
                    OrderingOperator::Greater => Relation::Greater,
                    OrderingOperator::GreaterOrEqual => Relation::GreaterOrEqual,
                };
                self.comparison(condition.at(*left), condition.at(*right), relation, facts)
            }
            NodeKind::Equality(operator, _, left, right)
                if is_integer(condition.at(*left)) && is_integer(condition.at(*right)) =>
            {
                let relation = match operator {
                    EqualityOperator::Equal => Relation::Equal,
                    EqualityOperator::NotEqual => Relation::NotEqual,
                };
                self.comparison(condition.at(*left), condition.at(*right), relation, facts)
            }
            _ => (Some(Rc::clone(facts)), Some(Rc::clone(facts))),
        }
    }

    fn refuse(node: CheckedNode<'_>, obligation: Obligation) -> CheckRefusal {
        CheckRefusal {
            location: node.location().clone(),
            cause: CheckCause::Unproved(obligation),
        }
    }

    fn shape(&self, argument: CheckedNode<'_>, facts: &Facts) -> ArgumentShape {
        let argument = match argument.kind() {
            NodeKind::Coerce(operand, _) => argument.at(*operand),
            _ => argument,
        };
        let parameter = |node: CheckedNode<'_>| match node.kind() {
            NodeKind::Local(slot) if *slot < self.parameters => Some(*slot),
            _ => None,
        };
        if let Some(slot) = parameter(argument) {
            return ArgumentShape::Parameter(slot);
        }
        if let NodeKind::Arithmetic(Arithmetic::Subtract, left, right) = argument.kind() {
            if let (Some(slot), Some(decrement)) = (
                parameter(argument.at(*left)),
                Self::literal(argument.at(*right)),
            ) {
                if decrement >= Integer::one() {
                    return ArgumentShape::Decremented(slot);
                }
            }
        }
        match self.stable_path(argument, facts) {
            Some(path) if path.root < self.parameters && !path.steps.is_empty() => {
                ArgumentShape::Projection(path.root)
            }
            _ => ArgumentShape::Other,
        }
    }

    /// Walk `root` under `facts`, depth first in evaluation order, on the
    /// walker toolkit. A node's obligation is discharged once its operands
    /// are walked, and a node whose later operands run under facts its first
    /// operand establishes (`if`, a connective, `let`) walks them once that
    /// operand is walked: the order, and so the first refusal and the order
    /// of [`Self::calls`], is the recursive walk's.
    fn walk(&mut self, root: CheckedNode<'_>, facts: Facts) -> Result<(), CheckRefusal> {
        let mut walk = DefinednessWalk {
            definedness: self,
            nodes: PhantomData,
        };
        match quire_walk::walk(&mut walk, DefinedNode::Walk(root, Rc::new(facts))) {
            ControlFlow::Continue(()) => Ok(()),
            ControlFlow::Break(refusal) => Err(refusal),
        }
    }

    /// `node`'s first operand is walked under `facts`: enter the operands
    /// that run under the facts its outcome establishes.
    fn branch<'n>(
        &mut self,
        node: CheckedNode<'n>,
        facts: &Rc<Facts>,
        children: &mut quire_walk::Children<'_, DefinedNode<'n>>,
    ) {
        match node.kind() {
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => {
                let (when_true, when_false) = self.outcomes(node.at(*condition), facts);
                if let Some(when_true) = when_true {
                    children.push(DefinedNode::Walk(node.at(*then), when_true));
                }
                if let Some(when_false) = when_false {
                    children.push(DefinedNode::Walk(node.at(*otherwise), when_false));
                }
            }
            NodeKind::Connective(connective, left, right) => {
                let (when_true, when_false) = self.outcomes(node.at(*left), facts);
                let under = match connective {
                    Connective::And | Connective::Implies => when_true,
                    Connective::Or => when_false,
                };
                if let Some(under) = under {
                    children.push(DefinedNode::Walk(node.at(*right), under));
                }
            }
            NodeKind::Let { slot, value, body } => {
                if let Some(path) = self.stable_path(node.at(*value), facts) {
                    self.aliases.insert(*slot, path);
                }
                children.push(DefinedNode::Walk(node.at(*body), Rc::clone(facts)));
            }
            _ => {}
        }
    }

    /// Discharge `node`'s own obligation, its operands walked under `facts`,
    /// and record the call sites it makes.
    fn discharge(&mut self, node: CheckedNode<'_>, facts: &Facts) -> Result<(), CheckRefusal> {
        match node.kind() {
            NodeKind::Coerce(operand, target) => {
                let proved = self.interval(node.at(*operand), facts);
                let required = Interval::of_domain(target);
                if proved.within(&required) {
                    Ok(())
                } else {
                    Err(Self::refuse(
                        node,
                        Obligation::Range {
                            required: Box::new(required.proved()),
                            proved: Box::new(proved.proved()),
                        },
                    ))
                }
            }
            NodeKind::Divide {
                left,
                right,
                domain,
            } => {
                let divisor = self.interval(node.at(*right), facts);
                let nonzero = divisor.excludes_zero()
                    || self
                        .stable_path(node.at(*right), facts)
                        .is_some_and(|path| facts.nonzero.contains(&path));
                if !nonzero {
                    return Err(Self::refuse(node, Obligation::Nonzero));
                }
                let dividend = self.interval(node.at(*left), facts);
                let zero = End::Finite(Integer::zero());
                let numerator = if divisor.lower > zero {
                    Interval {
                        lower: dividend.lower.clone().min(zero.clone()),
                        upper: dividend.upper.clone().max(zero),
                    }
                } else if divisor.upper < zero {
                    let negated = dividend.negate();
                    Interval {
                        lower: negated.lower.min(zero.clone()),
                        upper: negated.upper.max(zero),
                    }
                } else {
                    let magnitude = dividend.magnitude();
                    Interval {
                        lower: magnitude.negate(),
                        upper: magnitude,
                    }
                };
                let one = End::Finite(Integer::one());
                let denominator = Interval {
                    lower: one.clone(),
                    upper: divisor.magnitude().max(one),
                };
                if numerator.within(&Interval::of_domain(domain.numerator()))
                    && denominator.within(&Interval::of_domain(domain.denominator()))
                {
                    Ok(())
                } else {
                    Err(Self::refuse(node, Obligation::RationalRange))
                }
            }
            NodeKind::Rational {
                operator,
                left,
                right,
                domain,
            } => {
                let (ValueType::Rational(left), ValueType::Rational(right)) =
                    (node.at(*left).value_type(), node.at(*right).value_type())
                else {
                    return Err(Self::refuse(node, Obligation::RationalRange));
                };
                if *operator == ArithmeticOperator::Divide && !right.excludes_zero() {
                    return Err(Self::refuse(node, Obligation::Nonzero));
                }
                if domain.contains_domain(&left.result_of(*operator, right)) {
                    Ok(())
                } else {
                    Err(Self::refuse(node, Obligation::RationalRange))
                }
            }
            NodeKind::RationalNegate(operand, domain) => match node.at(*operand).value_type() {
                ValueType::Rational(source) if domain.contains_domain(&source.negated()) => Ok(()),
                _ => Err(Self::refuse(node, Obligation::RationalRange)),
            },
            NodeKind::Decimal {
                operator: ArithmeticOperator::Divide,
                right,
                ..
            } => {
                let zero = Integer::zero();
                match node.at(*right).value_type() {
                    ValueType::Decimal(divisor)
                        if divisor.lower() > &zero || divisor.upper() < &zero =>
                    {
                        Ok(())
                    }
                    _ => Err(Self::refuse(node, Obligation::Nonzero)),
                }
            }
            // A quantity carries no declared value interval and no guard form
            // proves one nonzero.
            NodeKind::Quantity(ArithmeticOperator::Divide, _, _) => {
                Err(Self::refuse(node, Obligation::Nonzero))
            }
            NodeKind::Query {
                visit: Visit::Sum,
                source,
                body,
                ..
            } => {
                let ValueType::Int(domain) = node.value_type() else {
                    return Ok(());
                };
                let size = match node.at(*source).value_type() {
                    ValueType::Collection(collection) => cardinality(collection),
                    _ => Interval::unbounded(),
                };
                let proved = prefix_sums(&self.interval(node.at(*body), facts), &size);
                let required = Interval::of_domain(domain);
                if proved.within(&required) {
                    Ok(())
                } else {
                    Err(Self::refuse(
                        node,
                        Obligation::Range {
                            required: Box::new(required.proved()),
                            proved: Box::new(proved.proved()),
                        },
                    ))
                }
            }
            NodeKind::Value(operand) => match self.stable_path(node.at(*operand), facts) {
                Some(path) if facts.present.contains(&path) => Ok(()),
                _ => Err(Self::refuse(node, Obligation::Presence)),
            },
            // A reduction (no identity) of a possibly empty source.
            NodeKind::Fold {
                source,
                identity: None,
                ..
            } => {
                let bound = match node.at(*source).value_type() {
                    ValueType::Collection(collection) => cardinality(collection),
                    _ => Interval::unbounded(),
                };
                let size = match self.stable_path(node.at(*source), facts) {
                    Some(path) => facts
                        .intervals
                        .get(&Subject::Size(path))
                        .map_or(bound, Interval::of_proved),
                    None => bound,
                };
                if size.lower < End::Finite(Integer::one()) {
                    return Err(Self::refuse(
                        node,
                        Obligation::NonemptyReduction {
                            size: Box::new(size.proved()),
                        },
                    ));
                }
                Ok(())
            }
            NodeKind::Call {
                function,
                arguments,
            } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| self.shape(node.at(*argument), facts))
                    .collect();
                self.calls.push(CallSite {
                    callee: *function,
                    location: node.location().clone(),
                    arguments,
                    kind: EdgeKind::Ordinary,
                });
                Ok(())
            }
            NodeKind::Dispatch {
                table, operation, ..
            } => {
                // Defense in depth: `PackageDeclarations::check`'s own
                // upfront validation already refuses an out-of-range
                // operation or table index before any node is walked, so
                // this should be unreachable — but silently treating either
                // as "no edges" would hide real call-graph edges rather
                // than refuse, so both still report a typed refusal. The
                // operation lookup happens once, up front, so the failure
                // path can read its `member` without cloning it on every
                // ordinary dispatch-node walk.
                let table_index = *table;
                let operation_index = *operation;
                let Some(declared_operation) = self.dispatch_operations.get(operation_index) else {
                    return Err(CheckRefusal {
                        location: node.location().clone(),
                        cause: CheckCause::InvalidDispatchDeclaration(Box::new(
                            InvalidDispatchDeclaration::OperationOutOfRange {
                                operation: operation_index,
                            },
                        )),
                    });
                };
                let callees = self
                    .dispatch_tables
                    .get(table_index)
                    .map(DispatchTable::callees)
                    .ok_or_else(|| CheckRefusal {
                        location: node.location().clone(),
                        cause: CheckCause::InvalidDispatchDeclaration(Box::new(
                            InvalidDispatchDeclaration::TableOutOfRange {
                                member: declared_operation.member.clone(),
                                table: table_index,
                            },
                        )),
                    })?;
                for callee in callees {
                    self.calls.push(CallSite {
                        callee,
                        location: node.location().clone(),
                        arguments: Vec::new(),
                        kind: EdgeKind::Dispatch,
                    });
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// [`Definedness::walk`]'s walk.
struct DefinednessWalk<'d, 'a, 'n> {
    definedness: &'d mut Definedness<'a>,
    nodes: PhantomData<CheckedNode<'n>>,
}

/// A node of the definedness walk.
enum DefinedNode<'n> {
    /// Walk a node under the facts of its path.
    Walk(CheckedNode<'n>, Rc<Facts>),
    /// The node's first operand is walked: enter the operands that run under
    /// the facts its outcome establishes.
    Branch(CheckedNode<'n>, Rc<Facts>),
    /// The node's obligation, between two of its operands.
    Discharge(CheckedNode<'n>, Rc<Facts>),
}

/// What the definedness walk keeps for an entered node.
enum DefinedFrame<'n> {
    /// Nothing is left to do when the node is exited.
    Done,
    /// The node's operands are walked: discharge its own obligation.
    Discharge(CheckedNode<'n>, Rc<Facts>),
}

impl<'n> quire_walk::Walk for DefinednessWalk<'_, '_, 'n> {
    type Node = DefinedNode<'n>;
    type Frame = DefinedFrame<'n>;
    type Stop = CheckRefusal;

    fn enter(
        &mut self,
        node: DefinedNode<'n>,
        children: &mut quire_walk::Children<'_, DefinedNode<'n>>,
    ) -> ControlFlow<CheckRefusal, DefinedFrame<'n>> {
        match node {
            DefinedNode::Branch(node, facts) => {
                self.definedness.branch(node, &facts, children);
                ControlFlow::Continue(DefinedFrame::Done)
            }
            DefinedNode::Discharge(node, facts) => match self.definedness.discharge(node, &facts) {
                Ok(()) => ControlFlow::Continue(DefinedFrame::Done),
                Err(refusal) => ControlFlow::Break(refusal),
            },
            DefinedNode::Walk(node, facts) => match node.kind() {
                NodeKind::If {
                    condition: first, ..
                }
                | NodeKind::Connective(_, first, _)
                | NodeKind::Let { value: first, .. } => {
                    children.push(DefinedNode::Walk(node.at(*first), Rc::clone(&facts)));
                    children.push(DefinedNode::Branch(node, facts));
                    ControlFlow::Continue(DefinedFrame::Done)
                }
                NodeKind::Fold {
                    source,
                    step,
                    identity,
                    ..
                } => {
                    // The source, then the identity or the nonempty
                    // obligation of a reduction, then the step.
                    children.push(DefinedNode::Walk(node.at(*source), Rc::clone(&facts)));
                    match identity {
                        Some(identity) => {
                            children.push(DefinedNode::Walk(node.at(*identity), Rc::clone(&facts)))
                        }
                        None => children.push(DefinedNode::Discharge(node, Rc::clone(&facts))),
                    }
                    children.push(DefinedNode::Walk(node.at(*step), facts));
                    ControlFlow::Continue(DefinedFrame::Done)
                }
                _ => {
                    for child in node.children() {
                        children.push(DefinedNode::Walk(child, Rc::clone(&facts)));
                    }
                    ControlFlow::Continue(DefinedFrame::Discharge(node, facts))
                }
            },
        }
    }

    fn exit(&mut self, frame: DefinedFrame<'n>) -> ControlFlow<CheckRefusal> {
        match frame {
            DefinedFrame::Done => ControlFlow::Continue(()),
            DefinedFrame::Discharge(node, facts) => {
                match self.definedness.discharge(node, &facts) {
                    Ok(()) => ControlFlow::Continue(()),
                    Err(refusal) => ControlFlow::Break(refusal),
                }
            }
        }
    }
}

/// The facts on a condition's true and false outcomes, each `None` when
/// that outcome cannot occur. The facts are shared, so a connective holds its
/// left outcomes without copying them.
type Outcomes = (Option<Rc<Facts>>, Option<Rc<Facts>>);

/// [`Definedness::interval`]'s walk: each node's exit leaves its interval on
/// `values`, built from its operands' intervals, which their exits left
/// there.
struct IntervalWalk<'d, 'a, 'f, 'n> {
    definedness: &'d Definedness<'a>,
    facts: &'f Facts,
    values: Vec<Interval>,
    nodes: PhantomData<CheckedNode<'n>>,
}

/// What an interval walk keeps for an entered node.
enum IntervalFrame {
    /// The node's interval is already on `values`.
    Done,
    Arithmetic(Arithmetic),
    Negate,
}

impl<'n> quire_walk::Walk for IntervalWalk<'_, '_, '_, 'n> {
    type Node = CheckedNode<'n>;
    type Frame = IntervalFrame;
    type Stop = Infallible;

    fn enter(
        &mut self,
        node: CheckedNode<'n>,
        children: &mut quire_walk::Children<'_, CheckedNode<'n>>,
    ) -> ControlFlow<Infallible, IntervalFrame> {
        let (definedness, facts) = (self.definedness, self.facts);
        let fact = |subject: Subject, fallback: Interval| {
            facts
                .intervals
                .get(&subject)
                .map_or(fallback, Interval::of_proved)
        };
        let leaf = match node.kind() {
            NodeKind::Arithmetic(operator, left, right) => {
                children.push(node.at(*left));
                children.push(node.at(*right));
                return ControlFlow::Continue(IntervalFrame::Arithmetic(*operator));
            }
            NodeKind::Negate(operand) => {
                children.push(node.at(*operand));
                return ControlFlow::Continue(IntervalFrame::Negate);
            }
            NodeKind::Literal(CheckedLiteral(Value::Integer(value))) => Interval::point(value),
            NodeKind::Coerce(_, target) => Interval::of_domain(target),
            NodeKind::Size(operand) => {
                let operand = node.at(*operand);
                let bound = match operand.value_type() {
                    ValueType::Collection(collection) => cardinality(collection),
                    _ => Interval::unbounded(),
                };
                match definedness.stable_path(operand, facts) {
                    Some(path) => fact(Subject::Size(path), bound),
                    None => bound,
                }
            }
            _ => match definedness.stable_path(node, facts) {
                Some(path) => fact(Subject::Value(path), declared(node.value_type())),
                None => declared(node.value_type()),
            },
        };
        self.values.push(leaf);
        ControlFlow::Continue(IntervalFrame::Done)
    }

    fn exit(&mut self, frame: IntervalFrame) -> ControlFlow<Infallible> {
        match frame {
            IntervalFrame::Done => {}
            IntervalFrame::Arithmetic(operator) => {
                let right = self.values.pop().unwrap_or_else(Interval::unbounded);
                let left = self.values.pop().unwrap_or_else(Interval::unbounded);
                self.values.push(match operator {
                    Arithmetic::Add => left.add(&right),
                    Arithmetic::Subtract => left.add(&right.negate()),
                    Arithmetic::Multiply => left.mul(&right),
                });
            }
            IntervalFrame::Negate => {
                let operand = self.values.pop().unwrap_or_else(Interval::unbounded);
                self.values.push(operand.negate());
            }
        }
        ControlFlow::Continue(())
    }
}

/// [`Definedness::outcomes`]'s walk: each condition's exit leaves its
/// outcomes on `results`, and the toolkit enters a connective's right
/// operand only after its left operand has been exited, so the right
/// operand's facts are read from the left's outcomes at the moment it is
/// entered.
struct OutcomeWalk<'d, 'a, 'n> {
    definedness: &'d Definedness<'a>,
    results: Vec<Outcomes>,
    nodes: PhantomData<CheckedNode<'n>>,
}

/// A node of an outcome walk.
enum OutcomeNode<'n> {
    /// Evaluate a condition's outcomes under these facts.
    Evaluate(CheckedNode<'n>, Rc<Facts>),
    /// The right operand of a connective whose left operand's outcomes are
    /// on `results`.
    Right(Connective, CheckedNode<'n>),
}

/// What an outcome walk keeps for an entered node.
enum OutcomeFrame {
    /// The node's outcomes are already on `results`.
    Done,
    /// Swap the outcomes of the operand: a `not`.
    Negate,
    /// Combine the left and right outcomes below the top of `results`.
    Connective(Connective),
}

impl<'n> quire_walk::Walk for OutcomeWalk<'_, '_, 'n> {
    type Node = OutcomeNode<'n>;
    type Frame = OutcomeFrame;
    type Stop = Infallible;

    fn enter(
        &mut self,
        node: OutcomeNode<'n>,
        children: &mut quire_walk::Children<'_, OutcomeNode<'n>>,
    ) -> ControlFlow<Infallible, OutcomeFrame> {
        let definedness = self.definedness;
        match node {
            OutcomeNode::Evaluate(node, facts) => match node.kind() {
                NodeKind::Not(operand) => {
                    children.push(OutcomeNode::Evaluate(node.at(*operand), facts));
                    ControlFlow::Continue(OutcomeFrame::Negate)
                }
                // FR-104: `pre(c)` holds exactly when `c` holds at `pre`;
                // the facts `c` establishes are keyed by the observations
                // its own reads carry.
                NodeKind::Pre(operand) if definedness.observations.is_some() => {
                    children.push(OutcomeNode::Evaluate(node.at(*operand), facts));
                    ControlFlow::Continue(OutcomeFrame::Done)
                }
                NodeKind::Connective(connective, left, right) => {
                    children.push(OutcomeNode::Evaluate(node.at(*left), facts));
                    children.push(OutcomeNode::Right(*connective, node.at(*right)));
                    ControlFlow::Continue(OutcomeFrame::Connective(*connective))
                }
                _ => {
                    self.results.push(definedness.guard(node, &facts));
                    ControlFlow::Continue(OutcomeFrame::Done)
                }
            },
            OutcomeNode::Right(connective, right) => {
                let (left_true, left_false) = self.results.last().cloned().unwrap_or_default();
                let under = match connective {
                    Connective::And | Connective::Implies => left_true,
                    Connective::Or => left_false,
                };
                match under {
                    Some(under) => children.push(OutcomeNode::Evaluate(right, under)),
                    None => self.results.push((None, None)),
                }
                ControlFlow::Continue(OutcomeFrame::Done)
            }
        }
    }

    fn exit(&mut self, frame: OutcomeFrame) -> ControlFlow<Infallible> {
        match frame {
            OutcomeFrame::Done => {}
            OutcomeFrame::Negate => {
                let (when_true, when_false) = self.results.pop().unwrap_or_default();
                self.results.push((when_false, when_true));
            }
            OutcomeFrame::Connective(connective) => {
                let (right_true, right_false) = self.results.pop().unwrap_or_default();
                let (left_true, left_false) = self.results.pop().unwrap_or_default();
                self.results.push(match connective {
                    Connective::And => (right_true, join(left_false, right_false)),
                    Connective::Or => (join(left_true, right_true), right_false),
                    Connective::Implies => (join(left_false, right_true), right_false),
                });
            }
        }
        ControlFlow::Continue(())
    }
}

/// One step of a stable path, before the facts it is checked against are
/// applied root first.
enum PathStep {
    Field(usize),
    Value,
    Attribute(Step),
}

/// Whether `node` is an integer.
fn is_integer(node: CheckedNode<'_>) -> bool {
    matches!(node.value_type(), ValueType::Integer | ValueType::Int(_))
}

/// What a guard's true outcome establishes about a field, as
/// [`established_field_fact`] reports it to `crate::model`'s FR-151
/// refinement-obligation check. Both facts are carried together — a
/// conjunction of clauses can establish presence and an interval at once —
/// rather than an either/or shape that would force a caller folding several
/// clauses into one guard to pick only one of them.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Established {
    /// Whether `present(self.<field>)` held on the true outcome.
    pub(crate) presence: bool,
    /// The proved integer interval on `self.<field>` on the true outcome,
    /// if any.
    pub(crate) interval: Option<ProvedInterval>,
}

/// Derives the facts `condition`'s true outcome establishes about
/// `self.<field>` (`self` bound at `self_slot`, `field` projected at stable
/// path step zero), by running the exact guard-fact propagation `walk`
/// itself uses (`outcomes`), not a bespoke re-implementation. Both the
/// presence and interval facts are read from the one resulting [`Facts`],
/// so a caller that folds several clauses into one `condition` (an `AND`
/// tree) gets everything that conjunction actually proves, not only
/// whichever fact form happened to be checked first. The default
/// (`Established::default()`, i.e. neither fact) when the true outcome is
/// unreachable or proves nothing about that path.
///
/// `crate::model` has no FR-146 expression parser, so `condition` is not
/// parsed from producer-supplied source; the caller
/// (`crate::model::conformance`) builds the small typed guard tree the
/// accepted `PostconditionClause` forms describe and hands it here, so what
/// they actually prove is decided by this same arithmetic a real checked
/// postcondition already runs, never restated from a clause's own literal.
pub(crate) fn established_field_fact(condition: CheckedNode<'_>, self_slot: Slot) -> Established {
    let checker = Definedness::new(self_slot + 1, &[], &[]);
    let (when_true, _) = checker.outcomes(condition, &Rc::new(Facts::default()));
    let Some(facts) = when_true else {
        return Established::default();
    };
    let path = StablePath {
        root: self_slot,
        steps: vec![Step::Field(0)],
    };
    Established {
        presence: facts.present.contains(&path),
        interval: facts.intervals.get(&Subject::Value(path)).cloned(),
    }
}

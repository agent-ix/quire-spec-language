// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 static definedness: stable paths, guard facts, interval proofs and
//! the call sites termination checking consumes.
//!
//! Facts are only ever added by the accepted guard forms: `present(p)` for a
//! stable path `p`, and an integer ordering or equality between a stable
//! integer path (or `size(p)`) and an integer literal. A comparison of two
//! non-literal operands yields no fact. The walk recurses over a typed tree
//! whose depth the checking limits already bound.

use std::collections::{BTreeMap, BTreeSet};

use super::check::DispatchOperation;
use super::ir::{Arithmetic, Connective, DispatchTable, Node, NodeKind, OrderedKind, Slot, Visit};
use super::refusal::{
    CheckCause, CheckRefusal, InvalidDispatchDeclaration, Location, Obligation, ProvedInterval,
};
use crate::value::collection::CollectionType;
use crate::value::composite::{Value, ValueType};
use crate::value::equality::EqualityOperator;
use crate::value::numeric::{ArithmeticOperator, OrderingOperator};
use crate::value::rational::rational_domain_result_of;
use quire_exact::{Integer, IntegerInterval};

/// One step of a stable path.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum Step {
    Field(usize),
    Value,
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

/// The facts proved on one reachable path.
#[derive(Clone, Debug, Default)]
struct Facts {
    intervals: BTreeMap<Subject, ProvedInterval>,
    present: BTreeSet<StablePath>,
    nonzero: BTreeSet<StablePath>,
    aliases: BTreeMap<Slot, StablePath>,
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
    let bound = collection.bound();
    Interval {
        lower: End::Finite(Integer::from(bound.minimum())),
        upper: End::Finite(Integer::from(bound.maximum())),
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
fn join(left: Option<Facts>, right: Option<Facts>) -> Option<Facts> {
    match (left, right) {
        (None, other) | (other, None) => other,
        (Some(left), Some(right)) => Some(Facts {
            intervals: left
                .intervals
                .iter()
                .filter_map(|(subject, interval)| {
                    right.intervals.get(subject).map(|other| {
                        let hull = Interval::of_proved(interval).hull(&Interval::of_proved(other));
                        (subject.clone(), hull.proved())
                    })
                })
                .collect(),
            present: left.present.intersection(&right.present).cloned().collect(),
            nonzero: left.nonzero.intersection(&right.nonzero).cloned().collect(),
            aliases: left.aliases,
        }),
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
        }
    }

    /// Check every obligation on a path that can execute.
    pub(crate) fn check(&mut self, root: &Node) -> Result<(), CheckRefusal> {
        self.walk(root, &Facts::default())
    }

    fn stable_path(&self, node: &Node, facts: &Facts) -> Option<StablePath> {
        match &node.kind {
            NodeKind::Local(slot) => Some(facts.aliases.get(slot).cloned().unwrap_or(StablePath {
                root: *slot,
                steps: Vec::new(),
            })),
            NodeKind::Field { operand, index, .. } => {
                let mut path = self.stable_path(operand, facts)?;
                path.steps.push(Step::Field(*index));
                Some(path)
            }
            NodeKind::Value(operand) => {
                let mut path = self.stable_path(operand, facts)?;
                if !facts.present.contains(&path) {
                    return None;
                }
                path.steps.push(Step::Value);
                Some(path)
            }
            _ => None,
        }
    }

    fn subject(&self, node: &Node, facts: &Facts) -> Option<Subject> {
        match &node.kind {
            NodeKind::Coerce(operand, _) => self.subject(operand, facts),
            NodeKind::Size(operand) => self.stable_path(operand, facts).map(Subject::Size),
            _ if matches!(node.value_type, ValueType::Integer | ValueType::Int(_)) => {
                self.stable_path(node, facts).map(Subject::Value)
            }
            _ => None,
        }
    }

    fn interval(&self, node: &Node, facts: &Facts) -> Interval {
        let fact = |subject: Subject, fallback: Interval| {
            facts
                .intervals
                .get(&subject)
                .map_or(fallback, Interval::of_proved)
        };
        match &node.kind {
            NodeKind::Literal(Value::Integer(value)) => Interval::point(value),
            NodeKind::Arithmetic(operator, left, right) => {
                let (left, right) = (self.interval(left, facts), self.interval(right, facts));
                match operator {
                    Arithmetic::Add => left.add(&right),
                    Arithmetic::Subtract => left.add(&right.negate()),
                    Arithmetic::Multiply => left.mul(&right),
                }
            }
            NodeKind::Negate(operand) => self.interval(operand, facts).negate(),
            NodeKind::Coerce(_, target) => Interval::of_domain(target),
            NodeKind::Size(operand) => {
                let bound = match &operand.value_type {
                    ValueType::Collection(collection) => cardinality(collection),
                    _ => Interval::unbounded(),
                };
                match self.stable_path(operand, facts) {
                    Some(path) => fact(Subject::Size(path), bound),
                    None => bound,
                }
            }
            _ => match self.stable_path(node, facts) {
                Some(path) => fact(Subject::Value(path), declared(&node.value_type)),
                None => declared(&node.value_type),
            },
        }
    }

    fn literal(node: &Node) -> Option<Integer> {
        match &node.kind {
            NodeKind::Literal(Value::Integer(value)) => Some(value.clone()),
            NodeKind::Negate(operand) => Self::literal(operand).map(|value| value.neg()),
            NodeKind::Coerce(operand, _) => Self::literal(operand),
            _ => None,
        }
    }

    /// Apply `subject R k` to `facts`, or `None` when it is unsatisfiable.
    fn relate(
        &self,
        subject: &Node,
        relation: Relation,
        literal: &Integer,
        facts: &Facts,
    ) -> Option<Facts> {
        let Some(key) = self.subject(subject, facts) else {
            return Some(facts.clone());
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
        let mut facts = facts.clone();
        if let (Relation::NotEqual, true, Subject::Value(path)) =
            (relation, literal.is_zero(), &key)
        {
            facts.nonzero.insert(path.clone());
        }
        facts.intervals.insert(key, refined.proved());
        Some(facts)
    }

    fn comparison(
        &self,
        left: &Node,
        right: &Node,
        relation: Relation,
        facts: &Facts,
    ) -> (Option<Facts>, Option<Facts>) {
        let (subject, relation, literal) = match (Self::literal(left), Self::literal(right)) {
            (None, Some(literal)) => (left, relation, literal),
            (Some(literal), None) => (right, relation.flipped(), literal),
            (Some(_), Some(_)) | (None, None) => {
                return (Some(facts.clone()), Some(facts.clone()));
            }
        };
        (
            self.relate(subject, relation, &literal, facts),
            self.relate(subject, relation.negated(), &literal, facts),
        )
    }

    /// The facts on the true and false outcomes of a reachable condition.
    fn outcomes(&self, condition: &Node, facts: &Facts) -> (Option<Facts>, Option<Facts>) {
        match &condition.kind {
            NodeKind::Literal(Value::Boolean(true)) => (Some(facts.clone()), None),
            NodeKind::Literal(Value::Boolean(false)) => (None, Some(facts.clone())),
            NodeKind::Not(operand) => {
                let (when_true, when_false) = self.outcomes(operand, facts);
                (when_false, when_true)
            }
            NodeKind::Present(operand) => match self.stable_path(operand, facts) {
                Some(path) => {
                    let mut when_true = facts.clone();
                    when_true.present.insert(path);
                    (Some(when_true), Some(facts.clone()))
                }
                None => (Some(facts.clone()), Some(facts.clone())),
            },
            NodeKind::Order(operator, OrderedKind::Integers, left, right) => {
                let relation = match operator {
                    OrderingOperator::Less => Relation::Less,
                    OrderingOperator::LessOrEqual => Relation::LessOrEqual,
                    OrderingOperator::Greater => Relation::Greater,
                    OrderingOperator::GreaterOrEqual => Relation::GreaterOrEqual,
                };
                self.comparison(left, right, relation, facts)
            }
            NodeKind::Equality(operator, _, left, right)
                if matches!(left.value_type, ValueType::Integer | ValueType::Int(_))
                    && matches!(right.value_type, ValueType::Integer | ValueType::Int(_)) =>
            {
                let relation = match operator {
                    EqualityOperator::Equal => Relation::Equal,
                    EqualityOperator::NotEqual => Relation::NotEqual,
                };
                self.comparison(left, right, relation, facts)
            }
            NodeKind::Connective(connective, left, right) => {
                let (left_true, left_false) = self.outcomes(left, facts);
                let right_under = |facts: Option<Facts>| match facts {
                    Some(facts) => self.outcomes(right, &facts),
                    None => (None, None),
                };
                match connective {
                    Connective::And => {
                        let (right_true, right_false) = right_under(left_true);
                        (right_true, join(left_false, right_false))
                    }
                    Connective::Or => {
                        let (right_true, right_false) = right_under(left_false);
                        (join(left_true, right_true), right_false)
                    }
                    Connective::Implies => {
                        let (right_true, right_false) = right_under(left_true);
                        (join(left_false, right_true), right_false)
                    }
                }
            }
            _ => (Some(facts.clone()), Some(facts.clone())),
        }
    }

    fn refuse(node: &Node, obligation: Obligation) -> CheckRefusal {
        CheckRefusal {
            location: node.location.clone(),
            cause: CheckCause::Unproved(obligation),
        }
    }

    fn shape(&self, argument: &Node, facts: &Facts) -> ArgumentShape {
        let argument = match &argument.kind {
            NodeKind::Coerce(operand, _) => operand,
            _ => argument,
        };
        let parameter = |node: &Node| match node.kind {
            NodeKind::Local(slot) if slot < self.parameters => Some(slot),
            _ => None,
        };
        if let Some(slot) = parameter(argument) {
            return ArgumentShape::Parameter(slot);
        }
        if let NodeKind::Arithmetic(Arithmetic::Subtract, left, right) = &argument.kind {
            if let (Some(slot), Some(decrement)) = (parameter(left), Self::literal(right)) {
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

    fn walk(&mut self, node: &Node, facts: &Facts) -> Result<(), CheckRefusal> {
        match &node.kind {
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => {
                self.walk(condition, facts)?;
                let (when_true, when_false) = self.outcomes(condition, facts);
                if let Some(when_true) = when_true {
                    self.walk(then, &when_true)?;
                }
                if let Some(when_false) = when_false {
                    self.walk(otherwise, &when_false)?;
                }
                Ok(())
            }
            NodeKind::Connective(connective, left, right) => {
                self.walk(left, facts)?;
                let (when_true, when_false) = self.outcomes(left, facts);
                let under = match connective {
                    Connective::And | Connective::Implies => when_true,
                    Connective::Or => when_false,
                };
                match under {
                    Some(under) => self.walk(right, &under),
                    None => Ok(()),
                }
            }
            NodeKind::Let { slot, value, body } => {
                self.walk(value, facts)?;
                let mut inner = facts.clone();
                if let Some(path) = self.stable_path(value, facts) {
                    inner.aliases.insert(*slot, path);
                }
                self.walk(body, &inner)
            }
            NodeKind::Coerce(operand, target) => {
                self.walk(operand, facts)?;
                let proved = self.interval(operand, facts);
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
                self.walk(left, facts)?;
                self.walk(right, facts)?;
                let divisor = self.interval(right, facts);
                let nonzero = divisor.excludes_zero()
                    || self
                        .stable_path(right, facts)
                        .is_some_and(|path| facts.nonzero.contains(&path));
                if !nonzero {
                    return Err(Self::refuse(node, Obligation::Nonzero));
                }
                let dividend = self.interval(left, facts);
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
                self.walk(left, facts)?;
                self.walk(right, facts)?;
                let (ValueType::Rational(left), ValueType::Rational(right)) =
                    (&left.value_type, &right.value_type)
                else {
                    return Err(Self::refuse(node, Obligation::RationalRange));
                };
                if *operator == ArithmeticOperator::Divide && !right.excludes_zero() {
                    return Err(Self::refuse(node, Obligation::Nonzero));
                }
                if domain.contains_domain(&rational_domain_result_of(left, *operator, right)) {
                    Ok(())
                } else {
                    Err(Self::refuse(node, Obligation::RationalRange))
                }
            }
            NodeKind::RationalNegate(operand, domain) => {
                self.walk(operand, facts)?;
                match &operand.value_type {
                    ValueType::Rational(source) if domain.contains_domain(&source.negated()) => {
                        Ok(())
                    }
                    _ => Err(Self::refuse(node, Obligation::RationalRange)),
                }
            }
            NodeKind::Decimal {
                operator: ArithmeticOperator::Divide,
                left,
                right,
                ..
            } => {
                self.walk(left, facts)?;
                self.walk(right, facts)?;
                let zero = Integer::zero();
                match &right.value_type {
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
            NodeKind::Quantity(ArithmeticOperator::Divide, left, right) => {
                self.walk(left, facts)?;
                self.walk(right, facts)?;
                Err(Self::refuse(node, Obligation::Nonzero))
            }
            NodeKind::Query {
                visit: Visit::Sum,
                source,
                body,
                ..
            } => {
                self.walk(source, facts)?;
                self.walk(body, facts)?;
                let ValueType::Int(domain) = &node.value_type else {
                    return Ok(());
                };
                let size = match &source.value_type {
                    ValueType::Collection(collection) => cardinality(collection),
                    _ => Interval::unbounded(),
                };
                let proved = prefix_sums(&self.interval(body, facts), &size);
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
            NodeKind::Value(operand) => {
                self.walk(operand, facts)?;
                match self.stable_path(operand, facts) {
                    Some(path) if facts.present.contains(&path) => Ok(()),
                    _ => Err(Self::refuse(node, Obligation::Presence)),
                }
            }
            NodeKind::Fold {
                source,
                step,
                identity,
                ..
            } => {
                self.walk(source, facts)?;
                if let Some(identity) = identity {
                    self.walk(identity, facts)?;
                } else {
                    let bound = match &source.value_type {
                        ValueType::Collection(collection) => cardinality(collection),
                        _ => Interval::unbounded(),
                    };
                    let size = match self.stable_path(source, facts) {
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
                }
                self.walk(step, facts)
            }
            NodeKind::Call {
                identity: _,
                function,
                arguments,
            } => {
                for argument in arguments {
                    self.walk(argument, facts)?;
                }
                let arguments = arguments
                    .iter()
                    .map(|argument| self.shape(argument, facts))
                    .collect();
                self.calls.push(CallSite {
                    callee: *function,
                    location: node.location.clone(),
                    arguments,
                    kind: EdgeKind::Ordinary,
                });
                Ok(())
            }
            NodeKind::Dispatch {
                receiver,
                table,
                operation,
                arguments,
            } => {
                self.walk(receiver, facts)?;
                for argument in arguments {
                    self.walk(argument, facts)?;
                }
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
                        location: node.location.clone(),
                        cause: CheckCause::InvalidDispatchDeclaration(
                            InvalidDispatchDeclaration::OperationOutOfRange {
                                operation: operation_index,
                            },
                        ),
                    });
                };
                let callees = self
                    .dispatch_tables
                    .get(table_index)
                    .map(DispatchTable::callees)
                    .ok_or_else(|| CheckRefusal {
                        location: node.location.clone(),
                        cause: CheckCause::InvalidDispatchDeclaration(
                            InvalidDispatchDeclaration::TableOutOfRange {
                                member: declared_operation.member.clone(),
                                table: table_index,
                            },
                        ),
                    })?;
                for callee in callees {
                    self.calls.push(CallSite {
                        callee,
                        location: node.location.clone(),
                        arguments: Vec::new(),
                        kind: EdgeKind::Dispatch,
                    });
                }
                Ok(())
            }
            _ => {
                for child in node.children() {
                    self.walk(child, facts)?;
                }
                Ok(())
            }
        }
    }
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
pub(crate) fn established_field_fact(condition: &Node, self_slot: Slot) -> Established {
    let checker = Definedness::new(self_slot + 1, &[], &[]);
    let (when_true, _) = checker.outcomes(condition, &Facts::default());
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

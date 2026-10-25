// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338's `["safety", branches]` state algebra (ADR-018 LA-2, §12): the
//! typed elementary configuration `[obligations, past, windows, acceptance]`,
//! its canonical sorted-set writer, and the reader that admits an imported
//! state against the checked formula it must resolve in.
//!
//! Every set is written sorted by complete canonical element octets and
//! without duplicates, so the key is a function of the facts alone and never
//! of the order a subset construction or a window insertion produced them. A
//! reader refuses an unsorted or duplicated set rather than repairing it.
//! Operand paths and operand vectors are ordered vectors and are never
//! sorted.

use std::collections::BTreeMap;

use qsl_foundation::bound::TemporalUpper;
use qsl_semantics::check::{
    BinaryTemporal, CheckedFormula, CheckedInterval, FormulaNode, UnaryTemporal,
};
use quire_canonical::{Limits, Node, NodeRef};

use super::key::{ascending, canonical, decimal, decimal_wide, exactly, KeyRefusal};
use crate::simulation::model::encode::Enc;

/// The most operand paths a formula shape indexes: a shared checked node at
/// several paths is several occurrences, and the count is bounded.
const MAX_PATHS: usize = 1 << 16;

/// The polarity of a signed occurrence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Polarity {
    /// `"positive"`.
    Positive,
    /// `"negative"`: the occurrence's dual.
    Negative,
}

impl Polarity {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
        }
    }
}

/// A signed occurrence `[o, s]`: an operand path from the checked formula's
/// root (the root is `[]`) and the polarity applied to the whole subformula.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Occurrence {
    /// The operand path: child positions in checked operand order.
    pub path: Vec<u64>,
    /// The polarity.
    pub polarity: Polarity,
}

impl Occurrence {
    /// The occurrence at `path` with `polarity`.
    pub fn new(path: Vec<u64>, polarity: Polarity) -> Self {
        Self { path, polarity }
    }

    fn enc(&self) -> Vec<Enc> {
        vec![path_enc(&self.path), Enc::text(self.polarity.as_str())]
    }
}

/// An obligation of an elementary configuration.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Obligation {
    /// `[o,s]`: an unbounded or current subformula.
    Current(Occurrence),
    /// `[o,s,lower,upper]`: an open interval obligation, in remaining
    /// offsets; `upper` is `None` for `"open"`.
    Interval {
        /// The signed occurrence.
        occurrence: Occurrence,
        /// The remaining lower offset.
        lower: u64,
        /// The remaining upper offset, or `None` when open.
        upper: Option<u64>,
    },
}

impl Obligation {
    fn occurrence(&self) -> &Occurrence {
        match self {
            Self::Current(occurrence) | Self::Interval { occurrence, .. } => occurrence,
        }
    }

    fn enc(&self) -> Enc {
        match self {
            Self::Current(occurrence) => Enc::Array(occurrence.enc()),
            Self::Interval {
                occurrence,
                lower,
                upper,
            } => {
                let mut members = occurrence.enc();
                members.push(number_enc(*lower));
                members.push(upper.map_or_else(|| Enc::text("open"), number_enc));
                Enc::Array(members)
            }
        }
    }
}

/// A `windows` entry: the canonical storage of one signed future interval
/// occurrence's open obligations.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Window {
    /// `[o,s,"deadline",d]`: the earliest remaining deadline of a lower-zero
    /// `F`/`U` with no interval in an operand.
    Deadline(Occurrence, u64),
    /// `[o,s,"extent",d]`: the furthest required remaining position of the
    /// same shape for `G`/`R`.
    Extent(Occurrence, u64),
    /// `[o,s,"offsets",pairs]`: the set of `[lower,upper]` remaining offsets
    /// of every other finite interval shape.
    Offsets(Occurrence, Vec<(u64, u64)>),
    /// `[o,s,"shift",d]`: an `[a,open]` occurrence before its lower shift
    /// completes.
    Shift(Occurrence, u64),
}

impl Window {
    fn occurrence(&self) -> &Occurrence {
        match self {
            Self::Deadline(occurrence, _)
            | Self::Extent(occurrence, _)
            | Self::Offsets(occurrence, _)
            | Self::Shift(occurrence, _) => occurrence,
        }
    }

    fn enc(&self, max_bytes: u64) -> Result<Enc, KeyRefusal> {
        let (occurrence, kind, payload) = match self {
            Self::Deadline(occurrence, remaining) => {
                (occurrence, "deadline", number_enc(*remaining))
            }
            Self::Extent(occurrence, remaining) => (occurrence, "extent", number_enc(*remaining)),
            Self::Shift(occurrence, remaining) => (occurrence, "shift", number_enc(*remaining)),
            Self::Offsets(occurrence, pairs) => {
                let pairs = pairs
                    .iter()
                    .map(|(lower, upper)| Enc::Array(vec![number_enc(*lower), number_enc(*upper)]))
                    .collect();
                (occurrence, "offsets", sorted_set(pairs, max_bytes)?)
            }
        };
        let mut members = occurrence.enc();
        members.push(Enc::text(kind));
        members.push(payload);
        Ok(Enc::Array(members))
    }
}

/// The past operator a memory entry tracks.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PastOperator {
    /// `once`.
    Once,
    /// `historically`.
    Historically,
    /// `since`.
    Since,
    /// `triggered`.
    Triggered,
}

impl PastOperator {
    /// The operator's operand count.
    const fn arity(self) -> usize {
        match self {
            Self::Once | Self::Historically => 1,
            Self::Since | Self::Triggered => 2,
        }
    }

    /// The tag of the unbounded flag form.
    const fn flag(self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Historically => "historically",
            Self::Since => "since",
            Self::Triggered => "triggered",
        }
    }

    /// The tag of the lower-zero finite age form.
    const fn age(self) -> &'static str {
        match self {
            Self::Once => "last-true",
            Self::Historically => "last-false",
            Self::Since => "since-age",
            Self::Triggered => "triggered-age",
        }
    }
}

/// One operand of a `history` or `shift-history` entry: its path and the set
/// of its true offsets.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct HistoryOperand {
    /// The operand's occurrence path.
    pub path: Vec<u64>,
    /// The offsets, from the next position, at which the operand was true.
    pub offsets: Vec<u64>,
}

impl HistoryOperand {
    fn enc(&self, max_bytes: u64) -> Result<Enc, KeyRefusal> {
        let offsets = self.offsets.iter().map(|offset| number_enc(*offset));
        Ok(Enc::Array(vec![
            path_enc(&self.path),
            sorted_set(offsets.collect(), max_bytes)?,
        ]))
    }
}

/// A `past` entry: the previous-position memory of one past occurrence.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PastMemory {
    /// `[o,"once",seen_true]` and its three siblings: the Boolean flag of an
    /// unbounded past operator.
    Flag {
        /// The occurrence path.
        path: Vec<u64>,
        /// The operator.
        operator: PastOperator,
        /// The flag.
        value: bool,
    },
    /// `[o,"last-true",age]` and its three siblings: the age counter of a
    /// finite lower-zero past operator, saturated at `b+1`.
    Age {
        /// The occurrence path.
        path: Vec<u64>,
        /// The operator.
        operator: PastOperator,
        /// The counted distance since the tracked truth, saturated at `b+1`.
        age: u128,
    },
    /// `[o,"history",operands]`: the true offsets `1..b` of every operand of
    /// another finite past interval.
    History {
        /// The occurrence path.
        path: Vec<u64>,
        /// One entry per operand, in operand order.
        operands: Vec<HistoryOperand>,
    },
    /// `[o,"shift-history",operands,tail]`: the true offsets `1..a` of every
    /// operand of a past `[a,open]`, and the older-positions flag.
    ShiftHistory {
        /// The occurrence path.
        path: Vec<u64>,
        /// One entry per operand, in operand order.
        operands: Vec<HistoryOperand>,
        /// The unbounded recurrence's result over the positions before the
        /// retained window.
        tail: bool,
    },
}

impl PastMemory {
    fn path(&self) -> &[u64] {
        match self {
            Self::Flag { path, .. }
            | Self::Age { path, .. }
            | Self::History { path, .. }
            | Self::ShiftHistory { path, .. } => path,
        }
    }

    fn enc(&self, max_bytes: u64) -> Result<Enc, KeyRefusal> {
        let operands = |operands: &[HistoryOperand]| -> Result<Enc, KeyRefusal> {
            Ok(Enc::Array(
                operands
                    .iter()
                    .map(|operand| operand.enc(max_bytes))
                    .collect::<Result<_, _>>()?,
            ))
        };
        let members = match self {
            Self::Flag {
                path,
                operator,
                value,
            } => vec![
                path_enc(path),
                Enc::text(operator.flag()),
                Enc::Bool(*value),
            ],
            Self::Age {
                path,
                operator,
                age,
            } => vec![
                path_enc(path),
                Enc::text(operator.age()),
                Enc::text(age.to_string()),
            ],
            Self::History {
                path,
                operands: held,
            } => vec![path_enc(path), Enc::text("history"), operands(held)?],
            Self::ShiftHistory {
                path,
                operands: held,
                tail,
            } => vec![
                path_enc(path),
                Enc::text("shift-history"),
                operands(held)?,
                Enc::Bool(*tail),
            ],
        };
        Ok(Enc::Array(members))
    }
}

/// One elementary configuration `[obligations, past, windows, acceptance]`
/// of a safety state. The empty configuration is true. Each set may be given
/// in any order: [`SafetyStates::key`](super::SafetyStates::key) writes it sorted.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Configuration {
    /// The conjunction of obligations.
    pub obligations: Vec<Obligation>,
    /// The previous-position memory of every past occurrence.
    pub past: Vec<PastMemory>,
    /// One entry per signed future interval occurrence with an open
    /// obligation.
    pub windows: Vec<Window>,
    /// The least-fixed-point occurrences whose acceptance set this
    /// configuration belongs to.
    pub acceptance: Vec<Occurrence>,
}

impl Configuration {
    fn enc(&self, max_bytes: u64) -> Result<Enc, KeyRefusal> {
        let obligations = self.obligations.iter().map(Obligation::enc).collect();
        let past = self
            .past
            .iter()
            .map(|entry| entry.enc(max_bytes))
            .collect::<Result<_, _>>()?;
        let windows = self
            .windows
            .iter()
            .map(|entry| entry.enc(max_bytes))
            .collect::<Result<_, _>>()?;
        let acceptance = self
            .acceptance
            .iter()
            .map(|entry| Enc::Array(entry.enc()))
            .collect();
        Ok(Enc::Array(vec![
            sorted_set(obligations, max_bytes)?,
            sorted_set(past, max_bytes)?,
            sorted_set(windows, max_bytes)?,
            sorted_set(acceptance, max_bytes)?,
        ]))
    }
}

/// `["safety", branches]` for `branches` written as a sorted duplicate-free
/// set, each configuration with its own sets sorted.
pub(super) fn safety_enc(branches: &[Configuration], max_bytes: u64) -> Result<Enc, KeyRefusal> {
    let branches = normal(branches)
        .iter()
        .map(|branch| branch.enc(max_bytes))
        .collect::<Result<_, _>>()?;
    Ok(Enc::Array(vec![
        Enc::text("safety"),
        sorted_set(branches, max_bytes)?,
    ]))
}

/// A decimal-string unsigned integer.
fn number_enc(value: u64) -> Enc {
    Enc::text(value.to_string())
}

fn path_enc(path: &[u64]) -> Enc {
    Enc::Array(path.iter().map(|step| number_enc(*step)).collect())
}

/// `items` as an array sorted by complete canonical octets, without
/// duplicates.
fn sorted_set(items: Vec<Enc>, max_bytes: u64) -> Result<Enc, KeyRefusal> {
    let mut keyed = Vec::with_capacity(items.len());
    for item in items {
        let octets = quire_canonical::to_vec(&item, Limits::new(max_bytes))
            .map_err(|_| KeyRefusal::Encoding)?;
        keyed.push((octets, item));
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    keyed.dedup_by(|left, right| left.0 == right.0);
    Ok(Enc::Array(
        keyed.into_iter().map(|(_, item)| item).collect(),
    ))
}

/// A temporal operator of the checked formula.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Op {
    Eventually,
    Always,
    Until,
    Release,
    Past(PastOperator),
}

/// A real operator interval: `[0,*]` is the unbounded operator and is
/// recorded as no interval.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Bounds {
    lower: u64,
    upper: Option<u64>,
}

/// One occurrence of the checked formula.
#[derive(Clone, Debug)]
struct Shaped {
    op: Option<Op>,
    interval: Option<Bounds>,
    /// Whether an operand's subtree holds an interval operator.
    nested_interval: bool,
    /// Whether the occurrence is read at positive and at negative polarity
    /// from a positive root.
    positive: bool,
    negative: bool,
}

/// The checked formula as the operand paths FR-338 resolves members in.
#[derive(Clone, Debug)]
pub(super) struct FormulaShape {
    nodes: BTreeMap<Vec<u64>, Shaped>,
}

/// How a past occurrence's interval selects its memory form.
enum PastClass {
    Unbounded,
    Age(u64),
    History(u64),
    Shift(u64),
}

impl FormulaShape {
    /// The shape of `formula`, or `None` when it holds a timed interval or
    /// more occurrences than the path bound.
    pub(super) fn of(formula: &CheckedFormula) -> Option<Self> {
        struct Entry {
            path: Vec<u64>,
            node: usize,
            parent: Option<usize>,
            signs: (bool, bool),
        }
        let nodes = formula.nodes();
        let mut entries = vec![Entry {
            path: Vec::new(),
            node: formula.root().index(),
            parent: None,
            signs: (true, false),
        }];
        let mut next = 0_usize;
        while next < entries.len() {
            let (path, node, signs) = {
                let entry = entries.get(next)?;
                (entry.path.clone(), entry.node, entry.signs)
            };
            let parent = nodes.get(node)?;
            for (position, child) in parent.operands().enumerate() {
                // `not` and an implication's antecedent dualize the operand.
                let flips = matches!(parent, FormulaNode::Not(_))
                    || (position == 0 && matches!(parent, FormulaNode::Implies(..)));
                if entries.len() >= MAX_PATHS {
                    return None;
                }
                let mut child_path = path.clone();
                child_path.push(u64::try_from(position).ok()?);
                entries.push(Entry {
                    path: child_path,
                    node: child.index(),
                    parent: Some(next),
                    signs: if flips { (signs.1, signs.0) } else { signs },
                });
            }
            next += 1;
        }
        let mut shaped = Vec::with_capacity(entries.len());
        for entry in &entries {
            let mut node = shape_of(nodes.get(entry.node)?)?;
            node.positive = entry.signs.0;
            node.negative = entry.signs.1;
            shaped.push(node);
        }
        // An operand precedes no operator in the breadth-first order, so one
        // reverse pass folds every subtree.
        let mut holds_interval: Vec<bool> =
            shaped.iter().map(|node| node.interval.is_some()).collect();
        for index in (0..entries.len()).rev() {
            let reaches = *holds_interval.get(index)?;
            if let Some(parent) = entries.get(index)?.parent {
                if reaches {
                    *holds_interval.get_mut(parent)? = true;
                    shaped.get_mut(parent)?.nested_interval = true;
                }
            }
        }
        Some(Self {
            nodes: entries
                .into_iter()
                .map(|entry| entry.path)
                .zip(shaped)
                .collect(),
        })
    }

    fn get(&self, path: &[u64]) -> Option<&Shaped> {
        self.nodes.get(path)
    }
}

fn shape_of(node: &FormulaNode) -> Option<Shaped> {
    let (op, interval) = match node {
        FormulaNode::Unary {
            operator, interval, ..
        } => (
            Some(match operator {
                UnaryTemporal::Eventually => Op::Eventually,
                UnaryTemporal::Always => Op::Always,
                UnaryTemporal::Once => Op::Past(PastOperator::Once),
                UnaryTemporal::Historically => Op::Past(PastOperator::Historically),
            }),
            interval,
        ),
        FormulaNode::Binary {
            operator, interval, ..
        } => (
            Some(match operator {
                BinaryTemporal::Until => Op::Until,
                BinaryTemporal::Release => Op::Release,
                BinaryTemporal::Since => Op::Past(PastOperator::Since),
                BinaryTemporal::Triggered => Op::Past(PastOperator::Triggered),
            }),
            interval,
        ),
        _ => (None, &None),
    };
    let interval = match interval {
        None => None,
        Some(CheckedInterval::Timed(_)) => return None,
        Some(CheckedInterval::Steps(steps)) => {
            let upper = match steps.upper() {
                TemporalUpper::Finite(upper) => Some(upper),
                TemporalUpper::Open => None,
            };
            (steps.lower() != 0 || upper.is_some()).then_some(Bounds {
                lower: steps.lower(),
                upper,
            })
        }
    };
    Some(Shaped {
        op,
        interval,
        nested_interval: false,
        positive: false,
        negative: false,
    })
}

impl Shaped {
    /// Whether the occurrence is read at `polarity`.
    fn reached(&self, polarity: Polarity) -> bool {
        match polarity {
            Polarity::Positive => self.positive,
            Polarity::Negative => self.negative,
        }
    }

    /// Whether the occurrence is a future temporal operator.
    fn is_future(&self) -> bool {
        matches!(
            self.op,
            Some(Op::Eventually | Op::Always | Op::Until | Op::Release)
        )
    }

    fn past_class(&self) -> Option<(PastOperator, PastClass)> {
        let Some(Op::Past(operator)) = self.op else {
            return None;
        };
        let class = match self.interval {
            None => PastClass::Unbounded,
            Some(Bounds {
                lower: 0,
                upper: Some(bound),
            }) => PastClass::Age(bound),
            Some(Bounds {
                upper: Some(bound), ..
            }) => PastClass::History(bound),
            Some(Bounds { lower, upper: None }) => PastClass::Shift(lower),
        };
        Some((operator, class))
    }

    /// Whether the signed occurrence is, after dualization, an `F`/`U`
    /// (a least-fixed-point operator) rather than a `G`/`R`; `None` for a
    /// non-future node.
    fn least_fixed_point(&self, polarity: Polarity) -> Option<bool> {
        let eventual = match self.op? {
            Op::Eventually | Op::Until => true,
            Op::Always | Op::Release => false,
            Op::Past(_) => return None,
        };
        Some(eventual == (polarity == Polarity::Positive))
    }
}

// --- the reader -----------------------------------------------------------

/// The elements of an array.
fn members<'d>(node: NodeRef<'d>) -> Result<Vec<NodeRef<'d>>, KeyRefusal> {
    match node.node() {
        Node::Array(items) => Ok(items.collect()),
        _ => Err(KeyRefusal::Members),
    }
}

fn string<'d>(node: NodeRef<'d>) -> Result<&'d str, KeyRefusal> {
    match node.node() {
        Node::String(text) => Ok(text),
        _ => Err(KeyRefusal::Members),
    }
}

fn boolean(node: NodeRef<'_>) -> Result<bool, KeyRefusal> {
    match node.node() {
        Node::Bool(value) => Ok(value),
        _ => Err(KeyRefusal::Members),
    }
}

fn path(node: NodeRef<'_>) -> Result<Vec<u64>, KeyRefusal> {
    members(node)?
        .into_iter()
        .map(|step| decimal(string(step)?))
        .collect()
}

#[ix_trace_rs::string_edge]
fn polarity(node: NodeRef<'_>) -> Result<Polarity, KeyRefusal> {
    match string(node)? {
        "positive" => Ok(Polarity::Positive),
        "negative" => Ok(Polarity::Negative),
        _ => Err(KeyRefusal::Members),
    }
}

/// A set read in order: strictly ascending by complete canonical octets.
fn set<'d, T>(
    node: NodeRef<'d>,
    max_bytes: u64,
    mut read: impl FnMut(NodeRef<'d>) -> Result<T, KeyRefusal>,
) -> Result<Vec<T>, KeyRefusal> {
    let mut previous: Option<Vec<u8>> = None;
    let mut out = Vec::new();
    for element in members(node)? {
        ascending(&mut previous, canonical(element, max_bytes)?)?;
        out.push(read(element)?);
    }
    Ok(out)
}

/// Whether two configurations differ at most in their obligations.
fn same_facts(left: &Configuration, right: &Configuration) -> bool {
    left.past == right.past && left.windows == right.windows && left.acceptance == right.acceptance
}

/// The branches of a state in FR-338's normal form: the windows of one
/// occurrence merged to the one retained counter (the earliest deadline, the
/// furthest extent, the union of offsets), and every branch an obligation-free
/// branch subsumes removed.
fn normal(branches: &[Configuration]) -> Vec<Configuration> {
    let merged: Vec<Configuration> = branches
        .iter()
        .map(|branch| {
            let mut windows: Vec<Window> = Vec::new();
            for window in &branch.windows {
                let held = windows.iter_mut().find(|held| {
                    std::mem::discriminant(*held) == std::mem::discriminant(window)
                        && held.occurrence() == window.occurrence()
                });
                match (held, window) {
                    (Some(Window::Deadline(_, kept)), Window::Deadline(_, new)) => {
                        *kept = (*kept).min(*new);
                    }
                    (Some(Window::Extent(_, kept)), Window::Extent(_, new)) => {
                        *kept = (*kept).max(*new);
                    }
                    (Some(Window::Offsets(_, kept)), Window::Offsets(_, new)) => {
                        kept.extend(new.iter().copied());
                    }
                    _ => windows.push(window.clone()),
                }
            }
            Configuration {
                windows,
                ..branch.clone()
            }
        })
        .collect();
    merged
        .iter()
        .filter(|branch| {
            branch.obligations.is_empty()
                || !merged
                    .iter()
                    .any(|free| free.obligations.is_empty() && same_facts(free, branch))
        })
        .cloned()
        .collect()
}

/// The reader of one safety state over one formula shape.
pub(super) struct Reader<'s> {
    pub(super) shape: &'s FormulaShape,
    pub(super) max_bytes: u64,
}

impl Reader<'_> {
    /// The configurations of a `["safety", branches]` state's `branches`.
    pub(super) fn branches(&self, branches: NodeRef<'_>) -> Result<Vec<Configuration>, KeyRefusal> {
        let parsed = set(branches, self.max_bytes, |branch| {
            self.configuration(branch)
        })?;
        let subsumed = parsed.iter().any(|free| {
            free.obligations.is_empty()
                && parsed
                    .iter()
                    .any(|other| !other.obligations.is_empty() && same_facts(free, other))
        });
        if subsumed {
            return Err(KeyRefusal::NonNormal);
        }
        Ok(parsed)
    }

    fn occurrence<'a>(
        &'a self,
        path_node: NodeRef<'_>,
        polarity_node: NodeRef<'_>,
    ) -> Result<(Occurrence, &'a Shaped), KeyRefusal> {
        let path = path(path_node)?;
        let polarity = polarity(polarity_node)?;
        let node = self.shape.get(&path).ok_or(KeyRefusal::Unresolved)?;
        Ok((Occurrence { path, polarity }, node))
    }

    fn configuration(&self, branch: NodeRef<'_>) -> Result<Configuration, KeyRefusal> {
        let [obligations, past, windows, acceptance] =
            exactly(members(branch)?.into_iter()).ok_or(KeyRefusal::Members)?;
        let configuration = Configuration {
            obligations: set(obligations, self.max_bytes, |entry| self.obligation(entry))?,
            past: set(past, self.max_bytes, |entry| self.past(entry))?,
            windows: set(windows, self.max_bytes, |entry| self.window(entry))?,
            acceptance: set(acceptance, self.max_bytes, |entry| self.accepted(entry))?,
        };
        self.coherent(&configuration)?;
        Ok(configuration)
    }

    /// The cross-member rules of one configuration: one window per signed
    /// occurrence, no interval debt stored in both obligations and windows,
    /// and past memory for exactly the formula's past occurrences.
    fn coherent(&self, configuration: &Configuration) -> Result<(), KeyRefusal> {
        let mut windowed: Vec<&Occurrence> = Vec::new();
        for window in &configuration.windows {
            if windowed.contains(&window.occurrence()) {
                return Err(KeyRefusal::Members);
            }
            windowed.push(window.occurrence());
        }
        for obligation in &configuration.obligations {
            let stored = match obligation {
                Obligation::Interval { .. } => true,
                // A completed shift's unbounded obligation replaces its
                // `shift` entry, so both together name one debt twice.
                Obligation::Current(occurrence) => self
                    .shape
                    .get(&occurrence.path)
                    .is_some_and(|node| node.interval.is_some()),
            };
            if stored && windowed.contains(&obligation.occurrence()) {
                return Err(KeyRefusal::Members);
            }
        }
        self.membership(configuration)?;
        let mut held: Vec<&[u64]> = configuration.past.iter().map(PastMemory::path).collect();
        held.sort_unstable();
        if held.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(KeyRefusal::Members);
        }
        let expected: Vec<&Vec<u64>> = self
            .shape
            .nodes
            .iter()
            .filter(|(_, node)| matches!(node.op, Some(Op::Past(_))))
            .map(|(path, _)| path)
            .collect();
        // The shape's paths are ordered; so is `held` after sorting.
        if expected.len() != held.len()
            || expected
                .iter()
                .zip(&held)
                .any(|(expected, held)| expected.as_slice() != *held)
        {
            return Err(KeyRefusal::Unresolved);
        }
        Ok(())
    }

    /// A branch belongs to a least-fixed-point occurrence's acceptance set
    /// exactly when that occurrence is not carried from this position.
    fn membership(&self, configuration: &Configuration) -> Result<(), KeyRefusal> {
        for (path, node) in &self.shape.nodes {
            if node.interval.is_some() {
                continue;
            }
            for polarity in [Polarity::Positive, Polarity::Negative] {
                if node.least_fixed_point(polarity) != Some(true) || !node.reached(polarity) {
                    continue;
                }
                let signed = Occurrence {
                    path: path.clone(),
                    polarity,
                };
                let carried = configuration
                    .obligations
                    .iter()
                    .any(|obligation| *obligation.occurrence() == signed);
                if configuration.acceptance.contains(&signed) == carried {
                    return Err(KeyRefusal::NonNormal);
                }
            }
        }
        Ok(())
    }

    /// `[o,s]` or `[o,s,lower,upper]`.
    #[ix_trace_rs::string_edge]
    fn obligation(&self, entry: NodeRef<'_>) -> Result<Obligation, KeyRefusal> {
        let items = members(entry)?;
        match items.as_slice() {
            [path, polarity] => {
                let (occurrence, node) = self.occurrence(*path, *polarity)?;
                // Spec ruling R-3 pending: only a future temporal carry is a
                // retained current obligation. An open-upper interval's
                // completed shift is its unbounded obligation (`[o,s]`).
                let unbounded = node.interval.is_none_or(|bounds| bounds.upper.is_none());
                if !node.is_future() || !unbounded {
                    return Err(KeyRefusal::Unresolved);
                }
                Ok(Obligation::Current(occurrence))
            }
            [path, polarity, lower, upper] => {
                let (occurrence, node) = self.occurrence(*path, *polarity)?;
                let lower = decimal(string(*lower)?)?;
                let upper = match string(*upper)? {
                    "open" => None,
                    text => Some(decimal(text)?),
                };
                let (Some(checked), Some(Op::Eventually | Op::Always | Op::Until | Op::Release)) =
                    (node.interval, node.op)
                else {
                    return Err(KeyRefusal::Unresolved);
                };
                let ordered = upper.is_none_or(|upper| lower <= upper);
                // Spec ruling R-1 pending: whether this form is reachable at
                // all; the conservative reading admits only a remaining
                // interval inside the checked one.
                let within = lower <= checked.lower
                    && match (upper, checked.upper) {
                        (None, None) => true,
                        (Some(upper), Some(bound)) => upper <= bound,
                        (Some(_), None) | (None, Some(_)) => false,
                    };
                if !ordered || !within || (lower == 0 && upper.is_none()) {
                    return Err(KeyRefusal::Unresolved);
                }
                Ok(Obligation::Interval {
                    occurrence,
                    lower,
                    upper,
                })
            }
            _ => Err(KeyRefusal::Members),
        }
    }

    /// `[o,s]` of a least-fixed-point occurrence.
    fn accepted(&self, entry: NodeRef<'_>) -> Result<Occurrence, KeyRefusal> {
        let [path, polarity] = exactly(members(entry)?.into_iter()).ok_or(KeyRefusal::Members)?;
        let (occurrence, node) = self.occurrence(path, polarity)?;
        // Spec ruling R-2 pending: an `F`/`U[a,open]` occurrence has no
        // acceptance membership here; only an unbounded `F`/`U` does.
        if node.least_fixed_point(occurrence.polarity) == Some(true)
            && node.interval.is_none()
            && node.reached(occurrence.polarity)
        {
            Ok(occurrence)
        } else {
            Err(KeyRefusal::Unresolved)
        }
    }

    /// `[o,s,kind,payload]`.
    #[ix_trace_rs::string_edge]
    fn window(&self, entry: NodeRef<'_>) -> Result<Window, KeyRefusal> {
        let [path, polarity, kind, payload] =
            exactly(members(entry)?.into_iter()).ok_or(KeyRefusal::Members)?;
        let (occurrence, node) = self.occurrence(path, polarity)?;
        let kind = string(kind)?;
        let Some(checked) = node.interval else {
            return Err(KeyRefusal::Unresolved);
        };
        let eventual = node.least_fixed_point(occurrence.polarity);
        let common = checked.lower == 0 && !node.nested_interval;
        match (kind, checked.upper) {
            ("deadline" | "extent", Some(upper)) => {
                let wants_deadline = kind == "deadline";
                let remaining = decimal(string(payload)?)?;
                if eventual != Some(wants_deadline) || !common || remaining > upper {
                    return Err(KeyRefusal::Unresolved);
                }
                Ok(if wants_deadline {
                    Window::Deadline(occurrence, remaining)
                } else {
                    Window::Extent(occurrence, remaining)
                })
            }
            ("offsets", Some(upper)) => {
                let pairs = set(payload, self.max_bytes, |pair| {
                    let [lower, high] =
                        exactly(members(pair)?.into_iter()).ok_or(KeyRefusal::Members)?;
                    let lower = decimal(string(lower)?)?;
                    let high = decimal(string(high)?)?;
                    Ok((lower, high))
                })?;
                let inside = |(lower, high): &(u64, u64)| {
                    lower <= high && *high <= upper && *lower <= checked.lower
                };
                if common || eventual.is_none() || pairs.is_empty() || !pairs.iter().all(inside) {
                    return Err(KeyRefusal::Unresolved);
                }
                Ok(Window::Offsets(occurrence, pairs))
            }
            ("shift", None) => {
                let remaining = decimal(string(payload)?)?;
                if eventual.is_none() || remaining == 0 || remaining > checked.lower {
                    return Err(KeyRefusal::Unresolved);
                }
                Ok(Window::Shift(occurrence, remaining))
            }
            _ => Err(KeyRefusal::Unresolved),
        }
    }

    /// One `past` entry.
    #[ix_trace_rs::string_edge]
    fn past(&self, entry: NodeRef<'_>) -> Result<PastMemory, KeyRefusal> {
        let items = members(entry)?;
        let [path_node, tag, rest @ ..] = items.as_slice() else {
            return Err(KeyRefusal::Members);
        };
        let held = path(*path_node)?;
        let node = self.shape.get(&held).ok_or(KeyRefusal::Unresolved)?;
        let (operator, class) = node.past_class().ok_or(KeyRefusal::Unresolved)?;
        let tag = string(*tag)?;
        let flags = [
            PastOperator::Once,
            PastOperator::Historically,
            PastOperator::Since,
            PastOperator::Triggered,
        ];
        let by_flag = flags.iter().find(|candidate| candidate.flag() == tag);
        let by_age = flags.iter().find(|candidate| candidate.age() == tag);
        let expected = match tag {
            "shift-history" => 2,
            "history" => 1,
            _ if by_flag.is_some() || by_age.is_some() => 1,
            _ => return Err(KeyRefusal::Unresolved),
        };
        if rest.len() != expected {
            return Err(KeyRefusal::Members);
        }
        match (class, rest) {
            (PastClass::Unbounded, [value]) if by_flag == Some(&operator) => Ok(PastMemory::Flag {
                path: held,
                operator,
                value: boolean(*value)?,
            }),
            (PastClass::Age(bound), [age]) if by_age == Some(&operator) => {
                let age = decimal_wide(string(*age)?)?;
                if age > u128::from(bound) + 1 {
                    return Err(KeyRefusal::Unresolved);
                }
                Ok(PastMemory::Age {
                    path: held,
                    operator,
                    age,
                })
            }
            (PastClass::History(bound), [operands]) if tag == "history" => {
                Ok(PastMemory::History {
                    operands: self.operands(*operands, &held, operator, bound)?,
                    path: held,
                })
            }
            (PastClass::Shift(shift), [operands, tail]) if tag == "shift-history" => {
                Ok(PastMemory::ShiftHistory {
                    operands: self.operands(*operands, &held, operator, shift)?,
                    tail: boolean(*tail)?,
                    path: held,
                })
            }
            _ => Err(KeyRefusal::Unresolved),
        }
    }

    /// The operand vector of a history entry: one `[operand_path,
    /// true_offsets]` per operand, in operand order, offsets in `1..=bound`.
    fn operands(
        &self,
        node: NodeRef<'_>,
        owner: &[u64],
        operator: PastOperator,
        bound: u64,
    ) -> Result<Vec<HistoryOperand>, KeyRefusal> {
        let items = members(node)?;
        if items.len() != operator.arity() {
            return Err(KeyRefusal::Members);
        }
        let mut operands = Vec::with_capacity(items.len());
        for (position, item) in items.into_iter().enumerate() {
            let [operand, offsets] =
                exactly(members(item)?.into_iter()).ok_or(KeyRefusal::Members)?;
            let operand = path(operand)?;
            let mut expected = owner.to_vec();
            expected.push(u64::try_from(position).map_err(|_| KeyRefusal::Unresolved)?);
            if operand != expected {
                return Err(KeyRefusal::Unresolved);
            }
            let offsets = set(offsets, self.max_bytes, |offset| {
                let offset = decimal(string(offset)?)?;
                if (1..=bound).contains(&offset) {
                    Ok(offset)
                } else {
                    Err(KeyRefusal::Unresolved)
                }
            })?;
            operands.push(HistoryOperand {
                path: operand,
                offsets,
            });
        }
        Ok(operands)
    }
}

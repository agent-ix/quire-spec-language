// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251 "Transitions": the four state-passing entries of a tick-monitor
//! plan, written as complete-V1 source over the ordinary checked expression
//! contract, so that the one checker, lowering and emitter give them their
//! types, node keys and wire form, and `qsl-eval`'s function evaluator runs
//! them (FR-251-AC-7). No statement vocabulary is used: an assignment is a
//! record construction, a branch an `if`, a loop a `fold` over a sequence
//! bounded by its capacity premise, a push `flatten(sequence[s, sequence[x]])`.
//!
//! Two record fields FR-487 names are reserved words of the complete-V1
//! surface (`origin`, `value`), so no record declaration can spell them
//! (QSpec shared-grammar "reserved words"); until the surface admits them
//! as field names the generated records spell them `origin_tick` and
//! `amount`. [`WIRE_FIELD_NAMES`] is the one place that records the
//! correspondence.
//!
//! The program covers the whole formula language. Every interval operator, at
//! any depth, owns a buffer (`buffer_<n>` of the state, `n` its position in
//! the plan's buffer table) whose entries carry the global event position
//! and the atoms of one event; the value of an operator at a position is a
//! pure function that folds its buffer and reads the operators below it at
//! the entry's position. An unbounded operator keeps one `activations`
//! element per live instance: its accumulators and its position, which is
//! `State.event_position` when the instance was created (QSpec FR-487
//! "status" is any record; the position is the field `position` of the
//! status). A root that is not an interval operator is such an instance too,
//! carrying the atoms of its activating event.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::ToPrimitive;
use qsl_eval::temporal::{operator_parts, temporal_leaves, TickBound, TickShape};
use qsl_replay::TimedMonitorPlan;
use qsl_semantics::check::{Activation, FormulaNode};
use quire_exact::{Integer as ExactInteger, Rational};

/// A Boolean expression over `holds` atoms and the temporal operators below
/// an operator, as the operand of that operator (or the clause root).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    /// `true` or `false`.
    Const(bool),
    /// The `holds` atom with this formula arena index.
    Atom(usize),
    /// The temporal operator with this formula arena index.
    Leaf(usize),
    /// `not`.
    Not(Box<Expr>),
    /// `and`.
    And(Box<Expr>, Box<Expr>),
    /// `or`.
    Or(Box<Expr>, Box<Expr>),
    /// `implies`.
    Implies(Box<Expr>, Box<Expr>),
}

/// The integer thresholds that decide a pair of ticks against an interval
/// under the widened cover (QSpec FR-160 "Counter-tick intervals and
/// endpoints"), derived exactly by the producer from the rational bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Thresholds {
    /// Smallest offset every point of which is at or above the lower bound.
    pub all_lo: i128,
    /// Largest offset every point of which is at or below the upper bound.
    pub all_hi: i128,
    /// Largest offset that lies wholly below the lower bound.
    pub none_lo: i128,
    /// Smallest offset that lies wholly above the upper bound.
    pub none_hi: i128,
    /// Whether the offset 0 of an event with itself is in the interval.
    pub zero_in: bool,
}

/// What an interval operator adds to a temporal operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntervalSpec {
    /// The decision thresholds.
    pub thresholds: Thresholds,
    /// The buffer capacity premise.
    pub capacity: u64,
    /// The ticks an entry is retained for (`ceil` of the exact retention).
    pub retention: i128,
    /// The ticks after which an instance is settled by time, if it is.
    pub lookahead: Option<i128>,
}

/// One temporal operator of the clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeSpec {
    /// The formula arena index, the instance's `subformula`.
    pub id: usize,
    /// Whether the operator looks to later events.
    pub future: bool,
    /// Whether the operator is the dual (`always`, `historically`,
    /// `release`, `triggered`).
    pub dual: bool,
    /// The left operand of a binary operator.
    pub phi: Option<Expr>,
    /// The (right) operand.
    pub psi: Expr,
    /// The interval and buffer of an interval operator; `None` on an
    /// unbounded one.
    pub interval: Option<IntervalSpec>,
    /// For an unbounded operator, the interval operator whose buffer carries
    /// its operands' events.
    pub carrier: Option<usize>,
}

/// How the clause activates its root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationKind {
    /// At the first event only.
    Origin,
    /// At every event.
    Each,
    /// At every event at which the guard holds.
    EachGuarded,
}

/// The clause text the atoms are evaluated from: the entries call one
/// helper per atom over the clause's own parameters, with the atom's
/// expression exactly as authored.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseSource {
    /// The clause parameters as `(name, type text)`; they are the fields of
    /// the `Inputs` record, one per admitted value.
    pub parameters: Vec<(String, String)>,
    /// The authored expression of each `holds` atom, by formula index.
    pub atoms: Vec<(usize, String)>,
    /// The authored activation guard, if the clause has one.
    pub guard: Option<String>,
    /// The unit's profile alias that admits the clause's model reads.
    pub profile: String,
}

/// Everything the source generator needs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntrySpec {
    /// The atoms' formula indices, in input-field order.
    pub atoms: Vec<usize>,
    /// The temporal operators, by ascending formula index.
    pub nodes: Vec<NodeSpec>,
    /// The formula root's arena index.
    pub root: usize,
    /// The root as an expression when it is not a temporal operator.
    pub root_expr: Option<Expr>,
    /// The clause activation.
    pub activation: ActivationKind,
    /// The subject text each result carries.
    pub subject: String,
    /// The plan's `max_reading_gap`.
    pub max_reading_gap: u64,
    /// The plan's `activation_capacity`.
    pub activation_capacity: u64,
    /// The clause's own parameters and `holds` expressions, when the atoms
    /// are evaluated by the entries (QSL FR-251 "Entry bindings").
    pub clause: Option<ClauseSource>,
    /// The `holds` atoms with no admitted projection (QSpec FR-487-AC-12),
    /// by formula index: an instance that reads one settles
    /// `Incomplete(LimitReached{atom_projection})`.
    pub unprojectable: Vec<usize>,
    /// The declared tick unit, which the generated unit mentions so the
    /// graph holds its node (the plan root's `tick_unit` binding).
    pub tick_unit: Option<String>,
}

/// Why the entries cannot be generated for a clause.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EntryRefusal {
    /// An unbounded operator that looks backwards (`once`, `historically`,
    /// `since`, `triggered` without an interval).
    #[error("unbounded past operator {node} is not generated")]
    UnboundedPast {
        /// The formula arena index.
        node: usize,
    },
    /// An unbounded operator whose operands hold another unbounded operator.
    #[error("unbounded operator {node} has an unbounded operator in its operands")]
    UnboundedOverUnbounded {
        /// The formula arena index.
        node: usize,
    },
    /// A tick threshold does not fit the generated integers.
    #[error("the ticks of operator {node} do not fit the generated integers")]
    TickRange {
        /// The formula arena index.
        node: usize,
    },
}

/// The record fields FR-487 names that the surface cannot spell yet:
/// `(source spelling, wire spelling)`.
pub const WIRE_FIELD_NAMES: [(&str, &str); 2] = [("origin_tick", "origin"), ("amount", "value")];

fn floor_of(value: &Rational) -> Option<i128> {
    let floor: BigInt = value
        .numerator()
        .as_big()
        .div_floor(value.denominator().as_big());
    floor.to_i128()
}

fn ceil_of(value: &Rational) -> Option<i128> {
    let ceil: BigInt = value
        .numerator()
        .as_big()
        .div_ceil(value.denominator().as_big());
    ceil.to_i128()
}

impl Thresholds {
    /// The thresholds of the tick interval `[lower, upper]` (with the given
    /// endpoint flags) under the cover width `1 + 2 * uncertainty`.
    pub fn of(
        lower: &Rational,
        lower_closed: bool,
        upper: &Rational,
        upper_closed: bool,
        uncertainty: &Rational,
    ) -> Option<Self> {
        let int = |value: i64| Rational::from_integer(ExactInteger::from(value));
        let width = int(1).add(&int(2).mul(uncertainty));
        let zero = int(0);
        let above = zero > *lower || (zero == *lower && lower_closed);
        let below = zero < *upper || (zero == *upper && upper_closed);
        Some(Self {
            all_lo: ceil_of(&lower.add(&width))?,
            all_hi: floor_of(&upper.sub(&width))?,
            none_lo: floor_of(&lower.sub(&width))?,
            none_hi: ceil_of(&upper.add(&width))?,
            zero_in: above && below,
        })
    }
}

fn expr_of(nodes: &[FormulaNode], index: usize) -> Expr {
    let child = |id: qsl_semantics::check::FormulaNodeId| Box::new(expr_of(nodes, id.index()));
    match &nodes[index] {
        FormulaNode::Constant(value) => Expr::Const(*value),
        FormulaNode::Holds { .. } => Expr::Atom(index),
        FormulaNode::Aggregate { .. } => Expr::Const(false),
        FormulaNode::Not(a) => Expr::Not(child(*a)),
        FormulaNode::And(a, b) => Expr::And(child(*a), child(*b)),
        FormulaNode::Or(a, b) => Expr::Or(child(*a), child(*b)),
        FormulaNode::Implies(a, b) => Expr::Implies(child(*a), child(*b)),
        FormulaNode::Unary { .. } | FormulaNode::Binary { .. } => Expr::Leaf(index),
    }
}

/// Derive the generator input from a produced plan, or name the construct
/// the generator does not cover.
pub fn spec_from_plan(plan: &TimedMonitorPlan) -> Result<EntrySpec, EntryRefusal> {
    let formula = plan.program().formula();
    let arena = formula.nodes();
    let root = plan.program().root().index();
    let bounds: BTreeMap<usize, TickBound> = plan
        .intervals()
        .iter()
        .map(|(id, interval)| {
            (
                id.index(),
                TickBound {
                    lower: interval.lower().clone(),
                    lower_closed: interval.lower_closed(),
                    upper: interval.upper().clone(),
                    upper_closed: interval.upper_closed(),
                },
            )
        })
        .collect();
    let shape = TickShape::of(formula, &bounds, plan.uncertainty());
    let mut nodes = Vec::new();
    for (index, info) in shape.nodes() {
        let interval = match &info.bound {
            Some(bound) => {
                let thresholds = Thresholds::of(
                    &bound.lower,
                    bound.lower_closed,
                    &bound.upper,
                    bound.upper_closed,
                    plan.uncertainty(),
                )
                .ok_or(EntryRefusal::TickRange { node: *index })?;
                let capacity = plan
                    .buffers()
                    .iter()
                    .find(|(id, _)| id.index() == *index)
                    .map_or(1, |(_, capacity)| *capacity);
                Some(IntervalSpec {
                    thresholds,
                    capacity,
                    retention: ceil_of(&info.retention)
                        .ok_or(EntryRefusal::TickRange { node: *index })?,
                    lookahead: match &info.lookahead {
                        Some(look) => {
                            Some(ceil_of(look).ok_or(EntryRefusal::TickRange { node: *index })?)
                        }
                        None => None,
                    },
                })
            }
            None => {
                if !info.future {
                    return Err(EntryRefusal::UnboundedPast { node: *index });
                }
                None
            }
        };
        let leaves: Vec<usize> = info
            .left
            .into_iter()
            .chain([info.right])
            .flat_map(|operand| temporal_leaves(formula, operand))
            .collect();
        let carrier = if interval.is_none() {
            if leaves.iter().any(|leaf| !bounds.contains_key(leaf)) {
                return Err(EntryRefusal::UnboundedOverUnbounded { node: *index });
            }
            leaves.first().copied()
        } else {
            None
        };
        nodes.push(NodeSpec {
            id: *index,
            future: info.future,
            dual: info.dual,
            phi: info.left.map(|left| expr_of(arena, left)),
            psi: expr_of(arena, info.right),
            interval,
            carrier,
        });
    }
    let root_is_temporal = operator_parts(&arena[root]).is_some();
    Ok(EntrySpec {
        atoms: (0..arena.len())
            .filter(|index| matches!(arena[*index], FormulaNode::Holds { .. }))
            .collect(),
        nodes,
        root,
        root_expr: (!root_is_temporal).then(|| expr_of(arena, root)),
        activation: match plan.program().activation() {
            Activation::Origin => ActivationKind::Origin,
            Activation::Each { guarded: false } => ActivationKind::Each,
            Activation::Each { guarded: true } => ActivationKind::EachGuarded,
        },
        subject: plan.subject().clause_name().to_owned(),
        max_reading_gap: plan.max_reading_gap(),
        activation_capacity: plan.activation_capacity(),
        clause: None,
        unprojectable: Vec::new(),
        tick_unit: Some(plan.binding().unit().to_owned()),
    })
}

/// The outcome codes the entries keep in a cell's `rep` and an activation's
/// status.
pub mod code {
    /// Nothing reported yet.
    pub const NONE: i64 = 0;
    /// `True`.
    pub const TRUE: i64 = 1;
    /// `False`.
    pub const FALSE: i64 = 2;
    /// `Pending`.
    pub const PENDING: i64 = 3;
    /// `Indeterminate`.
    pub const INDETERMINATE: i64 = 4;
    /// `Failed(ReadingGapExceeded)`.
    pub const FAILED: i64 = 6;
    /// `Incomplete(LimitReached{activation})`.
    pub const ACTIVATION: i64 = 7;
    /// An `Incomplete(LimitReached{atom_projection})` outcome is this plus
    /// the atom's formula index (QSpec FR-487-AC-12).
    pub const ATOM_BASE: i64 = 1_000;
    /// An `Incomplete(LimitReached{event_buffer})` outcome is this plus the
    /// buffer capacity.
    pub const BUFFER_BASE: i64 = 2_000;
}

/// The identifiers of the generated declarations.
pub mod names {
    /// The state record.
    pub const STATE: &str = "State";
    /// The inputs record.
    pub const INPUTS: &str = "Inputs";
    /// The record of atom truths a cell keeps.
    pub const ATOMS: &str = "Atoms";
    /// The scope record.
    pub const SCOPE: &str = "Scope";
    /// The result record.
    pub const RESULT: &str = "Result";
    /// The outcome union.
    pub const OUTCOME: &str = "Outcome";
    /// The `init` entry.
    pub const INIT: &str = "monitorInit";
    /// The `on_event` entry.
    pub const ON_EVENT: &str = "monitorOnEvent";
    /// The `on_watermark` entry.
    pub const ON_WATERMARK: &str = "monitorOnWatermark";
    /// The `on_gap` entry.
    pub const ON_GAP: &str = "monitorOnGap";
}

/// The sentinel `lost` marker of a value that depends on an instance the
/// activation capacity refused; a buffer overflow of operator `n` is `n + 1`.
const ACTIVATION_LOST: i64 = 1_000_000;

/// Fill `@NAME@` placeholders.
fn tpl(template: &str, pairs: &[(&str, String)]) -> String {
    let mut out = template.to_owned();
    for (name, value) in pairs {
        out = out.replace(&format!("@{name}@"), value);
    }
    out
}

fn sig(value: i128) -> String {
    if value < 0 {
        format!("(0 - {})", -value)
    } else {
        value.to_string()
    }
}

struct Gen<'a> {
    spec: &'a EntrySpec,
    /// The interval operators, ascending: the buffer table.
    buffers: Vec<usize>,
}

impl<'a> Gen<'a> {
    fn new(spec: &'a EntrySpec) -> Self {
        Self {
            spec,
            buffers: spec
                .nodes
                .iter()
                .filter(|node| node.interval.is_some())
                .map(|node| node.id)
                .collect(),
        }
    }

    fn buffer_of(&self, node: usize) -> usize {
        self.buffers.iter().position(|id| *id == node).unwrap_or(0)
    }

    fn node(&self, id: usize) -> Option<&NodeSpec> {
        self.spec.nodes.iter().find(|node| node.id == id)
    }

    fn unbounded(&self) -> impl Iterator<Item = &NodeSpec> {
        self.spec.nodes.iter().filter(|node| node.interval.is_none())
    }

    fn intervals(&self) -> impl Iterator<Item = &NodeSpec> {
        self.spec.nodes.iter().filter(|node| node.interval.is_some())
    }

    fn cap(&self, node: &NodeSpec) -> u64 {
        node.interval.map_or(1, |interval| interval.capacity.max(1))
    }

    /// The expression in the value algebra; `s`, `a`, `p`, `uv` and `hz` are
    /// the state, the atoms, the position, the finished unbounded values and
    /// the horizon of the frame it is read in.
    fn render(&self, expr: &Expr) -> String {
        match expr {
            Expr::Const(true) => "vTrue()".to_owned(),
            Expr::Const(false) => "vFalse()".to_owned(),
            Expr::Atom(index) if self.spec.unprojectable.contains(index) => {
                format!("vAtomLost({})", index + 1)
            }
            Expr::Atom(index) => format!("vOf(a.a{index})"),
            Expr::Leaf(node) => match self.node(*node) {
                Some(spec) if spec.interval.is_some() => format!("val_{node}(s, p, hz)"),
                _ => format!("uleaf_{node}(s, uv, p, hz)"),
            },
            Expr::Not(a) => format!("vNot({})", self.render(a)),
            Expr::And(a, b) => format!("vAnd({}, {})", self.render(a), self.render(b)),
            Expr::Or(a, b) => format!("vOr({}, {})", self.render(a), self.render(b)),
            Expr::Implies(a, b) => {
                format!("vOr(vNot({}), {})", self.render(a), self.render(b))
            }
        }
    }

    fn state_fields(&self) -> Vec<String> {
        let mut fields = vec!["now".to_owned(), "history_start".to_owned()];
        fields.extend((0..self.buffers.len()).map(|n| format!("buffer_{n}")));
        fields.push("activations".to_owned());
        fields.push("event_position".to_owned());
        fields
    }

    fn state_types(&self) -> String {
        let mut text = String::from("now: Integer; history_start: Integer; ");
        for n in 0..self.buffers.len() {
            let _ = write!(text, "buffer_{n}: Cells; ");
        }
        text.push_str("activations: Sequence<Activation>; event_position: Integer;");
        text
    }

    /// One `with_<field>` setter per state field.
    fn setters(&self) -> String {
        let fields = self.state_fields();
        let mut out = String::new();
        for field in &fields {
            let ty = if field.starts_with("buffer_") {
                "Cells"
            } else if field == "activations" {
                "Sequence<Activation>"
            } else {
                "Integer"
            };
            let body = fields
                .iter()
                .map(|f| {
                    if f == field {
                        format!("{f}: x")
                    } else {
                        format!("{f}: s.{f}")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                out,
                "function with_{field} using v(s: State, x: {ty}): State pure {{ State {{ {body} }} }}"
            );
        }
        out
    }

    fn atoms_expr(&self) -> String {
        let fields = self
            .spec
            .atoms
            .iter()
            .map(|atom| match &self.spec.clause {
                _ if self.spec.unprojectable.contains(atom) => format!("a{atom}: false"),
                None => format!("a{atom}: i.a{atom}"),
                Some(clause) => format!(
                    "a{atom}: atom{atom}({})",
                    clause
                        .parameters
                        .iter()
                        .map(|(name, _)| format!("i.{name}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{} {{ {fields} }}", names::ATOMS)
    }

    fn guard_expr(&self) -> String {
        match &self.spec.clause {
            Some(clause) if clause.guard.is_some() => format!(
                "guardFn({})",
                clause
                    .parameters
                    .iter()
                    .map(|(name, _)| format!("i.{name}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            _ => "i.guard".to_owned(),
        }
    }

    fn node_functions(&self, node: &NodeSpec) -> String {
        let id = node.id;
        let mut out = String::new();
        let psi = self.render(&node.psi);
        let _ = writeln!(
            out,
            "function psiX_{id} using v(s: State, a: Atoms, p: Integer, uv: Sequence<UVal>, hz: Integer): V pure {{ {psi} }}"
        );
        let phi = node.phi.as_ref().map(|phi| self.render(phi));
        if let Some(phi) = &phi {
            let _ = writeln!(
                out,
                "function phiX_{id} using v(s: State, a: Atoms, p: Integer, uv: Sequence<UVal>, hz: Integer): V pure {{ {phi} }}"
            );
        }
        match &node.interval {
            Some(interval) => out.push_str(&self.interval_functions(node, interval, phi.is_some())),
            None => out.push_str(&self.unbounded_functions(node, phi.is_some())),
        }
        out
    }


    fn interval_functions(&self, node: &NodeSpec, interval: &IntervalSpec, binary: bool) -> String {
        let id = node.id;
        let t = interval.thresholds;
        let pol = if node.dual { "vNot" } else { "vId" };
        let psi = format!("{pol}(psiX_{id}(s, x.inputs, x.pf, x.uv, hz))");
        let phi = if binary {
            format!("{pol}(phiX_{id}(s, x.inputs, x.pf, x.uv, hz))")
        } else {
            "vTrue()".to_owned()
        };
        let elapsed = match interval.lookahead {
            Some(look) => format!("hz - x.tf >= {}", sig(look)),
            None => "false".to_owned(),
        };
        let (instance, needed, lost_results) = if node.future {
            (
                "let r = fold<Acc>(a, x in cells: \
                    if x.pf < ip then a \
                    else Acc { acc: vOr(a.acc, vAnd(vAnd(member_@N@(x.tf - it, x.tl - it, x.pf = ip and x.pl = ip), psi_@N@(s, x, hz)), a.pfx)), \
                               pfx: vAnd(a.pfx, phi_@N@(s, x, hz)) }, \
                    identity: Acc { acc: vFalse(), pfx: vTrue() }) in \
                 @POL@(if hz - it >= @OWN@ then r.acc else vOr(r.acc, vAnd(vFuture(), r.pfx)))",
                "z.pf < x.pf and memberKind_@N@(x.tf - z.tf, x.tl - z.tf, false) > 0",
                "let buf = sb.buffer_@B@ in \
                 if size(buf) < @CAP@ then noResults() \
                 else let last = lastOf(buf) in \
                      let code = codeOf(vLostBy(@N@ + 1)) in \
                      flatten(pairResults(\
                          if not last.lost and not last.hasSet then oneResult(resultOf(@N@, last.tf, code)) else noResults(), \
                          oneResult(resultOf(@N@, now2, code))))",
            )
        } else {
            (
                "let r = fold<Acc>(a, x in cells: \
                    if x.pl > ip then a \
                    else Acc { acc: vOr(vAnd(a.acc, phi_@N@(s, x, hz)), vAnd(member_@N@(it - x.tl, it - x.tf, x.pf = ip and x.pl = ip), psi_@N@(s, x, hz))), \
                               pfx: a.pfx }, \
                    identity: Acc { acc: vFalse(), pfx: vTrue() }) in \
                 let early = s.history_start >= 0 and memberKind_@N@(it - (s.history_start - 1), it - (s.history_start - 1), false) > 0 in \
                 @POL@(if early then vOr(r.acc, vUnknown()) else r.acc)",
                "z.pf > x.pf and memberKind_@N@(z.tf - x.tl, z.tf - x.tf, false) > 0",
                "let f = findCell(sa.buffer_@B@, pos) in \
                 if f.found and f.c.lost and f.c.pl = pos and f.c.pf < pos \
                 then oneResult(resultOf(@N@, now2, codeOf(inst_@N@(sa, sa.buffer_@B@, pos, now2, hz)))) \
                 else noResults()",
            )
        };
        let pairs: Vec<(&str, String)> = vec![
            ("N", id.to_string()),
            ("B", self.buffer_of(id).to_string()),
            ("OWN", sig(t.none_hi)),
            ("POL", pol.to_owned()),
            ("ZERO", t.zero_in.to_string()),
            ("ALL_LO", sig(t.all_lo)),
            ("ALL_HI", sig(t.all_hi)),
            ("NONE_LO", sig(t.none_lo)),
            ("NONE_HI", sig(t.none_hi)),
            ("RETAIN", sig(interval.retention)),
            ("CAP", self.cap(node).to_string()),
            ("ELAPSED", elapsed),
            ("PSI", psi),
            ("PHI", phi),
        ];
        let expand = |text: &str| tpl(text, &pairs);
        let mut pairs = pairs.clone();
        pairs.push(("INSTANCE", expand(instance)));
        pairs.push(("NEEDED", expand(needed)));
        pairs.push(("LOSTRESULTS", expand(lost_results)));
        tpl(
            "function memberKind_@N@ using v(dmin: Integer, dmax: Integer, same: Boolean): Integer pure { \
                if same then (if @ZERO@ then 2 else 0) \
                else if dmin >= @ALL_LO@ and dmax <= @ALL_HI@ then 2 \
                else if dmax <= @NONE_LO@ or dmin >= @NONE_HI@ then 0 \
                else 1 }\n\
             function member_@N@ using v(dmin: Integer, dmax: Integer, same: Boolean): V pure { \
                let k = memberKind_@N@(dmin, dmax, same) in \
                if k = 2 then vTrue() else if k = 0 then vFalse() else vUnknown() }\n\
             function phi_@N@ using v(s: State, x: Cell, hz: Integer): V pure { \
                if x.lost then vLostBy(@N@ + 1) else @PHI@ }\n\
             function psi_@N@ using v(s: State, x: Cell, hz: Integer): V pure { \
                if x.lost then vLostBy(@N@ + 1) else @PSI@ }\n\
             function inst_@N@ using v(s: State, cells: Cells, ip: Integer, it: Integer, hz: Integer): V pure { @INSTANCE@ }\n\
             function val_@N@ using v(s: State, p: Integer, hz: Integer): V pure { \
                let f = findCell(s.buffer_@B@, p) in \
                if not f.found then vUnknown() \
                else if f.c.lost then vLostBy(@N@ + 1) \
                else if f.c.hasSet then f.c.fv \
                else inst_@N@(s, s.buffer_@B@, f.c.pf, f.c.tf, hz) }\n\
             function settle_@N@ using v(s: State, hz: Integer): State pure { \
                with_buffer_@B@(s, fold<CellsAcc>(a, x in s.buffer_@B@: \
                    if x.hasSet or x.lost then CellsAcc { cells: push(a.cells, x) } \
                    else let v = inst_@N@(s, s.buffer_@B@, x.pf, x.tf, hz) in \
                         let ok = isFinal(v) or (@ELAPSED@) in \
                         CellsAcc { cells: push(a.cells, if ok then setCell(x, v) else x) }, \
                    identity: CellsAcc { cells: noCells() }).cells) }\n\
             function needed_@N@ using v(cells: Cells, x: Cell): Boolean pure { \
                fold<Flag>(a, z in cells: \
                    Flag { b: a.b or (not z.hasSet and not z.lost and @NEEDED@) }, \
                    identity: Flag { b: false }).b }\n\
             function droppable_@N@ using v(cells: Cells, x: Cell, hz: Integer): Boolean pure { \
                x.hasSet and x.rep = codeOf(x.fv) and hz - x.tl >= @RETAIN@ and not needed_@N@(cells, x) }\n\
             function gone_@N@ using v(s: State, hz: Integer): Cells pure { \
                filter(x in s.buffer_@B@: droppable_@N@(s.buffer_@B@, x, hz)) }\n\
             function kept_@N@ using v(s: State, hz: Integer): Cells pure { \
                filter(x in s.buffer_@B@: not droppable_@N@(s.buffer_@B@, x, hz)) }\n\
             function report_@N@ using v(s: State, wm: Boolean, hz: Integer): StepR pure { \
                let r = fold<RepAcc>(a, x in s.buffer_@B@: \
                    if x.lost then RepAcc { cells: push(a.cells, x), results: a.results } \
                    else let v = if x.hasSet then x.fv else inst_@N@(s, s.buffer_@B@, x.pf, x.tf, hz) in \
                         let c = codeOf(v) in \
                         let rep = if c = 3 then wm else c != x.rep in \
                         RepAcc { cells: push(a.cells, repCell(x, c)), \
                                  results: if rep then push2(a.results, resultOf(@N@, x.tf, c)) else a.results }, \
                    identity: RepAcc { cells: noCells(), results: noResults() }) in \
                mk(with_buffer_@B@(s, r.cells), r.results) }\n\
             function merge_@N@ using v(s: State, now2: Integer, pos: Integer, atoms: Atoms): State pure { \
                let buf = s.buffer_@B@ in \
                with_buffer_@B@(s, (if size(buf) < @CAP@ \
                    then push(buf, Cell { pf: pos, pl: pos, tf: now2, tl: now2, lost: false, hasSet: false, fv: vUnknown(), rep: 0, inputs: atoms, uv: noUv() }) \
                    else let last = lastOf(buf) in \
                         push(dropLast(buf), Cell { pf: last.pf, pl: pos, tf: last.tf, tl: now2, lost: true, hasSet: true, fv: vLostBy(@N@ + 1), rep: codeOf(vLostBy(@N@ + 1)), inputs: zeroAtoms(), uv: noUv() }))) }\n\
             function newLost_@N@ using v(sb: State, sa: State, now2: Integer, pos: Integer, hz: Integer): Results pure { @LOSTRESULTS@ }\n\
             function leafSettled_@N@ using v(s: State, p: Integer): Boolean pure { \
                let f = findCell(s.buffer_@B@, p) in not f.found or f.c.lost or f.c.hasSet }\n",
            &pairs,
        )
    }

    fn unbounded_functions(&self, node: &NodeSpec, binary: bool) -> String {
        let id = node.id;
        let pol = if node.dual { "vNot" } else { "vId" };
        let phi_event = if binary {
            format!("{pol}(phiX_{id}(s, atoms, pos, noUv(), hz))")
        } else {
            "vTrue()".to_owned()
        };
        let mut out = String::new();
        let acc0 = "Acc { acc: e.status.acc, pfx: e.status.pfx }";
        match node.carrier {
            Some(carrier) => {
                let cb = self.buffer_of(carrier);
                let phi_cell = if binary {
                    format!("{pol}(phiX_{id}(s, x.inputs, x.pf, x.uv, hz))")
                } else {
                    "vTrue()".to_owned()
                };
                out.push_str(&tpl(
                    "function foldStep_@N@ using v(s: State, a: Acc, x: Cell, hz: Integer): Acc pure { \
                        let psi = if x.lost then vLostBy(@C@ + 1) else @POL@(psiX_@N@(s, x.inputs, x.pf, x.uv, hz)) in \
                        let phi = if x.lost then vLostBy(@C@ + 1) else @PHI@ in \
                        Acc { acc: vOr(a.acc, vAnd(psi, a.pfx)), pfx: vAnd(a.pfx, phi) } }\n\
                     function foldCellAct_@N@ using v(s: State, e: Activation, x: Cell, hz: Integer): Activation pure { \
                        let r = foldStep_@N@(s, @ACC0@, x, hz) in actAcc(e, r.acc, r.pfx) }\n\
                     function elemVal_@N@ using v(s: State, e: Activation, hz: Integer): V pure { \
                        let r = fold<Acc>(a, x in s.buffer_@CB@: \
                            if x.pl < e.status.position then a else foldStep_@N@(s, a, x, hz), \
                            identity: @ACC0@) in \
                        @POL@(vOr(r.acc, vAnd(vFuture(), r.pfx))) }\n",
                    &[
                        ("N", id.to_string()),
                        ("C", carrier.to_string()),
                        ("CB", cb.to_string()),
                        ("POL", pol.to_owned()),
                        ("PHI", phi_cell),
                        ("ACC0", acc0.to_owned()),
                    ],
                ));
            }
            None => {
                out.push_str(&tpl(
                    "function foldEvt_@N@ using v(s: State, e: Activation, atoms: Atoms, pos: Integer, hz: Integer): Activation pure { \
                        let psi = @POL@(psiX_@N@(s, atoms, pos, noUv(), hz)) in \
                        let phi = @PHI@ in \
                        actAcc(e, vOr(e.status.acc, vAnd(psi, e.status.pfx)), vAnd(e.status.pfx, phi)) }\n\
                     function elemVal_@N@ using v(s: State, e: Activation, hz: Integer): V pure { \
                        @POL@(vOr(e.status.acc, vAnd(vFuture(), e.status.pfx))) }\n",
                    &[
                        ("N", id.to_string()),
                        ("POL", pol.to_owned()),
                        ("PHI", phi_event),
                    ],
                ));
            }
        }
        out.push_str(&tpl(
            "function uleaf_@N@ using v(s: State, uv: Sequence<UVal>, p: Integer, hz: Integer): V pure { \
                let f = findAct(s.activations, @N@, p) in \
                if f.found then elemVal_@N@(s, f.a, hz) \
                else let g = uvFind(uv, @N@) in if g.found then g.v else vUnknown() }\n\
",
            &[("N", id.to_string())],
        ));
        out
    }

    /// The leaves of the clause root that the root instance waits for.
    fn root_leaves_settled(&self) -> String {
        let Some(expr) = &self.spec.root_expr else {
            return "false".to_owned();
        };
        let mut leaves = Vec::new();
        fn collect(expr: &Expr, out: &mut Vec<usize>) {
            match expr {
                Expr::Leaf(node) => out.push(*node),
                Expr::Not(a) => collect(a, out),
                Expr::And(a, b) | Expr::Or(a, b) | Expr::Implies(a, b) => {
                    collect(a, out);
                    collect(b, out);
                }
                Expr::Const(_) | Expr::Atom(_) => {}
            }
        }
        collect(expr, &mut leaves);
        leaves.sort_unstable();
        leaves.dedup();
        let parts: Vec<String> = leaves
            .iter()
            .map(|leaf| match self.node(*leaf) {
                Some(node) if node.interval.is_some() => format!("leafSettled_{leaf}(s, p)"),
                _ => format!("not findAct(s.activations, {leaf}, p).found"),
            })
            .collect();
        if parts.is_empty() {
            "true".to_owned()
        } else {
            parts.join(" and ")
        }
    }

    /// `if e.subformula = n then f_n(args) else ... else fallback`.
    fn chain(ids: &[usize], f: impl Fn(usize) -> String, fallback: &str) -> String {
        let mut text = String::new();
        for id in ids {
            let _ = write!(text, "if e.subformula = {id} then {} else ", f(*id));
        }
        text.push_str(fallback);
        text
    }

    fn sequential(&self, ids: &[usize], f: impl Fn(usize, &str, &str) -> String, last: &str) -> String {
        let mut text = String::new();
        let mut prev = "s0".to_owned();
        for (n, id) in ids.iter().enumerate() {
            let name = format!("s{}", n + 1);
            let _ = write!(text, "let {name} = {} in ", f(*id, &prev, &name));
            prev = name;
        }
        let _ = write!(text, "{}", last.replace("@LAST@", &prev));
        text
    }

    fn entries(&self) -> String {
        let spec = self.spec;
        let mut desc: Vec<usize> = self.intervals().map(|node| node.id).collect();
        desc.reverse();
        let asc: Vec<usize> = self.intervals().map(|node| node.id).collect();
        let unb: Vec<usize> = self.unbounded().map(|node| node.id).collect();
        let carrierless: Vec<usize> = self
            .unbounded()
            .filter(|node| node.carrier.is_none())
            .map(|node| node.id)
            .collect();
        let mut out = String::new();
        // Dispatchers.
        let elem_val = Self::chain(
            &unb,
            |id| format!("elemVal_{id}(s, e, hz)"),
            if spec.root_expr.is_some() {
                "rootX(s, e.status.atoms, e.status.position, e.status.uv, hz)"
            } else {
                "vUnknown()"
            },
        );
        let fold_evt = Self::chain(
            &carrierless,
            |id| format!("foldEvt_{id}(s, e, atoms, pos, hz)"),
            "e",
        );
        let _ = writeln!(
            out,
            "function elemVal using v(s: State, e: Activation, hz: Integer): V pure {{ {elem_val} }}\n\
             function foldEvtOne using v(s: State, e: Activation, atoms: Atoms, pos: Integer, hz: Integer): Activation pure {{ {fold_evt} }}\n\
             function isUnb using v(sub: Integer): Boolean pure {{ {unb_test} }}",
            unb_test = if unb.is_empty() {
                "false".to_owned()
            } else {
                unb.iter()
                    .map(|id| format!("sub = {id}"))
                    .collect::<Vec<_>>()
                    .join(" or ")
            }
        );
        if let Some(root) = &spec.root_expr {
            let _ = writeln!(
                out,
                "function rootX using v(s: State, a: Atoms, p: Integer, uv: Sequence<UVal>, hz: Integer): V pure {{ {} }}\n\
                 function rootSettled using v(s: State, p: Integer): Boolean pure {{ {} }}",
                self.render(root),
                self.root_leaves_settled()
            );
        } else {
            let _ = writeln!(
                out,
                "function rootSettled using v(s: State, p: Integer): Boolean pure {{ false }}"
            );
        }
        // Carried drops: each interval operator's dropped entries folded into
        // the instances that read them.
        for id in &asc {
            let carried: Vec<usize> = self
                .unbounded()
                .filter(|node| node.carrier == Some(*id))
                .map(|node| node.id)
                .collect();
            let body = Self::chain(
                &carried,
                |u| format!("(if x.pl >= e.status.position then foldCellAct_{u}(s, e, x, hz) else e)"),
                "e",
            );
            let _ = writeln!(
                out,
                "function foldCarried_{id} using v(s: State, e: Activation, x: Cell, hz: Integer): Activation pure {{ {body} }}\n\
                 function foldGone_{id} using v(s: State, acts: Sequence<Activation>, hz: Integer): Sequence<Activation> pure {{ \
                    fold<ActsAcc>(a, x in gone_{id}(s, hz): ActsAcc {{ acts: map(e in a.acts: foldCarried_{id}(s, e, x, hz)) }}, \
                        identity: ActsAcc {{ acts: acts }}).acts }}"
            );
        }
        let fold_acts = {
            let mut text = String::from("let a0 = s.activations in ");
            let mut prev = "a0".to_owned();
            for (n, id) in asc.iter().enumerate() {
                let name = format!("a{}", n + 1);
                let _ = write!(text, "let {name} = foldGone_{id}(s, {prev}, hz) in ");
                prev = name;
            }
            let _ = write!(text, "{prev}");
            text
        };
        let drop_state = {
            let mut text = format!("let t0 = with_activations(s, ({fold_acts})) in ");
            let mut prev = "t0".to_owned();
            for (n, id) in asc.iter().enumerate() {
                let name = format!("t{}", n + 1);
                let b = self.buffer_of(*id);
                let _ = write!(text, "let {name} = with_buffer_{b}({prev}, kept_{id}(s, hz)) in ");
                prev = name;
            }
            let _ = write!(text, "{prev}");
            text
        };
        let _ = writeln!(
            out,
            "function dropAll using v(s: State, hz: Integer): State pure {{ {drop_state} }}"
        );
        let settle_all = self.sequential(
            &desc,
            |id, prev, _| format!("settle_{id}({prev}, hz)"),
            "@LAST@",
        );
        let _ = writeln!(
            out,
            "function settleAll using v(s0: State, hz: Integer): State pure {{ {settle_all} }}"
        );
        let push_all = self.sequential(
            &asc,
            |id, prev, _| format!("merge_{id}({prev}, now2, pos, atoms)"),
            "@LAST@",
        );
        let _ = writeln!(
            out,
            "function pushAll using v(s0: State, now2: Integer, pos: Integer, atoms: Atoms, hz: Integer): State pure {{ {push_all} }}"
        );
        let lost_all = {
            let mut text = String::from("let r0 = noResults() in ");
            let mut prev = "r0".to_owned();
            for (n, id) in asc.iter().enumerate() {
                let name = format!("r{}", n + 1);
                let _ = write!(
                    text,
                    "let {name} = flatten(pairResults({prev}, newLost_{id}(sb, sa, now2, pos, hz))) in "
                );
                prev = name;
            }
            let _ = write!(text, "{prev}");
            text
        };
        let _ = writeln!(
            out,
            "function lostAll using v(sb: State, sa: State, now2: Integer, pos: Integer, hz: Integer): Results pure {{ {lost_all} }}"
        );
        let attach = {
            let mut text = String::from("let t0 = s in ");
            let mut prev = "t0".to_owned();
            for (n, id) in asc.iter().enumerate() {
                let name = format!("t{}", n + 1);
                let b = self.buffer_of(*id);
                let _ = write!(
                    text,
                    "let {name} = with_buffer_{b}({prev}, map(x in s.buffer_{b}: if x.pf <= p and p <= x.pl then uvCell(x, u, w) else x)) in "
                );
                prev = name;
            }
            let _ = write!(
                text,
                "with_activations({prev}, map(e in s.activations: if e.status.position = p then actUv(e, u, w) else e))"
            );
            text
        };
        let _ = writeln!(
            out,
            "function attach using v(s: State, u: Integer, p: Integer, w: V): State pure {{ {attach} }}\n\
             function startAct using v(s: State, u: Integer, now2: Integer, pos: Integer, atoms: Atoms): StepR pure {{ \
                if size(s.activations) >= {acap} \
                then mk(attach(s, u, pos, vLostBy({lost})), oneResult(resultOf(u, now2, {act}))) \
                else mk(with_activations(s, pushA(s.activations, Activation {{ subformula: u, origin_tick: now2, \
                    status: ActStatus {{ position: pos, rep: 0, acc: vFalse(), pfx: vTrue(), atoms: atoms, uv: noUv() }} }})), noResults()) }}",
            acap = spec.activation_capacity,
            lost = ACTIVATION_LOST,
            act = code::ACTIVATION,
        );
        // Start the instances at this position, ascending.
        let mut starting: Vec<(usize, bool)> = unb.iter().map(|id| (*id, false)).collect();
        if self.node(spec.root).is_none_or(|n| n.interval.is_none()) {
            if !unb.contains(&spec.root) {
                starting.push((spec.root, true));
            } else if let Some(entry) = starting.iter_mut().find(|(id, _)| *id == spec.root) {
                entry.1 = true;
            }
        }
        starting.sort_unstable();
        let start_all = {
            let mut text = String::from("let q0 = mk(s, noResults()) in ");
            let mut prev = "q0".to_owned();
            for (n, (id, is_root)) in starting.iter().enumerate() {
                let name = format!("q{}", n + 1);
                let call = format!("startAct({prev}.state, {id}, now2, pos, atoms)");
                let step = if *is_root {
                    format!("if active then joinR({prev}, {call}) else {prev}")
                } else {
                    format!("joinR({prev}, {call})")
                };
                let _ = write!(text, "let {name} = {step} in ");
                prev = name;
            }
            let _ = write!(text, "{prev}");
            text
        };
        let _ = writeln!(
            out,
            "function startAll using v(s: State, now2: Integer, pos: Integer, atoms: Atoms, active: Boolean): StepR pure {{ {start_all} }}"
        );
        let report_cells = {
            let mut text = String::from("let q0 = mk(s, noResults()) in ");
            let mut prev = "q0".to_owned();
            for (n, id) in asc.iter().enumerate() {
                let name = format!("q{}", n + 1);
                let _ = write!(
                    text,
                    "let {name} = joinR({prev}, report_{id}({prev}.state, wm, hz)) in "
                );
                prev = name;
            }
            let _ = write!(text, "{prev}");
            text
        };
        let fin_cond = if spec.root_expr.is_some() {
            format!("isFinal(v) or (e.subformula = {} and rootSettled(s, e.status.position))", spec.root)
        } else {
            "isFinal(v)".to_owned()
        };
        let _ = writeln!(
            out,
            "function reportActs using v(s: State, wm: Boolean, hz: Integer): StepR pure {{ \
                let r = fold<ActRep>(a, e in s.activations: \
                    let v = elemVal(s, e, hz) in \
                    let c = codeOf(v) in \
                    let rep = if c = 3 then wm else c != e.status.rep in \
                    let fin = {fin_cond} in \
                    ActRep {{ acts: if fin then a.acts else pushA(a.acts, actRep(e, c)), \
                              results: if rep then push2(a.results, resultOf(e.subformula, e.origin_tick, c)) else a.results, \
                              fins: if fin and isUnb(e.subformula) then pushF(a.fins, Fin {{ u: e.subformula, p: e.status.position, w: v }}) else a.fins }}, \
                    identity: ActRep {{ acts: noActs(), results: noResults(), fins: noFins() }}) in \
                let s2 = with_activations(s, r.acts) in \
                mk(fold<StateAcc>(a, f in r.fins: StateAcc {{ s: attach(a.s, f.u, f.p, f.w) }}, identity: StateAcc {{ s: s2 }}).s, r.results) }}\n\
             function reportAll using v(s: State, wm: Boolean, hz: Integer): StepR pure {{ \
                let c = {report_cells} in \
                joinR(c, reportActs(c.state, wm, hz)) }}"
        );
        // Entries.
        let zero_buffers = |s: &str| {
            let mut text = s.to_owned();
            for n in 0..self.buffers.len() {
                text = format!("with_buffer_{n}({text}, noCells())");
            }
            text
        };
        let gap_results = {
            let mut text = String::from("let g0 = noResults() in ");
            let mut prev = "g0".to_owned();
            for (n, id) in asc.iter().enumerate() {
                let b = self.buffer_of(*id);
                let name = format!("g{}", n + 1);
                let _ = write!(
                    text,
                    "let {name} = flatten(pairResults({prev}, map(x in filter(x in s.buffer_{b}: not x.lost and x.rep = {pending}): failedOf({id}, x.tf, gap)))) in ",
                    pending = code::PENDING
                );
                prev = name;
            }
            let _ = write!(text, "{prev}");
            text
        };
        let init_buffers = (0..self.buffers.len())
            .map(|n| format!("buffer_{n}: noCells()"))
            .collect::<Vec<_>>()
            .join(", ");
        let init_buffers = if init_buffers.is_empty() {
            String::new()
        } else {
            format!("{init_buffers}, ")
        };
        let zero_state = zero_buffers("s");
        let _ = writeln!(
            out,
            "function {init} using v(): State pure {{ \
                State {{ now: 0, history_start: 0 - 1, {init_buffers}activations: noActs(), event_position: 0 }} }}\n\
             function onEventR using v(s: State, delta: Integer, i: {inputs}): StepR pure {{ \
                let now2 = s.now + delta in \
                let pos = s.event_position + 1 in \
                let atoms = {atoms} in \
                let active = {active} in \
                let hz = now2 in \
                let sA = dropAll(settleAll(s, hz), hz) in \
                let sB = pushAll(sA, now2, pos, atoms, hz) in \
                let lost = lostAll(sA, sB, now2, pos, hz) in \
                let sC = settleAll(sB, hz) in \
                let st = startAll(sC, now2, pos, atoms, active) in \
                let sD = with_activations(st.state, map(e in st.state.activations: foldEvtOne(st.state, e, atoms, pos, hz))) in \
                let rp = reportAll(sD, false, hz) in \
                mk(with_event_position(with_now(rp.state, now2), pos), sortRes(flatten(tripleResults(lost, st.results, rp.results)))) }}\n\
             function onWatermarkR using v(s: State, boundary: Integer, scope_in: Scope): StepR pure {{ \
                let hz = if s.now > boundary then s.now else boundary + 1 in \
                let rp = reportAll(settleAll(s, hz), true, hz) in \
                mk(dropAll(rp.state, hz), sortRes(rp.results)) }}\n\
             function onGapR using v(s: State, gap: Integer): StepR pure {{ \
                let now2 = s.now + gap in \
                {gap_results_text}\
                mk(with_history_start(with_now(with_activations({zero_state}, noActs()), now2), now2), sortRes(flatten(tripleResults(gcells, gacts, noResults())))) }}\n\
             function {on_event} using v(s: State, delta: Integer, i: {inputs}): Step pure {{ let r = onEventR(s, delta, i) in Step(r.state, r.results) }}\n\
             function {on_watermark} using v(s: State, boundary: Integer, scope_in: {scope}): Step pure {{ let r = onWatermarkR(s, boundary, scope_in) in Step(r.state, r.results) }}\n\
             function {on_gap} using v(s: State, gap: Integer): Step pure {{ let r = onGapR(s, gap) in Step(r.state, r.results) }}",
            init = names::INIT,
            on_event = names::ON_EVENT,
            on_watermark = names::ON_WATERMARK,
            on_gap = names::ON_GAP,
            inputs = names::INPUTS,
            scope = names::SCOPE,
            atoms = self.atoms_expr(),
            active = match spec.activation {
                ActivationKind::Origin => "pos = 1".to_owned(),
                ActivationKind::Each => "true".to_owned(),
                ActivationKind::EachGuarded => self.guard_expr(),
            },
            gap_results_text = format!(
                "let gcells = {gap_results} in \
                 let gacts = map(e in filter(e in s.activations: e.status.rep = {pending}): failedOf(e.subformula, e.origin_tick, gap)) in ",
                pending = code::PENDING
            ),
        );
        out
    }
}

/// The complete-V1 declarations of the plan's types and entries.
pub fn render(spec: &EntrySpec) -> String {
    let gen = Gen::new(spec);
    let mut out = String::new();
    let w = &mut out;
    let atom_fields = spec
        .atoms
        .iter()
        .map(|atom| format!("a{atom}: Boolean;"))
        .collect::<Vec<_>>()
        .join(" ");
    let inputs_fields = match &spec.clause {
        Some(clause) => clause
            .parameters
            .iter()
            .map(|(name, ty)| format!("{name}: {ty};"))
            .collect::<Vec<_>>()
            .join(" "),
        None => {
            let mut fields = atom_fields.clone();
            if spec.activation == ActivationKind::EachGuarded {
                fields.push_str(" guard: Boolean;");
            }
            fields
        }
    };
    let _ = writeln!(
        w,
        "record V {{ oT: Boolean; oF: Boolean; cT: Boolean; cF: Boolean; lost: Integer; atomLost: Integer; }}\n\
         record UVal {{ u: Integer; v: V; }}\n\
         record {atoms} {{ {atom_fields} }}\n\
         record Cell {{ pf: Integer; pl: Integer; tf: Integer; tl: Integer; lost: Boolean; hasSet: Boolean; fv: V; rep: Integer; inputs: {atoms}; uv: Sequence<UVal>; }}\n\
         type Cells = Sequence<Cell>;\n\
         record ActStatus {{ position: Integer; rep: Integer; acc: V; pfx: V; atoms: {atoms}; uv: Sequence<UVal>; }}\n\
         record Activation {{ subformula: Integer; origin_tick: Integer; status: ActStatus; }}\n\
         record {state} {{ {state_types} }}\n\
         record {inputs} {{ {inputs_fields} }}\n\
         record {scope} {{ subject: Integer; }}\n\
         record Instance {{ subject: String; subformula: Integer; origin_tick: Integer; }}\n\
         record ReadingGapExceeded {{ gap: Integer; max_reading_gap: Integer; }}\n\
         record LimitReached {{ limit: String; amount: Integer; setting: String; }}\n\
         union {outcome} {{ Failed(ReadingGapExceeded), False, Incomplete(LimitReached), Indeterminate, Pending, True }}\n\
         record {result} {{ instance: Instance; outcome: {outcome}; }}\n\
         type Results = Sequence<{result}>;\n\
         tuple Step({state}, Results);\n\
         record StepR {{ state: {state}; results: Results; }}\n\
         record Acc {{ acc: V; pfx: V; }}\n\
         record Flag {{ b: Boolean; }}\n\
         record CellsAcc {{ cells: Cells; }}\n\
         record RepAcc {{ cells: Cells; results: Results; }}\n\
         record ActsAcc {{ acts: Sequence<Activation>; }}\n\
         record Fin {{ u: Integer; p: Integer; w: V; }}\n\
         record ActRep {{ acts: Sequence<Activation>; results: Results; fins: Sequence<Fin>; }}\n\
         record StateAcc {{ s: {state}; }}\n\
         record FoundCell {{ found: Boolean; c: Cell; }}\n\
         record FoundAct {{ found: Boolean; a: Activation; }}\n\
         record FoundUv {{ found: Boolean; v: V; }}",
        atoms = names::ATOMS,
        state = names::STATE,
        state_types = gen.state_types(),
        inputs = names::INPUTS,
        scope = names::SCOPE,
        result = names::RESULT,
        outcome = names::OUTCOME,
    );
    // The value algebra.
    let _ = writeln!(
        w,
        "function vTrue using v(): V pure {{ V {{ oT: true, oF: false, cT: true, cF: false, lost: 0, atomLost: 0 }} }}\n\
         function vFalse using v(): V pure {{ V {{ oT: false, oF: true, cT: false, cF: true, lost: 0, atomLost: 0 }} }}\n\
         function vUnknown using v(): V pure {{ V {{ oT: true, oF: true, cT: true, cF: true, lost: 0, atomLost: 0 }} }}\n\
         function vLostBy using v(k: Integer): V pure {{ V {{ oT: true, oF: true, cT: true, cF: true, lost: k, atomLost: 0 }} }}\n\
         function vFuture using v(): V pure {{ V {{ oT: true, oF: true, cT: false, cF: true, lost: 0, atomLost: 0 }} }}\n\
         function vId using v(a: V): V pure {{ a }}\n\
         function vNot using v(a: V): V pure {{ V {{ oT: a.oF, oF: a.oT, cT: a.cF, cF: a.cT, lost: a.lost, atomLost: a.atomLost }} }}\n\
         function minPos using v(a: Integer, b: Integer): Integer pure {{ if a = 0 then b else if b = 0 then a else if a < b then a else b }}\n\
         function vAnd using v(a: V, b: V): V pure {{ V {{ oT: a.oT and b.oT, oF: a.oF or b.oF, cT: a.cT and b.cT, cF: a.cF or b.cF, \
            lost: if a.lost != 0 then a.lost else b.lost, atomLost: minPos(a.atomLost, b.atomLost) }} }}\n\
         function vAtomLost using v(k: Integer): V pure {{ V {{ oT: true, oF: true, cT: true, cF: true, lost: 0, atomLost: k }} }}\n\
         function vOr using v(a: V, b: V): V pure {{ vNot(vAnd(vNot(a), vNot(b))) }}\n\
         function vOf using v(b: Boolean): V pure {{ if b then vTrue() else vFalse() }}\n\
         function isFinal using v(a: V): Boolean pure {{ a.oT != a.oF }}"
    );
    // Outcome codes.
    let cap_of = {
        let mut text = String::new();
        for node in gen.intervals() {
            let _ = write!(text, "if k = {} then {} else ", node.id, gen.cap(node));
        }
        text.push('0');
        text
    };
    let _ = writeln!(
        w,
        "function capOf using v(k: Integer): Integer pure {{ {cap_of} }}\n\
         function codeOf using v(a: V): Integer pure {{ \
            if a.oT and not a.oF then {t} \
            else if not a.oT then {f} \
            else if a.atomLost > 0 then {atom_base} + a.atomLost - 1 \
            else if a.lost > 0 then (if a.lost = {act_lost} then {act} else {buf_base} + capOf(a.lost - 1)) \
            else if a.cT != a.cF then {pend} \
            else {ind} }}\n\
         function outcomeOf using v(c: Integer): {outcome} pure {{ \
            if c >= {buf_base} then {outcome}::Incomplete(LimitReached {{ limit: \"event_buffer\", amount: c - {buf_base}, setting: \"monitor.max_event_rate\" }}) \
            else if c >= {atom_base} then {outcome}::Incomplete(LimitReached {{ limit: \"atom_projection\", amount: c - {atom_base}, setting: \"monitor.atom_projection\" }}) \
            else if c = {t} then {outcome}::True \
            else if c = {f} then {outcome}::False \
            else if c = {pend} then {outcome}::Pending \
            else if c = {act} then {outcome}::Incomplete(LimitReached {{ limit: \"activation\", amount: {acap}, setting: \"monitor.max_activations\" }}) \
            else {outcome}::Indeterminate }}\n\
         function resultOf using v(node: Integer, tick: Integer, c: Integer): {result} pure {{ \
            {result} {{ instance: Instance {{ subject: \"{subject}\", subformula: node, origin_tick: tick }}, outcome: outcomeOf(c) }} }}\n\
         function failedOf using v(node: Integer, tick: Integer, gap: Integer): {result} pure {{ \
            {result} {{ instance: Instance {{ subject: \"{subject}\", subformula: node, origin_tick: tick }}, \
                outcome: {outcome}::Failed(ReadingGapExceeded {{ gap: gap, max_reading_gap: {gapmax} }}) }} }}",
        t = code::TRUE,
        f = code::FALSE,
        pend = code::PENDING,
        ind = code::INDETERMINATE,
        act = code::ACTIVATION,
        act_lost = ACTIVATION_LOST,
        atom_base = code::ATOM_BASE,
        buf_base = code::BUFFER_BASE,
        outcome = names::OUTCOME,
        result = names::RESULT,
        subject = spec.subject,
        gapmax = spec.max_reading_gap,
        acap = spec.activation_capacity,
    );
    if let Some(clause) = &spec.clause {
        let parameters = clause
            .parameters
            .iter()
            .map(|(name, ty)| format!("{name}: {ty}"))
            .collect::<Vec<_>>()
            .join(", ");
        for (atom, expression) in clause
            .atoms
            .iter()
            .filter(|(atom, _)| !spec.unprojectable.contains(atom))
        {
            let _ = writeln!(
                w,
                "function atom{atom} using {profile}({parameters}): Boolean pure {{ {expression} }}",
                profile = clause.profile
            );
        }
        if let Some(guard) = &clause.guard {
            let _ = writeln!(
                w,
                "function guardFn using {profile}({parameters}): Boolean pure {{ {guard} }}",
                profile = clause.profile
            );
        }
    }
    if let Some(unit) = &spec.tick_unit {
        let _ = writeln!(w, "record TickUnit {{ elapsed: Quantity<{unit}>; }}");
    }
    let zero_atoms = format!(
        "{} {{ {} }}",
        names::ATOMS,
        spec.atoms
            .iter()
            .map(|atom| format!("a{atom}: false"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let zero_status = format!(
        "ActStatus {{ position: 0, rep: 0, acc: vFalse(), pfx: vTrue(), atoms: zeroAtoms(), uv: noUv() }}"
    );
    let _ = writeln!(
        w,
        "function pairCells using v(a: Cells, b: Cells): Sequence<Cells> pure {{ sequence[a, b] }}\n\
         function pairResults using v(a: Results, b: Results): Sequence<Results> pure {{ sequence[a, b] }}\n\
         function tripleResults using v(a: Results, b: Results, c: Results): Sequence<Results> pure {{ sequence[a, b, c] }}\n\
         function pairActs using v(a: Sequence<Activation>, b: Sequence<Activation>): Sequence<Sequence<Activation>> pure {{ sequence[a, b] }}\n\
         function pairFins using v(a: Sequence<Fin>, b: Sequence<Fin>): Sequence<Sequence<Fin>> pure {{ sequence[a, b] }}\n\
         function pairUv using v(a: Sequence<UVal>, b: Sequence<UVal>): Sequence<Sequence<UVal>> pure {{ sequence[a, b] }}\n\
         function noActs using v(): Sequence<Activation> pure {{ sequence[] }}\n\
         function noFins using v(): Sequence<Fin> pure {{ sequence[] }}\n\
         function noUv using v(): Sequence<UVal> pure {{ sequence[] }}\n\
         function noCells using v(): Cells pure {{ sequence[] }}\n\
         function oneCell using v(c: Cell): Cells pure {{ sequence[c] }}\n\
         function noResults using v(): Results pure {{ sequence[] }}\n\
         function oneResult using v(r: {result}): Results pure {{ sequence[r] }}\n\
         function zeroAtoms using v(): {atoms} pure {{ {zero_atoms} }}\n\
         function zeroCell using v(): Cell pure {{ Cell {{ pf: 0, pl: 0, tf: 0, tl: 0, lost: false, hasSet: false, fv: vUnknown(), rep: 0, inputs: zeroAtoms(), uv: noUv() }} }}\n\
         function zeroAct using v(): Activation pure {{ Activation {{ subformula: 0, origin_tick: 0, status: {zero_status} }} }}\n\
         function push using v(s: Cells, c: Cell): Cells pure {{ flatten(pairCells(s, oneCell(c))) }}\n\
         function push2 using v(s: Results, r: {result}): Results pure {{ flatten(pairResults(s, oneResult(r))) }}\n\
         function pushA using v(s: Sequence<Activation>, a: Activation): Sequence<Activation> pure {{ flatten(pairActs(s, sequence[a])) }}\n\
         function pushF using v(s: Sequence<Fin>, f: Fin): Sequence<Fin> pure {{ flatten(pairFins(s, sequence[f])) }}\n\
         function joinR using v(a: StepR, b: StepR): StepR pure {{ mk(b.state, flatten(pairResults(a.results, b.results))) }}\n\
         function mk using v(s: {state}, r: Results): StepR pure {{ StepR {{ state: s, results: r }} }}\n\
         function lastOf using v(s: Cells): Cell pure {{ fold<CellsLast>(a, x in s: CellsLast {{ c: x }}, identity: CellsLast {{ c: zeroCell() }}).c }}\n\
         record CellsLast {{ c: Cell; }}\n\
         function dropLast using v(s: Cells): Cells pure {{ \
            fold<Keep>(k, x in s: \
                if k.j + 1 < size(s) then Keep {{ j: k.j + 1, kept: push(k.kept, x) }} \
                else Keep {{ j: k.j + 1, kept: k.kept }}, \
                identity: Keep {{ j: 0, kept: noCells() }}).kept }}\n\
         record Keep {{ j: Integer; kept: Cells; }}\n\
         function findCell using v(s: Cells, p: Integer): FoundCell pure {{ \
            fold<FoundCell>(a, x in s: if x.pf <= p and p <= x.pl then FoundCell {{ found: true, c: x }} else a, \
                identity: FoundCell {{ found: false, c: zeroCell() }}) }}\n\
         function findAct using v(s: Sequence<Activation>, u: Integer, p: Integer): FoundAct pure {{ \
            fold<FoundAct>(a, e in s: if e.subformula = u and e.status.position = p then FoundAct {{ found: true, a: e }} else a, \
                identity: FoundAct {{ found: false, a: zeroAct() }}) }}\n\
         function uvFind using v(uv: Sequence<UVal>, u: Integer): FoundUv pure {{ \
            fold<FoundUv>(a, x in uv: if x.u = u then FoundUv {{ found: true, v: x.v }} else a, \
                identity: FoundUv {{ found: false, v: vUnknown() }}) }}\n\
         function setCell using v(x: Cell, w: V): Cell pure {{ \
            Cell {{ pf: x.pf, pl: x.pl, tf: x.tf, tl: x.tl, lost: x.lost, hasSet: true, fv: w, rep: x.rep, inputs: x.inputs, uv: x.uv }} }}\n\
         function repCell using v(x: Cell, c: Integer): Cell pure {{ \
            Cell {{ pf: x.pf, pl: x.pl, tf: x.tf, tl: x.tl, lost: x.lost, hasSet: x.hasSet, fv: x.fv, rep: c, inputs: x.inputs, uv: x.uv }} }}\n\
         function uvCell using v(x: Cell, u: Integer, w: V): Cell pure {{ \
            Cell {{ pf: x.pf, pl: x.pl, tf: x.tf, tl: x.tl, lost: x.lost, hasSet: x.hasSet, fv: x.fv, rep: x.rep, inputs: x.inputs, uv: flatten(pairUv(x.uv, sequence[UVal {{ u: u, v: w }}])) }} }}\n\
         function actAcc using v(e: Activation, acc: V, pfx: V): Activation pure {{ \
            Activation {{ subformula: e.subformula, origin_tick: e.origin_tick, status: ActStatus {{ position: e.status.position, rep: e.status.rep, acc: acc, pfx: pfx, atoms: e.status.atoms, uv: e.status.uv }} }} }}\n\
         function actRep using v(e: Activation, c: Integer): Activation pure {{ \
            Activation {{ subformula: e.subformula, origin_tick: e.origin_tick, status: ActStatus {{ position: e.status.position, rep: c, acc: e.status.acc, pfx: e.status.pfx, atoms: e.status.atoms, uv: e.status.uv }} }} }}\n\
         function actUv using v(e: Activation, u: Integer, w: V): Activation pure {{ \
            Activation {{ subformula: e.subformula, origin_tick: e.origin_tick, status: ActStatus {{ position: e.status.position, rep: e.status.rep, acc: e.status.acc, pfx: e.status.pfx, atoms: e.status.atoms, uv: flatten(pairUv(e.status.uv, sequence[UVal {{ u: u, v: w }}])) }} }} }}\n\
         function resKey using v(r: {result}): Integer pure {{ r.instance.subformula * 1000000000000 + r.instance.origin_tick }}\n\
         function insertRes using v(rs: Results, r: {result}): Results pure {{ \
            flatten(tripleResults(filter(x in rs: resKey(x) <= resKey(r)), oneResult(r), filter(x in rs: resKey(x) > resKey(r)))) }}\n\
         function sortRes using v(rs: Results): Results pure {{ fold<Results>(a, x in rs: insertRes(a, x), identity: noResults()) }}\n\
         {setters}",
        result = names::RESULT,
        atoms = names::ATOMS,
        state = names::STATE,
        setters = gen.setters(),
    );
    for node in &spec.nodes {
        let _ = writeln!(w, "{}", gen.node_functions(node));
    }
    let _ = writeln!(w, "{}", gen.entries());
    out
}

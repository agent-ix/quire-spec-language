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
//! The generated program covers a clause with exactly one interval operator
//! whose operands hold no temporal operator (an *interval node*). Anything
//! else is refused by [`EntryRefusal`] with the construct named, never
//! planned partially.

use std::fmt::Write as _;

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::ToPrimitive;
use qsl_replay::{SubformulaId, TimedMonitorPlan};
use qsl_semantics::check::{BinaryTemporal, FormulaNode, UnaryTemporal};
use quire_exact::{Integer as ExactInteger, Rational};

/// A temporal-free Boolean formula over `holds` atoms, as the operands of an
/// interval node and the body of an unbounded root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoolExpr {
    /// `true` or `false`.
    Const(bool),
    /// The `holds` atom with this formula arena index.
    Atom(usize),
    /// `not`.
    Not(Box<BoolExpr>),
    /// `and`.
    And(Box<BoolExpr>, Box<BoolExpr>),
    /// `or`.
    Or(Box<BoolExpr>, Box<BoolExpr>),
    /// `implies`.
    Implies(Box<BoolExpr>, Box<BoolExpr>),
}

impl BoolExpr {
    /// The expression over the atom bit mask named `mask`.
    fn render(&self, mask: &str, bits: &[usize]) -> String {
        match self {
            Self::Const(value) => value.to_string(),
            Self::Atom(index) => {
                let _ = bits;
                format!("{mask}.a{index}")
            }
            Self::Not(inner) => format!("(not {})", inner.render(mask, bits)),
            Self::And(a, b) => format!("({} and {})", a.render(mask, bits), b.render(mask, bits)),
            Self::Or(a, b) => format!("({} or {})", a.render(mask, bits), b.render(mask, bits)),
            Self::Implies(a, b) => format!(
                "((not {}) or {})",
                a.render(mask, bits),
                b.render(mask, bits)
            ),
        }
    }
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

/// The interval node the entries monitor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntervalNode {
    /// Whether the operator looks to later events (`eventually`, `always`,
    /// `until`, `release`) rather than earlier ones.
    pub future: bool,
    /// Whether the operator is the dual (`always`, `historically`,
    /// `release`, `triggered`).
    pub dual: bool,
    /// The left operand of a binary operator.
    pub phi: Option<BoolExpr>,
    /// The (right) operand.
    pub psi: BoolExpr,
    /// The decision thresholds.
    pub thresholds: Thresholds,
    /// The buffer capacity premise.
    pub capacity: u64,
    /// The formula arena index, the instance's `subformula`.
    pub subformula: usize,
}

/// Everything the source generator needs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntrySpec {
    /// The atoms' formula indices, in input-field order.
    pub atoms: Vec<usize>,
    /// The interval node.
    pub node: IntervalNode,
    /// The subject text each result carries.
    pub subject: String,
    /// The plan's `max_reading_gap`.
    pub max_reading_gap: u64,
    /// The plan's `activation_capacity`.
    pub activation_capacity: u64,
}

/// Why the entries cannot be generated for a clause.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EntryRefusal {
    /// The formula has other than exactly one interval operator.
    #[error("the formula has {found} interval operators; exactly one is generated")]
    IntervalOperators {
        /// How many were found.
        found: usize,
    },
    /// A temporal operator sits inside another temporal operator's operand.
    #[error("temporal operator {node} is nested in a temporal operand")]
    NestedTemporal {
        /// The formula arena index.
        node: usize,
    },
    /// An operator has no interval (an unbounded operator) where a
    /// constant-size accumulator is not generated.
    #[error("unbounded temporal operator {node} is not generated")]
    Unbounded {
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

fn temporal_free(nodes: &[FormulaNode], index: usize) -> Option<BoolExpr> {
    let child =
        |id: qsl_semantics::check::FormulaNodeId| temporal_free(nodes, id.index()).map(Box::new);
    Some(match &nodes[index] {
        FormulaNode::Constant(value) => BoolExpr::Const(*value),
        FormulaNode::Holds { .. } => BoolExpr::Atom(index),
        FormulaNode::Not(a) => BoolExpr::Not(child(*a)?),
        FormulaNode::And(a, b) => BoolExpr::And(child(*a)?, child(*b)?),
        FormulaNode::Or(a, b) => BoolExpr::Or(child(*a)?, child(*b)?),
        FormulaNode::Implies(a, b) => BoolExpr::Implies(child(*a)?, child(*b)?),
        FormulaNode::Aggregate { .. } | FormulaNode::Unary { .. } | FormulaNode::Binary { .. } => {
            return None
        }
    })
}

/// Derive the generator input from a produced plan, or name the construct
/// the generator does not cover.
pub fn spec_from_plan(plan: &TimedMonitorPlan) -> Result<EntrySpec, EntryRefusal> {
    let nodes = plan.program().formula().nodes();
    let temporal: Vec<usize> = (0..nodes.len())
        .filter(|index| {
            matches!(
                nodes[*index],
                FormulaNode::Unary { .. } | FormulaNode::Binary { .. }
            )
        })
        .collect();
    for node in &temporal {
        if !plan.intervals().iter().any(|(id, _)| id.index() == *node) {
            return Err(EntryRefusal::Unbounded { node: *node });
        }
    }
    let [node] = temporal.as_slice() else {
        return Err(EntryRefusal::IntervalOperators {
            found: temporal.len(),
        });
    };
    let node = *node;
    let (future, dual, phi, psi) = match &nodes[node] {
        FormulaNode::Unary {
            operator, operand, ..
        } => (
            matches!(operator, UnaryTemporal::Eventually | UnaryTemporal::Always),
            matches!(
                operator,
                UnaryTemporal::Always | UnaryTemporal::Historically
            ),
            None,
            temporal_free(nodes, operand.index()),
        ),
        FormulaNode::Binary {
            operator,
            left,
            right,
            ..
        } => (
            matches!(operator, BinaryTemporal::Until | BinaryTemporal::Release),
            matches!(
                operator,
                BinaryTemporal::Release | BinaryTemporal::Triggered
            ),
            Some(temporal_free(nodes, left.index()).ok_or(EntryRefusal::NestedTemporal { node })?),
            temporal_free(nodes, right.index()),
        ),
        _ => return Err(EntryRefusal::IntervalOperators { found: 0 }),
    };
    let psi = psi.ok_or(EntryRefusal::NestedTemporal { node })?;
    let interval = plan
        .intervals()
        .iter()
        .find(|(id, _)| *id == SubformulaId::new(node))
        .map(|(_, interval)| interval)
        .ok_or(EntryRefusal::Unbounded { node })?;
    let thresholds = Thresholds::of(
        interval.lower(),
        interval.lower_closed(),
        interval.upper(),
        interval.upper_closed(),
        plan.uncertainty(),
    )
    .ok_or(EntryRefusal::Unbounded { node })?;
    let capacity = plan
        .buffers()
        .iter()
        .find(|(id, _)| id.index() == node)
        .map_or(1, |(_, capacity)| *capacity);
    Ok(EntrySpec {
        atoms: (0..nodes.len())
            .filter(|index| matches!(nodes[*index], FormulaNode::Holds { .. }))
            .collect(),
        node: IntervalNode {
            future,
            dual,
            phi,
            psi,
            thresholds,
            capacity,
            subformula: node,
        },
        subject: plan.subject().clause_name().to_owned(),
        max_reading_gap: plan.max_reading_gap(),
        activation_capacity: plan.activation_capacity(),
    })
}

/// The outcome codes the entries keep in a cell's `out`.
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
    /// `Incomplete(LimitReached)`.
    pub const INCOMPLETE: i64 = 5;
    /// `Failed(ReadingGapExceeded)`.
    pub const FAILED: i64 = 6;
    /// Added to a code once the instance can no longer change.
    pub const SETTLED: i64 = 10;
}

/// The identifiers of the generated declarations.
pub mod names {
    /// The state record.
    pub const STATE: &str = "State";
    /// The inputs record.
    pub const INPUTS: &str = "Inputs";
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

fn sig(value: i128) -> String {
    if value < 0 {
        format!("(0 - {})", -value)
    } else {
        value.to_string()
    }
}

fn cell_literal(tick: &str, inputs: &str) -> String {
    format!("Cell {{ tFirst: {tick}, tLast: {tick}, lost: false, inputs: {inputs}, out: 0 }}")
}

fn state_with(now: &str, start: &str, cells: &str, spec: &EntrySpec) -> String {
    let _ = spec;
    format!(
        "{state} {{ now: {now}, history_start: {start}, buffer_0: {cells}, activations: s.activations }}",
        state = names::STATE,
    )
}

fn on_event_body(spec: &EntrySpec) -> String {
    let node = &spec.node;
    let cap = node.capacity.max(1);
    let none_hi = sig(node.thresholds.none_hi);
    let pending = code::PENDING;
    let incomplete = code::INCOMPLETE;
    let settled = code::SETTLED;
    let c = cell_literal("now2", "i");
    if !node.future {
        return format!(
            "let now2 = s.now + delta in \
             let c = {c} in \
             let kept = filter(x in s.buffer_0: now2 - x.tLast < {none_hi}) in \
             let full = size(kept) >= {cap} in \
             let cells = if full then push(dropLast(kept), merged(lastOf(kept), c, {settled} + {incomplete})) else push(kept, c) in \
             mk({state}, oneResult(resultOf(now2, codeOf(evalAt(cells, size(cells) - 1, s.history_start)))))",
            state = state_with("now2", "s.history_start", "cells", spec),
        );
    }
    format!(
        "let now2 = s.now + delta in \
         let c = {c} in \
         let full = size(s.buffer_0) >= {cap} in \
         let last = if size(s.buffer_0) = 0 then c else lastOf(s.buffer_0) in \
         let cells = if full then push(dropLast(s.buffer_0), merged(last, c, {settled} + {incomplete})) else push(s.buffer_0, c) in \
         let lostNew = if full then oneResult(resultOf(now2, {incomplete})) else noResults() in \
         let lostOld = if full and not last.lost and last.out < {settled} then oneResult(resultOf(last.tFirst, {incomplete})) else noResults() in \
         let sw = {sweep} in \
         let tail = pruned(sw.cells) in \
         mk({state}, flatten(tripleResults(lostOld, lostNew, sw.results)))",
        sweep = sweep("cells", "now2", false, spec, pending, settled),
        state = state_with("now2", "s.history_start", "tail", spec),
    )
}

/// The sweep over `cells` at `horizon`: re-evaluate every open instance and
/// report the changed ones.
fn sweep(
    cells: &str,
    horizon: &str,
    watermark: bool,
    spec: &EntrySpec,
    pending: i64,
    settled: i64,
) -> String {
    let node = &spec.node;
    let none_hi = sig(node.thresholds.none_hi);
    format!(
        "fold<Sweep>(a, x in {cells}: \
            if x.lost or x.out >= {settled} then Sweep {{ j: a.j + 1, cells: push(a.cells, x), results: a.results }} \
            else \
              let closed = {horizon} - x.tFirst >= {none_hi} in \
              let v = evalAt({cells}, a.j, closed) in \
              let code = codeOf(v) in \
              let fin = isFinal(v) or closed in \
              let report = if code = {pending} then {watermark} else code != x.out in \
              Sweep {{ j: a.j + 1, \
                cells: push(a.cells, Cell {{ tFirst: x.tFirst, tLast: x.tLast, lost: false, inputs: x.inputs, out: code + (if fin then {settled} else 0) }}), \
                results: if report then push2(a.results, resultOf(x.tFirst, code)) else a.results }}, \
            identity: Sweep {{ j: 0, cells: noCells(), results: noResults() }})"
    )
}

fn on_watermark_body(spec: &EntrySpec) -> String {
    let node = &spec.node;
    let none_hi = sig(node.thresholds.none_hi);
    let settled = code::SETTLED;
    let pending = code::PENDING;
    if !node.future {
        return format!(
            "let hz = if s.now > boundary then s.now else boundary + 1 in \
             mk({state}, noResults())",
            state = state_with(
                "s.now",
                "s.history_start",
                &format!("filter(x in s.buffer_0: (if s.now > boundary then s.now else boundary + 1) - x.tLast < {none_hi})"),
                spec
            ),
        );
    }
    format!(
        "let hz = if s.now > boundary then s.now else boundary + 1 in \
         let sw = {sweep} in \
         mk({state}, sw.results)",
        sweep =
            sweep("s.buffer_0", "hz", true, spec, pending, settled).replace("a.cells", "a.cells"),
        state = state_with("s.now", "s.history_start", "pruned(sw.cells)", spec),
    )
}

fn on_gap_body(spec: &EntrySpec) -> String {
    let pending = code::PENDING;
    let results = if spec.node.future {
        format!(
            "fold<Sweep>(a, x in s.buffer_0: \
                if not x.lost and x.out = {pending} then Sweep {{ j: a.j + 1, cells: a.cells, results: push2(a.results, failedOf(x.tFirst, gap)) }} \
                else Sweep {{ j: a.j + 1, cells: a.cells, results: a.results }}, \
                identity: Sweep {{ j: 0, cells: noCells(), results: noResults() }}).results"
        )
    } else {
        "noResults()".to_owned()
    };
    format!(
        "let now2 = s.now + gap in mk({state}, {results})",
        state = state_with("now2", "now2", "noCells()", spec),
    )
}

/// The complete-V1 declarations of the plan's types and entries.
pub fn render(spec: &EntrySpec) -> String {
    let node = &spec.node;
    let cap = node.capacity.max(1);
    let t = node.thresholds;
    let bits = spec.atoms.clone();
    let mut out = String::new();
    let w = &mut out;
    let _ = writeln!(
        w,
        "type Cells = Sequence<Cell>;
         record Idx {{ j: Integer; ix: Integer; }}
         record Last {{ c: Cell; }}
         record V {{ oT: Boolean; oF: Boolean; cT: Boolean; cF: Boolean; lost: Boolean; }}\n\
         record Cell {{ tFirst: Integer; tLast: Integer; lost: Boolean; inputs: {inputs}; out: Integer; }}\n\
         record Activation {{ subformula: Integer; origin_tick: Integer; status: Integer; }}\n\
         record {state} {{ now: Integer; history_start: Integer; buffer_0: Cells; activations: Sequence<Activation>; }}\n\
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
         record Acc {{ j: Integer; acc: V; pfx: V; }}\n\
         record Sweep {{ j: Integer; cells: Cells; results: Results; }}\n\
         record Keep {{ j: Integer; kept: Cells; }}",
        state = names::STATE,
        inputs = names::INPUTS,
        scope = names::SCOPE,
        result = names::RESULT,
        outcome = names::OUTCOME,
        inputs_fields = spec
            .atoms
            .iter()
            .map(|atom| format!("a{atom}: Boolean;"))
            .collect::<Vec<_>>()
            .join(" "),
    );
    // The value algebra.
    let psi_text = format!("vOf({})", node.psi.render("c.inputs", &bits));
    let _ = writeln!(
        w,
        "function vTrue using v(): V pure {{ V {{ oT: true, oF: false, cT: true, cF: false, lost: false }} }}\n\
         function vFalse using v(): V pure {{ V {{ oT: false, oF: true, cT: false, cF: true, lost: false }} }}\n\
         function vUnknown using v(): V pure {{ V {{ oT: true, oF: true, cT: true, cF: true, lost: false }} }}\n\
         function vLost using v(): V pure {{ V {{ oT: true, oF: true, cT: true, cF: true, lost: true }} }}\n\
         function vFuture using v(): V pure {{ V {{ oT: true, oF: true, cT: false, cF: true, lost: false }} }}\n\
         function vNot using v(a: V): V pure {{ V {{ oT: a.oF, oF: a.oT, cT: a.cF, cF: a.cT, lost: a.lost }} }}\n\
         function vAnd using v(a: V, b: V): V pure {{ V {{ oT: a.oT and b.oT, oF: a.oF or b.oF, cT: a.cT and b.cT, cF: a.cF or b.cF, lost: a.lost or b.lost }} }}\n\
         function vOr using v(a: V, b: V): V pure {{ vNot(vAnd(vNot(a), vNot(b))) }}\n\
         function vOf using v(b: Boolean): V pure {{ if b then vTrue() else vFalse() }}\n\
         function vPolar using v(a: V): V pure {{ {polar} }}\n\
         function memberKind using v(dmin: Integer, dmax: Integer, same: Boolean): Integer pure {{ \
            if same then (if {zero} then 2 else 0) \
            else if dmin >= {all_lo} and dmax <= {all_hi} then 2 \
            else if dmax <= {none_lo} or dmin >= {none_hi} then 0 \
            else 1 }}\n\
         function member using v(dmin: Integer, dmax: Integer, same: Boolean): V pure {{ \
            let k = memberKind(dmin, dmax, same) in \
            if k = 2 then vTrue() else if k = 0 then vFalse() else vUnknown() }}\n\
         function phi using v(c: Cell): V pure {{ {phi} }}\n\
         function psi using v(c: Cell): V pure {{ if c.lost then vLost() else vPolar({psi}) }}",
        polar = if node.dual { "vNot(a)" } else { "a" },
        zero = t.zero_in,
        all_lo = sig(t.all_lo),
        all_hi = sig(t.all_hi),
        none_lo = sig(t.none_lo),
        none_hi = sig(t.none_hi),
        phi = match &node.phi {
            Some(expr) => format!(
                "if c.lost then vLost() else vPolar(vOf({}))",
                expr.render("c.inputs", &bits)
            ),
            None => "vTrue()".to_owned(),
        },
        psi = psi_text,
    );
    // Instance value.
    if node.future {
        let _ = writeln!(
            w,
            "function evalAt using v(cells: Cells, k: Integer, closed: Boolean): V pure {{ \
                let ti = tickAt(cells, k) in \
                let r = fold<Acc>(a, x in cells: \
                    if a.j < k then Acc {{ j: a.j + 1, acc: a.acc, pfx: a.pfx }} \
                    else Acc {{ j: a.j + 1, \
                        acc: vOr(a.acc, vAnd(vAnd(a.pfx, member(x.tFirst - ti, x.tLast - ti, a.j = k and not x.lost)), psi(x))), \
                        pfx: vAnd(a.pfx, phi(x)) }}, \
                    identity: Acc {{ j: 0, acc: vFalse(), pfx: vTrue() }}) in \
                vPolar2(if closed then r.acc else vOr(r.acc, vAnd(vFuture(), r.pfx))) }}\n\
             function vPolar2 using v(a: V): V pure {{ {polar2} }}",
            polar2 = if node.dual { "vNot(a)" } else { "a" },
        );
    } else {
        let _ = writeln!(
            w,
            "function evalAt using v(cells: Cells, k: Integer, start: Integer): V pure {{ \
                let tk = tickAt(cells, k) in \
                let r = fold<Acc>(a, x in cells: \
                    if a.j > k then Acc {{ j: a.j + 1, acc: a.acc, pfx: a.pfx }} \
                    else Acc {{ j: a.j + 1, \
                        acc: vOr(vAnd(a.acc, phi(x)), vAnd(member(tk - x.tLast, tk - x.tFirst, a.j = k and not x.lost), psi(x))), \
                        pfx: a.pfx }}, \
                    identity: Acc {{ j: 0, acc: vFalse(), pfx: vTrue() }}) in \
                let early = start >= 0 and memberKind(tk - (start - 1), tk - (start - 1), false) > 0 in \
                vPolar2(if early then vOr(r.acc, vUnknown()) else r.acc) }}\n\
             function vPolar2 using v(a: V): V pure {{ {polar2} }}",
            polar2 = if node.dual { "vNot(a)" } else { "a" },
        );
    }
    // Outcome codes.
    let _ = writeln!(
        w,
        "function codeOf using v(a: V): Integer pure {{ \
            if a.oT and not a.oF then {t} \
            else if not a.oT then {f} \
            else if a.lost then {inc} \
            else if a.cT != a.cF then {pend} \
            else {ind} }}\n\
         function isFinal using v(a: V): Boolean pure {{ a.oT != a.oF }}\n\
         function outcomeOf using v(c: Integer, cap: Integer): {outcome} pure {{ \
            if c = {t} then {outcome}::True \
            else if c = {f} then {outcome}::False \
            else if c = {pend} then {outcome}::Pending \
            else if c = {inc} then {outcome}::Incomplete(LimitReached {{ limit: \"event_buffer\", amount: cap, setting: \"monitor.max_event_rate\" }}) \
            else {outcome}::Indeterminate }}\n\
         function resultOf using v(tick: Integer, c: Integer): {result} pure {{ \
            {result} {{ instance: Instance {{ subject: \"{subject}\", subformula: {sub}, origin_tick: tick }}, outcome: outcomeOf(c, {cap}) }} }}\n\
         function failedOf using v(tick: Integer, gap: Integer): {result} pure {{ \
            {result} {{ instance: Instance {{ subject: \"{subject}\", subformula: {sub}, origin_tick: tick }}, \
                outcome: {outcome}::Failed(ReadingGapExceeded {{ gap: gap, max_reading_gap: {gapmax} }}) }} }}",
        t = code::TRUE,
        f = code::FALSE,
        inc = code::INCOMPLETE,
        pend = code::PENDING,
        ind = code::INDETERMINATE,
        outcome = names::OUTCOME,
        result = names::RESULT,
        subject = spec.subject,
        sub = node.subformula,
        cap = cap,
        gapmax = spec.max_reading_gap,
    );
    let zero_inputs = format!(
        "{} {{ {} }}",
        names::INPUTS,
        spec.atoms
            .iter()
            .map(|atom| format!("a{atom}: false"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    // init.
    let _ = writeln!(
        w,
        "function {init} using v(): {state} pure {{ \
            {state} {{ now: 0, history_start: 0 - 1, buffer_0: noCells(), activations: noActivations() }} }}",
        init = names::INIT,
        state = names::STATE,
    );
    let _ = writeln!(
        w,
        "function pairCells using v(a: Cells, b: Cells): Sequence<Cells> pure {{ sequence[a, b] }}\n\
         function pairResults using v(a: Results, b: Results): Sequence<Results> pure {{ sequence[a, b] }}\n\
         function tripleResults using v(a: Results, b: Results, c: Results): Sequence<Results> pure {{ sequence[a, b, c] }}\n\
         function noActivations using v(): Sequence<Activation> pure {{ sequence[] }}\n\
         function noCells using v(): Cells pure {{ sequence[] }}\n\
         function oneCell using v(c: Cell): Cells pure {{ sequence[c] }}\n\
         function noResults using v(): Results pure {{ sequence[] }}\n\
         function oneResult using v(r: {result}): Results pure {{ sequence[r] }}\n\
         function zeroInputs using v(): {inputs} pure {{ {zero_inputs} }}\n\
         function push using v(s: Cells, c: Cell): Cells pure {{ flatten(pairCells(s, oneCell(c))) }}\n\
         function push2 using v(s: Results, r: {result}): Results pure {{ flatten(pairResults(s, oneResult(r))) }}\n\
         function lastOf using v(s: Cells): Cell pure {{ \
            fold<Last>(a, x in s: Last {{ c: x }}, \
                identity: Last {{ c: Cell {{ tFirst: 0, tLast: 0, lost: false, inputs: zeroInputs(), out: 0 }} }}).c }}\n\
         function tickAt using v(s: Cells, k: Integer): Integer pure {{ \
            fold<Idx>(a, x in s: \
                if a.j = k then Idx {{ j: a.j + 1, ix: x.tFirst }} else Idx {{ j: a.j + 1, ix: a.ix }}, \
                identity: Idx {{ j: 0, ix: 0 }}).ix }}\n\
         function dropLast using v(s: Cells): Cells pure {{ \
            fold<Keep>(k, x in s: \
                if k.j + 1 < size(s) then Keep {{ j: k.j + 1, kept: push(k.kept, x) }} \
                else Keep {{ j: k.j + 1, kept: k.kept }}, \
                identity: Keep {{ j: 0, kept: noCells() }}).kept }}\n\
         function fromIndex using v(s: Cells, ix: Integer): Cells pure {{ \
            fold<Keep>(k, x in s: \
                if k.j >= ix then Keep {{ j: k.j + 1, kept: push(k.kept, x) }} \
                else Keep {{ j: k.j + 1, kept: k.kept }}, \
                identity: Keep {{ j: 0, kept: noCells() }}).kept }}\n\
         function merged using v(last: Cell, c: Cell, out: Integer): Cell pure {{ \
            Cell {{ tFirst: last.tFirst, tLast: c.tLast, lost: true, inputs: zeroInputs(), out: out }} }}\n\
         function firstOpen using v(s: Cells): Integer pure {{ \
            fold<Idx>(a, x in s: \
                if a.ix < 0 and not x.lost and x.out < {settled} then Idx {{ j: a.j + 1, ix: a.j }} \
                else Idx {{ j: a.j + 1, ix: a.ix }}, \
                identity: Idx {{ j: 0, ix: 0 - 1 }}).ix }}\n\
         function pruned using v(s: Cells): Cells pure {{ \
            let ix = firstOpen(s) in \
            if ix >= 0 then fromIndex(s, ix) \
            else if size(s) = 0 then s \
            else if lastOf(s).lost then noCells() else oneCell(lastOf(s)) }}",
        settled = code::SETTLED,
        result = names::RESULT,
        inputs = names::INPUTS,
        zero_inputs = zero_inputs,
    );
    let _ = writeln!(
        w,
        "function mk using v(s: {state}, r: Results): StepR pure {{ StepR {{ state: s, results: r }} }}\n\
         function onEventR using v(s: {state}, delta: Integer, i: {inputs}): StepR pure {{ {on_event_body} }}\n\
         function onWatermarkR using v(s: {state}, boundary: Integer, scope_in: {scope}): StepR pure {{ {on_watermark_body} }}\n\
         function onGapR using v(s: {state}, gap: Integer): StepR pure {{ {on_gap_body} }}\n\
         function {on_event} using v(s: {state}, delta: Integer, i: {inputs}): Step pure {{ let r = onEventR(s, delta, i) in Step(r.state, r.results) }}\n\
         function {on_watermark} using v(s: {state}, boundary: Integer, scope_in: {scope}): Step pure {{ let r = onWatermarkR(s, boundary, scope_in) in Step(r.state, r.results) }}\n\
         function {on_gap} using v(s: {state}, gap: Integer): Step pure {{ let r = onGapR(s, gap) in Step(r.state, r.results) }}",
        on_event = names::ON_EVENT,
        on_watermark = names::ON_WATERMARK,
        on_gap = names::ON_GAP,
        state = names::STATE,
        inputs = names::INPUTS,
        scope = names::SCOPE,
        on_event_body = on_event_body(spec),
        on_watermark_body = on_watermark_body(spec),
        on_gap_body = on_gap_body(spec),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_atom_bit_mask_follows_input_order() {
        let expr = BoolExpr::And(
            Box::new(BoolExpr::Atom(7)),
            Box::new(BoolExpr::Not(Box::new(BoolExpr::Atom(3)))),
        );
        assert_eq!(expr.render("m", &[3, 7]), "(m.a7 and (not m.a3))");
    }
}

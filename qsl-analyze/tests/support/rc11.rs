// SPDX-License-Identifier: AGPL-3.0-or-later
//! An RC11 execution-graph checker for small litmus programs, written from
//! the RC11 definition (Lahav et al., PLDI 2017) and sharing nothing with
//! `qsl-eval`'s `Ra` component.
//!
//! A candidate execution fixes, for every read, the write it reads (`rf`) and,
//! for every location, a total modification order (`mo`). It is allowed when
//! `hb` is irreflexive, `hb ; eco?` is irreflexive (coherence), `sb ∪ rf` is
//! acyclic (no thin air) and `psc` is acyclic, which is the existence of the
//! total SC order RC11 asks for. Programs have atomic accesses and fences,
//! no read-modify-write and no non-atomic access; every location starts at 0.

// Relation algebra over event indices reads as the RC11 definition when indexed.
#![allow(clippy::needless_range_loop)]

use std::collections::BTreeSet;

/// The memory order of an access or fence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Order {
    Relaxed,
    Acquire,
    Release,
    SeqCst,
}

impl Order {
    fn releases(self) -> bool {
        matches!(self, Self::Release | Self::SeqCst)
    }

    fn acquires(self) -> bool {
        matches!(self, Self::Acquire | Self::SeqCst)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Write,
    Read,
    Fence,
}

/// One statement of a thread.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Op {
    kind: Kind,
    loc: usize,
    order: Order,
    /// The register a read fills (1-based) or the value a write stores.
    operand: i64,
}

impl Op {
    /// A store of `value` to location `loc`.
    pub(crate) fn store(loc: usize, value: i64, order: Order) -> Self {
        Self {
            kind: Kind::Write,
            loc,
            order,
            operand: value,
        }
    }

    /// A load of location `loc` into register `register` (1-based).
    pub(crate) fn load(loc: usize, register: usize, order: Order) -> Self {
        Self {
            kind: Kind::Read,
            loc,
            order,
            operand: i64::try_from(register).expect("a small register"),
        }
    }

    /// A fence.
    pub(crate) fn fence(order: Order) -> Self {
        Self {
            kind: Kind::Fence,
            loc: 0,
            order,
            operand: 0,
        }
    }
}

impl Op {
    /// The QSL attempt or fence named `node` that runs this statement.
    pub(crate) fn render(&self, node: &str) -> String {
        let order = match self.order {
            Order::Relaxed => "relaxed",
            Order::Acquire => "acquire",
            Order::Release => "release",
            Order::SeqCst => "seq_cst",
        };
        let cell = ["X", "Y", "Z"][self.loc];
        match self.kind {
            Kind::Fence => format!("fence {node} ordering {order};"),
            Kind::Write => format!(
                "attempt {node} by w on M::Cell::set ordering {order} \
                 contracts [Is{cell}, Store{}] as (b{node}: Boolean) {{ true }};",
                ["Zero", "One", "Two", "Three"][usize::try_from(self.operand).expect("small")]
            ),
            Kind::Read => format!(
                "attempt {node} by c on M::Core::read{n} ordering {order} \
                 contracts [Reads{cell}{n}, Read{n}] as (b{node}: Boolean) {{ true }};",
                n = self.operand
            ),
        }
    }
}

/// The value a register holds before its load runs, as in the engine.
pub(crate) const UNREAD: i64 = 3;
/// The registers a program may fill.
pub(crate) const REGISTERS: usize = 4;

struct Event {
    /// `None` for an initial write.
    thread: Option<usize>,
    index: usize,
    op: Op,
}

type Matrix = Vec<Vec<bool>>;

fn matrix(n: usize) -> Matrix {
    vec![vec![false; n]; n]
}

fn closure(relation: &Matrix) -> Matrix {
    let n = relation.len();
    let mut out = relation.clone();
    for k in 0..n {
        for i in 0..n {
            if out[i][k] {
                for j in 0..n {
                    if out[k][j] {
                        out[i][j] = true;
                    }
                }
            }
        }
    }
    out
}

fn acyclic(relation: &Matrix) -> bool {
    let closed = closure(relation);
    (0..relation.len()).all(|i| !closed[i][i])
}

/// The composition `a ; b`.
fn compose(a: &Matrix, b: &Matrix) -> Matrix {
    let n = a.len();
    let mut out = matrix(n);
    for i in 0..n {
        for k in 0..n {
            if a[i][k] {
                for j in 0..n {
                    if b[k][j] {
                        out[i][j] = true;
                    }
                }
            }
        }
    }
    out
}

fn union(a: &Matrix, b: &Matrix) -> Matrix {
    let mut out = a.clone();
    for (row, other) in out.iter_mut().zip(b) {
        for (cell, &set) in row.iter_mut().zip(other) {
            *cell |= set;
        }
    }
    out
}

/// `relation` with the identity added (`r?`).
fn optional(relation: &Matrix) -> Matrix {
    let mut out = relation.clone();
    for (i, row) in out.iter_mut().enumerate() {
        row[i] = true;
    }
    out
}

fn permutations(items: &[usize]) -> Vec<Vec<usize>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut out = Vec::new();
    for i in 0..items.len() {
        let mut rest = items.to_vec();
        let head = rest.remove(i);
        for mut tail in permutations(&rest) {
            tail.insert(0, head);
            out.push(tail);
        }
    }
    out
}

/// The register outcomes (`r1..r4`) of every RC11-allowed execution of
/// `threads` over `locations` locations.
pub(crate) fn outcomes(threads: &[Vec<Op>], locations: usize) -> BTreeSet<Vec<i64>> {
    let mut events: Vec<Event> = (0..locations)
        .map(|loc| Event {
            thread: None,
            index: 0,
            op: Op::store(loc, 0, Order::Relaxed),
        })
        .collect();
    for (thread, ops) in threads.iter().enumerate() {
        for (index, op) in ops.iter().enumerate() {
            events.push(Event {
                thread: Some(thread),
                index,
                op: *op,
            });
        }
    }
    let n = events.len();
    let writes: Vec<usize> = (0..n)
        .filter(|&e| events[e].op.kind == Kind::Write)
        .collect();
    let reads: Vec<usize> = (0..n)
        .filter(|&e| events[e].op.kind == Kind::Read)
        .collect();

    let mut sb = matrix(n);
    for a in 0..n {
        for b in 0..n {
            sb[a][b] = events[a].thread.is_some()
                && events[a].thread == events[b].thread
                && events[a].index < events[b].index;
        }
    }
    let candidates: Vec<Vec<usize>> = reads
        .iter()
        .map(|&r| {
            writes
                .iter()
                .copied()
                .filter(|&w| events[w].op.loc == events[r].op.loc)
                .collect()
        })
        .collect();
    let per_location: Vec<Vec<Vec<usize>>> = (0..locations)
        .map(|loc| {
            let later: Vec<usize> = writes
                .iter()
                .copied()
                .filter(|&w| events[w].op.loc == loc && events[w].thread.is_some())
                .collect();
            permutations(&later)
                .into_iter()
                .map(|mut order| {
                    order.insert(0, loc);
                    order
                })
                .collect()
        })
        .collect();

    let mut found = BTreeSet::new();
    let mut choice = vec![0usize; reads.len()];
    loop {
        let mut order_choice = vec![0usize; locations];
        loop {
            let mut rf = matrix(n);
            for (slot, &r) in reads.iter().enumerate() {
                rf[candidates[slot][choice[slot]]][r] = true;
            }
            let mut mo = matrix(n);
            for (loc, orders) in per_location.iter().enumerate() {
                let order = &orders[order_choice[loc]];
                for (i, &a) in order.iter().enumerate() {
                    for &b in &order[i + 1..] {
                        mo[a][b] = true;
                    }
                }
            }
            if allowed(&events, &sb, &rf, &mo) {
                let mut registers = vec![UNREAD; REGISTERS];
                for (slot, &r) in reads.iter().enumerate() {
                    let register = usize::try_from(events[r].op.operand).expect("a register");
                    registers[register - 1] = events[candidates[slot][choice[slot]]].op.operand;
                }
                found.insert(registers);
            }
            if !advance(
                &mut order_choice,
                &per_location.iter().map(Vec::len).collect::<Vec<_>>(),
            ) {
                break;
            }
        }
        if !advance(
            &mut choice,
            &candidates.iter().map(Vec::len).collect::<Vec<_>>(),
        ) {
            break;
        }
    }
    found
}

/// Step the mixed-radix counter `digits`; false once it wraps.
fn advance(digits: &mut [usize], radix: &[usize]) -> bool {
    for (digit, &limit) in digits.iter_mut().zip(radix) {
        *digit += 1;
        if *digit < limit {
            return true;
        }
        *digit = 0;
    }
    false
}

fn allowed(events: &[Event], sb: &Matrix, rf: &Matrix, mo: &Matrix) -> bool {
    let n = events.len();
    let is_access = |e: usize| events[e].op.kind != Kind::Fence;
    let same_loc =
        |a: usize, b: usize| is_access(a) && is_access(b) && events[a].op.loc == events[b].op.loc;
    let kind = |e: usize| events[e].op.kind;
    let order = |e: usize| events[e].op.order;
    let atomic = |e: usize| events[e].thread.is_some();

    // rb = rf^-1 ; mo
    let mut rf_inverse = matrix(n);
    for a in 0..n {
        for b in 0..n {
            rf_inverse[b][a] = rf[a][b];
        }
    }
    let rb = compose(&rf_inverse, mo);
    let eco = closure(&union(&union(rf, mo), &rb));

    // sw = [E>=rel] ; ([F] ; sb)? ; rs ; rf ; [R>=rlx] ; (sb ; [F])? ; [E>=acq]
    // rs = [W] ; sb|loc? ; [W>=rlx]
    let mut sw = matrix(n);
    for w in 0..n {
        if kind(w) != Kind::Write || !atomic(w) {
            continue;
        }
        let heads: Vec<usize> = (0..n)
            .filter(|&h| {
                (h == w && order(h).releases())
                    || (kind(h) == Kind::Fence && order(h).releases() && sb[h][w])
            })
            .collect();
        let tails_of = |r: usize| -> Vec<usize> {
            (0..n)
                .filter(|&t| {
                    (t == r && order(t).acquires())
                        || (kind(t) == Kind::Fence && order(t).acquires() && sb[r][t])
                })
                .collect()
        };
        for w2 in 0..n {
            let in_rs = w2 == w || (kind(w2) == Kind::Write && same_loc(w, w2) && sb[w][w2]);
            if !in_rs {
                continue;
            }
            for (r, _) in rf[w2].iter().enumerate().filter(|(_, &read)| read) {
                for &h in &heads {
                    for &t in &tails_of(r) {
                        sw[h][t] = true;
                    }
                }
            }
        }
    }
    let mut base = union(sb, &sw);
    for a in 0..n {
        if events[a].thread.is_none() {
            for b in 0..n {
                if events[b].thread.is_some() {
                    base[a][b] = true;
                }
            }
        }
    }
    let hb = closure(&base);

    // Coherence and no thin air.
    for a in 0..n {
        if hb[a][a] {
            return false;
        }
        for b in 0..n {
            if hb[a][b] && eco[b][a] {
                return false;
            }
        }
    }
    if !acyclic(&union(sb, rf)) {
        return false;
    }

    // psc
    let mut sb_other_loc = matrix(n);
    let mut hb_loc = matrix(n);
    for a in 0..n {
        for b in 0..n {
            sb_other_loc[a][b] = sb[a][b] && !same_loc(a, b);
            hb_loc[a][b] = hb[a][b] && same_loc(a, b);
        }
    }
    let through_hb = compose(&compose(&sb_other_loc, &hb), &sb_other_loc);
    let scb = union(&union(&union(sb, &through_hb), &union(&hb_loc, mo)), &rb);
    let sc = |e: usize| order(e) == Order::SeqCst && atomic(e);
    let hb_opt = optional(&hb);
    let mut head = matrix(n);
    let mut tail = matrix(n);
    for e in 0..n {
        if sc(e) {
            head[e][e] = true;
            tail[e][e] = true;
        }
    }
    let mut fence_head = matrix(n);
    let mut fence_tail = matrix(n);
    for a in 0..n {
        for b in 0..n {
            fence_head[a][b] = sc(a) && kind(a) == Kind::Fence && hb_opt[a][b];
            fence_tail[a][b] = sc(b) && kind(b) == Kind::Fence && hb_opt[a][b];
        }
    }
    let left = union(&head, &fence_head);
    let right = union(&tail, &fence_tail);
    let psc_base = compose(&compose(&left, &scb), &right);
    let through_eco = compose(&compose(&hb, &eco), &hb);
    let fence_path = union(&hb, &through_eco);
    let mut psc_fence = matrix(n);
    for a in 0..n {
        for b in 0..n {
            psc_fence[a][b] = sc(a)
                && sc(b)
                && kind(a) == Kind::Fence
                && kind(b) == Kind::Fence
                && fence_path[a][b];
        }
    }
    acyclic(&union(&psc_base, &psc_fence))
}

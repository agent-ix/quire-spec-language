// SPDX-License-Identifier: AGPL-3.0-or-later
//! The lifting of a clause's formula over possibility sets (FR-501
//! "Verdicts"; QSpec FR-160).
//!
//! Each atom at each eligible position has a possibility set over
//! `{true, false}`. Boolean and temporal operators are lifted pointwise: the
//! set a node can take at a position is computed from the sets its operands
//! can take, a position that may or may not take part (a counter cover that
//! straddles the bound) contributes the choice of either, and a window that
//! an admissible continuation may still extend (an unestablished future
//! window, or history before a cutoff) contributes one unseen position whose
//! operand takes either value. Combining sets of different nodes treats them
//! as independent, which can only widen the result; [`Lifter::exact`] then
//! narrows it by trying every assignment of the ambiguous atoms of a small
//! activation.

use std::collections::HashMap;

use crate::temporal::plan::{Node, Plan};

/// The values a node can take: true, false, or either.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Poss {
    pub(super) can_true: bool,
    pub(super) can_false: bool,
}

impl Poss {
    pub(super) const TRUE: Self = Self {
        can_true: true,
        can_false: false,
    };
    pub(super) const FALSE: Self = Self {
        can_true: false,
        can_false: true,
    };
    pub(super) const EITHER: Self = Self {
        can_true: true,
        can_false: true,
    };

    pub(super) fn is_singleton(self) -> bool {
        self.can_true != self.can_false
    }

    fn not(self) -> Self {
        Self {
            can_true: self.can_false,
            can_false: self.can_true,
        }
    }

    fn and(self, other: Self) -> Self {
        Self {
            can_true: self.can_true && other.can_true,
            can_false: self.can_false || other.can_false,
        }
    }

    fn or(self, other: Self) -> Self {
        Self {
            can_true: self.can_true || other.can_true,
            can_false: self.can_false && other.can_false,
        }
    }

    fn union(self, other: Self) -> Self {
        Self {
            can_true: self.can_true || other.can_true,
            can_false: self.can_false || other.can_false,
        }
    }

    fn flip(self, dual: bool) -> Self {
        if dual {
            self.not()
        } else {
            self
        }
    }

    /// The set of a position that may not take part: it can always read as
    /// not satisfying the scan.
    fn optional(self) -> Self {
        Self {
            can_true: self.can_true,
            can_false: true,
        }
    }
}

/// The valuation of one atom at one position (QSpec FR-160).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Cell {
    True,
    False,
    /// Absent from the admitted observation: both possibilities.
    Missing,
    /// Two differing valuations admitted at the position: both
    /// possibilities, with a reason of its own.
    Conflicting,
}

impl Cell {
    fn poss(self) -> Poss {
        match self {
            Self::True => Poss::TRUE,
            Self::False => Poss::FALSE,
            Self::Missing | Self::Conflicting => Poss::EITHER,
        }
    }

    pub(super) fn ambiguous(self) -> bool {
        matches!(self, Self::Missing | Self::Conflicting)
    }
}

/// Whether a position at a distance takes part in a scan for sure, or only
/// maybe (a cover that straddles the interval's bound).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Part {
    Sure,
    Maybe,
}

/// One candidate position of a scan, in the scan's order.
#[derive(Clone, Copy, Debug)]
pub(super) struct Candidate {
    pub(super) position: usize,
    pub(super) part: Part,
}

/// What one visit of a scan node at a position found.
#[derive(Clone, Debug, Default)]
pub(super) struct ScanFrame {
    pub(super) candidates: Vec<Candidate>,
    /// An admissible continuation may add a position to the window because
    /// the window is not established (future scans) or the interval is not
    /// covered by completeness (past scans).
    pub(super) unestablished: bool,
    /// An unseen position may precede the trace's first (a history cutoff).
    pub(super) history: bool,
}

/// A change to the evidence, to find out what a verdict depends on.
#[derive(Clone, Debug, Default)]
pub(super) struct Variant {
    /// Atoms whose possibility set is replaced, by (position, node).
    pub(super) fixed: Vec<((usize, usize), Poss)>,
    /// A position removed from the trace.
    pub(super) removed: Option<usize>,
    /// A position whose participation at a straddled bound is settled.
    pub(super) settled: Option<(usize, bool)>,
    /// Ignore the unseen position of a history cutoff.
    pub(super) no_history: bool,
    /// Ignore the unseen positions that missing completeness allows.
    pub(super) no_unestablished: bool,
}

/// What a lifting reads: the plan, the atoms read and the scans visited.
pub(super) struct Lifter<'a> {
    pub(super) plan: &'a Plan,
    pub(super) cells: &'a HashMap<(usize, usize), Cell>,
    pub(super) frames: &'a HashMap<(usize, usize), ScanFrame>,
}

impl Lifter<'_> {
    /// The set the root can take at the root's position, over the
    /// `closure` of node visits it depends on (operands after operators in
    /// arena order, so visiting nodes from the last to the first computes
    /// every operand first).
    pub(super) fn lift(
        &self,
        closure: &[(usize, usize)],
        root: (usize, usize),
        variant: &Variant,
    ) -> Option<Poss> {
        let mut values: HashMap<(usize, usize), Poss> = HashMap::new();
        for &(node, position) in closure {
            let value = self.node(node, position, variant, &values)?;
            values.insert((node, position), value);
        }
        values.get(&root).copied()
    }

    fn node(
        &self,
        node: usize,
        position: usize,
        variant: &Variant,
        values: &HashMap<(usize, usize), Poss>,
    ) -> Option<Poss> {
        let operand = |operand: usize, at: usize| values.get(&(operand, at)).copied();
        Some(match *self.plan.nodes.get(node)? {
            Node::Constant(value) => {
                if value {
                    Poss::TRUE
                } else {
                    Poss::FALSE
                }
            }
            Node::Atom => {
                let fixed = variant
                    .fixed
                    .iter()
                    .find(|(key, _)| *key == (position, node))
                    .map(|(_, poss)| *poss);
                match fixed {
                    Some(poss) => poss,
                    None => self.cells.get(&(position, node))?.poss(),
                }
            }
            Node::Not(inner) => operand(inner, position)?.not(),
            Node::And(left, right) => operand(left, position)?.and(operand(right, position)?),
            Node::Or(left, right) => operand(left, position)?.or(operand(right, position)?),
            Node::Implies(left, right) => {
                operand(left, position)?.not().or(operand(right, position)?)
            }
            Node::Scan(scan) => {
                let frame = self.frames.get(&(node, position))?;
                let mut result = Poss::FALSE;
                let mut guards = Poss::TRUE;
                for candidate in &frame.candidates {
                    if variant.removed == Some(candidate.position) {
                        continue;
                    }
                    let part = match variant.settled {
                        Some((settled, takes_part)) if settled == candidate.position => {
                            if !takes_part {
                                continue;
                            }
                            Part::Sure
                        }
                        _ => candidate.part,
                    };
                    let mut term = operand(scan.witness, candidate.position)?.flip(scan.dual);
                    term = term.and(guards);
                    if part == Part::Maybe {
                        term = term.optional();
                    }
                    result = result.or(term);
                    if let Some(guard) = scan.guard {
                        let mut value = operand(guard, candidate.position)?.flip(scan.dual);
                        if part == Part::Maybe {
                            value = Poss {
                                can_true: true,
                                can_false: value.can_false,
                            };
                        }
                        guards = guards.and(value);
                    }
                }
                let unseen = (frame.unestablished && !variant.no_unestablished)
                    || (frame.history && !variant.no_history);
                if unseen {
                    result = result.or(Poss {
                        can_true: guards.can_true,
                        can_false: true,
                    });
                }
                result.flip(scan.dual)
            }
        })
    }

    /// The set the root can take, narrowed by trying every assignment of the
    /// ambiguous atoms in `closure` when there are few enough
    /// (`EXACT_ATOMS`); otherwise the independent-combination set, which
    /// contains the exact one.
    pub(super) fn exact(
        &self,
        closure: &[(usize, usize)],
        root: (usize, usize),
        variant: &Variant,
    ) -> Option<Poss> {
        let base = self.lift(closure, root, variant)?;
        if base.is_singleton() {
            return Some(base);
        }
        let mut ambiguous: Vec<(usize, usize)> = closure
            .iter()
            .filter(|(node, position)| {
                matches!(self.plan.nodes.get(*node), Some(Node::Atom))
                    && self
                        .cells
                        .get(&(*position, *node))
                        .is_some_and(|cell| cell.ambiguous())
            })
            .map(|(node, position)| (*position, *node))
            .collect();
        ambiguous.sort_unstable();
        ambiguous.dedup();
        if ambiguous.is_empty() || ambiguous.len() > EXACT_ATOMS {
            return Some(base);
        }
        let mut union: Option<Poss> = None;
        for assignment in 0_u32..(1 << ambiguous.len()) {
            let mut trial = variant.clone();
            for (bit, key) in ambiguous.iter().enumerate() {
                let poss = if assignment >> bit & 1 == 1 {
                    Poss::TRUE
                } else {
                    Poss::FALSE
                };
                trial.fixed.push((*key, poss));
            }
            let value = self.lift(closure, root, &trial)?;
            union = Some(union.map_or(value, |seen| seen.union(value)));
        }
        union.map(|narrow| Poss {
            can_true: narrow.can_true && base.can_true,
            can_false: narrow.can_false && base.can_false,
        })
    }
}

/// The most ambiguous atoms of one activation [`Lifter::exact`] enumerates.
const EXACT_ATOMS: usize = 12;

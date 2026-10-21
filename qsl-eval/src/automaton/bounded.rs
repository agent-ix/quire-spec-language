// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338 "Canonical translation state", the TP-2 alternatives: the canonical
//! key of a bounded monitor's state, and the translation of the two bounded
//! forms EN-1 certifies, `eventually[0,u] holds(A)` and `always[0,u]
//! holds(A)` activated at the origin.
//!
//! The monitor's undecided state is `["bounded", ["origin", remaining],
//! branches]`: `remaining` is the counted distance left of the root's
//! horizon, and the one branch `[[], [], windows, []]` carries the root's
//! open debt as one window, `[[], "positive", "deadline", remaining]` for
//! `eventually` (the earliest open deadline) and `[[], "positive", "extent",
//! remaining]` for `always` (the furthest required position). Both counters
//! are measured from the letter just read. The decided states are the
//! absorbing `["accept"]` and `["reject"]`.
//!
//! A step first subtracts its counted distance: a step that counts leaves
//! the window once nothing remains, and closes the region before any letter
//! of the next distance is read (QSpec FR-091's closed boundary). An
//! undischarged `eventually` debt then rejects and a surviving `always`
//! obligation accepts. A step that does not count (ADR-027 PB-3) leaves the
//! counter where it was and its letter is still read. At a terminal state
//! the false extension supplies the counted false letters through the
//! remaining horizon and closes the last region.

use qsl_foundation::bound::TemporalUpper;
use qsl_foundation::IdentityLimits;
use qsl_semantics::check::{
    Activation, CheckedInterval, CheckedTemporalClause, FormulaNode, FormulaNodeId, UnaryTemporal,
};
use quire_canonical::{Limits, Node, NodeRef};

use crate::automaton::key::{
    ascending, canonical, decimal, exactly, AutomatonStateKey, ContextActivation, KeyRefusal,
    NotTranslated, StateContext, TranslationRefusal, AUTOMATON_STATE_KEY_V1,
};
use crate::simulation::model::encode::Enc;

/// Which operator the bounded root is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundedKind {
    /// `eventually[0,u] holds(A)`: a debt with a deadline.
    Eventually,
    /// `always[0,u] holds(A)`: an obligation with an extent.
    Always,
}

impl BoundedKind {
    /// The window kind's spelling in a key.
    const fn window(self) -> &'static str {
        match self {
            Self::Eventually => "deadline",
            Self::Always => "extent",
        }
    }
}

/// A state of a [`BoundedTranslation`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BoundedState {
    /// The root's debt is open, with `remaining` counted steps of its
    /// horizon left after the letter just read.
    Open {
        /// The counted distance left.
        remaining: u64,
    },
    /// Decided in favour of the clause; absorbing.
    Accept,
    /// Decided against the clause; absorbing.
    Reject,
}

impl BoundedState {
    /// Whether the state rejects.
    pub const fn is_rejecting(self) -> bool {
        matches!(self, Self::Reject)
    }
}

/// The canonical translation of one bounded clause under one context
/// (FR-338): the monitor both EN-1 and the core's closure checker read.
#[derive(Clone, Debug)]
pub struct BoundedTranslation {
    kind: BoundedKind,
    upper: u64,
    atom: FormulaNodeId,
    context: Enc,
    context_octets: Vec<u8>,
    max_bytes: u64,
}

impl BoundedTranslation {
    /// The translation of `clause` under `context`.
    ///
    /// # Errors
    ///
    /// [`TranslationRefusal::NotTranslated`] when the clause is not
    /// `eventually[0,u] holds(A)` or `always[0,u] holds(A)` on origin under
    /// a step-measured bounded profile, or `context` is not its own;
    /// [`TranslationRefusal::Key`] when the context does not encode.
    pub fn clause(
        clause: &CheckedTemporalClause,
        context: &StateContext,
    ) -> Result<Self, TranslationRefusal> {
        let (kind, upper, atom) =
            bounded_shape(clause).ok_or(TranslationRefusal::NotTranslated(NotTranslated))?;
        if context.profile != clause.profile()
            || !matches!(context.activation, ContextActivation::Origin)
        {
            return Err(TranslationRefusal::NotTranslated(NotTranslated));
        }
        let max_bytes = IdentityLimits::default().input_bytes;
        let (context, context_octets) = context
            .encoded(max_bytes)
            .map_err(TranslationRefusal::Key)?;
        Ok(Self {
            kind,
            upper,
            atom,
            context,
            context_octets,
            max_bytes,
        })
    }

    /// The clause's `holds` atom.
    pub const fn atom(&self) -> FormulaNodeId {
        self.atom
    }

    /// The horizon: the last counted distance the clause reads.
    pub const fn horizon(&self) -> u64 {
        self.upper
    }

    /// The root operator.
    pub const fn kind(&self) -> BoundedKind {
        self.kind
    }

    /// The state after position 0, whose atom is `holds`.
    pub fn start(&self, holds: bool) -> BoundedState {
        self.decide(self.upper, holds)
    }

    /// The state after one more position from `before`, reached by a step
    /// that `counted` or not, whose atom is `holds`. A step that counts out
    /// of an exhausted window reads no letter, so `holds` is ignored there.
    pub fn step(&self, before: BoundedState, counted: bool, holds: bool) -> BoundedState {
        let BoundedState::Open { remaining } = before else {
            return before;
        };
        if !counted {
            return self.decide(remaining, holds);
        }
        match remaining.checked_sub(1) {
            Some(remaining) => self.decide(remaining, holds),
            // The region closed before the next distance was read.
            None => match self.kind {
                BoundedKind::Eventually => BoundedState::Reject,
                BoundedKind::Always => BoundedState::Accept,
            },
        }
    }

    /// The state the false extension of a terminal state `before` closes
    /// to: the counted false letters through the remaining horizon, then
    /// the closed boundary.
    pub fn terminal(&self, before: BoundedState) -> BoundedState {
        match (before, self.kind) {
            (BoundedState::Open { remaining: 0 }, BoundedKind::Always) => BoundedState::Accept,
            (BoundedState::Open { .. }, _) => BoundedState::Reject,
            (decided, _) => decided,
        }
    }

    /// The state with `remaining` counted steps left after a letter whose
    /// atom is `holds`.
    fn decide(&self, remaining: u64, holds: bool) -> BoundedState {
        match (self.kind, holds) {
            (BoundedKind::Eventually, true) => BoundedState::Accept,
            (BoundedKind::Always, false) => BoundedState::Reject,
            (BoundedKind::Eventually, false) | (BoundedKind::Always, true) => {
                BoundedState::Open { remaining }
            }
        }
    }

    /// The canonical key of `state` under this translation's context.
    ///
    /// # Errors
    ///
    /// [`KeyRefusal::Encoding`] when the key exceeds the identity byte
    /// budget; no truncated key is produced.
    pub fn key(&self, state: BoundedState) -> Result<AutomatonStateKey, KeyRefusal> {
        let key = Enc::Array(vec![
            Enc::text(AUTOMATON_STATE_KEY_V1),
            self.context.clone(),
            self.state_enc(state),
        ]);
        quire_canonical::to_vec(&key, Limits::new(self.max_bytes))
            .map(AutomatonStateKey)
            .map_err(|_| KeyRefusal::Encoding)
    }

    fn state_enc(&self, state: BoundedState) -> Enc {
        match state {
            BoundedState::Accept => Enc::Array(vec![Enc::text("accept")]),
            BoundedState::Reject => Enc::Array(vec![Enc::text("reject")]),
            BoundedState::Open { remaining } => {
                let distance = || Enc::text(remaining.to_string());
                let window = Enc::Array(vec![
                    Enc::Array(Vec::new()),
                    Enc::text("positive"),
                    Enc::text(self.kind.window()),
                    distance(),
                ]);
                let branch = Enc::Array(vec![
                    Enc::Array(Vec::new()),
                    Enc::Array(Vec::new()),
                    Enc::Array(vec![window]),
                    Enc::Array(Vec::new()),
                ]);
                Enc::Array(vec![
                    Enc::text("bounded"),
                    Enc::Array(vec![Enc::text("origin"), distance()]),
                    Enc::Array(vec![branch]),
                ])
            }
        }
    }

    /// Admit `octets` as a key of this translation's context, refusing every
    /// FR-338 admission negative before the key is used for membership.
    ///
    /// # Errors
    ///
    /// The first [`KeyRefusal`] the octets meet.
    pub fn admit(&self, octets: &[u8]) -> Result<(AutomatonStateKey, BoundedState), KeyRefusal> {
        let document =
            quire_canonical::read(octets, self.max_bytes).map_err(|_| KeyRefusal::Malformed)?;
        if canonical(document.root(), self.max_bytes)? != octets {
            return Err(KeyRefusal::NotCanonical);
        }
        let Node::Array(items) = document.root().node() else {
            return Err(KeyRefusal::Shape);
        };
        let [discriminator, context, state] = exactly(items).ok_or(KeyRefusal::Shape)?;
        if !matches!(discriminator.node(), Node::String(AUTOMATON_STATE_KEY_V1)) {
            return Err(KeyRefusal::Discriminator);
        }
        if canonical(context, self.max_bytes)? != self.context_octets {
            return Err(KeyRefusal::ForeignContext);
        }
        let state = self.state(state)?;
        Ok((AutomatonStateKey(octets.to_vec()), state))
    }

    /// The bounded state `body` encodes. `#[string_edge]`: reads an imported
    /// key's state tag by its wire spelling.
    #[qsl_attrs::string_edge]
    fn state(&self, body: NodeRef<'_>) -> Result<BoundedState, KeyRefusal> {
        let Node::Array(mut items) = body.node() else {
            return Err(KeyRefusal::Members);
        };
        let tag = match items.next().map(|tag| tag.node()) {
            Some(Node::String(tag)) => tag,
            _ => return Err(KeyRefusal::Members),
        };
        match tag {
            "accept" | "reject" => {
                if items.next().is_some() {
                    return Err(KeyRefusal::Members);
                }
                Ok(if tag == "accept" {
                    BoundedState::Accept
                } else {
                    BoundedState::Reject
                })
            }
            "bounded" => {
                let [clock, branches] = exactly(items).ok_or(KeyRefusal::Members)?;
                self.open(clock, branches)
            }
            "safety" | "buchi" => Err(KeyRefusal::WrongForm(tag.to_owned())),
            other => Err(KeyRefusal::UnknownTag(other.to_owned())),
        }
    }

    /// `["origin", remaining]` and the one branch carrying the root's
    /// window with that same remaining distance. `#[string_edge]`: reads an
    /// imported key's clock and window spellings.
    #[qsl_attrs::string_edge]
    fn open(&self, clock: NodeRef<'_>, branches: NodeRef<'_>) -> Result<BoundedState, KeyRefusal> {
        let Node::Array(clock) = clock.node() else {
            return Err(KeyRefusal::Members);
        };
        let [mode, remaining] = exactly(clock).ok_or(KeyRefusal::Members)?;
        match mode.node() {
            Node::String("origin") => {}
            // The clock of an `on each` monitor is another activation's.
            Node::String("each") => return Err(KeyRefusal::ForeignContext),
            _ => return Err(KeyRefusal::Members),
        }
        let remaining = match remaining.node() {
            Node::String(text) => decimal(text)?,
            _ => return Err(KeyRefusal::Members),
        };
        if remaining > self.upper {
            return Err(KeyRefusal::Unreachable);
        }
        let Node::Array(branches) = branches.node() else {
            return Err(KeyRefusal::Members);
        };
        let mut previous: Option<Vec<u8>> = None;
        let mut count = 0_usize;
        for branch in branches {
            ascending(&mut previous, canonical(branch, self.max_bytes)?)?;
            count += 1;
            self.configuration(branch, remaining)?;
        }
        if count == 1 {
            Ok(BoundedState::Open { remaining })
        } else {
            // No branch is false, and the clause has one root debt.
            Err(KeyRefusal::Unreachable)
        }
    }

    /// One elementary configuration `[obligations, past, windows,
    /// acceptance]`: the root's one window and nothing else.
    #[qsl_attrs::string_edge]
    fn configuration(&self, branch: NodeRef<'_>, remaining: u64) -> Result<(), KeyRefusal> {
        let Node::Array(members) = branch.node() else {
            return Err(KeyRefusal::Members);
        };
        let [obligations, past, windows, acceptance] =
            exactly(members).ok_or(KeyRefusal::Members)?;
        // The root's debt is stored in its window, not as an obligation, and
        // the clause has no past or least-fixed-point occurrence an entry
        // could name: a finite debt on a model subject needs no acceptance
        // set, and on a protocol subject a carry is not a member.
        for fact in [obligations, past, acceptance] {
            let Node::Array(mut entries) = fact.node() else {
                return Err(KeyRefusal::Members);
            };
            if entries.next().is_some() {
                return Err(KeyRefusal::Unresolved);
            }
        }
        let Node::Array(windows) = windows.node() else {
            return Err(KeyRefusal::Members);
        };
        let windows: Vec<NodeRef<'_>> = windows.collect();
        let [window] = windows.as_slice() else {
            return Err(KeyRefusal::Unresolved);
        };
        let Node::Array(entry) = window.node() else {
            return Err(KeyRefusal::Members);
        };
        let [path, polarity, kind, distance] = exactly(entry).ok_or(KeyRefusal::Members)?;
        let Node::Array(mut path) = path.node() else {
            return Err(KeyRefusal::Members);
        };
        if path.next().is_some() {
            return Err(KeyRefusal::Unresolved);
        }
        match polarity.node() {
            Node::String("positive") => {}
            Node::String("negative") => return Err(KeyRefusal::Unresolved),
            _ => return Err(KeyRefusal::Members),
        }
        match kind.node() {
            Node::String(spelling) if spelling == self.kind.window() => {}
            Node::String(_) => return Err(KeyRefusal::Unresolved),
            _ => return Err(KeyRefusal::Members),
        }
        let distance = match distance.node() {
            Node::String(text) => decimal(text)?,
            _ => return Err(KeyRefusal::Members),
        };
        if distance == remaining {
            Ok(())
        } else {
            Err(KeyRefusal::Unreachable)
        }
    }
}

/// The root operator, upper bound and `holds` atom of a clause this module
/// translates: `eventually[0,u] holds(A)` or `always[0,u] holds(A)` on origin
/// under a step-measured bounded profile.
pub fn bounded_shape(clause: &CheckedTemporalClause) -> Option<(BoundedKind, u64, FormulaNodeId)> {
    if !clause.profile().is_bounded() || !matches!(clause.activation(), Activation::Origin) {
        return None;
    }
    let formula = clause.formula();
    let FormulaNode::Unary {
        operator,
        interval: Some(CheckedInterval::Steps(interval)),
        operand,
    } = formula.nodes().get(formula.root().index())?
    else {
        return None;
    };
    let kind = match operator {
        UnaryTemporal::Eventually => BoundedKind::Eventually,
        UnaryTemporal::Always => BoundedKind::Always,
        UnaryTemporal::Once | UnaryTemporal::Historically => return None,
    };
    let TemporalUpper::Finite(upper) = interval.upper() else {
        return None;
    };
    if interval.lower() != 0 {
        return None;
    }
    matches!(
        formula.nodes().get(operand.index()),
        Some(FormulaNode::Holds { .. })
    )
    .then_some((kind, upper, *operand))
}

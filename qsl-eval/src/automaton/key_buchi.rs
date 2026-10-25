// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338's two Büchi state alternatives (ADR-018 LA-2, §12): `["buchi",
//! branch]` for a TP-4 clause, and `["guarded-buchi", phase, branch]` for a
//! TP-4 clause whose `on each` activation has a `when` condition.
//!
//! A branch is one elementary configuration of the negated formula, with the
//! generalized Büchi membership of its acceptance sets. It is not a
//! subset-determinized safety state, so a state holds exactly one. The
//! `["activation"]` acceptance set is derived from the phase and is never a
//! member of a key.

use qsl_foundation::IdentityLimits;
use qsl_semantics::check::CheckedTemporalClause;
use quire_canonical::{Limits, Node, NodeRef};

use super::key::{
    admit_envelope, exactly, AutomatonStateKey, ContextActivation, KeyRefusal, NotTranslated,
    StateContext, SubjectKind, TranslationRefusal, AUTOMATON_STATE_KEY_V1,
};
use super::key_state::{normal, Configuration, FormulaShape, Polarity, Reader};
use crate::simulation::model::encode::Enc;

/// The phase of a guarded TP-4 state.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BuchiPhase {
    /// No activation has been selected.
    Waiting,
    /// One activation has been selected.
    Active,
}

impl BuchiPhase {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Active => "active",
        }
    }
}

/// A Büchi state: the one elementary configuration of the negated formula
/// and, under a guarded context, the phase.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum BuchiState {
    /// `["buchi", branch]`.
    Elementary(Configuration),
    /// `["guarded-buchi", phase, branch]`. A waiting branch holds no
    /// obligation and no window.
    Guarded(BuchiPhase, Configuration),
}

/// The Büchi translation of a TP-4 clause: the keys of its negated
/// formula's elementary configurations under one context.
#[derive(Clone, Debug)]
pub struct BuchiStates {
    context: Enc,
    context_octets: Vec<u8>,
    max_bytes: u64,
    shape: FormulaShape,
    guarded: bool,
}

impl BuchiStates {
    /// The Büchi states of the TP-4 clause `clause` over an ordinary
    /// `subject` under `context`; guarded when the context's activation is
    /// `"each-when"`.
    ///
    /// # Errors
    ///
    /// [`TranslationRefusal::NotTranslated`] when the subject is a protocol,
    /// the clause is not TP-4 (a bounded profile, or a formula in the safety
    /// fragment), holds a timed interval, has more operand paths than the
    /// path bound, or `context` names another profile or an activation that
    /// is neither `"infinite"` nor `"each-when"`;
    /// [`TranslationRefusal::Key`] when the context does not encode.
    pub fn clause(
        clause: &CheckedTemporalClause,
        context: &StateContext,
        subject: SubjectKind,
    ) -> Result<Self, TranslationRefusal> {
        let not_translated = || TranslationRefusal::NotTranslated(NotTranslated);
        let guarded = match context.activation {
            ContextActivation::EachWhen => true,
            ContextActivation::Infinite => false,
            ContextActivation::Origin | ContextActivation::Each => return Err(not_translated()),
        };
        if subject == SubjectKind::Protocol
            || clause.profile().is_bounded()
            || context.profile != clause.profile()
        {
            return Err(not_translated());
        }
        let formula = clause.formula();
        let live = FormulaShape::of(formula, Polarity::Positive)
            .ok_or_else(not_translated)?
            .is_liveness();
        if !live {
            return Err(not_translated());
        }
        let shape = FormulaShape::of(formula, Polarity::Negative).ok_or_else(not_translated)?;
        let max_bytes = IdentityLimits::default().input_bytes;
        let (context, context_octets) = context
            .encoded(max_bytes)
            .map_err(TranslationRefusal::Key)?;
        Ok(Self {
            context,
            context_octets,
            max_bytes,
            shape,
            guarded,
        })
    }

    /// The canonical key of `state`.
    ///
    /// # Errors
    ///
    /// [`KeyRefusal::WrongForm`] when the state's alternative is not this
    /// context's; [`KeyRefusal::Encoding`] when the key exceeds the identity
    /// byte budget; any other [`KeyRefusal`] when the facts, normalized as
    /// for a safety branch, are a state this context would refuse.
    pub fn key(&self, state: &BuchiState) -> Result<AutomatonStateKey, KeyRefusal> {
        let (tag, phase, branch) = match state {
            BuchiState::Elementary(branch) => ("buchi", None, branch),
            BuchiState::Guarded(phase, branch) => ("guarded-buchi", Some(*phase), branch),
        };
        if (phase.is_some()) != self.guarded {
            return Err(KeyRefusal::WrongForm(tag.to_owned()));
        }
        let branch = normal(std::slice::from_ref(branch), self.max_bytes)?
            .into_iter()
            .next()
            .ok_or(KeyRefusal::Members)?;
        let branch = branch.enc(self.max_bytes)?;
        let mut members = vec![Enc::text(tag)];
        members.extend(phase.map(|phase| Enc::text(phase.as_str())));
        members.push(branch);
        let key = Enc::Array(vec![
            Enc::text(AUTOMATON_STATE_KEY_V1),
            self.context.clone(),
            Enc::Array(members),
        ]);
        let key = quire_canonical::to_vec(&key, Limits::new(self.max_bytes))
            .map(AutomatonStateKey)
            .map_err(|_| KeyRefusal::Encoding)?;
        // A key is written only if this context would admit it back.
        self.admit(key.as_bytes())?;
        Ok(key)
    }

    /// Admit `octets` as a Büchi key of this clause's context, resolving
    /// every member in the negated formula.
    ///
    /// # Errors
    ///
    /// The first [`KeyRefusal`] the octets meet.
    pub fn admit(&self, octets: &[u8]) -> Result<(AutomatonStateKey, BuchiState), KeyRefusal> {
        admit_envelope(&self.context_octets, self.max_bytes, octets, |state| {
            self.state(state)
        })
    }

    /// The Büchi state `body` encodes. `#[string_edge]`: reads an imported
    /// key's state tag and phase by their wire spelling.
    #[ix_trace_rs::string_edge]
    fn state(&self, body: NodeRef<'_>) -> Result<BuchiState, KeyRefusal> {
        let Node::Array(mut items) = body.node() else {
            return Err(KeyRefusal::Members);
        };
        let tag = match items.next().map(|tag| tag.node()) {
            Some(Node::String(tag)) => tag,
            _ => return Err(KeyRefusal::Members),
        };
        let reader = Reader {
            shape: &self.shape,
            max_bytes: self.max_bytes,
        };
        match (tag, self.guarded) {
            ("buchi", false) => {
                let [branch] = exactly(items).ok_or(KeyRefusal::Members)?;
                Ok(BuchiState::Elementary(reader.configuration(branch)?))
            }
            ("guarded-buchi", true) => {
                let [phase, branch] = exactly(items).ok_or(KeyRefusal::Members)?;
                let phase = match phase.node() {
                    Node::String("waiting") => BuchiPhase::Waiting,
                    Node::String("active") => BuchiPhase::Active,
                    _ => return Err(KeyRefusal::Members),
                };
                let branch = reader.configuration(branch)?;
                if phase == BuchiPhase::Waiting
                    && (!branch.obligations.is_empty() || !branch.windows.is_empty())
                {
                    return Err(KeyRefusal::Members);
                }
                Ok(BuchiState::Guarded(phase, branch))
            }
            ("buchi" | "guarded-buchi" | "safety" | "bounded" | "accept" | "reject", _) => {
                Err(KeyRefusal::WrongForm(tag.to_owned()))
            }
            (other, _) => Err(KeyRefusal::UnknownTag(other.to_owned())),
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338 "Canonical translation state" (ADR-018 LA-2, §12): the canonical
//! key of a property-automaton state, and the safety translation of the two
//! items EN-1 decides today, a TP-1 clause `always holds(A)` and FR-124's
//! deadlock-freedom item `always holds(not deadlocked)`.
//!
//! An [`AutomatonStateKey`] is the RFC 8785 canonical UTF-8 octets of
//! `["quire.automaton-state-key/v1", context, state]`, written by
//! `quire-canonical`'s one encoder. `context` is `[root, profile, activation,
//! binding]` ([`StateContext`]); `state` is one closed alternative of the
//! translation's structural algebra. For a safety item it is `["safety",
//! branches]`: the viable elementary configurations of the formula itself,
//! each `[obligations, past, windows, acceptance]`. The key is a function of
//! these facts alone, never of the order states were reached, and no graph
//! index enters it.
//!
//! Both items here are `G a` over one current atom. Reading a letter expands
//! the carried root: the atom is discharged when it holds and removes the
//! branch otherwise, and `G a` is carried to the next position. So the
//! translation has exactly two states, the carried root ([`SafetyState::
//! Carried`], `["safety",[[[[[],"positive"]],[],[],[]]]]`) and the one
//! rejecting state ([`SafetyState::Rejected`], `["safety",[]]`), which is
//! absorbing. Neither formula has a past, interval or least-fixed-point
//! occurrence, so `past`, `windows` and `acceptance` are empty in every state,
//! and an imported key with a member there resolves in no occurrence of the
//! formula. The other forms of FR-338's algebra (bounded, TP-3 with
//! intervals, Büchi) belong to the EN-1 phases that decide them; a clause of
//! another shape is not translated here.

use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::IdentityLimits;
use qsl_semantics::check::{
    Activation, CheckedTemporalClause, FormulaNode, FormulaNodeId, TemporalProfile, UnaryTemporal,
};
use quire_canonical::{Limits, Node, NodeRef};
use quire_exact::Value;

use crate::simulation::model::encode::{hex, value_enc, Enc};

/// The discriminator of a key's octet encoding (not a digest domain).
pub const AUTOMATON_STATE_KEY_V1: &str = "quire.automaton-state-key/v1";

/// The root of the derived deadlock-freedom item's context.
const DEADLOCK_FREEDOM: &str = "deadlock-freedom";

/// FR-338's `AutomatonStateKey`: the complete canonical octets of one
/// automaton state under its context. Equality compares complete octets;
/// order is ascending unsigned lexicographic octet order.
///
/// A key is built only by a [`SafetyTranslation`], by encoding one of its
/// states or by admitting octets against its context.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AutomatonStateKey(Vec<u8>);

impl AutomatonStateKey {
    /// The key's canonical UTF-8 octets.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// The root of a [`StateContext`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextRoot {
    /// An authored clause, by its checked occurrence: `[node, role,
    /// ordinal]`.
    Clause(OccurrenceKey),
    /// The derived deadlock-freedom item: `["deadlock-freedom"]`.
    DeadlockFreedom,
}

/// A context's activation: `"origin"` or `"each"` for a bounded monitor,
/// `"infinite"` otherwise.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextActivation {
    /// A bounded monitor activated at the origin.
    Origin,
    /// A bounded monitor activated at each position.
    Each,
    /// An infinite-trace automaton.
    Infinite,
}

impl ContextActivation {
    /// The activation's spelling in a key.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Origin => "origin",
            Self::Each => "each",
            Self::Infinite => "infinite",
        }
    }
}

/// One entry of a context's binding: a parameter node of the item and the
/// complete typed value bound to it.
#[derive(Clone, Debug)]
pub struct ContextBinding {
    /// The parameter's checked node.
    pub parameter: WireNodeId,
    /// The value it is bound to, written in QSpec FR-181's typed canonical
    /// form.
    pub value: Value,
}

/// FR-338's key context `[root, profile, activation, binding]`. It does not
/// copy the model subject: the certificate request binds the subject.
#[derive(Clone, Debug)]
pub struct StateContext {
    /// The item's root.
    pub root: ContextRoot,
    /// The admitted temporal profile.
    pub profile: TemporalProfile,
    /// The activation.
    pub activation: ContextActivation,
    /// The item's concrete binding, in the item's binding order; `[]` for a
    /// parameter-free item.
    pub binding: Vec<ContextBinding>,
}

impl StateContext {
    /// The derived deadlock-freedom item's context: `[["deadlock-freedom"],
    /// "quire.temporal.infinite-trace/v1", "infinite", []]`.
    pub fn deadlock_freedom() -> Self {
        Self {
            root: ContextRoot::DeadlockFreedom,
            profile: TemporalProfile::InfiniteTrace,
            activation: ContextActivation::Infinite,
            binding: Vec::new(),
        }
    }

    /// The context of `clause` at `occurrence` under `binding`: its profile
    /// and activation are the checked clause's own.
    pub fn clause(
        clause: &CheckedTemporalClause,
        occurrence: OccurrenceKey,
        binding: Vec<ContextBinding>,
    ) -> Self {
        let profile = clause.profile();
        let activation = match (profile.is_bounded(), clause.activation()) {
            (false, _) => ContextActivation::Infinite,
            (true, Activation::Origin) => ContextActivation::Origin,
            (true, Activation::Each { .. }) => ContextActivation::Each,
        };
        Self {
            root: ContextRoot::Clause(occurrence),
            profile,
            activation,
            binding,
        }
    }

    fn enc(&self) -> Enc {
        let root = match &self.root {
            ContextRoot::Clause(occurrence) => Enc::Array(vec![
                node_enc(occurrence.node()),
                Enc::text(occurrence.origin().role().as_str()),
                Enc::text(occurrence.origin().ordinal().to_string()),
            ]),
            ContextRoot::DeadlockFreedom => Enc::Array(vec![Enc::text(DEADLOCK_FREEDOM)]),
        };
        let binding = self
            .binding
            .iter()
            .map(|entry| Enc::Array(vec![node_enc(entry.parameter), value_enc(&entry.value)]))
            .collect();
        Enc::Array(vec![
            root,
            Enc::text(self.profile.identity()),
            Enc::text(self.activation.as_str()),
            Enc::Array(binding),
        ])
    }
}

/// FR-322's NodeId object.
fn node_enc(node: WireNodeId) -> Enc {
    Enc::Object(vec![
        ("domain", Enc::text("quire.checked-semantic-node/v1")),
        ("digest", Enc::text(hex(node.as_bytes()))),
    ])
}

/// A state of a [`SafetyTranslation`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SafetyState {
    /// The root `G a` is carried: `["safety",[[[[[],"positive"]],[],[],[]]]]`.
    Carried,
    /// The unique rejecting state, `["safety",[]]`; absorbing.
    Rejected,
}

impl SafetyState {
    /// Whether the state rejects.
    pub const fn is_rejecting(self) -> bool {
        matches!(self, Self::Rejected)
    }

    fn enc(self) -> Enc {
        let branches = match self {
            Self::Carried => vec![Enc::Array(vec![
                Enc::Array(vec![root_obligation()]),
                Enc::Array(Vec::new()),
                Enc::Array(Vec::new()),
                Enc::Array(Vec::new()),
            ])],
            Self::Rejected => Vec::new(),
        };
        Enc::Array(vec![Enc::text("safety"), Enc::Array(branches)])
    }
}

/// The root occurrence `[[], "positive"]`.
fn root_obligation() -> Enc {
    Enc::Array(vec![Enc::Array(Vec::new()), Enc::text("positive")])
}

/// What a [`SafetyTranslation`] translates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafetyItem {
    /// A TP-1 clause `always holds(A)`, with its `holds` atom.
    ReachableInvariant {
        /// The clause's `holds` atom.
        atom: FormulaNodeId,
    },
    /// FR-124's deadlock-freedom item, `always holds(not deadlocked)`.
    DeadlockFreedom,
}

/// A clause this module does not translate: it is not `always holds(A)`
/// under infinite-trace.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("the clause is not a reachable invariant `always holds(A)` under infinite-trace")]
pub struct NotTranslated;

/// Why octets are not admitted as a key of a translation's context, or a key
/// could not be encoded (FR-338 admission negatives).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum KeyRefusal {
    /// The octets are not one JSON value within the identity byte budget.
    #[error("the key is not one JSON value within the identity byte budget")]
    Malformed,
    /// The octets are JSON but not their own RFC 8785 encoding.
    #[error("the key is not its own RFC 8785 canonical encoding")]
    NotCanonical,
    /// The outer array is not `[discriminator, context, state]`.
    #[error("the key is not a three-member array")]
    Shape,
    /// The discriminator is not `quire.automaton-state-key/v1`.
    #[error("the key's discriminator is not {AUTOMATON_STATE_KEY_V1}")]
    Discriminator,
    /// The context is not this item's: another clause, profile, activation
    /// or binding.
    #[error("the key's context is not this item's")]
    ForeignContext,
    /// The state's tag is not a state alternative of FR-338's algebra.
    #[error("the state tag {0:?} is not a state alternative")]
    UnknownTag(String),
    /// The state is an alternative of another form than this item's.
    #[error("the state alternative {0:?} is not this item's form")]
    WrongForm(String),
    /// A state member is missing, extra, or of the wrong JSON type.
    #[error("the state has a missing, extra or mistyped member")]
    Members,
    /// An unsigned integer is not a canonical decimal string in the u64
    /// domain.
    #[error("{0:?} is not a canonical unsigned decimal string")]
    Decimal(String),
    /// A set is unsorted or holds a duplicate.
    #[error("a set is unsorted or holds a duplicate")]
    Order,
    /// A member names no occurrence of the item's formula with the matching
    /// operator, polarity and bounds.
    #[error("a member resolves in no occurrence of the item's formula")]
    Unresolved,
    /// The state is well formed but is not a state of this translation.
    #[error("the state is not a state of this item's translation")]
    Unreachable,
    /// The key exceeds the identity byte budget or a bound value has no
    /// typed canonical form.
    #[error("the key has no canonical encoding within the identity byte budget")]
    Encoding,
}

/// The canonical safety translation of one item under one context
/// (FR-338): the automaton both EN-1 and the core's closure checker read.
#[derive(Clone, Debug)]
pub struct SafetyTranslation {
    item: SafetyItem,
    context: Vec<u8>,
    max_bytes: u64,
}

impl SafetyTranslation {
    /// The translation of the deadlock-freedom item.
    ///
    /// # Errors
    ///
    /// [`KeyRefusal::Encoding`] when the context does not encode within the
    /// identity byte budget.
    pub fn deadlock_freedom() -> Result<Self, KeyRefusal> {
        Self::new(
            SafetyItem::DeadlockFreedom,
            &StateContext::deadlock_freedom(),
        )
    }

    /// The translation of the TP-1 clause `clause` under `context`.
    ///
    /// # Errors
    ///
    /// [`TranslationRefusal::NotTranslated`] when the clause is not `always
    /// holds(A)` under infinite-trace or `context` names another profile;
    /// [`TranslationRefusal::Key`] when the context does not encode.
    pub fn reachable_invariant(
        clause: &CheckedTemporalClause,
        context: &StateContext,
    ) -> Result<Self, TranslationRefusal> {
        let atom =
            invariant_atom(clause).ok_or(TranslationRefusal::NotTranslated(NotTranslated))?;
        if clause.profile() != TemporalProfile::InfiniteTrace || context.profile != clause.profile()
        {
            return Err(TranslationRefusal::NotTranslated(NotTranslated));
        }
        Self::new(SafetyItem::ReachableInvariant { atom }, context).map_err(TranslationRefusal::Key)
    }

    fn new(item: SafetyItem, context: &StateContext) -> Result<Self, KeyRefusal> {
        let max_bytes = IdentityLimits::default().input_bytes;
        let context = quire_canonical::to_vec(&context.enc(), Limits::new(max_bytes))
            .map_err(|_| KeyRefusal::Encoding)?;
        Ok(Self {
            item,
            context,
            max_bytes,
        })
    }

    /// What the translation translates.
    pub const fn item(&self) -> SafetyItem {
        self.item
    }

    /// The state after reading a position whose atom is `holds`, from
    /// `before`, or from the initial root obligation when `before` is
    /// `None` (position zero).
    pub fn read(&self, before: Option<SafetyState>, holds: bool) -> SafetyState {
        match before {
            Some(SafetyState::Rejected) => SafetyState::Rejected,
            None | Some(SafetyState::Carried) if holds => SafetyState::Carried,
            None | Some(SafetyState::Carried) => SafetyState::Rejected,
        }
    }

    /// The canonical key of `state` under this translation's context.
    ///
    /// # Errors
    ///
    /// [`KeyRefusal::Encoding`] when the key exceeds the identity byte
    /// budget; no truncated key is produced.
    pub fn key(&self, state: SafetyState) -> Result<AutomatonStateKey, KeyRefusal> {
        let limits = Limits::new(self.max_bytes);
        let body =
            quire_canonical::to_vec(&state.enc(), limits).map_err(|_| KeyRefusal::Encoding)?;
        let discriminator = quire_canonical::to_vec(&Enc::text(AUTOMATON_STATE_KEY_V1), limits)
            .map_err(|_| KeyRefusal::Encoding)?;
        let length = discriminator.len() + self.context.len() + body.len() + 4;
        if u64::try_from(length).map_or(true, |length| length > self.max_bytes) {
            return Err(KeyRefusal::Encoding);
        }
        let mut octets = Vec::with_capacity(length);
        octets.push(b'[');
        octets.extend_from_slice(&discriminator);
        octets.push(b',');
        octets.extend_from_slice(&self.context);
        octets.push(b',');
        octets.extend_from_slice(&body);
        octets.push(b']');
        Ok(AutomatonStateKey(octets))
    }

    /// Admit `octets` as a key of this translation's context, refusing every
    /// FR-338 admission negative before the key is used for membership.
    ///
    /// # Errors
    ///
    /// The first [`KeyRefusal`] the octets meet.
    pub fn admit(&self, octets: &[u8]) -> Result<(AutomatonStateKey, SafetyState), KeyRefusal> {
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
        if canonical(context, self.max_bytes)? != self.context {
            return Err(KeyRefusal::ForeignContext);
        }
        let state = self.state(state)?;
        Ok((AutomatonStateKey(octets.to_vec()), state))
    }

    /// The safety state `body` encodes. `#[string_edge]`: reads an imported
    /// key's state tag by its wire spelling.
    #[qsl_attrs::string_edge]
    fn state(&self, body: NodeRef<'_>) -> Result<SafetyState, KeyRefusal> {
        let Node::Array(mut items) = body.node() else {
            return Err(KeyRefusal::Members);
        };
        let tag = match items.next().map(|tag| tag.node()) {
            Some(Node::String(tag)) => tag,
            _ => return Err(KeyRefusal::Members),
        };
        match tag {
            "safety" => {}
            "bounded" | "accept" | "reject" | "buchi" => {
                return Err(KeyRefusal::WrongForm(tag.to_owned()))
            }
            other => return Err(KeyRefusal::UnknownTag(other.to_owned())),
        }
        let [branches] = exactly(items).ok_or(KeyRefusal::Members)?;
        let Node::Array(branches) = branches.node() else {
            return Err(KeyRefusal::Members);
        };
        let mut carried = 0_usize;
        let mut count = 0_usize;
        let mut previous: Option<Vec<u8>> = None;
        for branch in branches {
            ascending(&mut previous, canonical(branch, self.max_bytes)?)?;
            count += 1;
            if self.configuration(branch)? {
                carried += 1;
            }
        }
        match (count, carried) {
            (0, _) => Ok(SafetyState::Rejected),
            (1, 1) => Ok(SafetyState::Carried),
            _ => Err(KeyRefusal::Unreachable),
        }
    }

    /// Check one elementary configuration `[obligations, past, windows,
    /// acceptance]`; whether it is exactly the carried root.
    fn configuration(&self, branch: NodeRef<'_>) -> Result<bool, KeyRefusal> {
        let Node::Array(members) = branch.node() else {
            return Err(KeyRefusal::Members);
        };
        let [obligations, past, windows, acceptance] =
            exactly(members).ok_or(KeyRefusal::Members)?;
        for fact in [past, windows, acceptance] {
            let Node::Array(mut entries) = fact.node() else {
                return Err(KeyRefusal::Members);
            };
            // Neither translated formula has a past, interval or
            // least-fixed-point occurrence for an entry to name.
            if entries.next().is_some() {
                return Err(KeyRefusal::Unresolved);
            }
        }
        let Node::Array(obligations) = obligations.node() else {
            return Err(KeyRefusal::Members);
        };
        let mut previous: Option<Vec<u8>> = None;
        let mut root = false;
        for obligation in obligations {
            ascending(&mut previous, canonical(obligation, self.max_bytes)?)?;
            root |= self.obligation(obligation)?;
        }
        if !root {
            // An obligation-free branch is true; `G a` never reaches it.
            return Err(KeyRefusal::Unreachable);
        }
        Ok(true)
    }

    /// Check one obligation `[o, s]`; it must be the carried root `G a`,
    /// the formula's one signed occurrence a state carries.
    fn obligation(&self, obligation: NodeRef<'_>) -> Result<bool, KeyRefusal> {
        let Node::Array(members) = obligation.node() else {
            return Err(KeyRefusal::Members);
        };
        let members: Vec<NodeRef<'_>> = members.collect();
        let (path, polarity) = match members.as_slice() {
            [path, polarity] => (*path, *polarity),
            // An interval obligation: neither formula has an interval.
            [path, polarity, _, _] => {
                self.path(*path)?;
                self.polarity(*polarity)?;
                return Err(KeyRefusal::Unresolved);
            }
            _ => return Err(KeyRefusal::Members),
        };
        let path = self.path(path)?;
        let positive = self.polarity(polarity)?;
        if path.is_empty() && positive {
            Ok(true)
        } else {
            Err(KeyRefusal::Unresolved)
        }
    }

    /// An occurrence path: canonical decimal operand positions.
    fn path(&self, path: NodeRef<'_>) -> Result<Vec<u64>, KeyRefusal> {
        let Node::Array(steps) = path.node() else {
            return Err(KeyRefusal::Members);
        };
        steps
            .map(|step| match step.node() {
                Node::String(text) => decimal(text),
                _ => Err(KeyRefusal::Members),
            })
            .collect()
    }

    /// A polarity; whether it is positive.
    fn polarity(&self, polarity: NodeRef<'_>) -> Result<bool, KeyRefusal> {
        match polarity.node() {
            Node::String("positive") => Ok(true),
            Node::String("negative") => Ok(false),
            _ => Err(KeyRefusal::Members),
        }
    }
}

/// Why a clause has no [`SafetyTranslation`].
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TranslationRefusal {
    /// The clause is not a reachable invariant.
    #[error(transparent)]
    NotTranslated(NotTranslated),
    /// The context does not encode.
    #[error(transparent)]
    Key(KeyRefusal),
}

/// The `holds` atom of a TP-1 clause `always holds(A)`.
pub fn invariant_atom(clause: &CheckedTemporalClause) -> Option<FormulaNodeId> {
    let formula = clause.formula();
    let FormulaNode::Unary {
        operator: UnaryTemporal::Always,
        interval: None,
        operand,
    } = formula.nodes().get(formula.root().index())?
    else {
        return None;
    };
    matches!(
        formula.nodes().get(operand.index()),
        Some(FormulaNode::Holds { .. })
    )
    .then_some(*operand)
}

/// The canonical octets of `node`.
fn canonical(node: NodeRef<'_>, max_bytes: u64) -> Result<Vec<u8>, KeyRefusal> {
    quire_canonical::to_vec(&node, Limits::new(max_bytes)).map_err(|_| KeyRefusal::Encoding)
}

/// Exactly `N` items, or `None`.
fn exactly<'d, const N: usize>(
    items: impl Iterator<Item = NodeRef<'d>>,
) -> Option<[NodeRef<'d>; N]> {
    let items: Vec<NodeRef<'d>> = items.collect();
    items.try_into().ok()
}

/// Require `next` to sort strictly after the previous set element.
fn ascending(previous: &mut Option<Vec<u8>>, next: Vec<u8>) -> Result<(), KeyRefusal> {
    if previous.as_ref().is_some_and(|previous| *previous >= next) {
        return Err(KeyRefusal::Order);
    }
    *previous = Some(next);
    Ok(())
}

/// A canonical unsigned decimal string in the u64 domain: `"0"` or
/// `[1-9][0-9]*`, at most `"18446744073709551615"`. `#[string_edge]`: reads
/// an imported key's decimal string.
#[qsl_attrs::string_edge]
fn decimal(text: &str) -> Result<u64, KeyRefusal> {
    let refuse = || KeyRefusal::Decimal(text.to_owned());
    let canonical =
        text == "0" || (text.bytes().all(|byte| byte.is_ascii_digit()) && !text.starts_with('0'));
    if text.is_empty() || !canonical {
        return Err(refuse());
    }
    text.parse().map_err(|_| refuse())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_strings_are_canonical_and_in_the_u64_domain() {
        assert_eq!(decimal("0"), Ok(0));
        assert_eq!(decimal("18446744073709551615"), Ok(u64::MAX));
        for refused in ["", "01", "-1", "+1", "18446744073709551616", "1e3", " 1"] {
            assert_eq!(
                decimal(refused),
                Err(KeyRefusal::Decimal(refused.to_owned())),
                "{refused:?}"
            );
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-101's canonical order: initial-state admission order and each parent's
//! successor order. `explore`, `sample` and `replay` walk the same ordered lists,
//! so this module is the one place that computes them.

use qsl_foundation::digest::DigestRecord;

use crate::simulation::expansion::ExpansionStop;
use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::{canonical_bytes, state_key, EncodingRefusal, StateKey};
use qsl_foundation::IdentityLimits;

/// One initial state, keyed and digested, before admission.
pub(crate) struct InitialState<S> {
    pub(crate) state: S,
    pub(crate) key: StateKey,
    pub(crate) digest: DigestRecord,
}

/// Every distinct initial state `system` declares, in ascending state-key
/// byte order, with equal keys coalesced into one state (FR-101).
///
/// # Errors
///
/// [`EncodingRefusal`] when any initial state's `TransitionSystem::Key` has
/// no RFC 8785 encoding.
pub(crate) fn sorted_initial<S: TransitionSystem>(
    system: &S,
    identity: IdentityLimits,
) -> Result<Vec<InitialState<S::State>>, EncodingRefusal> {
    let mut items: Vec<InitialState<S::State>> = system
        .initial()
        .into_iter()
        .map(|state| -> Result<InitialState<S::State>, EncodingRefusal> {
            let (key, digest) = state_key(&system.key(&state), identity)?;
            Ok(InitialState { state, key, digest })
        })
        .collect::<Result<Vec<_>, _>>()?;
    items.sort_by(|a, b| a.key.as_bytes().cmp(b.key.as_bytes()));
    items.dedup_by(|a, b| a.key == b.key);
    Ok(items)
}

/// One successor, keyed, digested and with its transition identity's
/// canonical bytes, ready to sort.
pub(crate) struct OrderedSuccessor<T, S> {
    pub(crate) transition: T,
    pub(crate) state: S,
    pub(crate) key: StateKey,
    pub(crate) digest: DigestRecord,
}

/// One successor paired with its transition identity's canonical bytes,
/// pending the sort in [`expand`].
type PendingSuccessor<T, S> = (Vec<u8>, OrderedSuccessor<T, S>);

/// One state's expansion with its successors in canonical order, or the
/// system's stop.
pub(crate) enum Expanded<T, S, F> {
    /// The state expanded.
    Successors {
        /// Every successor, in FR-101's canonical order.
        successors: Vec<OrderedSuccessor<T, S>>,
        /// The state's findings, in the system's own order.
        findings: Vec<F>,
    },
    /// The system returned an [`ExpansionStop`].
    Stopped(ExpansionStop),
}

/// [`expand`]'s result: the expansion, or the first encoding refusal reached.
type ExpandResult<T, S, F> = Result<Expanded<T, S, F>, EncodingRefusal>;

/// Expand `state` once. Its successors come back in ascending JCS byte
/// order of their transition identity; successors with equal transition
/// identities are ordered by ascending post-state key bytes (FR-101-AC-1).
///
/// # Errors
///
/// [`EncodingRefusal`] when any successor's transition identity or
/// `TransitionSystem::Key` has no RFC 8785 encoding.
pub(crate) fn expand<S: TransitionSystem>(
    system: &S,
    state: &S::State,
    identity: IdentityLimits,
) -> ExpandResult<S::TransitionId, S::State, S::Finding> {
    let expansion = match system.successors(state) {
        Ok(expansion) => expansion,
        Err(stop) => return Ok(Expanded::Stopped(stop)),
    };
    let mut items: Vec<PendingSuccessor<S::TransitionId, S::State>> = expansion
        .successors
        .into_iter()
        .map(|(transition, next)| -> Result<_, EncodingRefusal> {
            let transition_bytes = canonical_bytes(&transition, identity)?;
            let (key, digest) = state_key(&system.key(&next), identity)?;
            Ok((
                transition_bytes,
                OrderedSuccessor {
                    transition,
                    state: next,
                    key,
                    digest,
                },
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    items.sort_by(|(a_bytes, a_item), (b_bytes, b_item)| {
        a_bytes
            .cmp(b_bytes)
            .then_with(|| a_item.key.as_bytes().cmp(b_item.key.as_bytes()))
    });
    Ok(Expanded::Successors {
        successors: items.into_iter().map(|(_, item)| item).collect(),
        findings: expansion.findings,
    })
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-101's canonical order: initial-state admission order and each parent's
//! successor order. Both `explore` and `sample` walk the same ordered lists,
//! so this module is the one place that computes them.

use qsl_foundation::digest::DigestRecord;

use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::{canonical_bytes, state_key, EncodingRefusal, StateKey};

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
) -> Result<Vec<InitialState<S::State>>, EncodingRefusal> {
    let mut items: Vec<InitialState<S::State>> = system
        .initial()
        .into_iter()
        .map(|state| -> Result<InitialState<S::State>, EncodingRefusal> {
            let (key, digest) = state_key(&system.key(&state))?;
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
/// pending the sort in [`ordered_successors`].
type PendingSuccessor<T, S> = (Vec<u8>, OrderedSuccessor<T, S>);

/// [`ordered_successors`]'s result: every successor, in canonical order, or
/// the first encoding refusal reached.
type OrderedSuccessors<T, S> = Result<Vec<OrderedSuccessor<T, S>>, EncodingRefusal>;

/// `state`'s successors, in ascending JCS byte order of their transition
/// identity; successors with equal transition identities are ordered by
/// ascending post-state key bytes (FR-101-AC-1).
///
/// # Errors
///
/// [`EncodingRefusal`] when any successor's transition identity or
/// `TransitionSystem::Key` has no RFC 8785 encoding.
pub(crate) fn ordered_successors<S: TransitionSystem>(
    system: &S,
    state: &S::State,
) -> OrderedSuccessors<S::TransitionId, S::State> {
    let mut items: Vec<PendingSuccessor<S::TransitionId, S::State>> = system
        .successors(state)
        .into_iter()
        .map(|(transition, next)| -> Result<_, EncodingRefusal> {
            let transition_bytes = canonical_bytes(&transition)?;
            let (key, digest) = state_key(&system.key(&next))?;
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
    Ok(items.into_iter().map(|(_, item)| item).collect())
}

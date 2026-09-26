// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-101's canonical order: initial-state admission order and each parent's
//! successor order. Both `explore` and `sample` walk the same ordered lists,
//! so this module is the one place that computes them.

use qsl_foundation::digest::DigestRecord;

use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::{canonical_bytes, state_key, StateKey};

/// One initial state, keyed and digested, before admission.
pub(crate) struct InitialState<S> {
    pub(crate) state: S,
    pub(crate) key: StateKey,
    pub(crate) digest: DigestRecord,
}

/// Every distinct initial state `system` declares, in ascending state-key
/// byte order, with equal keys coalesced into one state (FR-101).
pub(crate) fn sorted_initial<S: TransitionSystem>(system: &S) -> Vec<InitialState<S::State>> {
    let mut items: Vec<InitialState<S::State>> = system
        .initial()
        .into_iter()
        .map(|state| {
            let (key, digest) = state_key(&system.key(&state));
            InitialState { state, key, digest }
        })
        .collect();
    items.sort_by(|a, b| a.key.as_bytes().cmp(b.key.as_bytes()));
    items.dedup_by(|a, b| a.key == b.key);
    items
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

/// `state`'s successors, in ascending JCS byte order of their transition
/// identity; successors with equal transition identities are ordered by
/// ascending post-state key bytes (FR-101-AC-1).
pub(crate) fn ordered_successors<S: TransitionSystem>(
    system: &S,
    state: &S::State,
) -> Vec<OrderedSuccessor<S::TransitionId, S::State>> {
    let mut items: Vec<PendingSuccessor<S::TransitionId, S::State>> = system
        .successors(state)
        .into_iter()
        .map(|(transition, next)| {
            let transition_bytes = canonical_bytes(&transition);
            let (key, digest) = state_key(&system.key(&next));
            (
                transition_bytes,
                OrderedSuccessor {
                    transition,
                    state: next,
                    key,
                    digest,
                },
            )
        })
        .collect();
    items.sort_by(|(a_bytes, a_item), (b_bytes, b_item)| {
        a_bytes
            .cmp(b_bytes)
            .then_with(|| a_item.key.as_bytes().cmp(b_item.key.as_bytes()))
    });
    items.into_iter().map(|(_, item)| item).collect()
}

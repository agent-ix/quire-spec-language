// SPDX-License-Identifier: AGPL-3.0-or-later
//! Numbering states in the order a caller reaches them, under the automaton
//! state limit (ADR-018 IV-6).

use std::collections::BTreeMap;

use crate::automaton::error::AutomatonError;
use crate::automaton::state::StateId;

/// The result of numbering one key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Interned {
    /// The key's number.
    pub state: StateId,
    /// Whether this call numbered the key for the first time.
    pub created: bool,
}

/// Numbers distinct keys 0, 1, 2, ... in first-reached order, refusing the
/// key that would exceed the limit. The number is storage local to one
/// explored graph; the key is the state's identity (ADR-018 §12).
#[derive(Clone, Debug)]
pub struct Numbering<K> {
    limit: u64,
    numbers: BTreeMap<K, StateId>,
    keys: Vec<K>,
}

impl<K: Ord + Clone> Numbering<K> {
    /// A numbering that admits at most `limit` distinct keys.
    pub fn new(limit: u64) -> Self {
        Self {
            limit,
            numbers: BTreeMap::new(),
            keys: Vec::new(),
        }
    }

    /// The number of `key`, numbering it when it is new. The count is
    /// checked against the limit before the key is numbered, with checked
    /// arithmetic.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::StateLimit`] when numbering `key` would exceed the
    /// limit; the numbering is unchanged.
    pub fn intern(&mut self, key: K) -> Result<Interned, AutomatonError> {
        if let Some(&state) = self.numbers.get(&key) {
            return Ok(Interned {
                state,
                created: false,
            });
        }
        let count_after = u64::try_from(self.keys.len())
            .ok()
            .and_then(|count| count.checked_add(1));
        if !count_after.is_some_and(|count| count <= self.limit) {
            return Err(AutomatonError::StateLimit { limit: self.limit });
        }
        let state = StateId::new(self.keys.len());
        self.keys.push(key.clone());
        self.numbers.insert(key, state);
        Ok(Interned {
            state,
            created: true,
        })
    }

    /// The key numbered `state`.
    pub fn key(&self, state: StateId) -> Option<&K> {
        self.keys.get(state.index())
    }

    /// The number of keys numbered.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether no key is numbered.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126's closure of a bounded execution at a terminal state: the profile's
//! false extension, shared by EN-1's model check and EN-5's product.

use std::collections::BTreeSet;

use qsl_eval::automaton::{AutomatonError, Letter, Monitor, SafetyAutomaton, StateId};

/// Whether the false extension rejects: from the monitor state `from`, read
/// the letter with every atom false until the monitor rejects, which violates
/// the event, or repeats a state, which satisfies it.
///
/// # Errors
///
/// The automaton's refusal when a step exceeds its state limit.
pub(crate) fn false_extension_rejects<A: SafetyAutomaton>(
    monitor: &mut Monitor<A>,
    from: StateId,
) -> Result<bool, AutomatonError> {
    let mut state = from;
    let mut seen = BTreeSet::from([state]);
    loop {
        state = monitor.step(state, Letter::default())?;
        if monitor.is_rejecting(state)? {
            return Ok(true);
        }
        if !seen.insert(state) {
            return Ok(false);
        }
    }
}

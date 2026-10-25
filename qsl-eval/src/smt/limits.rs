// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315's caller-selected bounds on temporal SMT query encoding.

/// Size limits for one temporal SMT query encoding (FR-315).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmtEncodingLimits {
    /// The most declared state and monitor constants.
    pub max_constants: u64,
    /// The most applications in one transition.
    pub max_applications: u64,
    /// The most atoms in a monitor letter.
    pub max_monitor_atoms: u64,
    /// The most reachable monitor states.
    pub max_monitor_states: u64,
    /// The most candidates expanded for one application.
    pub max_candidates: u64,
    /// The most bytes in a printed query.
    pub max_script_bytes: u64,
}

impl Default for SmtEncodingLimits {
    fn default() -> Self {
        Self {
            max_constants: 1 << 20,
            max_applications: 1 << 16,
            max_monitor_atoms: 8,
            max_monitor_states: 64,
            max_candidates: 1 << 16,
            max_script_bytes: 1 << 28,
        }
    }
}

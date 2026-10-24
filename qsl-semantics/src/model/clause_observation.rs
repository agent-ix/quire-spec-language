// SPDX-License-Identifier: AGPL-3.0-or-later
//! The state a model read observes. It lives in `model` because both the
//! checked layer (which classifies each read) and `model` (whose object
//! environment logs reads) name it, and `model` may not import `check`.

/// The state a model read in a state clause observes (FR-104
/// "Observations of reads"): `current` in an invariant, `pre` in a
/// precondition and `post` in a postcondition, except inside `pre(e)`,
/// where reads of `self` and reads through references obtained inside `e`
/// observe `pre`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Observation {
    /// An invariant's `at current` state.
    Current,
    /// The operation's pre-state.
    Pre,
    /// The operation's post-state.
    Post,
}

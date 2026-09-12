// SPDX-License-Identifier: AGPL-3.0-only
//! FR-043: the three v1 temporal profiles and what one interval tick means.
//!
//! A profile is selected by the admitted declaration, never inferred from a
//! trace, a backend capability report, a file shape or a similarly spelled
//! historical identity. The three interpretations are three source selections,
//! not defaults or compatible guesses.

/// One of the three registered v1 temporal interpretations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Profile {
    /// One tick is one admitted semantic-event position in the authoritative
    /// sequence. Atomic predicates are false outside a closed-complete scope.
    EventPosition,
    /// One tick is one required sample of the declared positive rational period
    /// from the declared epoch. Atomic predicates are false outside a
    /// closed-complete scope; an absent sample under a valid binding is
    /// incomplete, never a false sample.
    FixedSample,
    /// One tick is one tick of the declared timestamp unit. Quantification
    /// ranges over admitted instants inside a complete bounded window; no
    /// synthetic atom is added after closure.
    TimestampedWindow,
}

/// Exact registered identity of the event-position profile.
pub const EVENT_POSITION: &str = "quire.temporal.event-position.false-extension/v1";
/// Exact registered identity of the fixed-sample profile.
pub const FIXED_SAMPLE: &str = "quire.temporal.fixed-sample.false-extension/v1";
/// Exact registered identity of the timestamped finite-window profile.
pub const TIMESTAMPED_WINDOW: &str = "quire.temporal.timestamped-event.finite-window/v1";

impl Profile {
    /// Resolve an exact registered identity. An unknown or similarly spelled
    /// identity yields `None`; no nearest-compatible profile is selected.
    pub fn from_identity(identity: &str) -> Option<Self> {
        match identity {
            EVENT_POSITION => Some(Self::EventPosition),
            FIXED_SAMPLE => Some(Self::FixedSample),
            TIMESTAMPED_WINDOW => Some(Self::TimestampedWindow),
            _ => None,
        }
    }

    /// The exact registered identity this interpretation was selected by.
    pub fn identity(self) -> &'static str {
        match self {
            Self::EventPosition => EVENT_POSITION,
            Self::FixedSample => FIXED_SAMPLE,
            Self::TimestampedWindow => TIMESTAMPED_WINDOW,
        }
    }

    /// Whether atomic valuations extend false past a closed-complete decision
    /// scope. The finite-window profile instead quantifies only over admitted
    /// instants, so an empty universal window is true and an empty existential
    /// window is false once the authority establishes completeness.
    pub fn extends_false(self) -> bool {
        match self {
            Self::EventPosition | Self::FixedSample => true,
            Self::TimestampedWindow => false,
        }
    }

    /// Whether a progress watermark advances this clock through an offset that
    /// carries no business event. A declared sample period and a declared
    /// timestamp unit both advance without one; an event-position sequence does
    /// not advance during silence, so its watermark stays a retained premise.
    pub fn advances_on_progress(self) -> bool {
        match self {
            Self::FixedSample | Self::TimestampedWindow => true,
            Self::EventPosition => false,
        }
    }

    /// Whether an offset addresses a required position that must exist in a
    /// complete trace. Event-position and fixed-sample traces are dense in their
    /// own domain: every offset inside the scope is a required position, and a
    /// missing one is incomplete. A timestamped window is sparse: an offset with
    /// no admitted instant is simply not in the quantification range.
    pub fn requires_dense_positions(self) -> bool {
        match self {
            Self::EventPosition | Self::FixedSample => true,
            Self::TimestampedWindow => false,
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cancellation of a lifecycle operation (ADR-029 LC-3, FR-276).
//!
//! [`Cancel`] is a cloneable handle the caller owns. Any clone can cancel it
//! from any thread, with a [`CancelCause`]. The library reads no clock: a
//! frontend that owns a timer cancels with [`CancelCause::Deadline`].
//!
//! A work meter given the handle ([`crate::Meter::with_cancel`]) calls
//! [`Cancel::poll`] at every charge. A cancelled handle denies the charge as
//! an exhausted budget does, and the handle records that a charge saw it
//! ([`Cancel::tripped`]), so the operation can report its cancellation
//! rather than the denial it became on its way up. Polling in the meter's
//! own charge path keeps the check at one place instead of every charge site.

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::fmt;
use core::sync::atomic::{AtomicU8, Ordering};

/// Why a [`Cancel`] handle was cancelled (FR-276).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CancelCause {
    /// The caller asked for it.
    Requested,
    /// The frontend that owns the timer reached its deadline.
    Deadline,
}

const LIVE: u8 = 0;
const REQUESTED: u8 = 1;
const DEADLINE: u8 = 2;

impl CancelCause {
    const fn code(self) -> u8 {
        match self {
            Self::Requested => REQUESTED,
            Self::Deadline => DEADLINE,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            REQUESTED => Some(Self::Requested),
            DEADLINE => Some(Self::Deadline),
            _ => None,
        }
    }
}

/// A cancellation handle. Clones share one state, and two handles are equal
/// when they are clones of one another.
#[derive(Clone)]
pub struct Cancel {
    shared: Arc<Shared>,
}

struct Shared {
    /// The cause this handle was cancelled with, or [`LIVE`].
    state: AtomicU8,
    /// The cause a charge saw, or [`LIVE`] while none has.
    tripped: AtomicU8,
    /// Called at every [`Cancel::poll`], so a caller can watch the charges
    /// an operation makes.
    observer: Option<Box<dyn Fn() + Send + Sync>>,
}

impl Cancel {
    /// A live handle.
    pub fn new() -> Self {
        Self::with_observer(None)
    }

    /// A live handle that calls `observer` at every meter charge made under
    /// it, before the charge reads the handle.
    pub fn observing(observer: impl Fn() + Send + Sync + 'static) -> Self {
        Self::with_observer(Some(Box::new(observer)))
    }

    fn with_observer(observer: Option<Box<dyn Fn() + Send + Sync>>) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: AtomicU8::new(LIVE),
                tripped: AtomicU8::new(LIVE),
                observer,
            }),
        }
    }

    /// Cancel with `cause`. The first cancellation wins; a later one with
    /// another cause changes nothing.
    pub fn cancel(&self, cause: CancelCause) {
        // A lost race means a cause is already recorded, which is the one to
        // keep.
        let _ = self.shared.state.compare_exchange(
            LIVE,
            cause.code(),
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    /// The cause this handle was cancelled with, if it was.
    pub fn cause(&self) -> Option<CancelCause> {
        CancelCause::from_code(self.shared.state.load(Ordering::Acquire))
    }

    /// One meter charge: `true` when the charge must be denied because this
    /// handle is cancelled. A denial is recorded for [`Self::tripped`].
    pub fn poll(&self) -> bool {
        if let Some(observer) = &self.shared.observer {
            observer();
        }
        match self.cause() {
            Some(cause) => {
                // The first denial's cause is kept, as the handle's is.
                let _ = self.shared.tripped.compare_exchange(
                    LIVE,
                    cause.code(),
                    Ordering::AcqRel,
                    Ordering::Acquire,
                );
                true
            }
            None => false,
        }
    }

    /// The cause of the cancellation a charge saw, if any charge did. A
    /// handle cancelled after the last charge of an operation is not
    /// tripped, and that operation's result stands.
    pub fn tripped(&self) -> Option<CancelCause> {
        CancelCause::from_code(self.shared.tripped.load(Ordering::Acquire))
    }
}

impl Default for Cancel {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for Cancel {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.shared, &other.shared)
    }
}

impl Eq for Cancel {}

impl fmt::Debug for Cancel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cancel")
            .field("cause", &self.cause())
            .field("tripped", &self.tripped())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{Cancel, CancelCause};

    #[test]
    fn a_live_handle_denies_no_charge() {
        let cancel = Cancel::new();
        assert!(!cancel.poll());
        assert_eq!(cancel.tripped(), None);
    }

    #[test]
    fn the_first_cancellation_and_the_first_denial_keep_their_cause() {
        let cancel = Cancel::new();
        let other = cancel.clone();
        other.cancel(CancelCause::Deadline);
        other.cancel(CancelCause::Requested);
        assert_eq!(cancel.cause(), Some(CancelCause::Deadline));
        assert!(cancel.poll());
        assert_eq!(cancel.tripped(), Some(CancelCause::Deadline));
    }

    #[test]
    fn a_cancellation_no_charge_saw_is_not_a_trip() {
        let cancel = Cancel::new();
        cancel.cancel(CancelCause::Requested);
        assert_eq!(cancel.tripped(), None);
    }

    #[test]
    fn the_observer_sees_each_charge() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicU64, Ordering};
        let seen = Arc::new(AtomicU64::new(0));
        let counter = Arc::clone(&seen);
        let cancel = Cancel::observing(move || {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        assert!(!cancel.poll());
        assert!(!cancel.poll());
        assert_eq!(seen.load(Ordering::Relaxed), 2);
    }
}

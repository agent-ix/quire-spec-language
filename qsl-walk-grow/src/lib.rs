// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-walk-grow`: QSL's one `maybe_grow` wrapper (ADR-011 §6.1 layer WG,
//! ADR-030 D-1 item 6).
//!
//! Every walk in the qualified core is iterative, on `quire-walk` or in
//! arena order, and no core crate depends on this crate. Outside the core,
//! a walk whose conversion to the walker toolkit is awkward may recurse
//! natively by wrapping each recursive step in [`maybe_grow`], which grows
//! the native stack on demand. No other code calls `stacker`. Each call
//! site has a test that drives a 100,000-deep recursion through it on a
//! thread with a 512 KiB stack.

/// The stack left below which [`maybe_grow`] switches to a new segment.
#[cfg(not(kani))]
const RED_ZONE: usize = 64 * 1024;

/// The size of each new stack segment [`maybe_grow`] allocates.
#[cfg(not(kani))]
const SEGMENT: usize = 1024 * 1024;

/// Run `step`, first moving to a fresh heap-allocated stack segment when
/// less than 64 KiB of the current stack remains.
///
/// Wrap each recursive call of a native recursion in it, so the recursion
/// never exhausts the native stack; its depth is then bounded only by the
/// memory the calling stage charges. Under `cfg(kani)` it is a plain call
/// of `step`, since stack switching is outside what Kani verifies.
#[cfg(not(kani))]
pub fn maybe_grow<R>(step: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(RED_ZONE, SEGMENT, step)
}

/// Run `step`. Under `cfg(kani)` this is a plain call.
#[cfg(kani)]
pub fn maybe_grow<R>(step: impl FnOnce() -> R) -> R {
    step()
}

#[cfg(kani)]
mod proofs {
    use super::maybe_grow;

    /// Under `cfg(kani)`, `maybe_grow` returns exactly what its closure
    /// returns.
    #[kani::proof]
    fn maybe_grow_is_a_plain_call() {
        let input: u64 = kani::any();
        assert_eq!(maybe_grow(|| input.wrapping_mul(3)), input.wrapping_mul(3));
    }
}

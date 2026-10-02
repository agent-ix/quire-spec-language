// SPDX-License-Identifier: AGPL-3.0-or-later
//! `maybe_grow` carries a 100,000-deep native recursion on a thread with a
//! 512 KiB stack.

use ix_trace_rs::trace;
use qsl_walk_grow::maybe_grow;

/// A native recursion with a frame large enough that 100,000 levels need
/// far more than 512 KiB: each level keeps a 64-byte buffer live across
/// its recursive call.
fn depth_of(remaining: u32) -> u32 {
    if remaining == 0 {
        return 0;
    }
    let buffer = std::hint::black_box([0_u8; 64]);
    let below = maybe_grow(|| depth_of(remaining - 1));
    std::hint::black_box(&buffer);
    below + 1
}

#[trace("FR-356-AC-6")]
#[test]
fn maybe_grow_carries_a_100k_deep_recursion_on_a_512_kib_stack() {
    let depth = std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| depth_of(100_000))
        .expect("spawn a 512 KiB thread")
        .join()
        .expect("the recursion must not overflow a 512 KiB stack");
    assert_eq!(depth, 100_000);
}

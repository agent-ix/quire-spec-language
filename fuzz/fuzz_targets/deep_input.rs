// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-356-AC-7, TC-903: each fuzz input becomes a QSL source nested 1 to
//! 100,000 levels deep, compiled through the S1 parser, S2 forms and the S3
//! checker. Every input must return a result or a stated limit outcome;
//! a panic, abort or stack overflow fails the run.
//!
//! Run: `make fuzz-deep-input` (10,000 inputs).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let input = qsl_fuzz::DeepInput::from_bytes(data);
    // Either outcome is a result; reaching here without a panic, abort or
    // overflow is what the target checks.
    let _outcome = input.compile();
});

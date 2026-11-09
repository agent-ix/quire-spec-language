// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-jit` tests: the JIT against the interpreter over source compiled
//! through S1 to S4, tagged with the TC and AC ids they back.

mod arena;
mod call_targets;
mod clause;
mod collections;
mod common;
mod coverage;
mod differential;
mod handle;
mod kernel_mode;
mod lifecycle;
mod native;
mod parity;
mod records;
mod values;

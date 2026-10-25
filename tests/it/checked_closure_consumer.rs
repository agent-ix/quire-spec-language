// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1263: the CG-shaped fixture is a separate Cargo crate with only the facade and leaf imports.

#[path = "../fixtures/checked-closure-consumer/src/main.rs"]
mod consumer;

use ix_trace_rs::trace;

/// The fixture's consumer path actually acquires, binds, steps and runs.
#[trace("TC-1263", "FR-485-AC-3")]
#[test]
fn checked_closure_external_consumer_runs() {
    consumer::run();
}

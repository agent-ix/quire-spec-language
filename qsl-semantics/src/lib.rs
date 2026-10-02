// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §6.1 layer 3: QSL's semantic core (ADR-011 §7.3 X-6).
//!
//! In §6.1's order, `semantic_value < model < library < check core`:
//!
//! - [`value`]: the §6.2 `semantic_value` modules (definitions, enumerations,
//!   units, quantities, declarations, containment, semantic nodes), plus
//!   `value::model_query` (§6.2 `model`);
//! - [`model`]: domain packages, keys, normalization, dispatch, population
//!   and conformance, with `model::intake`, the only module that names the
//!   FCD crates;
//! - [`library`]: library resolution, package identity and the complete-V1
//!   definition bundle (`library::bundle`);
//! - [`check`] and [`family`]: the S3 check core, its family checkers and the
//!   check-core family contract.
//!
//! This crate depends on layer 2 (`qsl-forms`), F (`qsl-foundation`) and K
//! (`quire-exact`) only. It never parses: callers pass the values they pulled
//! out of their own parse. Later layers (the checked package, the S6a
//! evaluator, SEAM and native tooling) live in `quire-spec-language` and
//! depend on this crate, never the other way.

pub mod check;
pub mod family;
pub mod library;
pub mod model;
pub mod value;

#[cfg(test)]
mod qspec_diagnostics;

/// The longest each explicit heap stack grew in this thread, by name, so a
/// test can show each stack grows by a constant per node already charged.
#[cfg(test)]
pub(crate) mod stack_peak {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    thread_local! {
        static PEAKS: RefCell<BTreeMap<&'static str, usize>> =
            const { RefCell::new(BTreeMap::new()) };
    }

    /// Record that `stack` holds `len` entries.
    pub(crate) fn note(stack: &'static str, len: usize) {
        PEAKS.with(|peaks| {
            let mut peaks = peaks.borrow_mut();
            let peak = peaks.entry(stack).or_insert(0);
            *peak = (*peak).max(len);
        });
    }

    /// Counts a `quire_walk` walk's task stack from the walker's side:
    /// entering a node trades its enter task for its frame and adds one
    /// task per child, and exiting drops the frame.
    pub(crate) struct Gauge {
        stack: &'static str,
        live: usize,
    }

    impl Gauge {
        /// A gauge for a walk that starts with its root's enter task.
        pub(crate) fn new(stack: &'static str) -> Self {
            Self { stack, live: 1 }
        }

        /// A node was entered and named `children` children.
        pub(crate) fn enter(&mut self, children: usize) {
            self.live += children;
            note(self.stack, self.live);
        }

        /// A node was exited.
        pub(crate) fn exit(&mut self) {
            self.live -= 1;
        }
    }

    /// The longest `stack` grew since the last call, which resets it.
    pub(crate) fn take(stack: &'static str) -> usize {
        PEAKS.with(|peaks| peaks.borrow_mut().remove(stack).unwrap_or(0))
    }
}

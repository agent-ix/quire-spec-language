// SPDX-License-Identifier: AGPL-3.0-only
//! FR-043: retained per-binding progress, closure and completeness.
//!
//! Progress is asserted under one binding — one declaration, one clock binding
//! name and one asserted profile identity. A watermark that regresses under
//! that binding, or a completeness assertion revised in conflict under it, is a
//! typed contradiction: the retained progress is not rolled back, the retained
//! closure is not restamped and no earlier result is rewritten. Progress
//! asserted under any other declaration, clock or profile is progress for a
//! different binding and settles nothing here.
//!
//! Agent F owns observation transport and completeness authority. The ledger
//! neither verifies nor repairs an assertion; it retains what one binding has
//! already asserted so a later assertion that contradicts it is refused rather
//! than silently replacing it.

use std::collections::BTreeMap;

use super::result::{Closure, Completeness, Dimension, Refusal, Subject};
use super::trace::Trace;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct AuthenticatedBinding {
    pub package_digest: String,
    pub declaration: usize,
    pub definition_identity: String,
    pub definition_revision: String,
    pub clock: String,
    pub parameters: BTreeMap<String, String>,
}

/// The exact binding one progress assertion is made under. A foreign clock,
/// subject or profile is a different key, never the same progress.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Binding {
    /// Declaration index in the admitted package.
    pub declaration: usize,
    /// Emitted clock binding name, without the `clock:` requirement prefix.
    pub clock: String,
    /// Asserted registered temporal profile identity.
    pub profile_identity: String,
}

impl Binding {
    /// The binding a trace asserts for one declaration.
    pub fn of(declaration: usize, trace: &Trace) -> Self {
        Self {
            declaration,
            clock: trace.clock.name.clone(),
            profile_identity: trace.clock.profile_identity.clone(),
        }
    }
}

/// The progress one binding has asserted. Closure and completeness are retained
/// beside the watermark so a later assertion cannot restamp either silently.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Progress {
    /// Watermark in the profile's clock domain.
    pub watermark: i64,
    /// Input completeness asserted through that watermark.
    pub completeness: Completeness,
    /// Decision-scope closure asserted at that watermark.
    pub decision_scope: Closure,
}

impl Progress {
    /// The progress a trace asserts.
    pub fn of(trace: &Trace) -> Self {
        Self {
            watermark: trace.watermark,
            completeness: trace.completeness,
            decision_scope: trace.decision_scope,
        }
    }
}

/// Progress retained per binding across evaluations.
///
/// A caller that evaluates one declaration repeatedly as observation arrives
/// carries one ledger, so a contradiction is visible rather than absorbed. A
/// caller with no such history evaluates through [`super::evaluate`] and the
/// trace's own assertions stand alone.
#[derive(Clone, Debug, Default)]
pub struct Ledger {
    retained: BTreeMap<Binding, Progress>,
    authenticated: BTreeMap<AuthenticatedBinding, Progress>,
}

impl Ledger {
    /// An empty ledger, asserting no progress under any binding.
    pub fn new() -> Self {
        Self::default()
    }

    /// The progress retained under one binding, where that binding has asserted
    /// any. Progress under a different declaration, clock or profile is not
    /// reported here and never settles this binding's obligation.
    pub fn progress(&self, binding: &Binding) -> Option<Progress> {
        self.retained.get(binding).copied()
    }

    /// Retain one progress assertion under one binding, returning the progress
    /// now in force.
    ///
    /// A watermark below the retained one, or a completeness assertion revised
    /// from complete to incomplete under the same binding, is a contradiction:
    /// the assertion is refused and nothing retained is changed. Closure is
    /// carried forward from the assertion only once it is admitted, so a
    /// refused assertion cannot restamp it.
    pub fn record(&mut self, binding: Binding, progress: Progress) -> Result<Progress, Refusal> {
        let subject = Subject {
            declaration: binding.declaration,
            ..Subject::default()
        };
        if let Some(retained) = self.retained.get(&binding) {
            if progress.watermark < retained.watermark {
                return Err(Refusal::Progress {
                    dimension: Dimension::Watermark,
                    clock: binding.clock,
                    subject,
                });
            }
            if retained.completeness == Completeness::Complete
                && progress.completeness == Completeness::Incomplete
            {
                return Err(Refusal::Progress {
                    dimension: Dimension::Completeness,
                    clock: binding.clock,
                    subject,
                });
            }
        }
        self.retained.insert(binding, progress);
        Ok(progress)
    }

    pub(super) fn record_authenticated(
        &mut self,
        binding: AuthenticatedBinding,
        progress: Progress,
    ) -> Result<Progress, Refusal> {
        let subject = Subject {
            declaration: binding.declaration,
            ..Subject::default()
        };
        if let Some(retained) = self.authenticated.get(&binding) {
            if progress.watermark < retained.watermark {
                return Err(Refusal::Progress {
                    dimension: Dimension::Watermark,
                    clock: binding.clock,
                    subject,
                });
            }
            if retained.completeness == Completeness::Complete
                && progress.completeness == Completeness::Incomplete
            {
                return Err(Refusal::Progress {
                    dimension: Dimension::Completeness,
                    clock: binding.clock,
                    subject,
                });
            }
        }
        self.authenticated.insert(binding, progress);
        Ok(progress)
    }
}

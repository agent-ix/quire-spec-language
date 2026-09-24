// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 T-4's structured stage outcome as a family's `check` returns it.
//! `Staged`, `StageFailure` and `LimitExceeded` are the foundation
//! `diagnostic` module's (T-4); this module names only the alias.

use qsl_foundation::diagnostic::{StageFailure, Staged};

/// ADR-013 T-4's `Result<Staged<T>, StageFailure<C>>`: every S1-S4 stage
/// hook's return shape (FR-062 "structured outcome"), generic over the
/// family's own refusal cause `C`.
pub type CheckOutcome<T, C> = Result<Staged<T>, StageFailure<C>>;

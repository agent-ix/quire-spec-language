// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `absent` keyword a source `lookup<T>(p, r) absent m` wrote.
//!
//! [`AbsenceMode`] is a parsed-form value, not model-population state: it
//! records which of the three `absent` keywords the source text used, never
//! inferred from a result type (FR-153's own table). Its two consumers sit
//! on opposite sides of a layer boundary -- the parsed form (layer 2
//! `forms`/layer 5 `value::expression`) writes it, and `model::population`
//! (layer 3) reads it as a query parameter -- so it lives here in F rather
//! than in either.

/// `lookup`'s absence mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbsenceMode {
    /// `absent undefined`.
    Undefined,
    /// `absent empty`.
    Empty,
    /// `absent refused`.
    Refused,
}

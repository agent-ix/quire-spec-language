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

impl AbsenceMode {
    /// The mode's keyword, which is also its v2 `OperationMode` `absence`
    /// value spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Undefined => "undefined",
            Self::Empty => "empty",
            Self::Refused => "refused",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AbsenceMode;

    #[test]
    fn as_str_spells_each_absent_keyword() {
        assert_eq!(AbsenceMode::Undefined.as_str(), "undefined");
        assert_eq!(AbsenceMode::Empty.as_str(), "empty");
        assert_eq!(AbsenceMode::Refused.as_str(), "refused");
    }
}

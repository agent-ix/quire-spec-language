// SPDX-License-Identifier: AGPL-3.0-or-later
//! The one spelling of a preimage counter (FR-092 "Counters").
//!
//! A counter is a recursion group's `size` or `ordinal`, a `group_reference`
//! ordinal or an `operation.member` position. RFC 8785 renders a JSON number
//! exactly only up to 2^53-1, so a counter is that JSON number up to the
//! limit and its canonical decimal string beyond it. No magnitude is refused:
//! integer *values* (a bound or a literal) are always decimal strings and do
//! not come through here.

use quire_canonical::FixedShape;
use serde::{Serialize, Serializer};

/// The largest integer magnitude RFC 8785 renders exactly (2^53 - 1).
const JCS_SAFE_INTEGER: u64 = (1 << 53) - 1;

/// A non-negative preimage counter, serialized by the 2^53-1 rule.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Counter(u64);

impl Counter {
    /// The counter holding `value`.
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
}

impl From<usize> for Counter {
    fn from(value: usize) -> Self {
        // `usize` is at most 64 bits on every supported target, so the
        // fallback never runs.
        Self(u64::try_from(value).unwrap_or(u64::MAX))
    }
}

impl Serialize for Counter {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.0 <= JCS_SAFE_INTEGER {
            serializer.serialize_u64(self.0)
        } else {
            serializer.collect_str(&self.0)
        }
    }
}

/// A counter is a number or a string: no array or object.
impl FixedShape for Counter {
    const DEPTH: usize = <u64 as FixedShape>::DEPTH;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-092-AC-15: a counter is a number up to 2^53-1 and a string beyond.
    #[test]
    fn a_counter_is_a_number_to_the_safe_limit_and_a_string_beyond() {
        let spelled = |value: u64| serde_json::to_string(&Counter::new(value)).unwrap();
        assert_eq!(spelled(0), "0");
        assert_eq!(spelled(9_007_199_254_740_991), "9007199254740991");
        assert_eq!(spelled(9_007_199_254_740_992), "\"9007199254740992\"");
        assert_eq!(spelled(u64::MAX), "\"18446744073709551615\"");
    }
}

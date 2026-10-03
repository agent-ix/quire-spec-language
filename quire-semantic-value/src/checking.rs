// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checking call surface (ADR-011 §6.1 layer SV): how a standalone
//! expression is checked ([`CheckMode`]) and the node admission limits a
//! checker declares before accepting a package ([`CheckingLimits`],
//! NFR-011). `qsl-semantics`' checker consumes these; a backend names them
//! when it calls checked code.

/// How a standalone expression is checked.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CheckMode {
    /// Linking: typing and every static definedness obligation.
    Linked,
    /// Direct kernel evaluation over supplied values: typing only, so an
    /// empty `reduce` or `value(none)` is a located undefined outcome.
    Kernel,
}

/// NFR-011's default checking node ceiling: twice NFR-001's default
/// syntax-node ceiling (50,000). It is one budget for the typed expression
/// nodes and FR-093's text and recursion leaves together; the factor of two
/// is the rationale for its size, not a split the checker enforces.
pub const DEFAULT_CHECKING_NODES: u64 = 100_000;

/// NFR-011's default per-declaration preimage byte ceiling: NFR-007's
/// default package byte ceiling (16 MiB).
pub const DEFAULT_CHECKING_INPUT_BYTES: u64 = 16_777_216;

/// NFR-011's default checking work budget: the preimage byte ceiling
/// divided by the fewest bytes one charged preimage write produces (one,
/// for a flag). A declaration charges one work unit per write, so no single
/// declaration reaches this budget before its preimage reaches
/// [`DEFAULT_CHECKING_INPUT_BYTES`]. The same budget bounds the key bytes
/// FR-093's text-leaf walk materializes: each leaf charges its path's key
/// bytes.
pub const DEFAULT_CHECKING_WORK_BUDGET: u64 = DEFAULT_CHECKING_INPUT_BYTES;

/// The node admission limits this checker declares before accepting a
/// package (NFR-011). Every ceiling is used as given, above or below its
/// default: an implementation ceiling is not a domain bound (NFR-001). A
/// checked package records the limits it was checked under (the checked
/// graph's `effective_limits`).
///
/// Nesting depth is not a limit (ADR-030 D-1): every checking walk runs over
/// an explicit heap stack whose growth these ceilings charge, so an
/// expression or a type of any depth checks within them (FR-258).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CheckingLimits {
    nodes: u64,
    /// The checked-family contract's own preimage byte-length bound (the
    /// checked family's `StageLimits::input_bytes`, its one
    /// caller-configurable knob). Defaults to
    /// [`DEFAULT_CHECKING_INPUT_BYTES`]; [`Self::with_input_bytes`] sets it.
    input_bytes: u64,
    /// The checking stage's shared-meter `work_units` bound (PR
    /// #302 review finding 3 -- `LimitKind::WorkBudget`'s one
    /// caller-configurable knob, since that kind is produced by a denied
    /// charge against the contract meter, not a `StageLimits` field).
    /// Defaults to [`DEFAULT_CHECKING_WORK_BUDGET`]; [`Self::with_work_budget`]
    /// sets it.
    work_budget: u64,
}

impl CheckingLimits {
    /// Admit at most `nodes` expression nodes and text-leaf units per
    /// checked package or expression (`s3.nodes`), with the default
    /// input-byte and work ceilings. Any node count is admitted.
    pub fn new(nodes: u64) -> Self {
        Self {
            nodes,
            ..Self::default()
        }
    }

    /// The declared node limit.
    pub fn nodes(self) -> u64 {
        self.nodes
    }

    /// The checked-family contract's own preimage byte-length bound.
    pub fn input_bytes(self) -> u64 {
        self.input_bytes
    }

    /// Bound the checked-family contract's own preimage byte length:
    /// a declaration whose parsed structure encodes to more than
    /// `input_bytes` refuses with a `Limit` outcome naming
    /// `CheckingLimitKind::InputBytes`, before the identity it would have
    /// minted is ever used.
    pub fn with_input_bytes(mut self, input_bytes: u64) -> Self {
        self.input_bytes = input_bytes;
        self
    }

    /// The checking stage's shared-meter `work_units` bound.
    pub fn work_budget(self) -> u64 {
        self.work_budget
    }

    /// Bound the checking stage's shared-meter `work_units` spend: once
    /// checking has together charged more than `work_budget` work units,
    /// the next charge refuses naming `CheckingLimitKind::WorkBudget` and
    /// this bound.
    pub fn with_work_budget(mut self, work_budget: u64) -> Self {
        self.work_budget = work_budget;
        self
    }
}

impl Default for CheckingLimits {
    /// NFR-011's default ceilings: [`DEFAULT_CHECKING_NODES`] nodes,
    /// [`DEFAULT_CHECKING_INPUT_BYTES`] input bytes and a
    /// [`DEFAULT_CHECKING_WORK_BUDGET`] work budget.
    fn default() -> Self {
        Self {
            nodes: DEFAULT_CHECKING_NODES,
            input_bytes: DEFAULT_CHECKING_INPUT_BYTES,
            work_budget: DEFAULT_CHECKING_WORK_BUDGET,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `new` admits any node count and keeps the default byte and work
    /// ceilings.
    #[test]
    fn new_admits_any_node_count() {
        for nodes in [0, 1, u64::MAX] {
            let limits = CheckingLimits::new(nodes);
            assert_eq!(limits.nodes(), nodes);
            assert_eq!(limits.input_bytes(), DEFAULT_CHECKING_INPUT_BYTES);
            assert_eq!(limits.work_budget(), DEFAULT_CHECKING_WORK_BUDGET);
        }
    }

    /// The builders set only their own ceiling.
    #[test]
    fn builders_set_only_their_own_ceiling() {
        let limits = CheckingLimits::default()
            .with_input_bytes(7)
            .with_work_budget(9);
        assert_eq!(limits.input_bytes(), 7);
        assert_eq!(limits.work_budget(), 9);
        assert_eq!(limits.nodes(), DEFAULT_CHECKING_NODES);
    }
}

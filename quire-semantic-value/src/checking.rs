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

/// The largest expression nesting depth a checker may declare. The typing,
/// facts and lowering walks run over explicit heap stacks, so this bounds the
/// checked tree's size rather than protecting the host stack.
pub const MAX_CHECKING_DEPTH: u64 = 128;

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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CheckingLimits {
    nodes: u64,
    depth: u64,
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

/// A declared checking depth above [`MAX_CHECKING_DEPTH`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("checking depth {depth} exceeds the maximum {MAX_CHECKING_DEPTH}")]
pub struct DepthAboveMaximum {
    /// The requested depth.
    pub depth: u64,
}

impl CheckingLimits {
    /// Admit at most `nodes` expression nodes and text-leaf units per
    /// checked package or expression, nested at most `depth` deep, with the
    /// default input-byte and work ceilings.
    pub fn new(nodes: u64, depth: u64) -> Result<Self, DepthAboveMaximum> {
        if depth > MAX_CHECKING_DEPTH {
            return Err(DepthAboveMaximum { depth });
        }
        Ok(Self {
            nodes,
            depth,
            ..Self::default()
        })
    }

    /// The declared node limit.
    pub fn nodes(self) -> u64 {
        self.nodes
    }

    /// The declared depth limit.
    pub fn depth(self) -> u64 {
        self.depth
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
    /// [`MAX_CHECKING_DEPTH`] depth, [`DEFAULT_CHECKING_INPUT_BYTES`] input
    /// bytes and a [`DEFAULT_CHECKING_WORK_BUDGET`] work budget.
    fn default() -> Self {
        Self {
            nodes: DEFAULT_CHECKING_NODES,
            depth: MAX_CHECKING_DEPTH,
            input_bytes: DEFAULT_CHECKING_INPUT_BYTES,
            work_budget: DEFAULT_CHECKING_WORK_BUDGET,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A depth above [`MAX_CHECKING_DEPTH`] is refused naming it; the
    /// maximum itself is admitted with the default byte and work ceilings.
    #[test]
    fn new_refuses_a_depth_above_the_maximum() {
        assert_eq!(
            CheckingLimits::new(10, MAX_CHECKING_DEPTH + 1),
            Err(DepthAboveMaximum {
                depth: MAX_CHECKING_DEPTH + 1
            })
        );
        let limits = CheckingLimits::new(10, MAX_CHECKING_DEPTH).expect("maximum admitted");
        assert_eq!(limits.nodes(), 10);
        assert_eq!(limits.depth(), MAX_CHECKING_DEPTH);
        assert_eq!(limits.input_bytes(), DEFAULT_CHECKING_INPUT_BYTES);
        assert_eq!(limits.work_budget(), DEFAULT_CHECKING_WORK_BUDGET);
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
        assert_eq!(limits.depth(), MAX_CHECKING_DEPTH);
    }
}

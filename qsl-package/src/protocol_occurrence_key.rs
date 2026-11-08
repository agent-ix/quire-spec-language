// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-384 "Runtime key binding": a runtime supplies its workflow identity and
//! repeat ordinals, and the schema validates them into one occurrence key.
//!
//! The schema holds no concrete value and the key borrows every one it
//! retains: nothing is defaulted, generated or read from the package.

use quire_exact::{Integer, NodeKey};

use crate::protocol_occurrence::{ControlOwner, ProtocolOccurrenceSchema, RepeatDimension};

/// The largest workflow identity, in UTF-8 bytes (FR-384 Behavior).
const MAX_WORKFLOW_IDENTITY_BYTES: usize = 4_096;

/// A borrowed, nonempty workflow-instance identity supplied by a runtime.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WorkflowInstanceIdentity<'a>(&'a str);

impl<'a> WorkflowInstanceIdentity<'a> {
    /// Retains one caller-owned identity without interpreting its namespace.
    pub fn new(value: &'a str) -> Result<Self, OccurrenceKeyError> {
        if value.is_empty() {
            return Err(OccurrenceKeyError::EmptyWorkflowIdentity);
        }
        if value.len() > MAX_WORKFLOW_IDENTITY_BYTES {
            return Err(OccurrenceKeyError::WorkflowIdentityTooLong { bytes: value.len() });
        }
        Ok(Self(value))
    }

    /// The exact caller-supplied identity spelling.
    pub fn as_str(self) -> &'a str {
        self.0
    }
}

/// One validated runtime key, borrowing every concrete caller-supplied value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolOccurrenceKey<'a> {
    protocol: NodeKey,
    workflow: WorkflowInstanceIdentity<'a>,
    control: NodeKey,
    owner: ControlOwner,
    repeat_ordinals: &'a [Integer],
}

impl<'a> ProtocolOccurrenceKey<'a> {
    /// The `protocol_clause` node of the schema the key was bound against.
    pub fn protocol(&self) -> &NodeKey {
        &self.protocol
    }

    /// The workflow identity the caller supplied.
    pub fn workflow(&self) -> WorkflowInstanceIdentity<'a> {
        self.workflow
    }

    /// The control entry's node key.
    pub fn control(&self) -> &NodeKey {
        &self.control
    }

    /// The control entry's owner, structural ownership included.
    pub fn owner(&self) -> &ControlOwner {
        &self.owner
    }

    /// The caller's ordinals, one for each repeat dimension, outermost first.
    pub fn repeat_ordinals(&self) -> &'a [Integer] {
        self.repeat_ordinals
    }
}

/// Why a caller-supplied key component was refused.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum OccurrenceKeyError {
    /// The workflow identity is empty.
    #[error("workflow instance identity is empty")]
    EmptyWorkflowIdentity,
    /// The workflow identity is over 4096 bytes.
    #[error("workflow instance identity is {bytes} bytes; maximum is 4096")]
    WorkflowIdentityTooLong {
        /// The offered UTF-8 byte length.
        bytes: usize,
    },
    /// The control key is no entry of this schema.
    #[error("control is not an entry of the selected occurrence schema")]
    UnknownNode {
        /// The offered control key.
        node: NodeKey,
    },
    /// The offered owner is not the entry's owner.
    #[error("control owner does not match the occurrence schema")]
    WrongRole {
        /// The entry's owner.
        expected: ControlOwner,
        /// The offered owner.
        actual: ControlOwner,
    },
    /// The ordinal count is not the entry's repeat dimension count.
    #[error("repeat ordinal count is {actual}; expected {expected}")]
    RepeatCount {
        /// The entry's dimension count.
        expected: usize,
        /// The offered ordinal count.
        actual: usize,
    },
    /// An ordinal lies outside its dimension.
    #[error("repeat ordinal at dimension {dimension} is outside its domain")]
    RepeatOrdinal {
        /// The outer-to-inner dimension index.
        dimension: usize,
        /// The refused ordinal.
        value: Integer,
    },
}

impl ProtocolOccurrenceSchema {
    /// Validates caller-owned runtime values against this schema and retains
    /// them as one key (FR-384 "Runtime key binding").
    ///
    /// The first refusal, in order: unknown control, wrong owner, wrong
    /// ordinal count, ordinal outside its dimension.
    pub fn bind<'a>(
        &self,
        workflow: WorkflowInstanceIdentity<'a>,
        control: &NodeKey,
        owner: &ControlOwner,
        repeat_ordinals: &'a [Integer],
    ) -> Result<ProtocolOccurrenceKey<'a>, OccurrenceKeyError> {
        let entry = self
            .controls
            .iter()
            .find(|entry| &entry.key == control)
            .ok_or_else(|| OccurrenceKeyError::UnknownNode {
                node: control.clone(),
            })?;
        if &entry.owner != owner {
            return Err(OccurrenceKeyError::WrongRole {
                expected: entry.owner.clone(),
                actual: owner.clone(),
            });
        }
        if entry.dimensions.len() != repeat_ordinals.len() {
            return Err(OccurrenceKeyError::RepeatCount {
                expected: entry.dimensions.len(),
                actual: repeat_ordinals.len(),
            });
        }
        let zero = Integer::default();
        for (dimension, (domain, ordinal)) in
            entry.dimensions.iter().zip(repeat_ordinals).enumerate()
        {
            let inside = match domain {
                RepeatDimension::Iteration {
                    upper_exclusive: Some(upper),
                    ..
                } => *ordinal >= zero && ordinal < upper,
                RepeatDimension::Iteration {
                    upper_exclusive: None,
                    ..
                } => *ordinal >= zero,
                RepeatDimension::Exhaustion { value, .. } => ordinal == value,
            };
            if !inside {
                return Err(OccurrenceKeyError::RepeatOrdinal {
                    dimension,
                    value: ordinal.clone(),
                });
            }
        }
        Ok(ProtocolOccurrenceKey {
            protocol: self.workflow.clone(),
            workflow,
            control: entry.key.clone(),
            owner: entry.owner.clone(),
            repeat_ordinals,
        })
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// FR-384-AC-12 (FR-048-AC-1): the identity bound at zero, exact and one
    /// over, and the text is retained as given.
    #[trace("FR-384-AC-12", "FR-048-AC-1", "TC-1064", "TC-132")]
    #[test]
    fn a_workflow_identity_is_one_to_4096_bytes() {
        assert_eq!(
            WorkflowInstanceIdentity::new(""),
            Err(OccurrenceKeyError::EmptyWorkflowIdentity)
        );
        let one = WorkflowInstanceIdentity::new("O").expect("one byte is inside the bound");
        assert_eq!(one.as_str(), "O");
        let largest = "x".repeat(4_096);
        let exact = WorkflowInstanceIdentity::new(&largest).expect("4096 bytes is the bound");
        assert_eq!(exact.as_str(), largest);
        let over = "x".repeat(4_097);
        assert_eq!(
            WorkflowInstanceIdentity::new(&over),
            Err(OccurrenceKeyError::WorkflowIdentityTooLong { bytes: 4_097 })
        );
        // The bound counts bytes, not characters: 1366 three-byte characters
        // are 4098 bytes.
        let wide = "\u{20ac}".repeat(1_366);
        assert_eq!(
            WorkflowInstanceIdentity::new(&wide),
            Err(OccurrenceKeyError::WorkflowIdentityTooLong { bytes: 4_098 })
        );
    }
}

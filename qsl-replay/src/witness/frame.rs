// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-116 (ADR-012 §12.2 Witness and replay row): the `ProtocolClause`
//! family's own [`FamilyPayload`], [`FrameCounterexample`] -- a frame
//! counterexample carried on the FR-070 envelope as
//! `WitnessEnvelope<FrameCounterexample>`.
//!
//! The payload names the operation, the checked identities of its
//! `state`/`operation_anchor` and `state`/`frame` nodes and the frame's
//! `generated` occurrence (FR-104, FR-105), the invocation whose pre and
//! post snapshots the replay admits (FR-106), and the change the
//! counterexample claims. The replay reads it in `crate::execute`
//! (`replay_frame`).
//!
//! CG builds this payload from the frame witness bindings over IR's
//! `WitnessBinding` (AD-016) once agent-ix/quire-contract-ir#109 and
//! agent-ix/quire-contract-codegen#49 land; until then the envelope is
//! built in process. [`ClaimedChange::of_witness`] is the native decode: a
//! check-time [`FrameWitness`] (FR-115) as the change a payload claims.

use quire_exact::Identifier;

use super::FamilyPayload;
use crate::identity::QualifiedName;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::{DocumentRef, FrameChange, FrameWitness, SelectedObject};

/// The operation a frame counterexample refutes: its declaring object
/// type's [`QualifiedName`] (`M::T`, the model alias then the type) and the
/// operation's name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameOperation {
    /// The declaring object type, `M::T`.
    pub object: QualifiedName,
    /// The operation's name.
    pub operation: Identifier,
}

impl std::fmt::Display for FrameOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in self.object.segments() {
            write!(f, "{}::", segment.as_str())?;
        }
        f.write_str(self.operation.as_str())
    }
}

/// The change a frame counterexample claims happened outside the frame:
/// [`FrameChange`] (FR-115) without the frame permission it was checked
/// against, which the replay reads from the recompiled package instead
/// (QSpec FR-013-AC-3: the frame comes only from the package).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimedChange {
    /// A surviving object's field changed.
    FieldWrite {
        /// The object.
        object: SelectedObject,
        /// The field's declared name.
        field: String,
    },
    /// An object was created.
    Created {
        /// The created object.
        object: SelectedObject,
        /// Its most-specific type.
        type_name: DeclarationKey,
    },
    /// An object was deleted.
    Deleted {
        /// The deleted object.
        object: SelectedObject,
        /// Its most-specific type.
        type_name: DeclarationKey,
    },
    /// An object's most-specific type changed.
    Retyped {
        /// The object.
        object: SelectedObject,
        /// Its pre most-specific type.
        pre_type: DeclarationKey,
        /// Its post most-specific type.
        post_type: DeclarationKey,
    },
}

impl ClaimedChange {
    /// The native decode of a check-time frame witness (FR-115): the
    /// change `witness` found, as a payload claims it, with the object
    /// named by the witness's population and key. One arm per
    /// [`FrameChange`] variant, so a new change kind fails to compile here.
    pub fn of_witness(witness: &FrameWitness) -> Self {
        let object = |key: &str| SelectedObject {
            population: witness.population.clone(),
            key: key.to_owned(),
        };
        match &witness.change {
            FrameChange::FieldWrite {
                object: key, field, ..
            } => Self::FieldWrite {
                object: object(key),
                field: field.clone(),
            },
            FrameChange::Created {
                object: key,
                type_name,
                ..
            } => Self::Created {
                object: object(key),
                type_name: type_name.clone(),
            },
            FrameChange::Deleted {
                object: key,
                type_name,
                ..
            } => Self::Deleted {
                object: object(key),
                type_name: type_name.clone(),
            },
            FrameChange::Retyped {
                object: key,
                pre_type,
                post_type,
            } => Self::Retyped {
                object: object(key),
                pre_type: pre_type.clone(),
                post_type: post_type.clone(),
            },
        }
    }
}

/// FR-116: the `ProtocolClause` family's frame counterexample payload
/// (FR-070-AC-5). Every member is typed; none is a `String`-keyed map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameCounterexample {
    /// The operation whose frame the counterexample refutes.
    pub operation: FrameOperation,
    /// The wire identity of the operation's `state`/`operation_anchor` node
    /// (FR-105).
    pub anchor: WireNodeId,
    /// The wire identity of the operation's `state`/`frame` node (FR-105).
    pub frame: WireNodeId,
    /// The frame node's `generated` occurrence key (FR-104-AC-5).
    pub occurrence: OccurrenceKey,
    /// The `quire.state.invocation/v1` document (FR-106); it and its pre
    /// and post snapshots are read from the replay request's byte provision
    /// by their `sha256-jcs` digests.
    pub invocation: DocumentRef,
    /// The change the counterexample claims.
    pub change: ClaimedChange,
}

impl FamilyPayload for FrameCounterexample {}

#[cfg(test)]
mod tests {
    use super::*;
    use qsl_foundation::diagnostic::Code;

    fn document(name: &str) -> DocumentRef {
        DocumentRef {
            authority: "test".to_owned(),
            identity: name.to_owned(),
            revision_namespace: "ns".to_owned(),
            revision: "1".to_owned(),
            digest: [7; 32],
        }
    }

    fn key(node: &str) -> DeclarationKey {
        DeclarationKey {
            package: "test/config-version".to_owned(),
            node: node.to_owned(),
        }
    }

    fn witness(change: FrameChange) -> FrameWitness {
        FrameWitness {
            code: Code::FrameViolation,
            cause: "unauthorized-change",
            invocation: document("invocation"),
            pre: document("pre"),
            post: document("post"),
            population: "config_history".to_owned(),
            change,
        }
    }

    fn object(key: &str) -> SelectedObject {
        SelectedObject {
            population: "config_history".to_owned(),
            key: key.to_owned(),
        }
    }

    /// FR-116 Inputs' `change`: each FR-115 change kind decodes to the
    /// claimed change naming the witness's population, the object's key and
    /// the field or type, dropping the frame permission.
    #[test]
    fn each_frame_change_decodes_to_its_claimed_change() {
        let cases = [
            (
                FrameChange::FieldWrite {
                    object: "child".to_owned(),
                    field: "parent".to_owned(),
                    modifies: vec![key("ConfigVersion/versionNumber")],
                },
                ClaimedChange::FieldWrite {
                    object: object("child"),
                    field: "parent".to_owned(),
                },
            ),
            (
                FrameChange::Created {
                    object: "c2".to_owned(),
                    type_name: key("ConfigVersion"),
                    creates: Vec::new(),
                },
                ClaimedChange::Created {
                    object: object("c2"),
                    type_name: key("ConfigVersion"),
                },
            ),
            (
                FrameChange::Deleted {
                    object: "c2".to_owned(),
                    type_name: key("ConfigVersion"),
                    deletes: Vec::new(),
                },
                ClaimedChange::Deleted {
                    object: object("c2"),
                    type_name: key("ConfigVersion"),
                },
            ),
            (
                FrameChange::Retyped {
                    object: "c2".to_owned(),
                    pre_type: key("A"),
                    post_type: key("B"),
                },
                ClaimedChange::Retyped {
                    object: object("c2"),
                    pre_type: key("A"),
                    post_type: key("B"),
                },
            ),
        ];
        for (change, claimed) in cases {
            assert_eq!(ClaimedChange::of_witness(&witness(change)), claimed);
        }
    }

    #[test]
    fn frame_operation_displays_as_its_qualified_path() {
        let operation = FrameOperation {
            object: QualifiedName::new(vec![
                Identifier::new("Config").unwrap(),
                Identifier::new("ConfigVersion").unwrap(),
            ])
            .unwrap(),
            operation: Identifier::new("attemptUpdate").unwrap(),
        };
        assert_eq!(
            operation.to_string(),
            "Config::ConfigVersion::attemptUpdate"
        );
    }
}

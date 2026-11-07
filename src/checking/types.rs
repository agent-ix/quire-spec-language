// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-016: exact native type identities and borrowed model declaration indexes.

use qsl_semantics::model::admitted::{AdmittedPackage, Declaration, DeclarationKind};

/// A type declared by an admitted FR-056 domain package: an object type, an
/// FR-152 Interface or a record value type, identified by its FR-154
/// declaration key under the selection the package was admitted by.
#[derive(Clone, Copy, Debug)]
pub struct DomainType<'a> {
    /// The admitted package declaring the type.
    pub package: &'a AdmittedPackage,
    /// The type's declaration.
    pub declaration: Declaration<'a>,
}

impl<'a> DomainType<'a> {
    /// The domain type `declaration` names in `package`, when its kind types
    /// a value: an object type, an Interface or a record value type.
    pub fn new(package: &'a AdmittedPackage, declaration: Declaration<'a>) -> Option<Self> {
        match declaration.kind {
            DeclarationKind::ObjectType
            | DeclarationKind::Interface
            | DeclarationKind::RecordValueType
            | DeclarationKind::TupleValueType => Some(Self {
                package,
                declaration,
            }),
            DeclarationKind::ValueType
            | DeclarationKind::Clause
            | DeclarationKind::Namespace
            | DeclarationKind::Field
            | DeclarationKind::Operation
            | DeclarationKind::Part
            | DeclarationKind::Port
            | DeclarationKind::Connection
            | DeclarationKind::Relationship
            | DeclarationKind::Allocation
            | DeclarationKind::Population
            | DeclarationKind::EventType
            | DeclarationKind::VariantType
            | DeclarationKind::VariantCase
            | DeclarationKind::TuplePosition
            | DeclarationKind::StateMachine
            | DeclarationKind::Process
            | DeclarationKind::PersistenceInterface => None,
        }
    }

    /// Whether values of this type carry object identity (an object type or
    /// Interface), rather than being structural record values.
    pub fn is_object(&self) -> bool {
        matches!(
            self.declaration.kind,
            DeclarationKind::ObjectType | DeclarationKind::Interface
        )
    }

    /// The artifact id the type is exported under: its key's
    /// `ix://<package>/<artifact id>` segment.
    pub fn artifact_id(&self) -> &'a str {
        let key = self.declaration.key;
        key.node
            .rsplit_once('/')
            .map_or(key.node.as_str(), |(_, artifact)| artifact)
    }
}

/// An exact native type of an admitted domain package.
#[derive(Clone, Debug)]
pub enum NativeType<'a> {
    /// The profile Boolean type.
    Boolean,
    /// The opaque UUID native type from an admitted domain package.
    Uuid,
    /// The opaque String native type from an admitted domain package.
    String,
    /// Optional native value with distinct presence and payload.
    Option(Box<NativeType<'a>>),
    /// Ordered, duplicate-preserving finite sequence.
    Sequence {
        /// Exact native element type.
        element: Box<NativeType<'a>>,
        /// Inclusive maximum length.
        maximum: u32,
    },
    /// A type declared by an admitted FR-056 domain package.
    Domain(DomainType<'a>),
}

impl PartialEq for NativeType<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean, Self::Boolean)
            | (Self::Uuid, Self::Uuid)
            | (Self::String, Self::String) => true,
            (Self::Option(a), Self::Option(b)) => a == b,
            (
                Self::Sequence {
                    element: a,
                    maximum: am,
                },
                Self::Sequence {
                    element: b,
                    maximum: bm,
                },
            ) => a == b && am == bm,
            (Self::Domain(a), Self::Domain(b)) => {
                a.package.selection() == b.package.selection()
                    && a.declaration.key == b.declaration.key
            }
            _ => false,
        }
    }
}

impl Eq for NativeType<'_> {}

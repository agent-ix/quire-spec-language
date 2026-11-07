// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-016: exact native type identities and borrowed model declaration indexes.

use qsl_semantics::model::admitted::{AdmittedPackage, Declaration, DeclarationKind};
use qsl_semantics::model::domain_package::{
    DomainPackageRecord, Multiplicity, NativeValueType, ValueTypeRef,
};
use quire_exact::Presence;

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

/// What a domain type's field names, for a field access.
#[derive(Clone, Debug)]
pub enum DomainField<'a> {
    /// The field's value, typed.
    Typed(NativeType<'a>),
    /// The type declares no field of that name.
    Missing,
    /// The field's value type or multiplicity has no native type here: a
    /// native value type other than `Boolean`, a domain value type, a unit
    /// typeRef (`ix://quire/unit/...`), or a multiplicity other than `1`,
    /// `0..1` or `0..n`.
    Unrepresented,
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

    /// The type of this type's field `name`.
    pub fn field(&self, name: &str) -> DomainField<'a> {
        let Some(member) = self.package.member(self.declaration.key, name) else {
            return DomainField::Missing;
        };
        let DomainPackageRecord::FieldMember(field) = member.record else {
            return DomainField::Missing;
        };
        match domain_field_type(
            self.package,
            &field.value_type,
            field.multiplicity,
            field.presence,
        ) {
            Some(ty) => DomainField::Typed(ty),
            None => DomainField::Unrepresented,
        }
    }
}

fn domain_field_type<'a>(
    package: &'a AdmittedPackage,
    value_type: &ValueTypeRef,
    multiplicity: Multiplicity,
    presence: Presence,
) -> Option<NativeType<'a>> {
    field_native_type(
        domain_leaf_type(package, value_type)?,
        multiplicity,
        presence,
    )
}

fn domain_leaf_type<'a>(
    package: &'a AdmittedPackage,
    value_type: &ValueTypeRef,
) -> Option<NativeType<'a>> {
    Some(match value_type {
        ValueTypeRef::Native(NativeValueType::Boolean) => NativeType::Boolean,
        ValueTypeRef::Native(NativeValueType::Uuid) => NativeType::Uuid,
        ValueTypeRef::Native(NativeValueType::String) => NativeType::String,
        // The domain checker maps only the closed scalar leaves above; other
        // native leaves follow the existing unsupported-native path below.
        ValueTypeRef::Native(
            NativeValueType::Integer
            | NativeValueType::Rational
            | NativeValueType::Decimal
            | NativeValueType::Float32
            | NativeValueType::Float64
            | NativeValueType::Text
            | NativeValueType::Timestamp,
        ) => return None,
        // A unit type names a unit of the compiling QSL unit, not a native
        // scalar leaf or a package node, so the domain checker leaves it
        // unmapped.
        ValueTypeRef::Clock | ValueTypeRef::Unit(_) => return None,
        ValueTypeRef::Package(key) => {
            NativeType::Domain(DomainType::new(package, package.declaration_by_key(key)?)?)
        }
    })
}

fn field_native_type<'a>(
    leaf: NativeType<'a>,
    multiplicity: Multiplicity,
    presence: Presence,
) -> Option<NativeType<'a>> {
    let value = match (multiplicity.lower, multiplicity.upper) {
        (1, Some(1)) => leaf,
        (0, Some(maximum)) if multiplicity.ordered && !multiplicity.unique => {
            NativeType::Sequence {
                element: Box::new(leaf),
                maximum: u32::try_from(maximum).ok()?,
            }
        }
        _ => return None,
    };
    Some(match presence {
        Presence::Required => value,
        Presence::Optional => NativeType::Option(Box::new(value)),
    })
}

#[cfg(test)]
mod tests {
    use super::{field_native_type, NativeType};
    use ix_trace_rs::trace;
    use qsl_semantics::model::domain_package::Multiplicity;
    use quire_exact::Presence;

    fn multiplicity(lower: u64, upper: Option<u64>) -> Multiplicity {
        Multiplicity {
            lower,
            upper,
            ordered: true,
            unique: false,
        }
    }

    #[trace("TC-895", "FR-016-AC-10")]
    #[test]
    fn field_optional_type_uses_presence_after_multiplicity() {
        let required_singleton_collection = field_native_type(
            NativeType::Boolean,
            multiplicity(0, Some(1)),
            Presence::Required,
        );
        assert!(matches!(
            required_singleton_collection,
            Some(NativeType::Sequence { element, maximum: 1 })
                if *element == NativeType::Boolean
        ));

        let optional_scalar = field_native_type(
            NativeType::Boolean,
            multiplicity(1, Some(1)),
            Presence::Optional,
        );
        assert!(matches!(
            optional_scalar,
            Some(NativeType::Option(inner)) if *inner == NativeType::Boolean
        ));

        let optional_singleton_collection = field_native_type(
            NativeType::Boolean,
            multiplicity(0, Some(1)),
            Presence::Optional,
        );
        assert!(matches!(
            optional_singleton_collection,
            Some(NativeType::Option(inner))
                if matches!(inner.as_ref(), NativeType::Sequence { element, maximum: 1 }
                    if **element == NativeType::Boolean)
        ));
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

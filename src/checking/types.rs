// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-016: exact native type identities and borrowed model declaration indexes.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_ir as ir;

use crate::native_model::{NativeModel, ObjectRole, ScalarKind, ScalarRole, ScalarSite, Unit};
use qsl_semantics::model::admitted::{AdmittedPackage, Declaration, DeclarationKind};
use qsl_semantics::model::domain_package::{
    DomainPackageRecord, Multiplicity, NativeValueType, ValueTypeRef,
};

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
pub(crate) enum DomainField<'a> {
    /// The field's value, typed.
    Typed(NativeType<'a>),
    /// The type declares no field of that name.
    Missing,
    /// The field's value type or multiplicity has no native type here: a
    /// native value type other than `Boolean`, a domain value type, or a
    /// multiplicity other than `1`, `0..1` or `0..n`.
    Unrepresented,
}

impl<'a> DomainType<'a> {
    /// The domain type `declaration` names in `package`, when its kind types
    /// a value: an object type, an Interface or a record value type.
    pub fn new(package: &'a AdmittedPackage, declaration: Declaration<'a>) -> Option<Self> {
        match declaration.kind {
            DeclarationKind::ObjectType
            | DeclarationKind::Interface
            | DeclarationKind::RecordValueType => Some(Self {
                package,
                declaration,
            }),
            DeclarationKind::ValueType
            | DeclarationKind::Field
            | DeclarationKind::Operation
            | DeclarationKind::Part
            | DeclarationKind::Port
            | DeclarationKind::Connection
            | DeclarationKind::Relationship
            | DeclarationKind::Allocation
            | DeclarationKind::Population => None,
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
    pub(crate) fn field(&self, name: &str) -> DomainField<'a> {
        let Some(member) = self.package.member(self.declaration.key, name) else {
            return DomainField::Missing;
        };
        let DomainPackageRecord::FieldMember(field) = member.record else {
            return DomainField::Missing;
        };
        match domain_value_type(self.package, &field.value_type, field.multiplicity) {
            Some(ty) => DomainField::Typed(ty),
            None => DomainField::Unrepresented,
        }
    }
}

/// The native type of a domain value typed `value_type` with `multiplicity`,
/// or `None` when it has none here: a native value type other than
/// `Boolean`, a domain value type, or a multiplicity other than `1`, `0..1`
/// or `0..n`.
pub(crate) fn domain_value_type<'a>(
    package: &'a AdmittedPackage,
    value_type: &ValueTypeRef,
    multiplicity: Multiplicity,
) -> Option<NativeType<'a>> {
    let leaf = match value_type {
        ValueTypeRef::Native(NativeValueType::Boolean) => NativeType::Boolean,
        ValueTypeRef::Native(
            NativeValueType::Integer
            | NativeValueType::Rational
            | NativeValueType::Decimal
            | NativeValueType::Float32
            | NativeValueType::Float64
            | NativeValueType::Text,
        ) => return None,
        ValueTypeRef::Package(key) => {
            NativeType::Domain(DomainType::new(package, package.declaration_by_key(key)?)?)
        }
    };
    match (multiplicity.lower, multiplicity.upper) {
        (1, Some(1)) => Some(leaf),
        (0, Some(1)) => Some(NativeType::Option(Box::new(leaf))),
        (0, Some(maximum)) => Some(NativeType::Sequence {
            element: Box::new(leaf),
            maximum: u32::try_from(maximum).ok()?,
        }),
        _ => None,
    }
}

/// An exact native type; nominal identities are qualified by their model owner.
#[derive(Clone, Debug)]
pub enum NativeType<'a> {
    /// The profile Boolean type.
    Boolean,
    /// Explicit nominal primitive, retaining its original role and representation.
    Scalar {
        /// Exact admitted model owner.
        model: &'a NativeModel,
        /// Authored scalar identity, unit/text maximum and source locus.
        role: &'a ScalarRole,
        /// Actual primitive IR representation from a declared site.
        representation: &'a ir::ValueType,
    },
    /// Owner-qualified enumeration, with no implicit ordering.
    Enumeration {
        /// Exact admitted model.
        model: &'a NativeModel,
        /// Original enumeration declaration.
        declaration: &'a ir::EnumDeclaration,
    },
    /// Structural record, distinct from identity-bearing object access.
    Record {
        /// Exact admitted model.
        model: &'a NativeModel,
        /// Original record declaration.
        declaration: &'a ir::RecordDeclaration,
    },
    /// Identity-bearing object access under an explicit universe.
    Object {
        /// Exact admitted model.
        model: &'a NativeModel,
        /// Explicit object/reference role.
        role: &'a ObjectRole,
    },
    /// Opaque typed reference; carrier fields are not native access syntax.
    Reference {
        /// Exact admitted model.
        model: &'a NativeModel,
        /// Explicit target object/reference role.
        role: &'a ObjectRole,
    },
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

fn owner_equal(left: &NativeModel, right: &NativeModel) -> bool {
    left.environment().owner() == right.environment().owner()
}

impl PartialEq for NativeType<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean, Self::Boolean) => true,
            (
                Self::Scalar {
                    model: a, role: ar, ..
                },
                Self::Scalar {
                    model: b, role: br, ..
                },
            ) => owner_equal(a, b) && ar.name == br.name,
            (
                Self::Enumeration {
                    model: a,
                    declaration: ar,
                },
                Self::Enumeration {
                    model: b,
                    declaration: br,
                },
            ) => owner_equal(a, b) && ar.name() == br.name(),
            (
                Self::Record {
                    model: a,
                    declaration: ar,
                },
                Self::Record {
                    model: b,
                    declaration: br,
                },
            ) => owner_equal(a, b) && ar.name() == br.name(),
            (Self::Object { model: a, role: ar }, Self::Object { model: b, role: br })
            | (Self::Reference { model: a, role: ar }, Self::Reference { model: b, role: br }) => {
                owner_equal(a, b) && ar.record == br.record && ar.universe == br.universe
            }
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

impl<'a> NativeType<'a> {
    /// Exact rational representation, without integer conversion or new bounds.
    pub(super) fn rational(&self) -> Option<&'a ir::RationalType> {
        match self {
            Self::Scalar {
                representation: ir::ValueType::Rational { value },
                ..
            } => Some(value),
            _ => None,
        }
    }
    /// Numeric unit shared by integer and rational nominal roles.
    pub(super) fn numeric_unit(&self) -> Option<&'a Unit> {
        match self {
            Self::Scalar {
                role:
                    ScalarRole {
                        kind: ScalarKind::Integer { unit } | ScalarKind::Rational { unit },
                        ..
                    },
                ..
            } => Some(unit),
            _ => None,
        }
    }
    pub(super) fn integer(&self) -> Option<&'a ir::IntegerType> {
        match self {
            Self::Scalar {
                role,
                representation,
                ..
            } => match role.kind {
                ScalarKind::Integer { .. } => match representation {
                    ir::ValueType::Integer { value } => Some(value),
                    ir::ValueType::Rational { .. }
                    | ir::ValueType::Boolean
                    | ir::ValueType::Text
                    | ir::ValueType::Enum { .. }
                    | ir::ValueType::Record { .. }
                    | ir::ValueType::Option { .. }
                    | ir::ValueType::Collection { .. } => None,
                },
                ScalarKind::Rational { .. } | ScalarKind::Text { .. } => None,
            },
            Self::Boolean
            | Self::Enumeration { .. }
            | Self::Record { .. }
            | Self::Object { .. }
            | Self::Reference { .. }
            | Self::Option(_)
            | Self::Sequence { .. }
            | Self::Domain(_) => None,
        }
    }

    pub(super) fn dimensionless(&self) -> bool {
        match self {
            Self::Scalar { role, .. } => match &role.kind {
                ScalarKind::Integer { unit } => match unit {
                    Unit::Dimensionless => true,
                    Unit::Named(_) => false,
                },
                ScalarKind::Rational { .. } | ScalarKind::Text { .. } => false,
            },
            Self::Boolean
            | Self::Enumeration { .. }
            | Self::Record { .. }
            | Self::Object { .. }
            | Self::Reference { .. }
            | Self::Option(_)
            | Self::Sequence { .. }
            | Self::Domain(_) => false,
        }
    }

    pub(super) fn population_record(&self) -> Option<(&'a NativeModel, &'a ir::SymbolName)> {
        match self {
            Self::Object { model, role } | Self::Reference { model, role } => {
                Some((model, &role.record))
            }
            Self::Record { model, declaration } => Some((model, declaration.name())),
            Self::Option(value) | Self::Sequence { element: value, .. } => {
                value.population_record()
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct FrameIndex<'a> {
    pub owner: &'a ir::RequirementRef,
    pub fields: BTreeSet<(&'a ir::SymbolName, &'a ir::SymbolName)>,
    pub created: BTreeSet<&'a ir::SymbolName>,
    pub deleted: BTreeSet<&'a ir::SymbolName>,
}

#[derive(Debug)]
pub(crate) struct Catalog<'a> {
    pub model: &'a NativeModel,
    pub records: BTreeMap<&'a ir::SymbolName, &'a ir::RecordDeclaration>,
    pub enumerations: BTreeMap<&'a ir::SymbolName, &'a ir::EnumDeclaration>,
    pub values: BTreeMap<&'a ir::SymbolName, &'a ir::ValueDeclaration>,
    pub objects: BTreeMap<&'a ir::SymbolName, &'a ObjectRole>,
    pub fields: BTreeMap<(&'a ir::SymbolName, &'a ir::SymbolName), &'a ir::RecordFieldDeclaration>,
    pub ordered_fields: BTreeMap<&'a ir::SymbolName, Vec<&'a ir::RecordFieldDeclaration>>,
    pub variants: BTreeMap<&'a ir::SymbolName, BTreeSet<&'a ir::SymbolName>>,
    pub frames: BTreeMap<(&'a ir::SymbolName, &'a ir::SymbolName), FrameIndex<'a>>,
    references: BTreeMap<&'a ir::SymbolName, &'a ObjectRole>,
    scalars: BTreeMap<&'a ScalarSite, &'a ScalarRole>,
    interpretation: Interpretation,
}

#[derive(Clone, Copy, Debug)]
enum Interpretation {
    Historical,
    Composed,
}

impl<'a> Catalog<'a> {
    /// Historical expression interpretation retains its rational refusal.
    pub fn historical(model: &'a NativeModel) -> Self {
        Self::with_interpretation(model, Interpretation::Historical)
    }

    /// Composed binding exposes admitted types without granting expression support.
    pub fn composed(model: &'a NativeModel) -> Self {
        Self::with_interpretation(model, Interpretation::Composed)
    }

    fn with_interpretation(model: &'a NativeModel, interpretation: Interpretation) -> Self {
        let mut records = BTreeMap::new();
        let mut enumerations = BTreeMap::new();
        for declaration in model.environment().types() {
            match declaration {
                ir::TypeDeclaration::Record { declaration } => {
                    records.insert(declaration.name(), declaration);
                }
                ir::TypeDeclaration::Enum { declaration } => {
                    enumerations.insert(declaration.name(), declaration);
                }
            }
        }
        Self {
            model,
            interpretation,
            ordered_fields: records
                .values()
                .map(|record| {
                    let mut fields: Vec<_> = record.fields().iter().collect();
                    fields.sort_by_key(|field| field.name());
                    (record.name(), fields)
                })
                .collect(),
            frames: model
                .roles()
                .operations
                .iter()
                .map(|operation| {
                    (
                        (&operation.context, &operation.name),
                        FrameIndex {
                            owner: model.environment().owner(),
                            fields: operation
                                .frame
                                .fields
                                .iter()
                                .map(|(record, field)| (record, field))
                                .collect(),
                            created: operation.frame.created.iter().collect(),
                            deleted: operation.frame.deleted.iter().collect(),
                        },
                    )
                })
                .collect(),
            fields: records
                .values()
                .flat_map(|record| {
                    record
                        .fields()
                        .iter()
                        .map(move |field| ((record.name(), field.name()), field))
                })
                .collect(),
            variants: enumerations
                .values()
                .map(|declaration| {
                    (
                        declaration.name(),
                        declaration
                            .variants()
                            .iter()
                            .map(|variant| variant.name())
                            .collect(),
                    )
                })
                .collect(),
            records,
            enumerations,
            values: model
                .environment()
                .values()
                .iter()
                .map(|v| (v.name(), v))
                .collect(),
            objects: model
                .roles()
                .objects
                .iter()
                .map(|role| (&role.record, role))
                .collect(),
            references: model
                .roles()
                .objects
                .iter()
                .map(|role| (&role.reference, role))
                .collect(),
            scalars: model
                .roles()
                .scalars
                .iter()
                .flat_map(|role| role.sites.iter().map(move |site| (site, role)))
                .collect(),
        }
    }

    pub fn record_type(&self, name: &ir::SymbolName) -> Option<NativeType<'a>> {
        if let Some(role) = self.objects.get(name) {
            Some(NativeType::Object {
                model: self.model,
                role,
            })
        } else if let Some(role) = self.references.get(name) {
            Some(NativeType::Reference {
                model: self.model,
                role,
            })
        } else {
            self.records
                .get(name)
                .map(|declaration| NativeType::Record {
                    model: self.model,
                    declaration,
                })
        }
    }

    pub fn formal(&self, ty: &'a ir::ValueType, site: &ScalarSite) -> Option<NativeType<'a>> {
        match ty {
            ir::ValueType::Boolean => Some(NativeType::Boolean),
            ir::ValueType::Integer { .. } | ir::ValueType::Text => self.scalar(ty, site),
            ir::ValueType::Rational { .. } => match self.interpretation {
                Interpretation::Historical => None,
                Interpretation::Composed => self.scalar(ty, site),
            },
            ir::ValueType::Enum { name } => {
                self.enumerations
                    .get(name)
                    .map(|declaration| NativeType::Enumeration {
                        model: self.model,
                        declaration,
                    })
            }
            ir::ValueType::Record { name } => self.record_type(name),
            ir::ValueType::Option { value } => {
                Some(NativeType::Option(Box::new(self.formal(value, site)?)))
            }
            ir::ValueType::Collection { value } => Some(NativeType::Sequence {
                element: Box::new(self.formal(value.element(), site)?),
                maximum: value.maximum_items(),
            }),
        }
    }

    fn scalar(&self, ty: &'a ir::ValueType, site: &ScalarSite) -> Option<NativeType<'a>> {
        self.scalars.get(site).map(|role| NativeType::Scalar {
            model: self.model,
            role,
            representation: ty,
        })
    }

    pub fn equality(&self, ty: &NativeType<'_>) -> bool {
        match ty {
            NativeType::Boolean
            | NativeType::Enumeration { .. }
            | NativeType::Object { .. }
            | NativeType::Reference { .. } => true,
            // This historical expression capability does not establish rational
            // equality support, whether reached directly or through a record.
            NativeType::Scalar { role, .. } => match role.kind {
                ScalarKind::Integer { .. } | ScalarKind::Text { .. } => true,
                ScalarKind::Rational { .. } => false,
            },
            // No domain-package equality is admitted by this historical
            // checker; it reads only native models.
            NativeType::Option(_) | NativeType::Sequence { .. } | NativeType::Domain(_) => false,
            NativeType::Record { declaration, .. } => {
                // The model is acyclic and already node-bounded. Visit each
                // declaration once, avoiding recursive stack use and diamond expansion.
                let mut seen = BTreeSet::new();
                let mut pending = vec![*declaration];
                while let Some(record) = pending.pop() {
                    if !seen.insert(record.name()) {
                        continue;
                    }
                    for field in record.fields() {
                        match field.value_type() {
                            ir::ValueType::Option { .. }
                            | ir::ValueType::Collection { .. }
                            | ir::ValueType::Rational { .. } => return false,
                            ir::ValueType::Record { name } => {
                                if !self.objects.contains_key(name)
                                    && !self.references.contains_key(name)
                                {
                                    let Some(record) = self.records.get(name) else {
                                        return false;
                                    };
                                    pending.push(record);
                                }
                            }
                            ir::ValueType::Boolean
                            | ir::ValueType::Integer { .. }
                            | ir::ValueType::Text
                            | ir::ValueType::Enum { .. } => {}
                        }
                    }
                }
                true
            }
        }
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    use crate::formal_source::FormalSource;
    use crate::model_source::{read, ModelSourceLimits, FORMAT_V2};
    use crate::native_model::ModelLimits;
    use ix_trace_rs::trace;
    use qsl_foundation::{Source, SourceIdentity};

    #[test]
    #[trace("TC-120", "FR-041-AC-4", "FR-041-AC-5")]
    fn historical_and_composed_interpret_the_same_admitted_rational_differently() {
        let text = r#"{"license":"AGPL-3.0-or-later","package":"test/catalog","requirement":"Model","revision":1,
            "scalars":[{"kind":"rational","name":"Ratio","numerator_minimum":-1,"numerator_maximum":1,"maximum_denominator":2}],
            "records":[],"values":[
                {"name":"direct","kind":"state","type":{"kind":"scalar","name":"Ratio"}},
                {"name":"wrapped","kind":"state","type":{"kind":"option","value":{"kind":"scalar","name":"Ratio"}}}
            ],"objects":[],"operations":[]}"#;
        let source = FormalSource::new(
            Source::read(
                SourceIdentity {
                    authority: "test".into(),
                    identity: "catalog-model".into(),
                    revision_namespace: "test".into(),
                    revision: "1".into(),
                },
                "catalog.json",
                text.as_bytes(),
                1_048_576,
            )
            .unwrap(),
            ir::SourceIdentity::new(
                ir::SourceDocumentId::new("CatalogModel").unwrap(),
                ir::SourceRevision::new(1).unwrap(),
            ),
        );
        let model = read(source, FORMAT_V2, ModelSourceLimits::default())
            .unwrap()
            .admit(ModelLimits::default())
            .unwrap();
        let historical = Catalog::historical(&model);
        let composed = Catalog::composed(&model);
        assert_eq!(model.environment().values().len(), 2);
        for value in model.environment().values() {
            let site = ScalarSite::Value {
                name: value.name().clone(),
            };
            assert!(historical.formal(value.value_type(), &site).is_none());
            let actual = composed.formal(value.value_type(), &site).unwrap();
            let leaf = match &actual {
                NativeType::Option(inner) => inner.as_ref(),
                NativeType::Scalar { .. } => &actual,
                other => panic!("unexpected type for {}: {other:?}", value.name().as_str()),
            };
            let NativeType::Scalar {
                model: owner,
                role,
                representation: ir::ValueType::Rational { value },
            } = leaf
            else {
                panic!("the composed scalar must retain its exact rational representation")
            };
            assert!(std::ptr::eq(*owner, &model));
            assert_eq!(role.name.as_str(), "Ratio");
            assert_eq!(
                (
                    value.numerator_minimum(),
                    value.numerator_maximum(),
                    value.maximum_denominator()
                ),
                (-1, 1, 2)
            );
            assert!(leaf.integer().is_none());
            assert!(!leaf.dimensionless());
            assert!(!historical.equality(leaf));
        }
    }
}

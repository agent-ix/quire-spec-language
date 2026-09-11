// SPDX-License-Identifier: AGPL-3.0-only
//! FR-016: exact native type identities and borrowed model declaration indexes.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_ir as ir;

use crate::native_model::{NativeModel, ObjectRole, ScalarKind, ScalarRole, ScalarSite, Unit};

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
                representation: ir::ValueType::Integer { value },
                ..
            } => Some(value),
            _ => None,
        }
    }

    pub(super) fn dimensionless(&self) -> bool {
        matches!(
            self,
            Self::Scalar {
                role: ScalarRole {
                    kind: ScalarKind::Integer {
                        unit: Unit::Dimensionless
                    },
                    ..
                },
                ..
            }
        )
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
}

impl<'a> Catalog<'a> {
    pub fn new(model: &'a NativeModel) -> Self {
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
            ir::ValueType::Integer { .. }
            | ir::ValueType::Rational { .. }
            | ir::ValueType::Text => self.scalars.get(site).map(|role| NativeType::Scalar {
                model: self.model,
                role,
                representation: ty,
            }),
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

    pub fn equality(&self, ty: &NativeType<'_>) -> bool {
        match ty {
            NativeType::Boolean
            | NativeType::Scalar { .. }
            | NativeType::Enumeration { .. }
            | NativeType::Object { .. }
            | NativeType::Reference { .. } => true,
            NativeType::Option(_) | NativeType::Sequence { .. } => false,
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

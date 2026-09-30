// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-091 forms assembler, the entry of E3 (ADR-011 §2.1 E3 owner: QSL
//! `check`): it turns the S2 parsed unit of one source into the
//! [`PackageDeclarations`] value [`PackageDeclarations::check`] consumes.
//!
//! It reads the parsed unit only: no `qsl_cst` type, no source text and no
//! token text (ADR-011 §3 FB-01, FR-091-AC-20). It resolves every type form
//! of the unit to a kernel `ValueType` (the E3 name-binding phase, ADR-011
//! §1; ADR-013 O-11), each `using` alias to one of the unit's profile
//! selections, and gives each declared record and tuple the handle its
//! `ValueType::Composite` names, minted here over the unit's
//! `SourceOwner` (FB-13: only `check` mints). A refusal carries every
//! error found, each with its typed cause and the span it concerns, and no
//! partial output (ADR-011 §2.3 E3).
//!
//! Each `model M = … digest "sha256-jcs:…"` declaration names a domain
//! package I1 admitted (`model::intake::admit_unit`, FR-056): its object
//! types are named `M::T`, `T` the type's artifact id, and resolve to
//! `ValueType::Reference` of their effective identity. The package's
//! `TypeEnvironment` declares them with their supertypes, and its `models`
//! carries each admitted domain package for `check` to key model nodes
//! against (FR-094).
//!
//! An enum declaration is admitted here as an [`EnumBinding`]: `check` mints
//! its declaration key and each member's key over the unit's `SourceOwner`
//! (`node_key::nominal_key`, FB-13) and admits them with
//! `EnumDeclaration::admit` and `admit_member` (FR-091 "Enum declarations").
//! A predicate is a function of kind `Predicate` and assembles as a function.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{
    AliasForm, BuiltinType, DeclarationForm, DeclaredName, DimensionForm, EnumForm, Expression,
    FunctionDeclaration, ParsedUnit, RecordFieldForm, StateClauseForm, TypeForm, TypeFormHead,
    UnitForm,
};
use qsl_foundation::diagnostic::{CatalogCode, LimitExceeded, StageFailure};
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::{Code, Span};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, EffectiveId, IeeeWidth, Integer,
    IntegerInterval, Presence, RoundingMode, ValueType,
};

use super::check::{EnumBinding, PackageDeclarations, ResolvedSignature};
use super::lowering::{strongly_connected, AdmittedModel};
use super::node_key::{declared_type_handle, nominal_key, NodeKeyRefusal, SourceOwner};
use super::refusal::AliasKind;
use super::state_clause::{
    population_of, AttemptDeclaration, ClauseOperation, StateClauseDeclaration,
};
use super::type_form::{
    parse_rounding_mode, resolve_form, TypeFormError, TypeFormFault, TypeNames,
};
use crate::library::{ImportView, LibraryName};
use std::sync::Arc;

mod units;

use super::CheckedGraph;
use crate::model::domain_package::{
    DomainPackageRecord, FieldMemberRecord, Multiplicity, NativeValueType, OperationMemberRecord,
    ValueTypeRef,
};
use crate::model::intake::{member_identity_name, type_identity_segment, SelectedModel};
use crate::model::key::DeclarationKey;
use crate::value::declaration::{
    CompositeDeclaration, CompositeShape, DeclarationCause, FieldDeclaration, FieldRef,
    InvalidDeclaration, ObjectTypeDeclaration, OperationDeclaration, OperationLookup,
    TypeEnvironment,
};
use crate::value::enumeration::{EnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage};
use crate::value::semantic_node::{InvalidSemanticGraph, OwnerSelection};

/// The explicit limits the assembler takes (ADR-011 §2.3 Limits).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssemblyLimits {
    /// The largest scale `s` of a `decimal(c, s)` exact number the
    /// assembler forms `10^s` for (FR-091 "Dimension and unit
    /// declarations").
    pub decimal_scale: u64,
}

/// The default decimal-scale bound (FR-091).
pub const DEFAULT_DECIMAL_SCALE: u64 = 4096;

impl Default for AssemblyLimits {
    fn default() -> Self {
        Self {
            decimal_scale: DEFAULT_DECIMAL_SCALE,
        }
    }
}

/// Which unit-graph topology check refused (FR-091 "Dimension and unit
/// declarations").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopologyFault {
    /// A derived dimension whose normalized terms are empty.
    EmptyDerivedDimension,
    /// A unit whose scale is zero.
    ZeroScale,
    /// A targetless unit whose scale is not one or whose offset is not
    /// zero.
    NonIdentityRoot,
    /// A unit whose target has a different dimension.
    CrossDimensionTarget,
    /// A dimension whose units have two targetless roots.
    TwoRoots,
}

/// Why the assembler refused one part of a unit (FR-091 "The assembler
/// refuses in these cases").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssemblyCause {
    /// A qualified type name names no declaration of the unit and nothing
    /// in an admitted domain package.
    UnresolvedTypeName {
        /// The name as written.
        name: String,
    },
    /// A qualified type name names more than one declaration of the unit
    /// (ADR-013 O-11).
    AmbiguousTypeName {
        /// The name as written.
        name: String,
        /// The span of each candidate declaration's name.
        candidates: Vec<Span>,
    },
    /// A built-in constructor's declared bounds are rejected by its value
    /// type, with that value type's own cause.
    IllFormedBounds(TypeFormFault),
    /// A domain package's native `Float32`/`Float64` model field, which the
    /// assembler still refuses (QSL-285 owns admitting it). Function
    /// parameter, result and body floating types are admitted (FR-091-OQ-4).
    FloatingType {
        /// The width.
        width: IeeeWidth,
        /// The rounding mode, `exact` when none is written.
        rounding: RoundingMode,
        /// The profile selection the declaration's `using` alias names,
        /// when the type is in a function's signature.
        profile: Option<String>,
    },
    /// An alias's resolution reaches that alias again.
    AliasCycle {
        /// The cycle's dependency edges, `(alias, alias it names)`.
        edges: Vec<(String, String)>,
    },
    /// A `using` alias names no profile selection of the unit.
    UndeclaredAlias {
        /// The alias as written.
        alias: String,
    },
    /// The unit declares one selection alias more than once.
    DuplicateAlias {
        /// The alias.
        alias: String,
        /// The span of each selection declaring it.
        spans: Vec<Span>,
    },
    /// The unit's records and tuples are not an admitted declaration set
    /// (FR-143): a duplicate field name, an ill-typed member, or a
    /// recursion the rule refuses.
    InvalidTypeDeclaration(InvalidDeclaration),
    /// Admitting the unit's records and tuples reached a
    /// `TypeEnvironmentLimits` ceiling (FR-082, ADR-014 B-3).
    TypeLimit(LimitExceeded),
    /// A declared type's handle could not be encoded: a broken invariant,
    /// never a property of the source.
    Handle(NodeKeyRefusal),
    /// A name is declared by more than one dimension form, or by more than
    /// one unit form.
    DuplicateQuantityName {
        /// The name.
        name: String,
        /// The span of each declaration.
        spans: Vec<Span>,
    },
    /// A dimension term or a unit's `:` name names no dimension form, or a
    /// unit's target names no unit form.
    UnresolvedQuantityName {
        /// The name as written.
        name: String,
    },
    /// A dimension reaches itself through derived dimensions, or a unit's
    /// target chain reaches that unit again.
    QuantityCycle {
        /// The cycle's dependency edges, `(declaration, declaration it
        /// names)`.
        edges: Vec<(String, String)>,
    },
    /// An exact number `rational(n, 0)`.
    ZeroDenominator,
    /// A `decimal(c, s)` whose scale `s` is above the assembler's bound.
    DecimalScaleLimit(LimitExceeded),
    /// A unit-graph topology error, naming the declarations it concerns.
    UnitGraphTopology {
        /// Which check refused.
        fault: TopologyFault,
        /// The declarations concerned.
        declarations: Vec<String>,
    },
    /// An enum names one case more than once.
    DuplicateEnumMember {
        /// The enum's declared name.
        enumeration: String,
        /// The case.
        case: String,
        /// The span of each member declaring the case.
        spans: Vec<Span>,
    },
    /// A preimage constructor, `EnumDeclaration::admit` or `admit_member`
    /// refused a form the checks above admitted: a broken invariant of
    /// `check`, never a property of the source.
    NominalAdmission(InvalidSemanticGraph),
    /// A type position names a declaration of an imported library
    /// (`a::R` for an import qualifier `a`): an imported name stands only
    /// as a callee (ADR-015 D-5; `ill_typed`/`operator-ineligible`).
    ImportedTypeName {
        /// The qualified name as written.
        name: String,
    },
    /// An `import` declaration has no admitted import: the S4 source
    /// resolution admitted no library for its identity, so E3 refuses it
    /// rather than drop it from the package (ADR-011 §2.4, ADR-015 D-1,
    /// FR-307 `missing-selection`).
    UnsuppliedImport {
        /// The library identity the import names.
        identity: String,
    },
    /// A `model` declaration names no domain package admitted at I1.
    UnadmittedModel {
        /// The declaration's alias.
        alias: String,
    },
    /// An admitted domain package's object type has no artifact id or no
    /// effective identity in its view: a broken invariant of I1, never a
    /// property of the source.
    ModelType {
        /// The declaration's alias.
        alias: String,
        /// The object type's IR node identity.
        node: String,
    },
    /// An admitted domain package's record has no type environment form
    /// yet: a record value type, a systems part, port or allocation, or a
    /// field whose value type or multiplicity no kernel type represents.
    /// (FR-103, QSL-273: an operation is no longer refused here -- see
    /// `model_operation`.)
    UnsupportedModelMember {
        /// The declaration's alias.
        alias: String,
        /// The record's IR node identity.
        node: String,
    },
    /// A `pre` or `post` state clause's operation names no operation of its
    /// context type's effective view (FR-104, FR-103).
    UnresolvedOperation {
        /// The context type as written, `M::T`.
        context: String,
        /// The operation name as written.
        operation: String,
    },
    /// A `pre` or `post` state clause's operation names operations of
    /// several of its context type's ancestors, none more derived than the
    /// others (FR-103's effective view).
    AmbiguousOperation {
        /// The context type as written, `M::T`.
        context: String,
        /// The operation name as written.
        operation: String,
    },
    /// A state clause's context type (or the type declaring its operation)
    /// is a member type of two or more populations with no maximum, so its
    /// extent cannot name exactly one (FR-104 "Requirements").
    AmbiguousPopulation {
        /// The context type as written, `M::T`.
        context: String,
        /// The populations, in ascending `DeclarationKey` order.
        populations: Vec<DeclarationKey>,
    },
}

impl AssemblyCause {
    /// This cause's catalog code (FR-091 "Catalog codes").
    pub fn code(&self) -> Code {
        match self {
            Self::UnresolvedTypeName { .. }
            | Self::UndeclaredAlias { .. }
            | Self::UnresolvedOperation { .. } => Code::MissingDeclaration,
            Self::UnadmittedModel { .. } | Self::UnsuppliedImport { .. } => Code::MissingImport,
            Self::ModelType { .. } => Code::RuntimeInvariant,
            Self::AmbiguousTypeName { .. }
            | Self::DuplicateAlias { .. }
            | Self::DuplicateEnumMember { .. }
            | Self::DuplicateQuantityName { .. }
            | Self::AmbiguousOperation { .. }
            | Self::AmbiguousPopulation { .. } => Code::AmbiguousDeclaration,
            Self::UnresolvedQuantityName { .. } => Code::MissingDeclaration,
            Self::QuantityCycle { .. } => Code::InvalidPackage,
            Self::ZeroDenominator => Code::UndefinedExpression,
            Self::DecimalScaleLimit(_) => Code::StageLimitExceeded,
            // FR-091-OQ-12: STD-112 has not published the topology causes.
            Self::UnitGraphTopology { .. } => Code::InvalidPackage,
            Self::IllFormedBounds(_) | Self::ImportedTypeName { .. } => Code::IllTyped,
            Self::FloatingType { .. } | Self::UnsupportedModelMember { .. } => {
                Code::UnknownRequiredFeature
            }
            Self::AliasCycle { .. } => Code::InvalidPackage,
            Self::InvalidTypeDeclaration(invalid) => match &invalid.cause {
                DeclarationCause::DuplicateMember(_) => Code::AmbiguousDeclaration,
                DeclarationCause::Type(_)
                | DeclarationCause::Recursion { .. }
                | DeclarationCause::GeneralizationCycle { .. }
                | DeclarationCause::RedefinitionTarget(_)
                | DeclarationCause::RedefinitionConflict(_)
                | DeclarationCause::RedefinitionWidens(_) => Code::IllTyped,
                // Keys and object-type references come from the unit's own
                // handles and the admitted view's identities: one that does
                // not resolve is a broken invariant.
                DeclarationCause::DuplicateKey
                | DeclarationCause::UnknownDeclaration(_)
                | DeclarationCause::UnknownObjectType(_) => Code::RuntimeInvariant,
            },
            Self::TypeLimit(_) => Code::StageLimitExceeded,
            Self::Handle(_) | Self::NominalAdmission(_) => Code::RuntimeInvariant,
        }
    }

    /// This cause's catalog code and cause (FR-091 "Catalog codes",
    /// ADR-013 O-17).
    pub fn catalog_code(&self) -> CatalogCode {
        let cause = match self {
            Self::UnresolvedTypeName { .. } | Self::UnresolvedOperation { .. } => "missing-name",
            Self::AmbiguousTypeName { .. }
            | Self::DuplicateAlias { .. }
            | Self::DuplicateEnumMember { .. }
            | Self::DuplicateQuantityName { .. }
            | Self::AmbiguousOperation { .. }
            | Self::AmbiguousPopulation { .. } => "ambiguous-name",
            Self::UnresolvedQuantityName { .. } => "missing-name",
            Self::QuantityCycle { .. } => "definition-cycle",
            Self::ZeroDenominator => "unproved-nonzero",
            Self::DecimalScaleLimit(limit) => limit.kind().catalog_cause(),
            // Informational (FR-091-OQ-12): STD-112 replaces this cause
            // once QSpec publishes the topology causes; nothing reads it.
            Self::UnitGraphTopology { .. } => "unit-graph-topology",
            Self::IllFormedBounds(_) => "type-mismatch",
            Self::ImportedTypeName { .. } => "operator-ineligible",
            Self::FloatingType { .. } | Self::UnsupportedModelMember { .. } => {
                "unsupported-feature"
            }
            Self::AliasCycle { .. } => "definition-cycle",
            Self::UndeclaredAlias { .. }
            | Self::UnadmittedModel { .. }
            | Self::UnsuppliedImport { .. } => "missing-selection",
            Self::ModelType { .. } => "established-invariant-broken",
            Self::InvalidTypeDeclaration(invalid) => match &invalid.cause {
                DeclarationCause::DuplicateMember(_) => "ambiguous-name",
                DeclarationCause::Type(cause) => cause.tag().unwrap_or("type-mismatch"),
                DeclarationCause::Recursion { .. }
                | DeclarationCause::GeneralizationCycle { .. }
                | DeclarationCause::RedefinitionTarget(_)
                | DeclarationCause::RedefinitionConflict(_)
                | DeclarationCause::RedefinitionWidens(_) => "type-mismatch",
                DeclarationCause::DuplicateKey
                | DeclarationCause::UnknownDeclaration(_)
                | DeclarationCause::UnknownObjectType(_) => "established-invariant-broken",
            },
            Self::TypeLimit(limit) => limit.kind().catalog_cause(),
            Self::Handle(_) | Self::NominalAdmission(_) => "established-invariant-broken",
        };
        CatalogCode::new(self.code().as_str(), cause)
    }
}

/// One assembler error: its cause and the span it concerns, a
/// `Locus::Region` over the unit (ADR-013 T-5): the assembler runs before
/// check mints any occurrence key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssemblyError {
    /// The typed cause.
    pub cause: AssemblyCause,
    /// The span of the type form, `using` field, selection or declared
    /// name it concerns.
    pub span: Span,
}

/// The assembler's refusal: every error found in the unit, in the order
/// found (ADR-011 §2.3 E3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssemblyRefusal {
    /// Every error, never empty.
    pub errors: Vec<AssemblyError>,
}

/// A declared record's or tuple's members, as written.
enum Members {
    Record(Vec<RecordFieldForm>),
    Tuple(Vec<TypeForm>),
}

/// A declared record or tuple.
struct Composite {
    name: DeclaredName,
    members: Members,
}

/// Which declaration a declared type name binds.
#[derive(Clone, Copy)]
enum Declared {
    Alias(usize),
    Composite,
    Enum,
}

/// The unit's declarations, split by kind, and the names they bind.
struct Unit {
    aliases: Vec<AliasForm>,
    dimensions: Vec<DimensionForm>,
    units: Vec<UnitForm>,
    composites: Vec<Composite>,
    enums: Vec<EnumForm>,
    functions: Vec<FunctionDeclaration>,
    /// Each declared type name's declarations, with the span of each name.
    declared: BTreeMap<String, Vec<(Declared, Span)>>,
    /// State clause declarations in declaration order (FR-102), resolved
    /// by the caller into `StateClauseDeclaration`s (FR-104).
    state_clauses: Vec<StateClauseForm>,
    /// Each `protocol` declaration's form, in source order, for FR-113's
    /// `check::protocol_clause` checker to resolve at S3.
    protocols: Vec<qsl_forms::ProtocolDeclarationForm>,
}

impl Unit {
    fn new(forms: Vec<qsl_forms::ParsedForm>) -> Self {
        let mut unit = Self {
            aliases: Vec::new(),
            dimensions: Vec::new(),
            units: Vec::new(),
            composites: Vec::new(),
            enums: Vec::new(),
            functions: Vec::new(),
            declared: BTreeMap::new(),
            state_clauses: Vec::new(),
            protocols: Vec::new(),
        };
        for form in forms {
            match form.into_form() {
                DeclarationForm::Function(function) => unit.functions.push(*function),
                DeclarationForm::Alias(alias) => {
                    unit.declare(&alias.name, Declared::Alias(unit.aliases.len()));
                    unit.aliases.push(alias);
                }
                DeclarationForm::Record(record) => {
                    unit.declare(&record.name, Declared::Composite);
                    unit.composites.push(Composite {
                        name: record.name,
                        members: Members::Record(record.fields),
                    });
                }
                DeclarationForm::Tuple(tuple) => {
                    unit.declare(&tuple.name, Declared::Composite);
                    unit.composites.push(Composite {
                        name: tuple.name,
                        members: Members::Tuple(tuple.elements),
                    });
                }
                DeclarationForm::Dimension(dimension) => unit.dimensions.push(dimension),
                DeclarationForm::Unit(quantity) => unit.units.push(quantity),
                DeclarationForm::Enum(enumeration) => {
                    unit.declare(&enumeration.name, Declared::Enum);
                    unit.enums.push(enumeration);
                }
                DeclarationForm::StateClause(clause) => unit.state_clauses.push(*clause),
                DeclarationForm::Protocol(protocol) => unit.protocols.push(protocol),
            }
        }
        unit
    }

    fn declare(&mut self, name: &DeclaredName, declared: Declared) {
        self.declared
            .entry(name.name.clone())
            .or_default()
            .push((declared, name.span));
    }

    /// The spans of every declaration `name` binds.
    fn candidates(&self, name: &str) -> Vec<Span> {
        self.declared
            .get(name)
            .map(|declared| declared.iter().map(|(_, span)| *span).collect())
            .unwrap_or_default()
    }
}

/// The names the assembler resolves type forms against while it builds the
/// package: each declared record's and tuple's `ValueType::Composite`, each
/// alias once resolved, and each admitted domain package's object type
/// `M::T` as `ValueType::Reference` (the order `check`'s scope binds them
/// in).
#[derive(Default)]
struct Names {
    types: BTreeMap<String, Vec<ValueType>>,
    object_types: BTreeMap<String, EffectiveId>,
}

impl TypeNames for Names {
    fn named_types(&self, name: &str) -> &[ValueType] {
        self.types.get(name).map_or(&[], Vec::as_slice)
    }

    fn object_type_named(&self, name: &str) -> Option<EffectiveId> {
        self.object_types.get(name).copied()
    }
}

/// The object types of the domain package admitted for the `model`
/// declaration `alias`, each named `alias::T` with `T` its artifact id, and
/// declaring its supertypes by their effective identities and its fields
/// with their value types, presences and redefinitions, and its operations
/// (FR-103). Every other record an object type's shape would need and the
/// type environment cannot hold (a record value type, a systems part, port
/// or allocation) refuses as [`AssemblyCause::UnsupportedModelMember`].
/// Relationship and population records name no type environment entry;
/// they stay in the admitted model `check` keys model nodes from.
fn model_object_types(
    alias: &str,
    model: &SelectedModel,
    span: Span,
) -> Result<Vec<ObjectTypeDeclaration>, Vec<AssemblyError>> {
    let package = model.view.domain_package();
    let identities = model.view.type_identities();
    let error = |cause| AssemblyError { cause, span };
    let broken = |node: &str| {
        error(AssemblyCause::ModelType {
            alias: alias.to_owned(),
            node: node.to_owned(),
        })
    };
    let unsupported = |node: &str| {
        error(AssemblyCause::UnsupportedModelMember {
            alias: alias.to_owned(),
            node: node.to_owned(),
        })
    };
    let records: BTreeMap<&DeclarationKey, &DomainPackageRecord> = package
        .records
        .iter()
        .map(|record| (record.key(), record))
        .collect();
    let mut errors = Vec::new();
    let mut fields: BTreeMap<&DeclarationKey, Vec<FieldDeclaration>> = BTreeMap::new();
    let mut operations: BTreeMap<&DeclarationKey, Vec<OperationDeclaration>> = BTreeMap::new();
    for record in &package.records {
        match record {
            DomainPackageRecord::FieldMember(field) => {
                match model_field(field, &records, identities) {
                    Ok(declaration) => fields.entry(&field.owner).or_default().push(declaration),
                    Err(Unmapped::Unsupported) => errors.push(unsupported(&field.key.node)),
                    Err(Unmapped::Broken) => errors.push(broken(&field.key.node)),
                    Err(Unmapped::Float(width)) => {
                        errors.push(error(AssemblyCause::FloatingType {
                            width,
                            rounding: RoundingMode::Exact,
                            profile: None,
                        }))
                    }
                }
            }
            DomainPackageRecord::OperationMember(operation) => {
                match model_operation(operation, &records, identities) {
                    Ok(declaration) => operations
                        .entry(&operation.owner)
                        .or_default()
                        .push(declaration),
                    Err(Unmapped::Unsupported) => errors.push(unsupported(&operation.key.node)),
                    Err(Unmapped::Broken) => errors.push(broken(&operation.key.node)),
                    Err(Unmapped::Float(width)) => {
                        errors.push(error(AssemblyCause::FloatingType {
                            width,
                            rounding: RoundingMode::Exact,
                            profile: None,
                        }))
                    }
                }
            }
            DomainPackageRecord::RecordValueType(_)
            | DomainPackageRecord::Component(_)
            | DomainPackageRecord::Endpoint(_)
            | DomainPackageRecord::Allocation(_) => errors.push(unsupported(&record.key().node)),
            DomainPackageRecord::ObjectType(_)
            | DomainPackageRecord::ScalarType(_)
            | DomainPackageRecord::Relationship(_)
            | DomainPackageRecord::Population(_) => {}
        }
    }
    let mut declarations = Vec::new();
    for record in &package.records {
        let DomainPackageRecord::ObjectType(object) = record else {
            continue;
        };
        let key = &object.key;
        let (Some(artifact), Some(identity)) = (
            type_identity_segment(&key.package, &key.node),
            identities.get(key),
        ) else {
            errors.push(broken(&key.node));
            continue;
        };
        let mut supertypes = Vec::with_capacity(object.supertypes.len());
        for supertype in &object.supertypes {
            match identities.get(supertype) {
                Some(id) => supertypes.push(*id),
                None => errors.push(broken(&supertype.node)),
            }
        }
        declarations.push(
            ObjectTypeDeclaration::new(
                *identity,
                format!("{alias}::{artifact}"),
                fields.remove(key).unwrap_or_default(),
            )
            .with_supertypes(supertypes)
            .with_operations(operations.remove(key).unwrap_or_default()),
        );
    }
    // An operation whose owner is no object type: a broken record (every
    // operation record is read from inside an object type's own
    // `operations[]`, so its owner always resolves here unless I1 is
    // broken).
    for (owner, _) in operations {
        errors.push(broken(&owner.node));
    }
    // A field whose owner is no object type: its owner is a record value
    // type, already refused above, or a broken record.
    for (owner, _) in fields {
        if !matches!(
            records.get(owner),
            Some(DomainPackageRecord::RecordValueType(_))
        ) {
            errors.push(broken(&owner.node));
        }
    }
    if errors.is_empty() {
        Ok(declarations)
    } else {
        Err(errors)
    }
}

/// Why a domain package field has no type environment field.
#[derive(Debug)]
enum Unmapped {
    /// No kernel type or presence represents it.
    Unsupported,
    /// A floating type, refused as the assembler refuses every float.
    Float(IeeeWidth),
    /// A key the admitted view does not resolve: a broken invariant of I1.
    Broken,
}

/// `field` as an object type's field: named by its member name, typed by
/// its value type and multiplicity, and made `Optional` exactly when the
/// domain package's own `presence` is `optional` -- never by a lower bound
/// of `0` (QSpec's `model-complete.md` Presence row: "a lower bound of `0`
/// makes an empty collection legal and never makes a field optional").
/// `[1, 1]` gives the element type `E`; any other multiplicity gives the
/// collection its `ordered`/`unique` flags name (`Set`, `Bag`, `Sequence`
/// or `OrderedSet`), bounded when the upper bound is finite, and unbounded
/// only when the lower bound is `0` -- an unbounded multiplicity with a
/// lower bound above `0` has no kernel type (QSpec FR-322's "Model-owned
/// members" step 4). The returned declaration's own `value_type` is always the
/// unwrapped element or collection type; `check`'s `attribute`/`field`
/// readers wrap it in `Option` from `presence()` alone, as they already do
/// for every other `FieldDeclaration`.
/// A domain package's `value_type`/`multiplicity` pair as a kernel
/// [`ValueType`] (FR-056's field rule, reused by [`model_field`] and
/// [`model_operation`] for a parameter's or a result's own value type,
/// FR-103): a native `Boolean`/`Integer`, a scalar type as its `Int[lower,
/// upper]`, an object type as `Reference<M::T>`. `[1, 1]` gives the
/// element type `E`; any other multiplicity gives the collection its
/// `ordered`/`unique` flags name (`Set`, `Bag`, `Sequence` or
/// `OrderedSet`), bounded when the upper bound is finite, and unbounded
/// only when the lower bound is `0` -- an unbounded multiplicity with a
/// lower bound above `0` has no kernel type (QSpec FR-322's "Model-owned
/// members" step 4).
fn model_value_type(
    value_type: &ValueTypeRef,
    multiplicity: Multiplicity,
    records: &BTreeMap<&DeclarationKey, &DomainPackageRecord>,
    identities: &BTreeMap<DeclarationKey, EffectiveId>,
) -> Result<ValueType, Unmapped> {
    let element = match value_type {
        ValueTypeRef::Native(native) => match native {
            NativeValueType::Boolean => ValueType::Boolean,
            NativeValueType::Integer => ValueType::Integer,
            NativeValueType::Float32 => return Err(Unmapped::Float(IeeeWidth::Binary32)),
            NativeValueType::Float64 => return Err(Unmapped::Float(IeeeWidth::Binary64)),
            // Intake refuses these: their domain parameters have no Semantic
            // IR spelling, so no unparameterized kernel type stands for them.
            NativeValueType::Rational | NativeValueType::Decimal | NativeValueType::Text => {
                return Err(Unmapped::Unsupported)
            }
        },
        ValueTypeRef::Package(key) => match records.get(key) {
            Some(DomainPackageRecord::ObjectType(_)) => {
                ValueType::Reference(*identities.get(key).ok_or(Unmapped::Broken)?)
            }
            Some(DomainPackageRecord::ScalarType(scalar)) => ValueType::Int(
                IntegerInterval::new(Integer::from(scalar.lower), Integer::from(scalar.upper))
                    .map_err(|_| Unmapped::Unsupported)?,
            ),
            Some(_) => return Err(Unmapped::Unsupported),
            None => return Err(Unmapped::Broken),
        },
    };
    Ok(match (multiplicity.lower, multiplicity.upper) {
        (1, Some(1)) => element,
        (lower, upper) => {
            let kind = match (multiplicity.ordered, multiplicity.unique) {
                (true, true) => CollectionKind::OrderedSet,
                (true, false) => CollectionKind::Sequence,
                (false, true) => CollectionKind::Set,
                (false, false) => CollectionKind::Bag,
            };
            let bound = match upper {
                Some(upper) => {
                    Some(CardinalityBound::new(lower, upper).map_err(|_| Unmapped::Unsupported)?)
                }
                None if lower == 0 => None,
                None => return Err(Unmapped::Unsupported),
            };
            ValueType::collection(CollectionType::new(kind, element, bound))
        }
    })
}

/// `field` as an object type's field: named by its member name, typed by
/// [`model_value_type`], and made `Optional` exactly when the domain
/// package's own `presence` is `optional` -- never by a lower bound of `0`
/// (QSpec's `model-complete.md` Presence row: "a lower bound of `0` makes
/// an empty collection legal and never makes a field optional"). The
/// returned declaration's own `value_type` is always the unwrapped element
/// or collection type; `check`'s `attribute`/`field` readers wrap it in
/// `Option` from `presence()` alone, as they already do for every other
/// `FieldDeclaration`.
fn model_field(
    field: &FieldMemberRecord,
    records: &BTreeMap<&DeclarationKey, &DomainPackageRecord>,
    identities: &BTreeMap<DeclarationKey, EffectiveId>,
) -> Result<FieldDeclaration, Unmapped> {
    let name = member_identity_name(&field.owner.node, &field.key.node).ok_or(Unmapped::Broken)?;
    let value_type = model_value_type(&field.value_type, field.multiplicity, records, identities)?;
    let declaration = FieldDeclaration::new(name, value_type, field.presence);
    let Some(target) = &field.redefines else {
        return Ok(declaration);
    };
    let Some(DomainPackageRecord::FieldMember(redefined)) = records.get(target) else {
        return Err(Unmapped::Unsupported);
    };
    let owner = identities.get(&redefined.owner).ok_or(Unmapped::Broken)?;
    let redefined_name =
        member_identity_name(&redefined.owner.node, &redefined.key.node).ok_or(Unmapped::Broken)?;
    Ok(declaration.with_redefines(FieldRef::new(*owner, redefined_name)))
}

/// `operation` as an object type's operation (FR-103): named by its member
/// name, its parameters and result typed by [`model_value_type`] (FR-056's
/// field rule), and its effect frame carried unchanged -- I1 already
/// resolved every key it names against the whole domain package
/// (`resolve_effect`), so the assembler does no further resolution over it.
fn model_operation(
    operation: &OperationMemberRecord,
    records: &BTreeMap<&DeclarationKey, &DomainPackageRecord>,
    identities: &BTreeMap<DeclarationKey, EffectiveId>,
) -> Result<OperationDeclaration, Unmapped> {
    let name =
        member_identity_name(&operation.owner.node, &operation.key.node).ok_or(Unmapped::Broken)?;
    let mut parameters = Vec::with_capacity(operation.parameters.len());
    for parameter in &operation.parameters {
        let parameter_name = member_identity_name(&operation.key.node, &parameter.key.node)
            .ok_or(Unmapped::Broken)?;
        let value_type = model_value_type(
            &parameter.value_type,
            parameter.multiplicity,
            records,
            identities,
        )?;
        parameters.push((parameter_name.to_owned(), value_type));
    }
    let result = operation
        .result
        .as_ref()
        .map(|result| {
            model_value_type(&result.value_type, result.multiplicity, records, identities)
        })
        .transpose()?;
    Ok(OperationDeclaration::new(
        name,
        parameters,
        result,
        operation.effect.clone(),
    ))
}

/// A function signature's type forms: its parameters' and its result's,
/// in source order.
fn signature_type_forms(function: &FunctionDeclaration) -> Vec<&TypeForm> {
    function
        .parameters
        .iter()
        .map(|(_, form)| form)
        .chain(std::iter::once(&function.result))
        .collect()
}

/// The type forms inside a function's `decreases` measure and body, in
/// source order: `convert<T>` and `allInstances<T>` targets, and the named
/// types of `fold<A>`, `reduce<A>`, `count<N>` and `sum<N>` as name forms
/// over the name's own span.
fn body_type_forms(function: &FunctionDeclaration) -> Vec<TypeForm> {
    expression_type_forms(
        [function.measure.as_ref(), Some(&function.body)]
            .into_iter()
            .flatten(),
    )
}

/// Every type form written inside `roots`, in source order.
fn expression_type_forms<'e>(roots: impl IntoIterator<Item = &'e Expression>) -> Vec<TypeForm> {
    let mut forms = Vec::new();
    for root in roots {
        let mut stack = vec![root];
        while let Some(expression) = stack.pop() {
            match expression {
                Expression::Convert { target, .. } | Expression::AllInstances { target, .. } => {
                    forms.push(target.clone());
                }
                Expression::Accumulate {
                    accumulator_type: name,
                    accumulator_type_span: span,
                    ..
                }
                | Expression::Count {
                    result_type: name,
                    result_type_span: span,
                    ..
                }
                | Expression::Sum {
                    result_type: name,
                    result_type_span: span,
                    ..
                } => forms.push(TypeForm::name(name.clone(), *span)),
                Expression::Boolean(_)
                | Expression::Integer(_)
                | Expression::Rational(..)
                | Expression::Name(_)
                | Expression::Let { .. }
                | Expression::If { .. }
                | Expression::Binary { .. }
                | Expression::Negate(_)
                | Expression::Not(_)
                | Expression::Field { .. }
                | Expression::Present(_)
                | Expression::Value(_)
                | Expression::Deref(_)
                | Expression::Call { .. }
                | Expression::Record { .. }
                | Expression::Collection { .. }
                | Expression::Query { .. }
                | Expression::Flatten(_)
                | Expression::Size(_)
                | Expression::Contains { .. }
                | Expression::Lookup { .. }
                | Expression::Dispatch { .. }
                | Expression::Pre(_)
                | Expression::SelfRef
                | Expression::Result
                | Expression::Reaches { .. } => {}
                // Not the S2 seam (`Typer::infer_form`'s own doc,
                // `qsl-semantics/src/check/check/typing.rs`): an
                // unconditional probe arm so this match keeps compiling
                // under `--cfg seam_probe`.
                #[cfg(seam_probe)]
                Expression::__SeamProbe => {}
            }
            // Children last-first, so the first child is visited next and
            // forms come out in source order.
            stack.extend(expression.children().into_iter().rev());
        }
    }
    forms
}

/// The declared-name check of one type form tree (FR-091 "The assembler
/// refuses"): every name must bind exactly one declaration of the unit or
/// one object type of an admitted domain package, a `Reference<Q>` target
/// must name such an object type, and a floating type's rounding mode must
/// be spellable (FR-091-OQ-4: floating types are admitted).
fn check_names(
    unit: &Unit,
    object_types: &BTreeMap<String, EffectiveId>,
    imports: &BTreeMap<String, AdmittedImport>,
    form: &TypeForm,
    errors: &mut Vec<AssemblyError>,
) {
    let mut stack = vec![form];
    while let Some(form) = stack.pop() {
        match &form.head {
            // FR-091-OQ-4: a floating type carries its rounding mode and is
            // admitted; only an unspellable mode is refused.
            TypeFormHead::Builtin(BuiltinType::Float32 | BuiltinType::Float64) => {
                if let Some(spelled) = form.bounds.first() {
                    if parse_rounding_mode(spelled).is_none() {
                        errors.push(AssemblyError {
                            cause: AssemblyCause::IllFormedBounds(TypeFormFault::Malformed),
                            span: form.span,
                        });
                    }
                }
            }
            TypeFormHead::Builtin(BuiltinType::Reference) => {
                for target in &form.arguments {
                    let name = match &target.head {
                        TypeFormHead::Name(name) => name,
                        TypeFormHead::Builtin(_)
                        | TypeFormHead::Collection(_)
                        | TypeFormHead::Population => continue,
                    };
                    if !object_types.contains_key(name) {
                        errors.push(AssemblyError {
                            cause: AssemblyCause::UnresolvedTypeName { name: name.clone() },
                            span: target.span,
                        });
                    }
                }
            }
            TypeFormHead::Name(name) => match unit.declared.get(name).map(Vec::len) {
                None | Some(0) if object_types.contains_key(name) => {}
                None | Some(0)
                    if name
                        .split_once("::")
                        .is_some_and(|(qualifier, _)| imports.contains_key(qualifier)) =>
                {
                    errors.push(AssemblyError {
                        cause: AssemblyCause::ImportedTypeName { name: name.clone() },
                        span: form.span,
                    });
                }
                None | Some(0) => errors.push(AssemblyError {
                    cause: AssemblyCause::UnresolvedTypeName { name: name.clone() },
                    span: form.span,
                }),
                Some(1) => {}
                Some(_) => errors.push(AssemblyError {
                    cause: AssemblyCause::AmbiguousTypeName {
                        name: name.clone(),
                        candidates: unit.candidates(name),
                    },
                    span: form.span,
                }),
            },
            TypeFormHead::Builtin(
                BuiltinType::Boolean
                | BuiltinType::Integer
                | BuiltinType::Int
                | BuiltinType::Rational
                | BuiltinType::Decimal
                | BuiltinType::Text
                | BuiltinType::Option,
            )
            | TypeFormHead::Collection(_)
            | TypeFormHead::Population => stack.extend(&form.arguments),
        }
    }
}

/// The aliases one type form names, by index into the unit's aliases.
fn named_aliases(unit: &Unit, form: &TypeForm) -> Vec<usize> {
    let mut named = Vec::new();
    let mut stack = vec![form];
    while let Some(form) = stack.pop() {
        if let TypeFormHead::Name(name) = &form.head {
            if let Some([(Declared::Alias(index), _)]) = unit.declared.get(name).map(Vec::as_slice)
            {
                named.push(*index);
            }
        }
        stack.extend(&form.arguments);
    }
    named
}

/// A resolution error as an assembler error.
fn resolution_error(unit: &Unit, error: TypeFormError) -> AssemblyError {
    let cause = match error.fault {
        TypeFormFault::MissingName(name) => AssemblyCause::UnresolvedTypeName { name },
        TypeFormFault::AmbiguousName(name) => AssemblyCause::AmbiguousTypeName {
            candidates: unit.candidates(&name),
            name,
        },
        fault @ (TypeFormFault::Malformed
        | TypeFormFault::EmptyInterval
        | TypeFormFault::DenominatorBelowOne
        | TypeFormFault::MalformedDecimal
        | TypeFormFault::EmptyTextBounds
        | TypeFormFault::EmptyCardinality) => AssemblyCause::IllFormedBounds(fault),
    };
    AssemblyError {
        cause,
        span: error.span,
    }
}

/// The admitted enum of `form` under `owners` (FR-091 "Enum declarations"
/// steps 2 to 5). The cases are sorted by byte order unless the form is
/// `ordered`. A refusal here is a nominal-admission fault: the caller has
/// already refused a repeated case.
fn admit_enum(
    form: &EnumForm,
    owner: &SourceOwner,
    owners: &OwnerSelection,
) -> Result<EnumBinding, InvalidSemanticGraph> {
    let mut cases: Vec<String> = form
        .members
        .iter()
        .map(|member| member.case.name.clone())
        .collect();
    if !form.ordered {
        cases.sort();
    }
    let preimage = EnumDeclarationPreimage::new(
        owner.node_owner(),
        vec![form.name.name.clone()],
        form.ordered,
        cases.clone(),
    )?;
    let key = nominal_key(&preimage)?;
    let declaration = EnumDeclaration::admit(preimage, key, owners)?;
    let mut members = Vec::with_capacity(cases.len());
    for case in cases {
        let member = EnumMemberPreimage::new(declaration.key(), case)?;
        let member_key = nominal_key(&member)?;
        members.push(declaration.admit_member(&member, member_key)?);
    }
    Ok(EnumBinding {
        name: form.name.name.clone(),
        declaration,
        members,
    })
}

fn refuse<T>(errors: Vec<AssemblyError>) -> Result<T, AssemblyRefusal> {
    Err(AssemblyRefusal { errors })
}

/// The unit's records and tuples as one admitted declaration set (FR-143),
/// or every refusal: each refused declaration is set aside and the rest
/// admitted again, so one declaration's refusal does not hide another's.
/// A declaration refused only because it names one set aside
/// (`UnknownDeclaration`) is a consequence, not an error of its own, and is
/// not reported. Each round sets aside at least one declaration, so this
/// ends.
fn admit_types(
    mut declarations: Vec<CompositeDeclaration>,
    object_types: &[ObjectTypeDeclaration],
    spans: &BTreeMap<String, Span>,
) -> Result<TypeEnvironment, AssemblyRefusal> {
    let mut errors = Vec::new();
    loop {
        match TypeEnvironment::new(declarations.clone(), object_types.iter().cloned()) {
            Ok(types) if errors.is_empty() => return Ok(types),
            Ok(_) => return refuse(errors),
            Err(StageFailure::Limit(limit)) => {
                // Records and tuples charge no type-environment work, so
                // this is unreachable today. A ceiling names no declaration
                // and has no locus (FR-082, FR-096): the span is empty and
                // names no region, and the compile reports none.
                errors.push(AssemblyError {
                    cause: AssemblyCause::TypeLimit(limit),
                    span: Span { start: 0, end: 0 },
                });
                return refuse(errors);
            }
            Err(StageFailure::Refused(invalid)) => {
                let before = declarations.len();
                declarations.retain(|declaration| declaration.name() != invalid.declaration);
                let set_aside = declarations.len() < before;
                let consequence =
                    set_aside && matches!(invalid.cause, DeclarationCause::UnknownDeclaration(_));
                if !consequence {
                    let span = spans
                        .get(&invalid.declaration)
                        .copied()
                        .unwrap_or(Span { start: 0, end: 0 });
                    errors.push(AssemblyError {
                        cause: AssemblyCause::InvalidTypeDeclaration(invalid),
                        span,
                    });
                }
                if !set_aside {
                    return refuse(errors);
                }
            }
        }
    }
}

/// One `import` the S4 source resolution admitted (ADR-015 D-1): the
/// library identity it names, the library's verified import view (ADR-011
/// §4), and the library's checked graph, compiled from source, which E3
/// types an imported name from (ADR-015 D-5).
#[derive(Clone, Debug)]
pub struct AdmittedImport {
    /// The library identity the import names.
    pub identity: LibraryName,
    /// The library's import view, read from its emitted v2 bytes.
    pub view: ImportView,
    /// The library's checked graph.
    pub graph: Arc<CheckedGraph>,
}

impl PackageDeclarations {
    /// FR-091's assembler: the package declared by `unit`, the S2 output of
    /// the source unit `source` names, whose authority and identity are the
    /// owner of every declared node (ADR-013 O-04), over `models`, the
    /// domain packages I1 admitted for the unit's `model` declarations
    /// (`model::intake::admit_unit`). An admitted model whose alias no
    /// declaration of the unit spells binds nothing.
    ///
    /// `imports` are the imports the S4 source resolution admitted for the
    /// unit's `import` declarations (ADR-015 D-1); an `import` with no
    /// admitted entry of its identity is refused.
    pub fn assemble(
        source: RawSourceRef,
        unit: ParsedUnit,
        models: Vec<SelectedModel>,
        imports: Vec<AdmittedImport>,
    ) -> Result<Self, AssemblyRefusal> {
        Self::assemble_with_limits(source, unit, models, imports, AssemblyLimits::default())
    }

    /// [`Self::assemble`] under explicit `limits` (ADR-011 §2.3).
    pub fn assemble_with_limits(
        source: RawSourceRef,
        unit: ParsedUnit,
        models: Vec<SelectedModel>,
        imports: Vec<AdmittedImport>,
        limits: AssemblyLimits,
    ) -> Result<Self, AssemblyRefusal> {
        let (selections, forms) = unit.into_parts();
        let unit = Unit::new(forms);
        let mut errors = Vec::new();

        // Each `model` declaration's admitted domain package, and its
        // object types by name.
        let mut admitted = Vec::with_capacity(selections.models.len());
        let mut object_types = Vec::new();
        let mut object_spans = BTreeMap::new();
        for selection in &selections.models {
            let Some(model) = models.iter().find(|model| model.alias == selection.alias) else {
                errors.push(AssemblyError {
                    cause: AssemblyCause::UnadmittedModel {
                        alias: selection.alias.clone(),
                    },
                    span: selection.span,
                });
                continue;
            };
            match model_object_types(&selection.alias, model, selection.span) {
                Ok(declarations) => {
                    for declaration in &declarations {
                        object_spans.insert(declaration.name().to_owned(), selection.span);
                    }
                    object_types.extend(declarations);
                }
                Err(refused) => errors.extend(refused),
            }
            admitted.push(AdmittedModel::from_view(&model.view).with_alias(&selection.alias));
        }
        let object_names: BTreeMap<String, EffectiveId> = object_types
            .iter()
            .map(|declaration| (declaration.name().to_owned(), declaration.key()))
            .collect();

        // Selection aliases are unique within a unit, and a `using` alias
        // names one of its profile selections.
        let mut aliases: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
        for (alias, span) in selections
            .profiles
            .iter()
            .map(|profile| (profile.alias.as_str(), profile.span))
            .chain(
                selections
                    .imports
                    .iter()
                    .filter_map(|import| Some((import.alias.as_deref()?, import.span))),
            )
            .chain(
                selections
                    .models
                    .iter()
                    .map(|model| (model.alias.as_str(), model.span)),
            )
        {
            aliases.entry(alias).or_default().push(span);
        }
        for (alias, spans) in &aliases {
            if let [_, second, ..] = spans.as_slice() {
                errors.push(AssemblyError {
                    cause: AssemblyCause::DuplicateAlias {
                        alias: (*alias).to_owned(),
                        spans: spans.clone(),
                    },
                    span: *second,
                });
            }
        }
        let mut qualified = BTreeMap::new();
        for import in &selections.imports {
            let admitted = imports
                .iter()
                .find(|admitted| admitted.identity.as_str() == import.identity);
            if let (Some(admitted), Some(alias)) = (admitted, &import.alias) {
                qualified.insert(alias.clone(), admitted.clone());
            }
            if admitted.is_none() {
                errors.push(AssemblyError {
                    cause: AssemblyCause::UnsuppliedImport {
                        identity: import.identity.clone(),
                    },
                    span: import.span,
                });
            }
        }
        let mut function_selections = BTreeMap::new();
        for (index, function) in unit.functions.iter().enumerate() {
            if let Some(using) = function.using() {
                // A duplicate alias is already refused above, so the first
                // profile named `alias` is the only one that can be chosen
                // when the unit assembles.
                match selections
                    .profiles
                    .iter()
                    .find(|profile| profile.alias == using.alias)
                {
                    Some(profile) => {
                        function_selections.insert(index, profile.clone());
                    }
                    None => errors.push(AssemblyError {
                        cause: AssemblyCause::UndeclaredAlias {
                            alias: using.alias.clone(),
                        },
                        span: using.span,
                    }),
                }
            }
        }

        // A declared type is named by its declared name alone (its handle,
        // its region, every reference to it), so a name binds one
        // declaration: every declaration after the first is refused, in
        // whichever order the declarations are written.
        for (name, declared) in &unit.declared {
            let candidates: Vec<Span> = declared.iter().map(|(_, span)| *span).collect();
            for (_, span) in declared.iter().skip(1) {
                errors.push(AssemblyError {
                    cause: AssemblyCause::AmbiguousTypeName {
                        name: name.clone(),
                        candidates: candidates.clone(),
                    },
                    span: *span,
                });
            }
        }

        // An enum names each case once.
        for enumeration in &unit.enums {
            let mut spans: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
            for member in &enumeration.members {
                spans
                    .entry(member.case.name.as_str())
                    .or_default()
                    .push(member.case.span);
            }
            for (case, spans) in spans {
                if let [_, second, ..] = spans.as_slice() {
                    errors.push(AssemblyError {
                        cause: AssemblyCause::DuplicateEnumMember {
                            enumeration: enumeration.name.name.clone(),
                            case: case.to_owned(),
                            spans: spans.clone(),
                        },
                        span: *second,
                    });
                }
            }
        }

        // The unit's dimensions and units, admitted under its owner.
        let owner = SourceOwner::from(&source);
        let owners = OwnerSelection::new([owner.node_owner()]);
        let quantities =
            match units::assemble(&unit.dimensions, &unit.units, &owner, &owners, limits) {
                Ok(quantities) => Some(quantities),
                Err(found) => {
                    errors.extend(found);
                    None
                }
            };

        // Every type form names declarations of the unit or admitted model
        // object types.
        for alias in &unit.aliases {
            check_names(&unit, &object_names, &qualified, &alias.target, &mut errors);
        }
        for composite in &unit.composites {
            match &composite.members {
                Members::Record(fields) => {
                    for field in fields {
                        check_names(
                            &unit,
                            &object_names,
                            &qualified,
                            &field.type_form,
                            &mut errors,
                        );
                    }
                }
                Members::Tuple(elements) => {
                    for element in elements {
                        check_names(&unit, &object_names, &qualified, element, &mut errors);
                    }
                }
            }
        }
        for function in &unit.functions {
            for form in signature_type_forms(function) {
                check_names(&unit, &object_names, &qualified, form, &mut errors);
            }
            for form in body_type_forms(function) {
                check_names(&unit, &object_names, &qualified, &form, &mut errors);
            }
        }
        for clause in &unit.state_clauses {
            for form in expression_type_forms([&clause.body]) {
                check_names(&unit, &object_names, &qualified, &form, &mut errors);
            }
        }
        if !errors.is_empty() {
            return refuse(errors);
        }

        // Each declared record and tuple's handle, over the unit's owner.
        let mut names = Names {
            object_types: object_names.clone(),
            ..Names::default()
        };
        for (name, identity) in &object_names {
            names
                .types
                .entry(name.clone())
                .or_default()
                .push(ValueType::Reference(*identity));
        }
        // Each enum, admitted under the unit's owner: its declaration key
        // and its members' keys are minted here (FB-13).
        let mut enums = Vec::with_capacity(unit.enums.len());
        for enumeration in &unit.enums {
            match admit_enum(enumeration, &owner, &owners) {
                Ok(binding) => {
                    names
                        .types
                        .entry(enumeration.name.name.clone())
                        .or_default()
                        .push(ValueType::Enum(binding.shape()));
                    enums.push(binding);
                }
                Err(refusal) => errors.push(AssemblyError {
                    cause: AssemblyCause::NominalAdmission(refusal),
                    span: enumeration.name.span,
                }),
            }
        }
        let mut handles = Vec::with_capacity(unit.composites.len());
        for composite in &unit.composites {
            match declared_type_handle(&owner, &composite.name.name) {
                Ok(handle) => {
                    names
                        .types
                        .entry(composite.name.name.clone())
                        .or_default()
                        .push(ValueType::Composite(handle));
                    handles.push(handle);
                }
                Err(refusal) => errors.push(AssemblyError {
                    cause: AssemblyCause::Handle(refusal),
                    span: composite.name.span,
                }),
            }
        }
        if !errors.is_empty() {
            return refuse(errors);
        }

        // Aliases, each after every alias it names; a cycle is refused.
        let edges: Vec<Vec<usize>> = unit
            .aliases
            .iter()
            .map(|alias| named_aliases(&unit, &alias.target))
            .collect();
        let mut resolved: Vec<Option<ValueType>> = vec![None; unit.aliases.len()];
        let mut failed = vec![false; unit.aliases.len()];
        for component in strongly_connected(&edges) {
            let cyclic = component.len() > 1
                || component
                    .first()
                    .is_some_and(|member| edges[*member].contains(member));
            if cyclic {
                let members: BTreeSet<usize> = component.iter().copied().collect();
                let mut cycle = Vec::new();
                for &member in &component {
                    failed[member] = true;
                    for &target in &edges[member] {
                        if members.contains(&target) {
                            cycle.push((
                                unit.aliases[member].name.name.clone(),
                                unit.aliases[target].name.name.clone(),
                            ));
                        }
                    }
                }
                cycle.sort();
                cycle.dedup();
                let first = component.iter().min().copied().unwrap_or_default();
                errors.push(AssemblyError {
                    cause: AssemblyCause::AliasCycle { edges: cycle },
                    span: unit.aliases[first].name.span,
                });
                continue;
            }
            for member in component {
                if edges[member].iter().any(|target| failed[*target]) {
                    failed[member] = true;
                    continue;
                }
                let alias = &unit.aliases[member];
                match resolve_form(&names, &alias.target) {
                    Ok(value_type) => {
                        names
                            .types
                            .entry(alias.name.name.clone())
                            .or_default()
                            .push(value_type.clone());
                        resolved[member] = Some(value_type);
                    }
                    Err(error) => {
                        failed[member] = true;
                        errors.push(resolution_error(&unit, error));
                    }
                }
            }
        }
        if !errors.is_empty() {
            return refuse(errors);
        }

        // Records and tuples.
        let mut declarations = Vec::with_capacity(unit.composites.len());
        let mut declared_type_spans: BTreeMap<String, Span> = unit
            .enums
            .iter()
            .map(|enumeration| (enumeration.name.name.clone(), enumeration.name.span))
            .collect();
        for (composite, handle) in unit.composites.iter().zip(&handles) {
            let shape = match &composite.members {
                Members::Record(fields) => {
                    let mut declared = Vec::with_capacity(fields.len());
                    for field in fields {
                        match resolve_form(&names, &field.type_form) {
                            Ok(value_type) => declared.push(FieldDeclaration::new(
                                field.name.clone(),
                                value_type,
                                if field.optional {
                                    Presence::Optional
                                } else {
                                    Presence::Required
                                },
                            )),
                            Err(error) => errors.push(resolution_error(&unit, error)),
                        }
                    }
                    CompositeShape::Record(declared)
                }
                Members::Tuple(elements) => {
                    let mut declared = Vec::with_capacity(elements.len());
                    for element in elements {
                        match resolve_form(&names, element) {
                            Ok(value_type) => declared.push(value_type),
                            Err(error) => errors.push(resolution_error(&unit, error)),
                        }
                    }
                    CompositeShape::Tuple(declared)
                }
            };
            declarations.push(CompositeDeclaration::new(
                *handle,
                composite.name.name.clone(),
                shape,
            ));
            declared_type_spans.insert(composite.name.name.clone(), composite.name.span);
        }

        // Each function's check-owned resolved signature.
        let mut signatures: Vec<ResolvedSignature> = Vec::with_capacity(unit.functions.len());
        for function in &unit.functions {
            let mut parameters = Vec::with_capacity(function.parameters.len());
            for (name, form) in &function.parameters {
                match resolve_form(&names, form) {
                    Ok(value_type) => parameters.push((name.clone(), value_type)),
                    Err(error) => errors.push(resolution_error(&unit, error)),
                }
            }
            match resolve_form(&names, &function.result) {
                Ok(result) => signatures.push((parameters, result)),
                Err(error) => errors.push(resolution_error(&unit, error)),
            }
            // A type form inside the body or measure is resolved here too,
            // so its errors are assembler errors like a signature's; check
            // resolves it again against the package scope.
            for form in body_type_forms(function) {
                if let Err(error) = resolve_form(&names, &form) {
                    errors.push(resolution_error(&unit, error));
                }
            }
        }
        if !errors.is_empty() {
            return refuse(errors);
        }
        // A refused object type is located at its `model` declaration.
        let mut type_spans = object_spans;
        type_spans.extend(declared_type_spans.clone());
        let types = admit_types(declarations, &object_types, &type_spans)?;
        // SR-770 FND-006: both resolved before either refuses, so a unit
        // with a bad state clause and a bad attempt reports both in one
        // refusal (clause errors first, then attempt errors).
        let state_clauses = state_clauses(
            unit.state_clauses,
            &selections.profiles,
            &object_names,
            &types,
            &admitted,
        );
        let protocol_attempts =
            protocol_attempts(&unit.protocols, &object_names, &types, &admitted);
        let (state_clauses, protocol_attempts) = match (state_clauses, protocol_attempts) {
            (Ok(clauses), Ok(attempts)) => (clauses, attempts),
            (clauses, attempts) => {
                let mut errors = Vec::new();
                for refusal in [clauses.err(), attempts.err()].into_iter().flatten() {
                    errors.extend(refusal.errors);
                }
                return refuse(errors);
            }
        };

        let mut package = PackageDeclarations::new(source);
        package.types = types;
        package.models = admitted;
        package.aliases = unit
            .aliases
            .iter()
            .zip(resolved)
            .filter_map(|(alias, value_type)| Some((alias.name.name.clone(), value_type?)))
            .collect();
        package.alias_names = selections
            .profiles
            .iter()
            .map(|profile| (profile.alias.clone(), AliasKind::Profile))
            .chain(
                selections
                    .models
                    .iter()
                    .map(|model| (model.alias.clone(), AliasKind::Model)),
            )
            .collect();
        package.native_names = unit
            .dimensions
            .iter()
            .map(|dimension| dimension.name.name.clone())
            .chain(unit.units.iter().map(|quantity| quantity.name.name.clone()))
            .collect();
        for (index, signature) in signatures.into_iter().enumerate() {
            package.resolved_signatures.insert(index, signature);
        }
        package.enums = enums;
        if let Some(quantities) = quantities {
            package.units = quantities.graph;
            package.nominal_spans = quantities.spans;
        }
        package.functions = unit.functions;
        package.function_selections = function_selections;
        package.state_clauses = state_clauses;
        package.protocols = unit.protocols;
        package.protocol_attempts = protocol_attempts;
        package.declared_type_spans = declared_type_spans;
        package.imports = qualified;
        Ok(package)
    }
}

/// FR-104 "Resolution": each state clause's `using` alias resolved to one
/// of the unit's profile selections, as a function's is (FR-091); its
/// context `M::T` to an object type of an admitted domain package; a `pre`
/// or `post` clause's operation to one of `M::T`'s effective view (FR-103);
/// and the population its extent names, which must be exactly one for its
/// context type and for the type declaring its operation. Each unresolved
/// name refuses at its span, the ambiguous population at the clause's
/// context.
fn state_clauses(
    forms: Vec<StateClauseForm>,
    profiles: &[qsl_foundation::selection::ProfileSelection],
    object_names: &BTreeMap<String, EffectiveId>,
    types: &TypeEnvironment,
    models: &[AdmittedModel],
) -> Result<Vec<StateClauseDeclaration>, AssemblyRefusal> {
    let mut errors = Vec::new();
    let mut clauses = Vec::with_capacity(forms.len());
    for form in forms {
        let error = |cause, span| AssemblyError { cause, span };
        let selection = profiles
            .iter()
            .find(|profile| profile.alias == form.profile.alias)
            .cloned();
        if selection.is_none() {
            errors.push(error(
                AssemblyCause::UndeclaredAlias {
                    alias: form.profile.alias.clone(),
                },
                form.profile.span,
            ));
        }
        let context = object_names.get(&form.context.name).copied();
        let Some(context) = context else {
            errors.push(error(
                AssemblyCause::UnresolvedTypeName {
                    name: form.context.name.clone(),
                },
                form.context.span,
            ));
            continue;
        };
        let operation = match &form.operation {
            None => None,
            Some(operation) => match types.operation(context, &operation.name) {
                OperationLookup::Declared {
                    declaring,
                    operation,
                } => Some(ClauseOperation {
                    declaring,
                    declaration: operation.clone(),
                }),
                OperationLookup::Missing => {
                    errors.push(error(
                        AssemblyCause::UnresolvedOperation {
                            context: form.context.name.clone(),
                            operation: operation.name.clone(),
                        },
                        operation.span,
                    ));
                    continue;
                }
                OperationLookup::Ambiguous(_) => {
                    errors.push(error(
                        AssemblyCause::AmbiguousOperation {
                            context: form.context.name.clone(),
                            operation: operation.name.clone(),
                        },
                        operation.span,
                    ));
                    continue;
                }
            },
        };
        let ambiguous = |populations| {
            error(
                AssemblyCause::AmbiguousPopulation {
                    context: form.context.name.clone(),
                    populations,
                },
                form.context.span,
            )
        };
        let context_population = match population_of(models, context, types) {
            Ok(domain) => domain,
            Err(populations) => {
                errors.push(ambiguous(populations));
                continue;
            }
        };
        // The type declaring the operation shares the context's population
        // when it is the context type itself; only a distinct declaring
        // type (an inherited operation) needs its own resolution.
        let frame_population = match &operation {
            None => None,
            Some(operation) if operation.declaring == context => context_population,
            Some(operation) => match population_of(models, operation.declaring, types) {
                Ok(domain) => domain,
                Err(populations) => {
                    errors.push(ambiguous(populations));
                    continue;
                }
            },
        };
        let Some(selection) = selection else {
            continue;
        };
        clauses.push(StateClauseDeclaration {
            kind: form.kind,
            name: form.name.name,
            selection,
            context,
            operation,
            context_population,
            frame_population,
            body: form.body,
            spans: form.spans,
        });
    }
    if errors.is_empty() {
        Ok(clauses)
    } else {
        refuse(errors)
    }
}

/// FR-114 "Behavior": each protocol's `attempt`s' operations, resolved
/// exactly as [`state_clauses`] resolves a `pre`/`post` clause's operation
/// (`types.operation`) and its frame population (`population_of`). The
/// `contracts` list is checked later, at S3 (`check::protocol_clause`),
/// against the unit's own checked state clauses -- unlike a state clause's
/// own operation, which the assembler alone resolves, an attempt's
/// `contracts` entries name *other declarations of the same unit* (FR-114
/// "Behavior"'s `missing_declaration`/`wrong_snapshot` refusals), so
/// checking them here would fail the whole package at E3 over one
/// protocol's own defect rather than refusing that protocol alone at S3,
/// the same way FR-113's own anchor and binder refusals do (QSL-309).
fn protocol_attempts(
    protocols: &[qsl_forms::ProtocolDeclarationForm],
    object_names: &BTreeMap<String, EffectiveId>,
    types: &TypeEnvironment,
    models: &[AdmittedModel],
) -> Result<Vec<Vec<AttemptDeclaration>>, AssemblyRefusal> {
    let mut errors = Vec::new();
    let mut resolved = Vec::with_capacity(protocols.len());
    for protocol in protocols {
        let mut attempts = Vec::with_capacity(protocol.attempts.len());
        for attempt in &protocol.attempts {
            let error = |cause, span| AssemblyError { cause, span };
            let context = object_names.get(&attempt.context.name).copied();
            let Some(context) = context else {
                errors.push(error(
                    AssemblyCause::UnresolvedTypeName {
                        name: attempt.context.name.clone(),
                    },
                    attempt.context.span,
                ));
                continue;
            };
            let operation = match types.operation(context, &attempt.operation.name) {
                OperationLookup::Declared {
                    declaring,
                    operation,
                } => ClauseOperation {
                    declaring,
                    declaration: operation.clone(),
                },
                OperationLookup::Missing => {
                    errors.push(error(
                        AssemblyCause::UnresolvedOperation {
                            context: attempt.context.name.clone(),
                            operation: attempt.operation.name.clone(),
                        },
                        attempt.operation.span,
                    ));
                    continue;
                }
                OperationLookup::Ambiguous(_) => {
                    errors.push(error(
                        AssemblyCause::AmbiguousOperation {
                            context: attempt.context.name.clone(),
                            operation: attempt.operation.name.clone(),
                        },
                        attempt.operation.span,
                    ));
                    continue;
                }
            };
            let frame_population = match population_of(models, operation.declaring, types) {
                Ok(domain) => domain,
                Err(populations) => {
                    errors.push(error(
                        AssemblyCause::AmbiguousPopulation {
                            context: attempt.context.name.clone(),
                            populations,
                        },
                        attempt.context.span,
                    ));
                    continue;
                }
            };
            attempts.push(AttemptDeclaration {
                declaration: attempt.declaration,
                operation,
                frame_population,
            });
        }
        resolved.push(attempts);
    }
    if errors.is_empty() {
        Ok(resolved)
    } else {
        refuse(errors)
    }
}

#[cfg(test)]
mod tests;

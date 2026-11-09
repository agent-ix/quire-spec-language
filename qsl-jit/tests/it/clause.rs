// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: a state clause is a clause entry; this backend translates none, so
//! it lists the selected clause unsupported with the first construct outside
//! the coverage, and the interpreter runs it.

use ix_trace_rs::trace;
use qsl_eval::execution::{
    EntryKind, EntrySelection, ExecutionBackend, PrepareLimits, PreparedEntries,
};
use qsl_jit::Jit;
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, RelationshipDirection, RelationshipEnd, RelationshipRecord, ScalarTypeRecord,
    ScalarValueDomain, ValueTypeRef,
};
use qsl_semantics::model::intake::SelectedModels;
use quire_checked::key::DeclarationKey;
use quire_eval::{UnsupportedCause, UntranslatableConstruct};
use quire_exact::{Cancel, Presence};
use quire_semantic_value::checking::CheckingLimits;

fn key(node: &str) -> DeclarationKey {
    DeclarationKey::fixture(format!("ix://test/orders/{node}"))
}

fn housing() -> DeclarationKey {
    key("relationship/Pump-housing-Tank")
}

fn one() -> Multiplicity {
    Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    }
}

fn object_type(name: &str) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: key(name),
        interface_features: None,
        abstract_type: false,
        supertypes: Vec::new(),
    })
}

fn orders_package() -> CheckedPackage {
    let end = |object: &str, role: &str| RelationshipEnd {
        type_identity: key(object),
        role: Some(role.to_owned()),
        multiplicity: Some(one()),
    };
    let records = vec![
        object_type("Pump"),
        object_type("Tank"),
        DomainPackageRecord::ScalarType(ScalarTypeRecord {
            key: key("Level"),
            domain: ScalarValueDomain::Integer {
                lower: 0,
                upper: 100,
            },
        }),
        DomainPackageRecord::FieldMember(FieldMemberRecord {
            key: key("Tank/level"),
            owner: key("Tank"),
            value_type: ValueTypeRef::Package(key("Level")),
            multiplicity: one(),
            presence: Presence::Required,
            subsets: Vec::new(),
            redefines: None,
            position: 0,
        }),
        DomainPackageRecord::Relationship(RelationshipRecord {
            key: housing(),
            declared_at: None,
            source: end("Pump", "home"),
            target: end("Tank", "pump"),
            direction: RelationshipDirection::Undirected,
        }),
    ];
    let package = DomainPackage::new(DomainPackageRef::fixture("bundle.orders"), records);
    let identity = package.model_selection.identity.clone();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {identity:?};\n\
         invariant TankLevel using v on M::Pump at current {{ self.home.level < 50 }}\n"
    );
    let selected = SelectedModels::fixture(
        "M",
        qsl_foundation::Span { start: 0, end: 1 },
        package,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-1040"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        built,
        selected,
        vec![],
    )
    .expect("the model assembles");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the orders model checks"),
    )
}

#[trace("TC-1381")]
#[test]
fn a_selected_state_clause_is_listed_unsupported_with_its_first_outside_construct() {
    let package = orders_package();
    let selection = EntrySelection {
        kind: EntryKind::Clause,
        name: crate::common::name("TankLevel"),
    };
    let prepared = Jit
        .prepare(
            &package,
            std::slice::from_ref(&selection),
            &PrepareLimits::default(),
            &Cancel::new(),
        )
        .unwrap_or_else(|failure| panic!("the JIT prepares: {failure:?}"));
    assert_eq!(prepared.unsupported().len(), 1);
    let (listed, entry) = &prepared.unsupported()[0];
    assert_eq!(*listed, selection);
    let UnsupportedCause::Untranslatable { construct, .. } = &entry.cause else {
        panic!("expected Untranslatable, got {:?}", entry.cause);
    };
    // The clause's `self` is a reference, a type outside the coverage.
    assert!(
        matches!(construct, UntranslatableConstruct::ValueType(_)),
        "{construct:?}"
    );
}

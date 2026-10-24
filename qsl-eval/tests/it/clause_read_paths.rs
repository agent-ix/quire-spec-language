// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1040 step 9 (FR-107-AC-17): a selection on the result of a relationship
//! navigation is located at the referenced object's declared identity with the
//! `member` step alone, and the navigation records no entry.
//!
//! The model is the TC-1033 `orders` shape: `Pump` and `Tank` joined by the
//! one-to-one `home`/`pump` relationship, with `Tank.level` an `Int[0, 100]`.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{
    CheckedPackageEvaluation, ObservationIdentity, QualifiedName, RuntimeValuePath, ValuePathStep,
    ValuePathSubject,
};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, RelationshipDirection, RelationshipEnd, RelationshipRecord, ScalarTypeRecord,
    ScalarValueDomain, ValueTypeRef,
};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    AdmittedObservations, DocumentRef, Observation, PopulationEnumerationFailure,
    RelationshipExtents, SelfKind,
};
use quire_exact::{
    EffectiveId, FieldValue, Integer, Meter, ObjectId, ObjectReference, Presence, UniverseId, Value,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::object_closure::ObjectClosure;
use quire_semantic_value::relationship_extent::{
    NavigationRelationship, PopulationEntry, PopulationEnumerator, RelationshipEnd as ExtentEnd,
    RelationshipExtent,
};

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
    let digest = qsl_semantics::model::key::hex(&package.model_selection.digest);
    let identity = package.model_selection.identity.clone();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {identity:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
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
        qsl_foundation::SourceIdentity::new("test", "tc-1040", "fixture", "1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations =
        PackageDeclarations::assemble(parsed.source().reference().clone(), built, selected, vec![])
            .expect("the model assembles");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the orders model checks"),
    )
}

struct Rows(std::vec::IntoIter<PopulationEntry<DeclarationKey>>);

impl PopulationEnumerator<DeclarationKey> for Rows {
    type Error = PopulationEnumerationFailure;

    fn next_row(
        &mut self,
    ) -> Result<Option<PopulationEntry<DeclarationKey>>, PopulationEnumerationFailure> {
        Ok(self.0.next())
    }
}

/// TC-1040 step 9: `self.home.level < 50` over the current snapshot `N`,
/// `self` `p1`, housed by `t1` at level 10.
#[trace("TC-1040", "FR-107-AC-17")]
#[test]
fn a_selection_on_a_navigation_is_located_at_the_referenced_object() {
    let package = orders_package();
    let types = package.graph().scope().types();
    let id = |name: &str| -> EffectiveId {
        types
            .object_types()
            .find(|declared| declared.name() == format!("M::{name}"))
            .unwrap_or_else(|| panic!("M::{name} is declared"))
            .key()
    };
    let at = |name: &str, object: &str| {
        ObjectReference::new(
            UniverseId::from_digest([9; 32]),
            id(name),
            ObjectId::new(object.to_owned()).unwrap(),
        )
    };
    let (p1, t1) = (at("Pump", "p1"), at("Tank", "t1"));
    let navigation = vec![NavigationRelationship {
        key: housing(),
        source: ExtentEnd {
            object_type: id("Pump"),
            unique: true,
        },
        target: ExtentEnd {
            object_type: id("Tank"),
            unique: true,
        },
    }];
    let rows = vec![PopulationEntry::try_new(key("Q"), [id("Pump"), id("Tank")], true).unwrap()];
    let extents = RelationshipExtents::admit(
        types,
        navigation,
        vec![RelationshipExtent {
            relationship: housing(),
            complete: true,
            instances: vec![(p1.clone(), t1.clone())],
        }],
        &mut Rows(rows.into_iter()),
        1,
    )
    .expect("the extents admit");
    let closure = ObjectClosure::new(
        types,
        [
            (p1.clone(), vec![]),
            (
                t1.clone(),
                vec![(
                    "level",
                    FieldValue::Present(Value::Integer(Integer::from(10_i64))),
                )],
            ),
        ],
        &[],
    )
    .expect("the objects close");
    let current = Observation {
        identity: DocumentRef {
            authority: "test".to_owned(),
            identity: "N".to_owned(),
            revision_namespace: "tc1040".to_owned(),
            revision: "1".to_owned(),
            digest: [0; 32],
        },
        environment: ObjectEnvironment::new(closure).with_extents(extents),
        populations: BTreeMap::new(),
        out_of_range: Vec::new(),
    };
    let observations = AdmittedObservations {
        usage: Default::default(),
        clause: package
            .graph()
            .state_clause("TankLevel")
            .expect("the clause is declared")
            .identity(),
        current: Some(current),
        pre: None,
        post: None,
        self_object: p1,
        invocation: None,
        self_kind: SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: Vec::new(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    };
    let evaluation = package
        .evaluate_clause(
            &QualifiedName::unqualified("TankLevel").unwrap(),
            &observations,
            &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
        )
        .expect("the clause evaluates");
    assert!(matches!(
        evaluation.evaluation.outcome,
        quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(true)))
    ));
    let reads: Vec<_> = evaluation.reads().iter().map(|read| &read.path).collect();
    assert_eq!(
        reads,
        vec![&Some(RuntimeValuePath {
            observation: ObservationIdentity {
                authority: "test".to_owned(),
                identity: "N".to_owned(),
                revision_namespace: "tc1040".to_owned(),
                revision: "1".to_owned(),
            },
            subject: ValuePathSubject::Object(t1),
            steps: vec![ValuePathStep::Member("level".to_owned())],
        })],
        "one entry, `self.home.level`; the navigation `self.home` records none"
    );
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-364 at the model layer: extents supplied as full references are
//! admitted under four conditions and read in reference-key order (TC-1033),
//! the population manifest and `resolve` decide a missing object (TC-1041
//! rows 1 to 17), and a bundle binds the manifest, the closure and the type
//! environment to one key (TC-1041 row 20). Expected values come from the
//! tables of those test cases.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_semantics::check::CheckedGraph;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, ObjectTypeRecord,
};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::{
    AdmissionFailure, AdmissionRecord, BundleKey, BundlePart, Keyed, ObservationOrigin,
    ObservationRole, PopulationEnumerationFailure, RelationshipExtents, ResolveBundle,
};
use quire_exact::{EffectiveId, NodeKey, ObjectId, ObjectReference, UniverseId};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::object_closure::ObjectClosure;
use quire_semantic_value::relationship_extent::{
    ExtentState, ManifestState, NavigationRelationship, PopulationEntry, PopulationEnumerator,
    RelationshipEnd, RelationshipExtent, Resolution, StopReason,
};

fn key(node: &str) -> DeclarationKey {
    DeclarationKey::fixture(format!("ix://test/orders/{node}"))
}

fn object_type(name: &str, supertypes: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: key(name),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes.iter().copied().map(key).collect(),
    })
}

/// Fixture **orders**: `Sys`, `Pump`, `Tank`, with the **valve** type `Valve`
/// and the **parts** supertype `Part` of `Pump` and `Tank` declared beside
/// them (a variant only adds relationships and populations, which the tests
/// below supply directly).
struct Orders {
    graph: CheckedGraph,
}

impl Orders {
    fn new() -> Self {
        let records = vec![
            object_type("Sys", &[]),
            object_type("Part", &[]),
            object_type("Pump", &["Part"]),
            object_type("Tank", &["Part"]),
            object_type("Valve", &[]),
        ];
        let package = DomainPackage::new(DomainPackageRef::fixture("bundle.orders"), records);
        let graph = crate::relationship_navigation::check_invariant(package, "Sys", "true")
            .expect("the orders model checks");
        Self { graph }
    }

    fn types(&self) -> &TypeEnvironment {
        self.graph.scope().types()
    }

    fn id(&self, name: &str) -> EffectiveId {
        crate::relationship_navigation::object(&self.graph, name)
    }

    fn reference(&self, universe: u8, type_name: &str, object: &str) -> ObjectReference {
        ObjectReference::new(
            UniverseId::from_digest([universe; 32]),
            self.id(type_name),
            ObjectId::new(object.to_owned()).expect("a nonempty object id"),
        )
    }

    /// The reference of the object `object` of `type_name` in the one
    /// universe of the model.
    fn at(&self, type_name: &str, object: &str) -> ObjectReference {
        self.reference(9, type_name, object)
    }

    fn end(&self, type_name: &str, unique: bool) -> RelationshipEnd {
        RelationshipEnd {
            object_type: self.id(type_name),
            unique,
        }
    }

    /// `Sys-composition-Pump` and `Pump-housing-Tank`, with the given
    /// uniqueness of the composition's two ends; **valve** and **parts**
    /// relationships are always declared.
    fn navigation(
        &self,
        composition_unique: (bool, bool),
    ) -> Vec<NavigationRelationship<DeclarationKey>> {
        vec![
            NavigationRelationship {
                key: composition(),
                source: self.end("Sys", composition_unique.0),
                target: self.end("Pump", composition_unique.1),
            },
            NavigationRelationship {
                key: housing(),
                source: self.end("Pump", true),
                target: self.end("Tank", true),
            },
            NavigationRelationship {
                key: feeds(),
                source: self.end("Pump", true),
                target: self.end("Valve", true),
            },
            NavigationRelationship {
                key: holds(),
                source: self.end("Sys", true),
                target: self.end("Part", true),
            },
        ]
    }

    fn row(
        &self,
        population: &str,
        covered: &[&str],
        complete: bool,
    ) -> PopulationEntry<DeclarationKey> {
        PopulationEntry::try_new(
            key(population),
            covered.iter().map(|name| self.id(name)),
            complete,
        )
        .expect("a nonempty population row")
    }
}

fn composition() -> DeclarationKey {
    key("relationship/Sys-composition-Pump")
}
fn housing() -> DeclarationKey {
    key("relationship/Pump-housing-Tank")
}
fn feeds() -> DeclarationKey {
    key("relationship/Pump-feeds-Valve")
}
fn holds() -> DeclarationKey {
    key("relationship/Sys-holds-Part")
}

fn extent(
    relationship: DeclarationKey,
    complete: bool,
    instances: Vec<(ObjectReference, ObjectReference)>,
) -> RelationshipExtent<DeclarationKey> {
    RelationshipExtent {
        relationship,
        complete,
        instances,
    }
}

/// The enumerator the model layer passes: the rows in order, counting pulls,
/// failing at pull `fail_at` when set.
struct Rows {
    rows: std::vec::IntoIter<PopulationEntry<DeclarationKey>>,
    pulls: usize,
    fail_at: Option<usize>,
    failing: DeclarationKey,
}

impl Rows {
    fn new(rows: Vec<PopulationEntry<DeclarationKey>>) -> Self {
        Self {
            rows: rows.into_iter(),
            pulls: 0,
            fail_at: None,
            failing: key("Spare"),
        }
    }

    fn failing_at(mut self, pull: usize) -> Self {
        self.fail_at = Some(pull);
        self
    }
}

impl PopulationEnumerator<DeclarationKey> for Rows {
    type Error = PopulationEnumerationFailure;

    fn next_row(
        &mut self,
    ) -> Result<Option<PopulationEntry<DeclarationKey>>, PopulationEnumerationFailure> {
        self.pulls += 1;
        if self.fail_at == Some(self.pulls) {
            return Err(PopulationEnumerationFailure {
                declaration: self.failing.clone(),
            });
        }
        Ok(self.rows.next())
    }
}

fn refused(failure: AdmissionFailure) -> AdmissionRecord {
    match failure {
        AdmissionFailure::Refused(record) => record,
        other => panic!("a refusal, not {other:?}"),
    }
}

fn admit(
    orders: &Orders,
    unique: (bool, bool),
    extents: Vec<RelationshipExtent<DeclarationKey>>,
) -> Result<RelationshipExtents, AdmissionFailure> {
    let mut rows = Rows::new(vec![orders.row("Q", &["Sys", "Pump", "Tank"], true)]);
    RelationshipExtents::admit(
        orders.types(),
        orders.navigation(unique),
        extents,
        &mut rows,
        1,
    )
}

fn keys(references: Vec<&ObjectReference>) -> Vec<ObjectReference> {
    references.into_iter().cloned().collect()
}

/// TC-1033 step 1: the same extents built in either order give the same
/// reads, in ascending reference-key order.
#[trace("TC-1033", "FR-364-AC-3")]
#[test]
fn extents_read_in_reference_key_order_whatever_the_build_order() {
    let orders = Orders::new();
    let (s1, p1, p2, t1) = (
        orders.at("Sys", "s1"),
        orders.at("Pump", "p1"),
        orders.at("Pump", "p2"),
        orders.at("Tank", "t1"),
    );
    let forward = vec![
        extent(
            composition(),
            true,
            vec![(s1.clone(), p2.clone()), (s1.clone(), p1.clone())],
        ),
        extent(
            housing(),
            true,
            vec![(p2.clone(), t1.clone()), (p1.clone(), t1.clone())],
        ),
    ];
    let reverse: Vec<_> = forward
        .iter()
        .rev()
        .map(|held| {
            extent(
                held.relationship.clone(),
                held.complete,
                held.instances.iter().rev().cloned().collect(),
            )
        })
        .collect();
    for supplied in [forward, reverse] {
        let extents = admit(&orders, (true, true), supplied).expect("step 1 admits");
        assert!(extents.complete(&composition()));
        assert!(extents.complete(&housing()));
        assert_eq!(
            keys(extents.targets(&composition(), &s1).collect()),
            vec![p1.clone(), p2.clone()]
        );
        assert_eq!(
            keys(extents.sources(&composition(), &p1).collect()),
            vec![s1.clone()]
        );
        assert_eq!(
            keys(extents.targets(&housing(), &p1).collect()),
            vec![t1.clone()]
        );
        assert_eq!(
            keys(extents.sources(&housing(), &t1).collect()),
            vec![p1.clone(), p2.clone()]
        );
    }
}

/// TC-1033 step 2: an extent for a declaration that is no navigation
/// relationship, and for an unknown one.
#[trace("TC-1033", "FR-364-AC-1")]
#[test]
fn an_extent_of_a_non_navigation_relationship_is_refused_wrong_role_mapping() {
    let orders = Orders::new();
    for name in ["pipe", "relationship/Nope-x-Pump"] {
        let refusal = refused(
            admit(&orders, (true, true), vec![extent(key(name), true, vec![])])
                .expect_err("step 2 refuses"),
        );
        assert_eq!(refusal.code, Code::InvalidRuntimeInput);
        assert_eq!(refusal.cause, "wrong-role-mapping");
        assert_eq!(refusal.fields.get("relationship"), Some(&key(name).node));
    }
}

/// TC-1033 step 3: two extents for one relationship.
#[trace("TC-1033", "FR-364-AC-1")]
#[test]
fn a_second_extent_of_one_relationship_is_refused_conflicting_identity() {
    let orders = Orders::new();
    let refusal = refused(
        admit(
            &orders,
            (true, true),
            vec![
                extent(composition(), true, vec![]),
                extent(composition(), true, vec![]),
            ],
        )
        .expect_err("step 3 refuses"),
    );
    assert_eq!(refusal.code, Code::InvalidRuntimeInput);
    assert_eq!(refusal.cause, "conflicting-identity");
    assert_eq!(
        refusal.fields.get("relationship"),
        Some(&composition().node)
    );
}

/// TC-1033 step 4: a `Tank` at the source end, then at the target end.
#[trace("TC-1033", "FR-364-AC-1")]
#[test]
fn an_end_object_of_a_type_that_does_not_conform_is_refused_wrong_value_kind() {
    let orders = Orders::new();
    let (s1, p1, t1) = (
        orders.at("Sys", "s1"),
        orders.at("Pump", "p1"),
        orders.at("Tank", "t1"),
    );
    for (instance, end, declared) in [
        ((t1.clone(), p1.clone()), "source", "Sys"),
        ((s1.clone(), t1.clone()), "target", "Pump"),
    ] {
        let refusal = refused(
            admit(
                &orders,
                (true, true),
                vec![extent(composition(), true, vec![instance])],
            )
            .expect_err("step 4 refuses"),
        );
        assert_eq!(refusal.code, Code::InvalidRuntimeInput);
        assert_eq!(refusal.cause, "wrong-value-kind");
        assert_eq!(refusal.fields.get("end").map(String::as_str), Some(end));
        assert_eq!(
            refusal.fields.get("instance").map(String::as_str),
            Some("0")
        );
        assert_eq!(
            refusal.fields.get("type"),
            Some(
                &orders
                    .types()
                    .object_type(orders.id("Tank"))
                    .expect("Tank")
                    .name()
                    .to_owned()
            )
        );
        assert_eq!(
            refusal.fields.get("declared"),
            Some(
                &orders
                    .types()
                    .object_type(orders.id(declared))
                    .expect(declared)
                    .name()
                    .to_owned()
            )
        );
    }
}

/// TC-1033 steps 5 and 6: a repeated instance refuses at the later instance
/// unless both ends declare `unique: false`.
#[trace("TC-1033", "FR-364-AC-1", "FR-364-AC-3")]
#[test]
fn a_repeated_instance_admits_only_when_both_ends_are_not_unique() {
    let orders = Orders::new();
    let (s1, p1) = (orders.at("Sys", "s1"), orders.at("Pump", "p1"));
    let repeated = || {
        vec![extent(
            composition(),
            true,
            vec![(s1.clone(), p1.clone()), (s1.clone(), p1.clone())],
        )]
    };
    for unique in [(true, true), (true, false)] {
        let refusal = refused(admit(&orders, unique, repeated()).expect_err("step 5 refuses"));
        assert_eq!(refusal.code, Code::InvalidRuntimeInput);
        assert_eq!(refusal.cause, "conflicting-identity");
        assert_eq!(
            refusal.fields.get("instance").map(String::as_str),
            Some("1")
        );
    }
    let extents = admit(&orders, (false, false), repeated()).expect("both ends non-unique admit");
    assert_eq!(
        keys(extents.targets(&composition(), &s1).collect()),
        vec![p1.clone(), p1.clone()]
    );
    assert_eq!(
        keys(extents.sources(&composition(), &p1).collect()),
        vec![s1.clone(), s1.clone()]
    );
}

/// TC-1033 step 7: no multiplicity bound is compared at admission.
#[trace("TC-1033", "FR-364-AC-2")]
#[test]
fn admission_compares_no_count_with_a_bound() {
    let orders = Orders::new();
    let s1 = orders.at("Sys", "s1");
    let pumps: Vec<_> = (1..=4)
        .map(|n| orders.at("Pump", &format!("p{n}")))
        .collect();
    let extents = admit(
        &orders,
        (true, true),
        vec![
            extent(
                composition(),
                true,
                pumps
                    .iter()
                    .map(|pump| (s1.clone(), pump.clone()))
                    .collect(),
            ),
            extent(housing(), true, vec![]),
        ],
    )
    .expect("four targets through {0,3}, and no housing for p3, admit");
    assert_eq!(keys(extents.targets(&composition(), &s1).collect()), pumps);
    admit(&orders, (true, true), vec![]).expect("an observation with no extent admits");
}

/// TC-1033 step 8: unlisted, incomplete and complete-empty are three states.
#[trace("TC-1033", "FR-364-AC-4")]
#[test]
fn unlisted_incomplete_and_empty_extents_are_three_states() {
    let orders = Orders::new();
    let (s1, p1) = (orders.at("Sys", "s1"), orders.at("Pump", "p1"));
    let extents = admit(
        &orders,
        (true, true),
        vec![extent(composition(), true, vec![])],
    )
    .expect("a complete empty extent admits");
    assert!(extents.complete(&composition()));
    assert_eq!(extents.state(&composition()), ExtentState::Complete);
    assert_eq!(extents.targets(&composition(), &s1).count(), 0);
    assert!(!extents.complete(&housing()));
    assert_eq!(extents.state(&housing()), ExtentState::Unlisted);

    let extents = admit(
        &orders,
        (true, true),
        vec![extent(composition(), false, vec![(s1.clone(), p1.clone())])],
    )
    .expect("an incomplete extent admits");
    assert!(!extents.complete(&composition()));
    assert_eq!(extents.state(&composition()), ExtentState::Incomplete);
    assert_eq!(
        keys(extents.targets(&composition(), &s1).collect()),
        vec![p1]
    );
}

/// TC-1033 step 9: equal object identity in two universes is two references.
#[trace("TC-1033", "FR-364-AC-3")]
#[test]
fn references_with_equal_identity_in_different_universes_are_not_equal() {
    let orders = Orders::new();
    let (p1, t1) = (
        orders.reference(9, "Pump", "p1"),
        orders.reference(9, "Tank", "t1"),
    );
    let tank_p1 = orders.reference(8, "Tank", "p1");
    let extents = admit(
        &orders,
        (true, true),
        vec![extent(housing(), true, vec![(p1.clone(), t1.clone())])],
    )
    .expect("housing admits");
    assert_ne!(p1, tank_p1);
    assert_eq!(extents.sources(&housing(), &tank_p1).count(), 0);
    assert_eq!(keys(extents.targets(&housing(), &p1).collect()), vec![t1]);
}

/// TC-1033 step 10: reads are pure and the observation is unchanged by them.
#[trace("TC-1033", "FR-364-AC-4")]
#[test]
fn reading_leaves_the_extents_equal_to_what_they_were() {
    let orders = Orders::new();
    let (s1, p1, p2) = (
        orders.at("Sys", "s1"),
        orders.at("Pump", "p1"),
        orders.at("Pump", "p2"),
    );
    let extents = admit(
        &orders,
        (true, true),
        vec![extent(
            composition(),
            true,
            vec![(s1.clone(), p2), (s1.clone(), p1)],
        )],
    )
    .expect("composition admits");
    let before = extents.clone();
    let first = keys(extents.targets(&composition(), &s1).collect());
    let second = keys(extents.targets(&composition(), &s1).collect());
    assert_eq!(first, second);
    for relationship in [composition(), housing()] {
        assert_eq!(extents.state(&relationship), before.state(&relationship));
        assert_eq!(
            keys(extents.targets(&relationship, &s1).collect()),
            keys(before.targets(&relationship, &s1).collect())
        );
    }
}

fn manifest_rows(
    extents: &RelationshipExtents,
    orders: &Orders,
) -> (ManifestState, Vec<(DeclarationKey, Vec<EffectiveId>, bool)>) {
    let manifest = extents.manifest(orders.types()).expect("a manifest");
    (
        manifest.state(),
        manifest
            .rows()
            .map(|row| {
                (
                    row.population().clone(),
                    row.covered_types().to_vec(),
                    row.complete(),
                )
            })
            .collect(),
    )
}

fn with_rows(
    orders: &Orders,
    rows: Vec<PopulationEntry<DeclarationKey>>,
    extents: Vec<RelationshipExtent<DeclarationKey>>,
) -> RelationshipExtents {
    let limit = rows.len();
    RelationshipExtents::admit(
        orders.types(),
        orders.navigation((true, true)),
        extents,
        &mut Rows::new(rows),
        limit,
    )
    .expect("the observation admits")
}

/// TC-1041 rows 1 and 2: one row per declared population, in ascending
/// declaration-key order, `complete` exactly when bound.
#[trace("TC-1041", "FR-364-AC-10")]
#[test]
fn the_manifest_has_one_row_per_declared_population_in_key_order() {
    let orders = Orders::new();
    let q = orders.row("Q", &["Sys", "Pump", "Tank"], true);
    let spare = orders.row("Spare", &["Pump"], false);
    let spare_bound = orders.row("Spare", &["Pump"], true);

    let orders_only = with_rows(&orders, vec![q.clone()], vec![]);
    let (state, rows) = manifest_rows(&orders_only, &orders);
    assert_eq!(state, ManifestState::Closed);
    assert_eq!(
        rows,
        vec![(
            key("Q"),
            vec![orders.id("Sys"), orders.id("Pump"), orders.id("Tank")],
            true
        )]
    );

    // The rows are supplied out of order; the manifest sorts them.
    let (state, rows) = manifest_rows(
        &with_rows(&orders, vec![spare.clone(), q.clone()], vec![]),
        &orders,
    );
    assert_eq!(state, ManifestState::Closed);
    assert_eq!(
        rows.iter()
            .map(|(population, _, complete)| (population.clone(), *complete))
            .collect::<Vec<_>>(),
        vec![(key("Q"), true), (key("Spare"), false)]
    );
    assert_eq!(rows[1].1, vec![orders.id("Pump")]);

    let (_, rows) = manifest_rows(&with_rows(&orders, vec![q, spare_bound], vec![]), &orders);
    assert!(rows.iter().all(|(_, _, complete)| *complete));
}

/// TC-1041 row 3: an enumerator exhausted exactly at its row limit closes.
#[trace("TC-1041", "FR-364-AC-10")]
#[test]
fn an_enumerator_exhausted_exactly_at_its_row_limit_closes_the_manifest() {
    let orders = Orders::new();
    let mut rows = Rows::new(vec![
        orders.row("Q", &["Sys", "Pump", "Tank"], true),
        orders.row("Spare", &["Pump"], true),
    ]);
    let extents = RelationshipExtents::admit(
        orders.types(),
        orders.navigation((true, true)),
        vec![],
        &mut rows,
        2,
    )
    .expect("admits");
    // Two rows, then the pull that reports exhaustion.
    assert_eq!(rows.pulls, 3);
    assert_eq!(manifest_rows(&extents, &orders).0, ManifestState::Closed);
}

/// TC-1041 row 4: an enumerator that fails leaves a partial manifest, with
/// the rows read and the failure as stop reason.
#[trace("TC-1041", "FR-364-AC-10")]
#[test]
fn an_enumerator_failure_leaves_a_partial_manifest() {
    let orders = Orders::new();
    let mut rows = Rows::new(vec![
        orders.row("Q", &["Sys", "Pump", "Tank"], true),
        orders.row("Spare", &["Pump"], true),
    ])
    .failing_at(2);
    let extents = RelationshipExtents::admit(
        orders.types(),
        orders.navigation((true, true)),
        vec![],
        &mut rows,
        2,
    )
    .expect("admits");
    let manifest = extents.manifest(orders.types()).expect("a manifest");
    assert_eq!(manifest.state(), ManifestState::Partial);
    assert_eq!(manifest.rows().count(), 1);
    assert_eq!(
        manifest.stop(),
        Some(&StopReason::EnumeratorError(PopulationEnumerationFailure {
            declaration: key("Spare")
        }))
    );
}

/// The closure of the observation: the objects it holds, no fields.
fn closure(orders: &Orders, held: &[ObjectReference]) -> ObjectClosure {
    ObjectClosure::new(
        orders.types(),
        held.iter().cloned().map(|object| (object, vec![])),
        &[],
    )
    .expect("a closure of fieldless objects")
}

fn resolve(
    orders: &Orders,
    extents: &RelationshipExtents,
    held: &[ObjectReference],
    reference: &ObjectReference,
) -> Resolution {
    let bundle_key = bundle_key("pre");
    let closure = closure(orders, held);
    let bundle = ResolveBundle::new(
        &bundle_key,
        Keyed {
            key: bundle_key.clone(),
            part: extents.manifest(orders.types()).expect("a manifest"),
        },
        Keyed {
            key: bundle_key.clone(),
            part: &closure,
        },
        Keyed {
            key: bundle_key.clone(),
            part: orders.types(),
        },
    )
    .expect("one key");
    bundle.resolve(reference)
}

fn held(orders: &Orders) -> Vec<ObjectReference> {
    vec![
        orders.at("Sys", "s1"),
        orders.at("Pump", "p1"),
        orders.at("Pump", "p2"),
        orders.at("Pump", "p3"),
        orders.at("Tank", "t1"),
    ]
}

fn q(orders: &Orders) -> PopulationEntry<DeclarationKey> {
    orders.row("Q", &["Sys", "Pump", "Tank"], true)
}

/// TC-1041 rows 5 to 8: dangling only from a closed manifest whose covering
/// rows are all complete.
#[trace("TC-1041", "FR-364-AC-11")]
#[test]
fn resolve_is_dangling_only_when_every_covering_row_is_complete() {
    let orders = Orders::new();
    let (p1, p9, t9) = (
        orders.at("Pump", "p1"),
        orders.at("Pump", "p9"),
        orders.at("Tank", "t9"),
    );
    let held = held(&orders);

    let plain = with_rows(&orders, vec![q(&orders)], vec![]);
    assert_eq!(resolve(&orders, &plain, &held, &p9), Resolution::Dangling);
    assert_eq!(resolve(&orders, &plain, &held, &p1), Resolution::Present);

    let spare = with_rows(
        &orders,
        vec![q(&orders), orders.row("Spare", &["Pump"], false)],
        vec![],
    );
    assert_eq!(
        resolve(&orders, &spare, &held, &p9),
        Resolution::Undecidable
    );
    assert_eq!(resolve(&orders, &spare, &held, &t9), Resolution::Dangling);

    let bound = with_rows(
        &orders,
        vec![q(&orders), orders.row("Spare", &["Pump"], true)],
        vec![],
    );
    assert_eq!(resolve(&orders, &bound, &held, &p9), Resolution::Dangling);
}

/// TC-1041 row 6: under a partial manifest a held object is present and a
/// missing one undecidable.
#[trace("TC-1041", "FR-364-AC-11")]
#[test]
fn a_partial_manifest_makes_every_missing_object_undecidable() {
    let orders = Orders::new();
    let mut rows = Rows::new(vec![q(&orders), orders.row("Spare", &["Pump"], true)]).failing_at(2);
    let partial = RelationshipExtents::admit(
        orders.types(),
        orders.navigation((true, true)),
        vec![],
        &mut rows,
        2,
    )
    .expect("admits");
    let held = held(&orders);
    assert_eq!(
        resolve(&orders, &partial, &held, &orders.at("Pump", "p1")),
        Resolution::Present
    );
    assert_eq!(
        resolve(&orders, &partial, &held, &orders.at("Pump", "p9")),
        Resolution::Undecidable
    );
}

/// TC-1041 rows 9 to 11: a type no row covers, a parent of the declared
/// member types, and a type that is no object type are all undecidable.
#[trace("TC-1041", "FR-364-AC-11")]
#[test]
fn a_type_no_row_covers_is_undecidable_never_vacuously_dangling() {
    let orders = Orders::new();
    let held = held(&orders);
    let plain = with_rows(&orders, vec![q(&orders)], vec![]);
    // Row 9 (valve): no declared population names Valve.
    assert_eq!(
        resolve(&orders, &plain, &held, &orders.at("Valve", "v1")),
        Resolution::Undecidable
    );
    // Row 10 (parts): Q declares Pump and Tank, so it does not cover their
    // parent Part; a missing Pump is dangling.
    let parts = with_rows(
        &orders,
        vec![orders.row("Q", &["Sys", "Pump", "Tank"], true)],
        vec![],
    );
    assert_eq!(
        resolve(&orders, &parts, &held, &orders.at("Part", "x1")),
        Resolution::Undecidable
    );
    assert_eq!(
        resolve(&orders, &parts, &held, &orders.at("Pump", "p9")),
        Resolution::Dangling
    );
    // Row 11: a reference whose type is no object type of the model.
    #[allow(
        clippy::disallowed_methods,
        reason = "fixture mints an identity the model does not declare"
    )]
    let unknown = EffectiveId::from_digest([0xee; 32]);
    let stranger = ObjectReference::new(
        UniverseId::from_digest([9; 32]),
        unknown,
        ObjectId::new("v1".to_owned()).expect("an id"),
    );
    assert_eq!(
        resolve(&orders, &plain, &held, &stranger),
        Resolution::Undecidable
    );
}

/// TC-1041 rows 12 to 14 and 16: what the evaluator reads at the boundary.
/// The first selected target that is not present decides, and the kind of
/// outcome is `resolve`'s (the charges are the evaluator's, QSL-795).
#[trace("TC-1041")]
#[test]
fn the_first_selected_target_that_is_not_present_decides_the_outcome() {
    let orders = Orders::new();
    let held = held(&orders);
    let s1 = orders.at("Sys", "s1");
    let first_missing =
        |extents: &RelationshipExtents, relationship: &DeclarationKey, source: &ObjectReference| {
            extents
                .targets(relationship, source)
                .map(|target| (target.clone(), resolve(&orders, extents, &held, target)))
                .find(|(_, resolution)| *resolution != Resolution::Present)
        };

    // Row 12: orders; composition s1-p9 is dangling.
    let p9 = orders.at("Pump", "p9");
    let plain = with_rows(
        &orders,
        vec![q(&orders)],
        vec![extent(composition(), true, vec![(s1.clone(), p9.clone())])],
    );
    assert_eq!(
        first_missing(&plain, &composition(), &s1),
        Some((p9.clone(), Resolution::Dangling))
    );

    // Row 13: spare; the same extent is undecidable.
    let spare = with_rows(
        &orders,
        vec![q(&orders), orders.row("Spare", &["Pump"], false)],
        vec![extent(composition(), true, vec![(s1.clone(), p9.clone())])],
    );
    assert_eq!(
        first_missing(&spare, &composition(), &s1),
        Some((p9.clone(), Resolution::Undecidable))
    );

    // Row 14: three present targets precede the missing one.
    let targets: Vec<_> = ["p1", "p2", "p3"]
        .iter()
        .map(|id| orders.at("Pump", id))
        .collect();
    let mut instances: Vec<_> = targets
        .iter()
        .map(|pump| (s1.clone(), pump.clone()))
        .collect();
    instances.push((s1.clone(), p9.clone()));
    let four = with_rows(
        &orders,
        vec![q(&orders), orders.row("Spare", &["Pump"], false)],
        vec![extent(composition(), true, instances)],
    );
    let selected: Vec<_> = four.targets(&composition(), &s1).cloned().collect();
    assert_eq!(selected.len(), 4);
    assert_eq!(
        selected
            .iter()
            .take_while(|target| resolve(&orders, &four, &held, target) == Resolution::Present)
            .count(),
        3
    );
    assert_eq!(
        first_missing(&four, &composition(), &s1),
        Some((p9, Resolution::Undecidable))
    );

    // Row 16: valve; Pump-feeds-Valve p1-v1 is undecidable.
    let (p1, v1) = (orders.at("Pump", "p1"), orders.at("Valve", "v1"));
    let valve = with_rows(
        &orders,
        vec![q(&orders)],
        vec![extent(feeds(), true, vec![(p1.clone(), v1.clone())])],
    );
    assert_eq!(
        first_missing(&valve, &feeds(), &p1),
        Some((v1, Resolution::Undecidable))
    );
}

/// TC-1041 row 15: of `p9` and `t9` the one that sorts first decides; across
/// the variants **spare** and **spare-tank** both outcomes occur.
#[trace("TC-1041")]
#[test]
fn the_key_order_decides_between_dangling_and_undecidable_targets() {
    let orders = Orders::new();
    let held = held(&orders);
    let s1 = orders.at("Sys", "s1");
    let (p9, t9) = (orders.at("Pump", "p9"), orders.at("Tank", "t9"));
    let mut outcomes = Vec::new();
    for incomplete_member in ["Pump", "Tank"] {
        let extents = with_rows(
            &orders,
            vec![q(&orders), orders.row("Spare", &[incomplete_member], false)],
            vec![extent(
                holds(),
                true,
                vec![(s1.clone(), p9.clone()), (s1.clone(), t9.clone())],
            )],
        );
        let selected: Vec<_> = extents.targets(&holds(), &s1).cloned().collect();
        let mut expected = [&p9, &t9];
        expected.sort();
        assert_eq!(selected, vec![expected[0].clone(), expected[1].clone()]);
        let first = &selected[0];
        let resolution = resolve(&orders, &extents, &held, first);
        // The incomplete row covers one type: that type's reference is
        // undecidable and the other dangling.
        let undecidable_type = orders.id(incomplete_member);
        assert_eq!(
            resolution,
            if first.object_type() == undecidable_type {
                Resolution::Undecidable
            } else {
                Resolution::Dangling
            }
        );
        outcomes.push(resolution);
    }
    assert!(outcomes.contains(&Resolution::Dangling));
    assert!(outcomes.contains(&Resolution::Undecidable));
}

/// TC-1041 row 17: condition 3 checks the type of a full-reference end
/// whether or not its object is held.
#[trace("TC-1041", "FR-364-AC-13")]
#[test]
fn condition_three_checks_the_type_of_an_end_whose_object_is_absent() {
    let orders = Orders::new();
    let s1 = orders.at("Sys", "s1");
    let wrong = |target: ObjectReference| {
        admit(
            &orders,
            (true, true),
            vec![extent(composition(), true, vec![(s1.clone(), target)])],
        )
    };
    // t9 is not held and is a Tank at the Pump end.
    let refusal = refused(wrong(orders.at("Tank", "t9")).expect_err("a wrong type, absent"));
    assert_eq!(refusal.code, Code::InvalidRuntimeInput);
    assert_eq!(refusal.cause, "wrong-value-kind");
    assert_eq!(
        refusal.fields.get("end").map(String::as_str),
        Some("target")
    );
    // p9 is not held and conforms: admitted, and resolve decides it.
    wrong(orders.at("Pump", "p9")).expect("a conforming absent end admits");
    // t1 is held and still a Tank at the Pump end.
    let refusal = refused(wrong(orders.at("Tank", "t1")).expect_err("a wrong type, held"));
    assert_eq!(refusal.cause, "wrong-value-kind");
    // A reference whose type is no object type of the model.
    #[allow(
        clippy::disallowed_methods,
        reason = "fixture mints an identity the model does not declare"
    )]
    let unknown = EffectiveId::from_digest([0xee; 32]);
    let stranger = ObjectReference::new(
        UniverseId::from_digest([9; 32]),
        unknown,
        ObjectId::new("x".to_owned()).expect("an id"),
    );
    let refusal = refused(wrong(stranger).expect_err("an unknown type"));
    assert_eq!(refusal.cause, "wrong-value-kind");
}

fn bundle_key(label: &str) -> BundleKey {
    BundleKey {
        source: source("s", 1),
        clause: NodeKey::from_digest([3; 32]),
        documents: vec![],
        role: ObservationRole::Pre,
        origin: ObservationOrigin::Label(label.to_owned()),
    }
}

fn source(identity: &str, digest: u8) -> RawSourceRef {
    RawSourceRef::new(
        "authority",
        identity,
        DigestRecord::mint(DigestDomain::SourceBytesV1, [digest; 32]),
    )
    .expect("a source reference")
}

/// TC-1041 row 20: the first part whose key differs from the selected key
/// is refused, naming the part and the two keys; equal keys resolve.
#[trace("TC-1041", "FR-364-AC-14")]
#[test]
fn a_part_with_another_key_is_refused_before_it_can_classify_anything() {
    let orders = Orders::new();
    let bound = with_rows(
        &orders,
        vec![q(&orders), orders.row("Spare", &["Pump"], true)],
        vec![],
    );
    let held = held(&orders);
    let closure = closure(&orders, &held);
    let selected = bundle_key("pre");

    let build = |manifest_key: &BundleKey, closure_key: &BundleKey, types_key: &BundleKey| {
        ResolveBundle::new(
            &selected,
            Keyed {
                key: manifest_key.clone(),
                part: bound.manifest(orders.types()).expect("a manifest"),
            },
            Keyed {
                key: closure_key.clone(),
                part: &closure,
            },
            Keyed {
                key: types_key.clone(),
                part: orders.types(),
            },
        )
    };

    // Another source digest under the same authority and identity.
    let other_source = BundleKey {
        source: source("s", 2),
        ..selected.clone()
    };
    let refusal = build(&selected, &selected, &other_source).expect_err("types differ");
    assert_eq!(refusal.part, BundlePart::Types);
    assert_eq!(refusal.selected, selected);
    assert_eq!(refusal.differing, other_source);
    let record = refusal.record();
    assert_eq!(record.code, Code::InvalidRuntimeInput);
    assert_eq!(record.cause, "conflicting-identity");
    assert_eq!(record.fields.get("part").map(String::as_str), Some("types"));

    // Another selected clause.
    let other_clause = BundleKey {
        clause: NodeKey::from_digest([4; 32]),
        ..selected.clone()
    };
    let refusal = build(&selected, &other_clause, &selected).expect_err("closure differs");
    assert_eq!(refusal.part, BundlePart::Closure);
    assert_eq!(refusal.differing, other_clause);

    // The `pre` manifest with the `post` closure of one source and clause.
    let post = BundleKey {
        role: ObservationRole::Post,
        ..selected.clone()
    };
    let refusal = build(&selected, &post, &selected).expect_err("the observation differs");
    assert_eq!(refusal.part, BundlePart::Closure);
    assert_eq!(refusal.selected.role, ObservationRole::Pre);
    assert_eq!(refusal.differing.role, ObservationRole::Post);

    // Another document digest of one identity.
    let other_document = BundleKey {
        origin: ObservationOrigin::Document([9; 32]),
        ..selected.clone()
    };
    let refusal = build(&other_document, &selected, &selected).expect_err("manifest differs");
    assert_eq!(refusal.part, BundlePart::Manifest);

    // Three parts of one key resolve the missing Pump by the manifest.
    let bundle = build(&selected, &selected, &selected).expect("one key");
    assert_eq!(
        bundle.resolve(&orders.at("Pump", "p9")),
        Resolution::Dangling
    );
}

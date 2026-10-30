// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 "Product identity" (TC-1190): `HyperProductIdentity::of` and the
//! four refusals that precede every other check of `encode_hyper_query`.

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound, ProofBound};
use qsl_package::CheckedPackage;
use qsl_replay::{
    HyperProductIdentity, HyperQueryRequest, SmtEncodingLimits, SmtEncodingRefusal, SubjectIdentity,
};
use qsl_semantics::check::{admitted_source, PackageDeclarations};
use quire_exact::NodeKey;
use quire_exact::{LimitKind, Meter};
use quire_semantic_value::checking::CheckingLimits;

fn fixture_node(fill: u8) -> NodeKey {
    NodeKey::new(
        "fixture",
        vec![quire_exact::PathSegment::Text(fill.to_string())],
        0,
    )
    .expect("fixture node")
}

use crate::vault_world::{
    non_interference, World, EQUAL_INPUTS, FORALL_EXISTS, LIVENESS, SAVES_POWER, UNLIMITED,
};

fn instance(key: &str) -> Vec<(String, String)> {
    vec![("v".to_owned(), key.to_owned())]
}

fn pairs(parameter: &str, key: &str) -> Vec<(String, String)> {
    vec![(parameter.to_owned(), key.to_owned())]
}

/// `HyperProductIdentity::of` over `world`'s assembled clause and subject.
fn identity(
    world: &World,
    instance: &[(String, String)],
) -> Result<HyperProductIdentity, SmtEncodingRefusal> {
    let subjects = [world.subject()];
    let mut meter = Meter::new(UNLIMITED);
    HyperProductIdentity::of(&HyperQueryRequest {
        clause: world.clause(),
        subjects: &subjects,
        instance,
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    })
}

fn named(refusal: &SmtEncodingRefusal) -> (&'static str, String) {
    (
        refusal.code().expect("a refusal, not a fault"),
        refusal.to_string(),
    )
}

fn refused(result: Result<HyperProductIdentity, SmtEncodingRefusal>) -> SmtEncodingRefusal {
    result.expect_err("the request is refused")
}

fn bound(maximum: u64) -> ProofBound {
    ProofBound::new(
        DomainKey::Node {
            node: fixture_node(7),
            path: Vec::new(),
        },
        Some(DomainKind::Population),
        FiniteBound::Cardinality { maximum },
    )
    .expect("a population takes a cardinality bound")
}

/// FR-315-AC-12: the identity's three members are the clause's `claim`
/// occurrence, FR-336's projection of the subject and the instance; the
/// instance `w` differs only in `instance()`, a proof bound only in
/// `subjects()`, and meters of different budgets give equal identities.
#[trace("TC-1190", "FR-315-AC-12")]
#[test]
fn the_identity_is_the_clause_the_subjects_and_the_instance() {
    let world = World::new(&["v", "w"], false, &non_interference(EQUAL_INPUTS, ""));
    let base = identity(&world, &instance("v")).expect("the request is identified");
    assert_eq!(
        Some(base.clause()),
        world.clause().assembled_occurrence(),
        "the clause member is the assembled occurrence"
    );
    assert_eq!(base.subjects(), [SubjectIdentity::of(&world.subject())]);
    assert_eq!(base.instance(), instance("v"));

    let other = identity(&world, &instance("w")).expect("instance w");
    assert_eq!(other.clause(), base.clause());
    assert_eq!(other.subjects(), base.subjects());
    assert_eq!(other.instance(), instance("w"));
    assert_ne!(other, base);

    let bounded = [world.subject_over(world.package(), vec![bound(2)])];
    let mut meter = Meter::new(UNLIMITED);
    let bounded = HyperProductIdentity::of(&HyperQueryRequest {
        clause: world.clause(),
        subjects: &bounded,
        instance: &instance("v"),
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    })
    .expect("a proof bound");
    assert_eq!(bounded.clause(), base.clause());
    assert_eq!(bounded.instance(), base.instance());
    assert_ne!(bounded.subjects(), base.subjects());

    let mut small = UNLIMITED;
    small.work_units = 1;
    let subjects = [world.subject()];
    let mut meter = Meter::new(small);
    let small = HyperProductIdentity::of(&HyperQueryRequest {
        clause: world.clause(),
        subjects: &subjects,
        instance: &instance("v"),
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    })
    .expect("a small budget");
    assert_eq!(small, base);
}

/// FR-315-AC-12: a clause that differs only in a fairness set is a different
/// item, so its identity differs in `clause()` and in the checked package of
/// `subjects()`, and not in `instance()`.
#[trace("TC-1190", "FR-315-AC-12")]
#[test]
fn a_fairness_set_changes_the_clause_member() {
    let plain = World::new(&["v"], false, &non_interference(EQUAL_INPUTS, ""));
    let fair = World::new(
        &["v"],
        false,
        &non_interference(EQUAL_INPUTS, "fair weak V::Vault::step;"),
    );
    let plain = identity(&plain, &instance("v")).expect("identified");
    let fair = identity(&fair, &instance("v")).expect("identified");
    assert_ne!(plain.clause(), fair.clause());
    // The clause is a node of the subject's checked package, so the package
    // member of `subjects()` changes with it.
    assert_ne!(plain.subjects(), fair.subjects());
    assert_eq!(plain.instance(), fair.instance());
}

/// FR-315-AC-13: an `instance` that is not one pair for the parameter, with a
/// key in its universe, names `instance-binding` at the parameter.
#[trace("TC-1190", "FR-315-AC-13")]
#[test]
fn a_malformed_instance_names_the_parameter() {
    let world = World::new(&["v"], false, &non_interference(EQUAL_INPUTS, ""));
    for bad in [Vec::new(), instance("x"), pairs("u", "v")] {
        let refusal = refused(identity(&world, &bad));
        let (code, text) = named(&refusal);
        assert_eq!(code, "instance-binding", "{bad:?}");
        assert!(text.contains("`v`"), "{text}");
    }
    let liveness = World::new(&["v"], false, LIVENESS);
    let (code, _) = named(&refused(identity(&liveness, &[])));
    assert_eq!(code, "instance-binding");
}

/// FR-315-AC-13: an HP-3 clause names `hyper-form`, before any other check.
#[trace("TC-1190", "FR-315-AC-13")]
#[test]
fn an_hp3_clause_names_hyper_form() {
    let world = World::new(&["v"], false, FORALL_EXISTS);
    let (code, text) = named(&refused(identity(&world, &[])));
    assert_eq!(code, "hyper-form");
    assert!(text.contains("HP-3"), "{text}");
}

/// FR-315-AC-13: `SavesPower` with one subject, or with the `N` subject over
/// another checked package, names `subject-binding` at alias `N`; a subject
/// beyond the aliases names none.
#[trace("TC-1190", "FR-315-AC-13")]
#[test]
fn a_missing_or_foreign_subject_names_its_alias() {
    let world = World::new(&["v"], true, SAVES_POWER);
    let both = [("d", "v"), ("e", "v")]
        .iter()
        .map(|(parameter, key)| ((*parameter).to_owned(), (*key).to_owned()))
        .collect::<Vec<_>>();
    let run = |subjects: &[qsl_eval::model_subject::ModelSubject<'_>]| {
        let mut meter = Meter::new(UNLIMITED);
        HyperProductIdentity::of(&HyperQueryRequest {
            clause: world.clause(),
            subjects,
            instance: &both,
            meter: &mut meter,
            encoding: SmtEncodingLimits::default(),
        })
    };
    let aliases: Vec<&str> = world
        .clause()
        .aliases()
        .iter()
        .map(|alias| alias.as_str())
        .collect();
    assert_eq!(aliases, ["V", "N"]);

    let (code, text) = named(&refused(run(&[world.subject()])));
    assert_eq!(code, "subject-binding");
    assert!(text.contains("`N`"), "{text}");

    let elsewhere = World::new(&["v"], true, SAVES_POWER);
    let foreign = [world.subject(), elsewhere.subject()];
    let (code, text) = named(&refused(run(&foreign)));
    assert_eq!(code, "subject-binding");
    assert!(text.contains("`N`"), "{text}");

    let three = [world.subject(), world.subject(), world.subject()];
    let (code, text) = named(&refused(run(&three)));
    assert_eq!(code, "subject-binding");
    assert!(text.contains("more subjects"), "{text}");

    run(&[world.subject(), world.subject()]).expect("one subject per alias");
}

/// FR-315-AC-13: a subject whose checked package has no emission names
/// `product-identity` at the clause.
#[trace("TC-1190", "FR-315-AC-13")]
#[test]
fn a_package_with_no_emission_names_product_identity() {
    let world = World::new(&["v"], false, &non_interference(EQUAL_INPUTS, ""));
    let graph = PackageDeclarations::new(
        admitted_source(
            qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
            b"",
        ),
        "fixture",
        qsl_foundation::IdentityLimits::default(),
    )
    .check(CheckingLimits::default())
    .expect("an empty package checks");
    let empty = CheckedPackage::link(graph);
    let subjects = [world.subject_over(&empty, Vec::new())];
    let mut meter = Meter::new(UNLIMITED);
    let refusal = refused(HyperProductIdentity::of(&HyperQueryRequest {
        clause: world.clause(),
        subjects: &subjects,
        instance: &instance("v"),
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    }));
    let (code, text) = named(&refusal);
    assert_eq!(code, "product-identity");
    assert!(text.contains("NonInterference"), "{text}");
}

/// FR-315-AC-49: the clause as S3 checked it, before any package assembled
/// it, names `product-identity` and charges nothing; the assembled clause
/// returns the identity.
#[trace("TC-1190", "FR-315-AC-49")]
#[test]
fn an_unassembled_clause_names_product_identity_and_charges_nothing() {
    let world = World::new(&["v"], false, &non_interference(EQUAL_INPUTS, ""));
    assert!(world.unassembled.assembled_occurrence().is_none());
    let subjects = [world.subject()];
    let mut meter = Meter::new(UNLIMITED);
    let refusal = refused(HyperProductIdentity::of(&HyperQueryRequest {
        clause: &world.unassembled,
        subjects: &subjects,
        instance: &instance("v"),
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    }));
    assert_eq!(named(&refusal).0, "product-identity");
    assert_eq!(meter.admission_count(), 0);
    identity(&world, &instance("v")).expect("the assembled clause is identified");
}

/// FR-315-AC-13: `of` charges nothing to the request's meter, even a meter
/// whose budget is already exhausted.
#[trace("TC-1190", "FR-315-AC-13")]
#[test]
fn identifying_a_request_charges_nothing() {
    let world = World::new(&["v"], false, &non_interference(EQUAL_INPUTS, ""));
    let subjects = [world.subject()];
    let mut empty = UNLIMITED;
    empty.work_units = 0;
    let mut meter = Meter::new(empty);
    HyperProductIdentity::of(&HyperQueryRequest {
        clause: world.clause(),
        subjects: &subjects,
        instance: &instance("v"),
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    })
    .expect("identified");
    assert_eq!(meter.admission_count(), 0);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
}

/// FR-315-AC-12: two clauses of one checked package over the same subject
/// differ in `clause()` alone.
#[trace("TC-1190", "FR-315-AC-12")]
#[test]
fn two_clauses_of_one_package_differ_in_the_clause_member() {
    let declarations = format!(
        "{}\n{}",
        non_interference(EQUAL_INPUTS, ""),
        non_interference(EQUAL_INPUTS, "").replace("NonInterference", "Second")
    );
    let world = World::new(&["v"], false, &declarations);
    let clauses = world.package().graph().hyper_clauses();
    assert_eq!(clauses.len(), 2);
    let of = |clause: &qsl_semantics::check::CheckedHyperClause| {
        let subjects = [world.subject()];
        let mut meter = Meter::new(UNLIMITED);
        HyperProductIdentity::of(&HyperQueryRequest {
            clause,
            subjects: &subjects,
            instance: &instance("v"),
            meter: &mut meter,
            encoding: SmtEncodingLimits::default(),
        })
        .expect("identified")
    };
    let (first, second) = (of(&clauses[0]), of(&clauses[1]));
    assert_ne!(first.clause(), second.clause());
    assert_eq!(first.subjects(), second.subjects());
    assert_eq!(first.instance(), second.instance());
}

/// FR-315-AC-52: a parameter that names an alias the clause does not have
/// breaks an invariant of the checked clause, which `of` returns as a fault
/// and not as `instance-binding`.
#[trace("TC-1190", "FR-315-AC-52")]
#[test]
fn a_parameter_with_an_unknown_alias_is_a_fault() {
    let world = World::new(&["v"], false, &non_interference(EQUAL_INPUTS, ""));
    let altered = world.clause().clone().with_parameter_alias_after_s3("Z");
    let subjects = [world.subject()];
    let mut meter = Meter::new(UNLIMITED);
    let result = HyperProductIdentity::of(&HyperQueryRequest {
        clause: &altered,
        subjects: &subjects,
        instance: &instance("v"),
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    });
    assert!(
        matches!(result, Err(SmtEncodingRefusal::Fault(_))),
        "{result:?}"
    );
}

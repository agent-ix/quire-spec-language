// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-195 N01, N02, N06, N07, N09 and N12 (FR-150): the chain-link
//! identity vectors. Every effective identity below is written as the first
//! eight hex digits of the digest quire-specification publishes for that
//! fixture node; the encoder must reproduce them. Each fixture uses TC-195's
//! own node names (`ix://test/orders/X`) and placeholder selections.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_semantics::model::accounting::{
    ChargePoint, Incomplete, LimitKind, Meter, ModelNormalizationLimits,
};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, ValueTypeRef,
};
use qsl_semantics::model::key::{
    DeclarationKey, EffectiveId, EffectiveIdExt, Fact, RULE_INHERIT, RULE_QUALIFY, RULE_REDEFINE,
    RULE_SUBSET,
};
use qsl_semantics::model::normalize::{
    normalize, normalize_with_meter, EffectiveView, ModelRefusalCause, NormalizeOutcome, ViewEntry,
};

const PREFIX: &str = "ix://test/orders/";

const MULTIPLICITY_0_1: Multiplicity = Multiplicity {
    lower: 0,
    upper: Some(1),
    ordered: false,
    unique: true,
};

fn key(node: &str) -> DeclarationKey {
    DeclarationKey::fixture(format!("{PREFIX}{node}"))
}

fn object_type(name: &str, supertypes: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: key(name),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes.iter().map(|parent| key(parent)).collect(),
    })
}

fn field(name: &str, owner: &str, redefines: Option<&str>, subsets: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: key(name),
        owner: key(owner),
        value_type: ValueTypeRef::Package(key("A")),
        multiplicity: MULTIPLICITY_0_1,
        presence: quire_exact::Presence::Required,
        subsets: subsets.iter().map(|target| key(target)).collect(),
        redefines: redefines.map(key),
    })
}

fn f1(selection: &str) -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture(selection),
        vec![
            object_type("A", &[]),
            object_type("B", &["A"]),
            field("A/x", "A", None, &[]),
        ],
    )
}

fn f2_records() -> Vec<DomainPackageRecord> {
    vec![
        object_type("A", &[]),
        object_type("B", &["A"]),
        object_type("C", &["A"]),
        object_type("D", &["B", "C"]),
        field("A/x", "A", None, &[]),
    ]
}

fn f2() -> DomainPackage {
    DomainPackage::new(DomainPackageRef::fixture("n02"), f2_records())
}

/// N06 first stage: `B/x2` and `C/x3` redefine `A/x`.
fn n06_conflict() -> DomainPackage {
    let mut records = f2_records();
    records.push(field("B/x2", "B", Some("A/x"), &[]));
    records.push(field("C/x3", "C", Some("A/x"), &[]));
    DomainPackage::new(DomainPackageRef::fixture("n02"), records)
}

/// N06 second stage: plus `D/x4`, redefining `A/x`.
fn n06_resolved() -> DomainPackage {
    let mut records = n06_conflict().records;
    records.push(field("D/x4", "D", Some("A/x"), &[]));
    DomainPackage::new(DomainPackageRef::fixture("n02"), records)
}

/// N12 base: N06's admitted model with `C/x3` not redefining.
fn n12_without_c_redefinition() -> DomainPackage {
    let mut records = f2_records();
    records.push(field("B/x2", "B", Some("A/x"), &[]));
    records.push(field("C/x3", "C", None, &[]));
    records.push(field("D/x4", "D", Some("A/x"), &[]));
    DomainPackage::new(DomainPackageRef::fixture("n02"), records)
}

fn completed(package: &DomainPackage) -> EffectiveView {
    match normalize(package, ModelNormalizationLimits::default()) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed view, got {other:?}"),
    }
}

fn type_entry<'v>(view: &'v EffectiveView, name: &str) -> &'v ViewEntry {
    view.declarations()
        .iter()
        .find(|entry| {
            entry.preimage.owner_effective_type.is_none() && entry.preimage.original == key(name)
        })
        .unwrap_or_else(|| panic!("effective type {name}"))
}

fn member_entry<'v>(view: &'v EffectiveView, owner: &str, name: &str) -> &'v ViewEntry {
    let owner_id = type_entry(view, owner).effective_id;
    view.declarations()
        .iter()
        .find(|entry| {
            entry.preimage.owner_effective_type == Some(owner_id)
                && entry.preimage.original == key(name)
        })
        .unwrap_or_else(|| panic!("effective member ({owner}, {name})"))
}

fn short(entry: &ViewEntry) -> String {
    entry.effective_id.short_hex()
}

/// One retained fact as `(rule, inputs, parent short digest)`.
type Shape = (&'static str, Vec<String>, Option<String>);

fn shape(entry: &ViewEntry) -> Vec<Shape> {
    entry
        .preimage
        .derivation
        .iter()
        .map(|fact: &Fact| {
            let rule = if fact.rule == RULE_QUALIFY {
                "qualify"
            } else if fact.rule == RULE_INHERIT {
                "inherit"
            } else if fact.rule == RULE_SUBSET {
                "subset"
            } else if fact.rule == RULE_REDEFINE {
                "redefine"
            } else {
                panic!("a fact cites a rule outside the four")
            };
            (
                rule,
                fact.inputs
                    .iter()
                    .map(|input| input.node.trim_start_matches(PREFIX).to_owned())
                    .collect(),
                fact.parent.as_ref().map(EffectiveIdExt::short_hex),
            )
        })
        .collect()
}

fn expect(facts: &[(&'static str, &[&str], Option<&str>)]) -> Vec<Shape> {
    facts
        .iter()
        .map(|(rule, inputs, parent)| {
            (
                *rule,
                inputs.iter().map(|input| (*input).to_owned()).collect(),
                parent.map(str::to_owned),
            )
        })
        .collect()
}

fn universe_short(view: &EffectiveView) -> String {
    view.object_universe().identity().to_string()[..8].to_owned()
}

fn charge_points(meter: &Meter) -> Vec<ChargePoint> {
    meter.admitted_charges().to_vec()
}

/// The rule identities that moved to `/v2`.
#[trace("QSpec-TC-195", "QSpec-FR-150-AC-12")]
#[test]
fn the_three_rules_and_the_unchanged_qualify_rule_carry_their_published_identities() {
    assert_eq!(RULE_QUALIFY.identity, "quire.model.normalize.qualify/v1");
    assert_eq!(RULE_INHERIT.identity, "quire.model.normalize.inherit/v2");
    assert_eq!(RULE_SUBSET.identity, "quire.model.normalize.subset/v2");
    assert_eq!(RULE_REDEFINE.identity, "quire.model.normalize.redefine/v2");
}

/// N01: identities, retained facts, JCS lengths and the exact counters.
#[trace("QSpec-TC-195", "QSpec-FR-150-AC-6", "QSpec-FR-150-AC-12")]
#[test]
fn n01_identities_facts_and_counters() {
    let package = f1("n01");
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("N01 completes");
    };

    let a = type_entry(&view, "A");
    let b = type_entry(&view, "B");
    let ax = member_entry(&view, "A", "A/x");
    let bx = member_entry(&view, "B", "A/x");
    assert_eq!(short(a), "4531dd9e");
    assert_eq!(short(b), "c5264509");
    assert_eq!(short(ax), "86ed7ba6");
    assert_eq!(short(bx), "21125d3a");
    assert_eq!(view.identity().short_hex(), "76d04f1e");
    assert_eq!(universe_short(&view), "af5934e1");
    assert_eq!(
        view.object_universe().root_types,
        vec![a.effective_id],
        "the universe's root types"
    );

    assert_eq!(shape(a), expect(&[("qualify", &["A"], None)]));
    assert_eq!(
        shape(b),
        expect(&[
            ("qualify", &["B"], None),
            ("inherit", &["A"], Some("4531dd9e")),
        ])
    );
    assert_eq!(shape(ax), expect(&[("qualify", &["A/x"], None)]));
    assert_eq!(
        shape(bx),
        expect(&[("inherit", &["A", "A/x"], Some("86ed7ba6"))]),
        "an inherited member has inherit facts only"
    );

    assert_eq!(a.preimage.canonical_len(), 366);
    assert_eq!(b.preimage.canonical_len(), 666);
    assert_eq!(ax.preimage.canonical_len(), 491);
    assert_eq!(bx.preimage.canonical_len(), 695);
    assert_eq!(view.object_universe().canonical_len(), 335);
    assert_eq!(view.canonical_len(), 3101);

    use ChargePoint::{
        NormalizeCycleCheck, NormalizeDeclaration, NormalizeFact, NormalizeHash, NormalizeRecord,
    };
    assert_eq!(
        charge_points(&meter),
        vec![
            NormalizeRecord,
            NormalizeRecord,
            NormalizeRecord,
            NormalizeFact,
            NormalizeFact,
            NormalizeFact,
            NormalizeCycleCheck,
            NormalizeFact,
            NormalizeFact,
            NormalizeDeclaration,
            NormalizeHash,
            NormalizeDeclaration,
            NormalizeHash,
            NormalizeDeclaration,
            NormalizeHash,
            NormalizeDeclaration,
            NormalizeHash,
            NormalizeHash,
            NormalizeHash,
        ]
    );
    assert_eq!(meter.consumed(LimitKind::HashedBytes), 5654);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 19);
    assert_eq!(meter.consumed(LimitKind::DerivationFacts), 5);
}

/// N01's three boundary runs at the exact limits TC-195 names.
#[trace("QSpec-TC-195", "QSpec-FR-150-AC-6")]
#[test]
fn n01_boundaries_deny_at_the_published_charge() {
    let exact = ModelNormalizationLimits {
        declaration_records: 3,
        derivation_facts: 5,
        effective_declarations: 4,
        dispatch_candidates: 0,
        hashed_bytes: 5654,
        work_units: 19,
        ancestor_steps: u64::MAX,
        family_steps: u64::MAX,
        ..ModelNormalizationLimits::default()
    };
    let package = f1("n01");
    assert!(matches!(
        normalize(&package, exact),
        NormalizeOutcome::Completed(_)
    ));

    let denied = |limits: ModelNormalizationLimits| match normalize(&package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => incomplete,
        other => panic!("expected incomplete, got {other:?}"),
    };
    assert_eq!(
        denied(ModelNormalizationLimits {
            work_units: 18,
            ..exact
        }),
        Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 18,
            consumed: 18,
            next_charge: 1,
            charge_point: ChargePoint::NormalizeHash,
        }
    );
    assert_eq!(
        denied(ModelNormalizationLimits {
            hashed_bytes: 5653,
            ..exact
        }),
        Incomplete {
            limit_kind: LimitKind::HashedBytes,
            limit: 5653,
            consumed: 2553,
            next_charge: 3101,
            charge_point: ChargePoint::NormalizeHash,
        }
    );
    assert_eq!(
        denied(ModelNormalizationLimits {
            derivation_facts: 4,
            ..exact
        }),
        Incomplete {
            limit_kind: LimitKind::DerivationFacts,
            limit: 4,
            consumed: 4,
            next_charge: 5,
            charge_point: ChargePoint::NormalizeFact,
        }
    );
}

/// N02: a diamond holds one link per direct edge and none per path.
#[trace("QSpec-TC-195", "QSpec-FR-150-AC-6", "QSpec-FR-081-AC-9")]
#[test]
fn n02_the_diamond_stores_one_link_per_direct_edge() {
    let package = f2();
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("N02 completes");
    };
    assert_eq!(view.declarations().len(), 8);
    assert_eq!(short(type_entry(&view, "A")), "4531dd9e");
    assert_eq!(short(type_entry(&view, "B")), "c5264509");
    assert_eq!(short(type_entry(&view, "C")), "aa1ab053");
    assert_eq!(short(type_entry(&view, "D")), "9425c80d");
    assert_eq!(short(member_entry(&view, "A", "A/x")), "86ed7ba6");
    assert_eq!(short(member_entry(&view, "B", "A/x")), "21125d3a");
    assert_eq!(short(member_entry(&view, "C", "A/x")), "d4ce6547");
    assert_eq!(short(member_entry(&view, "D", "A/x")), "2fc1afee");
    assert_eq!(view.identity().short_hex(), "ecdf3d6d");
    assert_eq!(universe_short(&view), "6763207e");

    assert_eq!(
        shape(type_entry(&view, "D")),
        expect(&[
            ("qualify", &["D"], None),
            ("inherit", &["B"], Some("c5264509")),
            ("inherit", &["C"], Some("aa1ab053")),
        ])
    );
    assert_eq!(
        shape(member_entry(&view, "D", "A/x")),
        expect(&[
            ("inherit", &["B", "A/x"], Some("21125d3a")),
            ("inherit", &["C", "A/x"], Some("d4ce6547")),
        ])
    );

    let facts: usize = view
        .declarations()
        .iter()
        .map(|entry| entry.preimage.derivation.len())
        .sum();
    assert_eq!(facts, 13, "five qualify and eight inherit facts");
    assert_eq!(meter.consumed(LimitKind::DerivationFacts), 13);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 40);
    assert_eq!(meter.consumed(LimitKind::HashedBytes), 13088);
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeCycleCheck)
            .count(),
        4,
        "one cycle-check per direct edge"
    );
}

/// N02 also: `A` and `(A, A/x)` equal their F1 identities, while views and
/// universes differ (an effective identity binds no selection).
#[trace("QSpec-TC-195", "QSpec-FR-150-AC-6")]
#[test]
fn n02_an_effective_identity_binds_no_selection() {
    let one = completed(&f1("n01"));
    let two = completed(&f2());
    assert_eq!(type_entry(&one, "A").effective_id, type_entry(&two, "A").effective_id);
    assert_eq!(
        member_entry(&one, "A", "A/x").effective_id,
        member_entry(&two, "A", "A/x").effective_id
    );
    assert_ne!(one.identity(), two.identity());
}

/// N06: two undominated redefiners refuse; a most-derived redefiner resolves
/// the contest and the losers stay hidden.
#[trace("QSpec-TC-195", "QSpec-FR-150-AC-6", "QSpec-FR-081-AC-3", "QSpec-FR-081-AC-6")]
#[test]
fn n06_conflict_refuses_then_resolves_with_hidden_losers() {
    match normalize(&n06_conflict(), ModelNormalizationLimits::default()) {
        NormalizeOutcome::Refused(refusals) => {
            assert_eq!(refusals.len(), 1);
            let refusal = refusals.iter().next().expect("one refusal");
            match &refusal.cause {
                ModelRefusalCause::DerivationConflict {
                    type_,
                    member,
                    redefiners,
                    ..
                } => {
                    assert_eq!(type_, &key("D"));
                    assert_eq!(member, &key("A/x"));
                    assert_eq!(redefiners, &vec![key("B/x2"), key("C/x3")]);
                }
                other => panic!("expected a derivation conflict, got {other:?}"),
            }
        }
        other => panic!("expected a refusal, got {other:?}"),
    }

    let view = completed(&n06_resolved());
    let dx4 = member_entry(&view, "D", "D/x4");
    assert!(dx4.visible, "(D, D/x4) is exposed");
    assert_eq!(
        shape(dx4),
        expect(&[
            ("qualify", &["D/x4"], None),
            ("redefine", &["D/x4", "A/x"], None),
        ])
    );
    let dax = member_entry(&view, "D", "A/x");
    assert!(!dax.visible, "(D, A/x) is retained but hidden");
    let b_ax = short(member_entry(&view, "B", "A/x"));
    let c_ax = short(member_entry(&view, "C", "A/x"));
    assert_eq!(
        shape(dax),
        expect(&[
            ("inherit", &["B", "A/x"], Some(b_ax.as_str())),
            ("inherit", &["C", "A/x"], Some(c_ax.as_str())),
            ("redefine", &["B/x2", "A/x"], None),
            ("redefine", &["C/x3", "A/x"], None),
            ("redefine", &["D/x4", "A/x"], None),
        ])
    );
    let b_x2 = member_entry(&view, "B", "B/x2");
    let dbx2 = member_entry(&view, "D", "B/x2");
    assert!(!dbx2.visible);
    assert_eq!(
        shape(dbx2),
        expect(&[
            ("inherit", &["B", "B/x2"], Some(short(b_x2).as_str())),
            ("redefine", &["B/x2", "A/x"], None),
        ])
    );
    let c_x3 = member_entry(&view, "C", "C/x3");
    let dcx3 = member_entry(&view, "D", "C/x3");
    assert!(!dcx3.visible);
    assert_eq!(
        shape(dcx3),
        expect(&[
            ("inherit", &["C", "C/x3"], Some(short(c_x3).as_str())),
            ("redefine", &["C/x3", "A/x"], None),
        ])
    );
}

/// N07: IR node order changes nothing.
#[trace("QSpec-TC-195", "QSpec-FR-081-AC-7")]
#[test]
fn n07_reversed_nodes_give_the_same_identities_and_view() {
    let forward = completed(&f2());
    let mut reversed = f2();
    reversed.records.reverse();
    let reversed = completed(&reversed);
    assert_eq!(reversed.identity().short_hex(), "ecdf3d6d");
    assert_eq!(forward.identity(), reversed.identity());
    let ids = |view: &EffectiveView| -> BTreeMap<String, String> {
        view.declarations()
            .iter()
            .map(|entry| (format!("{:?}", entry.preimage.original), short(entry)))
            .collect()
    };
    assert_eq!(ids(&forward), ids(&reversed));
}

/// N09: no selection digest leaks into a declaration; only the view and the
/// universe bind it.
#[trace("QSpec-TC-195", "QSpec-FR-150-AC-6")]
#[test]
fn n09_a_second_selection_changes_only_the_view_and_the_universe() {
    let first = completed(&f1("n01"));
    let second = completed(&f1("n01v2"));
    let ids = |view: &EffectiveView| -> Vec<EffectiveId> {
        view.declarations()
            .iter()
            .map(|entry| entry.effective_id)
            .collect()
    };
    assert_eq!(ids(&first), ids(&second));
    assert_eq!(first.identity().short_hex(), "76d04f1e");
    assert_eq!(second.identity().short_hex(), "f81504ef");
    assert_eq!(universe_short(&first), "af5934e1");
    assert_eq!(universe_short(&second), "1164b658");
}

/// N12: the counters of the admitted fixture and the identities that move
/// when `C/x3` stops redefining.
#[trace("QSpec-TC-195", "QSpec-FR-150-AC-6", "QSpec-FR-081-AC-10", "QSpec-FR-081-AC-11")]
#[test]
fn n12_counters_and_the_ancestry_each_change_reaches() {
    let package = n12_without_c_redefinition();
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("N12 completes");
    };
    assert_eq!(view.declarations().len(), 13);
    assert_eq!(meter.consumed(LimitKind::DerivationFacts), 24);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 76);

    // Against the model where `C/x3` redefines `A/x` (the N06 admitted
    // model without `D/x4`'s siblings changing): exactly (C, A/x),
    // (C, C/x3), (D, A/x) and (D, C/x3) change identity.
    let mut records = f2_records();
    records.push(field("B/x2", "B", Some("A/x"), &[]));
    records.push(field("C/x3", "C", Some("A/x"), &[]));
    records.push(field("D/x4", "D", Some("A/x"), &[]));
    let with_redefinition = completed(&DomainPackage::new(
        DomainPackageRef::fixture("n02"),
        records,
    ));
    assert_eq!(with_redefinition.declarations().len(), 13);
    let changed: Vec<(&str, &str)> = [
        ("A", "A/x"),
        ("B", "A/x"),
        ("B", "B/x2"),
        ("C", "A/x"),
        ("C", "C/x3"),
        ("D", "A/x"),
        ("D", "B/x2"),
        ("D", "C/x3"),
        ("D", "D/x4"),
    ]
    .into_iter()
    .filter(|(owner, name)| {
        member_entry(&view, owner, name).effective_id
            != member_entry(&with_redefinition, owner, name).effective_id
    })
    .collect();
    assert_eq!(
        changed,
        vec![("C", "A/x"), ("C", "C/x3"), ("D", "A/x"), ("D", "C/x3")]
    );
    for name in ["A", "B", "C", "D"] {
        assert_eq!(
            type_entry(&view, name).effective_id,
            type_entry(&with_redefinition, name).effective_id,
            "a type has no phase-4 fact, so no member change moves it"
        );
    }
    assert_eq!(
        view.declarations().len(),
        with_redefinition.declarations().len()
    );
    let facts = |view: &EffectiveView| -> usize {
        view.declarations()
            .iter()
            .map(|entry| entry.preimage.derivation.len())
            .sum()
    };
    assert_eq!(facts(&view), 24);
    assert_eq!(facts(&with_redefinition), 28);
}

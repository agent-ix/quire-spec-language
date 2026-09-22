// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-195: model normalization provenance (FR-150).
//!
//! Fixtures F1/F2 are transcribed from the TC-195 procedure. Ground-truth
//! digest and preimage checks compare the crate's own computed output against
//! itself (determinism, order-independence, structural shape) rather than
//! against a pinned external hash.

use ix_trace_rs::trace;
use quire_spec_language::model::accounting::{
    ChargePoint, Incomplete, LimitKind, ModelNormalizationLimits,
};
use quire_spec_language::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, OperationEffect, OperationMemberRecord, ValueTypeRef,
};
use quire_spec_language::model::key::{DeclarationKey, EffectiveId, RULE_REDEFINE};
use quire_spec_language::model::normalize::{
    normalize, normalize_with_meter, ModelRefusal, ModelRefusalCause, NormalizeOutcome, Refusals,
};

const MULTIPLICITY_0_1: Multiplicity = Multiplicity {
    lower: 0,
    upper: Some(1),
    ordered: false,
    unique: true,
};

fn object_type(identity: &str, supertypes: Vec<&str>) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: DeclarationKey::fixture(identity),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes
            .into_iter()
            .map(DeclarationKey::fixture)
            .collect(),
    })
}

fn field_member(identity: &str, owner: &str, value_type: &str) -> DomainPackageRecord {
    field_member_redefining(identity, owner, value_type, None, vec![])
}

/// `field_member`, plus this field's own inline `redefines`
/// (`model-complete.md`:162) and `subsets` (`model-complete.md`:161)
/// properties -- never a separate redefinition or subsetting record.
fn field_member_redefining(
    identity: &str,
    owner: &str,
    value_type: &str,
    redefines: Option<&str>,
    subsets: Vec<&str>,
) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        value_type: ValueTypeRef::Package(DeclarationKey::fixture(value_type)),
        multiplicity: MULTIPLICITY_0_1,
        subsets: subsets.into_iter().map(DeclarationKey::fixture).collect(),
        redefines: redefines.map(DeclarationKey::fixture),
    })
}

/// A minimal operation member: no parameters, no result, no effect frame,
/// no postcondition -- enough to exist as a redefinable member without
/// pulling in `crate::model::dispatch`/`conformance`'s own richer fixtures.
fn operation_member(identity: &str, owner: &str) -> DomainPackageRecord {
    operation_member_redefining(identity, owner, None)
}

/// `operation_member`, plus this operation's own inline `redefines`
/// (`model-complete.md`:162) property -- never a separate redefinition
/// record.
fn operation_member_redefining(
    identity: &str,
    owner: &str,
    redefines: Option<&str>,
) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        parameters: Vec::new(),
        result: None,
        effect: OperationEffect {
            modifies: Vec::new(),
            creates: Vec::new(),
            deletes: Vec::new(),
        },
        has_own_precondition: false,
        own_postcondition_clauses: Vec::new(),
        has_body: true,
        redefines: redefines.map(DeclarationKey::fixture),
    })
}

/// F1 (domain package `test/orders` version 1, selection placeholder
/// `n01`): types `A`, `B`; field `A.x` of `A`; generalization `B` -> `A`.
/// Node identities are TC-195's own fixture nodes (`ix://test/orders/...`).
fn fixture_f1() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("n01"),
        vec![
            object_type("ix://test/orders/A", vec![]),
            object_type("ix://test/orders/B", vec!["ix://test/orders/A"]),
            field_member(
                "ix://test/orders/A/x",
                "ix://test/orders/A",
                "ix://test/orders/A",
            ),
        ],
    )
}

/// F2 (domain package `test/orders` version 1, selection placeholder
/// `n02`): F1's records plus types `C`, `D` and generalizations `C` -> `A`,
/// `D` -> `B`, `D` -> `C`.
fn fixture_f2() -> DomainPackage {
    let mut records = fixture_f1().records;
    records.push(object_type(
        "ix://test/orders/C",
        vec!["ix://test/orders/A"],
    ));
    records.push(object_type(
        "ix://test/orders/D",
        vec!["ix://test/orders/B", "ix://test/orders/C"],
    ));
    DomainPackage::new(DomainPackageRef::fixture("n02"), records)
}

/// F2 plus field members `B.x2` (`B`, redefines `A.x`) and `C.x3` (`C`,
/// redefines `A.x`), both of `A`'s own value type — TC-195 N06's first
/// stage: two undominated redefiners of `A.x` reach `D` through sibling
/// owners `B` and `C`.
fn fixture_n06_conflict() -> DomainPackage {
    let mut records = fixture_f2().records;
    records.push(field_member_redefining(
        "model.B.x2",
        "ix://test/orders/B",
        "ix://test/orders/A",
        Some("ix://test/orders/A/x"),
        vec![],
    ));
    records.push(field_member_redefining(
        "model.C.x3",
        "ix://test/orders/C",
        "ix://test/orders/A",
        Some("ix://test/orders/A/x"),
        vec![],
    ));
    DomainPackage::new(DomainPackageRef::fixture("bundle.n06"), records)
}

/// TC-195 N06's second stage: `fixture_n06_conflict` plus field `D.x4` of
/// `A`'s value type and record `redef.D` (`D`, `D.x4` redefines `A.x`). `D`
/// is a proper descendant of both `B` and `C`, so `D.x4` dominates every
/// other redefiner of `A.x` and the conflict resolves.
fn fixture_n06_resolved() -> DomainPackage {
    let mut records = fixture_n06_conflict().records;
    records.push(field_member_redefining(
        "model.D.x4",
        "ix://test/orders/D",
        "ix://test/orders/A",
        Some("ix://test/orders/A/x"),
        vec![],
    ));
    DomainPackage::new(DomainPackageRef::fixture("bundle.n06"), records)
}

/// `fixture_n06_conflict` plus a type `E` with no generalization, field
/// `E.y` of `A`'s value type, and redefinition record `redef.E` (`E`,
/// `E.y` redefines `A.x`). `E` does not inherit `A`, so `A.x` is not an
/// effective member of `E` -- this redefinition record's own target is
/// unreachable from its owner, distinct from N06's own dominance conflict
/// at `D`.
fn fixture_n06_conflict_with_unreachable_redefiner() -> DomainPackage {
    let mut records = fixture_n06_conflict().records;
    records.push(object_type("model.E", vec![]));
    records.push(field_member_redefining(
        "model.E.y",
        "model.E",
        "ix://test/orders/A",
        Some("ix://test/orders/A/x"),
        vec![],
    ));
    DomainPackage::new(DomainPackageRef::fixture("bundle.n06e"), records)
}

/// `fixture_n06_conflict` (`D` conflicts on `A.x`) plus a second, unrelated
/// diamond: root `M` (field `M.w`), `M1` and `M2` (both `<- M`,
/// `M1.w1`/`M2.w2` redefine `M.w` -- sibling owners, neither dominating the
/// other), and `B9` (`<- M1`, `<- M2`, so it inherits both undominated
/// redefiners of `M.w`). `B9` sorts before `D`. Node identities live under
/// `ix://n06/...` rather than `model.*` (the `model.` prefix sorts after
/// `ix://test/orders/D`, which would invert the ordering this test pins);
/// `ix://n06/...` sorts before `ix://test/orders/D` (`n` < `t`).
fn fixture_n06_conflict_with_a_second_diamond_sorting_first() -> DomainPackage {
    let mut records = fixture_n06_conflict().records;
    records.push(object_type("ix://n06/M", vec![]));
    records.push(object_type("ix://n06/M1", vec!["ix://n06/M"]));
    records.push(object_type("ix://n06/M2", vec!["ix://n06/M"]));
    records.push(object_type(
        "ix://n06/B9",
        vec!["ix://n06/M1", "ix://n06/M2"],
    ));
    records.push(field_member("ix://n06/M/w", "ix://n06/M", "ix://n06/M"));
    records.push(field_member_redefining(
        "ix://n06/M1/w1",
        "ix://n06/M1",
        "ix://n06/M",
        Some("ix://n06/M/w"),
        vec![],
    ));
    records.push(field_member_redefining(
        "ix://n06/M2/w2",
        "ix://n06/M2",
        "ix://n06/M",
        Some("ix://n06/M/w"),
        vec![],
    ));
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.n06.two-diamonds"),
        records,
    )
}

/// `E` (no generalization) with `redef.Ey` (`E`, `E.y` redefines `A.x`):
/// `A.x` is not an effective member of `E`. `Da` (`<- E`) inherits the
/// identical record and fails the identical check for the identical
/// reason. `Da` sorts before `E` in `type_keys`' ascending identity order.
fn fixture_unreachable_target_reached_by_owner_and_an_earlier_sorted_descendant() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.rank02"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.E", vec![]),
            object_type("model.Da", vec!["model.E"]),
            field_member("model.A.x", "model.A", "model.A"),
            field_member_redefining("model.E.y", "model.E", "model.A", Some("model.A.x"), vec![]),
        ],
    )
}

// `fixture_unreachable_redefiner_reached_by_owner_and_an_earlier_sorted_descendant`
// (and its consuming test,
// `n06_unreachable_redefiner_refusal_names_the_records_owning_type_not_a_tied_descendant`)
// are dropped: under QSpec's inline `redefines` property
// (`model-complete.md`:162), a redefining member's `redefines` is always
// evaluated at its own `owner` field (`src/model/normalize.rs`'s
// `apply_redefinitions`: `let Some(path) = owner_paths.get(owner) else {
// continue };` immediately followed by using that same `owner` to reach
// `type_key`), and a member declared with a given `owner` is by construction
// always an effective member of that same `owner` -- so "redefining member
// not an effective member of the owner its own record names" is not a
// reachable code path.

/// A second, independently-built instance of the same ranking shape as
/// `fixture_n06_conflict_with_unreachable_redefiner`: the
/// `normalize.conflict-check`-stage owner sorts before the
/// `normalize.redefinition-check`-stage owner in `type_keys`' ascending
/// order, to pin the ranking rule itself rather than any one domain package's own
/// identities. Types `G` (field `G.g`), `H` and `I` (both `<- G`, with
/// `H.h2`/`I.i3` redefining `G.g` -- sibling owners, neither dominating the
/// other), `J` (`<- H`, `<- I`, so it inherits both undominated redefiners
/// of `G.g`), and `K` (no generalization, `K.k` redefines `G.g`). `J`'s own
/// dominance conflict over `G.g` and `K`'s own unreachable target (`K` does
/// not inherit `G`) coexist; `J` sorts before `K`.
fn fixture_conflict_check_owner_sorts_before_redefinition_check_owner() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.rank01"),
        vec![
            object_type("model.G", vec![]),
            object_type("model.H", vec!["model.G"]),
            object_type("model.I", vec!["model.G"]),
            object_type("model.J", vec!["model.H", "model.I"]),
            object_type("model.K", vec![]),
            field_member("model.G.g", "model.G", "model.G"),
            field_member_redefining(
                "model.H.h2",
                "model.H",
                "model.G",
                Some("model.G.g"),
                vec![],
            ),
            field_member_redefining(
                "model.I.i3",
                "model.I",
                "model.G",
                Some("model.G.g"),
                vec![],
            ),
            field_member_redefining("model.K.k", "model.K", "model.G", Some("model.G.g"), vec![]),
        ],
    )
}

/// TC-196 R07's second shape (origin/main, after QSpec #86): `B` itself
/// (not two sibling lineages) declares two distinct redefining members,
/// `B/z` and `B/z2`, both `redefines: A/x` — the identical single inherited
/// target contended by two redefiners under one owner, distinct from N06's
/// diamond conflict (two different owners, neither dominating the other).
fn fixture_r07_same_owner_contending_redefiners() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.r07"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            field_member("model.A.x", "model.A", "model.A"),
            field_member_redefining("model.B.z", "model.B", "model.A", Some("model.A.x"), vec![]),
            field_member_redefining(
                "model.B.z2",
                "model.B",
                "model.A",
                Some("model.A.x"),
                vec![],
            ),
        ],
    )
}

/// Re-review of PR #157's round-3 fix: `C <= A` with `C/w` (owned by `C`)
/// `redefines: A/x`; `B <= C` with `B/z` and `B/z2` (both owned by `B`)
/// also `redefines: A/x`. `B`'s edges already dominate `C`'s single edge
/// (a more-derived owner beating a less-derived one, exactly as it would
/// if `B` had only one redefiner), so `C`'s edge takes no part in the
/// "one owner, several redefiners" test — only `B`'s two edges do, and
/// they share the identical owner.
fn fixture_r07_dominated_owner_takes_no_part_in_the_same_owner_test() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.r07d"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.C"]),
            object_type("model.C", vec!["model.A"]),
            field_member("model.A.x", "model.A", "model.A"),
            field_member_redefining("model.C.w", "model.C", "model.A", Some("model.A.x"), vec![]),
            field_member_redefining("model.B.z", "model.B", "model.A", Some("model.A.x"), vec![]),
            field_member_redefining(
                "model.B.z2",
                "model.B",
                "model.A",
                Some("model.A.x"),
                vec![],
            ),
        ],
    )
}

/// Two object types, `test/orders:model.A` and `other/pkg:model.A`, share
/// the display node `model.A` but are unrelated: neither generalizes the
/// other, and both directly generalize `model.Root`. Each declares its own
/// field redefining `model.Root.x`, and `model.D` generalizes both, so
/// `apply_redefinitions` must resolve the redefiners of `model.Root.x` by
/// each owner's whole `DeclarationKey`, not by `node` alone — a `node`-only
/// comparison would treat the two unrelated owners as one, changing this
/// derivation conflict into a false "one owner, several redefiners"
/// (`RedefinitionTarget`) refusal instead.
fn fixture_derivation_conflict_across_packages_sharing_an_owner_node() -> DomainPackage {
    let owner_orders = DeclarationKey {
        package: "test/orders".to_owned(),
        node: "model.A".to_owned(),
    };
    let owner_other = DeclarationKey {
        package: "other/pkg".to_owned(),
        node: "model.A".to_owned(),
    };
    let root = DeclarationKey::fixture("model.Root");
    let root_x = DeclarationKey::fixture("model.Root.x");
    let d = DeclarationKey::fixture("model.D");
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.cross-package-conflict"),
        vec![
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: root.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: vec![],
            }),
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: owner_orders.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: vec![root.clone()],
            }),
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: owner_other.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: vec![root.clone()],
            }),
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: d.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: vec![owner_orders.clone(), owner_other.clone()],
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: root_x.clone(),
                owner: root.clone(),
                value_type: ValueTypeRef::Package(root.clone()),
                multiplicity: MULTIPLICITY_0_1,
                subsets: vec![],
                redefines: None,
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: DeclarationKey {
                    package: "test/orders".to_owned(),
                    node: "model.A.x2".to_owned(),
                },
                owner: owner_orders.clone(),
                value_type: ValueTypeRef::Package(root.clone()),
                multiplicity: MULTIPLICITY_0_1,
                subsets: vec![],
                redefines: Some(root_x.clone()),
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: DeclarationKey {
                    package: "other/pkg".to_owned(),
                    node: "model.A.x3".to_owned(),
                },
                owner: owner_other.clone(),
                value_type: ValueTypeRef::Package(root.clone()),
                multiplicity: MULTIPLICITY_0_1,
                subsets: vec![],
                redefines: Some(root_x.clone()),
            }),
        ],
    )
}

/// The same-owner test in `apply_redefinitions` (immediately above the
/// `derivation-conflict`/`redefinition-target` split) compares owners by
/// their whole `DeclarationKey`, not by `node` alone: `test/orders:model.A`
/// and `other/pkg:model.A` are two unrelated owners that happen to share a
/// display node, so their two redefiners of `model.Root.x` are a genuine
/// derivation conflict, never a same-owner ambiguity.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r07_two_owners_sharing_a_node_across_packages_refuse_derivation_conflict_not_redefinition_target(
) {
    match normalize(
        &fixture_derivation_conflict_across_packages_sharing_an_owner_node(),
        ModelNormalizationLimits::UNLIMITED,
    ) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                vec![ModelRefusal {
                    code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                    cause: ModelRefusalCause::DerivationConflict {
                        type_: DeclarationKey::fixture("model.D"),
                        member: DeclarationKey::fixture("model.Root.x"),
                        redefiners: vec![
                            DeclarationKey {
                                package: "other/pkg".to_owned(),
                                node: "model.A.x3".to_owned(),
                            },
                            DeclarationKey {
                                package: "test/orders".to_owned(),
                                node: "model.A.x2".to_owned(),
                            },
                        ],
                    },
                    detail: "type model.D has 2 undominated redefinitions of model.Root.x: \
                              [model.A, model.A.x3, model.Root.x] and \
                              [model.A, model.A.x2, model.Root.x]"
                        .to_string(),
                }]
            );
        }
        other => panic!("expected Refused(derivation-conflict), got {other:?}"),
    }
}

/// `test/orders:model.Shared` and `other/pkg:model.Shared` share a display
/// node, and `other/pkg:model.Shared` genuinely descends from
/// `test/orders:model.Shared` (a real, full-key dominance relation, not a
/// node collision): the dominance loop in `apply_redefinitions` must compare
/// owners by their whole `DeclarationKey`, not by `node` alone, to exclude
/// the dominated owner's own redefiner from the same-owner test. `D2`
/// inherits `low`'s single redefiner and `mid`'s two contending redefiners
/// of `model.Root.x`; only `mid`'s two remain once `low`'s is correctly
/// excluded, so the refusal names a same-owner ambiguity of exactly two
/// redefiners, never three.
fn fixture_redefinition_target_excludes_an_owner_dominated_by_a_same_node_descendant(
) -> DomainPackage {
    let root = DeclarationKey::fixture("model.Root");
    let root_x = DeclarationKey::fixture("model.Root.x");
    let low = DeclarationKey {
        package: "test/orders".to_owned(),
        node: "model.Shared".to_owned(),
    };
    let mid = DeclarationKey {
        package: "other/pkg".to_owned(),
        node: "model.Shared".to_owned(),
    };
    let d2 = DeclarationKey::fixture("model.D2");
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.cross-package-dominance"),
        vec![
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: root.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: vec![],
            }),
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: low.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: vec![root.clone()],
            }),
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: mid.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: vec![low.clone()],
            }),
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: d2.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: vec![mid.clone()],
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: root_x.clone(),
                owner: root.clone(),
                value_type: ValueTypeRef::Package(root.clone()),
                multiplicity: MULTIPLICITY_0_1,
                subsets: vec![],
                redefines: None,
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: DeclarationKey {
                    package: "test/orders".to_owned(),
                    node: "model.Shared.w".to_owned(),
                },
                owner: low.clone(),
                value_type: ValueTypeRef::Package(root.clone()),
                multiplicity: MULTIPLICITY_0_1,
                subsets: vec![],
                redefines: Some(root_x.clone()),
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: DeclarationKey {
                    package: "other/pkg".to_owned(),
                    node: "model.Shared.z1".to_owned(),
                },
                owner: mid.clone(),
                value_type: ValueTypeRef::Package(root.clone()),
                multiplicity: MULTIPLICITY_0_1,
                subsets: vec![],
                redefines: Some(root_x.clone()),
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: DeclarationKey {
                    package: "other/pkg".to_owned(),
                    node: "model.Shared.z2".to_owned(),
                },
                owner: mid.clone(),
                value_type: ValueTypeRef::Package(root.clone()),
                multiplicity: MULTIPLICITY_0_1,
                subsets: vec![],
                redefines: Some(root_x.clone()),
            }),
        ],
    )
}

/// The dominance loop (the fix's first line) must compare owners by their
/// whole key: `other/pkg:model.Shared` genuinely dominates
/// `test/orders:model.Shared` and must exclude its redefiner from the
/// same-owner test, even though the two owners share a display node.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r07_a_same_node_owner_genuinely_dominated_across_packages_is_excluded_from_the_same_owner_test()
{
    match normalize(
        &fixture_redefinition_target_excludes_an_owner_dominated_by_a_same_node_descendant(),
        ModelNormalizationLimits::UNLIMITED,
    ) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                vec![ModelRefusal {
                    code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                    cause: ModelRefusalCause::RedefinitionTarget {
                        redefiners: vec![
                            DeclarationKey {
                                package: "other/pkg".to_owned(),
                                node: "model.Shared.z1".to_owned(),
                            },
                            DeclarationKey {
                                package: "other/pkg".to_owned(),
                                node: "model.Shared.z2".to_owned(),
                            },
                        ],
                        target: DeclarationKey::fixture("model.Root.x"),
                    },
                    detail: "model.Shared declares 2 redefining members \
                              (model.Shared.z1, model.Shared.z2) that all redefine \
                              model.Root.x, with no single valid target"
                        .to_string(),
                }]
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/redefinition-target), got {other:?}")
        }
    }
}

/// Finds the effective member declared with original identity
/// `original_identity` under owner effective type `owner`, regardless of
/// its `visible` bit — phase 4 retains hidden entries in the view.
fn find_member<'a>(
    view: &'a quire_spec_language::model::normalize::EffectiveView,
    owner: &EffectiveId,
    original_identity: &str,
) -> &'a quire_spec_language::model::normalize::ViewEntry {
    view.declarations()
        .iter()
        .find(|entry| {
            entry.preimage.owner_effective_type.as_ref() == Some(owner)
                && entry.preimage.original.node == original_identity
        })
        .unwrap_or_else(|| panic!("no member {original_identity} owned by {owner:?} in {view:?}"))
}

fn completed(
    domain_package: &DomainPackage,
    limits: ModelNormalizationLimits,
) -> quire_spec_language::model::normalize::EffectiveView {
    match normalize(domain_package, limits) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected a completed view, got {other:?}"),
    }
}

/// Finds the type-level effective declaration with original identity
/// `original_identity` (an object type has no owner).
fn find_type<'a>(
    view: &'a quire_spec_language::model::normalize::EffectiveView,
    original_identity: &str,
) -> &'a quire_spec_language::model::normalize::ViewEntry {
    view.declarations()
        .iter()
        .find(|entry| {
            entry.preimage.owner_effective_type.is_none()
                && entry.preimage.original.node == original_identity
        })
        .unwrap_or_else(|| panic!("no type declaration {original_identity} in {view:?}"))
}

/// `identity` is exactly a 64-character lowercase hex SHA-256 digest.
fn is_sha256_hex(identity: &str) -> bool {
    identity.len() == 64
        && identity
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[trace("TC-195", "FR-150-AC-1", "FR-150-AC-3")]
#[test]
fn n01_normalizes_f1_to_stable_deterministic_identities() {
    let view = completed(&fixture_f1(), ModelNormalizationLimits::UNLIMITED);
    assert_eq!(view.declarations().len(), 4);

    let type_a = find_type(&view, "ix://test/orders/A");
    let type_b = find_type(&view, "ix://test/orders/B");
    assert_eq!(type_a.preimage.owner_effective_type, None);
    assert_eq!(type_a.preimage.derivation.len(), 1);
    assert_eq!(
        type_b.preimage.derivation.len(),
        2,
        "qualify then inherit [A]"
    );
    assert!(is_sha256_hex(&type_a.effective_id.to_string()));
    assert!(is_sha256_hex(&type_b.effective_id.to_string()));
    assert_ne!(type_a.effective_id, type_b.effective_id);

    let member_a_x = find_member(&view, &type_a.effective_id, "ix://test/orders/A/x");
    let member_b_x = find_member(&view, &type_b.effective_id, "ix://test/orders/A/x");
    assert_ne!(
        member_a_x.effective_id, member_b_x.effective_id,
        "the same original member owned by two distinct types has two distinct identities"
    );

    assert!(is_sha256_hex(&view.identity().to_string()));
    // Determinism: re-normalizing the identical fixture reproduces the exact
    // same view identity.
    let rerun = completed(&fixture_f1(), ModelNormalizationLimits::UNLIMITED);
    assert_eq!(view.identity(), rerun.identity());

    let universe = quire_spec_language::model::normalize::object_universe(&fixture_f1()).unwrap();
    assert_eq!(universe.root_types, vec![type_a.effective_id]);
    assert!(is_sha256_hex(&universe.identity().to_string()));
    let universe_rerun =
        quire_spec_language::model::normalize::object_universe(&fixture_f1()).unwrap();
    assert_eq!(universe.identity(), universe_rerun.identity());
}

/// TC-195 N02: `A`/`B`/`C`/`D` in F2's diamond (`B`, `C` <- `A`; `D` <- `B`,
/// `D` <- `C`), every declaration's identity checked for shape, uniqueness
/// and rerun-stability rather than against a pinned upstream digest.
#[trace("TC-195", "FR-150-AC-6")]
#[test]
fn n02_normalizes_f2_diamond_inheritance_to_stable_deterministic_identities() {
    let view = completed(&fixture_f2(), ModelNormalizationLimits::UNLIMITED);
    assert_eq!(view.declarations().len(), 8);

    let types =
        ["A", "B", "C", "D"].map(|name| find_type(&view, &format!("ix://test/orders/{name}")));
    for (index, entry) in types.iter().enumerate() {
        assert!(is_sha256_hex(&entry.effective_id.to_string()));
        for other in &types[index + 1..] {
            assert_ne!(entry.effective_id, other.effective_id);
        }
    }
    let [type_a, type_b, type_c, type_d] = types[..] else {
        unreachable!("exactly four types were looked up")
    };
    assert_eq!(
        type_d.preimage.derivation.len(),
        5,
        "qualify plus four inherit facts: [B], [B, A], [C], [C, A]"
    );

    let members = [type_a, type_b, type_c, type_d]
        .map(|owner| find_member(&view, &owner.effective_id, "ix://test/orders/A/x"));
    for (index, entry) in members.iter().enumerate() {
        for other in &members[index + 1..] {
            assert_ne!(
                entry.effective_id, other.effective_id,
                "the same original member under two distinct owners has two distinct identities"
            );
        }
    }
    let member_d_x = members[3];
    assert_eq!(
        member_d_x.preimage.derivation.len(),
        2,
        "both diamond paths [B, A, A/x] and [C, A, A/x] retained"
    );

    assert!(is_sha256_hex(&view.identity().to_string()));
    let rerun = completed(&fixture_f2(), ModelNormalizationLimits::UNLIMITED);
    assert_eq!(view.identity(), rerun.identity());

    let universe = quire_spec_language::model::normalize::object_universe(&fixture_f2()).unwrap();
    assert!(is_sha256_hex(&universe.identity().to_string()));
}

/// TC-195 N09's third clause: F1's nodes as version `2` under placeholder
/// selection `n01v2` yield declarations byte-equal to F1's own version-1
/// declarations (an effective declaration identity binds no
/// `ModelSelection`), but a different view and universe, because only the
/// model selection differs.
#[trace("TC-195", "FR-150-AC-6")]
#[test]
fn n01v2_a_version_only_change_reuses_declarations_but_changes_view_and_universe() {
    let v1 = completed(&fixture_f1(), ModelNormalizationLimits::UNLIMITED);
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture_with_version("n01v2", "2"),
        fixture_f1().records,
    );
    let view = completed(&domain_package, ModelNormalizationLimits::UNLIMITED);
    assert_eq!(view.declarations().len(), 4);

    // Every declaration is byte-equal to version 1's own: version binds
    // nothing about an effective declaration's own identity.
    let mut v1_ids: Vec<_> = v1
        .declarations()
        .iter()
        .map(|entry| entry.effective_id)
        .collect();
    let mut v2_ids: Vec<_> = view
        .declarations()
        .iter()
        .map(|entry| entry.effective_id)
        .collect();
    v1_ids.sort();
    v2_ids.sort();
    assert_eq!(v1_ids, v2_ids);

    assert!(is_sha256_hex(&view.identity().to_string()));
    let universe = quire_spec_language::model::normalize::object_universe(&domain_package).unwrap();
    assert!(is_sha256_hex(&universe.identity().to_string()));
    // Different from version 1's own view/universe: the model selection
    // changed.
    let universe_v1 =
        quire_spec_language::model::normalize::object_universe(&fixture_f1()).unwrap();
    assert_ne!(view.identity(), v1.identity());
    assert_ne!(universe.identity(), universe_v1.identity());
}

#[trace("TC-195", "FR-150-AC-4")]
#[test]
fn n07_record_order_does_not_affect_identity_or_view() {
    let ordered = completed(&fixture_f2(), ModelNormalizationLimits::UNLIMITED);
    let mut reversed = fixture_f2();
    reversed.records.reverse();
    let view = completed(&reversed, ModelNormalizationLimits::UNLIMITED);
    assert_eq!(view.identity(), ordered.identity());
}

#[trace("TC-195", "FR-150-AC-8")]
#[test]
fn n01_exact_limits_complete_and_the_charge_totals_match_ground_truth() {
    let exact = ModelNormalizationLimits {
        declaration_records: 3,
        derivation_facts: 5,
        effective_declarations: 4,
        dispatch_candidates: 0,
        hashed_bytes: 5311,
        work_units: 19,
    };
    let (outcome, meter) = normalize_with_meter(&fixture_f1(), exact);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 19);
    assert_eq!(meter.consumed(LimitKind::HashedBytes), 5311);
    assert_eq!(meter.consumed(LimitKind::DeclarationRecords), 3);
    assert_eq!(meter.consumed(LimitKind::DerivationFacts), 5);
    assert_eq!(meter.consumed(LimitKind::EffectiveDeclarations), 4);
    assert_eq!(
        meter.admitted_charges().last(),
        Some(&ChargePoint::NormalizeHash)
    );
}

#[trace("TC-195", "FR-150-AC-8")]
#[test]
fn n01_one_less_work_unit_is_incomplete_at_the_view_hash() {
    let mut limits = ModelNormalizationLimits {
        declaration_records: 3,
        derivation_facts: 5,
        effective_declarations: 4,
        dispatch_candidates: 0,
        hashed_bytes: 5311,
        work_units: 18,
    };
    match normalize(&fixture_f1(), limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.limit, 18);
            assert_eq!(incomplete.consumed, 18);
            assert_eq!(incomplete.next_charge, 1);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeHash);
        }
        other => panic!("expected Incomplete, got {other:?}"),
    }

    limits.work_units = 19;
    limits.hashed_bytes = 5310;
    match normalize(&fixture_f1(), limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::HashedBytes);
            assert_eq!(incomplete.limit, 5310);
            assert_eq!(incomplete.consumed, 2370);
            assert_eq!(incomplete.next_charge, 2941);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeHash);
        }
        other => panic!("expected Incomplete, got {other:?}"),
    }

    limits.hashed_bytes = 5311;
    limits.derivation_facts = 4;
    match normalize(&fixture_f1(), limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::DerivationFacts);
            assert_eq!(incomplete.limit, 4);
            assert_eq!(incomplete.consumed, 4);
            assert_eq!(incomplete.next_charge, 5);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeFact);
        }
        other => panic!("expected Incomplete, got {other:?}"),
    }
}

/// Real N05 (digest-domain mismatch, stale package digest) is a byte-level
/// intake check against the wire `model-effective-declaration.schema.json`:
/// `DomainPackage`'s fields are already typed Rust here, not decoded from
/// wire bytes, so this rung cannot reconstruct it. Remaining work: #131
/// wires a real Semantic IR 2.0.0 intake in front of `normalize`, where
/// N05's schema-level and package-digest checks belong.
#[trace("TC-195")]
#[test]
fn n09_effective_and_universe_identities_never_collide_with_the_model_selection_digest() {
    let view = completed(&fixture_f1(), ModelNormalizationLimits::UNLIMITED);
    let universe = quire_spec_language::model::normalize::object_universe(&fixture_f1()).unwrap();

    let selection_digest =
        quire_spec_language::model::key::hex(&fixture_f1().model_selection.digest);

    for entry in view.declarations() {
        assert_ne!(
            entry.effective_id.to_string(),
            selection_digest,
            "effective identity {} collided with the model selection digest",
            entry.effective_id
        );
    }
    assert_ne!(view.identity().to_string(), selection_digest);
    assert_ne!(universe.identity().to_string(), selection_digest);
}

#[trace("TC-195")]
#[test]
fn a_field_member_naming_an_undeclared_owner_refuses_instead_of_dropping() {
    let mut domain_package = fixture_f1();
    domain_package.records.push(field_member(
        "model.orphan.x",
        "model.no-such-type",
        "ix://test/orders/A",
    ));
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::DanglingReference
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::UnknownOwner {
                    member: DeclarationKey::fixture("model.orphan.x"),
                    owner: DeclarationKey::fixture("model.no-such-type"),
                }
            );
            assert!(refusal.detail.contains("model.orphan.x"));
            assert!(refusal.detail.contains("model.no-such-type"));
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

// `a_generalization_naming_an_undeclared_specific_refuses_instead_of_being_ignored`
// is dropped: under QSpec's inline `supertypes[]` property
// (`model-complete.md`:155), `specific` is always the owning
// `ObjectTypeRecord`'s own key, already indexed from that identical record --
// an "unknown specific" is structurally unreachable from
// `validate_references` (see its `ObjectType` arm's own comment).

#[trace("TC-195")]
#[test]
fn a_generalization_naming_an_undeclared_general_refuses_instead_of_panicking() {
    let mut domain_package = fixture_f1();
    domain_package
        .records
        .push(object_type("model.orphan", vec!["model.no-such-type"]));
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::DanglingReference
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::UnknownGeneral {
                    supertype: DeclarationKey::fixture("model.orphan"),
                    general: DeclarationKey::fixture("model.no-such-type"),
                }
            );
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

/// #196 review finding 3: model-complete.md's "Populations" row ("Each
/// member type names an object type or a process... `missing_declaration`/
/// `missing-name`"). A `Population` record naming a member type that is not
/// a declared object type refuses instead of being silently accepted.
#[trace("TC-195")]
#[test]
fn a_population_naming_an_undeclared_member_type_refuses_instead_of_being_ignored() {
    let mut domain_package = fixture_f1();
    domain_package.records.push(DomainPackageRecord::Population(
        quire_spec_language::model::domain_package::PopulationRecord {
            key: DeclarationKey::fixture("model.pop.p1"),
            member_types: vec![
                DeclarationKey::fixture("ix://test/orders/A"),
                DeclarationKey::fixture("model.no-such-type"),
            ],
            extent: quire_spec_language::model::domain_package::Extent::Closed,
        },
    ));
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal,
                ModelRefusal {
                    code: qsl_foundation::diagnostic::Code::MissingDeclaration,
                    cause: ModelRefusalCause::UnknownPopulationMemberType {
                        population: DeclarationKey::fixture("model.pop.p1"),
                        type_name: DeclarationKey::fixture("model.no-such-type"),
                    },
                    detail: "population model.pop.p1 names member type model.no-such-type, \
                              which is not a declared object type"
                        .to_string(),
                }
            );
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

/// A field member's own inline `subsets` property
/// (`model-complete.md`:161) naming a member that is not a declared field
/// or operation member refuses `dangling_reference`/`unknown-member`
/// instead of being silently accepted.
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn a_field_members_subsets_naming_an_undeclared_member_refuses_instead_of_dropping() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.subsets-dangling"),
        vec![
            object_type("model.A", vec![]),
            field_member("model.A.x", "model.A", "model.A"),
            field_member_redefining(
                "model.A.y",
                "model.A",
                "model.A",
                None,
                vec!["model.A.no-such-member"],
            ),
        ],
    );
    let expected = NormalizeOutcome::Refused(Refusals::from_vec(vec![ModelRefusal {
        code: qsl_foundation::diagnostic::Code::DanglingReference,
        cause: ModelRefusalCause::UnknownMember {
            record: DeclarationKey::fixture("model.A.y"),
            member: DeclarationKey::fixture("model.A.no-such-member"),
        },
        detail: "field member model.A.y subsets model.A.no-such-member, \
                 which is not a declared field or operation member"
            .to_owned(),
    }]));
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        expected
    );
}

/// An operation member's own inline `redefines` property
/// (`model-complete.md`:162) naming a member that is not a declared field
/// or operation member refuses `dangling_reference`/`unknown-member`
/// instead of being silently accepted -- the operation-member analog of
/// `n06_redefinition_target_absent_from_the_bundle_refuses_instead_of_dropping`,
/// which only covers a field member's own `redefines`.
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn an_operation_members_redefines_naming_an_undeclared_member_refuses_instead_of_dropping() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.operation-redefines-dangling"),
        vec![
            object_type("model.A", vec![]),
            operation_member_redefining("model.A.op", "model.A", Some("model.A.no-such-member")),
        ],
    );
    let expected = NormalizeOutcome::Refused(Refusals::from_vec(vec![ModelRefusal {
        code: qsl_foundation::diagnostic::Code::DanglingReference,
        cause: ModelRefusalCause::UnknownMember {
            record: DeclarationKey::fixture("model.A.op"),
            member: DeclarationKey::fixture("model.A.no-such-member"),
        },
        detail: "operation member model.A.op redefines model.A.no-such-member, \
                 which is not a declared field or operation member"
            .to_owned(),
    }]));
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        expected
    );
}

// Retagged (PR #144 review finding #1): this is the conflict refusal AC-3
// names ("a conflicting phase 4 derivation... refuses with its named
// cause") and, together, the perfect test for AC-5 ("conflicting
// derivations report both rule paths and expose no chosen effective
// member") — asserting the refusal names both contending redefiners' rule
// paths is exactly that. Not AC-1: nothing here links a normalized
// identity to its contributing declarations.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-5")]
#[test]
fn n06_two_undominated_redefiners_of_the_same_target_refuse_as_a_conflict() {
    match normalize(&fixture_n06_conflict(), ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::DerivationConflict {
                    type_: DeclarationKey::fixture("ix://test/orders/D"),
                    member: DeclarationKey::fixture("ix://test/orders/A/x"),
                    redefiners: vec![
                        DeclarationKey::fixture("model.B.x2"),
                        DeclarationKey::fixture("model.C.x3"),
                    ],
                }
            );
            assert!(refusal.detail.contains("ix://test/orders/B"));
            assert!(refusal.detail.contains("model.B.x2"));
            assert!(refusal.detail.contains("ix://test/orders/C"));
            assert!(refusal.detail.contains("model.C.x3"));
            assert!(refusal.detail.contains("ix://test/orders/A/x"));
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

/// QSL #145: `build`'s own `fact_budget_exceeded` gate skips phase-4
/// resolution entirely once phase 2/3's own fact budget is already
/// exhausted, so a truncated ancestor-path set can never derive a false
/// phase-4 refusal from partial data --
/// `charge_all`'s own replay of the phase 2/3 facts that triggered the
/// truncation always denies, in charge order, before phase 4's own charges
/// are ever consulted (FR-150's "exhaustion ends checking"). Under
/// `ModelNormalizationLimits::UNLIMITED`, `fixture_n06_conflict` refuses
/// `derivation-conflict` (the test above); under a tight `derivation_facts`
/// budget the outcome is `Incomplete` instead, at every point along the
/// sweep, never that refusal or the `redefinition-target` refusal a
/// still-tighter budget's truncated `D` would otherwise expose.
///
/// Revert-probe: removing the `if !fact_budget_exceeded(...)` guard around
/// the per-type phase-4 loop in `build()` (running `apply_redefinitions`
/// unconditionally again) turns both cases below back into their pre-fix
/// refusals — confirmed locally, then restored.
#[trace("TC-195", "FR-150-AC-8")]
#[test]
fn n06_conflict_under_a_tight_fact_budget_is_incomplete_not_a_phase4_refusal() {
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.derivation_facts = 10;
    match normalize(&fixture_n06_conflict(), limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::DerivationFacts);
            assert_eq!(incomplete.limit, 10);
            assert_eq!(incomplete.consumed, 10);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeFact);
        }
        other => panic!(
            "expected Incomplete at normalize.fact, not a phase-4 derivation-conflict refusal, got {other:?}"
        ),
    }

    limits.derivation_facts = 0;
    match normalize(&fixture_n06_conflict(), limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::DerivationFacts);
            assert_eq!(incomplete.limit, 0);
            assert_eq!(incomplete.consumed, 0);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeFact);
        }
        other => panic!(
            "expected Incomplete at normalize.fact, not a redefinition-target refusal, got {other:?}"
        ),
    }
}

/// TC-196 R07's second shape, run through `normalize`'s own phase 4
/// (`apply_redefinitions`) rather than `model::conformance`'s
/// `resolve_redefinition_target` in isolation — the real boundary a model
/// actually normalizes through. Two redefining members owned by the
/// identical type both redefine the same inherited target: refuses
/// `redefinition-target`, not `derivation-conflict` (N06's diamond shape
/// above uses two different owners, neither dominating the other).
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r07_two_redefiners_owned_by_the_same_type_refuse_redefinition_target_through_normalize() {
    match normalize(
        &fixture_r07_same_owner_contending_redefiners(),
        ModelNormalizationLimits::UNLIMITED,
    ) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::RedefinitionTarget {
                    redefiners: vec![
                        DeclarationKey::fixture("model.B.z"),
                        DeclarationKey::fixture("model.B.z2"),
                    ],
                    target: DeclarationKey::fixture("model.A.x"),
                }
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/redefinition-target), got {other:?}")
        }
    }
}

/// Re-review of PR #157's round-3 fix: the same-owner test must look only
/// at the *most-derived* owners among the undominated edges, not every
/// edge's owner. `C/w` (owner `C`) and `B/z`/`B/z2` (owner `B <= C`) all
/// redefine `A/x`: `C`'s edge is already dominated by `B`'s (either of
/// them), so it takes no part in the ambiguity test, leaving only `B`'s
/// two edges — one owner, refuses `redefinition-target`, not
/// `derivation-conflict`.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r07_a_less_derived_owners_redefiner_is_excluded_from_the_same_owner_test() {
    match normalize(
        &fixture_r07_dominated_owner_takes_no_part_in_the_same_owner_test(),
        ModelNormalizationLimits::UNLIMITED,
    ) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            // The typed `redefiners` list is exactly B's two edges -- C's
            // already-dominated `C.w` edge, checked equal here, takes no
            // part in the ambiguity.
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::RedefinitionTarget {
                    redefiners: vec![
                        DeclarationKey::fixture("model.B.z"),
                        DeclarationKey::fixture("model.B.z2"),
                    ],
                    target: DeclarationKey::fixture("model.A.x"),
                }
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/redefinition-target), got {other:?}")
        }
    }
}

// Retagged (PR #144 review finding #1): every assertion here checks that
// the winner's/hidden target's derivation facts link to every contributing
// original declaration and rule — AC-1's own language. It never checks
// replay/record-order independence, which is what AC-4 actually requires.
#[trace("TC-195", "FR-150-AC-1")]
#[test]
fn n06_a_strictly_more_derived_redefiner_resolves_the_conflict_and_hides_every_contender() {
    let view = completed(&fixture_n06_resolved(), ModelNormalizationLimits::UNLIMITED);

    // Type-level identities are unaffected by phase 4 (field-only): D's
    // identity is exactly N02's fixture_f2() type D.
    let type_d = find_type(&view, "ix://test/orders/D");
    let owner_d = type_d.effective_id;

    let winner = find_member(&view, &owner_d, "model.D.x4");
    assert!(
        winner.visible,
        "D.x4 must win outright: D is a descendant of both B and C"
    );
    let winner_redefine: Vec<_> = winner
        .preimage
        .derivation
        .iter()
        .filter(|fact| fact.rule == RULE_REDEFINE)
        .collect();
    assert_eq!(winner_redefine.len(), 1);
    assert_eq!(
        winner_redefine[0].inputs,
        vec![
            DeclarationKey::fixture("model.D.x4"),
            DeclarationKey::fixture("ix://test/orders/A/x"),
        ]
    );

    let target = find_member(&view, &owner_d, "ix://test/orders/A/x");
    assert!(!target.visible, "A.x is retained for provenance but hidden");
    let target_redefine: Vec<_> = target
        .preimage
        .derivation
        .iter()
        .filter(|fact| fact.rule == RULE_REDEFINE)
        .collect();
    assert_eq!(
        target_redefine.len(),
        3,
        "one redefine fact per competing redefiner: B, C, D"
    );
    assert_eq!(
        target_redefine[0].inputs,
        vec![
            DeclarationKey::fixture("ix://test/orders/B"),
            DeclarationKey::fixture("model.B.x2"),
            DeclarationKey::fixture("ix://test/orders/A/x"),
        ]
    );
    assert_eq!(
        target_redefine[1].inputs,
        vec![
            DeclarationKey::fixture("ix://test/orders/C"),
            DeclarationKey::fixture("model.C.x3"),
            DeclarationKey::fixture("ix://test/orders/A/x"),
        ]
    );
    assert_eq!(
        target_redefine[2].inputs,
        vec![
            DeclarationKey::fixture("model.D.x4"),
            DeclarationKey::fixture("ix://test/orders/A/x"),
        ]
    );

    let loser_b = find_member(&view, &owner_d, "model.B.x2");
    assert!(
        !loser_b.visible,
        "B.x2 loses to D.x4's more-derived redefinition"
    );
    let loser_b_redefine: Vec<_> = loser_b
        .preimage
        .derivation
        .iter()
        .filter(|fact| fact.rule == RULE_REDEFINE)
        .collect();
    assert_eq!(loser_b_redefine.len(), 1, "only B.x2's own edge, not D's");
    assert_eq!(
        loser_b_redefine[0].inputs,
        vec![
            DeclarationKey::fixture("ix://test/orders/B"),
            DeclarationKey::fixture("model.B.x2"),
            DeclarationKey::fixture("ix://test/orders/A/x"),
        ]
    );

    let loser_c = find_member(&view, &owner_d, "model.C.x3");
    assert!(
        !loser_c.visible,
        "C.x3 loses to D.x4's more-derived redefinition"
    );
    let loser_c_redefine: Vec<_> = loser_c
        .preimage
        .derivation
        .iter()
        .filter(|fact| fact.rule == RULE_REDEFINE)
        .collect();
    assert_eq!(loser_c_redefine.len(), 1, "only C.x3's own edge, not D's");
    assert_eq!(
        loser_c_redefine[0].inputs,
        vec![
            DeclarationKey::fixture("ix://test/orders/C"),
            DeclarationKey::fixture("model.C.x3"),
            DeclarationKey::fixture("ix://test/orders/A/x"),
        ]
    );
}

// A dangling redefinition target refuses with a named cause, closer to AC-3
// than to AC-1 (which is about linking a normalized identity to its
// contributing declarations — there is no normalized identity here at all,
// only a refusal). Under QSpec's inline `redefines` property
// (`model-complete.md`:162), the redefining side is always a real declared
// member (it IS the record), so only the redefined *target* can dangle;
// `record` in the refusal below names the redefining member's own key.
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn n06_redefinition_target_absent_from_the_bundle_refuses_instead_of_dropping() {
    let mut domain_package = fixture_f2();
    domain_package.records.push(field_member_redefining(
        "model.B.orphan",
        "ix://test/orders/B",
        "ix://test/orders/A",
        Some("model.A.no-such-member"),
        vec![],
    ));
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::DanglingReference
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::UnknownMember {
                    record: DeclarationKey::fixture("model.B.orphan"),
                    member: DeclarationKey::fixture("model.A.no-such-member"),
                }
            );
            assert!(refusal.detail.contains("model.A.no-such-member"));
        }
        other => panic!("expected Refused, got {other:?}"),
    }
}

/// FR-154: "Two nodes share one identity" refuses
/// `invalid_model_binding`/`conflicting-binding`. Retargets the pre-#131
/// `f2_producer_keys_sharing_an_identity_but_differing_in_revision_both_survive`
/// regression test: under the dropped `revision`/`digest` fields, two
/// `ObjectType` records that once differed only in `revision` now share the
/// exact same `DeclarationKey` and must refuse, never silently collapse to
/// one declaration.
#[trace("TC-195", "FR-150-AC-2")]
#[test]
fn two_object_types_sharing_one_declaration_key_refuse_conflicting_binding() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.conflict-object-type"),
        vec![
            object_type("model.T", vec![]),
            object_type("model.T", vec![]),
        ],
    );
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ConflictingBinding {
                    key: DeclarationKey::fixture("model.T"),
                }
            );
            assert!(refusal.detail.contains("model.T"));
        }
        other => {
            panic!("expected Refused(invalid_model_binding/conflicting-binding), got {other:?}")
        }
    }
}

/// FR-154's same "Two nodes share one identity" refusal, retargeting the
/// pre-#131 `f2_field_members_sharing_an_identity_but_differing_in_revision_both_survive`
/// regression test: two `FieldMember` records under the same owner that once
/// differed only in `revision` now share the exact same `DeclarationKey`.
#[trace("TC-195")]
#[test]
fn two_field_members_sharing_one_declaration_key_refuse_conflicting_binding() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.conflict-field-member"),
        vec![
            object_type("model.A", vec![]),
            field_member("model.A.x", "model.A", "model.A"),
            field_member("model.A.x", "model.A", "model.A"),
        ],
    );
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ConflictingBinding {
                    key: DeclarationKey::fixture("model.A.x"),
                }
            );
            assert!(refusal.detail.contains("model.A.x"));
        }
        other => {
            panic!("expected Refused(invalid_model_binding/conflicting-binding), got {other:?}")
        }
    }
}

/// FR-154: "Intake reports every declaration refusal in node order, and
/// then no later phase runs. Within one node, every check of the table that
/// fails reports, in table order." Both an *earlier* node's own
/// dangling-reference check (row 4, `missing-name` in FR-154's own
/// vocabulary -- `UnknownOwner` here) and a *later* node's own duplicate-key
/// check (row 6, `conflicting-binding`) are reported -- FR-154 reports every
/// refusal intake exposes, in node order, never only the earliest -- with
/// the earlier node's own refusal first.
#[trace("TC-195", "FR-154")]
#[test]
fn ordering_a_dangling_owner_at_an_earlier_node_reports_before_a_later_nodes_conflicting_binding() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.f2-order-dangling-first"),
        vec![
            field_member("model.A.y", "model.no-such-owner", "model.A.y"),
            object_type("model.B", vec![]),
            object_type("model.B", vec![]),
        ],
    );
    let expected = NormalizeOutcome::Refused(Refusals::from_vec(vec![
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::DanglingReference,
            cause: ModelRefusalCause::UnknownOwner {
                member: DeclarationKey::fixture("model.A.y"),
                owner: DeclarationKey::fixture("model.no-such-owner"),
            },
            detail: "field member model.A.y names owner model.no-such-owner, \
                     which is not a declared object type"
                .to_owned(),
        },
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
            cause: ModelRefusalCause::ConflictingBinding {
                key: DeclarationKey::fixture("model.B"),
            },
            detail: "model.B is declared by more than one record in this domain package".to_owned(),
        },
    ]));
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        expected
    );
}

/// The same fixture, records in the other order: `model.A.y` still sorts
/// before `model.B` (`"model.A.y" < "model.B"` as UTF-8 bytes, `model-complete.md`:73
/// "Nodes are read ascending by declaration key"), so `validate_references`
/// still reaches node `A.y`'s own dangling owner before either `B` record's
/// duplicate-key check, regardless of the records' own input order.
#[trace("TC-195", "FR-154")]
#[test]
fn ordering_input_record_order_does_not_change_which_sorted_node_wins() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.f2-order-conflict-first"),
        vec![
            object_type("model.B", vec![]),
            object_type("model.B", vec![]),
            field_member("model.A.y", "model.no-such-owner", "model.A.y"),
        ],
    );
    let expected = NormalizeOutcome::Refused(Refusals::from_vec(vec![
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::DanglingReference,
            cause: ModelRefusalCause::UnknownOwner {
                member: DeclarationKey::fixture("model.A.y"),
                owner: DeclarationKey::fixture("model.no-such-owner"),
            },
            detail: "field member model.A.y names owner model.no-such-owner, \
                     which is not a declared object type"
                .to_owned(),
        },
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
            cause: ModelRefusalCause::ConflictingBinding {
                key: DeclarationKey::fixture("model.B"),
            },
            detail: "model.B is declared by more than one record in this domain package".to_owned(),
        },
    ]));
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        expected
    );
}

/// A conflicting key that sorts *before* a dangling reference elsewhere:
/// two `model.A` records plus a dangling owner at `model.Z.y`. `"model.A" <
/// "model.Z.y"` as UTF-8 bytes, so node `A`'s own duplicate-key check
/// reports first in every input order, unlike the two tests above where the
/// dangling node happens to sort first; `Z.y`'s own dangling-owner refusal
/// is still reported after it (FR-154: every refusal, in node order).
#[trace("TC-195", "FR-154")]
#[test]
fn ordering_a_conflicting_binding_at_an_earlier_node_reports_before_a_later_nodes_dangling_owner() {
    let expected = NormalizeOutcome::Refused(Refusals::from_vec(vec![
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
            cause: ModelRefusalCause::ConflictingBinding {
                key: DeclarationKey::fixture("model.A"),
            },
            detail: "model.A is declared by more than one record in this domain package".to_owned(),
        },
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::DanglingReference,
            cause: ModelRefusalCause::UnknownOwner {
                member: DeclarationKey::fixture("model.Z.y"),
                owner: DeclarationKey::fixture("model.no-such-owner"),
            },
            detail: "field member model.Z.y names owner model.no-such-owner, \
                     which is not a declared object type"
                .to_owned(),
        },
    ]));

    let dangling_first = DomainPackage::new(
        DomainPackageRef::fixture("bundle.f2-conflict-sorts-first-a"),
        vec![
            field_member("model.Z.y", "model.no-such-owner", "model.Z.y"),
            object_type("model.A", vec![]),
            object_type("model.A", vec![]),
        ],
    );
    assert_eq!(
        normalize(&dangling_first, ModelNormalizationLimits::UNLIMITED),
        expected
    );

    let conflict_first = DomainPackage::new(
        DomainPackageRef::fixture("bundle.f2-conflict-sorts-first-b"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.A", vec![]),
            field_member("model.Z.y", "model.no-such-owner", "model.Z.y"),
        ],
    );
    assert_eq!(
        normalize(&conflict_first, ModelNormalizationLimits::UNLIMITED),
        expected
    );
}

/// FR-321: "Missing required properties, duplicate keys, out-of-domain
/// values and non-canonical encodings refuse before consumption." An empty
/// `package`/`node`/`identity`/`version` string is schema `minLength`-invalid
/// (out of domain) and refuses `invalid_model_binding`/`malformed-declaration`.
/// Retargets the deleted pre-#131
/// `n04_absent_revision_refuses_wrong_model_selection` regression test (its
/// own empty-`revision` check no longer applies to the flat `DeclarationKey`)
/// onto the phase-1 empty-component check this restores in
/// `validate_references`.
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn n04_empty_declaration_key_component_refuses_malformed_declaration() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.n04-empty-node"),
        vec![object_type("", vec![])],
    );
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(refusal.cause, ModelRefusalCause::MalformedDeclaration);
        }
        other => {
            panic!("expected Refused(invalid_model_binding/malformed-declaration), got {other:?}")
        }
    }
}

/// The same phase-1 check, over an empty domain package `identity`.
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn n04_empty_model_selection_identity_refuses_malformed_declaration() {
    let mut model_selection = DomainPackageRef::fixture("bundle.n04-empty-identity");
    model_selection.identity.clear();
    let domain_package = DomainPackage::new(model_selection, vec![object_type("model.A", vec![])]);
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(refusal.cause, ModelRefusalCause::MalformedDeclaration);
        }
        other => {
            panic!("expected Refused(invalid_model_binding/malformed-declaration), got {other:?}")
        }
    }
}

/// The same phase-1 check, over an empty domain package `version`.
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn n04_empty_model_selection_version_refuses_malformed_declaration() {
    let mut model_selection = DomainPackageRef::fixture("bundle.n04-empty-version");
    model_selection.version.clear();
    let domain_package = DomainPackage::new(
        model_selection.clone(),
        vec![object_type("model.A", vec![])],
    );
    let expected = NormalizeOutcome::Refused(Refusals::from_vec(vec![ModelRefusal {
        code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
        cause: ModelRefusalCause::MalformedDeclaration,
        detail: format!(
            "domain package selection has an empty identity or version: {model_selection:?}"
        ),
    }]));
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        expected
    );
}

/// The same phase-1 check, over a `DeclarationKey` with an empty `package`.
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn n04_empty_declaration_key_package_refuses_malformed_declaration() {
    let key = DeclarationKey {
        package: String::new(),
        node: "model.A".to_owned(),
    };
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.n04-empty-package"),
        vec![DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: key.clone(),
            interface_features: None,
            abstract_type: false,
            supertypes: Vec::new(),
        })],
    );
    let expected = NormalizeOutcome::Refused(Refusals::from_vec(vec![ModelRefusal {
        code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
        cause: ModelRefusalCause::MalformedDeclaration,
        detail: format!("declaration key has an empty package or node: {key:?}"),
    }]));
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        expected
    );
}

/// `model-complete.md`:81: within a node, the malformed-declaration check is
/// that node's own first check -- ahead of any dangling-reference check for
/// that same node. This pins the axis across two different nodes: an
/// earlier-sorting node (`test/orders`/`A.y`) has a genuine dangling owner,
/// a later-sorting node (`zzz.package`/empty `node`) is itself malformed.
/// The earlier node's own dangling-owner refusal is reported first, since
/// `validate_references` reads nodes ascending by declaration key (H1,
/// `model-complete.md`:73); the later node's own malformed-declaration
/// refusal is still reported after it (FR-154: every refusal, in node
/// order).
#[trace("TC-195", "FR-150-AC-3", "FR-154")]
#[test]
fn n04_an_earlier_nodes_dangling_owner_outranks_a_later_nodes_malformed_key() {
    let malformed_key = DeclarationKey {
        package: "zzz.package".to_owned(),
        node: String::new(),
    };
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.n04-earlier-dangling-later-malformed"),
        vec![
            field_member("model.A.y", "model.no-such-owner", "model.A.y"),
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: malformed_key.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: Vec::new(),
            }),
        ],
    );
    let expected = NormalizeOutcome::Refused(Refusals::from_vec(vec![
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::DanglingReference,
            cause: ModelRefusalCause::UnknownOwner {
                member: DeclarationKey::fixture("model.A.y"),
                owner: DeclarationKey::fixture("model.no-such-owner"),
            },
            detail: "field member model.A.y names owner model.no-such-owner, \
                     which is not a declared object type"
                .to_owned(),
        },
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
            cause: ModelRefusalCause::MalformedDeclaration,
            detail: format!("declaration key has an empty package or node: {malformed_key:?}"),
        },
    ]));
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        expected
    );
}

/// TC-195 N08 (`spec/test-cases/TC-195-model-normalization-provenance.md`:62):
/// "after three `normalize.record` charges, two refusals in node order and
/// no effective view." This crate's `FieldMemberRecord` has no `presence`
/// property to reproduce N08's own first refusal byte-for-byte, so this
/// pins the same shape over two nodes this crate's own checks refuse
/// instead: every `normalize.record` charge admits before intake's own
/// refusals are ever reported (`Built::intake_refusals`'s own doc), and
/// every node's own refusal is collected, in node order -- `model.A.y`'s
/// dangling owner (`UnknownOwner`) and `model.B`'s dangling generalization
/// (`UnknownGeneral`). `"model.A.y" < "model.B"` as UTF-8 bytes
/// (`model-complete.md`:73), so `A.y`'s own refusal is collected first.
/// `work_units = 1` pins the exact boundary: only the first of the two
/// `normalize.record` charges (`value-accounting.md`:489) admits, so intake
/// is never consulted and neither refusal is ever reported; `work_units =
/// 2` admits both charges and reports both refusals.
#[trace("TC-195", "FR-150-AC-3", "FR-154")]
#[test]
fn n08_every_normalize_record_charge_admits_before_intake_reports_both_refusals_in_node_order() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.n08-two-intake-refusals"),
        vec![
            field_member("model.A.y", "model.no-such-owner", "model.A.y"),
            object_type("model.B", vec!["model.no-such-type"]),
        ],
    );

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 1;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 1,
                    consumed: 1,
                    next_charge: 1,
                    charge_point: ChargePoint::NormalizeRecord,
                }
            );
        }
        other => panic!("expected Incomplete at the second normalize.record charge, got {other:?}"),
    }

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 2;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                vec![
                    ModelRefusal {
                        code: qsl_foundation::diagnostic::Code::DanglingReference,
                        cause: ModelRefusalCause::UnknownOwner {
                            member: DeclarationKey::fixture("model.A.y"),
                            owner: DeclarationKey::fixture("model.no-such-owner"),
                        },
                        detail: "field member model.A.y names owner model.no-such-owner, \
                                 which is not a declared object type"
                            .to_owned(),
                    },
                    ModelRefusal {
                        code: qsl_foundation::diagnostic::Code::DanglingReference,
                        cause: ModelRefusalCause::UnknownGeneral {
                            supertype: DeclarationKey::fixture("model.B"),
                            general: DeclarationKey::fixture("model.no-such-type"),
                        },
                        detail: "object type model.B names supertype model.no-such-type, \
                                 which is not a declared object type"
                            .to_owned(),
                    },
                ]
            );
        }
        other => {
            panic!("expected Refused([UnknownOwner for A.y, UnknownGeneral for B]), got {other:?}")
        }
    }
}

/// A root type plus seven levels of two distinct types apiece (15 records
/// total, one `ObjectType` per type), each level's own pair both
/// generalizing to *both* of the previous level's types via inline
/// `supertypes[]` (`model-complete.md`:155) -- a genuine diamond ladder.
/// This replaces a prior `vec![&prev, &prev]` shape that named one ancestor
/// twice inside a single record's own `supertypes`, which QSpec never
/// defines: a type's `supertypes[]` names its distinct generals, never the
/// same general listed more than once. Doubling the live ancestor ladder at
/// every level reproduces the same diamond/parallel-generalization blowup
/// the original review found (unbounded `ancestor_paths` took ~51s over an
/// equivalent record count). Under a tight budget this must return a typed
/// `Incomplete` quickly rather than enumerate every path first.
fn fixture_deep_parallel_generalization_chain() -> DomainPackage {
    let mut records: Vec<DomainPackageRecord> = vec![object_type("model.T0", vec![])];
    let mut previous_level = vec!["model.T0".to_owned()];
    for level in 1..=7 {
        let current_level: Vec<String> = ["a", "b"]
            .into_iter()
            .map(|branch| format!("model.T{level}{branch}"))
            .collect();
        let supertypes: Vec<&str> = previous_level.iter().map(String::as_str).collect();
        for type_name in &current_level {
            records.push(object_type(type_name, supertypes.clone()));
        }
        previous_level = current_level;
    }
    assert_eq!(
        records.len(),
        15,
        "one root type plus seven levels of two distinct types apiece"
    );
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.deep-parallel-chain"),
        records,
    )
}

#[trace("TC-195")]
#[test]
fn f1_deep_parallel_generalization_bounds_enumeration_instead_of_exploding() {
    let domain_package = fixture_deep_parallel_generalization_chain();
    let tight = ModelNormalizationLimits {
        declaration_records: 15,
        derivation_facts: 1,
        effective_declarations: 1,
        dispatch_candidates: 0,
        hashed_bytes: 1,
        work_units: 1,
    };
    let start = std::time::Instant::now();
    let outcome = normalize(&domain_package, tight);
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "normalize took {elapsed:?} under a saturated budget; ancestor-path \
         enumeration is not bounded during the walk (F1 regression)"
    );
    match outcome {
        NormalizeOutcome::Incomplete(_) => {}
        other => panic!("expected a typed Incomplete under limits of 1, got {other:?}"),
    }
}

// Real N04 (wrong model selection) is a byte-level intake check against
// the admitted package's own `ModelSelection` identity/version, run before
// a caller builds a typed `DomainPackage` -- see the `normalize` module
// doc comment. Remaining work: #131 wires a real Semantic IR 2.0.0 intake
// in front of `normalize`, where N04 belongs.

/// PR #140 F5 / TC-195 N10: the three `invalid_mutations` named "refused by
/// the semantic check" over an already-constructed effective declaration or
/// view. The other three N10 mutations (`stale-digest`,
/// `cross-domain-producer-digest`, `owner-as-producer-key`) are "refused by
/// schema" against the wire `model-effective-declaration.schema.json` this
/// rung does not decode from wire bytes (`DomainPackage`'s fields are already typed
/// Rust, not JSON); they are honestly uncovered here for that reason.
#[trace("TC-195")]
#[test]
fn n10_unsorted_derivation_refuses_by_the_semantic_check() {
    let view = completed(&fixture_f1(), ModelNormalizationLimits::UNLIMITED);
    let type_b = find_type(&view, "ix://test/orders/B");
    let mut mutated = type_b.preimage.clone();
    assert!(
        mutated.derivation.len() >= 2,
        "type B has a qualify fact plus at least one inherit fact"
    );
    mutated.derivation.swap(0, 1);
    let (cause, _detail) = mutated
        .validate_derivation()
        .expect_err("a derivation whose ordinals no longer match array position must be refused");
    assert_eq!(
        cause,
        ModelRefusalCause::UnsortedDerivation {
            original: mutated.original.clone(),
            position: 0,
            ordinal: 1,
        }
    );
}

#[trace("TC-195")]
#[test]
fn n10_duplicate_path_refuses_by_the_semantic_check() {
    let view = completed(&fixture_f2(), ModelNormalizationLimits::UNLIMITED);
    let type_d = find_type(&view, "ix://test/orders/D");
    let member_d_x = find_member(&view, &type_d.effective_id, "ix://test/orders/A/x");
    let mut mutated = member_d_x.preimage.clone();
    assert_eq!(mutated.derivation.len(), 2, "both diamond paths retained");
    let duplicate_inputs = mutated.derivation[0].inputs.clone();
    mutated.derivation[1].inputs = duplicate_inputs;
    let (cause, _detail) = mutated
        .validate_derivation()
        .expect_err("a derivation retaining the same input path twice must be refused");
    assert_eq!(
        cause,
        ModelRefusalCause::DuplicatePath {
            original: mutated.original.clone(),
            earlier: 0,
            later: 1,
        }
    );
}

// TC-195 N10 (a view whose declarations are not ascending by effective
// identity refuses `invalid_model_binding`/`unsorted-view`) is now a unit
// test, `crate::model::normalize::tests::n10_unsorted_view_refuses_by_the_semantic_check`:
// `EffectiveView`'s fields are private outside `crate::model::normalize`
// (#151: construction only through `normalize`), so an out-of-crate
// integration test can no longer build a deliberately unsorted view to
// exercise `validate_order` directly.

/// TC-195 N01's exact charge sequence and per-declaration JCS lengths —
/// three `normalize.record` (`A`, `A/x`, `B`; the `Supertype` record `B` ->
/// `A` is a relationship between declarations, not one of its own); three
/// phase-2 `normalize.fact` (qualify A, qualify B, qualify A.x); one
/// `normalize.cycle-check` (`L = 1`) then a phase-3 `normalize.fact`
/// (inherit B, inputs `[A]`); one more phase-3 `normalize.fact` (inherit
/// `(B, A/x)`, inputs `[A, A/x]`, no cycle-check — cycle-check is charged
/// only for phase-3 type-level facts); four `(normalize.declaration,
/// normalize.hash)` pairs with JCS lengths 375 (A), 563 (B), 500 (A.x owned
/// by A), 583 (A.x inherited by B); then `normalize.hash` of the universe
/// (349) and of the view (2941) — 5311 hashed bytes and 19 work units total,
/// verified against the running preimage's own `jcs_bytes()`, not just
/// against the meter's own bookkeeping.
#[trace("TC-195", "FR-150-AC-1", "FR-150-AC-8")]
#[test]
fn n01_charges_the_exact_ground_truth_sequence_in_order() {
    let (outcome, meter) = normalize_with_meter(&fixture_f1(), ModelNormalizationLimits::UNLIMITED);
    let view = match outcome {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected Completed, got {other:?}"),
    };

    use ChargePoint::{
        NormalizeCycleCheck, NormalizeDeclaration, NormalizeFact, NormalizeHash, NormalizeRecord,
    };
    let expected = vec![
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
    ];
    assert_eq!(meter.admitted_charges().to_vec(), expected);

    let type_a = find_type(&view, "ix://test/orders/A");
    let type_b = find_type(&view, "ix://test/orders/B");
    let member_a_x = find_member(&view, &type_a.effective_id, "ix://test/orders/A/x");
    let member_b_x = find_member(&view, &type_b.effective_id, "ix://test/orders/A/x");
    assert_eq!(type_a.preimage.jcs_bytes().len(), 375);
    assert_eq!(type_b.preimage.jcs_bytes().len(), 563);
    assert_eq!(member_a_x.preimage.jcs_bytes().len(), 500);
    assert_eq!(member_b_x.preimage.jcs_bytes().len(), 583);

    let universe = quire_spec_language::model::normalize::object_universe(&fixture_f1()).unwrap();
    assert_eq!(universe.jcs_bytes().len(), 349);
    assert_eq!(view.jcs_bytes().len(), 2941);

    assert_eq!(meter.consumed(LimitKind::HashedBytes), 5311);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 19);
}

/// A member's own inline `subsets` property adds no extra `normalize.record`
/// charge -- `value-accounting.md:489` charges one `normalize.record` per IR
/// node, and `normalize.record` here charges exactly the domain package's
/// own four declared records (`A`, `A.x`, `B`, `B.y`; `records.len()`).
/// `model.B.y`'s `subsets: ["model.A.x"]` names the relationship inline on
/// `B.y`'s own record, admitting no separate record of its own.
#[trace("TC-195", "FR-150-AC-1")]
#[test]
fn f1_a_members_own_subsets_property_charges_no_extra_normalize_record() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.f1-subsets-charge"),
        vec![
            object_type("model.A", vec![]),
            field_member("model.A.x", "model.A", "model.A"),
            object_type("model.B", vec![]),
            field_member_redefining("model.B.y", "model.B", "model.A", None, vec!["model.A.x"]),
        ],
    );
    let (outcome, meter) =
        normalize_with_meter(&domain_package, ModelNormalizationLimits::UNLIMITED);
    match outcome {
        NormalizeOutcome::Completed(_) => {}
        other => panic!("expected Completed, got {other:?}"),
    }
    use ChargePoint::{NormalizeDeclaration, NormalizeFact, NormalizeHash, NormalizeRecord};
    let expected = vec![
        NormalizeRecord,
        NormalizeRecord,
        NormalizeRecord,
        NormalizeRecord,
        NormalizeFact,
        NormalizeFact,
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
    ];
    assert_eq!(meter.admitted_charges().to_vec(), expected);
    let record_charges = meter
        .admitted_charges()
        .iter()
        .filter(|c| **c == ChargePoint::NormalizeRecord)
        .count();
    assert_eq!(record_charges, domain_package.records.len());
    assert_eq!(record_charges, 4);
}

/// TC-195 N02: five `normalize.record` (`A`, `B`, `C`, `D`, `A/x`; F2's four
/// generalization edges are inline `supertypes[]` entries on their owning
/// `ObjectType` records, not declarations of their own), fifteen
/// `normalize.fact` charges (five phase-2 qualify facts for A, B, C, D and
/// A.x; ten phase-3 inherit facts) and six `normalize.cycle-check` charges
/// (one per phase-3 type-level ancestor path: `B`->`A`, `C`->`A`, `D`->`B`,
/// `D`->`C` via `B`, `D`->`C` via `C`'s own two-hop paths — `L` = 1 for `B`,
/// 1 for `C`, then 1, 2, 1 and 2 for `D`'s four paths), ten
/// `(normalize.declaration, normalize.hash)` pairs (5482 bytes), the
/// universe (349) and the view (7022) — forty-six work units and 12853
/// hashed bytes.
#[trace("TC-195", "FR-150-AC-4", "FR-150-AC-6")]
#[test]
fn n02_charges_fifteen_facts_and_six_cycle_checks() {
    let (outcome, meter) = normalize_with_meter(&fixture_f2(), ModelNormalizationLimits::UNLIMITED);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));

    let admitted = meter.admitted_charges();
    let record_count = admitted
        .iter()
        .filter(|point| **point == ChargePoint::NormalizeRecord)
        .count();
    let fact_count = admitted
        .iter()
        .filter(|point| **point == ChargePoint::NormalizeFact)
        .count();
    let cycle_check_count = admitted
        .iter()
        .filter(|point| **point == ChargePoint::NormalizeCycleCheck)
        .count();
    assert_eq!(record_count, 5, "A, B, C, D and A/x, no Supertype record");
    assert_eq!(fact_count, 15, "five qualify plus ten inherit facts");
    assert_eq!(
        cycle_check_count, 6,
        "one cycle-check per phase-3 type-level ancestor path"
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 46);
    assert_eq!(meter.consumed(LimitKind::HashedBytes), 12853);
}

/// TC-196 R01: a closing generalization cycle (`model.A` -> `model.B` ->
/// `model.A`) refuses `invalid_model_binding`/`specialization-cycle` naming
/// every contributing declaration in the cycle, rotated to start at its
/// least key (`model.A`), regardless of which type's own walk closes it
/// first — see `src/model/normalize.rs`'s module docs for this rung's own
/// recorded scope decision (the refusal's shape is reproduced exactly; the
/// exact six-`normalize.cycle-check`-charge accounting across both types'
/// walks is not).
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r01_a_closing_generalization_cycle_names_the_full_rotated_chain() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.r01"),
        vec![
            object_type("model.A", vec!["model.B"]),
            object_type("model.B", vec!["model.A"]),
        ],
    );
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::SpecializationCycle {
                    ancestor: DeclarationKey::fixture("model.A"),
                    via: DeclarationKey::fixture("model.B"),
                }
            );
            assert!(
                refusal.detail.contains("[model.A, model.B]"),
                "the chain must be rotated to start at its least key regardless of \
                 which type's own walk closes the cycle first, got: {}",
                refusal.detail
            );
            assert!(refusal.detail.contains("generalizes back to itself via"));
        }
        other => {
            panic!("expected Refused(invalid_model_binding/specialization-cycle), got {other:?}")
        }
    }
}

/// TC-196 R01, the prefix-plus-rotation shape review found the R01 test above
/// cannot catch: `model.A` -> `model.C`, `model.C` -> `model.B`,
/// `model.B` -> `model.C`. `model.A` is not part of the cycle at all — it is
/// only how the walk *reaches* it — so the listing must name just the cycle
/// itself, `[model.B, model.C]`, never the whole path from the walk's root
/// (`[model.A, model.C, model.B]`).
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn r01b_the_cycle_listing_excludes_a_type_that_only_leads_into_it() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.r01b"),
        vec![
            object_type("model.A", vec!["model.C"]),
            object_type("model.B", vec!["model.C"]),
            object_type("model.C", vec!["model.B"]),
        ],
    );
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::SpecializationCycle {
                    ancestor: DeclarationKey::fixture("model.C"),
                    via: DeclarationKey::fixture("model.B"),
                }
            );
            assert!(
                refusal.detail.contains("[model.B, model.C]"),
                "the listing must name only the cycle itself, excluding model.A, \
                 which only leads into it, got: {}",
                refusal.detail
            );
            assert!(
                !refusal.detail.contains("model.A"),
                "model.A is not part of the cycle and must not be named, got: {}",
                refusal.detail
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/specialization-cycle), got {other:?}")
        }
    }
}

/// F2 plus a single, uncontested redefiner owned by `B2` (`B2 <= A`,
/// `B2.x2 redefines A.x`) -- exactly
/// `fixture_n06_resolved`'s own diamond, reused so the exact `m + r` here
/// (`value-accounting.md:455`) is cross-checked against N06's own already
/// hand-verified `f(o)`/`m` values below, not derived from a fresh fixture.
fn fixture_single_redefiner_no_conflict() -> DomainPackage {
    let mut records = fixture_f1().records;
    records.push(object_type("model.B2", vec!["ix://test/orders/A"]));
    records.push(field_member_redefining(
        "model.B2.x2",
        "model.B2",
        "ix://test/orders/A",
        Some("ix://test/orders/A/x"),
        vec![],
    ));
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.single-redefiner"),
        records,
    )
}

/// `Owner` declares `n_parents` direct generalizations (one to `Base`, the
/// rest to unrelated, ancestor-less filler types) and a single field
/// (`Owner.x2 redefines Base.x`) with no other redefiner contesting
/// `Base.x` -- an uncontested redefinition group (`edges.len() == 1`) whose
/// owner's own breadth alone, before this fix, was enough to exceed
/// `MAX_CONFORMANCE_DEPTH` (128) computing a dominance closure no single
/// redefiner ever needs.
fn fixture_wide_ancestry_single_redefiner(n_parents: usize) -> DomainPackage {
    let parent_ids: Vec<String> = (0..n_parents.saturating_sub(1))
        .map(|i| format!("model.P{i}"))
        .collect();
    let mut records = vec![
        object_type("model.Base", vec![]),
        field_member("model.Base.x", "model.Base", "model.Base"),
    ];
    for id in &parent_ids {
        records.push(object_type(id, vec![]));
    }
    let mut owner_supertypes: Vec<&str> = vec!["model.Base"];
    owner_supertypes.extend(parent_ids.iter().map(String::as_str));
    records.push(object_type("model.Owner", owner_supertypes));
    records.push(field_member_redefining(
        "model.Owner.x2",
        "model.Owner",
        "model.Base",
        Some("model.Base.x"),
        vec![],
    ));
    DomainPackage::new(
        DomainPackageRef::fixture(format!("bundle.wide-{n_parents}")),
        records,
    )
}

/// TC-195 N06's own resolved diamond (`fixture_n06_resolved`, three
/// redefiners of `A.x` -- `B`, `C`, and the
/// dominating winner `D` -- own owners `B`/`C`/`D`) charges exactly one
/// `normalize.conflict-check` at `Σ (c − 1) × f(o)`
/// (`value-accounting.md:456`), not a flat one work unit per group:
///
/// - `c = 3` (three redefiners of `A.x` reachable at `D`'s own effective
///   view: `redef.B`, `redef.C`, `redef.D`).
/// - `f(B) = 2`: `B`'s own type-level derivation is one `qualify` fact plus
///   one `inherit` fact for its single ancestor path `B -> A`.
/// - `f(C) = 2`: symmetric to `B`, one ancestor path `C -> A`.
/// - `f(D) = 5`: one `qualify` fact plus four `inherit` facts, one per
///   distinct ancestor path (`D -> B`, `D -> C`, `D -> A` via `B`, `D -> A`
///   via `C` -- the same four paths TC-195 N02's own ground truth charges
///   four of its six `normalize.cycle-check` charges against).
/// - `Σ (c − 1) × f(o) = (3 − 1) × (f(B) + f(C) + f(D)) = 2 × (2 + 2 + 5)
///   = 18`.
///
/// `normalize.redefinition-check` (`value-accounting.md:455`) charges
/// `m + r` once per redefinition record in the whole domain package, ascending by
/// the record's own producer key -- never once per (record, effective type
/// reaching it) pair (QSL #145): `redef.B`
/// (`r = 0`, `m(B) = 2`) charges `2`; `redef.C` (`r = 1`, `m(C) = 2`)
/// charges `3`; `redef.D` (`r = 2`, `m(D) = 4`: `D`'s own effective members
/// are `A.x`, `B.x2`, `C.x3`, all inherited, plus its own direct `D.x4`)
/// charges `6`. Three charges, `2 + 3 + 6 = 11` total -- not five charges
/// re-examining `redef.B`/`redef.C` a second time at `D`'s own pass, which
/// is what a per-(type, record) charge wrongly did before this fix.
///
/// Cross-checked against the crate by running it directly: with every
/// other limit unlimited, `work_units = 47` (36 for every phase-2/3
/// `normalize.record`/`normalize.fact`/`normalize.cycle-check` charge --
/// `normalize.record` charges only this fixture's eight declaration
/// records, never its seven `Supertype`/`Redefinition` records, which state
/// relationships, not declarations of their own -- plus the three
/// `normalize.redefinition-check` charges' own `2 + 3 + 6 = 11` work) is
/// exactly enough to admit every charge up to and including the last
/// `normalize.redefinition-check`. It no longer denies at
/// `normalize.conflict-check` there (QSL #169): ten phase-4 redefine facts
/// -- two per redefinition record reaching the type that resolves it,
/// `2` at `B` + `2` at `C` + `6` at `D` (see
/// `n06_redefine_facts_are_charged_as_normalize_fact_between_the_two_phase4_checks`
/// below for the full count) -- are now charged as `normalize.fact` first,
/// so `47` denies at the first of those instead. `work_units = 57`
/// (`47 + 10`) is the boundary that now lands exactly before
/// `normalize.conflict-check`, whose reported `next_charge` is still
/// exactly `18`.
///
/// Revert probe: reverting the `Σ (c − 1) × f(o)` charge back to a flat,
/// unconditional `Charge::new(ChargePoint::NormalizeConflictCheck)` (no
/// `.work(...)` override, i.e. PR #167's own pre-fix shape) makes both
/// assertions below fail -- the exact-bound one because `work_units = 57`
/// then completes outright (a flat charge of 1 fits), and the total
/// because `103` no longer matches. Confirmed by hand: reintroducing that
/// exact one-line regression locally reproduces both failures, then
/// removing it again restores this test to green.
#[trace("TC-195", "TC-196", "FR-150-AC-8", "FR-151-AC-2")]
#[test]
fn n06_conflict_check_charges_exactly_sigma_c_minus_1_times_f_o() {
    let domain_package = fixture_n06_resolved();

    let (outcome, meter) =
        normalize_with_meter(&domain_package, ModelNormalizationLimits::UNLIMITED);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeRedefinitionCheck)
            .count(),
        3,
        "one normalize.redefinition-check per redefinition record in the \
         whole domain_package -- redef.B, redef.C and redef.D each examined exactly \
         once, ascending by the record's own producer key, never once per \
         (record, effective type reaching it) pair"
    );
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeConflictCheck)
            .count(),
        1,
        "exactly one contested group (A.x, reachable at D) across the whole build"
    );
    // Includes the ten phase-4 redefine facts' own `normalize.fact` charges
    // (see `n06_redefine_facts_are_charged_as_normalize_fact_between_the_two_phase4_checks`
    // for their derivation).
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 103);

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 57;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.limit, 57);
            assert_eq!(incomplete.consumed, 57);
            assert_eq!(incomplete.next_charge, 18);
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeConflictCheck);
        }
        other => panic!("expected Incomplete at normalize.conflict-check, got {other:?}"),
    }
}

/// Phase-4 redefine facts (`RULE_REDEFINE`) are themselves derivation facts
/// and are charged as `normalize.fact`
/// (`value-accounting.md:453`: "each derivation fact before it is formed:
/// phase 2, then phase 3, then phase 4 ... `derivation_facts=k`"), in the
/// order `value-accounting.md:455`/`:456` fix relative to phase 4's own two
/// checks: `normalize.redefinition-check` fires "before its first
/// `normalize.fact`" and `normalize.conflict-check` fires "after its last
/// `normalize.fact`" -- so the phase-4 charge sequence is
/// redefinition-check, then every phase-4 `normalize.fact` (one charge per
/// fact, continuing the same `derivation_facts` running total phase 2/3
/// already charge), then conflict-check.
///
/// `fixture_n06_resolved` (reused from
/// `n06_conflict_check_charges_exactly_sigma_c_minus_1_times_f_o` above,
/// same hand-verified `m`/`f(o)` values) produces exactly ten phase-4
/// redefine facts -- two per redefinition record reaching the type that
/// resolves it (`quire.model.normalize.redefine/v1`,
/// `model-complete.md:231`: "one fact on (T, redefining feature) and one on
/// (T, redefined feature)"), for every record reaching that type, contested
/// or not:
///
/// - at `B`: `redef.B` reaches only `B` -- 2 facts (`B.x2`, `A.x`).
/// - at `C`: `redef.C` reaches only `C` -- 2 facts (`C.x3`, `A.x`).
/// - at `D`: `redef.B`, `redef.C` and `redef.D` all reach `D` -- 3 records
///   x 2 facts = 6 facts (`B.x2`, `A.x`, `C.x3`, `A.x`, `D.x4`, `A.x`).
///
/// Total: `2 + 2 + 6 = 10`.
///
/// Cross-checked against the crate: a completed (`UNLIMITED`) run charges
/// `work_units = 103` and peaks at `derivation_facts = 30` (the `20`
/// phase-2/3 facts plus these ten). `work_units = 102` is exactly one short
/// of that total, `Incomplete` at the run's own last charge; `work_units =
/// 103` completes.
///
/// Revert probe: dropping the phase-4 `normalize.fact` charge loop out of
/// `charge_all` makes `work_units` read `93`, not `103`, and moves the
/// `work_units = 47` boundary's denial to `normalize.conflict-check` --
/// confirmed by hand: reverting the change locally reproduces both,
/// restoring it returns this test to green.
#[trace("TC-195", "FR-150-AC-8")]
#[test]
fn n06_redefine_facts_are_charged_as_normalize_fact_between_the_two_phase4_checks() {
    let domain_package = fixture_n06_resolved();

    let (outcome, meter) =
        normalize_with_meter(&domain_package, ModelNormalizationLimits::UNLIMITED);
    match outcome {
        NormalizeOutcome::Completed(view) => assert_eq!(view.declarations().len(), 13),
        other => panic!("expected Completed, got {other:?}"),
    }
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 103);
    assert_eq!(meter.consumed(LimitKind::DerivationFacts), 30);

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 47;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 47,
                    consumed: 47,
                    next_charge: 1,
                    charge_point: ChargePoint::NormalizeFact,
                },
                "the first phase-4 normalize.fact charge, never once with a \
                 flat cost other than one work unit"
            );
        }
        other => {
            panic!("expected Incomplete at the first phase-4 normalize.fact charge, got {other:?}")
        }
    }

    // Exact-bound pair (FR-150-AC-8): one short of the full charge total
    // denies at the run's own last charge; the full total completes.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 102;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 102,
                    consumed: 102,
                    next_charge: 1,
                    charge_point: ChargePoint::NormalizeHash,
                }
            );
        }
        other => panic!("expected Incomplete one work unit short of completion, got {other:?}"),
    }
    limits.work_units = 103;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Completed(view) => assert_eq!(view.declarations().len(), 13),
        other => panic!("expected Completed at work_units=103, got {other:?}"),
    }
}

/// A redefinition target with only one redefiner reaching it (`c = 1`)
/// admits zero `normalize.conflict-check`
/// charges -- `value-accounting.md:456`'s own `c >= 2` condition -- and no
/// dominance closure is ever computed for it, rather than the pre-fix
/// shape that walked one unconditionally regardless of `edges.len()`.
///
/// Cross-checked by running the crate directly: with every other limit
/// unlimited, this domain package completes at exactly `work_units = 36`
/// (`normalize.record` charges only its five declaration records --
/// `A`, `B`, `B2`, `A/x`, `B2.x2`; the two `Supertype` records and the one
/// `Redefinition` record state relationships, not declarations of their
/// own, and are never charged -- plus the single redefinition record's own
/// two phase-4 redefine facts -- one on `B2.x2`, one on the redefined
/// `A.x` -- charged as `normalize.fact`, `+2`), and its
/// one `normalize.redefinition-check` charge is exactly `2` (`m = 2`:
/// `B2`'s own effective members are `A.x`, inherited, and `B2.x2`, direct;
/// `r = 0`: the only redefinition record examined in this build).
///
/// Revert probe: reverting the `edges.len() < 2` guard in
/// `apply_redefinitions` back to computing `ensure_owner_closures` and a
/// `Σ (c − 1) × f(o)` charge unconditionally makes the conflict-check-count
/// assertion fail (it becomes 1, charged at `(1 − 1) × f(B2) = 0` work
/// units under the new formula, or a flat 1 under the older pre-#167
/// shape) -- confirmed by hand: removing the guard locally reproduces the
/// failure, restoring it returns this test to green.
#[trace("TC-195", "TC-196", "FR-150-AC-8", "FR-151-AC-2")]
#[test]
fn n06_a_single_redefiner_admits_no_conflict_check_charge() {
    let domain_package = fixture_single_redefiner_no_conflict();

    let (outcome, meter) =
        normalize_with_meter(&domain_package, ModelNormalizationLimits::UNLIMITED);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeRedefinitionCheck)
            .count(),
        1
    );
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeConflictCheck)
            .count(),
        0,
        "a single redefiner has nothing to dominate and admits no conflict-check charge"
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 36);

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 16;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.consumed, 16);
            assert_eq!(incomplete.next_charge, 2);
            assert_eq!(
                incomplete.charge_point,
                ChargePoint::NormalizeRedefinitionCheck
            );
        }
        other => panic!("expected Incomplete at normalize.redefinition-check, got {other:?}"),
    }
}

/// Before this fix, resolving a redefinition group unconditionally
/// computed every contesting owner's ancestor-dominance closure -- even a
/// group with a single, uncontested
/// redefiner, which has nothing to dominate. A redefiner's owner with 128
/// or more *direct* generalizations (never a deep chain; a single wide
/// fan-out is enough) hit `crate::model::conformance`'s own
/// `MAX_CONFORMANCE_DEPTH` (128) ceiling computing that unneeded closure,
/// wrongly refusing `conformance-depth` on a domain package with no actual
/// dominance question to resolve. Both 128 (the exact boundary: 128 direct
/// generalizations plus the owner itself is the 129th node the walk would
/// visit) and 200 (comfortably over) must complete cleanly and quickly.
///
/// Revert probe: reverting the `edges.len() < 2` guard in
/// `apply_redefinitions` (skipping `ensure_owner_closures` entirely for a
/// single-edge group) back to calling it unconditionally reproduces the
/// original refusal at both widths -- confirmed by hand: removing the
/// guard locally makes this test fail with a `conformance-depth` refusal
/// at both 128 and 200, restoring it returns this test to green.
#[trace("TC-195", "TC-196", "FR-151-AC-2")]
#[test]
fn n06_wide_ancestry_with_a_single_uncontested_redefiner_completes() {
    for n_parents in [128usize, 200usize] {
        let domain_package = fixture_wide_ancestry_single_redefiner(n_parents);
        let start = std::time::Instant::now();
        let outcome = normalize(&domain_package, ModelNormalizationLimits::UNLIMITED);
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "normalize took {elapsed:?} for {n_parents} direct generalizations \
             with a single uncontested redefiner"
        );
        let view = match outcome {
            NormalizeOutcome::Completed(view) => view,
            other => panic!(
                "expected Completed for {n_parents} direct generalizations, got {other:?} \
                 (an uncontested redefiner's owner breadth alone must never \
                 force a dominance-closure walk)"
            ),
        };

        let winner = view
            .declarations()
            .iter()
            .find(|entry| {
                entry.preimage.original.node == "model.Owner.x2"
                    && entry.preimage.owner_effective_type.is_some()
            })
            .unwrap_or_else(|| {
                panic!("no declaration for model.Owner.x2 in {n_parents}-parent view")
            });
        assert!(
            winner.visible,
            "Owner.x2 is the only redefiner and must win outright"
        );
    }
}

/// The `edges.len() < 2` guard above only ever fixed the *uncontested*
/// wide-ancestry case. `Owner` (`O`) still declares `n_parents` direct
/// generalizations, one of them to `Base` (`G000`), but now `G000` itself
/// also declares a second field, `G000.w`, that redefines `G000.x` — a
/// genuine two-redefiner contest (`c = 2`: `O.z` and `G000.w`) whose
/// dominance resolution, before this fix, still built `G000`'s own
/// [`crate::model::conformance::ancestor_closure`] and exceeded
/// `MAX_CONFORMANCE_DEPTH` on `O`'s breadth alone, exactly as the
/// uncontested case did. `O` is a proper descendant of `G000` (one of its
/// `n_parents` direct generalizations), so `O` dominates `G000` and `O.z`
/// wins outright.
fn fixture_wide_ancestry_contested_redefiners(n_parents: usize) -> DomainPackage {
    let mut records = vec![
        object_type("model.G000", vec![]),
        field_member("model.G000.x", "model.G000", "model.G000"),
        field_member_redefining(
            "model.G000.w",
            "model.G000",
            "model.G000",
            Some("model.G000.x"),
            vec![],
        ),
    ];
    let parent_ids: Vec<String> = (1..n_parents).map(|i| format!("model.G{i:03}")).collect();
    for id in &parent_ids {
        records.push(object_type(id, vec![]));
    }
    let mut owner_supertypes: Vec<&str> = vec!["model.G000"];
    owner_supertypes.extend(parent_ids.iter().map(String::as_str));
    records.push(object_type("model.O", owner_supertypes));
    records.push(field_member_redefining(
        "model.O.z",
        "model.O",
        "model.G000",
        Some("model.G000.x"),
        vec![],
    ));
    DomainPackage::new(
        DomainPackageRef::fixture(format!("bundle.wide-contested-{n_parents}")),
        records,
    )
}

/// A wide (128+ direct generalizations) but genuinely *contested* ancestry
/// must resolve by dominance exactly as a
/// narrow one would, not refuse `conformance-depth` — the breadth-vs-depth
/// defect the `edges.len() < 2` guard alone left unfixed for `c >= 2`
/// groups.
///
/// Revert probe: reverting phase 4's dominance lookup from
/// `owner_ancestor_sets` (derived from phase 3's own `type_paths`) back to
/// `ensure_owner_closures`/`ancestor_closure`'s own bounded walk reproduces
/// the original `conformance-depth` refusal at both widths — confirmed by
/// hand: reintroducing that walk locally makes this test fail at both 128
/// and 200, restoring the fix returns it to green.
#[trace("TC-195", "TC-196", "FR-151-AC-2")]
#[test]
fn n06_wide_ancestry_with_two_contesting_redefiners_completes() {
    for n_parents in [128usize, 200usize] {
        let domain_package = fixture_wide_ancestry_contested_redefiners(n_parents);
        let start = std::time::Instant::now();
        let outcome = normalize(&domain_package, ModelNormalizationLimits::UNLIMITED);
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "normalize took {elapsed:?} for {n_parents} direct generalizations \
             with two contesting redefiners"
        );
        let view = match outcome {
            NormalizeOutcome::Completed(view) => view,
            other => panic!(
                "expected Completed for {n_parents} direct generalizations, got {other:?} \
                 (a genuinely contested wide ancestry must resolve by \
                 dominance, not refuse conformance-depth)"
            ),
        };

        if n_parents == 128 {
            assert_eq!(
                view.declarations().len(),
                134,
                "129 type declarations (G000..G127, O) plus 5 member \
                 declarations (G000.x and G000.w at G000's own view; O's own \
                 inherited x and w plus its direct z)"
            );
        }

        let owner_o = view
            .declarations()
            .iter()
            .find(|entry| {
                entry.preimage.owner_effective_type.is_none()
                    && entry.preimage.original.node == "model.O"
            })
            .unwrap_or_else(|| panic!("no type declaration for model.O in {n_parents}-parent view"))
            .effective_id;

        let winner = find_member(&view, &owner_o, "model.O.z");
        assert!(
            winner.visible,
            "O properly dominates G000, so O.z must win over G000.w"
        );
        let loser = find_member(&view, &owner_o, "model.G000.w");
        assert!(
            !loser.visible,
            "G000.w loses to the more-derived O.z at O's own effective view"
        );
    }
}

/// An operation-member redefinition record is charged by
/// `normalize.redefinition-check` exactly like a field one, and
/// its owner's effective-member count `m` includes operation members —
/// `apply_redefinitions` itself still skips operation-member redefinition
/// for conflict *resolution* (see the module docs; `crate::model::conformance`
/// resolves that directly), so this checks the charge alone.
fn fixture_operation_redefinition() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.op-redef"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            operation_member("model.A.op", "model.A"),
            operation_member_redefining("model.B.op2", "model.B", Some("model.A.op")),
        ],
    )
}

/// Cross-checked against the crate by running it directly: `B`'s own
/// effective members are `A.op` (inherited) and `B.op2` (direct), so
/// `m(B) = 2`; this is the only redefinition record in the domain package, so
/// `r = 0`, and `normalize.redefinition-check` charges exactly `2`.
///
/// `normalize.record` charges declaration records only (`A`, `B`, `A.op`,
/// `B.op2`); the fixture's `Supertype` and `Redefinition` records describe
/// relationships, not declarations, and never charge their own
/// `normalize.record`.
///
/// Revert probe: reverting the `m` computation to count only
/// `member_counts_by_owner`'s pre-fix (field-only) tally, with no
/// operation-member contribution, makes both assertions below fail: the
/// total drops from `16` to `14` (the redefinition-check charge drops from
/// `2` to `0`), and `work_units = 8` no longer denies at
/// `normalize.redefinition-check` (`next_charge` drops to `0`) — confirmed
/// by hand: removing the operation-member contribution locally reproduces
/// both failures, restoring it returns this test to green.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn operation_redefinition_is_charged_like_a_field_redefinition() {
    let domain_package = fixture_operation_redefinition();

    let (outcome, meter) =
        normalize_with_meter(&domain_package, ModelNormalizationLimits::UNLIMITED);
    assert!(matches!(outcome, NormalizeOutcome::Completed(_)));
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeRedefinitionCheck)
            .count(),
        1,
        "the operation redefinition record is charged even though \
         apply_redefinitions itself never resolves it"
    );
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeConflictCheck)
            .count(),
        0,
        "operation-member redefinition resolution stays out of scope for \
         apply_redefinitions (see the module docs); only the charge changed"
    );
    assert_eq!(
        meter.consumed(LimitKind::WorkUnits),
        16,
        "4 normalize.record (A, B, A.op, B.op2 -- Supertype/Redefinition \
         records are relationships, not declarations) + 2 normalize.fact \
         (A/B qualify) + 1 normalize.cycle-check + 1 normalize.fact (B's \
         own inherit-A path) + 1 normalize.redefinition-check (m + r = 2 + \
         0) + 2 normalize.declaration + 2 normalize.hash (per type \
         declaration) + 2 normalize.hash (universe/view) = 16; no member \
         declarations at all, since operation members never enter \
         member_preimages"
    );

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 8;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.consumed, 8);
            assert_eq!(
                incomplete.next_charge, 2,
                "m(B) = 2 (A.op inherited, B.op2 direct), r = 0"
            );
            assert_eq!(
                incomplete.charge_point,
                ChargePoint::NormalizeRedefinitionCheck
            );
        }
        other => panic!("expected Incomplete at normalize.redefinition-check, got {other:?}"),
    }
}

/// QSL #145: `B` declares two operation members, `B.op2` and `B.op3`, both
/// redefining the identical inherited
/// operation `A.op` — the operation-member analog of `fixture_n06_conflict`'s
/// field contest, except `B` is the only owner (nothing dominates anything;
/// this fixture is only about the `c >= 2` *charge*, never resolution).
fn fixture_operation_redefinition_conflict() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.op-redef-conflict"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            operation_member("model.A.op", "model.A"),
            operation_member_redefining("model.B.op2", "model.B", Some("model.A.op")),
            operation_member_redefining("model.B.op3", "model.B", Some("model.A.op")),
        ],
    )
}

/// `value-accounting.md:456` prices every `(effective type, redefined
/// member)` reached by `c >= 2` redefinition records, and an operation
/// member is a redefined member same as a field one — `apply_redefinitions`
/// charges a contested operation-redefinition group exactly like a
/// contested field group, and (#173) also resolves it with the identical
/// dominance search fields use: `B.op2` and `B.op3` share the same owner
/// `B`, so no edge dominates another and the group refuses
/// `redefinition-target`, not `derivation-conflict` (that shape needs
/// distinct, non-dominating owners; see
/// `r07_two_redefiners_owned_by_the_same_type_refuse_redefinition_target_through_normalize`
/// above for the field analog). Resolving an operation contest still builds
/// no `EffectiveView` member entry (see the module docs): only the ambiguity
/// check itself is shared with fields.
///
/// Cross-checked by hand: `f(A) = 1` (`A`'s own qualify fact only), `f(B) =
/// 2` (qualify plus its own inherit-`A` fact) — both types are otherwise
/// identical to `fixture_operation_redefinition`'s. The contested group at
/// `A.op` has `c = 2` edges, both owned by `B`, so `Σ (c − 1) × f(o) = (2 −
/// 1) × (f(B) + f(B)) = 1 × (2 + 2) = 4`.
///
/// Revert-probe: reverting `resolve_redefinition_contest`'s call site in the
/// operation loop back to charge-only (dropping the `Err` branch's
/// `record_phase4_refusal` call) turns this test's outcome back into
/// `Completed`, confirmed locally, then restored.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn operation_redefinition_group_with_two_or_more_redefiners_is_charged_a_conflict_check() {
    let domain_package = fixture_operation_redefinition_conflict();

    let (outcome, meter) =
        normalize_with_meter(&domain_package, ModelNormalizationLimits::UNLIMITED);
    match &outcome {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::RedefinitionTarget {
                    redefiners: vec![
                        DeclarationKey::fixture("model.B.op2"),
                        DeclarationKey::fixture("model.B.op3"),
                    ],
                    target: DeclarationKey::fixture("model.A.op"),
                }
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/redefinition-target), got {other:?}")
        }
    }
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeConflictCheck)
            .count(),
        1,
        "the contested operation-redefinition group is still charged once, \
         exhaustively, before phase 4's own refusal is reported \
         (value-accounting.md:481) -- B.op2 and B.op3 share the same owner, \
         so #173's dominance search now also resolves this contest, and \
         same-owner contention refuses redefinition-target"
    );
    assert_eq!(
        meter.consumed(LimitKind::WorkUnits),
        20,
        "5 normalize.record (A, B, A.op, B.op2, B.op3 -- Supertype/\
         Redefinition records are relationships, not declarations) + 2 \
         normalize.fact (A/B qualify) + 1 normalize.cycle-check + 1 \
         normalize.fact (B's own inherit-A path) + 2 \
         normalize.redefinition-check (m + r = 3 + 0, then 3 + 1) + 1 \
         normalize.conflict-check ((c-1) * (f(B)+f(B)) = 1 * (2+2) = 4) = \
         20; phase 4's own charges are exhaustive up to and including \
         normalize.conflict-check (value-accounting.md:481), but the \
         redefinition-target refusal that same-owner contest now reports \
         (#173) ends checking there -- no normalize.declaration or \
         normalize.hash charge ever runs (value-accounting.md:482)"
    );

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 16;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.consumed, 16);
            assert_eq!(
                incomplete.next_charge, 4,
                "(c-1) * (f(B)+f(B)) = 1 * (2+2) = 4"
            );
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeConflictCheck);
        }
        other => panic!("expected Incomplete at normalize.conflict-check, got {other:?}"),
    }
}

/// M2 (#204 round 1): the operation-member analog of
/// `fixture_n06_conflict` -- a diamond (`D <- B, C`; `B <- A`; `C <- A`)
/// where `B.op2` and `C.op3` both redefine `A.op` and neither dominates the
/// other (distinct sibling owners, unlike
/// `operation_redefinition_group_with_two_or_more_redefiners_is_charged_a_conflict_check`'s
/// same-owner `B.op2`/`B.op3` shape above), and `D` (unlike
/// `operation_redefinition_group_resolved_by_a_dominating_owner_completes`
/// below) declares no `D.op4` of its own to dominate them. #173's dominance
/// search resolves this the same way `n06_two_undominated_redefiners_of_the_same_target_refuse_as_a_conflict`
/// resolves the field analog: no edge dominates, so it refuses
/// `derivation-conflict`, naming the full typed payload.
fn fixture_operation_diamond_conflict(reversed: bool) -> DomainPackage {
    let mut records = vec![
        object_type("model.A", vec![]),
        object_type("model.B", vec!["model.A"]),
        object_type("model.C", vec!["model.A"]),
        object_type("model.D", vec!["model.B", "model.C"]),
        operation_member("model.A.op", "model.A"),
        operation_member_redefining("model.B.op2", "model.B", Some("model.A.op")),
        operation_member_redefining("model.C.op3", "model.C", Some("model.A.op")),
    ];
    if reversed {
        records.reverse();
    }
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.op-diamond-conflict"),
        records,
    )
}

/// M1's own fix (#204 round 1) is exactly what this test needs: without it,
/// `domain_package.records`' own (reversed) order would leak into
/// `DerivationConflict`'s `redefiners` list, and this test's own second
/// half (the reversed-order run) would see `[C.op3, B.op2]` instead of the
/// sorted `[B.op2, C.op3]` both runs assert here.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn operation_diamond_derivation_conflict_reports_the_full_typed_payload() {
    for reversed in [false, true] {
        let domain_package = fixture_operation_diamond_conflict(reversed);
        match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
            NormalizeOutcome::Refused(refusal) => {
                assert_eq!(
                    refusal.len(),
                    1,
                    "expected exactly one refusal, reversed={reversed}: {refusal:?}"
                );
                let refusal = refusal[0].clone();
                assert_eq!(
                    refusal.code,
                    qsl_foundation::diagnostic::Code::InvalidModelBinding
                );
                assert_eq!(
                    refusal.cause,
                    ModelRefusalCause::DerivationConflict {
                        type_: DeclarationKey::fixture("model.D"),
                        member: DeclarationKey::fixture("model.A.op"),
                        redefiners: vec![
                            DeclarationKey::fixture("model.B.op2"),
                            DeclarationKey::fixture("model.C.op3"),
                        ],
                    },
                    "reversed={reversed}"
                );
            }
            other => panic!(
                "expected Refused(invalid_model_binding/derivation-conflict), reversed={reversed}, got {other:?}"
            ),
        }
    }
}

/// The operation-member analog of `fixture_n06_resolved`: `B.op2` (owner
/// `B <- A`) and `C.op3` (owner `C <- A`) both redefine `A.op` -- sibling
/// owners, neither dominating the other -- but `D` (`<- B`, `<- C`) also
/// declares `D.op4` redefining `A.op`, and `D` is a proper descendant of
/// both `B` and `C`. #173's dominance search resolves this contest (`D.op4`
/// dominates every other redefiner), so the group completes with no
/// refusal -- the "a winner if one dominates" half of #173's rule, as
/// opposed to the same-owner and diamond refusal cases exercised above and
/// in `n06_two_undominated_redefiners_of_the_same_target_refuse_as_a_conflict`.
/// Resolving an operation contest builds no `EffectiveView` member entry
/// (see the module docs): there is nothing to assert about the view here,
/// only that the ambiguity check itself does not refuse.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn operation_redefinition_group_resolved_by_a_dominating_owner_completes() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.op-redef-resolved"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            object_type("model.C", vec!["model.A"]),
            object_type("model.D", vec!["model.B", "model.C"]),
            operation_member("model.A.op", "model.A"),
            operation_member_redefining("model.B.op2", "model.B", Some("model.A.op")),
            operation_member_redefining("model.C.op3", "model.C", Some("model.A.op")),
            operation_member_redefining("model.D.op4", "model.D", Some("model.A.op")),
        ],
    );

    let (outcome, meter) =
        normalize_with_meter(&domain_package, ModelNormalizationLimits::UNLIMITED);
    assert!(
        matches!(outcome, NormalizeOutcome::Completed(_)),
        "D.op4 dominates every other redefiner of A.op (B and C are both \
         proper ancestors of D), so the contest resolves without a \
         refusal: {outcome:?}"
    );
    // M3 (#204 round 1): a *resolved* contest still owes its
    // `normalize.conflict-check` charge (`value-accounting.md:456`'s own
    // `c >= 2` condition, independent of whether the contest resolves --
    // `n06_wide_ancestry_with_two_contesting_redefiners_completes` is this
    // same rule's field analog), which the prior version of this test never
    // asserted.
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeConflictCheck)
            .count(),
        1,
        "D's c=3 group (B.op2, C.op3, D.op4) is still charged once even though it resolves"
    );
    // Cross-checked by running the crate directly (outcome is `Completed`,
    // so this is the exhaustive total through the final `normalize.hash`):
    // 8 `normalize.record` (A, B, C, D, A.op, B.op2, C.op3, D.op4) + facts
    // for every type's own qualify/inherit path (`f(A)=1, f(B)=f(C)=2,
    // f(D)=4` under the diamond's own ancestor set `{A}`/`{A,B,C}`) + one
    // `normalize.cycle-check` per generalization edge (B->A, C->A, D->B,
    // D->C) + the `c=3` group's own `normalize.redefinition-check`/
    // `normalize.conflict-check` charges + `normalize.declaration`/
    // `normalize.hash` once the group resolves.
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 65);
}

/// `value-accounting.md:456` prices every contested `(effective type,
/// redefined member)` in one ascending pass "by effective member key" --
/// field and operation targets
/// interleaved by that one key, never field targets charged as a block
/// before operation targets as a block. `C <= B <= A` (a two-step
/// generalization chain, so `f(A) = 1`, `f(B) = 2`, `f(C) = 3`):
///
/// - Field `A.z` is redefined by `B.z1` (owner `B`) and `C.z2` (owner `C`).
///   `C` is a proper descendant of `B`, so this contest resolves (`C.z2`
///   wins) without a refusal, but a resolved contest still owes its charge
///   (`c >= 2` is the only condition, `value-accounting.md:456`). `c = 2`,
///   `Σ (c − 1) × f(o) = (2 − 1) × (f(B) + f(C)) = 1 × (2 + 3) = 5`.
/// - Operation `A.a` is redefined by `C.a2` and `C.a3`, both owned by `C`.
///   `apply_redefinitions` charges this contest exactly like the field one,
///   `c = 2`, `Σ (c − 1) × f(o) = (2 − 1) × (f(C) + f(C)) = 1 × (3 + 3) =
///   6`, and (#173) also resolves it with the same dominance search: no
///   edge dominates another (both share owner `C`), so it refuses
///   `redefinition-target`.
///
/// `model.A.a` sorts before `model.A.z` (identity bytes: `a` < `z`), so the
/// merged-and-sorted order charges the operation group (`6`) before the
/// field group (`5`) -- the reverse of the pre-fix order, which charged
/// every field group (here, just the one `5`) before any operation group
/// (`6`), regardless of which target key sorts first.
///
/// Cross-checked by running the crate directly: with every other limit
/// unlimited, this domain package's phase 4 charges exhaustively up to and
/// including the last `normalize.conflict-check`
/// (value-accounting.md:481) at exactly `work_units = 69`
/// (`normalize.record` charges its nine declaration records -- three
/// object types, three fields and three operation members; `B`'s and `C`'s
/// own `supertypes[]` entries and every member's own `redefines` property
/// are inline on those same nine records, not separate declarations of
/// their own, so there is nothing else to charge -- plus the field redefinition
/// group's own six phase-4 redefine facts, `+6`: two at `B` for `redef.z1`
/// reaching only `B`, and four at `C` for `redef.z1`/`redef.z2` both
/// reaching `C`; the operation group's own two records contribute no facts
/// at all, since operation members never enter `member_preimages` and get
/// no `Fact` here, only a `normalize.conflict-check` charge -- see the
/// module docs), then refuses `redefinition-target` there
/// (value-accounting.md:482 ends checking at that stage's refusal): no
/// `normalize.declaration` or `normalize.hash` charge ever runs, unlike
/// before #173 resolved this contest. `work_units = 58` (`52 + 6`) is
/// exactly enough to admit every charge up to and including the six
/// phase-4 `normalize.fact` charges, denying at the first
/// `normalize.conflict-check` -- the operation group's `6`, not the field
/// group's `5`.
///
/// Revert probe: reverting `apply_redefinitions` back to each loop pushing
/// its own charge straight to `conflict_check_work` (the pre-fix shape)
/// makes the `work_units = 58` assertion fail -- `next_charge` becomes `5`
/// (the field group, charged first again) instead of `6` -- confirmed by hand:
/// reverting the two loops to push directly, locally, reproduces the
/// failure; restoring the collect-sort-push shape returns this test to
/// green.
#[trace("TC-195", "TC-196", "FR-150-AC-8", "FR-151-AC-2")]
#[test]
fn conflict_check_charges_interleave_field_and_operation_groups_by_target_key() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.field-op-order"),
        vec![
            object_type("model.A", vec![]),
            object_type("model.B", vec!["model.A"]),
            object_type("model.C", vec!["model.B"]),
            field_member("model.A.z", "model.A", "model.A"),
            field_member_redefining(
                "model.B.z1",
                "model.B",
                "model.A",
                Some("model.A.z"),
                vec![],
            ),
            field_member_redefining(
                "model.C.z2",
                "model.C",
                "model.A",
                Some("model.A.z"),
                vec![],
            ),
            operation_member("model.A.a", "model.A"),
            operation_member_redefining("model.C.a2", "model.C", Some("model.A.a")),
            operation_member_redefining("model.C.a3", "model.C", Some("model.A.a")),
        ],
    );

    let (outcome, meter) =
        normalize_with_meter(&domain_package, ModelNormalizationLimits::UNLIMITED);
    match &outcome {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::RedefinitionTarget {
                    redefiners: vec![
                        DeclarationKey::fixture("model.C.a2"),
                        DeclarationKey::fixture("model.C.a3"),
                    ],
                    target: DeclarationKey::fixture("model.A.a"),
                }
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/redefinition-target), got {other:?}")
        }
    }
    let admitted = meter.admitted_charges();
    assert_eq!(
        admitted
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeConflictCheck)
            .count(),
        2,
        "one contested group for A.z (field) and one for A.a (operation) -- \
         both still charged exhaustively (value-accounting.md:481) even \
         though the operation group (C.a2/C.a3, same owner C) now also \
         refuses redefinition-target (#173); the field group resolves \
         (C.z2 dominates B.z1) and stays unrefused"
    );
    // Includes the field redefinition group's own six phase-4 redefine
    // facts' `normalize.fact` charges (operation members derive no facts of
    // their own -- see the module docs). Phase 4's own charges are
    // exhaustive up to and including the last normalize.conflict-check
    // (value-accounting.md:481), but the operation group's
    // redefinition-target refusal ends checking there (#173,
    // value-accounting.md:482): no normalize.declaration or normalize.hash
    // charge ever runs, unlike before #173 resolved this contest.
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 69);

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 58;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
            assert_eq!(incomplete.limit, 58);
            assert_eq!(incomplete.consumed, 58);
            assert_eq!(
                incomplete.next_charge, 6,
                "model.A.a sorts before model.A.z, so the operation group's \
                 charge (6) is denied before the field group's (5)"
            );
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeConflictCheck);
        }
        other => panic!("expected Incomplete at normalize.conflict-check, got {other:?}"),
    }
}

/// A phase-3 refusal (`specialization-cycle`) wins over a phase-4 refusal
/// (`derivation-conflict`) when a domain package has
/// both, matching FR-150's "each normalization phase ... reports every
/// refusal it exposes in charge order; a phase that reports a refusal ends
/// checking." `fixture_n06_conflict` (`B`/`C`, two undominated redefiners
/// of `A.x`, with no `D` to resolve them — phase 4's own defect) plus an
/// unrelated `Y <-> Z` generalization cycle (phase 3's own defect, TC-196
/// R01) exercises this precedence directly: phase 3 runs, and refuses,
/// before phase 4 ever gets a turn.
fn fixture_n06_conflict_with_unrelated_cycle() -> DomainPackage {
    let mut records = fixture_n06_conflict().records;
    records.push(object_type("model.Y", vec!["model.Z"]));
    records.push(object_type("model.Z", vec!["model.Y"]));
    DomainPackage::new(DomainPackageRef::fixture("bundle.n06-cycle"), records)
}

/// A minimal diamond conflict (`R`, `D1 <- R`, `D2 <- R`, `O9 <- D1, D2`,
/// both `D1.w1`/`D2.w2` redefining `R.w`) with every own identity built
/// from `prefix`, so two calls with different prefixes produce two
/// structurally identical, independently conflicting families.
fn fixture_diamond_conflict_family(prefix: &str) -> Vec<DomainPackageRecord> {
    let root = format!("ix://{prefix}/R");
    let d1 = format!("ix://{prefix}/D1");
    let d2 = format!("ix://{prefix}/D2");
    let owner = format!("ix://{prefix}/O9");
    let field = format!("ix://{prefix}/R/w");
    let f1 = format!("ix://{prefix}/D1/w1");
    let f2 = format!("ix://{prefix}/D2/w2");
    vec![
        object_type(&root, vec![]),
        object_type(&d1, vec![&root]),
        object_type(&d2, vec![&root]),
        object_type(&owner, vec![&d1, &d2]),
        field_member(&field, &root, &root),
        field_member_redefining(&f1, &d1, &root, Some(&field), vec![]),
        field_member_redefining(&f2, &d2, &root, Some(&field), vec![]),
    ]
}

/// Two independent `fixture_diamond_conflict_family` instances, prefixed
/// `aaa` and `zzz`: `"ix://aaa/O9" < "ix://zzz/O9"` as UTF-8 bytes, so
/// `aaa`'s own conflict sorts first by producer `DeclarationKey` -- QSL
/// #195's own pre-fix, buggy order. `aaa`'s and `zzz`'s own effective
/// identities (content hashes of their own canonicalized preimage bytes,
/// `model-complete.md`:206) do not follow that same byte order: empirically
/// (confirmed by hand, independent of which family's records are appended
/// first in `records`, ruling out array-position as the cause) `zzz`'s own
/// effective identity sorts *before* `aaa`'s, opposite producer-key order,
/// so this fixture pins the two orders apart.
fn fixture_two_diamonds_where_effective_identity_disagrees_with_producer_key() -> DomainPackage {
    let mut records = fixture_diamond_conflict_family("aaa");
    records.extend(fixture_diamond_conflict_family("zzz"));
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.n06-effective-order"),
        records,
    )
}

/// QSL #195: `normalize.conflict-check` charges ascending by *effective*
/// member key (`value-accounting.md:456`; `model-complete.md`:206's
/// `(owner effective type identity, original declaration key)`), not by
/// the owning type's producer `DeclarationKey` -- `apply_redefinitions`
/// used to sort `type_keys` and tag every conflict-check charge and
/// refusal by `type_key` itself. `fixture_two_diamonds_where_effective_identity_disagrees_with_producer_key`
/// pins the fix directly: `zzz`'s own `derivation-conflict` is reported
/// first, ahead of `aaa`'s, even though `"ix://aaa/O9" <
/// "ix://zzz/O9"` as producer keys -- the pre-fix code would have reported
/// `aaa` first.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n06_conflict_check_charges_order_by_effective_identity_not_producer_key() {
    let domain_package =
        fixture_two_diamonds_where_effective_identity_disagrees_with_producer_key();
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                vec![
                    ModelRefusal {
                        code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                        cause: ModelRefusalCause::DerivationConflict {
                            type_: DeclarationKey::fixture("ix://zzz/O9"),
                            member: DeclarationKey::fixture("ix://zzz/R/w"),
                            redefiners: vec![
                                DeclarationKey::fixture("ix://zzz/D1/w1"),
                                DeclarationKey::fixture("ix://zzz/D2/w2"),
                            ],
                        },
                        detail: "type ix://zzz/O9 has 2 undominated redefinitions of ix://zzz/R/w: \
                                  [ix://zzz/D1, ix://zzz/D1/w1, ix://zzz/R/w] and \
                                  [ix://zzz/D2, ix://zzz/D2/w2, ix://zzz/R/w]"
                            .to_string(),
                    },
                    ModelRefusal {
                        code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                        cause: ModelRefusalCause::DerivationConflict {
                            type_: DeclarationKey::fixture("ix://aaa/O9"),
                            member: DeclarationKey::fixture("ix://aaa/R/w"),
                            redefiners: vec![
                                DeclarationKey::fixture("ix://aaa/D1/w1"),
                                DeclarationKey::fixture("ix://aaa/D2/w2"),
                            ],
                        },
                        detail: "type ix://aaa/O9 has 2 undominated redefinitions of ix://aaa/R/w: \
                                  [ix://aaa/D1, ix://aaa/D1/w1, ix://aaa/R/w] and \
                                  [ix://aaa/D2, ix://aaa/D2/w2, ix://aaa/R/w]"
                            .to_string(),
                    },
                ]
            );
        }
        other => panic!(
            "expected Refused([derivation-conflict for zzz, derivation-conflict for aaa]), got {other:?}"
        ),
    }
}

/// Two unrelated owner types, each with exactly one direct field member:
/// `ix://aaa/Owner` (field `x`, a short JCS preimage) and `ix://zzz/Owner`
/// (field `yyyyyyyyyyyyyyyyyyyy`, a longer one). `"ix://aaa/Owner" <
/// "ix://zzz/Owner"` as producer `DeclarationKey`s, so QSL #195's own
/// pre-fix, buggy order would hash `aaa`'s member before `zzz`'s. `aaa`'s
/// and `zzz`'s own effective identities do not follow that byte order
/// (confirmed by hand, the same inversion as
/// `fixture_two_diamonds_where_effective_identity_disagrees_with_producer_key`,
/// and independent of which family's records are appended first in
/// `records`): `zzz`'s own effective identity sorts before `aaa`'s.
fn fixture_two_owners_where_effective_identity_disagrees_with_producer_key() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("bundle.n08-declaration-order"),
        vec![
            object_type("ix://aaa/Owner", vec![]),
            field_member("ix://aaa/Owner/x", "ix://aaa/Owner", "ix://aaa/Owner"),
            object_type("ix://zzz/Owner", vec![]),
            field_member(
                "ix://zzz/Owner/yyyyyyyyyyyyyyyyyyyy",
                "ix://zzz/Owner",
                "ix://zzz/Owner",
            ),
        ],
    )
}

/// QSL #195: `normalize.declaration` (and the `normalize.hash` charge that
/// backs it) charges effective members ascending by effective member key
/// (`value-accounting.md:494`; `model-complete.md`:206's `(owner effective
/// type identity, original declaration key)`), not by the owning type's
/// producer `DeclarationKey` -- `build()` used to sort `member_keys` by the
/// owner's own producer key. Both owner types' own declarations are hashed
/// first (`367` bytes each, identical shape, so their own order does not
/// distinguish the fix); both together consume `734`. The exact boundary is
/// pinned by hand: at `hashed_bytes = 1263` the next charge is still
/// `zzz`'s `530`-byte member (not yet admitted); at `hashed_bytes = 1264`
/// (`734 + 530`) that charge has been admitted and the next charge is
/// `aaa`'s own, smaller, `492`-byte member -- proving `zzz`'s own member
/// was hashed *before* `aaa`'s, even though
/// `"ix://aaa/Owner" < "ix://zzz/Owner"` as producer keys. The pre-fix code
/// would have hashed `aaa`'s smaller member first, reporting `next_charge:
/// 530` (still `zzz`'s, now the *last* one) at `hashed_bytes = 1264`
/// instead.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n08_declaration_charges_order_by_effective_identity_not_producer_key() {
    let domain_package = fixture_two_owners_where_effective_identity_disagrees_with_producer_key();
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.hashed_bytes = 1263;
    assert_eq!(
        normalize(&domain_package, limits),
        NormalizeOutcome::Incomplete(Incomplete {
            limit_kind: LimitKind::HashedBytes,
            limit: 1263,
            consumed: 734,
            next_charge: 530,
            charge_point: ChargePoint::NormalizeHash,
        }),
        "zzz's own member (530 bytes) must still be the next charge one byte short of admitting it"
    );
    limits.hashed_bytes = 1264;
    assert_eq!(
        normalize(&domain_package, limits),
        NormalizeOutcome::Incomplete(Incomplete {
            limit_kind: LimitKind::HashedBytes,
            limit: 1264,
            consumed: 1264,
            next_charge: 492,
            charge_point: ChargePoint::NormalizeHash,
        }),
        "zzz's own member (530 bytes) must be admitted first, leaving aaa's own \
         smaller member (492 bytes) as the next charge"
    );
}

#[trace("TC-195", "TC-196", "FR-150-AC-3", "FR-150-AC-8", "FR-151-AC-2")]
#[test]
fn n06_specialization_cycle_refusal_waits_for_every_phase3_charge_to_admit() {
    let domain_package = fixture_n06_conflict_with_unrelated_cycle();

    for work_units in 18u64..=45 {
        let mut limits = ModelNormalizationLimits::UNLIMITED;
        limits.work_units = work_units;
        match normalize(&domain_package, limits) {
            NormalizeOutcome::Incomplete(incomplete) => {
                assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
                assert_eq!(incomplete.limit, work_units);
                assert!(
                    matches!(
                        incomplete.charge_point,
                        ChargePoint::NormalizeRecord
                            | ChargePoint::NormalizeFact
                            | ChargePoint::NormalizeCycleCheck
                    ),
                    "work_units={work_units} stopped at {:?}, a phase-4 charge point -- \
                     phase 3 must exhaust its own charges before phase 4 ever starts",
                    incomplete.charge_point
                );
            }
            other => panic!("expected Incomplete at work_units={work_units}, got {other:?}"),
        }
    }

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 45;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 45,
                    consumed: 45,
                    next_charge: 1,
                    charge_point: ChargePoint::NormalizeFact,
                }
            );
        }
        other => panic!("expected Incomplete at work_units=45, got {other:?}"),
    }

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 46;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                vec![ModelRefusal {
                    code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                    cause: ModelRefusalCause::SpecializationCycle {
                        ancestor: DeclarationKey::fixture("model.Y"),
                        via: DeclarationKey::fixture("model.Z"),
                    },
                    detail: "model.Y generalizes back to itself via model.Z, \
                              through the cycle [model.Y, model.Z]"
                        .to_string(),
                }]
            );
        }
        other => panic!("expected Refused([SpecializationCycle]) at work_units=46, got {other:?}"),
    }
}

#[trace("TC-195", "TC-196", "FR-150-AC-8", "FR-151-AC-2")]
#[test]
fn phase3_specialization_cycle_refusal_wins_over_phase4_derivation_conflict() {
    match normalize(
        &fixture_n06_conflict_with_unrelated_cycle(),
        ModelNormalizationLimits::UNLIMITED,
    ) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal.code,
                qsl_foundation::diagnostic::Code::InvalidModelBinding
            );
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::SpecializationCycle {
                    ancestor: DeclarationKey::fixture("model.Y"),
                    via: DeclarationKey::fixture("model.Z"),
                },
                "phase 3's own refusal must win over phase 4's undominated-\
                 redefiner derivation-conflict, matching FR-150's \
                 charge-order precedence"
            );
            assert!(
                refusal.detail.contains("[model.Y, model.Z]"),
                "detail must name the unrelated cycle, got: {}",
                refusal.detail
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/specialization-cycle), got {other:?}")
        }
    }
}

/// A phase-4 refusal (`derivation-conflict`/`redefinition-target`) does not
/// short-circuit `build()` before `charge_all` replays phase 4's own
/// charges. `value-accounting.md:481`'s "checking is exhaustive within a
/// stage" means every `normalize.redefinition-check`, every phase-4
/// `normalize.fact` and every `normalize.conflict-check` for
/// `fixture_n06_conflict` (`B.x2`/`C.x3`, undominated redefiners of `A.x`
/// reaching `D`) must be admitted before the `derivation-conflict` refusal
/// they expose is reported; a tighter limit that runs out first reports
/// that `Incomplete` instead, never the refusal.
///
/// `fixture_n06_conflict`'s exact phase-4 shape: `redef.B` reaches only `B`
/// (2 facts), `redef.C` reaches only `C` (2 facts), and both
/// `redef.B`/`redef.C` reach `D` (4 facts) -- eight phase-4 facts total. A
/// completed (`UNLIMITED`) run charges 27 `derivation_facts` (19 through
/// phase 2/3 plus these 8) and 51 `work_units` (34 through phase 3 --
/// `normalize.record` charges only this fixture's seven declaration
/// records, never its six `Supertype`/`Redefinition` records -- `+5`
/// for the two `normalize.redefinition-check` charges, `+8` for the
/// phase-4 facts, `+4` for the one `normalize.conflict-check` group) before
/// reporting the refusal.
///
/// Revert probe: hand-reverting `build()` to return the phase-4 refusal
/// directly makes every case below fail -- `derivation_facts = 19` reports
/// `Refused` instead of `Incomplete`, and every `work_units` case in
/// `16..=50` reports `Refused` instead of `Incomplete` -- confirmed by
/// hand: reverting `build`/`charge_all` locally reproduces every failure,
/// restoring them returns this test to green.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n06_conflict_refusal_waits_for_every_phase4_charge_to_admit() {
    let domain_package = fixture_n06_conflict();

    // A `derivation_facts` budget that exhausts exactly at the 19th
    // phase-2/3 fact -- one short of the first phase-4 fact -- reports
    // `Incomplete` at that first phase-4 `normalize.fact`, never the
    // `derivation-conflict` refusal `D`'s own resolution would otherwise
    // expose.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.derivation_facts = 19;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::DerivationFacts,
                    limit: 19,
                    consumed: 19,
                    next_charge: 20,
                    charge_point: ChargePoint::NormalizeFact,
                }
            );
        }
        other => panic!("expected Incomplete at the first phase-4 normalize.fact, got {other:?}"),
    }

    // `work_units` in `39..=46` land inside the eight phase-4
    // `normalize.fact` charges, after both `normalize.redefinition-check`
    // charges and before the one `normalize.conflict-check` charge.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 42;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 42,
                    consumed: 42,
                    next_charge: 1,
                    charge_point: ChargePoint::NormalizeFact,
                }
            );
        }
        other => panic!("expected Incomplete at a phase-4 normalize.fact, got {other:?}"),
    }

    // `work_units` in `16..=33` land inside phase 2/3's own facts, well
    // before phase 4 starts.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 24;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 24,
                    consumed: 24,
                    next_charge: 1,
                    charge_point: ChargePoint::NormalizeFact,
                }
            );
        }
        other => panic!("expected Incomplete at a phase 2/3 normalize.fact, got {other:?}"),
    }

    // `work_units = 50` is one short of the one `normalize.conflict-check`
    // charge's own `work_units` price (`4`): 47 are already consumed (34
    // through phase 3, `+5` redefinition-check, `+8` phase-4 facts), so the
    // attempted 4-unit conflict-check charge would reach 51, one over the
    // limit -- `consumed` reports that pre-charge total, not the limit
    // itself.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 50;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 50,
                    consumed: 47,
                    next_charge: 4,
                    charge_point: ChargePoint::NormalizeConflictCheck,
                }
            );
        }
        other => {
            panic!("expected Incomplete at the normalize.conflict-check charge, got {other:?}")
        }
    }

    // `work_units = 51` is exactly enough to admit every phase-4 charge
    // (34 through phase 3, `+5` redefinition-check, `+8` phase-4 facts,
    // `+4` conflict-check), so the refusal these charges expose is finally
    // reported.
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 51;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal,
                ModelRefusal {
                    code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                    cause: ModelRefusalCause::DerivationConflict {
                        type_: DeclarationKey::fixture("ix://test/orders/D"),
                        member: DeclarationKey::fixture("ix://test/orders/A/x"),
                        redefiners: vec![
                            DeclarationKey::fixture("model.B.x2"),
                            DeclarationKey::fixture("model.C.x3"),
                        ],
                    },
                    detail: "type ix://test/orders/D has 2 undominated redefinitions of ix://test/orders/A/x: \
                              [ix://test/orders/B, model.B.x2, ix://test/orders/A/x] and \
                              [ix://test/orders/C, model.C.x3, ix://test/orders/A/x]"
                        .to_string(),
                }
            );
        }
        other => panic!("expected Refused(derivation-conflict) at work_units=57, got {other:?}"),
    }
}

/// The deferred-refusal treatment covers `RedefinitionTarget` -- a
/// redefinition record whose own target is not an effective member of its
/// owner -- exactly like the dominance refusals in
/// `n06_conflict_refusal_waits_for_every_phase4_charge_to_admit` above:
/// `apply_redefinitions` collects it into `accounting.refusals` and moves
/// on to the next target group, rather than returning it directly out of
/// `build()`. `fixture_n06_conflict_with_unreachable_redefiner` adds type
/// `E` (no generalization) with `redef.E` (`E`, `E.y` redefines `A.x`):
/// since `E` does not inherit `A`, `A.x` is not an effective member of
/// `E`, so this redefinition's own target is unreachable, alongside `D`'s
/// own dominance conflict over the same `A.x` (from `fixture_n06_conflict`).
///
/// `E`'s own `RedefinitionTarget` refusal wins over `D`'s
/// `derivation-conflict` under `UNLIMITED`: `record_phase4_refusal` ranks a
/// `normalize.redefinition-check`-stage refusal (`value-accounting.md:455`)
/// ahead of a `normalize.conflict-check`-stage refusal (`:456`), since every
/// redefinition-check charge precedes every phase-4 fact charge, which in
/// turn precedes every conflict-check charge -- `E`'s target check fails at
/// its own redefinition-check charge, long before `D`'s ambiguity is even
/// checked at its own later conflict-check charge, regardless of `D`
/// sorting before `E` in `type_keys`' ascending identity order.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n06_unreachable_redefinition_target_also_waits_for_every_phase4_charge() {
    let domain_package = fixture_n06_conflict_with_unreachable_redefiner();

    // A `work_units` budget that runs out inside phase 2/3's own charges
    // reports `Incomplete` there, never `E`'s own `RedefinitionTarget`
    // refusal (which an eager `return Err(...)` would report immediately,
    // with zero charges replayed).
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 30;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 30,
                    consumed: 29,
                    next_charge: 2,
                    charge_point: ChargePoint::NormalizeCycleCheck,
                }
            );
        }
        other => panic!("expected Incomplete at a phase 2/3 normalize.cycle-check, got {other:?}"),
    }

    // Under `UNLIMITED`, every phase-4 charge is admitted, and QSL #195:
    // `build` reports every phase-4 refusal in charge order, not only the
    // earliest-charged one, so both `E`'s own `RedefinitionTarget` refusal
    // and `D`'s own conflict are reported, `E`'s own refusal first, since
    // it belongs to the earlier-charged redefinition-check stage (see the
    // doc comment above).
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(refusal, unreachable_redefiner_refusals());
        }
        other => panic!(
            "expected Refused([RedefinitionTarget for E, DerivationConflict for D]), got {other:?}"
        ),
    }

    // The exact `work_units` bound between the last-admitted phase-4 charge
    // and the refusal it exposes: `57` is one short of the one
    // `normalize.conflict-check` charge's own price (`4`, added to `54`
    // already consumed through phase 3, redefinition-check and every
    // phase-4 fact); `58` admits it and reports the refusal -- `E`'s
    // `RedefinitionTarget` refusal, ranked ahead of `D`'s conflict (see the
    // doc comment above).
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 57;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(
                incomplete,
                Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 57,
                    consumed: 54,
                    next_charge: 4,
                    charge_point: ChargePoint::NormalizeConflictCheck,
                }
            );
        }
        other => {
            panic!("expected Incomplete at the normalize.conflict-check charge, got {other:?}")
        }
    }

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.work_units = 58;
    match normalize(&domain_package, limits) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(refusal, unreachable_redefiner_refusals());
        }
        other => {
            panic!("expected Refused([RedefinitionTarget for E, DerivationConflict for D]) at work_units=58, got {other:?}")
        }
    }
}

/// `E`'s own `RedefinitionTarget` refusal (redefinition-check stage) and
/// `D`'s own `derivation-conflict` (conflict-check stage), in charge order
/// -- the whole-outcome shape `fixture_n06_conflict_with_unreachable_redefiner`
/// reports under `UNLIMITED` (QSL #195: every phase-4 refusal, not only the
/// earliest-charged one).
fn unreachable_redefiner_refusals() -> Vec<ModelRefusal> {
    vec![
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
            cause: ModelRefusalCause::RedefinitionTarget {
                redefiners: vec![DeclarationKey::fixture("model.E.y")],
                target: DeclarationKey::fixture("ix://test/orders/A/x"),
            },
            detail:
                "model.E.y redefines ix://test/orders/A/x, which is not a member model.E inherits"
                    .to_string(),
        },
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
            cause: ModelRefusalCause::DerivationConflict {
                type_: DeclarationKey::fixture("ix://test/orders/D"),
                member: DeclarationKey::fixture("ix://test/orders/A/x"),
                redefiners: vec![
                    DeclarationKey::fixture("model.B.x2"),
                    DeclarationKey::fixture("model.C.x3"),
                ],
            },
            detail:
                "type ix://test/orders/D has 2 undominated redefinitions of ix://test/orders/A/x: \
                      [ix://test/orders/B, model.B.x2, ix://test/orders/A/x] and \
                      [ix://test/orders/C, model.C.x3, ix://test/orders/A/x]"
                    .to_string(),
        },
    ]
}

/// The conflict-check owner (`J`) sorts before the redefinition-check owner
/// (`K`) in `type_keys`' ascending order. QSL #195: `build` reports every
/// phase-4 refusal in charge order, not only the earliest-charged one, so
/// both `K`'s own `RedefinitionTarget` refusal -- a
/// `normalize.redefinition-check`-stage refusal (`value-accounting.md:455`)
/// -- and `J`'s own `derivation-conflict` -- a
/// `normalize.conflict-check`-stage refusal (`:456`) -- are reported, with
/// `K`'s own refusal first: every redefinition-check charge precedes every
/// conflict-check charge, exactly as `record_phase4_refusal` ranks them.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n06_redefinition_check_refusal_outranks_earlier_processed_conflict_check_refusal() {
    let domain_package = fixture_conflict_check_owner_sorts_before_redefinition_check_owner();
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                vec![
                    ModelRefusal {
                        code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                        cause: ModelRefusalCause::RedefinitionTarget {
                            redefiners: vec![DeclarationKey::fixture("model.K.k")],
                            target: DeclarationKey::fixture("model.G.g"),
                        },
                        detail:
                            "model.K.k redefines model.G.g, which is not a member model.K inherits"
                                .to_string(),
                    },
                    ModelRefusal {
                        code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                        cause: ModelRefusalCause::DerivationConflict {
                            type_: DeclarationKey::fixture("model.J"),
                            member: DeclarationKey::fixture("model.G.g"),
                            redefiners: vec![
                                DeclarationKey::fixture("model.H.h2"),
                                DeclarationKey::fixture("model.I.i3"),
                            ],
                        },
                        detail: "type model.J has 2 undominated redefinitions of model.G.g: \
                                  [model.H, model.H.h2, model.G.g] and \
                                  [model.I, model.I.i3, model.G.g]"
                            .to_string(),
                    },
                ]
            );
        }
        other => panic!(
            "expected Refused([RedefinitionTarget for K, DerivationConflict for J]), got {other:?}"
        ),
    }
}

/// Two independent conflict-check-stage refusals: `B9`'s own dominance
/// conflict over `M.w` and `D`'s own dominance conflict over `A.x`
/// (`fixture_n06_conflict`). `normalize.conflict-check` charges per type,
/// in `type_keys` order, and only then by target within that type
/// (`value-accounting.md:456`; `model-complete.md:160` puts the owning
/// type first in the effective member key), and `B9` sorts before `D`, so
/// `B9`'s own conflict-check is charged first. QSL #195: `build` reports
/// every phase-4 refusal in charge order, not only the earliest-charged
/// one, so `record_phase4_refusal` reports both, `B9`'s own refusal ahead
/// of `D`'s.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n06_conflict_check_refusal_ranks_by_type_before_target() {
    let domain_package = fixture_n06_conflict_with_a_second_diamond_sorting_first();
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal,
                vec![
                    ModelRefusal {
                        code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                        cause: ModelRefusalCause::DerivationConflict {
                            type_: DeclarationKey::fixture("ix://n06/B9"),
                            member: DeclarationKey::fixture("ix://n06/M/w"),
                            redefiners: vec![
                                DeclarationKey::fixture("ix://n06/M1/w1"),
                                DeclarationKey::fixture("ix://n06/M2/w2"),
                            ],
                        },
                        detail: "type ix://n06/B9 has 2 undominated redefinitions of ix://n06/M/w: \
                                  [ix://n06/M1, ix://n06/M1/w1, ix://n06/M/w] and \
                                  [ix://n06/M2, ix://n06/M2/w2, ix://n06/M/w]"
                            .to_string(),
                    },
                    ModelRefusal {
                        code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                        cause: ModelRefusalCause::DerivationConflict {
                            type_: DeclarationKey::fixture("ix://test/orders/D"),
                            member: DeclarationKey::fixture("ix://test/orders/A/x"),
                            redefiners: vec![
                                DeclarationKey::fixture("model.B.x2"),
                                DeclarationKey::fixture("model.C.x3"),
                            ],
                        },
                        detail: "type ix://test/orders/D has 2 undominated redefinitions of ix://test/orders/A/x: \
                                  [ix://test/orders/B, model.B.x2, ix://test/orders/A/x] and \
                                  [ix://test/orders/C, model.C.x3, ix://test/orders/A/x]"
                            .to_string(),
                    },
                ]
            );
        }
        other => panic!("expected Refused([derivation-conflict for B9, derivation-conflict for D]), got {other:?}"),
    }
}

/// `E`'s own resolution and `Da`'s own resolution (`Da <- E`) both fail the
/// identical "target not an effective member" check for the identical
/// record, so both rank identically -- `Da` sorts before `E`, but the
/// refusal still names `E`, the record's own owning type, since the check
/// always ranks and names the record's own owner, never the resolving
/// `type_key`.
#[trace("TC-195", "FR-150-AC-3", "FR-150-AC-8")]
#[test]
fn n06_unreachable_target_refusal_names_the_records_owning_type_not_a_tied_descendant() {
    let domain_package =
        fixture_unreachable_target_reached_by_owner_and_an_earlier_sorted_descendant();
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(
                refusal,
                ModelRefusal {
                    code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                    cause: ModelRefusalCause::RedefinitionTarget {
                        redefiners: vec![DeclarationKey::fixture("model.E.y")],
                        target: DeclarationKey::fixture("model.A.x"),
                    },
                    detail: "model.E.y redefines model.A.x, which is not a member model.E inherits"
                        .to_string(),
                }
            );
        }
        other => panic!("expected Refused(RedefinitionTarget) naming E, got {other:?}"),
    }
}

// `n06_unreachable_redefiner_refusal_names_the_records_owning_type_not_a_tied_descendant`
// is dropped along with its fixture above; see that comment.

// PR #228 round-1 review findings H1/H2/M1/M2: `check_node` accumulating
// every failing check (H1), the phase-3 walk cap no longer charging closing
// extensions against `derivation_facts` (H2), the same-owner
// `redefinition-target` refusal ranking ahead of `derivation-conflict` (M1),
// and `RedefinitionTarget`'s typed `redefiners` field (M2).

/// H1 finding, PR #228 review: before the fix, `check_node` returned at the
/// first failing check per node, so a field member whose owner AND
/// `redefines` are both dangling reported only `unknown-owner`, silently
/// dropping `unknown-member`. `check_node` now accumulates every failing
/// check in table order (`model-complete.md`:81: owner ahead of
/// `redefines`), so both are reported together.
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn a_field_member_with_a_dangling_owner_and_a_dangling_redefines_reports_both() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.h1a"),
        vec![field_member_redefining(
            "model.A.y",
            "model.nope",
            "ix://test/orders/A",
            Some("model.nope2"),
            vec![],
        )],
    );
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        NormalizeOutcome::Refused(Refusals::from_vec(vec![
            ModelRefusal {
                code: qsl_foundation::diagnostic::Code::DanglingReference,
                cause: ModelRefusalCause::UnknownOwner {
                    member: DeclarationKey::fixture("model.A.y"),
                    owner: DeclarationKey::fixture("model.nope"),
                },
                detail: "field member model.A.y names owner model.nope, which is not a declared object type".to_string(),
            },
            ModelRefusal {
                code: qsl_foundation::diagnostic::Code::DanglingReference,
                cause: ModelRefusalCause::UnknownMember {
                    record: DeclarationKey::fixture("model.A.y"),
                    member: DeclarationKey::fixture("model.nope2"),
                },
                detail: "field member model.A.y redefines model.nope2, which is not a declared field or operation member".to_string(),
            },
        ]))
    );
}

/// H1 finding, PR #228 review: the same accumulation as the test above, for
/// an object type naming two undeclared supertypes -- before the fix, only
/// the first `supertypes[]` entry's `unknown-general` refusal was ever
/// reported.
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn an_object_type_with_two_undeclared_supertypes_reports_both() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.h1b"),
        vec![object_type(
            "model.Orphan",
            vec!["model.nope1", "model.nope2"],
        )],
    );
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        NormalizeOutcome::Refused(Refusals::from_vec(vec![
            ModelRefusal {
                code: qsl_foundation::diagnostic::Code::DanglingReference,
                cause: ModelRefusalCause::UnknownGeneral {
                    supertype: DeclarationKey::fixture("model.Orphan"),
                    general: DeclarationKey::fixture("model.nope1"),
                },
                detail: "object type model.Orphan names supertype model.nope1, which is not a declared object type".to_string(),
            },
            ModelRefusal {
                code: qsl_foundation::diagnostic::Code::DanglingReference,
                cause: ModelRefusalCause::UnknownGeneral {
                    supertype: DeclarationKey::fixture("model.Orphan"),
                    general: DeclarationKey::fixture("model.nope2"),
                },
                detail: "object type model.Orphan names supertype model.nope2, which is not a declared object type".to_string(),
            },
        ]))
    );
}

/// H1 finding, PR #228 review: the `seen_keys` collision check now always
/// runs, even when an earlier check for that same node already failed.
/// Before the fix, a node whose own reference checks already failed
/// returned early and never reached `seen_keys.insert`, so whichever of two
/// same-key records was processed first never registered its key --
/// silently losing `conflicting-binding` regardless of which of the two
/// duplicate records (the dangling-owner one or the well-formed one) is
/// processed first. Both input orders are asserted to produce the identical
/// two-refusal outcome (`model-complete.md`:73: node order is by
/// declaration key, not input order, so two records sharing one key are
/// processed adjacently regardless of which one appears first in
/// `domain_package.records`; only their relative order between themselves,
/// preserved by `sort_by`'s stability, differs).
#[trace("TC-195", "FR-150-AC-3")]
#[test]
fn a_duplicate_key_with_one_dangling_owner_copy_still_reports_conflicting_binding_in_both_input_orders(
) {
    let real = object_type("model.Real", vec![]);
    let bad_owner = field_member("model.Dup.f", "model.nope", "ix://test/orders/A");
    let good_owner = field_member("model.Dup.f", "model.Real", "ix://test/orders/A");

    let expected = NormalizeOutcome::Refused(Refusals::from_vec(vec![
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::DanglingReference,
            cause: ModelRefusalCause::UnknownOwner {
                member: DeclarationKey::fixture("model.Dup.f"),
                owner: DeclarationKey::fixture("model.nope"),
            },
            detail: "field member model.Dup.f names owner model.nope, which is not a declared object type".to_string(),
        },
        ModelRefusal {
            code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
            cause: ModelRefusalCause::ConflictingBinding {
                key: DeclarationKey::fixture("model.Dup.f"),
            },
            detail: "model.Dup.f is declared by more than one record in this domain package"
                .to_string(),
        },
    ]));

    for (order, records) in [
        (
            "bad-owner record first",
            vec![real.clone(), bad_owner.clone(), good_owner.clone()],
        ),
        (
            "bad-owner record second",
            vec![real.clone(), good_owner.clone(), bad_owner.clone()],
        ),
    ] {
        let domain_package = DomainPackage::new(DomainPackageRef::fixture("bundle.h1c"), records);
        assert_eq!(
            normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
            expected,
            "order: {order}"
        );
    }
}

/// H2 finding, PR #228 review: a closing-cycle extension of `ancestor_paths`
/// charges only `normalize.cycle-check`, never `normalize.fact`
/// (`value-accounting.md:491/513`), so it must be capped by
/// `remaining_cycle_budget`'s own `work_units` room alone, never
/// intersected with the real-path `derivation_facts` room
/// `remaining_fact_budget` computes. `A`/`B`/`Z` (`A <- Z`, `B <- Z`,
/// `Z <- A, B`, no members) has exactly two closing cycles (`A`->`Z`->`A`
/// via `Z`->`A`, `Z`->`B`->`Z` via `B`->`Z`) and exactly nine phase-2/3
/// facts -- `derivation_facts = 9` is exactly enough to leave the fact
/// budget unconstrained. The full, unlimited build charges 34 work units
/// total; at `work_units = 33` (N) the walk is one `normalize.cycle-check`
/// charge short and the outcome is `Incomplete`, never a refusal built from
/// a truncated walk; at `work_units = 34` (N+1) both closing cycles are
/// found and reported. Revert-probe (L4 finding, PR #228 round 2 review:
/// this text previously named the fixed code's own condition, not the old
/// one it replaced): the pre-fix loop shared one combined budget for both
/// counts, `if out.len() + closing_cycles.len() >= fact_budget { break; }`,
/// with no separate `cycle_budget` parameter at all -- reintroducing that
/// summed condition here turns `work_units = 33` into a
/// `Refused(SpecializationCycle)` (both closing cycles found one
/// `normalize.cycle-check` charge early, at the combined budget's own
/// `derivation_facts = 9` cap) instead of the `Incomplete` this test
/// asserts -- confirmed locally, then restored.
#[trace("TC-196", "FR-151-AC-2", "FR-150-AC-8")]
#[test]
fn h2_a_closing_cycle_extension_is_capped_by_work_units_room_not_derivation_facts() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.h2-abz"),
        vec![
            object_type("model.A", vec!["model.Z"]),
            object_type("model.B", vec!["model.Z"]),
            object_type("model.Z", vec!["model.A", "model.B"]),
        ],
    );
    let mut limits = ModelNormalizationLimits::UNLIMITED;
    limits.derivation_facts = 9;

    limits.work_units = 33;
    assert_eq!(
        normalize(&domain_package, limits),
        NormalizeOutcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 33,
            consumed: 32,
            next_charge: 2,
            charge_point: ChargePoint::NormalizeCycleCheck,
        }),
        "one normalize.cycle-check short of both closing cycles, with derivation_facts \
         left unconstrained (9 facts exist, budget is 9): must be Incomplete, not a \
         refusal built from a truncated walk"
    );

    limits.work_units = 34;
    assert_eq!(
        normalize(&domain_package, limits),
        NormalizeOutcome::Refused(Refusals::from_vec(vec![
            ModelRefusal {
                code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                cause: ModelRefusalCause::SpecializationCycle {
                    ancestor: DeclarationKey::fixture("model.A"),
                    via: DeclarationKey::fixture("model.Z"),
                },
                detail: "model.A generalizes back to itself via model.Z, through the cycle [model.A, model.Z]".to_string(),
            },
            ModelRefusal {
                code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                cause: ModelRefusalCause::SpecializationCycle {
                    ancestor: DeclarationKey::fixture("model.Z"),
                    via: DeclarationKey::fixture("model.B"),
                },
                detail: "model.Z generalizes back to itself via model.B, through the cycle [model.B, model.Z]".to_string(),
            },
        ])),
        "one normalize.cycle-check charge above the N boundary: both closing cycles found"
    );
}

/// M1 finding, PR #228 review: the same-owner `redefinition-target` refusal
/// (`O` declares `O.a` and `O.b`, both redefining `OA.x`, with no single
/// valid target) must rank `Phase4Rank::RedefinitionCheck`
/// (`value-accounting.md:492`), which sorts ahead of every
/// `Phase4Rank::ConflictCheck` (`:493`) regardless of either candidate's own
/// inner value -- before the fix this branch ranked
/// `Phase4Rank::ConflictCheck(owner_effective_id, target_key)` instead, the
/// identical rank family the `derivation-conflict` candidate below uses, so
/// ordering between them depended on `owner_effective_id`/`target_key`
/// comparison rather than always favoring the redefinition-check. `J`'s own
/// `derivation-conflict` group (`aaa_g`/`aaa_h`/`aaa_i`/`aaa_j`, an
/// unrelated diamond) is named with keys sorting well before `O`'s own
/// group's (`zzz_o`/`zzz_oa`) in plain `DeclarationKey` order -- under the
/// pre-fix ranking, `J`'s `ConflictCheck` candidate would plausibly have
/// sorted first; the fix reports `O`'s `redefinition-target` refusal first
/// regardless.
#[trace("TC-196", "FR-151-AC-2", "FR-150-AC-3")]
#[test]
fn m1_a_same_owner_redefinition_target_refusal_outranks_a_derivation_conflict_with_an_earlier_sorting_key(
) {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.m1"),
        vec![
            object_type("model.aaa_g", vec![]),
            object_type("model.aaa_h", vec!["model.aaa_g"]),
            object_type("model.aaa_i", vec!["model.aaa_g"]),
            object_type("model.aaa_j", vec!["model.aaa_h", "model.aaa_i"]),
            field_member("model.aaa_g.g", "model.aaa_g", "model.aaa_g"),
            field_member_redefining(
                "model.aaa_h.h2",
                "model.aaa_h",
                "model.aaa_g",
                Some("model.aaa_g.g"),
                vec![],
            ),
            field_member_redefining(
                "model.aaa_i.i3",
                "model.aaa_i",
                "model.aaa_g",
                Some("model.aaa_g.g"),
                vec![],
            ),
            object_type("model.zzz_oa", vec![]),
            object_type("model.zzz_o", vec!["model.zzz_oa"]),
            field_member("model.zzz_oa.x", "model.zzz_oa", "model.zzz_oa"),
            field_member_redefining(
                "model.zzz_o.a",
                "model.zzz_o",
                "model.zzz_oa",
                Some("model.zzz_oa.x"),
                vec![],
            ),
            field_member_redefining(
                "model.zzz_o.b",
                "model.zzz_o",
                "model.zzz_oa",
                Some("model.zzz_oa.x"),
                vec![],
            ),
        ],
    );
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        NormalizeOutcome::Refused(Refusals::from_vec(vec![
            ModelRefusal {
                code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                cause: ModelRefusalCause::RedefinitionTarget {
                    redefiners: vec![
                        DeclarationKey::fixture("model.zzz_o.a"),
                        DeclarationKey::fixture("model.zzz_o.b"),
                    ],
                    target: DeclarationKey::fixture("model.zzz_oa.x"),
                },
                detail: "model.zzz_o declares 2 redefining members (model.zzz_o.a, model.zzz_o.b) \
                          that all redefine model.zzz_oa.x, with no single valid target"
                    .to_string(),
            },
            ModelRefusal {
                code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                cause: ModelRefusalCause::DerivationConflict {
                    type_: DeclarationKey::fixture("model.aaa_j"),
                    member: DeclarationKey::fixture("model.aaa_g.g"),
                    redefiners: vec![
                        DeclarationKey::fixture("model.aaa_h.h2"),
                        DeclarationKey::fixture("model.aaa_i.i3"),
                    ],
                },
                detail: "type model.aaa_j has 2 undominated redefinitions of model.aaa_g.g: \
                          [model.aaa_h, model.aaa_h.h2, model.aaa_g.g] and \
                          [model.aaa_i, model.aaa_i.i3, model.aaa_g.g]"
                    .to_string(),
            },
        ]))
    );
}

/// M2 finding, PR #228 review: `RedefinitionTarget`'s `redefiners` field
/// names every redefining member, not only one of them. `E` (no
/// supertypes) declares `E.y` and `E.z`, both `redefines: A.x`; `E` does not
/// inherit `A`, so `A.x` is unreachable from `E`. Both redefiners share the
/// same owner (`E`), so this is exactly one `(owner, target)` group -- one
/// `RedefinitionTarget` refusal listing both, not two (L2 ruling, PR #228
/// round 2 review: an unreachable target's own reachability check is now
/// the *only* check that runs for it -- `resolve_redefinition_contest`'s
/// same-owner ambiguity check never runs for an unreachable group at all,
/// so there is no second, independent refusal left to also fire).
#[trace("TC-196", "FR-151-AC-2", "FR-150-AC-3")]
#[test]
fn m2_two_redefiners_of_an_unreachable_target_are_named_in_one_refusal() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.m2"),
        vec![
            object_type("model.A", vec![]),
            field_member("model.A.x", "model.A", "model.A"),
            object_type("model.E", vec![]),
            field_member_redefining("model.E.y", "model.E", "model.A", Some("model.A.x"), vec![]),
            field_member_redefining("model.E.z", "model.E", "model.A", Some("model.A.x"), vec![]),
        ],
    );
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        NormalizeOutcome::Refused(Refusals::from_vec(vec![ModelRefusal {
            code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
            cause: ModelRefusalCause::RedefinitionTarget {
                redefiners: vec![
                    DeclarationKey::fixture("model.E.y"),
                    DeclarationKey::fixture("model.E.z"),
                ],
                target: DeclarationKey::fixture("model.A.x"),
            },
            detail: "model.E.y, model.E.z redefine model.A.x, which is not a member model.E \
                      inherits"
                .to_string(),
        }]))
    );
}

/// H1 finding, PR #228 round 2 review: the unreachable-target refusal groups
/// by `(owner, target)`, not by `target` alone. `A` declares `A.x`; `B` (no
/// supertypes) declares `B.z` redefining `A.x`; a descendant of `B` (one
/// inheritance line: `A`, `B`, descendant) separately declares its own
/// redefiner of `A.x`. The descendant does not inherit `A` (only `B`, and
/// `B` has no supertypes of its own), so `A.x` is unreachable from the
/// descendant too, and the descendant's own phase-4 query sees *both*
/// owners' edges (its own redefiner, and `B.z` inherited along its one
/// supertype edge) in a single target group -- exactly the shape that
/// grouping by `target` alone conflated. Grouped by `(owner, target)`
/// instead, the descendant's query yields two refusals, `B.z`'s own and its
/// own, each naming only its own owner's redefiner; `B.z`'s own refusal is
/// also independently derived at `B`'s own query (`A.x` is unreachable from
/// `B` on its own), so the two collapse to one by the domain-wide rank
/// dedup below -- itself only correct because both queries now agree on
/// `B.z`'s own owner.
///
/// Run under both namings (L2 ruling, PR #228 round 2 review): `model.D`
/// sorts after `model.B` (`"model.D.w" > "model.B.z"`), `model.AA` sorts
/// before it (`"model.AA.w" < "model.B.z"`) -- the reviewer's own repro
/// used the `AA` ordering, the one where the old rank-only dedup
/// misattributed and duplicated `B.z`. Renaming the descendant changes
/// nothing about which group each redefiner lands in, and nothing about
/// there being exactly two refusals, only their relative order (ascending
/// by each refusal's own least redefining member): grouping is by
/// declaration key identity, not by name.
#[trace("TC-196", "FR-151-AC-2", "FR-150-AC-3")]
#[test]
fn h1_two_owners_on_one_inheritance_line_redefining_an_unreachable_target_are_each_their_own_refusal(
) {
    for descendant in ["model.D", "model.AA"] {
        let redefiner = format!("{descendant}.w");
        let domain_package = DomainPackage::new(
            DomainPackageRef::fixture("bundle.h1"),
            vec![
                object_type("model.A", vec![]),
                field_member("model.A.x", "model.A", "model.A"),
                object_type("model.B", vec![]),
                field_member_redefining(
                    "model.B.z",
                    "model.B",
                    "model.A",
                    Some("model.A.x"),
                    vec![],
                ),
                object_type(descendant, vec!["model.B"]),
                field_member_redefining(
                    &redefiner,
                    descendant,
                    "model.A",
                    Some("model.A.x"),
                    vec![],
                ),
            ],
        );
        let b_refusal = ModelRefusal {
            code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
            cause: ModelRefusalCause::RedefinitionTarget {
                redefiners: vec![DeclarationKey::fixture("model.B.z")],
                target: DeclarationKey::fixture("model.A.x"),
            },
            detail: "model.B.z redefines model.A.x, which is not a member model.B inherits"
                .to_string(),
        };
        let descendant_refusal = ModelRefusal {
            code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
            cause: ModelRefusalCause::RedefinitionTarget {
                redefiners: vec![DeclarationKey::fixture(&redefiner)],
                target: DeclarationKey::fixture("model.A.x"),
            },
            detail: format!(
                "{redefiner} redefines model.A.x, which is not a member {descendant} inherits"
            ),
        };
        // Ascending by each refusal's own least redefining member -- the
        // same `str` order `DeclarationKey`'s own `Ord` uses over `node`
        // (both share `test/orders` as `package`, so this is exactly that
        // comparison).
        let expected = if redefiner.as_str() < "model.B.z" {
            vec![descendant_refusal, b_refusal]
        } else {
            vec![b_refusal, descendant_refusal]
        };
        assert_eq!(
            normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
            NormalizeOutcome::Refused(Refusals::from_vec(expected)),
            "descendant: {descendant}"
        );
    }
}

/// TEST GAP (#193, PR #228 review): `n06_specialization_cycle_refusal_waits_for_every_phase3_charge_to_admit`
/// only swept `work_units` from 18, the point phase 2's own `normalize.fact`
/// charges begin, silently skipping phase 1's own `normalize.record`
/// charges (0 through 8, one per this fixture's nine records) entirely.
/// Swept from `work_units = 0`, asserting the spec's own charge order
/// precisely, not merely charge-point *class* membership: every stop from
/// `0` through `8` is `normalize.record` (nine records: `A`, `B`, `C`, `D`,
/// `A.x`, `B.x2`, `C.x3`, `Y`, `Z`), and every stop from `9` through `17` is
/// `normalize.fact`, before phase 3's own facts and cycle-checks interleave
/// from `18` on (already covered by the existing sweep below).
#[trace("TC-195", "TC-196", "FR-150-AC-3", "FR-150-AC-8", "FR-151-AC-2")]
#[test]
fn n06_budget_sweep_from_zero_stops_in_spec_charge_order() {
    let domain_package = fixture_n06_conflict_with_unrelated_cycle();

    for work_units in 0u64..=8 {
        let mut limits = ModelNormalizationLimits::UNLIMITED;
        limits.work_units = work_units;
        match normalize(&domain_package, limits) {
            NormalizeOutcome::Incomplete(incomplete) => {
                assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
                assert_eq!(incomplete.limit, work_units);
                assert_eq!(incomplete.consumed, work_units);
                assert_eq!(
                    incomplete.charge_point,
                    ChargePoint::NormalizeRecord,
                    "work_units={work_units} must still be stopped on a normalize.record \
                     charge (nine records exist)"
                );
            }
            other => panic!("expected Incomplete at work_units={work_units}, got {other:?}"),
        }
    }

    for work_units in 9u64..=17 {
        let mut limits = ModelNormalizationLimits::UNLIMITED;
        limits.work_units = work_units;
        match normalize(&domain_package, limits) {
            NormalizeOutcome::Incomplete(incomplete) => {
                assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
                assert_eq!(incomplete.limit, work_units);
                assert_eq!(incomplete.consumed, work_units);
                assert_eq!(
                    incomplete.charge_point,
                    ChargePoint::NormalizeFact,
                    "work_units={work_units} must have admitted every normalize.record \
                     charge and be stopped on a normalize.fact charge"
                );
            }
            other => panic!("expected Incomplete at work_units={work_units}, got {other:?}"),
        }
    }
}

/// TEST GAP (#193, PR #228 review): two distinct closing cycles (`A`<->`B`
/// and `Y`<->`Z`, unrelated to one another) in the same domain package are
/// both reported, in charge order -- `A`/`B`'s own cycle, whose least key
/// (`model.A`) sorts before `Y`/`Z`'s own (`model.Y`), is reported first.
#[trace("TC-196", "FR-151-AC-2")]
#[test]
fn two_distinct_cycles_are_both_reported_in_charge_order() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.two-cycles"),
        vec![
            object_type("model.A", vec!["model.B"]),
            object_type("model.B", vec!["model.A"]),
            object_type("model.Y", vec!["model.Z"]),
            object_type("model.Z", vec!["model.Y"]),
        ],
    );
    assert_eq!(
        normalize(&domain_package, ModelNormalizationLimits::UNLIMITED),
        NormalizeOutcome::Refused(Refusals::from_vec(vec![
            ModelRefusal {
                code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                cause: ModelRefusalCause::SpecializationCycle {
                    ancestor: DeclarationKey::fixture("model.A"),
                    via: DeclarationKey::fixture("model.B"),
                },
                detail: "model.A generalizes back to itself via model.B, through the cycle [model.A, model.B]".to_string(),
            },
            ModelRefusal {
                code: qsl_foundation::diagnostic::Code::InvalidModelBinding,
                cause: ModelRefusalCause::SpecializationCycle {
                    ancestor: DeclarationKey::fixture("model.Y"),
                    via: DeclarationKey::fixture("model.Z"),
                },
                detail: "model.Y generalizes back to itself via model.Z, through the cycle [model.Y, model.Z]".to_string(),
            },
        ]))
    );
}

/// TEST GAP (#193, PR #228 review): pins R01's own exact charge sequence and
/// work-unit total for the closing `model.A`<->`model.B` mutual-cycle
/// fixture -- `r01_a_closing_generalization_cycle_names_the_full_rotated_chain`'s
/// own doc comment explicitly flagged this as not reproduced. Two records
/// (`A`, `B`), two phase-2 qualify facts, four phase-3
/// `normalize.cycle-check` charges (`A`'s own walk closes once via `B`;
/// `B`'s own walk closes once via `A`, needing two cycle-checks of its own
/// -- the exact interleaving below, captured from the real charge log, not
/// derived by hand), and twelve work units total.
#[trace("TC-196", "FR-151-AC-2", "FR-150-AC-8")]
#[test]
fn r01_pins_the_full_cycle_check_charge_and_work_unit_accounting() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.r01"),
        vec![
            object_type("model.A", vec!["model.B"]),
            object_type("model.B", vec!["model.A"]),
        ],
    );
    let (outcome, meter) =
        normalize_with_meter(&domain_package, ModelNormalizationLimits::UNLIMITED);
    assert!(
        matches!(outcome, NormalizeOutcome::Refused(_)),
        "expected Refused, got {outcome:?}"
    );
    use ChargePoint::{NormalizeCycleCheck, NormalizeFact, NormalizeRecord};
    assert_eq!(
        meter.admitted_charges().to_vec(),
        vec![
            NormalizeRecord,
            NormalizeRecord,
            NormalizeFact,
            NormalizeFact,
            NormalizeCycleCheck,
            NormalizeFact,
            NormalizeCycleCheck,
            NormalizeCycleCheck,
            NormalizeFact,
            NormalizeCycleCheck,
        ]
    );
    let cycle_check_count = meter
        .admitted_charges()
        .iter()
        .filter(|point| **point == ChargePoint::NormalizeCycleCheck)
        .count();
    assert_eq!(cycle_check_count, 4);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 12);
    assert_eq!(meter.consumed(LimitKind::HashedBytes), 0);
}

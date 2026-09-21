// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-192 integer division profiles over the real `value` boundary.
//!
//! The signed table and DIV-01–DIV-13 exercise every closed division law;
//! every `DefinitionRef` is built from the compiled-in lock's own catalog
//! entries rather than authored ad hoc.

use std::num::NonZeroU32;

use ix_trace_rs::trace;
use quire_exact::{Integer, IntegerDomain, IntegerInterval};
use quire_spec_language::value::{
    divide, modulo, AdmittedIntegerDivision, CatalogRole, ChargePoint, DefinitionLock,
    DefinitionReference, DefinitionRevision, DivisionProfile, Incomplete, InjectedDenial,
    LimitKind, Meter, Outcome, PackageCause, PackageRefusal, PackageRefusalCode, QuotientRemainder,
    Refusal, ScalarLimits, Undefined,
};

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn lock() -> &'static DefinitionLock {
    DefinitionLock::pinned()
}

fn role(profile: DivisionProfile) -> CatalogRole {
    match profile {
        DivisionProfile::Truncating => CatalogRole::IntegerDivisionTruncating,
        DivisionProfile::Floor => CatalogRole::IntegerDivisionFloor,
        DivisionProfile::Euclidean => CatalogRole::IntegerDivisionEuclidean,
    }
}

/// A well-formed [`DefinitionReference`] for `role`, built from the catalog's
/// own identity/authority/revision fields. There is no digest to carry over:
/// the catalog holds none, so this uses a placeholder that admission never
/// inspects.
fn reference(role: CatalogRole) -> DefinitionReference {
    let entry = lock().entry(role).unwrap();
    DefinitionReference {
        authority: entry.authority.to_owned(),
        identity: entry.identity.to_owned(),
        revision: DefinitionRevision {
            namespace: entry.revision_namespace.to_owned(),
            value: entry.revision_value.to_owned(),
        },
        digest_domain: "quire.definition.bytes/v1".to_owned(),
        digest: "0".repeat(64),
    }
}

fn admitted(profile: DivisionProfile) -> AdmittedIntegerDivision {
    let admitted = lock()
        .admit_integer_division(&[reference(role(profile))], None)
        .unwrap();
    assert_eq!(admitted.profile(), profile);
    assert_eq!(
        reference(role(profile)).identity,
        profile.definition_identity()
    );
    admitted
}

fn int(value: &Integer) -> i128 {
    value.to_string().parse().unwrap()
}

fn big(value: i128) -> Integer {
    Integer::from(value)
}

fn run(
    profile: DivisionProfile,
    a: i128,
    b: i128,
    domain: &IntegerDomain,
) -> Outcome<QuotientRemainder> {
    divide(
        &admitted(profile),
        &big(a),
        &big(b),
        domain,
        &mut Meter::new(UNLIMITED),
    )
}

fn pair(outcome: Outcome<QuotientRemainder>) -> (i128, i128) {
    match outcome {
        Outcome::Completed(result) => (int(result.quotient()), int(result.remainder())),
        other => panic!("expected a completed pair, got {other:?}"),
    }
}

/// Independent law check: identity plus the profile's remainder constraint.
fn assert_law(profile: DivisionProfile, a: i128, b: i128, (q, r): (i128, i128)) {
    assert_eq!(a, b * q + r, "{profile:?} ({a},{b})");
    assert!(r.abs() < b.abs(), "{profile:?} ({a},{b})");
    match profile {
        DivisionProfile::Truncating => assert!(r == 0 || (r < 0) == (a < 0)),
        DivisionProfile::Floor => assert!(r == 0 || (r < 0) == (b < 0)),
        DivisionProfile::Euclidean => assert!(r >= 0),
    }
}

#[trace("TC-192", "FR-147-AC-1", "FR-147-AC-4")]
#[test]
fn signed_table_distinguishes_the_three_laws() {
    let operands = [(7, 3), (7, -3), (-7, 3), (-7, -3)];
    let table = [
        (
            DivisionProfile::Truncating,
            [(2, 1), (-2, 1), (-2, -1), (2, -1)],
        ),
        (DivisionProfile::Floor, [(2, 1), (-3, -2), (-3, 2), (2, -1)]),
        (
            DivisionProfile::Euclidean,
            [(2, 1), (-2, 1), (-3, 2), (3, 2)],
        ),
    ];
    for (profile, expected) in table {
        for ((a, b), cell) in operands.into_iter().zip(expected) {
            let actual = pair(run(profile, a, b, &IntegerDomain::Mathematical));
            assert_eq!(actual, cell, "{profile:?} ({a},{b})");
            assert_law(profile, a, b, actual);
        }
    }
}

#[trace("TC-192", "FR-147-AC-2", "FR-147-AC-5")]
#[test]
fn div_01_zero_divisors_are_undefined_for_every_law_and_mod() {
    for profile in DivisionProfile::ALL {
        for a in [-7, 0, 7] {
            let mut meter = Meter::new(UNLIMITED);
            assert_eq!(
                divide(
                    &admitted(profile),
                    &big(a),
                    &big(0),
                    &IntegerDomain::Mathematical,
                    &mut meter
                ),
                Outcome::Undefined(Undefined::DivisionByZero)
            );
            assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
            assert_eq!(
                modulo(
                    &big(a),
                    &big(0),
                    &IntegerDomain::Mathematical,
                    &mut Meter::new(UNLIMITED)
                ),
                Outcome::Undefined(Undefined::DivisionByZero)
            );
        }
    }
}

#[trace("TC-192", "FR-147-AC-5")]
#[test]
fn div_02_div_03_mod_is_euclidean_and_non_euclidean_claims_refuse() {
    for profile in DivisionProfile::ALL {
        // Selecting any div/rem law leaves mod unchanged.
        let _selected = admitted(profile);
        for ((a, b), expected) in [((-7, 3), 2), ((7, -3), 1), ((-7, -3), 2), ((7, 3), 1)] {
            let outcome = modulo(
                &big(a),
                &big(b),
                &IntegerDomain::Mathematical,
                &mut Meter::new(UNLIMITED),
            );
            assert_eq!(outcome.completed().map(|r| int(&r)), Some(expected));
        }
    }
    let euclidean = reference(CatalogRole::IntegerDivisionEuclidean);
    for law in [
        DivisionProfile::Truncating,
        DivisionProfile::Floor,
        DivisionProfile::Euclidean,
    ] {
        assert!(lock()
            .admit_integer_division(&[reference(role(law))], Some(&euclidean))
            .is_ok());
    }
    for claim in [
        CatalogRole::IntegerDivisionTruncating,
        CatalogRole::IntegerDivisionFloor,
    ] {
        assert_eq!(
            lock().admit_integer_division(
                &[reference(CatalogRole::IntegerDivisionFloor)],
                Some(&reference(claim))
            ),
            Err(PackageRefusal {
                code: PackageRefusalCode::InvalidPackage,
                cause: PackageCause::IncompatibleDefinition,
            })
        );
    }
}

fn signed_64() -> IntegerDomain {
    IntegerDomain::Bounded(IntegerInterval::signed_twos_complement(
        NonZeroU32::new(64).unwrap(),
    ))
}

#[trace("TC-192", "FR-147-AC-1", "FR-147-AC-4")]
#[test]
fn div_04_div_06_mathematical_and_signed_64_domains() {
    let (min, max) = (i128::from(i64::MIN), i128::from(i64::MAX));
    for profile in DivisionProfile::ALL {
        assert_eq!(
            pair(run(profile, min, -1, &IntegerDomain::Mathematical)),
            (9_223_372_036_854_775_808, 0)
        );
        let mut meter = Meter::new(UNLIMITED);
        assert_eq!(
            divide(
                &admitted(profile),
                &big(min),
                &big(-1),
                &signed_64(),
                &mut meter
            ),
            Outcome::Refused(Refusal::DivisionPairOutOfDomain {
                quotient_admitted: false,
                remainder_admitted: true,
            })
        );
        assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
        assert_eq!(pair(run(profile, min, 1, &signed_64())), (min, 0));
        assert_eq!(pair(run(profile, max, 1, &signed_64())), (max, 0));
        for outside in [min - 1, max + 1] {
            assert!(matches!(
                run(profile, outside, 1, &signed_64()),
                Outcome::Refused(Refusal::DivisionPairOutOfDomain {
                    quotient_admitted: false,
                    remainder_admitted: true,
                })
            ));
        }
    }
}

const DIV_08: ScalarLimits = ScalarLimits {
    integer_bits: 3,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 2,
    work_units: 4,
    result_units: 2,
};

fn div_08(meter: &mut Meter) -> Outcome<QuotientRemainder> {
    divide(
        &admitted(DivisionProfile::Truncating),
        &big(7),
        &big(3),
        &IntegerDomain::Mathematical,
        meter,
    )
}

#[trace("TC-192", "FR-147-AC-6")]
#[test]
fn div_08_exact_bound_succeeds_and_each_named_denial_is_atomic() {
    let mut meter = Meter::new(DIV_08);
    assert_eq!(pair(div_08(&mut meter)), (2, 1));
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::IntegerDivisionOperands,
            ChargePoint::IntegerDivisionArithmetic,
            ChargePoint::IntegerDivisionDomainPair,
            ChargePoint::IntegerDivisionResultPair,
        ]
    );
    let consumed: Vec<_> = LimitKind::ALL.map(|kind| meter.consumed(kind)).to_vec();
    assert_eq!(consumed, [3, 0, 0, 0, 0, 0, 0, 2, 4, 2]);

    assert_eq!(
        div_08(&mut Meter::new(ScalarLimits {
            work_units: 3,
            ..DIV_08
        })),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 3,
            consumed: 3,
            next_charge: big(1),
            charge_point: ChargePoint::IntegerDivisionResultPair,
        })
    );
    assert_eq!(
        div_08(&mut Meter::new(ScalarLimits {
            result_units: 1,
            ..DIV_08
        })),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::ResultUnits,
            limit: 1,
            consumed: 0,
            next_charge: big(2),
            charge_point: ChargePoint::IntegerDivisionResultPair,
        })
    );
    for (work, point) in (0_u64..).zip(meter.admitted_charges()) {
        let mut denied = Meter::new(DIV_08).with_injected_denial(InjectedDenial {
            point: *point,
            occurrence: 1,
        });
        assert_eq!(div_08(&mut denied), work_denied(work, *point));
        assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
    }
}

/// The injected-denial record after `work` admitted work units.
fn work_denied<T>(work: u64, point: ChargePoint) -> Outcome<T> {
    Outcome::Incomplete(Incomplete {
        limit_kind: LimitKind::WorkUnits,
        limit: work,
        consumed: work,
        next_charge: big(1),
        charge_point: point,
    })
}

const DIV_10: ScalarLimits = ScalarLimits {
    result_units: 1,
    ..DIV_08
};

fn mod_10(domain: &IntegerDomain, meter: &mut Meter) -> Outcome<Integer> {
    modulo(&big(-7), &big(3), domain, meter)
}

#[trace("TC-192", "FR-147-AC-5", "FR-147-AC-6")]
#[test]
fn div_10_mod_charges_only_the_integer_modulus_points() {
    let mut meter = Meter::new(DIV_10);
    assert_eq!(
        mod_10(&IntegerDomain::Mathematical, &mut meter).completed(),
        Some(big(2))
    );
    let points = [
        ChargePoint::IntegerModulusOperands,
        ChargePoint::IntegerModulusArithmetic,
        ChargePoint::IntegerModulusDomain,
        ChargePoint::IntegerModulusResultRetain,
    ];
    assert_eq!(meter.admitted_charges(), points);
    for (work, point) in (0_u64..).zip(points) {
        let mut denied = Meter::new(DIV_10).with_injected_denial(InjectedDenial {
            point,
            occurrence: 1,
        });
        assert_eq!(
            mod_10(&IntegerDomain::Mathematical, &mut denied),
            work_denied(work, point)
        );
        assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
    }
}

#[trace("TC-192", "FR-147-AC-2", "FR-147-AC-6")]
#[test]
fn div_11_zero_divisors_are_undefined_after_the_operands_charge() {
    let one = |limits: ScalarLimits| ScalarLimits {
        work_units: 1,
        ..limits
    };
    let mut meter = Meter::new(one(DIV_08));
    assert_eq!(
        divide(
            &admitted(DivisionProfile::Truncating),
            &big(7),
            &big(0),
            &IntegerDomain::Mathematical,
            &mut meter
        ),
        Outcome::Undefined(Undefined::DivisionByZero)
    );
    assert_eq!(
        meter.admitted_charges(),
        [ChargePoint::IntegerDivisionOperands]
    );
    let mut meter = Meter::new(one(DIV_10));
    assert_eq!(
        modulo(&big(7), &big(0), &IntegerDomain::Mathematical, &mut meter),
        Outcome::Undefined(Undefined::DivisionByZero)
    );
    assert_eq!(
        meter.admitted_charges(),
        [ChargePoint::IntegerModulusOperands]
    );

    let zero = |limits: ScalarLimits| ScalarLimits {
        work_units: 0,
        ..limits
    };
    assert_eq!(
        divide(
            &admitted(DivisionProfile::Truncating),
            &big(7),
            &big(0),
            &IntegerDomain::Mathematical,
            &mut Meter::new(zero(DIV_08))
        ),
        work_denied(0, ChargePoint::IntegerDivisionOperands)
    );
    assert_eq!(
        modulo(
            &big(7),
            &big(0),
            &IntegerDomain::Mathematical,
            &mut Meter::new(zero(DIV_10))
        ),
        work_denied(0, ChargePoint::IntegerModulusOperands)
    );
}

#[trace("TC-192", "FR-147-AC-5", "FR-147-AC-6")]
#[test]
fn div_12_mod_domain_refusal_precedes_the_retain_charge() {
    let unit_interval = IntegerDomain::Bounded(IntegerInterval::new(big(0), big(1)).unwrap());
    let limits = ScalarLimits {
        result_units: 0,
        ..DIV_10
    };
    let mut meter = Meter::new(limits);
    assert_eq!(
        mod_10(&unit_interval, &mut meter),
        Outcome::Refused(Refusal::ModuloOutOfDomain)
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::IntegerModulusOperands,
            ChargePoint::IntegerModulusArithmetic,
            ChargePoint::IntegerModulusDomain,
        ]
    );
    assert_eq!(
        mod_10(
            &unit_interval,
            &mut Meter::new(ScalarLimits {
                work_units: 2,
                ..limits
            })
        ),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 2,
            consumed: 2,
            next_charge: big(1),
            charge_point: ChargePoint::IntegerModulusDomain,
        })
    );
}

#[trace("TC-192", "FR-147-AC-6")]
#[test]
fn div_13_the_first_short_counter_in_field_order_is_reported() {
    assert_eq!(
        div_08(&mut Meter::new(ScalarLimits {
            integer_bits: 2,
            work_units: 0,
            ..DIV_08
        })),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::IntegerBits,
            limit: 2,
            consumed: 0,
            next_charge: big(3),
            charge_point: ChargePoint::IntegerDivisionOperands,
        })
    );
}

#[trace("TC-192")]
#[test]
fn div_09_missing_conflicting_or_stale_division_definitions_refuse_admission() {
    let refuse = |cause| {
        Err(PackageRefusal {
            code: PackageRefusalCode::InvalidPackage,
            cause,
        })
    };
    let floor = reference(CatalogRole::IntegerDivisionFloor);
    assert_eq!(
        lock().admit_integer_division(&[], None),
        refuse(PackageCause::MissingMember)
    );
    assert_eq!(
        lock().admit_integer_division(
            &[
                floor.clone(),
                reference(CatalogRole::IntegerDivisionEuclidean)
            ],
            None
        ),
        refuse(PackageCause::ConflictingDefinition)
    );
    let mut stale = floor.clone();
    stale.revision.value = "1-draft.2".into();
    assert_eq!(
        lock().admit_integer_division(&[stale], None),
        refuse(PackageCause::RevisionMismatch)
    );
    let mut wrong_domain = floor.clone();
    wrong_domain.digest_domain = "quire.definition.jcs/v1".into();
    assert_eq!(
        lock().admit_integer_division(&[wrong_domain], None),
        refuse(PackageCause::DigestDomainMismatch)
    );
    assert_eq!(
        lock().admit_integer_division(&[reference(CatalogRole::Root)], None),
        refuse(PackageCause::IncompatibleDefinition)
    );
}

// ---- generated vectors -----------------------------------------------------

fn oracle(profile: DivisionProfile, a: i128, b: i128) -> (i128, i128) {
    let (q, r) = (a / b, a % b);
    match profile {
        DivisionProfile::Truncating => (q, r),
        DivisionProfile::Floor if r != 0 && (r < 0) != (b < 0) => (q - 1, r + b),
        DivisionProfile::Floor => (q, r),
        DivisionProfile::Euclidean if r < 0 && b > 0 => (q - 1, r + b),
        DivisionProfile::Euclidean if r < 0 => (q + 1, r - b),
        DivisionProfile::Euclidean => (q, r),
    }
}

#[trace(
    "TC-192",
    "FR-147-AC-1",
    "FR-147-AC-2",
    "FR-147-AC-4",
    "FR-147-AC-5",
    "FR-147-AC-6"
)]
#[test]
fn generated_pairs_match_the_law_oracle_domains_and_every_denial() {
    let bounded = IntegerDomain::Bounded(IntegerInterval::new(big(-5), big(5)).unwrap());
    for profile in DivisionProfile::ALL {
        let selection = admitted(profile);
        for a in -20..=20 {
            for b in -20..=20 {
                let mut meter = Meter::new(UNLIMITED);
                let outcome = divide(
                    &selection,
                    &big(a),
                    &big(b),
                    &IntegerDomain::Mathematical,
                    &mut meter,
                );
                let euclid = modulo(&big(a), &big(b), &bounded, &mut Meter::new(UNLIMITED));
                if b == 0 {
                    assert_eq!(outcome, Outcome::Undefined(Undefined::DivisionByZero));
                    assert_eq!(euclid, Outcome::Undefined(Undefined::DivisionByZero));
                    continue;
                }
                let expected = oracle(profile, a, b);
                assert_law(profile, a, b, expected);
                assert_eq!(pair(outcome), expected, "{profile:?} ({a},{b})");
                let euclidean = oracle(DivisionProfile::Euclidean, a, b).1;
                if euclidean.abs() <= 5 {
                    assert_eq!(euclid.completed().map(|r| int(&r)), Some(euclidean));
                } else {
                    assert_eq!(euclid, Outcome::Refused(Refusal::ModuloOutOfDomain));
                }

                let in_bounds = |value: i128| value.abs() <= 5;
                match divide(
                    &selection,
                    &big(a),
                    &big(b),
                    &bounded,
                    &mut Meter::new(UNLIMITED),
                ) {
                    Outcome::Completed(result) => {
                        assert!(in_bounds(expected.0) && in_bounds(expected.1));
                        assert_eq!((int(result.quotient()), int(result.remainder())), expected);
                    }
                    Outcome::Refused(Refusal::DivisionPairOutOfDomain {
                        quotient_admitted,
                        remainder_admitted,
                    }) => {
                        assert_eq!(
                            (quotient_admitted, remainder_admitted),
                            (in_bounds(expected.0), in_bounds(expected.1))
                        );
                        assert!(!(quotient_admitted && remainder_admitted));
                    }
                    other => panic!("{profile:?} ({a},{b}): {other:?}"),
                }

                for point in meter.admitted_charges() {
                    let mut denied = Meter::new(UNLIMITED).with_injected_denial(InjectedDenial {
                        point: *point,
                        occurrence: 1,
                    });
                    assert!(matches!(
                        divide(&selection, &big(a), &big(b), &IntegerDomain::Mathematical, &mut denied),
                        Outcome::Incomplete(Incomplete { charge_point, .. }) if charge_point == *point
                    ));
                    assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
                }
            }
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-192 integer division profiles over the real `value` boundary.
//!
//! The signed table and DIV-01–DIV-13 exercise every closed division law;
//! every `DefinitionRef` is built from the catalog's own entries rather than
//! authored ad hoc.

use std::num::NonZeroU32;

use ix_trace_rs::trace;
use qsl_semantics::value::{
    divide, modulo, AdmittedIntegerDivision, CatalogRole, DefinitionLock, DefinitionReference,
};
use quire_exact::{
    ChargePoint, DivisionMember, DivisionProfile, Incomplete, InjectedDenial, Integer,
    IntegerDomain, IntegerInterval, LimitKind, Meter, Outcome, Refusal, ScalarLimits, Undefined,
};
use quire_semantic_value::definition::{PackageCause, PackageRefusal, PackageRefusalCode};

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

fn lock() -> DefinitionLock {
    DefinitionLock::pinned()
}

fn role(profile: DivisionProfile) -> CatalogRole {
    match profile {
        DivisionProfile::Truncating => CatalogRole::IntegerDivisionTruncating,
        DivisionProfile::Floor => CatalogRole::IntegerDivisionFloor,
        DivisionProfile::Euclidean => CatalogRole::IntegerDivisionEuclidean,
    }
}

/// The [`DefinitionReference`] for `role`: the catalog entry's
/// `{authority, identity}`.
fn reference(role: CatalogRole) -> DefinitionReference {
    lock().entry(role).reference()
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
    member: DivisionMember,
    a: i128,
    b: i128,
    domain: &IntegerDomain,
) -> Outcome<Integer> {
    divide(
        &admitted(profile),
        member,
        &big(a),
        &big(b),
        domain,
        &mut Meter::new(UNLIMITED),
    )
}

fn completed(outcome: Outcome<Integer>) -> i128 {
    match outcome {
        Outcome::Completed(result) => int(&result),
        other => panic!("expected a completed member, got {other:?}"),
    }
}

/// `(a div b, a rem b)` under `profile`, each exposed on its own over the
/// mathematical domain.
fn pair(profile: DivisionProfile, a: i128, b: i128) -> (i128, i128) {
    let member = |member| completed(run(profile, member, a, b, &IntegerDomain::Mathematical));
    (
        member(DivisionMember::Quotient),
        member(DivisionMember::Remainder),
    )
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

#[trace(
    "QSpec-TC-192",
    "QSpec-FR-147-AC-1",
    "QSpec-FR-147-AC-4",
    "TC-202",
    "FR-078-AC-3"
)]
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
            let actual = pair(profile, a, b);
            assert_eq!(actual, cell, "{profile:?} ({a},{b})");
            assert_law(profile, a, b, actual);
        }
    }
}

#[trace(
    "QSpec-TC-192",
    "QSpec-FR-147-AC-2",
    "QSpec-FR-147-AC-5",
    "TC-202",
    "FR-078-AC-3"
)]
#[test]
fn div_01_zero_divisors_are_undefined_for_every_law_and_mod() {
    for profile in DivisionProfile::ALL {
        for a in [-7, 0, 7] {
            let mut meter = Meter::new(UNLIMITED);
            assert_eq!(
                divide(
                    &admitted(profile),
                    DivisionMember::Quotient,
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

#[trace("QSpec-TC-192", "QSpec-FR-147-AC-5", "TC-202", "FR-078-AC-3")]
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

#[trace(
    "QSpec-TC-192",
    "QSpec-FR-147-AC-1",
    "QSpec-FR-147-AC-4",
    "TC-202",
    "FR-078-AC-3"
)]
#[test]
fn div_04_div_06_mathematical_and_signed_64_domains() {
    let (min, max) = (i128::from(i64::MIN), i128::from(i64::MAX));
    let signed_64_interval =
        || IntegerInterval::signed_twos_complement(NonZeroU32::new(64).unwrap());
    for profile in DivisionProfile::ALL {
        assert_eq!(pair(profile, min, -1), (9_223_372_036_854_775_808, 0));
        // Only the exposed member must be in the domain: `min div -1` is
        // outside signed 64, `min rem -1` is 0 and inside.
        let mut meter = Meter::new(UNLIMITED);
        assert_eq!(
            divide(
                &admitted(profile),
                DivisionMember::Quotient,
                &big(min),
                &big(-1),
                &signed_64(),
                &mut meter
            ),
            Outcome::Refused(Refusal::DivisionOutOfDomain {
                domain: Box::new(signed_64_interval()),
                member: DivisionMember::Quotient,
            })
        );
        assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
        assert_eq!(
            completed(run(
                profile,
                DivisionMember::Remainder,
                min,
                -1,
                &signed_64()
            )),
            0
        );
        for member in [DivisionMember::Quotient, DivisionMember::Remainder] {
            let expected = match member {
                DivisionMember::Quotient => (min, max),
                DivisionMember::Remainder => (0, 0),
            };
            assert_eq!(
                completed(run(profile, member, min, 1, &signed_64())),
                expected.0
            );
            assert_eq!(
                completed(run(profile, member, max, 1, &signed_64())),
                expected.1
            );
        }
        for outside in [min - 1, max + 1] {
            assert_eq!(
                run(profile, DivisionMember::Quotient, outside, 1, &signed_64()),
                Outcome::Refused(Refusal::DivisionOutOfDomain {
                    domain: Box::new(signed_64_interval()),
                    member: DivisionMember::Quotient,
                })
            );
            assert_eq!(
                completed(run(
                    profile,
                    DivisionMember::Remainder,
                    outside,
                    1,
                    &signed_64()
                )),
                0
            );
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
    result_units: 1,
};

fn div_08(meter: &mut Meter) -> Outcome<Integer> {
    divide(
        &admitted(DivisionProfile::Truncating),
        DivisionMember::Quotient,
        &big(7),
        &big(3),
        &IntegerDomain::Mathematical,
        meter,
    )
}

#[trace("QSpec-TC-192", "QSpec-FR-147-AC-6", "TC-202", "FR-078-AC-3")]
#[test]
fn div_08_exact_bound_succeeds_and_each_named_denial_is_atomic() {
    let mut meter = Meter::new(DIV_08);
    assert_eq!(completed(div_08(&mut meter)), 2);
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::IntegerDivisionOperands,
            ChargePoint::IntegerDivisionArithmetic,
            ChargePoint::IntegerDivisionDomain,
            ChargePoint::IntegerDivisionResultRetain,
        ]
    );
    let consumed: Vec<_> = LimitKind::ALL.map(|kind| meter.consumed(kind)).to_vec();
    assert_eq!(consumed, [3, 0, 0, 0, 0, 0, 0, 2, 4, 1]);

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
            charge_point: ChargePoint::IntegerDivisionResultRetain,
        })
    );
    assert_eq!(
        div_08(&mut Meter::new(ScalarLimits {
            result_units: 0,
            ..DIV_08
        })),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::ResultUnits,
            limit: 0,
            consumed: 0,
            next_charge: big(1),
            charge_point: ChargePoint::IntegerDivisionResultRetain,
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

#[trace(
    "QSpec-TC-192",
    "QSpec-FR-147-AC-5",
    "QSpec-FR-147-AC-6",
    "TC-202",
    "FR-078-AC-3"
)]
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

#[trace(
    "QSpec-TC-192",
    "QSpec-FR-147-AC-2",
    "QSpec-FR-147-AC-6",
    "TC-202",
    "FR-078-AC-3"
)]
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
            DivisionMember::Quotient,
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
            DivisionMember::Quotient,
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

#[trace(
    "QSpec-TC-192",
    "QSpec-FR-147-AC-5",
    "QSpec-FR-147-AC-6",
    "TC-202",
    "FR-078-AC-3"
)]
#[test]
fn div_12_mod_domain_refusal_precedes_the_retain_charge() {
    let unit_interval = IntegerDomain::Bounded(IntegerInterval::new(big(0), big(1)).unwrap());
    let limits = ScalarLimits {
        result_units: 0,
        ..DIV_10
    };
    let mut meter = Meter::new(limits);
    assert!(matches!(
        mod_10(&unit_interval, &mut meter),
        Outcome::Refused(Refusal::ModuloOutOfDomain { .. })
    ));
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

#[trace("QSpec-TC-192", "QSpec-FR-147-AC-6", "TC-202", "FR-078-AC-3")]
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

#[trace("QSpec-TC-192", "TC-202", "FR-078-AC-3")]
#[test]
fn div_09_missing_conflicting_or_foreign_division_definitions_refuse_admission() {
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
    // A definition is `{authority, identity}`: the catalogued identity under
    // another authority is another definition.
    let mut foreign = floor.clone();
    foreign.authority = "other".into();
    assert_eq!(
        lock().admit_integer_division(&[foreign], None),
        refuse(PackageCause::IncompatibleDefinition)
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
    "QSpec-TC-192",
    "QSpec-FR-147-AC-1",
    "QSpec-FR-147-AC-2",
    "QSpec-FR-147-AC-4",
    "QSpec-FR-147-AC-5",
    "QSpec-FR-147-AC-6",
    "TC-202",
    "FR-078-AC-3"
)]
#[test]
fn generated_members_match_the_law_oracle_domains_and_every_denial() {
    let bounded = IntegerDomain::Bounded(IntegerInterval::new(big(-5), big(5)).unwrap());
    let in_bounds = |value: i128| value.abs() <= 5;
    for profile in DivisionProfile::ALL {
        let selection = admitted(profile);
        for a in -20..=20 {
            for b in -20..=20 {
                let euclid = modulo(&big(a), &big(b), &bounded, &mut Meter::new(UNLIMITED));
                if b == 0 {
                    for member in [DivisionMember::Quotient, DivisionMember::Remainder] {
                        assert_eq!(
                            run(profile, member, a, b, &IntegerDomain::Mathematical),
                            Outcome::Undefined(Undefined::DivisionByZero)
                        );
                    }
                    assert_eq!(euclid, Outcome::Undefined(Undefined::DivisionByZero));
                    continue;
                }
                let expected = oracle(profile, a, b);
                assert_law(profile, a, b, expected);
                assert_eq!(pair(profile, a, b), expected, "{profile:?} ({a},{b})");
                let euclidean = oracle(DivisionProfile::Euclidean, a, b).1;
                if euclidean.abs() <= 5 {
                    assert_eq!(euclid.completed().map(|r| int(&r)), Some(euclidean));
                } else {
                    assert!(matches!(
                        euclid,
                        Outcome::Refused(Refusal::ModuloOutOfDomain { .. })
                    ));
                }

                for (member, value) in [
                    (DivisionMember::Quotient, expected.0),
                    (DivisionMember::Remainder, expected.1),
                ] {
                    let mut meter = Meter::new(UNLIMITED);
                    let outcome =
                        divide(&selection, member, &big(a), &big(b), &bounded, &mut meter);
                    if in_bounds(value) {
                        assert_eq!(
                            completed(outcome),
                            value,
                            "{profile:?} {member:?} ({a},{b})"
                        );
                    } else {
                        assert!(
                            matches!(
                                &outcome,
                                Outcome::Refused(Refusal::DivisionOutOfDomain { member: got, .. })
                                    if *got == member
                            ),
                            "{profile:?} {member:?} ({a},{b}): {outcome:?}"
                        );
                        assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
                    }

                    let mut metered = Meter::new(UNLIMITED);
                    let _ = divide(
                        &selection,
                        member,
                        &big(a),
                        &big(b),
                        &IntegerDomain::Mathematical,
                        &mut metered,
                    );
                    for point in metered.admitted_charges() {
                        let mut denied =
                            Meter::new(UNLIMITED).with_injected_denial(InjectedDenial {
                                point: *point,
                                occurrence: 1,
                            });
                        assert!(matches!(
                            divide(
                                &selection,
                                member,
                                &big(a),
                                &big(b),
                                &IntegerDomain::Mathematical,
                                &mut denied
                            ),
                            Outcome::Incomplete(Incomplete { charge_point, .. })
                                if charge_point == *point
                        ));
                        assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
                    }
                }
            }
        }
    }
}

/// QSpec FR-147 member-only domain: `10 div y` over `1..=10` at `y = 5`
/// evaluates to 2 although the remainder 0 and quotient are both fine, and
/// `x rem -1` over `-10..=5` at `x = -10` evaluates to 0 although the
/// quotient 10 is outside that domain.
#[trace("QSpec-TC-192", "QSpec-FR-147-AC-1", "QSpec-FR-147-AC-4")]
#[test]
fn only_the_exposed_member_must_lie_in_the_consumer_domain() {
    let one_to_ten = IntegerDomain::Bounded(IntegerInterval::new(big(1), big(10)).unwrap());
    let minus_ten_to_five = IntegerDomain::Bounded(IntegerInterval::new(big(-10), big(5)).unwrap());
    for profile in DivisionProfile::ALL {
        assert_eq!(
            completed(run(profile, DivisionMember::Quotient, 10, 5, &one_to_ten)),
            2
        );
        assert_eq!(
            completed(run(
                profile,
                DivisionMember::Remainder,
                -10,
                -1,
                &minus_ten_to_five
            )),
            0
        );
        // The exposed member outside the domain refuses with that member as the
        // cause: quotient 10 of `-10 div -1` and remainder 0 of `10 rem 5`.
        assert_eq!(
            run(
                profile,
                DivisionMember::Quotient,
                -10,
                -1,
                &minus_ten_to_five
            ),
            Outcome::Refused(Refusal::DivisionOutOfDomain {
                domain: Box::new(IntegerInterval::new(big(-10), big(5)).unwrap()),
                member: DivisionMember::Quotient,
            })
        );
        assert_eq!(
            run(profile, DivisionMember::Remainder, 10, 5, &one_to_ten),
            Outcome::Refused(Refusal::DivisionOutOfDomain {
                domain: Box::new(IntegerInterval::new(big(1), big(10)).unwrap()),
                member: DivisionMember::Remainder,
            })
        );
    }
}

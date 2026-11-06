// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-264 Behavior 5, FR-255 `emit.output_bytes` (TC-738, TC-721): the
//! caller-owned ceiling on one emission's canonical bytes, driven through
//! zero, `N - 1` and `N` for the package's measured length `N`.

use qsl_foundation::diagnostic::LimitKind;
use qsl_foundation::{Setting, SettingLimits};

use super::*;
use crate::EmitLimits;

/// The refusal an emission under `bound` returned, whose limit is the one
/// `emit.output_bytes` reached.
fn limit_of(refusal: EmitRefusal) -> qsl_foundation::diagnostic::LimitExceeded {
    assert_eq!(refusal.code(), Code::StageLimitExceeded);
    assert_eq!(refusal.cause(), Some("input-bytes-exceeded"));
    match refusal {
        EmitRefusal::OutputLimit(limit) => limit,
        other => panic!("expected the output limit, got {other:?}"),
    }
}

fn emitted_at(package: &CheckedPackage, bound: u64) -> Result<Emission, EmitRefusal> {
    crate::emit::emit_package(package, whole_unit, EmitLimits::new(bound))
}

/// The bound is inclusive: `N` bytes under bound `N` are emitted, byte for
/// byte as under a larger bound; `N - 1` and `0` refuse, naming the setting,
/// the configured bound and a count above it, and write nothing.
#[trace("TC-738", "FR-264-AC-6")]
#[test]
fn a_package_is_emitted_at_its_exact_length_and_refused_one_below_and_at_zero() {
    let package = package(vec![t()]);
    let larger = emitted_at(&package, u64::MAX).expect("an unbounded ceiling emits");
    let bytes = larger.package().bytes();
    let n = u64::try_from(bytes.len()).unwrap();
    assert!(n > 1, "the fixture package has bytes to bound");

    let exact = emitted_at(&package, n).expect("a ceiling of exactly N bytes emits");
    assert_eq!(exact.package().bytes(), bytes);

    for bound in [n - 1, 0] {
        let refusal = emitted_at(&package, bound).expect_err("a ceiling below N refuses");
        let limit = limit_of(refusal);
        assert_eq!(limit.setting(), Setting::EmitOutputBytes);
        assert_eq!(limit.kind(), LimitKind::InputBytes);
        assert_eq!(limit.configured_bound(), bound);
        assert!(
            limit.actual() > u128::from(bound),
            "the refused write would have reached more than {bound}: {}",
            limit.actual()
        );
        assert!(
            limit.to_string().contains("emit.output_bytes"),
            "the refusal names the setting that raises it: {limit}"
        );
    }
}

/// An emission remembered by an earlier call counts the same bytes against
/// the bound the later call supplies: refused below `N` with actual exactly
/// `N`, returned unchanged at `N`.
#[trace("TC-738", "FR-264-AC-7")]
#[test]
fn a_remembered_emission_is_held_to_the_bound_of_the_later_call() {
    let package = package(vec![t_read_from_text()]);
    let first =
        crate::emit::emit_checked(&package, EmitLimits::fixture()).expect("the package emits");
    let bytes = first.package().bytes().to_vec();
    let n = u64::try_from(bytes.len()).unwrap();

    let at = crate::emit::emit_checked(&package, EmitLimits::new(n))
        .expect("the remembered bytes fit N");
    assert_eq!(at.package().bytes(), bytes);

    for bound in [n - 1, 0] {
        let limit = limit_of(
            crate::emit::emit_checked(&package, EmitLimits::new(bound))
                .expect_err("N bytes exceed the bound"),
        );
        assert_eq!(limit.setting(), Setting::EmitOutputBytes);
        assert_eq!(limit.configured_bound(), bound);
        assert_eq!(limit.actual(), u128::from(n));
    }
}

/// The dependency emissions a link makes run under the caller's ceiling: a
/// dependency whose bytes exceed it refuses the link with the output limit.
#[trace("TC-738", "FR-264-AC-6")]
#[test]
fn a_link_refuses_a_dependency_whose_emission_exceeds_the_ceiling() {
    let refused = CheckedPackage::link_with(
        root_graph(),
        vec![import("test/units", dependency("test/units"))],
        EmitLimits::new(1),
    )
    .expect_err("the dependency does not fit one byte");
    let crate::LinkRefusal::DependencyEmission {
        identity,
        refusal: Some(refusal),
    } = refused
    else {
        panic!("expected a dependency emission refusal, got {refused:?}");
    };
    assert_eq!(identity, lib("test/units"));
    let limit = limit_of(refusal);
    assert_eq!(limit.configured_bound(), 1);
    CheckedPackage::link_with(
        root_graph(),
        vec![import("test/units", dependency("test/units"))],
        EmitLimits::fixture(),
    )
    .expect("the same link under the fixture ceiling succeeds");
}

/// FR-255-AC-20: the setting names exactly the output-byte field, accepts
/// zero and `u64::MAX`, and no other setting changes it.
#[trace("TC-721", "FR-255-AC-20")]
#[test]
fn emit_output_bytes_sets_only_the_output_byte_bound() {
    let mut limits = EmitLimits::new(7);
    assert_eq!(limits.bounds(), vec![(Setting::EmitOutputBytes, 7)]);
    for bound in [0, u64::MAX] {
        assert!(limits.set_bound(Setting::EmitOutputBytes, bound));
        assert_eq!(limits.output_bytes, bound);
    }
    assert!(!limits.set_bound(Setting::I2InputBytes, 1));
    assert_eq!(limits.output_bytes, u64::MAX);
    assert_eq!(Setting::EmitOutputBytes.name(), "emit.output_bytes");
    assert_eq!(Setting::EmitOutputBytes.kind(), LimitKind::InputBytes);
    assert_eq!(EmitLimits::new(1).with_output_bytes(9).output_bytes, 9);
}

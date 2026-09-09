// SPDX-License-Identifier: AGPL-3.0-only
//! Compare the actual private canonical pass separately from its digest.

use crate::runtime_test_setup::native_rule_model;
#[path = "../../tests/support/package_vector_setup.rs"]
mod package_vector_setup;

use ix_trace_rs::trace;

use super::*;
use crate::runtime::{
    evaluate, validate, EvaluationLimits, EvaluationOutcome, ValidationLimits, ValueNode,
};
use crate::runtime_test_setup as setup;

#[test]
#[trace("TC-090", "FR-021-AC-1", "FR-021-AC-6")]
fn canonical_pass_bytes_match_each_independent_fixed_vector_before_hashing() {
    let models = [native_rule_model::from_text(
        include_str!("../../tests/fixtures/native-package/model-source.json"),
        "fixed-model.json",
        "1",
    )
    .unwrap()
    .model()];
    for vector in package_vector_setup::cases() {
        let checked = package_vector_setup::checked(&vector, &models);
        let features = features::derive(&checked).unwrap();
        let mut usage = PackageUsage::default();
        let bytes = run(
            &view::Manifest::new(&checked, &features, false, None),
            PackageLimits::default(),
            Pass::Canonical,
            &mut usage,
        )
        .unwrap();
        assert_eq!(bytes, vector.canonical, "{} canonical bytes", vector.name);
        assert_eq!(
            usage.canonical.unwrap().output_bytes,
            vector.canonical.len()
        );
        assert!(usage.encode.is_none());
        let package = NativePackage::new(checked, PackageLimits::default()).unwrap();
        assert_eq!(package.bytes(), vector.artifact);
        assert_eq!(
            package.canonical_identity().to_string(),
            vector.digest.strip_suffix('\n').unwrap()
        );
    }
}

#[test]
#[trace("TC-082", "TC-090", "FR-019-AC-8", "FR-019-AC-9")]
#[trace("FR-021-AC-6")]
fn runtime_populations_and_evaluator_budgets_do_not_enter_static_passes() {
    let models = [native_rule_model::parts().model()];
    let make = || {
        NativePackage::new(
            setup::checked(&models, "self.n = 1"),
            PackageLimits::default(),
        )
        .unwrap()
    };
    let package = make();
    let canonical = |package: &NativePackage<'_>| {
        let features = features::derive(package.checked()).unwrap();
        run(
            &view::Manifest::new(package.checked(), &features, false, None),
            PackageLimits::default(),
            Pass::Canonical,
            &mut PackageUsage::default(),
        )
        .unwrap()
    };
    let original_canonical = canonical(&package);
    let mut runtime_digests = std::collections::BTreeSet::new();
    for (number, truth) in [(0, false), (1, true), (2, false)] {
        let mut data = setup::draft(&models[0]);
        setup::change_field(&mut data, "n", ValueNode::Integer { value: number });
        let snapshot = setup::snapshot(data);
        assert!(runtime_digests.insert(snapshot.digest().to_string()));
        let selected = setup::selection(&models[0], snapshot.reference());
        let context = validate(
            package.checked(),
            setup::input(snapshot),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        for budget in [0, 3, 4, EvaluationLimits::default().expression_steps] {
            let report = evaluate(
                &context,
                EvaluationLimits {
                    expression_steps: budget,
                    ..EvaluationLimits::default()
                },
                || false,
            );
            if budget < 4 {
                let EvaluationOutcome::Incomplete(diagnostic) = report.outcome() else {
                    panic!("the independent four-step expression must exhaust {budget} steps");
                };
                assert_eq!(diagnostic.code, Code::ResourceExhausted);
                assert_eq!(report.usage().expression_steps, budget);
            } else {
                assert_eq!(report.outcome(), &EvaluationOutcome::Completed(truth));
                assert_eq!(report.usage().expression_steps, 4);
            }
            let rebuilt = make();
            assert_eq!(canonical(&package), original_canonical);
            assert_eq!(canonical(&rebuilt), original_canonical);
            assert_eq!(rebuilt.bytes(), package.bytes());
            assert_eq!(rebuilt.digest(), package.digest());
            assert_eq!(rebuilt.canonical_identity(), package.canonical_identity());
            assert_eq!(rebuilt.usage(), package.usage());
        }
    }
    assert_eq!(runtime_digests.len(), 3);
}

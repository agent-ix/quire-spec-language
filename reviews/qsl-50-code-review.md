---
id: SR-7836
title: "QSL-50 code review with Rust lane of PR #782"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2c5d8bf910658162d2568f5a4cd8df52a19f9404; PR #782 base 8d293ebf3b90878eda2703e717bd519273ea9b70; qsl-semantics/src/model/normalize.rs and qsl-semantics/tests/it/model_normalization.rs"
review_set: subset
---
## Summary

Reviewed the exact QSL-50 PR #782 diff, including Rust idioms, operation-member phase 2/3/4 derivation, charge accounting, test quality, and ticket scope. The implementation and behavioral oracles are sound in the reviewed area; one stale test comment still describes the pre-change behavior.

## Verdict

**CONDITIONAL** — one low-severity documentation defect remains. The full `make ci` gate is also not green: it exits 2 at `qsl-bench::model::tests::source_byte_intake_preserves_reader_refusals`; the identical isolated failure was reported on the clean base.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Stale operation-contest explanation says no EffectiveView entry is built, although this PR now retains operation members and their facts in the view. | qsl-semantics/tests/it/model_normalization.rs:3260-3262 |

## Rust review

The new member collection is sorted before charging and includes fields, clauses, and operations. Inheritance records qualify and inherit facts for operation members; reachable operation redefinitions append paired facts and update visibility. Assertions check fact rules, input identities, visibility, charge points, and limit boundaries. I found no panic, unsafe, boundary-conversion, stub, tautology, or untraced-new-test defect in the changed code. The one finding is a stale comment in a test helper explanation.

## Gate evidence

- Reported focused checks pass: `cargo check --locked --workspace --all-targets`; `model_normalization` 74/0; `model_conformance` 34/0; `model_dispatch` 14/0; formatting and default-feature Clippy.
- Reported full gate: `make ci` exits 2 at `qsl-bench::model::tests::source_byte_intake_preserves_reader_refusals`.
- Reported baseline comparison: the same isolated test fails with the same retained-refusals output on clean base `8d293ebf3b90878eda2703e717bd519273ea9b70`.
- The baseline comparison establishes that the observed test failure is not introduced by this diff. It does not turn the aggregate gate green.

## Verdict rationale

The ticket behavior is implemented with direct derivation records and charge tests. FND-001 is low because it affects reviewer and maintainer understanding, not runtime behavior. Merge readiness remains blocked by the repository's documented full local gate (`make ci`) until the failure is resolved or an authorized policy exception is recorded; baseline provenance is not a passing gate.

## Dispositions

Round 2, reviewed at `d931afed01d1b68dcef6529983818e5142379e2b`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4d645b3867a13a1e240308bc9476234cb7bc59c2 |

FND-001 after_excerpt: “Operation redefinitions now populate `EffectiveView` member entries; this contested group itself refuses before producing a completed view, preserving its existing conflict semantics.”

The replacement text is present at both the fix commit and current PR head. The trailing `d931afed` commit changes only the two `reviews/` documents, so it does not alter the fix.

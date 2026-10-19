---
id: SR-12805
title: "IR-878 QSL evaluator owner allocation base review"
type: SpecReview
analysis: base
scope: "spec/functional/FR-391-supplied-admission-evaluator-boundary.md; spec/spec.md"
review_set: base
---

## Summary

Reviewed the QSL evaluator and caller owner allocation for QSV FR-109-AC-6 and quire-exact FR-372-AC-5/FR-380-AC-6. The three criteria retain evaluator meter, conversion budget, `FunctionCall` projection and omitted-bound behavior at the `qsl-eval` owner. No new test evidence is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No spec defects found in the scoped allocation; owner tests remain planned. | FR-391 |

## Base checks

FR-391 and its three criteria have unique sequential IDs, explicit inputs, outputs, owner behavior and error boundaries. Each Verification cell names Test only. The criteria cover a shared-helper debit, evaluator meter invariance, conversion/admission separation, `FunctionCall` failure projection and the omitted 4096-bit bound. The source owner was checked against `qsl-eval/src/value/expression/evaluate.rs` and its production `FunctionCall` charge. The QSV supplied path cannot observe the evaluator meter. External FR links point to the original obligation owners. No digest, pin, fixture copy, trace tag or test case was introduced.

FR-391 is PLANNED: the user requested spec allocation before test PRs, so the absence of a local trace tag is an explicit evidence gap, not a passed coverage claim. Option, constraint and state boundaries relevant to this owner split are stated in the three criteria. Structural validation of the edited files passed.

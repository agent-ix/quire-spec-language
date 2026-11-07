---
id: SR-20261106210001
title: "QSL-499 base spec review of FR-505 Rational normalization and FR-056-AC-43 Decimal value"
type: SpecReview
analysis: base
scope: "origin/main...c0fa3e831; spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md, spec/test-cases/TC-1300-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md, spec/functional/FR-056-admit-domain-package-model-declarations.md (AC-43), spec/test-cases/TC-897-text-is-the-native-text-name.md"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-505
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1300
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-897
    type: reviews
---
# SR-20261106210001: QSL-499 base spec review

## Summary

Base checklist only, scoped to the diff of `origin/main...c0fa3e831`. No assurance profile requires an enforced spec-review run for this change; this is a direct review.

These claims hold:

- FR-505's value-admission bullet, AC-5 and TC-1300 rows 21-22 now follow FR-106 "Rational value": the supplied pair is normalized, bounds are checked on the normalized pair, `2/4` admits as `1/2` (FR-106-AC-55), `16/2` refuses as `8/1` (matching FR-106-AC-56's `22/2`), and a `float64` object refuses `wrong-value-kind` (FR-106 check 6, FR-106-AC-56). FR-106 is unchanged. No "no value is reduced" or "lowest terms" wording remains in FR-505 or TC-1300.
- FR-056-AC-43 and TC-897 step 9 now state the Decimal-at-bare-Decimal refusal that FR-056 prose (line 850, "an FR-106 Decimal or Text value never fills a member typed by bare `ix://quire/native/Decimal`") already requires. The cause and path match the existing `memo` case.
- QSpec is cited by ID only. The diff adds no hash, digest, pin or tracking record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The unit-resolution rule (a unit resolves against the unit table of the document that declares it; `Config::m` is a config name, not a unit) is normative only inside an AC-9 parenthetical. FR-505's prose `unit` input bullet (line 58) does not state it, and the parenthetical is long. Fix: add one sentence to the `unit` input bullet and shorten AC-9 to "Over AC-7's units declared in `Ratio`'s model document, ...". | FR-505 line 58; FR-505-AC-9 line 211 |
| FND-002 | medium | TC-1300 row 19 asserts that `m` resolves "never to a config `Config::m`", but the fixture declares no `Config` with a member `m`, so that clause cannot fail. Fix: add a config `Config` with member `m` to row 19's mutation, or drop the clause from row 19 and AC-9. | TC-1300 line 51; FR-505-AC-9 line 211 |
| FND-003 | low | FR-505 says an out-of-bounds normalized pair refuses `invalid-value` "naming the member path", and AC-5 the same. FR-106 check 7 also names the declared type and the normalized pair, and check 6 names `kind` for the Float refusal. These do not contradict, but they are incomplete. Fix: add "naming the declared type and the normalized pair" and "naming `kind`", or cite FR-106 check 6 and 7. | FR-505 lines 170-179; FR-505-AC-5 line 207 |

## Checklist disposition

- **IDs and links:** the AC ids are unchanged and the TC trace tag list is unchanged.
- **Coverage:** AC-5 and AC-9 map to TC-1300 rows 21-22 and 19. FR-056-AC-43 maps to TC-897 step 9.
- **Consistency:** consistent with FR-106 "Rational value", checks 6-7 and AC-55/56. Consistent with FR-056 prose on bare natives. FR-497 (line 57: "the unit `u` resolves as for `Quantity<u>` (FR-094)") does not contradict FR-505-AC-9, but it does not state the document-local rule either (see FND-001).
- **Hash/digest/pin check:** none added.

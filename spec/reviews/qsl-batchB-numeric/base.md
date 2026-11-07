---
id: SR-20261106202488
title: "Batch B numeric base specification review (QSL PR #2488)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR #2488, branch spec/qsl-batchB-numeric, commit 1aebef809; FR-194 window aggregate value types and FR-194-AC-11/12, TC-629 steps 11-12, FR-035-AC-7, TC-113 step 5"
review_set: base
---
# Batch B numeric base specification review (QSL)

## Summary

The review covers only commit 1aebef809, using the `base` review set with no analysis lenses. It checked the commit against the paired QSpec requirements: FR-414 "Values", FR-414-AC-13/14, FR-142-AC-19 to AC-21, FR-154 "Unit bounds" and FR-339-AC-5. It also checked QSL FR-497, the carried-quantity type.

These claims hold:

- **Value typing.** FR-194 types `sum`/`min`/`max`/`quantile` as the type of the value expression and keeps a package unit's own node. It refuses a carried value or threshold `ill_typed`/`type-mismatch` before any charge and checks the refused value's threshold against `Quantity<u>`. Each of these matches QSpec FR-414 "Values" and the FR-497 rule that a carried quantity is not an operand of comparison. The new `references` edge to FR-497 is correct.
- **Citations.** The QSL text cites QSpec by ID and section name only (FR-414 "Values", FR-142-AC-19 to AC-21, FR-414-AC-13/14, FR-339-AC-5). It has no QSpec paths, branches or links.
- **Unit declarations.** The source unit declarations use `rational(1, 1)` and `rational(1, 1000) * s`, which are valid `unit-decl` forms under the shared grammar.
- **Expected values.** Every expected number in FR-194-AC-11 and FR-194-AC-12 was recomputed and is correct:
  - FR-194-AC-11 at position 2:
    - `5, 1, 4` of `M::ms` give sum `10`, min `1`, max `5`, and a nearest-rank median of `4`, the 2nd smallest of `1, 4, 5`.
    - `1 M::s` converts to `1000 M::ms`, so the threshold atom is `true`.
  - FR-194-AC-12:
    - `2, -1, 3` fit `Int[-5, 5]`.
    - The round trip back to `Quantity<m>` sums to `4 m`, and `4 m <= 5 m` is `true`.
- **FR-035-AC-7 and TC-113 step 5.** Both now follow the QSpec FR-339-AC-5 ruling. The open-owner-question text and the test-file path tracking were removed, which is correct.
- **No ceremony.** No hash, digest, pin or tracking record is introduced.

The review found one medium and four low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | **`quantile 1/2` is not an aggregate term under the shared grammar.** The QSpec shared grammar's `aggregate-term` takes `'quantile', exact-value`. `exact-value` is a `uint`, a decimal literal or `rational(n, d)`/`decimal(n, s)`, and `/` is never rational-literal punctuation. So `quantile 1/2` in FR-194-AC-11, FR-194-AC-12 and TC-629 steps 11-12 does not parse. FR-194-AC-10 already had the same spelling. QSpec has the same defect in FR-414-AC-13/14 (QSpec batch B review FND-002). Fix: write `quantile 0.5` (or `quantile rational(1, 2)`) in FR-194-AC-10/11/12 and TC-629 steps 10-12, together with the QSpec fix. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:215-217; spec/test-cases/TC-629-window-aggregates-in-monitors.md:31-32 |
| FND-002 | low | **The FR-194-AC-11 fixture leaves out one QSpec case.** QSpec FR-414-AC-13 also requires `sum of e.latency where holds(e.latency > 9 M::ms) over past[0, 2] min_count 0 = 0 M::ms` to be `true`: no event passes the filter, so the empty sum is in the package unit. FR-194-AC-11 and TC-629 step 11 omit it. Without it, the QSL tests do not check that the zero of an empty filtered sum is typed by the package unit node. Fix: add that atom to FR-194-AC-11, to TC-629 step 11 and to its expected result (`true`). | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:216; spec/test-cases/TC-629-window-aggregates-in-monitors.md:31, 48 |
| FND-003 | low | **The package unit is described in source syntax that does not parse.** FR-194-AC-11 says the package declares `unit ms : Time = 1/1000 * s;`. The package is an FCD document, not source, and that spelling is not a valid `unit-decl` either. Fix: describe it as package data, for example "a unit `ms` of `Time` (`target` `s`, `scale` `1/1000`)", or use the valid source form `rational(1, 1000) * s`. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:216 |
| FND-004 | low | **The model-alias unit of FR-035-AC-7 has no declaration.** A complete unit needs at least one declaration after its `model` lines, so a unit that only declares `model w = "...";` refuses `invalid_syntax` at EOF. TC-113 step 5 has the same gap. QSpec FR-339-AC-5 has the same defect (QSpec batch B review FND-003). Fix: state that the model-alias unit also carries a declaration, for example a function, in FR-035-AC-7 and TC-113 step 5. | spec/functional/FR-035-parse-composed-native-units.md:76; spec/test-cases/TC-113-parse-composed-native-syntax.md:34 |
| FND-005 | low | **"Never dropped" and "reaches S3" have no observable.** FR-035-AC-7 says "a `using w(...)` selection is never dropped", and TC-113 says it "reaches S3". Neither names something a test can assert. Fix: name the observable, for example "the checked function's profile alias resolves to the profile `w` selected with `profile w = ...;` and the function is admitted under it". | spec/functional/FR-035-parse-composed-native-units.md:76; spec/test-cases/TC-113-parse-composed-native-syntax.md:34 |

## Verdict

The QSL text matches the QSpec carried-quantity and package-unit rules, cites QSpec by ID only, and every expected value is correct. Fix FND-001 together with QSpec, because the paired criteria use the same non-parsing `quantile 1/2`. The low findings are coverage and wording fixes. The QSpec 0-draft `Tuple` finding (QSpec FND-004) does not affect QSL. QSL's `0-draft` claim describes its own historical recognizer.

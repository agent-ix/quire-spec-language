---
id: SR-15480
title: "integrity review of QSL-974 quantity literal admission (QSL PR #1214)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@b155d3ea321b8dbde055fe4a282ec7789ec3a3a7; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md, spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-483
    type: reviews
---
# SR-15480: integrity review of QSL-974 quantity literal admission (QSL PR #1214)

## Summary

Ticket: QSL-974. Diff reviewed: `git diff origin/main...b155d3ea3` (FR-093 "Quantity literals",
its table row, FR-093-AC-33 and AC-34, the FR-094 quantity paragraph, and TC-483 steps 5 and 6).

Sources checked read-only: QSpec at 429c59d (FR-322 "Scalar literal values" and FR-322-AC-79,
FR-142 Behavior and FR-142-AC-11/12), Contract IR origin/main bcf7e35 (FR-038-AC-73 and
AC-299 to AC-302), QSL FR-091 (AC-33, AC-34, AC-44, the catalog table and OQ-11), FR-094
(divisor rule), FR-185 "Exact values" and "Package emission", FR-195, FR-186, ADR-014 §4,
FR-097-AC-2 and NFR-011 Scope.

The lowering form (`value_kind` `quantity`, `type` the unit node, canonical magnitude) matches
FR-322 and IR FR-038-AC-299. The domain paragraph matches ADR-014 §4, FR-097-AC-2 and IR
FR-038-AC-73. The refusal codes exist in the closed tables: `missing_declaration`/`missing-name`
(FR-091 catalog), `ill_typed`/`type-mismatch` (`CheckRefusal::from_ill_typed` maps
`DistinctUnits` and `IncompatibleDimensions` to it), `undefined_expression`/`unproved-nonzero`
(FR-091 catalog, `ProbabilisticCause::ZeroDenominator`, `check::facts` quantity divide arm), and
`stage_limit_exceeded`/`rational-bits-exceeded` naming `s3.rational_bits` (`qsl-foundation`
setting and stage tables). The rational-bits vector is grounded and testable as written: NFR-011
Scope already applies the ceiling to each quantity literal check types, `check::typing` measures
the literal's magnitude against `limits.rational_bits()` before it resolves the unit, a caller can
set the ceiling through `s3.rational_bits` (`settings.rs`), and `100000` has 17 bits, past a
ceiling of 16. FR-093-AC-34 is a new id with no collision among the open QSL PRs (only #706 also
touches FR-093/FR-094, and it adds FR-094-AC-9 and AC-10). The FR-094 anchor `#quantity-literals`
resolves. FR-185 and FR-195 probabilistic forms stay refused at S4. The gate
(`make check-spec-validation`) exits 0 with no warning on the changed files.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: FR-093-AC-33's vector `function c using v(d: Quantity<ms>): Boolean pure { convert<s>(d) = 3 s }` writes the conversion target as the bare unit name `s`. A unit name in a type position is not a type: QSL FR-091-AC-33 refuses a type `m` naming a unit, FR-091-OQ-11(d) names units as types only through `Quantity<u>`, and QSpec FR-142 states "A conversion into a unit is written `convert<Quantity<u>>(e)`" (FR-142-AC-12 uses `convert<Quantity<s>>(d)`; `qsl-package/src/emit/tests/admission_corpus.rs:812,819` use `convert<Quantity<m>>`). As written, a correct checker refuses `c` `missing_declaration`/`missing-name` at `s`, so AC-33 and TC-483 step 6 (which expects `c` to lower) cannot pass. Write `convert<Quantity<s>>(d) = 3 s` in AC-33 and in TC-483 steps 6 and its expected result. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:931; spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md:44-49,71-74 |
| FND-002 | low | minor: lines 166-170 cite FR-094 for "a different dimension, and a different unit of the same dimension, each refuse `ill_typed`/`type-mismatch`". FR-094 states only the divisor rule (FR-094:310-313). The rule is QSpec FR-142 Behavior ("Addition/comparison require compatible dimensions and an explicit common-unit conversion", "Conversion never occurs implicitly", and "including a quantity in another unit of the same dimension, the checker SHALL refuse `ill_typed`/`type-mismatch`"). Cite QSpec FR-142 for the unit and dimension rule and keep FR-094 for the divisor. This also grounds lead default 2, which the PR body says no FR states. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:166-170 |
| FND-003 | low | minor: lines 153-155 say `check` measures the magnitude "as FR-185 'Exact values' states for every exact value". FR-185 scopes that rule to weights and to "every `exact-value` that FR-186 reads". It does not mention value-expression literals. NFR-011 Scope (lines 33-34) is the statement that applies the ceiling to each quantity literal check types. Reword to "by FR-185's measure, under the ceiling NFR-011 applies to each quantity literal `check` types". | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:153-155 |
| FND-004 | low | minor: AC-33's "`f(0.5 ms)` and `f(rational(2, 4) ms)` in one unit" and "two `3 ms` in one unit" do not name the function bodies, so the vectors are not concrete. Writing the two `3 ms` in one body (`f(3 ms) and f(3 ms)`) would also fold the two calls into one node (FR-092-AC-11), which masks the literal-sharing check. Name the bodies, for example `function h2 using v(): Boolean pure { f(0.5 ms) }`, `function h3 ... { f(rational(2, 4) ms) }` and a second function `h4` whose body is `f(3 ms)` beside `h`, and use the same names in TC-483 step 6. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:931; spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md:44-46 |

## Verdict

Not mergeable: FND-001 (major) makes AC-33's conversion vector unsatisfiable. FND-002 to FND-004
are minor wording and citation fixes. Everything else in the diff is consistent with QSpec FR-322,
FR-142, IR FR-038 and the QSL FRs it cites.

## Dispositions

Round 1, reviewed QSL #1214 at 3fd9a7398 (fix commit over b155d3ea3); code facts re-verified in the QSL tree at that head.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | AC-33 and TC-483 step 6 now write `convert<Quantity<s>>(d) = 3 s` (QSpec FR-142 form) |
| FND-002 | fixed | the unit and dimension rule now cites QSpec FR-142 Behavior; FR-094 is kept for the divisor only |
| FND-003 | fixed | Typing now reads "by FR-185's measure, under the rational-bits ceiling NFR-011 applies to each quantity literal check types (NFR-011 Scope)" |
| FND-004 | fixed | AC-33 names h2, h3 and h4 as separate bodies beside h; TC-483 step 6 uses the same names |

---
id: SR-20261105101060
title: "base review of QSL-1693 Rational bounds"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; QSL-1693; PR #2430; FR-505/TC-1300, FR-517-AC-12/TC-1312; intake.rs, model_intake.rs, checked_handoff.rs, R library and model fixture"
review_set: subset
---

## Summary

Independent frozen-source review; no substantive change defect found. The native reader dependency prevents end-to-end acceptance.

## Verdict

**CONDITIONAL** — End-to-end admission is BLOCKED by native Contract IR rational_value_type using Value::as_i64: canonical signed-64 strings refuse MalformedDeclaration at /types/11/constraints/0/operands (outer /semantic_graph/nodes/34/owner). No native fix or normal dependency adoption is present. Not merge-ready; the owner must fix IR and adopt main normally, then run the one final scoped gate. No pins, patches, skipped assertion, bound change or constraint removal is acceptable.

## Scope

- FR-505-AC-1; examined; spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md: "A package with value type `Ratio` (`scalar` `rational`; constraints `maxDenominator` 12, `minNumerator` -3, `minDenominator` 1 and `maxNumerator` 7, in that order) and a field, an operation parameter and an operation result typed `Ratio` admits; each of the three sites and a plain alias of `Ratio` has the type `Rational[-3, 7; 1, 12]`, the package keeps one original declaration for `Ratio`, and the four sites name one `bounded_domain`/`rational_range` node over the `scalar_type`/`rational` node with bindings `numerator_min` -3, `numerator_max` 7, `denominator_min` 1 and `denominator_"
- FR-505-AC-3; examined; spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md: "Operands `1.5`, `1e2`, `\"3/4\"`, `\"01\"`, `\"+1\"`, `\"-0\"`, `\"9223372036854775808\"` and `\"-9223372036854775809\"` for `maxNumerator`, `0.5` for `maxDenominator`, a `maxNumerator` with no `operands.value`, and a `maxNumerator` whose `operands` carries a second member, each refuse `invalid_model_binding`/`malformed-declaration` at that constraint's `Constraint.operands` naming the keyword, with no bound rounded or widened. Integer numbers `9007199254740993`, `9007199254740994` and `-9007199254740994` refuse earlier as `noncanonical_wire`/`inexact-integer` at their exact document pointer, no"
- FR-517-AC-12; examined; spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md: "**R**'s model declares the Rational value types `Ratio` and `Exact` with no unit, the compiled source reaches both, and the emitted package of **R** carries the rationals `9223372036854775807/9223372036854775806` and `-9223372036854775808/9223372036854775807` as `Exact` values, with no declared bound widened to admit them."
- FR-505; examined; spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md: "object whose only member `value` is either a JSON integer of magnitude at\n  most `2^53` (`9007199254740992`), or a canonical signed-64 decimal integer\n  string in `i64::MIN..=i64::MAX`: `0`, a positive digit sequence without a\n  leading zero, or `-` followed by a nonzero digit and further digits."
- FR-505; examined; spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md: "- Before declaration intake, the package's FR-272 canonical-number check\n  SHALL refuse an integer number of magnitude above `2^53` as\n  `noncanonical_wire`/`inexact-integer` at its `document_pointer`. Such a\n  number never reaches row 5."
- FR-517; examined; spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md: "The shared library of **R** SHALL reach both unit-less Rational types through\ntyped predicate parameters and Rational literals."
- TC-1300; examined; spec/test-cases/TC-1300-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md: "| 4 | `minNumerator` `\"-9223372036854775808\"`; `maxNumerator` `\"9223372036854775807\"`; numeric `minNumerator` -9007199254740992; numeric `maxNumerator` 9007199254740992 | each admitted with exact unchanged bounds |"
- TC-1300; examined; spec/test-cases/TC-1300-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md: "| 9 | `maxNumerator` operand `1.5`; `1e2`; `\"3/4\"`; `\"01\"`; `\"+1\"`; `\"-0\"`; `\"9223372036854775808\"`; `\"-9223372036854775809\"`; `maxDenominator` operand `0.5`; `maxNumerator` with no `operands.value`; `maxNumerator` `operands` with a second member | each `invalid_model_binding`/`malformed-declaration` at that `Constraint.operands` naming the keyword; no bound rounded or widened |"
- TC-1300; examined; spec/test-cases/TC-1300-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md: "| 26 | numeric `minNumerator` 9007199254740993, 9007199254740994 and -9007199254740994 | each FR-272 `noncanonical_wire`/`inexact-integer` at the operand document pointer before intake; never an FR-154 operand refusal |"
- TC-1312; examined; spec/test-cases/TC-1312-write-a-checked-package-handoff-directory-from-source-in-process.md: "| 12 | 12 | write each of **R**'s three packages; independently admit the bytes and read the Rational literal nodes and their semantic types; read the model document included in each handoff | `Ratio` and `Exact` are unit-less Rational declarations with numerator/denominator bounds -1..1/1..2 and -9223372036854775808..9223372036854775807/1..9223372036854775807 respectively; both bounded types are reached; the two extreme rationals are literal values typed by the emitted bounded `Exact` type, with no bound widened |"
- QSpec-FR-154; context_only; agent-ix/quire-specification/spec/functional/type-model/FR-154-admit-domain-package-model.md: "**Rational value types.** A value type (FR-208\n`quire.meaning.model.value-type/v1`) whose `scalar` is `rational` is a\n**Rational value type**. Intake SHALL read its bounds from its own\n`constraints` only, by the four keywords `minNumerator`, `maxNumerator`,\n`minDenominator` and `maxDenominator`, each given once in any order, each with\n`appliesTo` the value type itself. Its `operands.value` SHALL be either a JSON\ninteger of magnitude at most 2^53 (9007199254740992), or a canonical signed-64\ndecimal integer string: `0`, a positive digit sequence with no leading zero,\nor `-` followed by"

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

FR-505 Inputs, rule 5, denominator explanation, early-number behavior and AC1/AC3 match merged QSpec FR-154 and all TC-981 vectors read first. TC-1300 reflects string extremes and early numeric failures; FR-517 and TC-1312 require real emitted typed values. Existing upstream ownership links remain intact; no new dependency edges, version pin or tracking mechanism. Changed criteria have real trace bindings. No applicable AssuranceProfile was found by type search.

## Evidence and limits

Source review only; no builds, tests, Cargo, locks or gates executed by this reviewer. Existing /tmp/qsl1693-rational-tests.log records 14 passed; clippy, root-clippy, format and spec logs were inspected. The handoff-test log records the named native IR failure. These are coder evidence, not reviewer reruns. No whole-repository claim.

Runtime model identity is unavailable in the harness metadata; model=unknown is disclosed rather than inferred.

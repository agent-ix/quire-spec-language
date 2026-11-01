---
id: SR-202610310140
title: "Gap analysis — QSL-1390 equality sidecar producer"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@436299918af3ea46a5257f705eab08575b50a0e9; git diff origin/main...HEAD; qsl-package/src/emit.rs; qsl-package/src/emit/tests/converted_equality.rs; qsl-package/src/emit/tests/variant_types.rs; qsl-semantics/src/check/ir.rs; qsl-semantics/src/check/lowering.rs; QSpec FR-322-AC-288; IR FR-353-AC-1; IR pinned ref f9e28341"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-093", type: references }
---

## Summary

Audited the QSL-1390 producer-side implementation and tagged tests against QSpec FR-322-AC-288 and the relevant QSL lowering and enum requirements. The required Quoin matrix did not complete within the review window, so the scoped acceptance-to-test/code reconciliation is manual; plan completion was not assessed.

## Verdict

**CONDITIONAL** — the sidecar has production coverage for its schedules and descriptors, but the test for occurrence-specific producer facts stops at the in-memory identity rows and leaves the serialized per-occurrence conversion data unverified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The structurally reused equality test proves that two producer entries share a NodeKey but differ in Location and authored conversion target, then checks only emitted identity rows. It does not assert the two `checked_term_data.equality_decisions` rows or their distinct `conversion_target` values. The generic sidecar test uses separate application keys, so it cannot catch loss or cross-join of occurrence-specific decisions for a shared key. | qsl-package/src/emit/tests/converted_equality.rs:142-186, 275-337 |

## Coverage

- Reconciliation: manual acceptance-criteria-to-tests/code check for the requested QSL-1390 equality sidecar scope. `quoin matrix --repo . --json` produced no output and was interrupted after more than 180 seconds; no whole-repository matrix result or coverage counts are claimed.
- Plan completion: not assessed
- Examined requirement: QSpec FR-322-AC-288, including each sidecar family and the producer-selected conversion, schedule, quantity and enum facts.
- Reverse-gap review: touched implementation behavior maps to QSL-1390 and QSpec FR-322-AC-288; no new user-visible interface, side effect, or feature removal was found in the diff.
- Stubs: none found in the changed source or test code.
- Semantic review: scoped intent/test/code comparison performed for the equality sidecar producer contract; the cross-repository IR read remains explicitly out of scope and typed-refuses nonempty equality rows at pinned `f9e28341`.

## Examined scope

- {id: QSpec-FR-322-AC-288, path: "QSpec/spec/objects/interfaces/FR-322-checked-package-artifact.md", role: examined, excerpt: "Each row retains the `equal` or `not_equal` operator, the two original operand positions in order, each operand's source type and producer-selected conversion target (or `null`), the selected `text`, `enum`, `quantity` or `plan` schedule, the resolved top-level quantity unit facts, and the filtered enum member facts."}
- {id: IR-FR-353-AC-1, path: "quire-contract-ir/spec/checked_package/functional/FR-353-read-admitted-equality-decisions.md", role: context_only, excerpt: "A public external consumer reads admitted `=` and `!=` applications by each equality application's full node identity and exact occurrence and receives QSV `AdmittedEqualityDecision`; its read-only typed API exposes the producer-selected operator, original operand positions, source and conversion target types, and exact schedule in source order."}
- {id: FR-093-AC-31, path: "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md", role: context_only, excerpt: "A conversion between Rational, Decimal and Int types in either direction, and a Quantity conversion `convert<Km>(q) = r` (`quire.op.quantity.convert` with mode `rounding` = `exact`, then `quire.op.quantity.eq`), each lowers a convert node and the equality of the converted type."}
- {id: FR-360-AC-8, path: "spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md", role: context_only, excerpt: "The checked package emits `Config::Tier` as a `model`/`variant_type` node, the case literal as an `enum_value` of it and the argument by `ref` with no conversion node, and reads back `Verified`, also for a type whose cases include `Gold Plus`, `2x` and `Café`."}
- {id: qsl-package/src/emit.rs, path: "qsl-package/src/emit.rs", role: examined, excerpt: "Joins retained equality facts to exact source occurrences and builds typed sidecar decisions, quantity unit rows and enum member rows."}
- {id: qsl-package/src/emit/tests/converted_equality.rs, path: "qsl-package/src/emit/tests/converted_equality.rs", role: examined, excerpt: "Covers schedules, unit facts, sidecar shape and missing witness refusals; the shared-key test does not inspect the serialized equality decisions."}
- {id: qsl-package/src/emit/tests/variant_types.rs, path: "qsl-package/src/emit/tests/variant_types.rs", role: examined, excerpt: "Checks model enum sidecar entries against exact emitted enum and member nodes, including non-identifier case names."}
- {id: qsl-semantics/src/check/lowering.rs, path: "qsl-semantics/src/check/lowering.rs", role: examined, excerpt: "Retains equality producer data under `(NodeKey, Location)` and derives exact unit, dimension, text/plan type and enum-member identities during lowering."}
- {id: QSL-1390, path: "Linear QSL-1390", role: context_only, excerpt: "This PR is only the QSL producer-side equality sidecar; IR's trusted reader consumes typed no-recheck data after the separate IR work."}

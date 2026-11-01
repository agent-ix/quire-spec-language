---
id: SR-202610310140
title: "Gap analysis — QSL-1390 equality sidecar producer"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language#2013@31070ce6c178705abdb0d2e928c9600d5c9ec645; git diff origin/main...HEAD; qsl-package/src/emit.rs; qsl-package/src/emit/tests.rs; qsl-package/src/emit/tests/converted_equality.rs; qsl-package/src/emit/tests/golden.rs; qsl-package/src/emit/tests/variant_types.rs; qsl-semantics/src/check/ir.rs; qsl-semantics/src/check/lowering.rs; QSpec FR-322-AC-288; IR FR-353-AC-1; IR main 67f4ffe6"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-093", type: references }
---

## Summary

Audited PR #2013 at `31070ce6` against current `origin/main` and QSpec FR-322-AC-288 / IR FR-353-AC-1. The emitter's per-fact associations remain covered by the fixed prior findings; two enum reader tests were not migrated to the newly landed accessor contract.

## Verdict

**CONDITIONAL** — two enum-sidecar tests still require the obsolete package refusal and therefore fail after the current IR model admits equality carriers.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The structurally reused equality test proves that two producer entries share a NodeKey but differ in Location and authored conversion target, then checks only emitted identity rows. It does not assert the two `checked_term_data.equality_decisions` rows or their distinct `conversion_target` values. The generic sidecar test uses separate application keys, so it cannot catch loss or cross-join of occurrence-specific decisions for a shared key. | qsl-package/src/emit/tests/converted_equality.rs:142-186, 275-337 |
| FND-002 | medium | Two variant-type tests assert a typed package-level `UnsupportedConstruct` refusal at the first equality row. IR `67f4ffe6` now admits the equality carrier and the tests fail on `Read::Verified`, leaving enum witness outcomes unverified. They should use the shared admitted-equality outcome assertion, which checks the exact `UnavailableEqualityWitness` without changing wire coverage. | qsl-package/src/emit/tests/variant_types.rs:100-121, 383-402 |

## Coverage

- Reconciliation: manual acceptance-criteria-to-tests/code check for the QSL-1390/QSL-1430 producer and IR integration. `quoin matrix --repo /private/tmp/qsl1390-review --json` produced no output and was interrupted after about four minutes; no whole-repository matrix result or coverage counts are claimed.
- Plan completion: not assessed
- Examined requirements: QSpec FR-322-AC-288 and IR FR-353-AC-1, including exact application/occurrence joins, source and conversion types, schedules, quantity and enum facts, and typed missing-witness outcomes.
- Reverse-gap review: touched implementation behavior maps to QSL-1390 and QSpec FR-322-AC-288; no new user-visible interface, side effect, or feature removal was found in the diff.
- Stubs: none found in the changed source or test code.
- Semantic review: scoped intent/test/code comparison performed. The producer rows are in scope; the separate reader implementation is not. The two stale variant tests are a test gap because the integration's package read now succeeds while its admitted accessor can return the expected typed missing enum witness.
- Gates: `converted_equality::` passed 22/22; `conforming_reference_equality_is_accepted_by_the_i2_read` passed 1/1; `enum_members_no_literal_names_are_placed_under_an_equality` passed 1/1. `pending_equality_sidecar` failed both variant-type tests. `cargo fmt --check` and `git diff --check origin/main...HEAD` passed. Current QSpec main is `1b0f6604`; its CL-57..61 incompatibility remains an external aggregate blocker, and the full aggregate was not run.

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

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commit `60c4b386be5c55b8069de3e0a48dc23c96a4322a` adds a regression that reads canonical serialized package bytes, joins each decision by emitted application and occurrence, and asserts `null` for the unconverted occurrence versus the exact emitted ms type for the converted occurrence. The focused test passes. |
| FND-002 | fixed | Commit `89697bbe0b654d02ca4c29207d1ead8cd8452e69` routes the variant tests through the shared reader-outcome check. The focused tests now assert the exact missing enum-member witness and pass. |

FND-001 after_excerpt:

```rust
if facts.authored_operands()[0].target().is_some() {
    assert_ne!(decision["left"]["conversion_target"], Value::Null);
    assert_eq!(
        decision["left"]["conversion_target"], decision["right"]["source_type"],
        "the converted occurrence targets the exact emitted ms type"
    );
    assert!(converted_type
        .replace(decision["right"]["source_type"].clone())
        .is_none());
} else {
    assert_eq!(decision["left"]["conversion_target"], Value::Null);
}
```

FND-002 after_excerpt:

```rust
assert_eq!(error.witness, Some(expected_witness), "decision {index}");
assert_eq!(serde_json::to_value(error.at.as_ref()).unwrap(), expected_at);
```

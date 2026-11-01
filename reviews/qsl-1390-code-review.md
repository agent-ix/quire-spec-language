---
id: SR-202610310139
title: "Code review — QSL-1390 equality sidecar producer"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@436299918af3ea46a5257f705eab08575b50a0e9; git diff origin/main...HEAD; qsl-package/src/emit.rs; qsl-package/src/emit/tests/converted_equality.rs; qsl-package/src/emit/tests/variant_types.rs; qsl-semantics/src/check/ir.rs; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/refusal.rs; QSpec FR-322-AC-288; IR FR-353-AC-1; IR pinned ref f9e28341; QSL-1390"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-093", type: references }
---

## Summary

Reviewed the frozen QSL-1390 producer-side equality sidecar implementation and its tests against QSpec FR-322-AC-288, the QSL ticket contract, and the IR pinned at f9e28341. The IR reader's typed refusal for nonempty equality rows is an expected boundary for this PR; one occurrence-specific sidecar assertion is still missing.

## Verdict

**CONDITIONAL** — the emitter preserves the exact `(NodeKey, Location)` association in implementation, but the regression test for structurally shared equality nodes does not assert that the serialized sidecar retains both distinct conversion decisions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The shared-key regression checks producer facts and emitted identity rows, but never reads `checked_term_data.equality_decisions` for the two occurrences. The separate sidecar test uses distinct equality applications. A regression that assigns the same conversion target to both serialized rows would therefore pass, despite FR-322 requiring occurrence-specific nullable conversion targets. Assert each row's application/occurrence key and `left.conversion_target` (`null` vs the exact emitted ms type) in the sidecar. | qsl-package/src/emit/tests/converted_equality.rs:142-186, 275-337 |

## Coverage

- Rust review: changed emitter, lowering, refusal and test files; exact identity joins, conversion provenance, unit and enum descriptor provenance, missing-witness refusals, canonical sidecar shape, side effects and feature preservation inspected.
- Gates: `cargo fmt --check` passed; `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed; `cargo test -p qsl-package --lib converted_equality::` passed (20/20); `cargo test -p qsl-package --lib variant_types::` passed (9/9). The broader `cargo test -p qsl-package -p qsl-semantics` run was interrupted while unrelated 100,000-node/deep-body tests were still running. `cargo deny check` exited 4 on the existing AGPL license allowlist and Git-source policy.
- No CI workflow files changed. No applicable AssuranceProfile was present under `spec/`.
- IR boundary: reader refusal at `/checked_term_data/equality_decisions/0` is expected at pinned IR `f9e28341`; no reader implementation is requested by this review.

## Examined scope

- {id: QSpec-FR-322-AC-288, path: "QSpec/spec/objects/interfaces/FR-322-checked-package-artifact.md", role: examined, excerpt: "Each row retains the `equal` or `not_equal` operator, the two original operand positions in order, each operand's source type and producer-selected conversion target (or `null`), the selected `text`, `enum`, `quantity` or `plan` schedule, the resolved top-level quantity unit facts, and the filtered enum member facts."}
- {id: IR-FR-353-AC-1, path: "quire-contract-ir/spec/checked_package/functional/FR-353-read-admitted-equality-decisions.md", role: context_only, excerpt: "A public external consumer reads admitted `=` and `!=` applications by each equality application's full node identity and exact occurrence and receives QSV `AdmittedEqualityDecision`; its read-only typed API exposes the producer-selected operator, original operand positions, source and conversion target types, and exact schedule in source order."}
- {id: FR-093-AC-31, path: "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md", role: context_only, excerpt: "A conversion between Rational, Decimal and Int types in either direction, and a Quantity conversion `convert<Km>(q) = r` (`quire.op.quantity.convert` with mode `rounding` = `exact`, then `quire.op.quantity.eq`), each lowers a convert node and the equality of the converted type."}
- {id: FR-360-AC-8, path: "spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md", role: context_only, excerpt: "The checked package emits `Config::Tier` as a `model`/`variant_type` node, the case literal as an `enum_value` of it and the argument by `ref` with no conversion node, and reads back `Verified`, also for a type whose cases include `Gold Plus`, `2x` and `Café`."}
- {id: QSL-1390, path: "Linear QSL-1390", role: context_only, excerpt: "The emitter captures the equality schedule, conversion-target decisions, resolved UnitTable and filtered EnumMemberIndex at emit/admission time into the checked package; the reader consumes IR's typed no-recheck read and does not expose a public constructor from arbitrary data."}
- {id: qsl-package/src/emit.rs, path: "qsl-package/src/emit.rs", role: examined, excerpt: "Builds checked_term_data equality decision rows from retained producer data and identity witnesses, refusing when exact joins or required identity context are missing."}
- {id: qsl-package/src/emit/tests/converted_equality.rs, path: "qsl-package/src/emit/tests/converted_equality.rs", role: examined, excerpt: "Tests source occurrence retention, shared structural equality keys, serialized sidecar shape, operand order, schedules, quantity units, and typed refusal for missing or duplicate witnesses."}
- {id: qsl-package/src/emit/tests/variant_types.rs, path: "qsl-package/src/emit/tests/variant_types.rs", role: examined, excerpt: "Tests exact selected model-enum member identities in sidecar rows and retains typed refusal from the pinned IR reader."}
- {id: qsl-semantics/src/check/ir.rs, path: "qsl-semantics/src/check/ir.rs", role: examined, excerpt: "Retains equality operator, schedule, authored and compared operands, resolved units, filtered enum-member index, and exact producer identity witnesses."}
- {id: qsl-semantics/src/check/lowering.rs, path: "qsl-semantics/src/check/lowering.rs", role: examined, excerpt: "Stores producer data and identities under `(application key, source location)` and constructs exact unit, dimension, text/plan type and enum-member identities."}
- {id: qsl-semantics/src/check/refusal.rs, path: "qsl-semantics/src/check/refusal.rs", role: examined, excerpt: "Adds a typed MissingEnumMember keying refusal when a compared enum member has no unique exact source or admitted-model identity."}

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commit `60c4b386be5c55b8069de3e0a48dc23c96a4322a` adds a regression that reads canonical serialized package bytes, joins each decision by emitted application and occurrence, and asserts `null` for the unconverted occurrence versus the exact emitted ms type for the converted occurrence. The focused test passes. |

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

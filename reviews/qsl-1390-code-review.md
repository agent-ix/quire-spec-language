---
id: SR-202610310139
title: "Code review — QSL-1390 equality sidecar producer"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language#2013@31070ce6c178705abdb0d2e928c9600d5c9ec645; git diff origin/main...HEAD; qsl-package/src/emit.rs; qsl-package/src/emit/tests.rs; qsl-package/src/emit/tests/converted_equality.rs; qsl-package/src/emit/tests/golden.rs; qsl-package/src/emit/tests/variant_types.rs; qsl-semantics/src/check/ir.rs; qsl-semantics/src/check/lowering.rs; QSpec FR-322-AC-288; IR FR-353-AC-1; IR main 67f4ffe6; QSL-1390; QSL-1430"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-093", type: references }
---

## Summary

Reviewed PR #2013 at `31070ce6` against current `origin/main` and QSpec FR-322-AC-288 / IR FR-353-AC-1. The per-fact producer rows remain within scope; the newly integrated IR read exposes one stale enum test that still expects the prior package-level refusal.

## Verdict

**CONDITIONAL** — one enum-sidecar test still asserts the obsolete `UnsupportedConstruct` package refusal instead of exercising the admitted equality accessor's typed missing-witness outcome.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The shared-key regression checks producer facts and emitted identity rows, but never reads `checked_term_data.equality_decisions` for the two occurrences. The separate sidecar test uses distinct equality applications. A regression that assigns the same conversion target to both serialized rows would therefore pass, despite FR-322 requiring occurrence-specific nullable conversion targets. Assert each row's application/occurrence key and `left.conversion_target` (`null` vs the exact emitted ms type) in the sidecar. | qsl-package/src/emit/tests/converted_equality.rs:142-186, 275-337 |
| FND-002 | medium | The two variant-type reader tests still require `UnsupportedConstruct` at `/checked_term_data/equality_decisions/0`, but locked IR `67f4ffe6` now admits the package and exposes typed equality outcomes through `admitted_equality`. Both tests fail because the read is `Verified`; they no longer verify the intended enum `UnavailableEqualityWitness`. Route these cases through `assert_equality_reader_outcomes` and preserve the wire assertions. | qsl-package/src/emit/tests/variant_types.rs:100-121, 383-402 |

## Coverage

- Rust review: changed emitter, lowering, refusal and test files; exact identity joins, conversion provenance, unit and enum descriptor provenance, missing-witness refusals, canonical sidecar shape, side effects and feature preservation inspected.
- Gates: `cargo fmt --check` passed; `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed; `cargo test -p qsl-package --lib converted_equality::` passed (20/20); `cargo test -p qsl-package --lib variant_types::` passed (9/9). The broader `cargo test -p qsl-package -p qsl-semantics` run was interrupted while unrelated 100,000-node/deep-body tests were still running. `cargo deny check` exited 4 on the existing AGPL license allowlist and Git-source policy.
- No CI workflow files changed. No applicable AssuranceProfile was present under `spec/`.
- IR boundary: the producer emits rows and calls IR's typed admitted-equality accessor; the known converted-Plan and TextBounds mismatches are tracked in IR-1233. No IR implementation changes are requested from this QSL PR.
- Updated integration checks: `converted_equality::` passed 22/22; `conforming_reference_equality_is_accepted_by_the_i2_read` passed 1/1; `enum_members_no_literal_names_are_placed_under_an_equality` passed 1/1. `pending_equality_sidecar` failed both variant-type tests because their assertion still expects `UnsupportedConstruct`, while package admission now returns `Verified`. `cargo fmt --check` and `git diff --check origin/main...HEAD` passed. A broader `emit::tests::` invocation hit known unrelated package failures and was interrupted during a 100,000-depth test; no aggregate pass is claimed.
- Main alignment: current QSL main `61c05758` changes only probabilistic scheduler/spec paths; no changed paths overlap the PR's equality implementation. The IR-1233 converted-Plan mismatch and TextBounds witness mismatch remain external tracked issues.

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
| FND-002 | fixed | Commit `89697bbe0b654d02ca4c29207d1ead8cd8452e69` replaces the obsolete package-refusal assertion with the shared equality outcome check. The two enum tests pass and check `UnavailableEqualityWitness(EnumMember)` at the serialized member node. |

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

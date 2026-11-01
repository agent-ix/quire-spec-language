---
id: SR-202610310141
title: "Code review — QSL-1430 equality witness stage rows"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language#2013@1a65d4a6ac6903f2a39cef10af3bb475c7e36d52; git diff origin/main...HEAD; qsl-package/src/emit.rs; qsl-package/src/emit/tests/converted_equality.rs; qsl-semantics/src/check/ir.rs; qsl-semantics/src/check/lowering.rs; QSL-1390; QSL-1430; QSpec FR-322-AC-288/289; IR FR-352/353"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-093", type: references }
---

## Summary

Reviewed the expanded QSL-1390/QSL-1430 producer change at the frozen PR head. Existing QSL-1390 equality sidecar behavior, canonical wire rows, and the explicit IR reader-refusal boundary remain intact. The new per-fact witness stage rows add useful structural IDs, but dimension-role retention is incomplete, compound-unit identity is reconstructed from matched contents, and the tests do not assert the required row pairings.

## Verdict

**CONDITIONAL** — the new producer API does not preserve every dimension role and uses a content search to recover compound structural identity. Focused equality tests, formatting, and touched-crate Clippy pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Dimension identity traversal deduplicates only by `NodeKey`, before considering `EqualityDimensionRole`. A normal declared-unit conversion reaches the same dimension first as `UnitDimension` and again as `ConversionDimension`; the second visit is discarded, so the emitted per-fact rows omit that distinct role and its multiplicity. Preserve visits by exact `(node, role)` provenance and test that both roles join to the same structural id with their distinct ordinals. | qsl-semantics/src/check/lowering.rs:4754-4761, 4815-4840 |
| FND-002 | medium | Compound UnitId rows recover `quantity_key`/`unit_key` by scanning all graph nodes and selecting the first `compound_unit` whose body terms equal the compound's terms. This reverse-maps semantic contents to identity instead of carrying the exact operand/type witness; a future structural representation change or competing candidate can produce an unrelated association. Carry the exact checked type key with the producer fact and validate its UnitId, or refuse when that witness is absent. | qsl-package/src/emit.rs:2062-2078, 2830-2868 |
| FND-003 | medium | The new stage-row test asserts that some rows of each variant exist and that each selected structural id occurs somewhere in the wire node list, but never checks the row's UnitId-to-quantity id, dimension role/ordinal, declaration-to-base id, edge source/target/scale, or Text/Plan type id/role against producer facts. Wrong cross-joins therefore pass the test despite these rows being the new trust boundary. Assert exact producer-to-emitted pairings for each row family, including source and conversion roles. | qsl-package/src/emit/tests/converted_equality.rs:335-379, 428-448 |

## Coverage

- Rust review: emitter API and joins, lowering identity provenance, refusal paths, row ownership/source context, canonical sidecar tests, old QSL-1390 behavior, and no-reader-support boundary inspected.
- Gates: `cargo fmt --check` passed; `cargo test -p qsl-package --lib converted_equality::` passed (21/21); `cargo clippy -p qsl-package -p qsl-semantics --all-targets --all-features -- -D warnings` passed; `git diff --check origin/main...HEAD` passed. The full aggregate workspace gate remains with the lead.
- No CI workflow files changed. No applicable AssuranceProfile was present under `spec/`.
- IR boundary: the checked-package model used by this PR is pinned at `f9e28341`; its typed refusal for nonempty equality rows is expected here. No reader implementation is requested by this review.

## Examined scope

- {id: IR-FR-353-AC-1, path: "/private/tmp/ir754-api/spec/checked_package/functional/FR-353-read-admitted-equality-decisions.md", role: examined, excerpt: "A public external consumer reads admitted `=` and `!=` applications by full application node identity and exact occurrence, and receives the producer-selected operator, original operand positions, source and conversion target types, and exact schedule in source order."}
- {id: IR-FR-352, path: "/private/tmp/ir754-api/spec/checked_package/functional/FR-352-project-admitted-exact-value-types.md", role: context_only, excerpt: "A witness SHALL bind that exact structural node to the selected declaration and its Exact identity in the same selected package closure."}
- {id: QSpec-FR-322-AC-288, path: "/private/tmp/qsl1390-equality-qspec/spec/objects/interfaces/FR-322-checked-package-artifact.md", role: examined, excerpt: "Quantity records carry normalized dimension terms and either a declared unit's exact root and conversion-edge path or a compound unit's canonical root terms and exponents."}
- {id: QSpec-FR-322-AC-289, path: "/private/tmp/qsl1390-equality-qspec/spec/objects/interfaces/FR-322-checked-package-artifact.md", role: context_only, excerpt: "The reader joins each equality row to exactly one local equality application `NodeRef` and source occurrence. The row has an empty `term_path`, which selects that application node as the equality root; a non-empty path refuses `invalid_package`/`invalid-value` at the first path segment."}
- {id: QSL-1430, path: "Linear QSL-1430", role: examined, excerpt: "The emitter publishes per-fact stage rows for each fact kind while it still holds emitted node ids; IR must not synthesize witnesses from names or iteration order and refuses typed until exact rows are present."}
- {id: QSL-1390, path: "Linear QSL-1390", role: examined, excerpt: "The QSL change captures equality schedules, conversion-target decisions, resolved UnitTable facts and enum identities at producer/admission time; IR trusted reader work is separate."}
- {id: qsl-package/src/emit.rs, path: "qsl-package/src/emit.rs", role: examined, excerpt: "Builds exact occurrence sidecar rows and newly added per-fact equality witnesses joined to final CheckedNodeIds."}
- {id: qsl-semantics/src/check/lowering.rs, path: "qsl-semantics/src/check/lowering.rs", role: examined, excerpt: "Retains per-UnitId dimension declarations, ordered terms and conversion edges, and Text/Plan operand type keys."}
- {id: qsl-package/src/emit/tests/converted_equality.rs, path: "qsl-package/src/emit/tests/converted_equality.rs", role: examined, excerpt: "Tests canonical occurrence-specific sidecar rows, witness-row existence and typed refusal for missing, foreign and mismatched joins."}


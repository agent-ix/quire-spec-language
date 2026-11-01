---
id: SR-202610310142
title: "Gap analysis — QSL-1430 equality witness stage rows"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language#2013@1a65d4a6ac6903f2a39cef10af3bb475c7e36d52; git diff origin/main...HEAD; QSL-1390; QSL-1430; QSpec FR-322-AC-288/289; IR FR-352/353; qsl-package/src/emit.rs; qsl-semantics/src/check/lowering.rs; qsl-package/src/emit/tests/converted_equality.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-093", type: references }
---

## Summary

Manually reconciled the expanded producer slice against the QSL-1390 and QSL-1430 contracts, QSpec FR-322, IR FR-352/353, the changed source and the focused tests. The equality wire sidecar remains sound for the previously reviewed scope, and the new stage rows cover the requested fact families. However, the row construction loses a distinct dimension role, and the tests do not verify exact producer-to-structural-id pairings or bind the new acceptance test to an AC-level trace tag.

## Verdict

**CONDITIONAL** — a dimension-role witness is missing, and the new identity associations lack discriminating test evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | For a declared-unit equality with a conversion path, lowering first records the source unit's dimension as `UnitDimension`; when the path reaches a target unit in that same dimension, its `ConversionDimension` visit is discarded by the NodeKey-only visited set. The emitted stage rows therefore omit a required role for an otherwise valid conversion case. Track distinct role provenance while retaining the exact structural key and ordinal, then assert both rows. | qsl-semantics/src/check/lowering.rs:4754-4761, 4815-4840; qsl-package/src/emit.rs:2170-2225 |
| FND-002 | medium | The QSL-1430 witness test does not establish that any row is correctly paired: it checks variant existence and that a row id is present somewhere in the package node list, but does not compare UnitId/quantity type, dimension role and ordinal, dimension declaration/base ids, edge source/target/data, or Text/Plan type id/role with the exact producer facts. Its trace attribute names tickets/FR only (`QSL-1430`, `IR-FR-353`), not an acceptance criterion. Add exact row assertions tagged to `IR-FR-353-AC-1` and the relevant QSpec criterion. | qsl-package/src/emit/tests/converted_equality.rs:382-448 |
| FND-003 | medium | Compound UnitId association is recovered by scanning checked graph bodies for a compound unit with matching root terms. This synthesizes a structural identity from semantic contents instead of joining an exact producer-held `NodeKey` to the final `CheckedNodeId`; the UnitId itself is an opaque digest and is not that structural key. Carry the exact type key from the equality operand with its UnitId and refuse if no direct witness exists. | qsl-package/src/emit.rs:2062-2078, 2830-2868 |

## Coverage

- Reconciliation: manual acceptance-to-test/code review of QSL-1430's four row families and QSpec FR-322-AC-288/289 plus IR FR-353-AC-1. The requested `quoin matrix --repo . --json` produced no output and was interrupted after 180 seconds; no whole-repository matrix result or coverage counts are claimed.
- The explicit intent↔test↔code review found the dimension-role gap and insufficient exact-row test assertions described above. The remaining producer rows were inspected for source/target structural IDs, owner/source occurrence context and typed refusal on absent joins.
- Plan completion: not assessed.
- Stubs/coverage inflation: no TODO/FIXME/XXX placeholders or empty test bodies were found in the changed implementation/test scope.
- No CI workflow files changed; no applicable AssuranceProfile was present under `spec/`.
- The typed refusal for nonempty equality rows in the pinned IR model `f9e28341` is an expected integration boundary for this producer PR; reader implementation is not required here.

## Examined scope

- {id: IR-FR-353-AC-1, path: "/private/tmp/ir754-api/spec/checked_package/functional/FR-353-read-admitted-equality-decisions.md", role: examined, excerpt: "A public external consumer reads admitted `=` and `!=` applications by full application node identity and exact occurrence, and receives the producer-selected operator, original operand positions, source and conversion target types, and exact schedule in source order."}
- {id: IR-FR-352, path: "/private/tmp/ir754-api/spec/checked_package/functional/FR-352-project-admitted-exact-value-types.md", role: context_only, excerpt: "A witness SHALL bind that exact structural node to the selected declaration and its Exact identity in the same selected package closure."}
- {id: QSpec-FR-322-AC-288, path: "/private/tmp/qsl1390-equality-qspec/spec/objects/interfaces/FR-322-checked-package-artifact.md", role: examined, excerpt: "Quantity records carry normalized dimension terms and either a declared unit's exact root and conversion-edge path or a compound unit's canonical root terms and exponents."}
- {id: QSpec-FR-322-AC-289, path: "/private/tmp/qsl1390-equality-qspec/spec/objects/interfaces/FR-322-checked-package-artifact.md", role: context_only, excerpt: "The reader joins each equality row to exactly one local equality application `NodeRef` and source occurrence. The row has an empty `term_path`, which selects that application node as the equality root; a non-empty path refuses `invalid_package`/`invalid-value` at the first path segment."}
- {id: QSL-1430, path: "Linear QSL-1430", role: examined, excerpt: "Per-fact stage rows pair UnitIds with quantity types; dimension declarations and edges preserve structural IDs, roles, multiplicity/order/ordinal; dimension terms retain base and declaration IDs; Text/Plan rows retain type ids and source/conversion role."}
- {id: QSL-1390, path: "Linear QSL-1390", role: examined, excerpt: "The emitter captures equality schedules, conversion-target decisions, resolved UnitTable facts and enum identities at producer/admission time; the reader consumes IR's typed no-recheck read in the separate IR work."}
- {id: qsl-semantics/src/check/lowering.rs, path: "qsl-semantics/src/check/lowering.rs", role: examined, excerpt: "Constructs equality identity facts for unit dimensions, ordered dimension terms and conversion edges, and typed Text/Plan operand keys."}
- {id: qsl-package/src/emit.rs, path: "qsl-package/src/emit.rs", role: examined, excerpt: "Joins producer identity facts to final emitted structural IDs and carries occurrence and owner/source context in the stage rows."}
- {id: qsl-package/src/emit/tests/converted_equality.rs, path: "qsl-package/src/emit/tests/converted_equality.rs", role: examined, excerpt: "Tests occurrence-specific canonical wire rows, per-fact witness row existence, selected-id membership and typed missing/foreign/mismatched refusal."}

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The lowering now retains same-node dimensions under distinct roles, and the new test verifies UnitDimension and ConversionDimension rows for the same key with distinct ordinals. |
| FND-002 | still-open | The test now checks exact producer-to-node fields but filters witness rows by application only. It does not assert that each witness row's occurrence equals the exact producer identity occurrence, so cross-occurrence witness swaps can satisfy the assertions. |
| FND-003 | fixed | Unit identity lookup now uses the checker-retained structural type key, and tests cover exact, missing and ambiguous key joins. |

FND-001 after_excerpt:

```rust
if !visited.insert((key.clone(), role)) {
    continue;
}
```

FND-003 after_excerpt:

```rust
let quantity_key = &unit_fact.structural_node;
```

## Fix-round 2 coder update (pending reviewer disposition)

The first expanded review's findings are retained above. The code review marked its FND-001/FND-002 fixed and FND-003 open; this gap review marked its FND-001/FND-003 fixed and FND-002 open. The code-review FND-002 and gap-analysis FND-003 name the compound UnitId direct-key join. The code-review FND-003 and gap-analysis FND-002 name exact row-pairing coverage; this fix round addresses the remaining application-plus-occurrence join.

At fix-round 2 head, the coder update is:

| Finding | Update |
| --- | --- |
| FND-001 | Retained: dimension visits preserve `(NodeKey, EqualityDimensionRole)` and test the same structural ID under both roles with distinct ordinals. |
| Compound UnitId key join (code FND-002 / gap FND-003) | Retained: exact checker-held type keys drive the UnitId join; no compound body/terms search remains; missing and ambiguous direct mappings refuse. |
| Exact row assertions (code FND-003 / gap FND-002) | Updated: row selection binds both application and occurrence, with a regression covering two occurrences sharing one application key. |

The final status awaits reviewer disposition round 2. Earlier reviewer comments and findings remain preserved above.

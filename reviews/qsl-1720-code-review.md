---
id: SR-20261105101031
title: "Code review with Rust lane of producer-origin unit identity routes"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; tests/it/producer_unit_identity.rs, tests/it/main.rs, spec/functional/FR-398-expose-the-admitted-unit-table-through-the-replay-facade.md, spec/test-cases/TC-1092-the-replay-facade-returns-the-admitted-unit-table.md"
review_set: subset
---

## Summary

Reviewed the complete test-only Rust diff and its registration. No production API, lock, workflow, dependency or identity constructor changes occur. The source generator creates actual QSL declarations; graph reconstruction copies the same context’s nominal keys, originals, dimensions, scale, offset and targets through the public owner API. Search of tests and qsl-replay found no reusable existing public graph reconstruction fixture. Panic paths are test assertions, all collections are fixture-bounded, and structural root ordering is deterministic.

The cross-arm oracle asserts both domains, the exact same admitted root term, inequality, ordering, magnitude preservation and formed resolution. The small route strict-reads real canonical bytes and verifies each tested root is present. The large route uses retained checked context rather than asserting byte presence; it completes exact dimensionless conversion before matching the real default-limit HandoffRefusal::Identity(InputBytes), with required greater than the fixed bound. A different refusal or an early IllTyped result fails the test. The direct 64-byte helper check is additional counter evidence only. Source mutations dropping Power’s compound arm, bypassing the handoff limit, changing the typed cause, or dropping an emitted root are caught by these assertions. No implementation was mutated during review.

Inspected focused evidence reports 3 passed, 0 failed, and typed InputBytes refusal. Warm large-route runtime evidence reports 168312 KiB peak RSS. No Cargo invocation, build, gate or test repeat was run by this reviewer; final scoped gate remains the lead’s responsibility.

## Examined scope

- FR-398 (examined): The context SHALL retain admitted declared-unit identities componentwise, including the explicit package name, without imposing a declaration-key encoding limit.
- FR-398-AC-2 (examined): Over a unit declaring two dimensions and three units (two of one dimension) and a function whose parameters are `Quantity<m>`, `Quantity<km>` and `Quantity<s>`, the context's tables equal, per unit and dimension, the spine's own checked package read independently of the unit table: the node id and name of each `unit` and `dimension` semantic node, and the `UnitId` the checked function signature carries for each `Quantity<u>` parameter, which carries the declared-unit domain and the node id's structural components. A derived dimension lists its base parts and exponents; a chain km to mm converts exactly (2 km is 2,000,000 mm), an offset unit with its offset (0 C is 5463/20 K) and units of unlike dimension do not convert. The unit ids differ; each unit names its dimension's node id; `m` and `km` convert to each other and neither converts to `s`; `unit_table` holds every admitted unit by its id; and a repeated lookup and a recompile return the same entries.
- FR-398-AC-8 (examined): A source family declaring distinct derived dimensions over one base dimension and one root unit per derived dimension yields distinct admitted root keys with the caller's explicit package name. The context's retained nominal facts reconstruct the same QSV unit topology without creating a node key. Equal positive and negative root terms give a dimensionless target. With sufficiently long admitted package names, QSV's exact conversion completes but its `identified_quantity` handoff refuses `Identity(InputBytes)` at the fixed 16 MiB owner limit, yielding neither quantity nor identified unit and leaving `UnitScope` unchanged. A short-name control completes the handoff; a direct identity encoder control under a smaller explicit limit is separate evidence, not a substitute for the handoff.
- TC-1092 (examined): The source generates declarations; it never supplies a constructed node key.
- FR-088-AC-12 (examined): assert_ne!(declared.unit(), compound.unit());
- FR-398-AC-1 (context_only): without decoding or constructing a node key or a unit id

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS within the stated incremental scope. Final scoped gate remains pending.

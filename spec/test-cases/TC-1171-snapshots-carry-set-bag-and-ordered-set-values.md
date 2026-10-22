---
id: TC-1171
title: "Snapshots and invocations carry set, bag and ordered-set values"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
---
# TC-1171: Snapshots and invocations carry set, bag and ordered-set values

## Description

Verify that admission reads `set`, `bag` and `ordered-set` values by QSpec FR-144's supplied-collection rules, writes sets and bags in canonical element order, decides frame changes by collection equality, witnesses out-of-range elements and counts elements toward `observation.values`.

Scope: FR-106-AC-50, FR-106-AC-51, FR-106-AC-52.

## Test Procedure

Fixture: a package with object type `Tagged` (fields `ids: Set<UUID>[0, 3]`, `counts: Bag<Int[0, 9]>[0, 4]`, `order: OrderedSet<Int[0, 9]>[0, 3]`, `peers: Set<Reference<Tagged>>[0, 2]`), a population `T` of `Tagged` holding one object `t1`, and an operation `retag` whose frame grants no field.

1. Admit the current snapshot FR-106-AC-50 writes; then the same document with the `ids` and `counts` elements reversed, and with the `order` elements reversed. Compare the typed observations and digests.
2. Admit each FR-106-AC-51 mutation of that snapshot, then the same mutations in a `retag` parameter of the same type and in a state-machine context field.
3. Admit `retag` invocations whose post snapshot reorders `ids` and `counts`, drops one `2` from `counts`, and writes `order` as `1, 3`; admit the `counts` post value `10` under `PostStateRange::Witness`; admit the document under `observation.values` at its exact count and one less.

Tag the tests `#[trace("TC-1171", "FR-106-AC-n")]`.

## Expected Results

- Step 1: admitted with the members and multiplicities FR-106-AC-50 lists; the reversed `ids`/`counts` document gives an equal observation and digest; the reversed `order` document gives a different one.
- Step 2: each refusal FR-106-AC-51 names, with its code, cause and path, rooted at the parameter or context field for the second pass.
- Step 3: the reorderings admit as no change; the dropped element and the reordered ordered set refuse `frame_violation`/`unauthorized-change`; the witness run admits with one `OutOfRange` at index 1; the exact limit admits and one less refuses `stage_limit_exceeded`/`node-count-exceeded`.

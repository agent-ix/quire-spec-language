---
id: TC-1201
title: "A schedulability claim emits a v2 claim node the I2 reader resolves"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-454
    type: verifies
---
# TC-1201: A schedulability claim emits a v2 claim node the I2 reader resolves

## Description

Verify that S4 emits each checked `schedulable` and `response` claim as a
QSpec FR-482 `claim`/`schedulability` node holding its task set, that the
policy and every premise enter the node id while spelling does not, that the
I2 reader resolves the claim by its triple for FR-248's checker, and that it
refuses a malformed node.

Scope: FR-454-AC-1 to FR-454-AC-4.

## Test Procedure

Fixtures: ADR-026 §12's `Ctl` in a unit declaring the dimension `Time` and
the unit `ms`, tasks `a`, `b`, `c` with priorities 1, 2, 3; the claims
`CtlFp`, `CtlEdf` and `response R on Ctl.c <= 12 ms under fixed_priority;`;
`Ctl` with the third WCET 6; `Ctl` with its tasks listed `c`, `a`, `b`; `Ctl`
with `jitter 0 ms` on `a`; a unit declaring `CtlFp`'s claim twice.

1. Compile the unit, read the three nodes and read each back through the I2
   reader.
2. Compare node ids, obligation identities and `package_id`s across the
   variants.
3. Resolve `CtlFp`'s triple in the recompiled package; resolve an unknown
   NodeId, a `claim`/`state_graph` node's NodeId and another ordinal through
   the schedulability lookup; give `CtlFp`'s triple to the temporal and
   state-graph lookups; settle a request with an unresolved triple; check
   `CtlFp`'s outcome against `CtlEdf`'s triple.
4. Edit `CtlFp`'s node (policy `rate-monotonic`; tasks `b`, `a`, `c`; `a`'s
   period `0`), recompute the node key and `package_id`, and feed each to
   the I2 reader.

Tag the tests `#[trace("TC-1201", "FR-454-AC-n")]`.

## Expected Results

- Step 1: the bodies FR-454-AC-1 states, `dependencies` `[]`, occurrences
  of role `claim`; the reader returns each claim and its task set.
- Step 2: `CtlFp` and `CtlEdf` differ in node id and identity; the WCET-6
  variant differs in node id and `package_id`; the reordered and
  `jitter 0 ms` variants keep `CtlFp`'s node id; the doubled claim is one
  node with ordinals 0 and 1.
- Step 3: the claim and its task set; `clause-root-unresolved` for each bad
  triple and for the other lookups; `Failed` with no `check_closed_form`
  call; `stale_dependency`/`content-mismatch` naming both node ids.
- Step 4: `invalid_package`/`invalid-value`; `invalid_semantic_graph` at
  `tasks`; `invalid_timed_model`/`invalid-task-set` at `a`'s aggregate.

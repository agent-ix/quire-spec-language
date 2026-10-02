---
id: TC-589
title: "Complete enabling footprints and membership locations keep safety violations under partial-order reduction"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-156
    type: verifies
---
# TC-589: Complete enabling footprints and membership locations keep safety violations under partial-order reduction

## Description

Verify that a transition's enabling footprint is its whole read footprint
and that footprints carry receiver, argument and any-object membership, on
ADR-021 §7.6's six safety vectors, each of which reports a false `proved`
when either is missing.

Scope: FR-155-AC-5, FR-155-AC-6, FR-156-AC-5, FR-156-AC-6.

## Test Procedure

Fixtures: ADR-021 §7.6's guard-in-postcondition subject with `always
holds(not g.alarm)`; its deleting-the-receiver subject with `always
holds(not k.bad)`; its type-wide-delete subject with Job `m`
(`cancellable`, `target = j`) and `cancel()` framed `deletes Job` and
`modifies self.target`; its blocked-delete subject with `always holds(k.x =
0)`; its navigation subject with `always holds(not g.alarm)`; and its
`result` subject with `always holds(not g.alarm)`.

1. Derive and instantiate the footprints of `boom`, `set(a)`, `cancel(j)`,
   `boom(j)`, `cancel(m)`, `cancel(k)`, `unlink(j)`, `point` and the
   navigation subject's `boom`, and test the stated dependences.
2. Run each subject with partial-order reduction and without it, recording
   the ample set at the initial state, the stored state count and the
   outcome.

Tag the tests `#[trace("TC-589", "<AC id>")]`.

## Expected Results

- Step 1: `boom`'s enabling footprint holds `AnyField{v}`, which `set(a)`
  meets; `cancel(j)` writes `Membership{jobs, j}` and is dependent with
  `boom(j)`; `cancel(m)` writes `AnyMembership{jobs}` and is dependent with
  `boom(j)`; `cancel(k)` reads `AnyField{peer}`, which `unlink(j)` writes;
  the navigation subject's `boom` reads `(g, cell)`, which `point` writes.
- Step 2: guard-in-postcondition: the initial state is fully expanded and
  both runs return `Violated` with prefix `set, boom`, 6 states each.
  Deleting the receiver: `{cancel(j)}` is not chosen and both runs return
  `Violated` by `boom(j)`, 4 states each. Type-wide delete: `{cancel(m)}`
  is not chosen and both runs return `Violated` by `boom(j)`, 4 states
  each. Blocked delete: `{lock(k)}` is not chosen and both runs return
  `Violated` with `UndefinedEvaluation` after `unlink(j), cancel(k)`, 5
  states each. Navigation: `{disarm}` is not chosen and both runs return
  `Violated` by `point, boom`, 6 states each. No reduced run returns
  `Holds`.
- `result` subject: `pick`'s enabling footprint holds `AnyField{v}` and `AnyMembership{cells}`; `{disarm}` is not chosen and both runs return `Violated` by `set, pick`, 6 states each.

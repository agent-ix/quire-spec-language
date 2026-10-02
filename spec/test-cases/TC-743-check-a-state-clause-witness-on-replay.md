---
id: TC-743
title: "Replay re-derives a state-clause witness and checks that the element separates the clause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-268
    type: verifies
---
# TC-743: Replay re-derives a state-clause witness and checks that the element separates the clause

## Description

Verify FR-268: `replay_state_clause` re-derives the payload's record,
agrees only on an equal record, and its separation check rejects a record
whose quantifier, element or body does not separate the clause.

Scope: FR-268-AC-1 to FR-268-AC-4.

## Test Procedure

Compile FR-265's `witness` unit; build `Current` envelopes by hand with the
clause node and `claim` occurrence from the compiled package, and the
unit's source, the domain package and FR-265's snapshots in the byte
provision.

1. `AllBelow` over `high` carrying FR-265-AC-1's record, on the `Witness`
   and the `Input` arm; `NestedAll` over `high` with FR-265-AC-4's record;
   `FilteredAll` over `high` with FR-265-AC-5's record.
2. `AllBelow` over `high` with a record naming `leaf` at index 2; with a
   record naming `GuardedAll`'s `forall`; with no record.
3. Call the separation check over the admitted `high` observation and
   `AllBelow` with FR-265-AC-1's record; with its quantifier set to a node
   outside the claim; with index 3; with the value path naming another
   member; with the deciding element `leaf` at index 1; with `root` at
   index 0. Repeat FR-265-AC-1's record over variants of `AllBelow` whose
   domain expression evaluates `undefined` on `high`, whose body evaluates
   `undefined` for `mid`, and whose domain is refused (a `lookup` of an
   absent key under `absent refused`); then over `AllBelow` with the
   evaluation meter one unit below step 2's need.
4. TC-517 step 1's `VersionUnchanged` and `ParentOrder` envelopes; an
   `AllBelow` envelope whose reader bound is smaller than its encoded
   record; step 1's `Witness`-arm envelope twice.

Tag the tests `#[trace("TC-743", "FR-268-AC-n")]`.

## Expected Results

- Step 1: `reproduced-with-evaluated-witness` holding the record,
  `reproduced-without-witness`, and `reproduced-with-evaluated-witness`
  twice.
- Step 2: `inconclusive`, `Witness`, three times, each holding the given
  record (or its absence) and the re-derived `mid` record.
- Step 3: pass; then failures at steps 1, 3, 3, 3 and 4; then failures at
  step 2 (`UndefinedEvaluation`), step 4 (`UndefinedEvaluation`) and step 2
  (the refusal record); then `inconclusive` with `NoValue`.
- Step 4: no record on either envelope or result and FR-122-AC-1's
  settlements; a decode refusal under FR-070-AC-7; equal results.

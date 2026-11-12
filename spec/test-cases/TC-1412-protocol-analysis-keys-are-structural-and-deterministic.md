---
id: TC-1412
title: "Protocol analysis keys are structural, complete and deterministic"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-572
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-174
    type: references
  - target: ix://agent-ix/quire-specification/FR-175
    type: references
---
# TC-1412: Protocol analysis keys are structural, complete and deterministic

## Description

Verify which changes alter the key, that an absent member is refused by
builder and reader, that no digest is a member, and determinism.

Scope: FR-572-AC-1 to FR-572-AC-6; QSpec FR-174-AC-6, FR-175-AC-6.

## Test Procedure

1. Change, one at a time, each projection key member, then each of display
   name, path, span and producing run; compare keys.
2. Repeat step 1 for each realizability key member, including analyzer
   identity and version.
3. Read the key off each result type and inspect each member's type.
4. For each member, build a key without it and read a stored result with it
   removed; then build a realizability key with the role-knowledge premise
   `None`, and with it `Some` of the empty set.
5. Run each kind twice on one key and one assessed-subject root and compare
   serialized results; then run once under a different root and compare.
6. Change only the assessed-subject root of a request; vary `max_states`, `max_transitions` and the meter, stopping a run.

Tag the tests `#[trace("TC-1412", "FR-572-AC-n")]`.

## Expected Results

- Steps 1 and 2: every member change gives a different key; the four
  presentation changes give an equal key.
- Step 3: every member present and typed; none a digest.
- Step 4: each refused `invalid_runtime_input`/`missing-member` naming the
  member; the `None` and `Some`-empty keys are both accepted and unequal.
- Step 5: equal, byte for byte, under one root; under the other root they
  differ only in the separating witness record's root.
- Step 6: the key is unchanged; the stopped run carries no artifact.

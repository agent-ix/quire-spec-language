---
id: TC-744
title: "A witness disagreement settles inconclusive with a typed Witness cause that round-trips"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-269
    type: verifies
---
# TC-744: A witness disagreement settles inconclusive with a typed Witness cause that round-trips

## Description

Verify FR-269: `DisagreementCause::Witness` carries both verdicts, both
records and the failure, keeps the disputed record off the result, and
round-trips and compares strictly.

Scope: FR-269-AC-1 to FR-269-AC-4.

## Test Procedure

1. Replay TC-743 step 2's index-2 and no-record envelopes.
2. Call the separation check with each of TC-743 step 3's failing records
   and build the `Witness` cause from its failure.
3. Serialize and read back step 1's first result; read it with `failure`
   set to an undefined value; read it with `given.index` removed; compare
   it with a copy whose `derived` record names 0 at index 0. Round-trip a
   cause whose record decides on each value kind, including an
   `Option<Option<Int>>` and an `Option<Sequence<Int>>` payload type and a
   record with an absent and a null slot; read a `Refused` reason whose
   code, cause or a field name is empty or not a catalog spelling.
4. On a 512 KiB stack, round-trip a cause whose deciding element is an
   `Option` type nested 100,000 levels, and one whose deciding element is a
   record nested 100,000 levels.

Tag the tests `#[trace("TC-744", "FR-269-AC-n")]`.

## Expected Results

- Step 1: `Witness { proved: violation, replayed: violation, given: index-2
  record, derived: index-1 record, failure: Mismatch }` with no QSpec FR-351
  record on the result; then `given` absent and `derived` the index-1 record.
- Step 2: `Separation { Quantifier, Unmet }`, `Separation { Element,
  Unmet }` three times, `Separation { Body, Unmet }`; then
  `Separation { Domain, UndefinedEvaluation }`, `Separation { Body,
  UndefinedEvaluation }` and `Separation { Domain, Refused }`.
- Step 3: an equal result; two refusals; unequal. Each value kind reads
  back equal and encodes back to the same document; each bad spelling
  refuses.
- Step 4: each element reads back and encodes to the same document; no
  depth refuses.

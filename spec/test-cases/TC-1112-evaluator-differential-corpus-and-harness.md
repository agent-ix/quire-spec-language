---
id: TC-1112
title: "Encode, load and refuse evaluator corpus cases and fail on any evaluator divergence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-402
    type: verifies
---
# TC-1112: Encode, load and refuse evaluator corpus cases and fail on any evaluator divergence

## Description

Verify the FR-402 evaluator corpus: the facade's reference entry and the
loader agree on one encoding, the loader refuses a defective bundle whole
with the stated code, the harness reports every divergence with its pointer
and tolerates none, the generated cases are reproducible, and the harness
runs in `make ci` and over CG's bundle. Scope: FR-402-AC-1 to FR-402-AC-8.

## Test Procedure

1. Build a `CaseInput` for the FR-262-AC-1 `count` program with argument
   `3` and default limits; call `corpus::reference_record`; write the
   record's `line()` as a one-line bundle and load it (AC-1).
2. Rerun step 1 with the `work_units` limit at `w - 1` and at `w`, `w` being
   step 1's consumed `work_units`, and with the argument
   `WitnessValue::Boolean(true)` (AC-2).
3. Apply each single mutation AC-3 lists to step 1's bundle and load it;
   also load a two-line bundle repeating step 1's line. Count evaluator
   invocations during each load (AC-3).
4. Apply each member mutation of step 3 to step 1's `CaseInput` and call
   `reference_record`; list the public items of `qsl_replay::corpus` (AC-4).
5. Run the harness over a bundle holding step 1's record with one
   `function.call` entry removed from `expected.charges`, together with the
   QSL-owned corpus; then over step 1's unmodified record (AC-5).
6. Generate the corpus's generated cases twice with seed `0x5153_4C37` and
   count 256, compare the bytes and load each output (AC-6).
7. In the change that adds the `no_std` implementation, run `make ci` and
   inspect the harness's lanes and options (AC-7).
8. Build CG against the QSL revision under test, run CG's
   `make evaluator-corpus`, then
   `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>` (AC-8).

## Expected Results

- Step 1: `expected.result` is an `evaluation`, `outcome` `completed`,
  `value` `3`; exactly four `charges` entries have `point` `function.call`;
  the line is the record's JCS bytes plus `\n`; the loaded record equals the
  produced one.
- Step 2: `w - 1` gives `incomplete` with `limit_kind` `work_units`, at most
  `w - 1` consumed and no recorded denied charge; `w` gives step 1's record;
  the Boolean argument gives `call_failure` arm `input` with a
  `WrongValueKind` record and loads without refusal.
- Step 3: each mutation refuses the whole bundle with the code AC-3 states,
  naming line 1 and the member pointer; the repeated line refuses
  `case_order` at line 2; no evaluation runs during any load.
- Step 4: each `CaseInput` refuses with the loader's code for the same
  record; `qsl_replay::corpus` has no public checked-closure, evaluator or
  session type.
- Step 5: the run fails and reports the case with pair `expected/std` and a
  pointer under `/charges`, and reports every other case's result; the
  unmodified record passes.
- Step 6: the two outputs are byte-identical and both load.
- Step 7: `make ci` runs `evaluator-differential` over the QSL-owned corpus
  with zero `std/no_std` divergences, and the harness has no tolerance or
  skip setting.
- Step 8: every CG record loads and the run reports zero `std/no_std` and
  zero `expected/std` divergences.

## Metadata

- Priority: P1
- Target Integration: `qsl-replay/src/corpus.rs`, `xtask` `evaluator-differential`
- Automation: Automated Rust tests (steps 1-7); cross-repository run (step 8)

## Dependencies

**Upstream:** [FR-402](../functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md).
**Downstream:** none.

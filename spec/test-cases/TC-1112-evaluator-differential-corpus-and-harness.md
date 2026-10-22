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

Verify the FR-402 evaluator corpus: the facade's encoder and loader agree on
one encoding that binds the source identity and dependencies, the loader
refuses a defective bundle whole with the stated code and position, the
compared record carries the defined forms, the harness reports every
divergence with its pointer and tolerates none, the corpus holds exactly
its listed cases and reproducible generated cases, and the harness runs in
`make ci` and over CG's bundle. Scope: FR-402-AC-1 to FR-402-AC-9.

## Test Procedure

1. Build the `fr402/count-3` `CaseInput`, call `corpus::encode`, compare
   its `package_id` with `compile_package`'s for the same inputs, write
   `line()` as a one-line bundle and load it; encode again with a changed
   `source.identity.revision` (AC-1).
2. Encode and load a case whose source imports a library given in
   `dependencies`; encode it without the entry; load a record with two
   entries out of `identity` order (AC-2).
3. Load each bundle AC-3 lists, counting evaluator invocations (AC-3).
4. Call `encode` on each `CaseInput` built from AC-3's member mutations;
   list the public items of `qsl_replay::corpus` (AC-4).
5. Run the harness's std build over `fr402/count-3`,
   `fr402/count-3-short`, `fr402/count-3-boolean` and
   `fr262/ac1/count-100000` and read their compared records (AC-5).
6. Run the harness over a bundle holding `fr402/count-3` with an `expected`
   whose `consumed.work_units` is one less than the std record's, then with
   the std record as `expected`; feed the comparator two record sets
   differing in one `charges` entry's `sizes`, and two sets one of which
   lacks a `case_id` (AC-6).
7. Load the checked-in bundle and list its `case_id`s; generate the
   generated cases twice with seed `0x5153_4C37` and count 256, compare the
   bytes, load and compile each; diff `Cargo.lock` for new random-number or
   property-testing crates (AC-7).
8. Run `make ci` in a tree that ships both implementations; run FR-262-AC-3,
   AC-4, AC-6, AC-7 and AC-8's tests against each build; inspect the
   harness's command line, environment and configuration (AC-8).
9. Build CG against the QSL revision under test, run CG's
   `make evaluator-corpus`, then
   `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>` (AC-9).

## Expected Results

- Step 1: `expected` is `null`, `package_id` equals `compile_package`'s, the
  line is the `quire-canonical` text plus `\n`, the loaded record equals the
  encoded one, and the changed revision gives a different `package_id`.
- Step 2: the case encodes and loads with `compile_package`'s `package_id`;
  without the entry `encode` refuses `compile_refused` carrying
  `missing_import`; the misordered record refuses `dependency_order`.
- Step 3: each bundle refuses with AC-3's code; bundle refusals name no
  line, `malformed_line` and `noncanonical_line` name line 1 and no
  pointer, member refusals name line 1 and AC-3's pointer, the repeated
  line refuses `case_order` at line 2, and no load evaluates.
- Step 4: each input refuses with the loader's code; the module has no
  evaluating function and no checked-closure, evaluator, session or
  compared-record type.
- Step 5: `fr402/count-3` is `completed` with `3`'s canonical JSON form,
  four `function.call` charges and ten decimal-string counters;
  `fr402/count-3-short` is `incomplete` with `limit_kind` `work_units` and
  no denied-charge entry; `fr402/count-3-boolean` is `call_failure` `input`
  with the `WrongValueKind` code; `fr262/ac1/count-100000` has `charges`
  `null` and value `100000`.
- Step 6: the first run fails with pair `expected/std` and pointer
  `/consumed/work_units` and reports every other case; the second passes for
  that case; the comparator reports `std/no_std` under `/charges` and names
  the missing case and the build that produced no record.
- Step 7: the `case_id`s are exactly FR-402's list; the two generations are
  byte-identical and every unit loads and compiles; `Cargo.lock` gains no
  such crate.
- Step 8: `make ci` runs both builds with zero divergences; each FR-262
  test passes against both builds; no tolerance, skip or exclusion setting
  exists.
- Step 9: every CG record loads with `expected` `null`, and the run reports
  zero divergences.

## Metadata

- Priority: P1
- Target Integration: `qsl-replay/src/corpus.rs`, `xtask` `evaluator-differential`
- Automation: Automated Rust tests (steps 1-8); cross-repository run (step 9)

## Dependencies

**Upstream:** [FR-402](../functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md).
**Downstream:** none.

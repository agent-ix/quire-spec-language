---
id: TC-1112
title: "Load, admit and refuse evaluator corpus cases and fail on any evaluator divergence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-402
    type: verifies
---
# TC-1112: Load, admit and refuse evaluator corpus cases and fail on any evaluator divergence

## Description

Verify the FR-402 evaluator corpus: `quire-eval-corpus` depends on no QSL
crate, its case record binds the source identity and dependencies, its
loader and QSL's std admission refuse a defective bundle whole with the
stated code and position, the compared record carries the defined forms,
the harness reports every divergence with its pointer and tolerates none,
the corpus holds exactly its listed cases and reproducible generated cases,
and the harness runs in `make ci` and over CG's bundle. Scope: FR-402-AC-1
to FR-402-AC-10.

## Test Procedure

1. Inspect `quire-eval-corpus`'s manifest: dependency tables and the
   `charge-log` feature (AC-1).
2. Build the `fr402/count-3` `CaseRecord` with `compile_package`'s
   `package_id`, write `line()` as a one-line bundle, load it and admit it
   with QSL's std admission; admit it again with a changed
   `source.identity.authority`, with a changed `source.identity.identity`
   and with a changed `source.identity.revision`, each with the old
   `package_id` (AC-2).
3. Load and admit a case whose source imports a library given in
   `dependencies`; admit it without the entry; load a record with two
   entries out of `identity` order (AC-3).
4. Load each bundle AC-4 lists (AC-4).
5. Admit each bundle AC-5 lists, counting evaluator invocations (AC-5).
6. Run the harness's std build over `fr402/count-3`,
   `fr402/count-3-short`, `fr402/count-3-boolean` and
   `fr262/ac1/count-100000`, and build a compared record for a run that
   completes with a `Value::Text` (AC-6).
7. Run the harness over a bundle holding `fr402/count-3` with an `expected`
   whose `consumed.work_units` is one less than the std record's, then with
   the std record as `expected`, then with `expected` `null`; feed the
   comparator two record sets differing in one `charges` entry's `sizes`,
   and two sets one of which lacks a `case_id` (AC-7).
8. Load and admit the checked-in bundle and list its `case_id`s; generate
   the generated cases twice for count 256, compare the bytes and ids, load
   and admit each; run with an external bundle reusing the `case_id`
   `fr402/count-3`; diff `Cargo.lock` for new random-number or
   property-testing crates (AC-8).
9. Run `make ci` in a tree that ships both implementations and inspect each
   implementation's records; feed the comparator a `no_std` record set lacking one
   admitted case; run FR-262-AC-2,
   AC-3, AC-4, AC-6, AC-7 and AC-8's tests against each implementation; inspect the
   harness's command line, environment and configuration (AC-9).
10. Run CG's `make evaluator-corpus`, then
    `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>` (AC-10).

## Expected Results

- Step 1: no QSL crate in any table, normal dependencies within the stated
  list, and `charge-log` enables only `quire-exact/test-support`.
- Step 2: the line is the `quire-canonical` text plus `\n`, the loaded
  record equals the written one and admits; the changed authority and the
  changed identity each refuse `package_mismatch`, and the changed revision
  admits.
- Step 3: the case loads and admits; without the entry admission refuses
  `compile_refused` carrying `missing_import`; the misordered record refuses
  `dependency_order`.
- Step 4: each bundle refuses with AC-4's code; bundle refusals name no
  line, `malformed_line` and `noncanonical_line` name line 1 and no
  pointer, member refusals name line 1 and AC-4's pointer, and the repeated
  line refuses `case_order` at line 2.
- Step 5: each bundle refuses with AC-5's code, bundle, `case_id` and
  pointer, and no case is evaluated.
- Step 6: `fr402/count-3` is `completed` with `{"integer": "3"}`, four
  `function.call` charges and ten decimal-string counters;
  `fr402/count-3-short` is `incomplete` with `limit_kind` `work_units` and
  no denied-charge entry; `fr402/count-3-boolean` is `call_failure` `input`
  with the `WrongValueKind` code; `fr262/ac1/count-100000` has `charges`
  `null` and `{"integer": "100000"}`; the `Text` run refuses
  `unencodable_value` naming `text`.
- Step 7: the first run fails with pair `expected/std` and pointer
  `/consumed/work_units` and reports every other case; the second passes for
  that case; the third compares `std/no_std` only; the comparator reports
  `std/no_std` under `/charges` and names the missing case and the build
  that produced no record.
- Step 8: the `case_id`s are exactly FR-402's list; the two generations are
  byte-identical with ids `gen/000` to `gen/255`, and every case loads and
  admits; the reused id runs and reports as a second case under bundle
  `external`; `Cargo.lock` gains no such crate.
- Step 9: `make ci` runs the runner's one test build, which admits each
  case once and records every admitted case with each implementation,
  both records for `fr402/count-3` carry non-null `charges`, and the run has
  zero divergences; the lacking set is reported as a divergence; each FR-262
  test passes against both implementations; no tolerance, skip or exclusion setting
  exists.
- Step 10: every CG record loads and admits, the run reports zero
  divergences, and each non-null `expected` passes `expected/std`.

## Metadata

- Priority: P1
- Target Integration: `agent-ix/quire-eval` `quire-eval-corpus`, `xtask` `evaluator-differential`
- Automation: Automated Rust tests (steps 2-9); inspection (step 1); cross-repository run (step 10)

## Dependencies

**Upstream:** [FR-402](../functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md).
**Downstream:** none.

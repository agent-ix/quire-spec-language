---
id: TC-1124
title: "quire-eval is QSL's only evaluator after the switch"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-405
    type: verifies
---
# TC-1124: quire-eval is QSL's only evaluator after the switch

## Description

Verify FR-405. The switch from the std reference evaluator to `quire-eval`
is made only on its criterion. After it, `quire-eval` evaluates every family
in QSL, the std reference evaluator is gone, and the corpus is checked
against recorded expectations. Scope: FR-405-AC-1 to FR-405-AC-10.

## Test Procedure

1. On the switch base, run `make evaluator-differential`. Build CG's
   bundle with `make evaluator-corpus` on CG's `main` and run
   `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>`. Read the
   switch change's description.
2. On the switch base, run FR-262-AC-1 to FR-262-AC-8 with each
   implementation, AC-1 and AC-2 on a thread with a 512 KiB stack. Read the
   switch change's description.
3. Read the `quire-eval` revision in QSL's `Cargo.lock` at the switch head.
   In `agent-ix/quire-eval` at that revision, run FR-401-AC-1's
   `thumbv7em-none-eabi` build and slice S8's unsafe audit.
4. At the switch head, list `qsl-eval`'s files and its shipped types. For
   each `qsl-eval` function returning a `Value`, `Outcome` or `Evaluation`,
   trace where the result comes from. List every `NodeKind` match in
   `qsl-eval` with what it returns. Read every QSL manifest, `quire-eval`'s
   manifest, the `xtask` commands, the Makefile and the test-support
   features.
5. Build the test inventory (crate, file, test name, expected values) on
   the switch base and on the switch head, and diff them. Classify each
   test absent at the head. At the head, run the suites FR-405-AC-5 names.
6. At the switch head, build `pre(allInstances<T>(p))` with
   `p: Population<T>[3]` through `check_postcondition_expression`. Begin it
   through the `Value`-family entry. Evaluate it through the clause entry
   with a pre observation recording members `a`, `b` and a post observation
   recording `a`, `b`, `c`, with the charge log. Compare the charges with
   the checked-in fixture the switch base's reference produced.
7. At the switch head, read the checked-in bundle and
   `tests/fixtures/evaluator-corpus/generated/cases.jsonl`. Compare each
   `expected` with the switch base's std compared record for that case.
   Run the generator for count 256.
8. At the switch head, run `make ci`. Run `evaluator-regression` over a
   bundle holding `fr402/count-3` with `expected.consumed.work_units` one
   less than the run's, and again with `expected` `null`. Feed the
   comparator expectations and a `quire-eval` record set that differ only
   in one `charges` entry's `sizes`, and a record set that lacks one
   admitted case. Try `make evaluator-differential`.
9. At the switch head, run `make evaluator-regression
   EVALUATOR_CORPUS_DIR=<dir>` over CG's bundle with expectations recorded
   through `quire-eval`, and again with one record's `expected` set to
   `null`.
10. Read the history of `main` around the switch change.

## Expected Results

1. Every case admits, and both runs report zero divergences. The
   description reports the switch base and both results.
2. Every criterion passes with each implementation. No depth refusal and
   no stack overflow occur. The description reports the result.
3. Both gates pass.
4. `qsl-eval/src/value/expression/evaluate.rs` and its submodules,
   `Machine`, the std reference adapter and the separate `quire-eval`
   entry are absent. No type keeps an evaluation's task or value stack.
   Every traced result comes from a `quire-eval` session. Every remaining
   `NodeKind` match is a structural walk that returns no value, such as the
   ADR-031 SW-2 decision-path walk. No feature selects an evaluator, no test-only path
   holds one, and no deleted path re-exports or forwards.
5. Every absent test ran only the std reference as a second
   implementation, paired std and `no_std` records, or tested the separate
   entry. Each test of the deleted files exists ported with its expected
   values. Every other test exists with unchanged expected values, and
   every suite passes.
6. The `Value`-family Begin returns `CallFailure::Fault` with stage
   `quire-eval`, invariant `outside-value-family` and no charge. The clause
   entry completes with `a` and `b` in ascending reference-key order, with
   the fixture's admitted charges.
7. Every `expected` is non-null and equal to the base record. The
   generator's output equals the generated bundle with each `expected`
   replaced by `null`, byte for byte.
8. `make ci` runs `evaluator-regression` with zero divergences. The first
   bundle reports `fr402/count-3` with pair `expected/quire-eval` and
   pointer `/consumed/work_units`, and every other case still runs. The
   second reports `missing_expected`. The `sizes` difference reports pair
   `expected/quire-eval` and a pointer under `/charges`; the missing case is
   reported as a divergence with `quire-eval` producing no record.
   `evaluator-differential` does not exist. No setting tolerates, skips or excludes a case.
9. The first run reports zero divergences. The second reports that case as
   `missing_expected`.
10. No commit of `main` holds both `Machine` and a production path that
    evaluates on `quire-eval`.

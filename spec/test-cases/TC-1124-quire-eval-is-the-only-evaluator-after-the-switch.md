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
   `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>`.
2. On the switch base, run FR-262-AC-1 to FR-262-AC-8 with each
   implementation, AC-1 and AC-2 on a thread with a 512 KiB stack.
3. Read the `quire-eval` revision in QSL's `Cargo.lock` at the switch head.
   In `agent-ix/quire-eval` at that revision, run FR-401-AC-1's
   `thumbv7em-none-eabi` build and slice S8's unsafe audit.
4. At the switch head, read `qsl-eval`'s source, every QSL manifest,
   `quire-eval`'s manifest, the `xtask` commands, the Makefile and the
   test-support features.
5. At the switch head, run the suites FR-405-AC-5 names. Diff every test
   file's expected values against the switch base.
6. At the switch head, build `pre(allInstances<T>(p))` with
   `p: Population<T>[3]` through `check_postcondition_expression`. Begin it
   through the `Value`-family entry. Evaluate it through the clause entry
   with a pre observation recording members `a`, `b` and a post observation
   recording `a`, `b`, `c`, with the charge log.
7. At the switch head, read the checked-in bundle and
   `tests/fixtures/evaluator-corpus/generated/cases.jsonl`. Compare each
   `expected` with the switch base's std compared record for that case.
   Run the generator for count 256.
8. At the switch head, run `make ci`. Run `evaluator-regression` over a
   bundle holding `fr402/count-3` with `expected.consumed.work_units` one
   less than the run's, and again with `expected` `null`. Try
   `make evaluator-differential`.
9. At the switch head, run `make evaluator-regression
   EVALUATOR_CORPUS_DIR=<dir>` over CG's bundle with expectations recorded
   through `quire-eval`, and again with one record's `expected` set to
   `null`.
10. Read the history of `main` around the switch change.

## Expected Results

1. Every case admits, and both runs report zero divergences.
2. Every criterion passes with each implementation. No depth refusal and
   no stack overflow occur.
3. Both gates pass.
4. `Machine`, `qsl_eval::value::expression::evaluate`, the std reference
   adapter and the separate `quire-eval` entry are absent. `qsl-eval`'s
   shipped source names no `NodeKind` variant. No feature selects an
   evaluator, no test-only path holds one, and no deleted path re-exports
   or forwards.
5. Every suite passes, and no expected value differs from the switch base.
6. The `Value`-family Begin returns `CallFailure::Fault` with stage
   `quire-eval`, invariant `outside-value-family` and no charge. The clause
   entry completes with `a` and `b` in ascending reference-key order, with
   the admitted charges the reference recorded on the switch base.
7. Every `expected` is non-null and equal to the base record. The
   generator's output equals the generated bundle with each `expected`
   replaced by `null`, byte for byte.
8. `make ci` runs `evaluator-regression` with zero divergences. The first
   bundle reports `fr402/count-3` with pair `expected/quire-eval` and
   pointer `/consumed/work_units`, and every other case still runs. The
   second reports `missing_expected`. `evaluator-differential` does not
   exist. No setting tolerates, skips or excludes a case.
9. The first run reports zero divergences. The second reports that case as
   `missing_expected`.
10. No commit of `main` holds both `Machine` and a production path that
    evaluates on `quire-eval`.

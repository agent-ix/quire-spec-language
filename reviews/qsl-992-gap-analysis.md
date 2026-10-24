---
id: SR-16636
title: "Gap analysis of PR #1434: FR-194 undefined window positions and EN-4 test gaps"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2f6f9e7be215f2ab4485391ed3c7d5229122ba09; spec/functional/FR-188-*, FR-189-*, FR-193-*, FR-194-*, FR-196-*; qsl-eval/src/temporal/window.rs, qsl-eval/src/temporal/observed_window.rs, qsl-eval/tests/it/window_aggregates.rs, qsl-eval/src/simulation/choice.rs, qsl-analyze/src/statistical/tests.rs, qsl-analyze/tests/it/exact_probabilistic.rs, qsl-analyze/tests/it/statistical_witness_replay.rs; ticket QSL-992"
review_set: subset
date: '2026-10-24'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-194
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-193
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: reviews
---
# SR-16636: Gap analysis of PR #1434, FR-194 undefined window positions and EN-4 test gaps

## Summary

Ticket: QSL-992. Plan completion: not assessed. This is a planless audit using `quire matrix --scope .` at 2f6f9e7be.

All eight criteria the PR claims are tagged in the computed matrix: FR-188-AC-7, FR-189-AC-13, FR-189-AC-14, FR-193-AC-5, FR-194-AC-5, FR-194-AC-6, FR-196-AC-10 and FR-196-AC-11. Each tagged test passes. FR-196-AC-7 remains untagged; AC-11 cites it.

Every new production item belongs to FR-194 "Evaluation". The PR adds no stub and removes no test; two tests gained AC tags in place of a bare `TC-631`.

The reviewer compared each AC with its assertions clause by clause, so a semantic review was done as part of this pass.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-188-AC-7 | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md | examined |
| FR-189-AC-13 | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-AC-14 | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-193-AC-5 | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md | examined |
| FR-193-AC-1, FR-193-AC-3 | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md | context_only |
| FR-194-AC-5 | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md | examined |
| FR-194-AC-6 | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md | examined |
| FR-196-AC-10 | spec/functional/FR-196-build-the-probabilistic-product.md | examined |
| FR-196-AC-11 | spec/functional/FR-196-build-the-probabilistic-product.md | examined |
| FR-196-AC-7 | spec/functional/FR-196-build-the-probabilistic-product.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-193-AC-5's last clause says "`replay_sampled_witness` takes only the checked subject, the provenance and the witness". The test asserts nothing about this clause. The clause is also not literally true: the call takes `WitnessReplay{system, workload, provenance, cancel}`, the witness, and a reader that `witness_reader` builds from the checked package, the claim and the sampler. The coder's claim that the AC is "covered in full" is overstated. Either assert what the clause means (no S4-emitted artefact is needed) or align the AC with the real signature. | qsl-analyze/tests/it/statistical_witness_replay.rs:60-90, 472-509 |
| FND-002 | low | FR-196-AC-11's parenthetical "(its canonical bodies are the worked vector of AC-7)" is not asserted. AC-7 states no vector for an unactivated `steps` body and is itself untagged in the matrix, so this clause has neither a test nor a precise referent. The rest of AC-11 is asserted: decided-only-unactivated, outgoing-empty iff decided, the terminal-stutter chain counting to censored, and the fraction's source read. | qsl-analyze/tests/it/exact_probabilistic.rs:1256-1362 |
| FND-003 | low | FR-194-AC-5 specifies the filter and predicate `6 / e.x > 1`; the test uses `6 / e.x > e.one`, with a field fixed at 1. The reviewer reproduced the cause: with the literal `1`, the kernel `check_expression` refuses `IllTyped(AmbiguousLiteral)`. The two are semantically equal over the vector, so the AC is realised, but not in its own spelling. A surface-checker realisation becomes possible once S3 admits aggregate terms. | qsl-eval/tests/it/window_aggregates.rs:898-911 |
| FND-004 | low | FR-194-AC-5 says the guarded `min_count 5` sum is "O-16 `undefined` and never `UndefinedEvaluation`". The test checks only `category() == Undefined`, while the clause result it matches is `TemporalOutcome::Undefined(UndefinedEvaluation{cause: InsufficientData, ..})`. That is the pre-existing representation and not FR-327's distinct `InsufficientData{where, atom, count, min_count}`. The test also leaves the clause-level `at`, `count` and `min_count` unchecked. | qsl-eval/tests/it/window_aggregates.rs:1052-1069; qsl-eval/src/temporal/outcome.rs:58-70 |

## Verdict

CONDITIONAL. Findings by AC:

- **FR-188-AC-7.** Complete: empty, `(1, 0)` and `(1, -1/2)` each refuse with digest count 0.
- **FR-189-AC-13.** Complete: N = 4, estimate 3/4 per test and overall, and `StrictBound` for `Below`.
- **FR-189-AC-14.** Complete: the regenerative two-test case with `u64::MAX` accepts, and Okamoto with two tests is refused `TraceIndexRange{u64::MAX, 2}` with zero sampler calls.
- **FR-193-AC-5.** Asserted except its final clause (FND-001): both packages are refused `not-yet-implemented` by S4, and the AC-1 witnesses (seed 7, `max_witnesses` 3) and the AC-3 `Health` fault witness replay `Reproduced`.
- **FR-194-AC-5.** Asserted through the production `ObservedWindowSource`, apart from FND-003 and FND-004:
  - the clause is `UndefinedEvaluation{at 2, division-by-zero}` in category violation;
  - the atom at position 3 is undefined;
  - the guarded sum is in [21/2, 11), true for `<= 20`;
  - the undefined filter and the undefined predicate are each detected;
  - with `min_count 5`, the unguarded sum is undefined (division by zero) and the guarded sum is `InsufficientData{3, 5}`;
  - over `past[2, 3]`, the sum is 9, the count 2 and the fraction 1.
- **FR-194-AC-6.** Complete, using a `Rows` stub that charges nothing, so the meter delta is visit units only:
  - 2 units at position 3 and visit order `[0, 1]`, for both the unlimited limit and limit 2;
  - the clause result is `UndefinedEvaluation` at position 1, with readings `[True, undefined]` and 3 window units;
  - a 10-of-20 filter charges 20 units.
- **FR-196-AC-10.** Complete: both bindings start from one initial state; equal model and monitor with differing keys and bindings; distinct labels.
- **FR-196-AC-11.** Asserted except the AC-7 parenthetical (FND-002).
- **Doc comments.** Every AC id in a new doc comment carries a matching `#[trace]` tag.

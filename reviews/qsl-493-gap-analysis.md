---
id: SR-12060
title: "QSL-493 PR #928 gap analysis: ModelSystem against FR-120 and TC-471 to TC-473"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0001dc310de17b9d57f2d534bd3dd38aa070ebcc; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, spec/test-cases/TC-471-model-successors-follow-operations-arguments-and-frames.md, spec/test-cases/TC-472-invariant-violations-are-recorded-and-undecided-expansions-stop.md, spec/test-cases/TC-473-model-effects-are-trace-data-and-ambient-reads-refuse.md, qsl-eval/tests/it/model_simulation.rs, qsl-eval/src/simulation/model.rs, qsl-semantics/src/model/state.rs"
review_set: subset
---

## Summary

Ticket: QSL-493. PR quire-spec-language#928 at head 0001dc310. This is a planless gap analysis of FR-120-AC-1 to FR-120-AC-13, through TC-471, TC-472 and TC-473, against the 17 tagged tests and the code. The QSpec references FR-181 and FR-013 are cited by id only. Plan completion was not assessed. Read-only: tests were read, not run. As briefed, only major gaps are reported.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-120-AC-7 requires that "a universe population absent from the snapshot returns `Incomplete` with `incomplete_population`/`incomplete-scope`". No test asserts it. The doc comment on `an_unbounded_population_or_parameter_requires_a_bound` claims the clause, but its body tests only `RequiresBound`. The two AC-7 tests also check that `RequiresBound.domains` is non-empty or has one entry, not that it names the root's `WireNodeId`, which the AC states. The `Incomplete` branch at model.rs:361-368 has no test, and this gap is not among the author-recorded ones. | qsl-eval/tests/it/model_simulation.rs:987-1031, qsl-eval/src/simulation/model.rs:361-368 |
| FND-002 | medium | The PR adds nine public items to the `qsl_semantics::model::state` seam: `populations`, `type_identity`, `type_key`, `is_abstract`, `fields`, `object_reference`, `field_is_granted`, `StateField` and `StatePopulation`. FR-120 "Frame and observation seam" states the seam as `new`, `admit_initial` and `observation` only, and says the frame decision runs "with no `StateModel` method between". The ticket's Touches names qsl-eval and QSV only. The implemented public seam is not the one the spec states. | qsl-semantics/src/model/state.rs:50-246, spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:223-253 |

## Verdict

PASS on coverage of the main path. There are no high findings. FND-001 is an untested AC clause. FND-002 is a mismatch between the spec and the code at the public seam. The spec needs to state the accessors, or the accessors need to move behind qsl-eval. That decision is for the planner.

Covered with exact oracles:

- **AC-1:** the transition JCS bytes, three successors, kept fields, and the `Inc` and `CanInc` selection.
- **AC-2:** per-`n` identities and per-receiver identities, and A/B conjoined.
- **AC-3:** 1 + 18 spawn successors; the remove deletion sets {}, {c1}, {c1, c2}; the dangling {c2} excluded.
- **AC-4:** the full envelope compared byte for byte after canonicalization, and the label change changing the key.
- **AC-5:** stats (3, 2, 2) and (3, 5, 2); the exact `InvariantViolated` with handler anchor; the initialization anchor.
- **AC-6:** the undetermined `Incomplete` invariant, `Stopped` with frontier length 1 and category incomplete, and the `max_candidates` 2 stop.
- **AC-7:** the five `new` refusals and `RequiresBound` for no universe and for an `Integer` parameter.
- **AC-8:** the sampled trace's length, stop reason, finding depth, `FindingMismatch` at step 2, `ReplayError::Stopped`, and equal runs.
- **AC-9 (`pre` half):** the exact `ContractUndetermined` evaluation, and `Never` hiding it.
- **AC-10:** the boolean result text and the root in `domains()`.
- **AC-11:** the exact `FieldChange` texts, created [c2], deleted [c1], and the key unchanged across explores.
- **AC-12:** the four S3 codes and causes.
- **AC-13:** `Stopped` over the product of more than 10^9.

The AC-13 "at most 11 candidates" bound and AC-12's "at the source span" are not observable through the API, or not asserted. Both are low and are not counted.

The author-recorded gaps are planner questions and are not counted as defects here:

- AC-9 runs on the test/orders fixture, not Tally/touch, and its `post LowPost` half is untested.
- AC-11 has no child process in an empty directory.
- The clause spellings changed.
- A `?` field is keyed `{type: option}`.
- Unenumerable roots are listed as unbounded.
- Some types refuse `KeyEncoding`.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | c4ec15148 |
| FND-002 | fixed | c4ec15148 |

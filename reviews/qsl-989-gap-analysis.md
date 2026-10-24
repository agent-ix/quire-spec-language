---
id: SR-16626
title: "Gap analysis of PR #1430: FR-183 Projected (HP-6) replay"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1ee181562530b8029b14bc6296071d9b670ee9c6; spec/functional/FR-183-replay-a-hyper-counterexample.md, qsl-replay/src/model_check/hyper_replay.rs, qsl-analyze/tests/it/hyper_replay.rs, qsl-analyze/tests/it/hyper_projected.rs; ticket QSL-989"
review_set: subset
date: '2026-10-24'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: reviews
---
# SR-16626: Gap analysis of PR #1430, FR-183 Projected replay

## Summary

Ticket: QSL-989. Plan completion: not assessed. This is a planless audit using `quire matrix --scope .` at 1ee181562.

FR-183 rows:

- Tagged: AC-1, AC-3 (the new aggregate `every_listed_refusal_settles_no_result`), AC-4 and AC-6.
- Untagged: AC-2, AC-5 and AC-7 to AC-12.

The code-to-requirement direction is clean. Every new production function (`projected`, `visible_steps`, `ProjectedLetters`, `about_clause`, `universal_reading`) belongs to FR-183 "Projected (HP-6)" or "Common". There is no stub, no unreachable placeholder and no dead fallback, and the old Projected refusal path is deleted, as the ticket requires.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-183-AC-2 | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| FR-183-AC-3 | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| FR-183-AC-7 | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| FR-183 "Projected (HP-6)" | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| ADR-023 PA-7 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-183-AC-7 has no tagged test, although `an_undefined_counterexample_of_an_align_skip_clause_is_refused_by_replay` now asserts exactly its case. That is FR-178-AC-9's mid-operation `Undefined` counterexample, refused `HyperProjected` (`unsupported_construct`/`not-yet-implemented`) with no result. The test lives in the TC-603 file, while the AC names TC-608. The ticket says AC-7 "stays as is", so deferring the tag is acceptable, but the matrix row stays untagged. | qsl-analyze/tests/it/hyper_projected.rs:416-434 |
| FND-002 | low | Pre-existing spec inconsistency, not introduced by this PR. ADR-023 PA-7 settles a projected replay that is not `false` as "inconclusive, `ReplayParity`", while FR-183 "Projected (HP-6)" says "inconclusive, `Verdicts`". The code follows FR-183. A spec owner needs to align PA-7 with FR-183, or the reverse. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:494; spec/functional/FR-183-replay-a-hyper-counterexample.md |

## Verdict

The trace decisions are correct apart from FND-001.

- **AC-3.** The tag holds. Through the three called tests, the aggregate asserts every refusal AC-3 lists: one trace one step shorter, `μ_U` false, post-state digest altered, Projected with a `step` marked skipped, and the unfair loop. Each settles no result. The literal "first input" case and which rule refuses the Projected case are covered in SR-16625 (FND-001, FND-005).
- **AC-2.** Correctly untagged. Its `StepTuple` half has no arm. The Projected half is tested by `a_projected_counterexample_replays_to_its_evaluated_witness`, tagged TC-608 only.
- **Doc comments.** The module doc names FR-183-AC-2 and AC-5 only as "not claimed", and `quire matrix` does not count that text as a binder. No doc comment presents an untagged AC as covered.

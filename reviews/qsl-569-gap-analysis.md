---
id: SR-12210
title: "Gap analysis of QSL-569 E17b part 1 FR-188 weighted choice against FR-188-AC-1, AC-2 and FR-101 (PR #1060)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5463c8a4734107683f5bf956d937b217515bcd24; spec/functional/FR-188-*.md, spec/functional/FR-101-*.md, spec/test-cases/TC-623-*.md, spec/test-cases/TC-454-*.md; qsl-eval/src/simulation/choice.rs, qsl-eval/src/simulation/sample.rs, qsl-eval/tests/it/weighted_sampler.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
---
# SR-12210: Gap analysis of QSL-569 E17b part 1 FR-188 weighted choice against FR-188-AC-1, AC-2 and FR-101 (PR #1060)

## Summary

Ticket: QSL-569 (E17b), part 1. PR quire-spec-language#1060 at head 5463c8a47. The audit is scoped to the diff and to FR-188-AC-1 and AC-2, QSpec TC-354 vectors SM-01 to SM-03, and FR-101's uniform sampler, which now goes through the weighted choice. Plan completion: not assessed. The QSpec definition (`quire.simulation.sampler/v1`) and QSpec TC-354 were read from a local read-only QSpec checkout and are cited by id only.

Criterion coverage:

- **FR-188-AC-1, vectors (examined).** SM-01: five unit weights at choice 0, steps 0 to 4, select `1, 3, 3, 4, 4`, and `Weights::uniform(5)` equals the scaled unit weights. SM-02: choice 1, weights `(1, 3)`, select `1, 0, 0, 1, 1`. SM-03: `(1/2, 3/4)` scales to `(2, 3)` with `n = 5`. QSpec states only the scaling for SM-03, so no selection is expected. The other TC-210 uniform vectors (trace 1 `n = 5`, `n = 3`, the step with `n = 1`) run through `PinnedSampler`, which now calls `choose` at choice 0, so they are also unit-weight runs at choice 0. Covered.
- **FR-188-AC-1, GeneratorMismatch clause (examined).** Not covered in this PR. `weighted_choice` takes no `DefinitionRef`, and neither does FR-188's `sample_probabilistic`. The PR body declares this. FR-101-AC-10's existing `sample_request` test still covers the refusal for the uniform sampler.
- **FR-188-AC-2 (examined).** `(1/3, 1/6, 1/2)` scales to `(2, 1, 3)` with `N = 6`. At 60,000 consecutive trace indices, step 0, choice 0, every selection equals an independent reference computation and every index is selected. Choices 0 and 1 differ among the first 100 points, and choice 1 also matches the reference. Covered and tagged `FR-188-AC-2`.
- **FR-188-AC-3 (context only).** Out of scope for part 1: `sample_probabilistic` is not implemented. The PR's Remainder section records this.
- **FR-101-AC-3 (examined).** It still holds through the new path: every vector in `sample.rs` and `finite_simulation.rs` passes. The two deleted `U256` tests tested deleted code, and `choice.rs`'s own bound tests replace them.

There are no stubs, no coverage inflation and no untested production branch, apart from the unreachable `n = 0` fallback (SR-12209 FND-002).

No major finding. FND-001 and FND-002 are medium. Neither is a code defect in this diff: FND-001 is about ticket state, and FND-002 is a spec conflict that part 2 will hit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | PR #1060 is part 1 of QSL-569, but the ticket's GitHub attachment has `linkKind: closes`. Merging it would move QSL-569 to Done even though FR-187 (AC-1 to AC-4), FR-188-AC-3 and FR-188-AC-1's GeneratorMismatch clause are not implemented. The ticket's acceptance requires tests backing FR-187 and FR-188. Fix: the lead keeps QSL-569 open after merge, or files the declared remainder under its own ticket, before or at merge. | QSL-569 attachment (PR #1060); PR body "Remainder (blocked)" |
| FND-002 | medium | FR-188 Behavior fixes choice index 0 for the operation, 1 for the scheduled identity and 2 onward for random parameters. The QSpec generator definition ("Choices within a step") and QSpec TC-354 SM-05 and SM-07 say a one-item choice takes no choice index, and the next choice that draws takes that index. With one enabled operation, which is `Service`'s case, the two rules give different preimages, so FR-188-AC-3 cannot satisfy both. QSpec owns the definition, so FR-188 must adopt its rule before part 2 is written. The PR body declares this as spec gap 1. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:79-83 |
| FND-003 | low | FR-188 Outputs gives `weighted_choice(at: ChoicePoint, weights: &[Rational]) -> usize` and `ChoicePoint { seed: Seed, … }`. The code takes a validated `&Weights` and `seed: u64`. The departure is reasonable, because a raw slice would need a `Result` or a panic, and the PR body declares it. FR-188's Outputs are not updated, so spec and code disagree. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:46-48; qsl-eval/src/simulation/choice.rs:22-33,221-223 |
| FND-004 | low | Neither FR-188 nor the QSpec generator definition says what happens when the scaled total `n` is above `2^256`. In that case no draw is ever accepted. The code refuses it with `WeightsRefusal::TotalAboveDrawRange`, which is a sound meaning, but it is not in either spec. FR-188 should state the refusal (and the empty and non-positive refusals), so a second implementation does not loop forever or choose differently. The PR body declares this as spec gap 2. | qsl-eval/src/simulation/choice.rs:71-75,143-148; spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:66-71 |
| FND-005 | low | FR-188-AC-1 has no test tagged `FR-188-AC-1`. The SM-01 to SM-03 tests carry only `TC-623`, because the GeneratorMismatch clause is uncovered, so the Test Matrix reports AC-1 as untested even though its vector clauses are fully tested. The cause is the compound AC (vectors plus a refusal belonging to `sample_probabilistic`). Split FR-188-AC-1 into a vector AC and a refusal AC, so the vector tests can be tagged now. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:100; qsl-eval/tests/it/weighted_sampler.rs:79,93 |

## Verdict

**PASS: mergeable. No major finding.** FR-188-AC-2 is fully covered. FR-188-AC-1's vector clauses (QSpec TC-354 SM-01 to SM-03 and every TC-210 uniform vector at choice 0) are covered. FR-101's uniform sampler goes through the weighted choice with its vectors unchanged. FR-188-AC-1's GeneratorMismatch clause, FR-188-AC-3 and FR-187 are the declared remainder. Before merge, the lead must keep QSL-569 open for that remainder (FND-001). The FR-188 choice-index conflict with QSpec (FND-002) must be resolved in the spec before part 2.

## New findings (disposition pass 1)

FR-188 was revised on main by STD-222 (#1061, c3ed9a2f5) and STD-223 (#1063, 2d78cc488). This round checks the code at 54a09ad4c against the merged text.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The merged FR-188 Outputs give `weighted_choice(at: ChoicePoint, weights: &[Rational]) -> Result<usize, TotalAboveDrawRange>`, where `TotalAboveDrawRange { total: Integer }` is a struct that carries the scaled total. FR-188-AC-5 expects `TotalAboveDrawRange{total: 2^256 + 1}`. The code has `weighted_choice(at, &Weights) -> usize`, with the refusal on `Weights::new` as the unit variant `WeightsRefusal::TotalAboveDrawRange`, which has no `total`. It also adds `Empty` and `NotPositive` refusals that the spec does not name. The behaviour agrees: refusal happens before any digest, and `N = 2^256` accepts every draw. The public interface and the error payload contradict the merged spec. Not blocking for part 1, because FR-188-AC-5 is not in this PR's scope. Part 2 must either adopt the spec's signature and payload or amend FR-188. | qsl-eval/src/simulation/choice.rs:61-76,119,267-269; spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md (origin/main, Outputs and FR-188-AC-5) |

The other merged rules do not contradict the code. A one-item choice selects index 0 with no digest and takes no choice index: `choose` returns `(0, 0)` and computes no digest, and assigning choice indices belongs to `sample_probabilistic`, which is not built. `DefinitionRef` and `SampleStop::GeneratorMismatch` are parameters and results of `sample_probabilistic`, also not built. FR-101's sampler without a workload makes one choice per step at choice index 0, which is consistent with the rule.

## Dispositions

Round 1, reviewed at 54a09ad4c9258e2a6ce3d37a22014aaf8309bebb.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | The QSL-569 attachment for PR #1060 still reads `linkKind: closes`, and the ticket is In Progress. This is a lead action at merge, not a code change: keep QSL-569 open, or reopen it, for FR-187, FR-188-AC-1's GeneratorMismatch clause and FR-188-AC-3 to AC-6. |
| FND-002 | fixed | c3ed9a2f5 (STD-222, on main): FR-188 now adopts the generator definition's one-item, no-choice-index rule. |
| FND-003 | deferred | Superseded by FND-006. STD-222 kept `&[Rational]` and added `Result<usize, TotalAboveDrawRange>`, so the code, not the spec, must adapt. Owned by QSL-569 part 2. |
| FND-004 | fixed | c3ed9a2f5 / 2d78cc488 (STD-222, STD-223, on main): FR-188 now states the `TotalAboveDrawRange` refusal before any digest, citing QSpec FR-181-AC-14. |
| FND-005 | accepted-no-change | STD-222 kept FR-188-AC-1 as a single AC (vectors plus `sample_probabilistic`'s GeneratorMismatch refusal). The `FR-188-AC-1` tag lands with part 2's GeneratorMismatch test. |

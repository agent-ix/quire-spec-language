---
id: SR-13342
title: "base review of STD-222 FR-188 sampler gaps (PR #1061)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4f66d6d1e7ecc32ac0a376c3f76fc93dacd37706; spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md, spec/test-cases/TC-623-weighted-sampler-draws.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-623
    type: reviews
---
# SR-13342: base review of STD-222 (PR #1061)

## Summary

Ticket: STD-222. Base checklist over `git diff refs/remotes/origin/main...4f66d6d1e`
(FR-188 Description, Inputs, Outputs, Behavior, AC-1, AC-4, AC-5, References;
TC-623 scope, fixtures, steps and expected results). The diff was compared with
merged QSpec `refs/remotes/origin/main` (96e8fd777):
`proposals/quire-v1/definitions/simulation-sampler.md` ("Choices within a step",
"Uniform and weighted selection") and TC-354 SM-01, SM-02, SM-05, SM-07. It was
also compared with the QSL crate graph (`qsl-eval`, `qsl-replay`, ADR-011 FB-05,
ADR-024 "Amendments" and "References") and with the downstream consumers
FR-189, FR-190 and FR-193.

Checks made by hand:

- Choice index: the new rule (a one-item choice draws nothing and takes no
  index; the next drawing choice takes the index it would have taken) matches
  the QSpec sampler definition word for word, and so does the worked ordering
  sentence.
- AC-4 vector: SHA-256 of the JCS preimage
  `{"choice":"0","draw":"0","seed":"424242","step":"0","trace":"0"}` is
  `d5160380…205995f2`, the TC-210/SM-01 digest. `v mod 5 = 1` (SM-01 index 1)
  and `v mod 4 = 2`, so weights `(1, 3)` select item 1. The SM-02 sequence at
  choice 1 recomputes to `1, 0, 0, 2, 3`. AC-4 is correct. It does not state
  the selected item.
- 2^256: QSpec accepts `v < n·⌊2^256/n⌋`. At `n = 2^256` every `v` is
  accepted, so AC-5's "selects" is right. At `n = 2^256 + 1` the bound is 0,
  no draw is ever accepted and the QSpec rule never terminates, so a refusal
  is needed.
- `DefinitionRef` exists in `qsl_foundation::selection`, and
  `NotSimulated::GeneratorMismatch { supplied }` exists in
  `qsl-eval/src/simulation/not_simulated.rs`. With the new `sampler` parameter,
  AC-1's mismatch clause can be tested.
- Crate home: ADR-024 places the sampler in layer 5 `qsl-eval`. `ExpansionStop`,
  `NotSimulated` and `ModelSystem` are defined in `qsl-eval`. ADR-011 FB-05
  limits SV's dependencies to K, `quire-canonical`, `serde` and `thiserror`, so
  `SampleStop` cannot live in SV, and `qsl-replay` (FR-193's `SampledWitness`)
  already depends on `qsl-eval`. The decision holds. Linear QSL-569 ("SampledStep
  in SV", "Touches: quire-semantic-value") now disagrees with it. That is a
  tracker correction for the lead, not a spec defect.

The method `code-review` did not run: the diff contains no code.
`make check-spec-validation` exits 0 with no diagnostic on FR-188 or TC-623.
`tools/check-index-completeness.sh` exits 0. quoin 0.28.3, quire 0.36.2
(engine 0.50.2).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-188 adds two `SampleStop` variants (`NotSimulated`, `TotalAboveDrawRange`), but FR-189 "Decision and stops" still maps only `NotMarkov`, `ExpansionStop` and budgets, and FR-190 has no mapping either. No statistical outcome is specified for a sample stopped by a total above 2^256 or by a sampler mismatch. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:148-150; spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:59-65 |
| FND-002 | medium | `weighted_choice` now returns `Result<usize, TotalAboveDrawRange>`, but FR-193 replay ("recompute each choice with FR-188's `weighted_choice`") gives no replay outcome for the `Err` case. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:92-94 |
| FND-003 | medium | The refusal for a total above 2^256 is a QSL-only rule on top of a generator that QSpec owns. QSpec's acceptance rule never terminates there, so QSpec is the defective side. The References note this, but no QSpec ticket raises it, so RT and CG implementations of `quire.simulation.sampler/v1` can disagree. STD-222 asked for the rule to be raised to QSpec if QSpec is wrong. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:148-152 |
| FND-004 | medium | `TotalAboveDrawRange { pub total: Natural }` names a type that exists nowhere: `quire-exact` exports `Integer` and `Rational` and no `Natural`, and no other spec text defines it. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:51 |
| FND-005 | medium | TC-623 names no fixture model for the sample-level clauses of AC-4 (one operation, one identity, and one parameter of weights `(1, 3)`) or AC-5 (a step whose weights scale above 2^256). Step 3 "stops a sample" has nothing to run. | spec/test-cases/TC-623-weighted-sampler-draws.md:19,26,32 |
| FND-006 | low | The range wording is wrong at the 2^256 boundary that AC-5 tests. "rejecting draws outside the largest multiple of `N` below the generator's range", read strictly, rejects everything at `N = 2^256`, and 0 is a multiple of `N` for "no multiple of `N` inside the range". State QSpec's `v < N·⌊2^256/N⌋`. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:75-79 |
| FND-007 | low | "record the step with the indices of the choices that drew" makes `SampledStep.choices` always `[0, …, k-1]`, which records only how many choices drew. FR-193 replay checks the recorded selections, so the field should say whether it holds choice indices or selected item indices. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:110-112 |
| FND-008 | low | The crate-home bullet reads as a build log ("no `quire-semantic-value` change is part of this requirement"). Its rationale gives the `NotMarkov`/`ExpansionStop` reason for all three types, although only `SampleStop` names them, and no AC verifies the SHALL. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:113-119 |
| FND-009 | low | `SampleStop::NotSimulated(NotSimulated)` also admits `RequiresBound`, `Extent`, `EmptyInitial` and `KeyEncoding`, but the FR states only `GeneratorMismatch`. Narrow the variant, or state which others `sample_probabilistic` can return. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:63 |
| FND-010 | low | References credit QSpec FR-181 and TC-210 with the one-item rule and its vectors. The rule is in the sampler definition, and AC-4's vectors are TC-354 SM-05/SM-07, which References does not cite. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:148-152 |
| FND-011 | low | "with no draw" cannot be observed at `weighted_choice`'s pure return value: over one weight every draw selects 0, and the refusal is an `Err` either way. As written, that clause of AC-4 and AC-5 cannot fail. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:136-137 |

## Verdict

Not mergeable as is: five medium findings (FND-001 to FND-005). The four
decisions are sound on their own terms. The choice-index rule matches QSpec
and the TC-354 vectors, the AC-4 vector recomputes exactly, the 2^256 boundary
in AC-5 is correct, AC-1 can now be tested through the `sampler` parameter, and
`qsl-eval` is the right home. The gaps are in what the change leaves
downstream (FR-189, FR-190, FR-193), in QSpec ownership of the new refusal, in
one undefined type and in TC-623 fixtures. No compatibility layer was added.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | medium | AC-4 gives `Coin` "weights `(1, 3)`", but the fixture declares `{ true: 1, false: 3 }`, and in canonical value order (QSpec FR-144: `false` before `true`) that vector is `(3, 1)`. The draw `v mod 4 = 2` selects index 0 (`false`) under `(3, 1)` and index 1 under `(1, 3)`, so "`i` the index the generator selects" passes either ordering. Pin the literal `choices = [0]` and `random = [false]`, or reorder `Coin` to `{ false: 1, true: 3 }` and pin `choices = [1]` and `random = [true]`. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:140; spec/test-cases/TC-623-weighted-sampler-draws.md:19 |
| FND-013 | low | The `Wide` fixture writes the weight `false: 2^256`. In the QSL grammar, `^` occurs only in a `DimensionTerm` exponent (`qsl-cst/src/grammar.rs:458-462`), so this is not a valid weight literal. Spell the decimal value. | spec/test-cases/TC-623-weighted-sampler-draws.md:19 |
| FND-014 | low | AC-1, AC-4 and AC-5 observe "zero digests computed" through a counting generator wrapper, but `weighted_choice(at, weights)` and `sample_probabilistic` take no generator, so the Outputs give the wrapper nothing to attach to. State the test seam (for example a test-support digest counter), or observe the outcome another way. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:52-57,137-141 |
| FND-015 | low | The fix left a broken sentence: "each by the rule above: so with several operations the operation is choice 0 …". | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:109 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-189 maps `TotalAboveDrawRange` to `Unsupported(TotalAboveDrawRange{total})` and `GeneratorMismatch` to `StatisticalRefusal`. FR-190 maps every sampler stop as FR-189 does, and FR-192's `UnsupportedCause` gains the cause. |
| FND-002 | fixed | FR-193 replay refuses a `TotalAboveDrawRange` from `weighted_choice` with `invalid_runtime_input`/`invalid-value`. |
| FND-003 | fixed | QSpec ticket STD-223 is filed, and FR-188 states the refusal as QSL's own behaviour. FR-188 does not cite STD-223 by id. |
| FND-004 | fixed | The field is now `total: Integer` (quire-exact). |
| FND-005 | fixed | TC-623 adds the `Lone`, `Coin` and `Wide` fixtures and the counting wrapper; their remaining defects are FND-012 to FND-014. |
| FND-006 | fixed | Acceptance is now stated as `v < N · ⌊2^256 / N⌋`, so `N = 2^256` accepts every draw. |
| FND-007 | fixed | `choices` holds the selected item index of each choice that drew, in choice-index order. |
| FND-008 | fixed | The build-log clause is gone, the rationale covers `SampleStop` only, and AC-6 verifies the home. |
| FND-009 | fixed | The variant is now `SampleStop::GeneratorMismatch { supplied: DefinitionRef }`. |
| FND-010 | fixed | References cite the sampler definition and TC-354 SM-05/SM-07. |
| FND-011 | fixed | "Zero digests computed" through the counting wrapper is observable; the wrapper's seam is FND-014. |

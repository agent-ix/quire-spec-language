---
id: SR-13348
title: "base review of STD-223 FR-188, FR-189 and TC-623 total refusal citation (PR #1063)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0f16ce205ed71006cb003e607e5b0459b390c4cc; spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md, spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md, spec/test-cases/TC-623-weighted-sampler-draws.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-623
    type: reviews
---
# SR-13348: base review of STD-223 (PR #1063)

## Summary

Ticket: STD-223. Base checklist over `git diff refs/remotes/origin/main...0f16ce205`
(FR-188 Behavior bullets on the total refusal, FR-188-AC-5, FR-188
Dependencies; FR-189 "Decision and stops"; TC-623 Step 3 expected result).
The diff was compared with merged QSpec `refs/remotes/origin/main` (9542172,
PR #289): FR-181-AC-14 and AC-15, FR-331's `unsupported` cause list, FR-408's
unsupported list, TC-354 SM-08 and SM-09, and
`proposals/quire-v1/definitions/simulation-sampler.md` ("Uniform and weighted
selection"). Downstream QSL consumers FR-190, FR-192 and FR-193 were read for
consistency.

Checks made by hand:

- Spelling: the wire cause `total-above-draw-range{total}`, the `unsupported`
  family and `total` as a decimal string match QSpec FR-181-AC-14, FR-331,
  FR-408 and the sampler definition exactly. The SM-09 decimal
  `115792089237316195423570985008687907853269984665640564039457584007913129639937`
  equals `2^256 + 1` (recomputed).
- Stale text: no sentence anywhere under `spec/` (FR-187 to FR-193, every
  ADR, every TC) still says QSpec states no rule for the case or that the
  refusal is QSL's own. FR-190, FR-192 and FR-193 use the QSL type name only
  and agree with the new text.
- Stop scope: not a contradiction (see Verdict).
- `make check-spec-validation` exits 0 with no warning on the three touched
  files; `tools/check-index-completeness.sh` passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-188-AC-5 and TC-623 Step 3 now expect the wire cause (`total-above-draw-range`, `total` a decimal string) from `weighted_choice` and `sample_probabilistic`. Those are `qsl-eval` functions that return the Rust values `TotalAboveDrawRange{total}` and `SampleStop::TotalAboveDrawRange`; the wire form only exists after EN-4 maps the stop (FR-189) and the `qsl-replay` settlement map writes it (FR-192). A TC-623 test has nothing wire-shaped to assert. Keep FR-188-AC-5 and TC-623 on the Rust values (the `total` equal to `2^256 + 1`), and move the wire expectation to the settlement layer. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:143; spec/test-cases/TC-623-weighted-sampler-draws.md:32; spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:90-93 |
| FND-002 | medium | The obligation this PR exists for (EN-4 settles a total above 2^256 as `unsupported` with wire cause `total-above-draw-range{total}`) has no acceptance criterion where it is produced. No FR-189 or FR-190 criterion exercises `SampleStop::TotalAboveDrawRange`, and FR-192-AC-1 (TC-627) checks only `Unsupported(NotMarkov)` among the unsupported rows. Add a criterion, for example to FR-192-AC-1 or a new FR-189 AC over `Wide`, that the run settles `unsupported` with cause `total-above-draw-range` and `total` `"1157…9937"`. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:151-154; spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:124 |
| FND-003 | low | Citations are loose. TC-623 Step 3 cites "QSpec TC-354 SM-08, SM-09" for the wire form, but SM-08 is the in-range `2^256` case and carries no wire cause. FR-188-AC-5 cites SM-09 for weights `(1, 2^256)`, while SM-09's weights are `(2^256, 1)`; the total agrees but the vector does not. Cite SM-09 alone for the cause, and say the total matches SM-09 rather than citing it as the same vector. | spec/test-cases/TC-623-weighted-sampler-draws.md:32; spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:143 |

## Verdict

Not mergeable as is: two medium findings, one low. The QSpec alignment
itself is right. Every name and payload matches QSpec, and no stale "QSpec has
no rule" or "the refusal is QSL's" text remains. The problem is where the
wire spelling lands: in the sampler layer and its test, which cannot observe
it, while the settlement layer that writes it gets no criterion.

Stop scope, the author's flagged mismatch, is consistent, not a
contradiction. QSpec's sampler definition says the run "stops at the first
such choice, before any draw and with no digest or choice index taken", and
that "the stop applies to the whole sampling run, not only to the current
trace". "Before any draw" means before any draw of that choice. FR-408 says
the same thing ("at the first such choice"). FR-181-AC-14's shorter wording
("stops the whole sampling run before any draw") can be misread in
isolation, but the definition settles it. That is a QSpec wording point,
outside this PR. QSL gets the same result in two layers.
`sample_probabilistic` (FR-188) stops the behaviour at the offending choice
with no digest computed. FR-189 then returns `Unsupported` for the whole
decision, and FR-190 maps the stop the same way, so no later trace or test is
sampled. The QSL cause carries `total` only and no position, as QSpec
requires. No spec fix is needed. Optionally, FR-189 could say "the run SHALL
stop", which would make whole-run scope explicit.

Examined and clean: the FR-188 refusal bullet's rationale (behaviour, not
changelog), the FR-188 Dependencies text (states what is read from QSpec, not
a history), FR-189's mapping to `Unsupported(TotalAboveDrawRange{total})`, and
the agreement with FR-190, FR-192's `UnsupportedCause` and FR-193's replay
refusal.

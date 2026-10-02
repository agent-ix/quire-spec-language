---
id: SR-1125
title: "QSL-365 Wave C spec review of ADR-019 and FR-129 to FR-134 (#563)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ca75a721; git diff origin/spec/366-temporal-properties...HEAD over spec/: ADR-019; ADR-014 A-4 and ADR-018 FA-2, FA-4, FA-5, §4, V-8, CX-3, QS-4 amendments; FR-127; FR-129 to FR-134; US-016, US-017; TC-522, TC-530 to TC-535; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-019
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-131
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-132
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-133
    type: reviews
---

## Summary

Ticket: QSL-365. PR quire-spec-language#563, stacked on #562. One review
file for the PR, covering spec-review base, EARS, integrity and
scope-boundary checks over the diff against
`origin/spec/366-temporal-properties` only.

Checked against the QSL-365 rulings:
- unmarked fairness is weak whole (SY-2, SY-3, FR-129);
- the strong hint is added, and widened to any refuted liveness claim whose loop strong fairness would exclude (SV-6, FR-133);
- no enabledness on supplied traces (SV-2, FR-132);
- an unfair EN-1 lasso settles `ReplayParity` (SV-4, CX-3 amendment, FR-127, FR-131);
- the registry filters backends by advertised fairness kinds (DS-2, FR-134).

Every ruling is reflected.

`quire validate` on every changed spec file passes; the index check passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-127 and FR-131 split replay refusals into "refused as an unfair lasso" (`ReplayParity`) and every other refusal (`ReplayRefused`). But FR-131 refuses an unfair lasso with `invalid_runtime_input`/`invalid-value`, the same code FR-128 uses for a loop that does not close, a disabled step, a bad `initial` index and a misplaced stutter marker. No typed refusal variant identifies the unfair-lasso case, so the settlement cannot tell which row applies. | spec/functional/FR-131-replay-checks-strong-fairness-on-a-model-counterexample.md:66-75; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:97-98 |
| FND-002 | low | ADR-019 SV-2 names "the first constraint of its fairness set in canonical order". FR-132 names "the first constraint of the checked fairness set", which FR-123 orders by source order of first occurrence. The two orders differ when constraints are written out of canonical order. | spec/decisions/ADR-019-strong-fairness.md:147; spec/functional/FR-132-settle-fairness-over-a-supplied-trace-as-a-missing-premise.md:59 |
| FND-003 | low | ADR-019 SV-6 says replay computes the hint "so every engine's refutation gets it". FR-133 has only `model_check` write it on the terminal record. An EN-2 refutation settles through CG's map (ADR-018 DS-2), so no requirement puts the hint on that record. | spec/decisions/ADR-019-strong-fairness.md:151; spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md:74 |
| FND-004 | low | ADR-019 SR-5 says phase two's "time budget, meter and cancellation settle V-7", but FR-126's `ModelCheckLimits` has no time budget member, and FR-130 specifies only the cancellation poll for phase two. | spec/decisions/ADR-019-strong-fairness.md:118 |
| FND-005 | low | ADR-019's local item prefix `SR-` (SR-1 to SR-9, cited as "ADR-019 SR-2") reuses the repository's SpecReview id prefix, so a bare `SR-2` is ambiguous in text and in tooling that mines review ids. Also, §8 lists QS-8 before QS-7. | spec/decisions/ADR-019-strong-fairness.md:38,114,253-254 |

## Verdict

Mergeable once FND-001 is fixed, and once #562's high and medium findings
land in the base. The SR-2 refinement is the standard Emerson-Lei and
Latvala-Heljanko Streett check, and SR-3 and SR-4 argue it correctly. I
checked these by hand:
- §6's mutex product (5 states, one accepting SCC);
- its three verdicts and its SV-6 hint;
- FR-130-AC-2's `Handoff` refinement (stem `0 -acq(2)-> 2`, loop `2 -pass-> 3 -pass-> 2`);
- FR-133-AC-1 and FR-133-AC-2's hints.

EARS phrasing holds. Every AC has a behaviour TC, and the PR adds no pins,
caps or compat paths. FND-001 needs a typed unfair-lasso refusal so that
FR-127's two V-6 rows can be told apart. The rest are low.

## Dispositions

Round 1, reviewed at 1d248ff47b3f75cb334d1dd987939cb64f728b5d (stacked on 18a9c22b).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 1d248ff4 |
| FND-002 | fixed | 1d248ff4 |
| FND-003 | fixed | 1d248ff4 |
| FND-004 | fixed | 1d248ff4 |
| FND-005 | fixed | 1d248ff4 |

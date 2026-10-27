---
id: SR-1650002
title: "gap analysis of PR #1650 (QSL-583d, FR-242 timed claim automaton)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-27
scope: "agent-ix/quire-spec-language; PR #1650 git diff origin/qsl-583c-zone-certificates...934926e63; FR-242 (Behavior, AC-1), FR-239-AC-4, FR-237-AC-1 (Settles half), FR-235-AC-2 (Settles half); TC-697, TC-694, TC-692, TC-690"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-242
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-237
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: reviews
---
# SR-1650002: gap analysis of PR #1650 (QSL-583d, FR-242 timed claim automaton)

## Summary

Ticket: QSL-583. Head reviewed: 934926e63. Method: a manual check of each AC against its tests, the PR's claims re-measured in the code and the test run, and a mutation check of the agreement test. Plan completion: not assessed.

| AC | PR claim | Measured |
| --- | --- | --- |
| FR-242 Behavior (TT-3/TT-4) | covered for the one-clock subset | **Partial.** The subset is exact; my probes found no mismatches. But the Behavior's MightyL SHALL also covers nested non-punctual intervals that do not start at a closed 0, and those are refused (FND-002). The agreement test does not detect a broken `release` subsumption rule (FND-001), and it has no `on each` coverage (FND-004). |
| FR-242-AC-1 | covered | **Covered.** All four vectors × 1,000 lassos of length ≤ 8, with quarter-ms stamps. |
| FR-239-AC-4 | part | **Partial.** The budget, cancellation and repeatability halves are covered. The `RequiresBound` half has no test (FND-003). |
| FR-237-AC-1 (`Settles` half) | covered | **Covered.** `[0,3)` is refuted by `send` at 0 and `reply` after 3, and the counterexample reproduces. |
| FR-235-AC-2 (`Settles` half) | covered | **Covered.** `Proved{ZoneCertified, Certified}`, and `CertificateRejected` with a node removed. |

The four deferrals:
- **(a) Nested intervals not starting at a closed 0:** this is unbuilt work FR-242 requires, not a spec gap (FND-002). The one real spec gap is a punctual nested interval inside a TT-2 window. FR-242 says one horizon clock suffices there, which cannot be true for nested activations.
- **(b) FR-239-AC-4's `RequiresBound` half:** not forced (FND-003).
- **(c) Idle-tail stutter delays:** sound. The worst case is `inconclusive` (SR-1650001). It is tracked only in the PR body (FND-005).
- **(d) Byte equality:** structural equality, consistent with STD-493's open FR-418 encoding gap. No finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The agreement tests do not discriminate the `release` re-activation rule. I mutated `TimedClaim::branches` so a restarted `release` keeps the older clock instead of resetting to the newer one, which is a wrong automaton. Both `the_timed_claim_automaton_agrees_with_the_evaluator_on_random_lassos` and `the_mitl_vectors_agree_with_the_evaluator` still pass. A targeted vector catches it at once: `eventually (holds(p) and eventually[0 ms, 1 ms] holds(q))`, whose negation restarts `always[0,1]` at every `p`, with p at stamps 0 and 1 (quarters). The same vector with `p until[0,1] q` also catches it. The `until` mutation is caught, as the PR says. Add these vectors, under both activations, as a regression test of the subsumption rules. | qsl-eval/tests/it/timed_trace.rs:930-1080; qsl-eval/src/automaton/timed_claim.rs:1095-1110 |
| FND-002 | medium | Deferral (a) is unbuilt FR-242 work, not a spec gap. FR-242 says "a `TimedSafety` or `TimedLiveness` formula SHALL translate by the MightyL construction", and MightyL covers every non-punctual MITL interval, nested `(0,b]` and `[a,b]` included. Refusing them leaves part of FR-242's Behavior unmet. File it as its own ticket (583e) that names the SHALL and the refused shapes, instead of as a STD-493 spec gap. A nested punctual interval in a TT-2 window remains a genuine spec gap. | qsl-eval/src/automaton/timed_claim.rs:30-43,491-497; spec/functional/FR-242-translate-a-timed-formula-into-a-claim-automaton.md |
| FND-003 | medium | FR-239-AC-4's `RequiresBound` half ("a subject with an unbounded population root refuses `RequiresBound` with no state explored") has no test, and the fixture is not out of reach. The untimed suite builds an unbounded root already (`model_check_captures.rs`, FR-337-AC-12). EN-6 also names this refusal `ZoneCheckRefusal::Unbounded` rather than the AC's `RequiresBound`. Add a timed fixture with an unbounded root, and align the refusal with the AC or file the naming as a spec gap. | qsl-analyze/src/zone_check/search.rs:268-272,372; qsl-analyze/tests/it/zone_mitl.rs:69-75 |
| FND-004 | low | The agreement tests use `on origin` only (`timed()` is `timed_on("on origin", …)`). Under `on each` every operator is nested, and the root becomes `true until (when ∧ ¬body)`; no test of the PR exercises that. My probe found no mismatch (150 random formulas plus 15 targeted ones, oracle: some position is violated). Add it as a test. | qsl-eval/tests/it/timed_trace.rs:45-47,930-1080 |
| FND-005 | low | Deferral (c) is tracked only in the PR body: idle-tail stutter positions take the replay's canonical delays, so a violation that depends on stutter instants may replay as not reproduced. It is sound, but it needs a ticket or a QSL-583 scope note naming FR-236's stutter-delay lasso form, so the incompleteness is not lost. | qsl-analyze/src/zone_check/liveness.rs (stutter positions); spec/functional/FR-236-carry-exact-rational-delays-in-a-timed-counterexample.md |

## Verdict

**No major gaps, and no soundness issues.** Fix FND-001 (a cheap regression vector) and FND-003 in this PR. FND-002 needs a ticket for the unbuilt MightyL work rather than a spec-gap label. FND-004 and FND-005 are low.

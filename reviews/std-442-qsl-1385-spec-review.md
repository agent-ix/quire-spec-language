---
id: SR-16307
title: "spec-review of STD-442 hybrid routing, solver method table and proof coverage (quire-spec-language PR 1385)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7bb6fc37; spec/functional/FR-075-compute-candidates-from-registered-backends.md; spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md; spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md; spec/functional/FR-235-settle-a-timed-verdict-as-a-terminal-record.md; spec/functional/FR-289-keep-results-unchanged-by-provider-installation.md; spec/decisions/ADR-026-dense-time.md; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md; spec/test-cases/TC-193, TC-705, TC-1187, TC-773, TC-774"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: references
---

# Spec review of STD-442 hybrid routing, solver method table and proof coverage (quire-spec-language PR 1385)

## Summary

Ticket: STD-442 (G1 to G5). One spec-review pass over the diff against origin/main b46bd15d (an ancestor of head 7bb6fc37), read with QSpec #423 at 002fbf96.

Examined, clean:

- G1: the FR-075 candidate rule (match = kind, evidence kind, dynamics); `{discrete}` when unsupplied; `invalid-dynamics` refusal order and its placement after pairs, domains, fairness kinds and evidence kinds; `dynamics` ignored on a non-temporal descriptor; the duplicate-registration comparison. Also FR-447's request writer (`requirements` on hybrid items only, in identity), FR-250-AC-1 and FR-447-AC-17. These agree with QSpec FR-193 "Routing a hybrid item" and FR-290.
- G2: `SolverMethod` is the five-kind table, `SolverCoverage` is `Bounded | Inductive`, and the sound triples are exactly the three listed. The first-party refusal of `flowpipe-unrounded` and `unclassified` at `/basis/method` holds. FR-290 reads the four bounded spellings and `inductive-solver`.
- QSL FR-290's out-of-table method is now `Malformed`, where it was `Unsupported`. I judge this correct. QSpec FR-193 closes the table and the FR-331 schema refuses any other spelling, so a spelling outside it is a value outside FR-331's domain (QSpec FR-272 `invalid-value`), not an in-domain value that QSL's types cannot hold, which is the only case that is `Unsupported`. `cause` `form` stays `Unsupported`, which is right.
- G3, hand-derived from FR-447's reach rule: `SlowEnough` is 10 s, so `horizon` 10 and 21/2 cover and 1 gives `InsufficientHorizon{1, 3}`. `always [0, 10] eventually [0, 5]` is 15 s. `Recent` (`once [0, 2] holds(h.v = 20)`, model-steps) is `Positions(max(0, 0 - 0)) = Positions(0)`, so `jumps` 0 covers. `eventually [0, 1]` under model-steps is `Positions(1)`. `Slows` (`Each`) and `always` with no interval are `None`. The FR-250-AC-6/AC-7 and FR-290-AC-18 vectors follow from these.
- G4: ADR-026 RU-7, the rejected alternative and the §14 bullet now match HY-2, FR-250 and FR-450. HY-1 and HY-2 carry routing and coverage. ADR-018, ADR-029 PL-7, FR-235, FR-289 and TC-773 gain `InductiveSolver` and `InsufficientHorizon` consistently. No other QSL spec names `bounded-solver`.
- G5: "QSL has no built-in hybrid solver" (FR-250, FR-447, HY-3) resolves the contradiction in favour of merged ADR-026 RU-7/HY-3, as the ticket's G5 asks. No specified capability is removed, and the owner question is flagged in the PR body without narrowing, so it is not a scope deferral. G6 (the ticket text) is ticket work owned by terra, not spec scope.
- The new ids FR-075-AC-19/20, FR-250-AC-6/7, FR-447-AC-17 and FR-290-AC-18 appear on no other open QSL PR head and are unique in the tree.

Gates at 7bb6fc37: `make check-spec-validation` exits 0 both with QSPEC_DIR at the shared QSpec clone (main) and with QSPEC_DIR at #423 head. Its warnings are all pre-existing, none in touched files. QSL conformance `cargo test -p qsl-package --lib checked_v2` with QSPEC_DIR at #423 head: 49 pass.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-250's coverage bullet repeats QSpec FR-193's one-dimensional coverage rule (QSpec #423 SR-16305 FND-001). Under `ModelTime`, `horizon >= r` covers whatever `jumps` is. Under `ModelSteps`, `jumps >= n` covers whatever `horizon` is. So `Proved` is written for claims whose states lie past the proof's other bound, for example FR-250-AC-7's `jumps` 1, `horizon` 10 over `eventually [0, 1]` on Braking, whose first jump can occur at t = 20. That contradicts RU-7/HY-2 ("proved only where ... its proof covers the claim"). Fix it together with QSpec FR-193: either require both bounds, or state the residual bound as a qualification of `Proved` and pin the `jumps` 0 case in an AC. | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:133-139 | wrong-requirement |
| FND-002 | medium | FR-075 makes an admitted descriptor with no `dynamics` hold `{discrete}`, but it does not say what the FR-331 manifest QSL writes from that descriptor carries: the member absent, or `["discrete"]`. The two give different descriptor bytes, so different manifest digests and cache keys (FR-306) for EN-6 and every existing temporal request. Fix: state that the manifest keeps `dynamics` as supplied (absent stays absent) and that `{discrete}` is only the routing reading, and pin it in FR-075-AC-20. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:317-318 | wrong-requirement |
| FND-003 | low | The existing test `qsl-replay/src/proof_result/plugin_tests.rs:131-187` (`#[trace("TC-774", "FR-290-AC-6")]`) asserts `Unsupported{feature: "dreal"}` for a `bounded-solver` method `dreal`. FR-290-AC-6 now says `Malformed`, so the tagged test contradicts its AC until the code changes. QSL-900, or another named coding ticket, must list this change (the FR-290 reader for `unclassified`, `inductive-solver` and `insufficient-horizon` as well) so it is not lost. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:544 | missing-requirement |

## Verdict

Not mergeable. FND-001 (high) blocks the PR, together with its QSpec counterpart. FND-002 should be fixed in the same round, and FND-003 is a tracker action for the code lead. Routing, the method table, the ADR amendments and the hand-derived vectors are otherwise correct.

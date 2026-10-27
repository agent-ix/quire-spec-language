---
id: SR-1650001
title: "code-review and rust-review of PR #1650 (QSL-583d, FR-242 timed claim automaton)"
type: SpecReview
analysis: code-review
date: 2026-10-27
scope: "agent-ix/quire-spec-language; PR #1650 git diff origin/qsl-583c-zone-certificates...934926e63: qsl-eval/src/automaton/timed_claim.rs, qsl-eval/src/automaton/{mod.rs,error.rs,timed_window.rs}, qsl-analyze/src/zone_check/{search.rs,liveness.rs,concretise.rs,lasso.rs,dbm.rs,time_lock.rs}, qsl-replay/src/certificate/zone_certificate.rs, qsl-replay/src/certificate/zone_certificate/product.rs, qsl-eval/tests/it/timed_trace.rs, qsl-analyze/tests/it/{zone_mitl.rs,zone_certificate.rs,zone_search.rs,zone_liveness.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-242
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: reviews
---
# SR-1650001: code-review and rust-review of PR #1650 (QSL-583d, FR-242 timed claim automaton)

## Summary

Ticket: QSL-583. Head reviewed: 934926e63 (`qsl-583d-mightyl`), scoped to the diff against `origin/qsl-583c-zone-certificates` (#1647, which has its own review). The review was led by soundness.

**The one-clock construction is exact for the subset it admits.** I traced the tableau in `TimedClaim::branches`:
- A pending `until` is kept before or in its window, discharged in it, and dies after it.
- When an `until` is restarted, the older instance is kept, with no reset. With a closed-0 lower end, the older window ends first and its witness serves the newer instance, so older implies newer.
- When a `release` is restarted, the newer instance's clock is reset. Its window covers the older one's remaining positions, so newer implies older.
- In each case one clock is exact.
- The acceptance flag (not pending before, or discharged now) gives one Büchi set per unbounded `until`.
- A past tracker keeps the latest witness, which is the nearest, so it is the one a closed-0 window reads.

**The probes agree with the evaluator, with no mismatches.** I ran the automaton against `evaluate_timed_at` on random lassos with stamps in quarter-ms steps:
- 15 targeted formulas under `on origin`, 3,000 lassos each, length ≤ 10. They cover re-activation before expiry, open and closed ends, `[0,0]`, `until` and `release` nested under each other, `once`/`since`/`historically` under `always`, and a single-read `(1,2]` over a nested `[0,1]`.
- The same 15 under `on each`, 2,000 lassos each, with the oracle "some position is violated".
- 150 random `on each` formulas × 80 lassos.
- 300 random depth-4 formulas × 100 lassos of length ≤ 10 with denser stamps.

**The checker really rebuilds the automaton.** `position_automaton` is rebuilt from the request's clause. `read_at` and `read_before` place the automaton clocks after the clock places, consistently: the guard comes first, then the resets, and `pre` inverts them in reverse.
- The automaton's ends are checked against the scale, and its `L`/`U` are required.
- Tampers rejected:
  - `Settles` `[0,3]` moved onto `[0,2]` (EN-6 refutes it): `SuccessorMissing`.
  - Each `L`/`U` entry lowered, the automaton clock's included: `Malformed` at `bounds`.
  - FR-245's own strict-clause test.
- Tampers accepted, legitimately:
  - automaton numbers permuted;
  - `Settles` moved onto two clauses EN-6 also proves (`eventually[0,3] holds(c.phase = 2)`, and the `on each` form of `Settles`).

**Outside the subset, EN-6 refuses `UnsupportedForm` and decides nothing.** Probed:
- nested `(0,3]`, `[1,3]` and `[3,3]`;
- an `on each` root `[1,3]`;
- a nested past `(0,1]`.

**Deferral (c), stutter delays:** cannot settle wrongly. EN-6 and the checker read idle-tail stutter positions at every valuation of the time-elapsed zone, which over-approximates the stutter instants, so proofs are sound. A refutation needs FR-237's replay to reproduce, so the worst outcome is `inconclusive`.

Gates, run with locked-build at jobs 4 and `--all-features`:
- qsl-analyze `zone_`: 83/83.
- qsl-eval `timed_trace` and `property_automata`: 35/35.
- qsl-replay lib `certificate`: 27/27.
- clippy on qsl-eval, qsl-replay, qsl-analyze and qsl-semantics with `-D warnings`: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The module doc and the refusal's doc call the nested-interval refusal a "spec gap STD-493: MightyL's general non-punctual construction". FR-242's Behavior requires the MightyL translation for every TT-3/TT-4 formula, and nested non-punctual intervals such as `(0,b]` and `[a,b]` are inside it. So this is unbuilt work, not a spec gap (see SR-1650002 FND-002). Reword both docs to name the owning ticket. | qsl-eval/src/automaton/timed_claim.rs:36-43; qsl-analyze/src/zone_check/search.rs:262-267 |

## Verdict

**No major bugs.** The construction, EN-6's use of it and the checker's rebuild are sound and exact on the admitted subset, and everything outside it is refused. One low documentation finding. The test-strength and AC gaps are in SR-1650002.

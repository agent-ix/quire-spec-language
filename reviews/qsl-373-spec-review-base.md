---
id: SR-1110
title: "QSL-373 base spec review of ADR-026, FR-230 to FR-254 and their amendments"
type: SpecReview
analysis: base
date: 2026-10-02
scope: "agent-ix/quire-spec-language@2b54b7e47b9e99fe95392e5749671333d9025f07; diff 404a5a88..2b54b7e4 (the QSL-373 commits on top of the stale #366 stack); spec/decisions/ADR-026-dense-time.md; ADR-018 amendments; spec/functional/FR-230..FR-254; spec/test-cases/TC-685..TC-710; spec/usecase/US-026; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-237
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-240
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-243
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-247
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-248
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-252
    type: reviews
---

## Summary

Ticket: QSL-373 (PR #573). This review covers ADR-026, FR-230 to FR-254, TC-685 to TC-710, US-026, the ADR-018 amendments and the index rows. It checks them against the owner rulings on QSL-373 and RES-54, the team-leader delay decision, the Wave B and Wave C rules, and the QSpec counterpart FR-415 to FR-421 (QSpec branch spec/wave-b-q4-prob-time).

**Consistent:**
- Dense time is specified (RU-1). Both clock bindings are kept, with `model-time` the default for a timed model (CB-5, FR-233).
- A non-local time-lock reuses ADR-022's trap evidence and V-10, and `TimeLock` stays its own kind (TD-3, FR-232, FR-237).
- EN-6 is a native zone engine with Wimmer-style certificates; a proof counts only after `check_zone_certificate` accepts it (CF-1 to CF-6, FR-244, FR-245). FR-245 places the checker in the qualified core.
- Hybrid dynamics go to QSpec FR-193 providers, and no solver result settles `refuted` (HY-1 to HY-3, FR-250).
- Schedulability is native closed-form (RTA, EDF QPA, AMC-rtb) with WCET as a premise in the obligation identity (RT-1 to RT-8, FR-246 to FR-249).
- Monitors run on integer ticks with sound rounding and a caller-set event rate (MN-1 to MN-4, FR-251).
- Delay distributions are specified; exact checking routes to EN-5 through digital clocks (SS-7, FR-254).
- Delay decision applied: an operation with no `delay` member has a free, nondeterministic delay, and a probabilistic bound is read on the min/max over its choices (SD-1, SD-5, FR-253); statistical checking settles a racing free delay `unsupported`, as QSpec FR-420 states.
- An undefined evaluation refutes with `UndefinedEvaluation{where, cause}` (TV-1, FR-235, FR-237, FR-239); see base FND-001/002 for the admitted-behaviour gap.
- No fixed caps: every limit has a published default and names itself when reached. No pins, ledgers, provenance records or compatibility paths. Ticket ids appear only in References.
- The worked examples were rechecked by hand: the §11 counterexample and time-lock, the §9 retry run, RTA `(1, 3, 12)` and the `C = 6` demands `9, 10, 12, 13`, the AMC set `Mc`, the stochastic `1/20`, the monitor ticks and buffer 21.
- Every AC has a TC row that tests behaviour. `quire validate` on the 56 changed spec files exits 0 (4 EARS warnings, ears FND-001/002). `tools/check-index-completeness.sh` passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Ticker's time invariant `x <= 1` holds everywhere and `step` needs `n < 2`, so the state with `n = 2` reached at time 2 is a local time-lock (time stops at `x = 1`, no step enabled). No time-divergent behaviour passes through position 2, so it is not an admitted behaviour (TS-4, TD-1) and QSpec FR-416 refutes only an undefined value 'at a position of an admitted timed behaviour'. The expected `Violated`/`UndefinedEvaluation{where: 2}` contradicts the semantics; the time-lock-freedom item is what reports this model. Guard the invariant (`when self.n < 2`) so the `n = 2` state is quiescent. TC-710, FR-235-AC-5 and FR-237-AC-5 inherit it. | spec/functional/FR-239-check-a-timed-claim-by-symbolic-zone-search.md:153 |
| FND-002 | medium | The search refutes on the first symbolic state holding an undefined point, and decides TT-1/TT-2/deadlock items 'by this search alone', with no check that the violating prefix extends to a time-divergent behaviour. Under TS-4 and TD-1 a violation reached only through a time-locked state holds vacuously, and QSpec FR-416 requires an admitted behaviour. State that a refutation needs a divergent extension of its prefix (or that the prefix's last state is not a time-lock), in FR-239 and ADR-026 TV-1/EZ-3. | spec/functional/FR-239-check-a-timed-claim-by-symbolic-zone-search.md:126-133 |
| FND-003 | medium | `NoLateReply` over `Rpc` with `T = 4 ms` must return `HoldsDigitized` here, but `Holds` with a zone certificate in FR-239-AC-1, FR-244-AC-1 and TC-694. EZ-10 says EN-6 'may' digitize and FR-243 only says when the path is allowed, so path choice is unspecified while FR-239 claims the outcome is a function of subject, item and limits. Fix the selection rule (always digitize when eligible, or a request setting) and make the four ACs agree. | spec/functional/FR-243-digitize-a-closed-timed-subject-for-explicit-state-checking.md:70 |
| FND-004 | medium | Replay picks the local arm whenever no positive delay is admissible at the last state. A non-local time-lock can sit at such a state with a Zeno step enabled (Stall at `x = 1`, which FR-232-AC-3 names), and the local arm then finds `ping` enabled and settles `ReplayParity` on a genuine time-lock. TD-3 and QSpec FR-417 select the arm by local versus non-local kind. Select by kind: run the trap exploration whenever some transition is enabled. | spec/functional/FR-237-replay-a-timed-counterexample.md:87-100 |
| FND-005 | medium | Vacuity (`NoAdmittedBehaviour`) lives in FR-240, whose scope is TT-3/TT-4 and time-lock items, yet this AC, FR-232-AC-3 and FR-235-AC-4 apply it to `always holds(true)`, a TT-1 claim FR-239 decides by its search alone. FR-239 has no vacuity rule, and a CF-1 reachability certificate does not show an admitted behaviour exists, so TD-6 is unimplemented for TT-1, TT-2 and deadlock-freedom proofs. Add the vacuity check (and its certificate part) to every proof path. | spec/functional/FR-240-decide-timed-liveness-and-time-lock-freedom-on-the-symbolic-graph.md:101 |
| FND-006 | medium | Fixed-priority miss evidence is the demand at scheduling points up to `D_i`. That is the exact test only for constrained deadlines with no jitter or blocking; RT-4 also admits `J`, `B` and `D_i > T_i` (busy-period extension), where a genuine RTA miss has no such point set (the miss may be a later job, or at `D_i - J_i`). `check_closed_form` would then settle `ReplayParity` on a real miss. Use the RTA iterate that exceeds `D_i` (per job for the busy-period case) as the evidence. | spec/functional/FR-247-analyse-a-task-set-in-closed-form.md:58-60 |
| FND-007 | medium | `kani_events` (default 8) is an unwinding depth that defines what the agreement obligation proves (traces of up to k events), yet it is presented as an ADR-014 B-5 resource budget. Depth is never a limit kind; reaching it is not a stop, it is the obligation's horizon. Name it the obligation's stated horizon, as #366 did for `max_depth`, and say the discharged obligation holds for traces up to that length. | spec/functional/FR-252-hand-timing-obligations-to-code-generation.md:38-40 |
| FND-008 | medium | Modular differences cannot tell a backwards reading from a wraparound without a maximum gap between readings. The plan's only rate input is a maximum event rate, which bounds spacing from below, not the gap from above, so 'no wraparound possible under the rate limit' is undecidable from the inputs. Add a caller-set maximum inter-reading gap (or minimum rate) to `MonitorTarget`, and state the fault rule against it. | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:91 |
| FND-009 | medium | The branch is stacked on a stale copy of #366 (404a5a88). origin/spec/366-temporal-properties is now e6a5fb56 after a rebase, so `git diff origin/spec/366-temporal-properties...HEAD` shows all of #366 again, and this PR's ADR-018 §1/SM-3/SM-4 amendments are written against superseded ADR-018 text. Rebase onto the current #366 and re-apply the amendments to its text. | spec/decisions/ADR-026-dense-time.md:- |
| FND-010 | low | 'A location reachability item' names no item kind that exists in QSL: the forms are TT-1 to TT-4 plus the derived deadlock- and time-lock-freedom items. Name the item kinds that qualify, or drop the phrase. | spec/functional/FR-243-digitize-a-closed-timed-subject-for-explicit-state-checking.md:50-54 |
| FND-011 | low | `check_closed_form` takes no limits (only `poll`), but its map has a row for 'a check a budget stopped' and `ClosedFormCheck::Stopped`. Give the check a caller-set budget with a published default, or drop the row. | spec/functional/FR-248-settle-a-closed-form-verdict-after-recomputing-its-evidence.md:42-47,73 |

## Verdict

The design follows every owner ruling and the delay decision, and the arithmetic in the examples is right. Two problems are semantic: an undefined value or violation reached only through a time-locked state is refuted, against TS-4/TD-1 and QSpec FR-416 (FND-001 high, FND-002), and vacuity is missing from the TT-1/TT-2/deadlock proof path (FND-005). The digitization path choice is unspecified, so four ACs disagree on one item's outcome (FND-003), and time-lock replay picks its arm by the wrong test (FND-004). The branch needs a rebase onto the current #366 (FND-009). Not mergeable as it stands.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | medium | The fix routes a non-probabilistic TT-1 or deadlock-freedom item to EN-5 when it names `exact` evidence. `exact` evidence is a setting of probabilistic claims (ADR-024 SV-9), and EN-5's digital route reads only probabilistic claims (ADR-028 TA-1: 'A probabilistic claim over a timed subject'; QSpec FR-421: 'a probabilistic-satisfaction item'). A temporal-satisfaction item has no `exact` evidence kind and EN-5 has no reading for it. Either define that route in ADR-028 and QSpec FR-421, or drop it: EN-6 decides every non-probabilistic timed item, and `HoldsDigitized` comes only from EN-5 over probabilistic claims (FR-243-AC-1/AC-2, FR-235-AC-2 and TC-698 change with it). | spec/functional/FR-243-digitize-a-closed-timed-subject-for-explicit-state-checking.md:22-35, 49-51 |
| FND-013 | medium | For a miss by job `q > 0`, the checker recomputes job `q`'s iterates but never checks that jobs 0 to `q - 1` keep the level-`i` busy period open (each earlier job's completion `w_p` exceeds the next release, `w_p > (p + 1) · T_i − J_i`). Past the end of the busy period, the job-`q` recurrence overstates the completion time, so evidence naming such a `q` could refute a schedulable set. Add that check (or have the evidence carry the earlier jobs' completions) to FR-248 and RT-7. | spec/functional/FR-248-settle-a-closed-form-verdict-after-recomputing-its-evidence.md:59-64 |
| FND-014 | low | FR-235's Inputs name only `ZoneCheckOutcome` (FR-239), which no longer has `HoldsDigitized`, while its table maps `HoldsDigitized` (FR-243). Add FR-243's `DigitalOutcome` to the inputs, or drop the row with FND-012. | spec/functional/FR-235-settle-a-timed-verdict-as-a-terminal-record.md:48, 77 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 41f6943a |
| FND-002 | fixed | 41f6943a |
| FND-003 | fixed | 41f6943a |
| FND-004 | fixed | 41f6943a |
| FND-005 | fixed | 41f6943a |
| FND-006 | fixed | 41f6943a |
| FND-007 | fixed | 41f6943a |
| FND-008 | fixed | 41f6943a |
| FND-009 | fixed | 41f6943a |
| FND-010 | fixed | 41f6943a |
| FND-011 | fixed | 41f6943a |

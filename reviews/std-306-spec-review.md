---
id: SR-15760
title: "Spec review of quire-spec-language PR #1241: STD-306, refinement, reductions and strong fairness over timed subjects"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a48fc5a7db281e26a3ab872eb81bdf2b712bb870; PR #1241 (git diff origin/main...a48fc5a7d): spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md (new), spec/test-cases/TC-1170-timed-refinement-reductions-and-strong-fairness.md (new), spec/functional/FR-159-pre-check-selected-reductions-against-the-preservation-table.md; read for context: ADR-026 §2 CK-1 to CK-7, §3 TS-1 to TS-6, §4 TD-1 to TD-6, §7 TV-1, CT-2, §8 EZ-1 to EZ-8, CF-2, §15; FR-144, FR-152, FR-159 Order and Capability, spec/spec.md; agent-ix/quire-specification main 5ee0071fc FR-377, FR-382, FR-416"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-430
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1170
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-159
    type: reviews
---
# SR-15760: Spec review of quire-spec-language PR #1241: STD-306

## Summary

Ticket: STD-306. Sub-analyses folded in: integrity, EARS, failure-domain, dependency. The head a48fc5a7d is behind main 43f4e2086 but merges cleanly. `make check-spec-validation` on the merge result exits 0 with no diagnostic on FR-430, TC-1170 or FR-159. FR-430 and TC-1170 are unique across all 47 open PRs. The other lanes' new TCs are TC-1160 to TC-1167, TC-1171 and TC-1172, and their new FRs are FR-394, FR-405, FR-410 and FR-411. No other open PR touches FR-159. The files do not overlap #1238 or #1239.

What checks out against ADR-026:

- Delays map to no abstract step, matching §15.
- The refusal of a clock read in the mapping or a step row is consistent with CK-4.
- The safety half refutes only at a pre-state point with a time-divergent continuation, matching TV-1 and EZ-5.
- The liveness half reads only time-divergent behaviours, matching TS-4 and EZ-6.
- Symmetry permutes clocks with their objects, matching §15.
- The strong-fairness reading mirrors TD-5, its lasso form CT-2, and its certificate CF-2's nested numbering.
- AC-1, AC-2's `Violated` vector without the invariant (send after delay 1, undo after delay 1, `RefinementFailure` at position 2, by EZ-8's least-delay rule) and AC-3's `Timer` verdicts are correct by analysis.
- The QSpec follow-up is accurate: QSpec FR-377, FR-382 and FR-416 say nothing about timed refinement, reductions over a timed subject or strong fairness over time.

## Verdict

**NOT MERGEABLE.** The strong-fairness vector's `Holds` is wrong by analysis (FND-001). The partial-order and state-constraint outcomes contradict FR-159's own negotiation rule (FND-002). AC-3 claims a canonical equality that the FR's tie-break does not give (FND-003). The safety half is routed to EN-6, but FR-144 routes it to the explicit-state engine (FND-004). AC-2's time-invariant variant has no admitted behaviour, so it settles `NoAdmittedBehaviour`, not `Holds` (FND-005).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | [major] AC-4's `Holds` under `fair { strong Blink::fire }` is wrong. `Blink` has no time invariant and no urgency, so every discrete state is quiescent (ADR-026 TS-5). The behaviour that never ticks and delays forever from the initial state is admitted. Its stutter tail is outside every fairness constraint, and `fire` is enabled only while `x` is in `(0, 1)` of the first delay, never at arbitrarily late instants. Strong fairness is therefore satisfied without `fire`, `b.fired` stays false, and the claim is `Violated` by that idle-tail lasso (CT-2: `x` above 1 at entry). Under weak fairness the canonical counterexample may also be that idle tail rather than "a lasso of `tick` steps". Fix: add `time invariant when true { x <= 1 }`. Then `tick` fires exactly at `x = 1`, every delay is 1, and `fire` is enabled inside every delay and at no position: weak gives `Violated` with a `tick` lasso of loop delay 1, strong gives `Holds`. | spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md:139; spec/test-cases/TC-1170-timed-refinement-reductions-and-strong-fairness.md:24,33 |
| FND-002 | high | [major] The partial-order outcome contradicts FR-159's Capability rule. Negotiation "SHALL route an item that selects reductions only to a candidate advertising every one of them, and SHALL settle the item V-8 `unsupported-requested-capability` when no candidate does". FR-430 makes EN-6 advertise `symmetry` only, so a dense-time item selecting partial-order reduction is never routed to an engine whose pre-check reaches step (a). It settles V-8 at negotiation, not `ReductionNotPreserving{PartialOrder, form}`. The state-constraint outcome (V-8) is right, but it is settled at negotiation, not at FR-159 step (c), which tests the system's `offered_reductions`. Two related problems: the FR-159 row reads "every form or item over a timed subject", and under CK-1/TS-1 a timed subject includes `time tick` digital-time subjects, which §15 does not cover (it speaks of EN-6); and `preserves(reduction, form)` has no subject input to read. Planner decision: settle partial-order over a dense-time subject as V-8 at negotiation (what §15's "EN-6 applies ... no partial-order reduction" says), or state that the preservation lookup runs before negotiation for dense-time subjects and give `preserves` the subject's time kind. In both cases, scope the rule to `time dense` and fix AC-3's two outcomes and loci. | spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md:107-115,138; spec/functional/FR-159-pre-check-selected-reductions-against-the-preservation-table.md:95-96,114-119 |
| FND-003 | medium | [major] AC-3's "a symbolic state and its image under swapping `t1` and `t2` (discrete fields and clocks) canonicalise to equal states" is false for states whose two timers have equal discrete state. FR-430 breaks those ties "by the least permutation in key order", which is FR-152's identity on equal sort keys. So `(unarmed, unarmed, Z)` and its image `(unarmed, unarmed, swap(Z))` stay distinct whenever `Z` orders `x1` and `x2`. Such a state is reachable: arm then disarm `t1` gives `x1 < x2`. The PR body itself says the canonical form is "not a unique orbit representative". Name a state whose timers differ in `armed` (for example `t1` armed, `t2` not), or state that equality holds only when the discrete permutation is unique. | spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md:103-106,138 |
| FND-004 | medium | [major] FR-430 has EN-6 decide the safety half on the symbolic product. FR-144 says that negotiation "SHALL route the record [TP-5] ... to the explicit-state engine's provider". FR-430 neither amends FR-144 nor says that EN-6's manifest advertises the TP-5 form for a timed concrete subject (EZ-1 lists only `temporal-satisfaction` bounded and unbounded). As written, a timed-concrete refinement routes to EN-1, which has no zone semantics. Add the routing rule and the EN-6 advertisement, as an FR-144 amendment or in FR-430. | spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md:86-92 |
| FND-005 | medium | [major] AC-2's `Holds` with the time invariant `x <= 1` is wrong by analysis, and it does not test the rule it cites. No clock is ever reset (`send` and `undo` reset nothing), and `x <= 1` holds everywhere. So `x` reaches at most 1 and no behaviour of the subject is time-divergent. At `x = 1` no delay is admissible, and `send` (no precondition) stays enabled, so every continuation is Zeno. ADR-026 TD-6 then settles the proof `inconclusive`, `NoAdmittedBehaviour`, not `Holds`, and the time-lock-freedom item refutes. Separately, the guard `x >= 2` never meets the zone, so the `undo` edge does not exist (EZ-3), and an engine without TV-1's "refute only at a time-divergent pre-state point" filter gives the same result. Use a variant that is time-lock free, for example one where `send` resets `x` and `undo` is only reachable through a time-locked region, and whose failing edge exists but leaves only from points with no divergent continuation. Alternatively, state the `NoAdmittedBehaviour` outcome. | spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md:137 |
| FND-006 | low | [minor] FR-430 is not linked from spec/spec.md, where nearly every FR on main appears in `relationships` and the artifact table (FR-400 and FR-403 are the only other gaps). Add it append-only. #1237, #1242, #1253, #995, #703, #686 and #683 also edit spec.md, so keep the rows minimal to avoid textual conflicts. AC-1 also leaves `Req`'s and `TReq`'s initial values and the concrete model of the timed-abstract variant unstated. | spec/spec.md; spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md:136 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | [minor] FR-430 says that EN-6's manifest "SHALL advertise (`temporal-satisfaction`, TP-5)". FR-075 descriptors advertise `(Capability, mode)` pairs, with `mode` `bounded` or `unbounded`, and TP-5 is a form, not a mode, so the advertised pair has no wire shape. ADR-026 EZ-1, which lists EN-6's advertised pairs, is not amended in the PR (only the PR body flags it). Planner decision: route by the concrete subject's `time dense` declaration alone (FR-144 already says so) and drop the TP-5 advertisement sentence, or add an EZ-1 amendment row giving the form-scoped advertisement a defined shape. | spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md (Timed refinement, routing bullet) |
| FND-008 | low | [minor] TC-1170's Description still says "partial-order reduction and state constraints settling at pre-check over a timed subject", and its title says "partial-order refusal". Both now settle V-8 at negotiation (FR-430, TC-1170 step 3 expected result). Reword the Description to "at negotiation". | spec/test-cases/TC-1170-timed-refinement-reductions-and-strong-fairness.md:3,13 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Head 9e925df56: Blink has "time invariant `when true { x <= 1 }`". Worked by hand: `tick` fires exactly at x = 1 and every loop delay is 1; `fire` is enabled only on (0, 1). Weak fairness gives `Violated` with a `tick` lasso of loop delay 1. Under strong fairness, every divergent behaviour enables `fire` at arbitrarily late instants, so `Holds`. No state is quiescent, so there is no idle tail. TD-6 passes. |
| FND-002 | fixed | Head 9e925df56: FR-159 is untouched. Over a `time dense` subject, POR or a state constraint settles V-8 `unsupported-requested-capability` at negotiation by FR-159 Capability ("EN-6's provider manifest SHALL advertise `symmetry` and no other reduction"). `time tick` subjects keep FR-159. AC-3 and TC-1170 step 3 match. Planner decision applied. |
| FND-003 | fixed | Head 9e925df56: AC-3 uses "the symbolic state with `t1` armed, `t2` unarmed and zone `Z`" and its swapped image. FR-152's sort key differs on `armed`, so the discrete permutation is unique and both map to one canonical state. The FR now says that tied states may stay distinct. |
| FND-004 | fixed | Head 9e925df56: FR-144 reads "a record whose concrete subject declares `time dense` routes to EN-6's provider instead (FR-430)", and FR-430 has the matching routing bullet. The advertisement wording is FND-007. |
| FND-005 | fixed | Head 9e925df56: new trap-region vector, worked by hand. With no invariant: `send`@1, `enter`@1, `undo`@0, `RefinementFailure` at position 3 (minimum 3 steps; least delays 1, 1, 0). With `when self.trap { x <= 0 }`, `undo`'s only pre-state is (sent, trap, x = 0) with no delay admissible. After `undo` nothing is enabled, a local time-lock, so the failing edge has no divergent continuation and does not refute (TV-1). The initial state is quiescent, so TD-6 passes and the result is `Holds`. The time-lock-freedom item is refuted. |
| FND-006 | fixed | Head 9e925df56: spec.md appends FR-430 to `relationships` and to the artifact table (+3 lines, append-only). AC-1 states initial values, and the timed-abstract variant's concrete model is `TReq2`. |
| FND-007 | fixed | Head 8033385e4: the advertisement sentence is dropped. FR-430 reads "Negotiation SHALL route the refinement's record (FR-144) whose concrete subject declares `time dense` to EN-6's provider, as FR-144 states". This leaves no undefined (Capability, mode) pair and no ADR-026 EZ-1 amendment is needed. |
| FND-008 | fixed | Head 8033385e4: the TC-1170 title is "... reductions settled at negotiation ...", and the Description says "partial-order reduction and state constraints over a `time dense` subject settling V-8 at negotiation". |

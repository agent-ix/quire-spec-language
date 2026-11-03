---
id: SR-2198001
title: "spec-review of QSL spec batch 7 (quire-spec-language PR 2198)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR 2198 head d6f1f444d; FR-251, FR-106, FR-084, FR-391, FR-390, FR-514, TC-706, TC-1081, TC-1171, TC-1064, TC-1309"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-514
    type: references
---

# Spec review of QSL spec batch 7 (PR 2198)

## Summary

Tickets: QSL-1463, QSL-1512, QSL-1481, QSL-1487, QSL-852. Diff `origin/main...HEAD` of 11 files. Integrity, EARS, failure-domain, object and criterion-strength lenses were applied inline as one pass.

Verified: `quire validate` exit 0 on all 11 files (only module duplicate-archetype warnings; no EARS warnings). New ids FR-251-AC-22..26, FR-106-AC-66/67, FR-514-AC-7..10 are unused on main and no other open QSL PR (2184, 2192, 2193, 2195, 2196, 2197) touches spec/. `quire matrix`: the new ACs are untagged (expected for a spec-only PR). FR-251 capacities recomputed with rate 0.002/tick, u=0: single once[0,10ms] = ceil(20.002)+1 = 22; AC-21 inner = ceil(0.002 x 15002)+1 = 32, outer once 22, outer eventually ceil(0.002 x 5001)+1 = 12, second-order inner 32; AC-18 inner 32; AC-22 unbounded once = ceil(0.002)+1 = 2 (u=2 ticks: ceil(0.01)+1 = 2); AC-23 nested = ceil(0.002 x 10002)+1 = 22. All numbers hold. QSpec citations (FR-153-AC-14, FR-144-AC-16, FR-322-AC-310, Contract IR FR-053-AC-4/FR-038-AC-279) exist and say what is cited. QSL-1487 AC-14 correctly left unedited (QSpec FR-144-AC-14 vs QSL FR-323-AC-3 conflict stands; planner owns).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-514 contradicts itself and FR-362-AC-4: the "Ok(unit)" bullet and AC-3 ("with sufficient limits, Complete") and FR-362-AC-4 say a Complete inventory has one row per requirement record and per pending obligation, but the new IR-identities text and FR-514-AC-8 make qsl_inventory over any unit with at least one obligation return InventoryRefusal::UnmappedObligationKind and no rows. T0 (AC-3 sufficient limits) and AC-8 (same T0) cannot both hold. This also drops ticketed behavior (QSL-1512 AC-8 / FR-362 item 3, 10: a usable inventory for every unit with a claim) until an IR member exists; a scope cut that needs a planner ruling. | FR-514 lines 70-75, 202-212; FR-362-AC-4 | wrong-requirement |
| FND-002 | medium | FR-514 "The IR identities": SourceTypeExhaustion requires prior, requested and a full site, which the spine does not hold, and the requirement-form row has no IR identity; the text then says the cause is reported "with the fields the IR has" plus QSL's own InventoryCause. No producible IR value exists for an Incomplete outcome, so the stage-limit path cannot be handed to IR as written. The three IR asks are only listed under Dependencies, with no IR ticket named. | FR-514 "The IR identities", Dependencies | missing-requirement |
| FND-003 | high | FR-251 Overflow with an unbounded past operator is unsound for later verdicts. The overflowing event is neither appended nor folded ("summary cell unchanged"), yet "no event dropped"; FR-487 says the body returns Incomplete "instead of appending". After that, a later once p can settle false (or historically true) over history that lacks the lost event. Only the "affected obligations" settle Incomplete; nothing makes the loss sticky as a gap does (history_start). | FR-251 Unbounded past operators, Unknown outcomes (1); AC-23 | missing-requirement |
| FND-004 | high | "Plan derivation SHALL NOT refuse these clauses ... no operator is typed unsupported" but the rules cover only an unnested unbounded past operator and unbounded-over-unbounded future. Uncovered: a bounded operator nested under an unbounded past one (L(n) sums the upper bounds of enclosing operators; an unbounded one has none), an unbounded past operator over a future operator (historically (req implies eventually[0,5 ms] ack): the fold would take an operand value not yet settled when the entry ages out at one cover), unbounded past over unbounded future, and intervals [a,*] (ADR-014 TR-3 Open upper bound) whose lower bound a delays folding. The no-refusal rule makes these silently unspecified. | FR-251 Buffers, Unbounded past operators | missing-requirement |
| FND-005 | medium | Lower-boundary inconsistency. "Before any gap the history begins at the monitor's baseline and the operators are decided without that cap" (once false, historically true) contradicts the existing rule that history_start is initially the baseline and a past operator whose window reaches before history_start is indeterminate (bounded once at the start), and FR-501's HistoryCutoff/AuthoritativeOrigin choice is not named. State which lower-boundary kind the monitor uses. | FR-251 Unknown outcomes (2), lines 92, 334; FR-501-AC-8 | wrong-requirement |
| FND-006 | medium | FR-251-AC-25 claims for both always eventually p and eventually always p that "the first p settles the two live instances true". For eventually always p the inner instances are always p, which the first p does not settle true. | FR-251-AC-25 | wrong-requirement |
| FND-007 | medium | Post-gap behaviour of since and triggered is "follow their own definite witnesses the same way", with no rule and no AC (AC-24 covers once and historically only); the fold-state sufficiency claim for since/triggered (one accumulated value equals the operator over the folded prefix) is asserted, not shown against FR-092. | FR-251 Unknown outcomes (2); AC-24 | correct-requirement-no-evidence |
| FND-008 | medium | FR-084-AC-2 and FR-391-AC-5 now say the family-owned Undefined(incomplete-closure) result "carries" incomplete_population/unclosed-subtypes as code and cause, "one result under two names". FR-090 defines that result with reason, queried type and uncovered_subtypes and no catalog code, and TC-227 / TC-1076 row 7 expected results are unchanged, so the new claim is neither reconciled with FR-090 nor tested. FR-391-AC-5's new "no row for counters" case also has no TC-1076 row. | FR-084-AC-2; FR-391-AC-5; FR-090 line 62; TC-227; TC-1076 row 7 | correct-requirement-no-evidence |
| FND-009 | medium | Check 7 now makes an unlisted required population missing-required-artifact, but TC-1048 row 16, TC-1034 row 9 and the AC text of FR-106-AC-65 ("empty/unlisted declaration ... with it incomplete") still say an unlisted extent or population is incomplete-scope. Reconcile (admission vs evaluator, population vs relationship extent). | FR-106 check 7; FR-106-AC-65, AC-67; TC-1048; TC-1034 | wrong-requirement |
| FND-010 | low | FR-251-AC-26 says "a node index alone is not a body" while the key rule (FR-487) covers a helper by the index segment of its body-root node. Say whether that segment is the body-root node_id (content) or a graph position. | FR-251-AC-26; QSpec FR-487 key | wrong-requirement |
| FND-011 | low | FR-251 State row (line 92) still describes each buffer as a sequence of entries and says every fold is bounded by capacity; the summary cell (a first element of a different shape) and the wording "the +1 is its own slack ... cell not counted" are not reflected there. | FR-251 State row | wrong-requirement |
| FND-012 | low | FR-390 Unavailable location becomes Option with no AC or TC row; the non-reader view case (location Some?) is not stated. | FR-390 lines 238-242, 739-743; FR-390-AC-7 | missing-requirement |
| FND-013 | low | FR-106 collection members: "an object whose one member is another collection tag is wrong-value-kind" leaves an object with two other tags unspecified; FR-251-AC-22 and AC-26 and FR-106-AC-66 each bundle several independent claims (single-claim rule). | FR-106 Collection document members; FR-106-AC-66; FR-251-AC-22, AC-26 | correct-requirement-no-evidence |

## Verdict

Changes requested; not mergeable. FND-001, FND-003 and FND-004 are high. The numeric examples, FR-106 native member causes, FR-390 text, TC-1064 row 9 and the FR-251 PlanId body coverage are otherwise sound. QSL-1481: no ticketed QSL behavior is dropped by the amendments to FR-106/FR-391/FR-084 beyond the cause renaming for unlisted populations (FND-009) and the unreconciled FR-090 mapping (FND-008). QSL-1512: FR-514 drops the Complete-with-obligations behavior (FND-001); needs a planner or Peter ruling (option: Incomplete with requirement rows and a typed unmapped-kind cause rather than total refusal, or an IR member now).

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | UnmappedObligationKind and the total refusal are gone; Ok(unit) is Complete with requirement and obligation rows (FR-514 bullet, AC-3, AC-8, TC-1309 row 13 agree). Interim clauses I-1..I-3 each state the placeholder values and a removal condition. Residual for Peter's ruling, not a defect: I-2 puts the placeholder kind GraphClosure and I-3 prior=bound into IR-bound identities; they are documented as placeholders with QSL's own row carrying the real values. |
| FND-002 | fixed | The three IR asks are listed precisely (form identity, capability-kind member with a non-Copy payload, stage/setting/observed plus optional prior/requested/site); the interim clauses make the Incomplete path producible. IR ticket ids are still not named; the planner should file them. |
| FND-003 | fixed | Overflow of an unbounded past operator is sticky: buffer emptied, cell reset to indeterminate with its own history_start; once never false and historically never true from the lost event (AC-26, AC-27, TC-1081 step 17). |
| FND-004 | fixed | [a,*], bounded under unbounded past (adds 0 to L), past over bounded future (fold after settle, Pending), past over unbounded future (fold on settle, shared activation capacity) are specified with AC-31 to AC-34. Recomputed: 22, 32/22/12, 2, AC-31 22 and 22 and 2, AC-32 12, AC-34 12; all correct. |
| FND-005 | fixed | Lower boundary is HistoryCutoff, accumulator starts indeterminate (AC-25). AuthoritativeOrigin being not selectable narrows no ticketed behavior (bounded past already read indeterminate before history_start), but see FND-014. |
| FND-006 | fixed | AC-35 (always eventually) and AC-36 (eventually always) are separate and each correct. |
| FND-007 | fixed | Re-derived against QSpec FR-092 (p since[a,b] q: some d in [a,b], q at i-d, p at i-e for e in [a,d)): S_i = q_i or (p_i and S_(i-1)) is exact, triggered is the dual, and the [a,*] case is the [0,*] operator shifted by a, so one folded value is sufficient. Post-gap AC-28 and AC-30 cover once, historically, since and triggered. |
| FND-008 | fixed | FR-090 now states the projection of the undefined record to incomplete_population/unclosed-subtypes (derived, not stored); FR-090-AC-21, TC-227 and TC-1076 rows 7 and 7b follow. |
| FND-009 | fixed | FR-106 check 7 is scoped to a required population; an unlisted relationship extent keeps incomplete-scope (TC-1048 row 16, TC-1034 row 9 say so); AC-65 amended; AC-67 added. |
| FND-010 | fixed | FR-251-AC-38 says the key covers the helper through its body root's content-bearing node_id, not a graph position. |
| FND-011 | fixed | The State row names the summary cell (an accumulator and its own history_start, not counted in the capacity). |
| FND-012 | fixed | FR-390-AC-64 and TC-1072 row 10 assert Some and None. |
| FND-013 | fixed | The two-other-tags case is specified; AC-66, AC-68, AC-69 split the claims; FR-251 ACs split to AC-22..39. AC-29 and AC-31 still bundle sub-claims (low, accepted). |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-014 | medium | QSpec FR-092 refuses an unbounded past interval under a bounded profile and, under the infinite-trace profile, reads atoms before position 0 as false (FR-092-AC-8), which is the AuthoritativeOrigin result FR-251 says a monitor cannot select. FR-251 does not say which profile the AC-7 reference evaluator uses for an unbounded past operator, so the differential for AC-23..34 has no stated oracle. Needs a ruling. | FR-251 Lower boundary, AC-7; QSpec FR-092 line 79, AC-8 | missing-requirement |
| FND-015 | high | Bounded-operator overflow can yield a wrong definite verdict: the lost event is not appended, the buffer keeps older entries, and only the parenthetical "affects every obligation whose window contains the lost event" (not normative, no AC) addresses later obligations. A later once[0,10ms] p can settle false though the lost event had p true; historically can settle true. Not introduced by this PR (pre-existing), but the PR now makes the unbounded case sound and states the bounded one only in a parenthesis. Cheap fix with existing State fields: on overflow raise history_start to the overflowing tick (as on_gap does), so windows reaching before it are indeterminate until refilled; add an AC. | FR-251 Overflow; FR-487 (Incomplete instead of appending) | missing-requirement |
| FND-016 | low | FR-251-AC-29's first case omits p's value at tick 5000, which the recursion needs (S := q or (p and S)); the result is not computable from the AC as written. | FR-251-AC-29 | ambiguous |
| FND-017 | low | FR-106-AC-67 names TC-1076 as verifier (row 7b) but TC-1076's Scope line lists only FR-391-AC-1 to AC-9. | FR-106-AC-67; TC-1076 | correct-requirement-no-evidence |

Residual disposition table, new findings: FND-014 open (ruling), FND-015 open, FND-016 open, FND-017 open.

## Verdict (disposition pass 1)

13 of 13 original findings fixed; `quire validate` exits 0 on all touched files; ids FR-251-AC-22..39, FR-106-AC-66..69, FR-390-AC-64, FR-514-AC-7..10 are unused on main and no other open QSL PR (2184, 2192, 2193, 2195, 2196, 2197, 2199) touches spec/. Mergeable except FND-015: close it in this batch (two sentences plus one AC) or the planner records an explicit ticket and accepts the pre-existing gap; FND-014 needs a planner or Peter ruling but does not block.

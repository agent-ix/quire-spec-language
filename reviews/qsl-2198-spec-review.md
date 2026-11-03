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

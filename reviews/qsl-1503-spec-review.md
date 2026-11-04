---
id: SR-2227001
title: "spec-review of QSL-1503 evidence-layer rulings (quire-spec-language PR 2227)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-515, FR-516, FR-522, FR-523, FR-524, FR-526 and TC-1310, TC-1311, TC-1317, TC-1318, TC-1319, TC-1321 (git diff origin/main...HEAD at 0af2e270)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-524
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: references
---

# Spec review of QSL-1503 evidence-layer rulings (quire-spec-language PR 2227)

## Summary

Ticket: QSL-1503. One spec-review pass over the 12 changed files at head
0af2e270eb77319d6c3b5c083e68759229630c0f, with the integrity, dependency,
failure-domain and criterion-strength checks done inline (no separate sub-analysis
artifacts; the Jev-based strength analysis was not run).

`quire validate --scope .` exits 0 on all 12 files. No id collision: the new
FR-515-AC-18/19, FR-516-AC-25..30, FR-522-AC-24..26, FR-523-AC-20..22 and
FR-524-AC-32..36 are unused on main and on open PRs 2222, 2224, 2225, 2226; PR 2222
touches only FR-517 and TC-1312. Each TC row maps to its AC id and each trace-tag
range ends at the new top AC (TC-1310 AC-19, TC-1311 AC-30, TC-1317 AC-26, TC-1318
AC-22, TC-1319 AC-36). TC-1321 and FR-526 add no AC and its tag range is unchanged.
The counter-versus-list reading of ruling 2 is stated consistently in FR-516, FR-523
and FR-524 and pinned by FR-523-AC-20, FR-524-AC-34 and AC-35. FR-523-AC-5 and
FR-526-AC-12 agree with the Incomplete-as-result rule. No changelog text; no scope
is cut by wording, except the multi-occurrence case that stays behind a gate
(FND-003).

Author's six points: (1) surplus refusal on deliveries as well as origins is
consistent with FR-524's capture model and is not a defect (question for the planner,
non-blocking); (2) the list/counter reading is sound, but retention of a faulted
instance in the ledger is unspecified (FND-005); (3) the silent-sample mapping needs
the charge point and counters named (FND-007); (4) finding, FND-002 and FND-003;
(5) a withdrawn anchor keeping `Unavailable` plus a `Missing` non-value, with
`incomplete` unset, matches FR-516's existing independent-facts rule and is correct;
only the wording of FR-516-AC-25 that lumps it under "stopped by" is loose (FND-004);
(6) the name `surplus_capture_identity` parallels `contradicted_identity` and is
acceptable, but the index member has no name (FND-006).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | An `Incomplete` is now an `Ok` result that carries `ledger` and `progress_ledger`, but only the lineage stop says what the ledger holds (no entry recorded). For a counter, meter, evicted-valuation or clause stop the spec does not say whether instances created before the stop, the progress-ledger update and the latest-results entry are kept or discarded. FR-523-AC-11 (retry after a stop is byte-equal to a call that never stopped) only holds if they are discarded, and that is not stated. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:117-131 | missing-requirement |
| FND-002 | medium | The call names its clause by `CheckedNodeId`, which names a node shared by structurally equal clauses, and the request has no occurrence selector, so which occurrence's clock row the call assesses is undefined for a shared node. The pairing rule "occurrence with ordinal k pairs with the k-th clock row of that node in declaration order" also differs from QSpec FR-322, where each row is keyed by (declaration, occurrence {role, ordinal}) with producer-assigned ordinals that need not be consecutive from zero, so k-th position and ordinal value can disagree. Pair by the row's occurrence key and say how a request selects the occurrence. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:226-231 | missing-requirement |
| FND-003 | medium | The shared-node, several-occurrence case (a ticketed feature) stays gated on IR-1279 ("the Contract IR decision for the shared node's `temporal_bindings` row"), yet QSpec FR-322 "Temporal clock bindings" already defines one row per occurrence of a node, with vectors BO-P1 to BO-R9 for twins with different and equal configurations. The gate reads as stale or at least unverified, and FR-515-AC-18/19 and FR-516-AC-29/30 cover only clauses of different nodes, so the ordinal pairing the PR states for one node with several occurrences has no AC that can fail. Planner: confirm whether IR-1279 is still open; if FR-322 answers it, replace the gate with an AC over twins of one node with different clock rows. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:184 | missing-requirement |
| FND-004 | medium | FR-516-AC-25 asserts `incomplete` naming the stop only for the first four stops and none for `history_span`, while FR-516's refusal paragraph, FR-523-AC-12 and TC-1318 row 16 say a `history_span` stop returns an `Incomplete` result naming `history_span`. The `incomplete` output row lists only the counter, meter, lineage and evicted stops and omits the clause stops (`formula_nodes`, `history_span`). A test written from AC-25 does not fail when `incomplete` is unset for `history_span`. The AC also files the withdrawn anchor under "stopped by", which is not an Incomplete. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:262 | wrong-requirement |
| FND-005 | medium | A guard-fault instance is listed in `instances` but "not created", and the spec does not say whether it enters the updated `ledger`. If it is retained, a later redelivery contradiction and the `instances` charge rules apply to it; if not, each call re-faults it. FR-524-AC-35 charges 1 and lists three but does not pin the ledger after the call. | spec/functional/FR-524-create-one-temporal-instance-per-trigger-or-origin-with-immutable-captures-and-name-every-capture-fault.md:309-314 | missing-requirement |
| FND-006 | low | "Examine the refusals of the whole call before it charges anything, in list order: a redelivery contradiction, then a surplus capture" reads two ways (all contradictions then all surpluses, or per delivery contradiction then surplus), and no AC has a call carrying both. The surplus refusal names "the index of the first capture fact beyond the declared captures" with no member name, so the refusal's fields cannot be asserted by name. | spec/functional/FR-524-create-one-temporal-instance-per-trigger-or-origin-with-immutable-captures-and-name-every-capture-fault.md:278-279 | missing-requirement |
| FND-007 | low | The silent-sample rule charges the lookup to the call's meter and not to `evaluation_steps`, but FR-523's `valuations` counter is defined as "one atomic `holds` lookup, including one that finds no valuation", which covers a silent-sample lookup, and `positions` may also apply. The charge point name (compare `temporal.visit`) is not given, though FR-523-AC-10 compares charge logs. Say which counters and charge point a silent sample charges. FR-516-AC-27 and FR-523-AC-21 can then assert it. | spec/functional/FR-523-count-temporal-evidence-work-on-thirteen-named-counters-and-keep-a-stopped-instance-apart-from-a-stopped-declaration.md:97-98 | missing-requirement |
| FND-008 | low | TC-1318 now has two rows numbered 23 (the new ledger row and the existing replay row) and the table order runs 22, 23, 24, 25, 23. | spec/test-cases/TC-1318-count-temporal-evidence-work-on-thirteen-named-counters-and-keep-a-stopped-instance-apart-from-a-stopped-declaration.md:41-45 | missing-requirement |
| FND-009 | low | `ActivationInput` gains `execution` ("which an `Unknown` disposition carries") and trigger-record completeness, but no behavior bullet says how `execution` changes a disposition and no AC varies it, and FR-516's request table maps `trigger_scope` to `Evidence` and `Closure` only, with the trigger-record completeness member unmapped. | spec/functional/FR-524-create-one-temporal-instance-per-trigger-or-origin-with-immutable-captures-and-name-every-capture-fault.md:76-82 | missing-requirement |

## Verdict

Not mergeable as written: five medium findings, no high. FND-001 to FND-005 are
specification gaps a coder would have to guess at; each is a one- or two-sentence fix
inside this PR. FND-003 needs a planner answer on IR-1279 against QSpec FR-322 before
the gate is kept or replaced. FND-006 to FND-009 are low.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-010 | medium | The silent-sample rule charges the meter "at the `temporal.visit` charge point" but not `evaluation_steps`, while FR-523's counter table defines `evaluation_steps` as "one charge at the `temporal.visit` charge point (FR-327)" and FR-523-AC-15 distinguishes a meter denial from an `evaluation_steps` denial at that same point. Every `temporal.visit` charge is therefore an `evaluation_steps` charge by definition and the new rule contradicts it without amending the table row. Amend the `evaluation_steps` definition to exclude a silent-sample lookup, or name a separate meter charge point. | spec/functional/FR-523-count-temporal-evidence-work-on-thirteen-named-counters-and-keep-a-stopped-instance-apart-from-a-stopped-declaration.md:90,97-101 | wrong-requirement |
| FND-011 | low | Residue of the occurrence fix: the FR-515 output table row for `clock()` still reads "the `temporal_bindings()` row for this declaration; `None` when the lock selects no clock configuration" (no `None` for several occurrences, no `clock_of` row), FR-522 still reads the clock from "the row of the declaration (FR-515 `clock()`)" with no occurrence, and FR-515 does not state the order of `occurrences()`. FR-515-AC-20 and FR-516-AC-32 defeat a position pairing only if `occurrences()` lists the twins in declaration order (7 then 3) while rows ascend by occurrence (3, 7) as QSpec FR-322 requires; with ascending-ordinal order both pairings coincide and the criteria cannot fail. State the order of `occurrences()` and what "listed in that order" lists. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:104,197 | missing-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-516 and FR-523 now return the request's own ledger and progress_ledger unchanged for every stop kind; FR-516-AC-31 and FR-523-AC-23 pin it. |
| FND-002 | fixed | clock_of keys the row by (declaration, role, ordinal), never position; occurrence selector with occurrence-required and occurrence-unknown refusals; FR-516-AC-32/33. Residue is FND-011. |
| FND-003 | fixed | IR-1279 gate removed from FR-515-AC-13 and Dependencies, replaced by QSpec FR-322; FR-515-AC-20 and FR-516-AC-32 use ordinals 7 and 3. FR-322 keys rows by (declaration, occurrence) and forbids selection by array position, so the text agrees; the AC strength depends on FND-011. |
| FND-004 | fixed | FR-516-AC-25 now covers formula_nodes, history_span, tick sums with incomplete naming the stop, and states the withdrawn-anchor case separately with incomplete None; the output row names clause stops. |
| FND-005 | fixed | A faulted instance is recorded as an Unavailable ledger entry without captures and is not counted; FR-524-AC-35 and AC-37 pin retention and redelivery. |
| FND-006 | fixed | Contradiction check over the whole list first, then surplus; surplus_capture_index is named; FR-524-AC-38. |
| FND-007 | fixed | Valuations, visited and a meter unit at temporal.visit are named, evaluation_steps and positions excluded; the new rule conflicts with the evaluation_steps definition, recorded as FND-010. |
| FND-008 | fixed | TC-1318 rows run 22 to 27 with unique numbers; rows map AC-20, 21, 22, 19 and 23. |
| FND-009 | fixed | Execution and trigger-record bullets in FR-516 and FR-524, request table maps the record completeness, FR-516-AC-34/35 and FR-524-AC-39. |
| FND-010 | still-open | medium: silent-sample charge point contradicts the evaluation_steps definition. |
| FND-011 | still-open | low: clock() table row, FR-522 clock read and occurrences() order not updated. |

## Verdict (disposition pass 1)

FND-001 to FND-009 are fixed. FND-010 (medium) is a new contradiction introduced by the FND-007 fix and blocks merge until the evaluation_steps definition or the charge point is amended; FND-011 is low. Verified: quire validate exit 0 on all 12 changed files and on this file; TC rows and tag ranges end at FR-515-AC-20, FR-516-AC-35, FR-523-AC-23, FR-524-AC-39 and match the AC ids; the new ids are unused on open PRs 2222, 2224, 2225 and 2226.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-012 | medium | FR-515 now says `occurrences()` lists a node's occurrences "in declaration (source) order" and FR-515-AC-20, FR-516-AC-32 and TC-1310 row 20 depend on 7 then 3 with lock rows 3 then 7. In the QSL writer a claim ordinal is a repeat count: `Spans::next_ordinal` gives the next counted occurrence last+1 from 0, so occurrences of one clause node are numbered 0, 1, 2 in source order and `temporal_binding_rows` writes `occurrence: temporal.claim().ordinal()`; source order and ascending ordinal coincide for every package QSL emits, so no QSL source produces 7 then 3 and the in-memory half of the criteria (the I2 package) cannot realize the twin. For a read-back package the view reads the node's `occurrences` array in wire order, which QSpec FR-322 does not constrain (it states ordinals are producer-assigned and a source-map occurrence order "in ascending ordinal, which is not a requirement"), and a read-back package does not carry source order, so "declaration (source) order" is not what a reader can compute. QSpec FR-322 rows ascend by occurrence and BO-P1 to BO-R9 use ordinals 0 to 2 only. Say `occurrences()` follows the order of the node's `occurrences` array, state that the twin fixture is a constructed package (and constructed in-memory package) with that array order 7, 3, and keep the 3, 7 row order. Otherwise a test over a QSL-emitted package passes under position pairing and the AC cannot fail. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:153-157,197 | wrong-requirement |
| FND-013 | low | FR-523 now defines `evaluation_steps` as "one charge per evaluated position visit" and says a silent-sample lookup "is not a visit", but FR-501's work rule charges `temporal.visit` "once per visit of a temporal node at a position" and FR-504 defines a silent sample as a fixed-sample watermark sample where every atom reads false; "evaluated position visit" is defined nowhere and the holds atom at a silent sample reads as a visit under FR-501's wording. Define it by FR-504 (an observed position versus a watermark sample with no observation). | spec/functional/FR-523-count-temporal-evidence-work-on-thirteen-named-counters-and-keep-a-stopped-instance-apart-from-a-stopped-declaration.md:90 | missing-requirement |

## Dispositions (round 2)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-010 | fixed | The `evaluation_steps` row now excludes a silent-sample lookup and FR-523-AC-15, FR-523-AC-21, FR-516-AC-27 and TC-1318 row 24 assert that meter usage at `temporal.visit` exceeds `evaluation_steps` by the number of silent lookups, which fails if a silent lookup is charged to `evaluation_steps`; counters stay thirteen. Residue is FND-013. |
| FND-011 | fixed | The `clock()` row, FR-522 and `occurrences()` order are stated. The order rule itself is wrong for what the writer and reader produce, recorded as FND-012. |
| FND-012 | still-open | medium: declaration (source) order equals ascending ordinal in every QSL-emitted package and is not computable from a read-back package; the twin AC can fail only on a constructed wire order. |
| FND-013 | still-open | low: "evaluated position visit" undefined and read against FR-501's per-position visit charge. |

## Verdict (disposition pass 2)

FND-001 to FND-011 are fixed. FND-012 (medium) blocks merge: restate the `occurrences()` order as the node's array order and name a constructed-package fixture. FND-013 is low. quire validate exits 0 on all 12 changed files and on this file; TC ranges end at FR-515-AC-20, FR-516-AC-35, FR-523-AC-23, FR-524-AC-39; no id collides with open PRs.

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-014 | medium | The "Evaluated position visit" definition is self-inconsistent for the position scan at a silent sample. Its first clause makes a visit a charge "made at a position that carries an observation of the trace, or at an observed position's false extension beyond the watermark's samples", which excludes a silent sample (a watermark sample with no observation, inside the watermark, so neither); its last sentence says the only charge that is not a visit is the `holds` lookup at a silent sample and that "the node-activation and scan charges ... are unchanged", which makes the scan of a silent position a visit. FR-523-AC-3 pins only observed positions (8 = 1 origin + 7). So whether a silent-sample scan charges `evaluation_steps` and `positions` is unspecified, the charge logs of FR-523-AC-10 and the exact meter ceilings of FR-523-AC-14 differ between readings, and FR-516-AC-27 and FR-523-AC-21 do not pin it. State that a scan (and a node activation) at a silent sample is, or is not, charged to `evaluation_steps` and `positions`, and add the count to FR-523-AC-21. | spec/functional/FR-523-count-temporal-evidence-work-on-thirteen-named-counters-and-keep-a-stopped-instance-apart-from-a-stopped-declaration.md:97-104 | missing-requirement |

## Dispositions (round 3)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-012 | fixed | `occurrences()` follows the node's `occurrences` array as carried; lock rows ascend; the twin AC in FR-515-AC-20, FR-516-AC-32, TC-1310 row 20 and TC-1311 row 32 is a constructed package, in memory and read back, with array order 7, 3 and rows 3, 7, and says QSL's emitter cannot produce it. Position pairing fails it, and FR-322 prose agrees. |
| FND-013 | fixed | FR-523 defines the evaluated position visit through FR-501-AC-12's charges and FR-504; the excluded charge is the `holds` lookup at a silent sample, consistent with FR-523-AC-3 (observed positions, 8 charges). Residue is FND-014. |
| FND-014 | still-open | medium: the charge of a position scan at a silent sample is contradicted between the definition's first clause and its last sentence. |

## Verdict (disposition pass 3)

FND-001 to FND-013 are fixed. FND-014 (medium) blocks merge: one sentence stating whether a scan and node activation at a silent sample charge `evaluation_steps` and `positions`, with the count in FR-523-AC-21. quire validate exits 0 on all 12 changed files and on this file; TC ranges unchanged (FR-515-AC-20, FR-516-AC-35, FR-523-AC-23, FR-524-AC-39); no new contradiction elsewhere.

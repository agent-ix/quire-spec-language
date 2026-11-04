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

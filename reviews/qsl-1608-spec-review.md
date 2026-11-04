---
id: SR-2232001
title: "spec-review of the package route for evaluate_temporal_evidence (quire-spec-language PR 2232)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-337, FR-515, FR-516; spec/test-cases/TC-1310, TC-1311, TC-1326"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
---

# Spec review of QSL-1608 package route (quire-spec-language PR 2232)

## Summary

Ticket: QSL-1608. One spec-review pass over `git diff origin/main...HEAD` at head f0ffaf62a183447d8524d71fe06892c9253b516b. Sub-analyses folded in as inline passes: integrity, criterion strength (no Jev client, so judged by hand), evidence, scope boundary.

Checked against FR-050, FR-337, FR-515, FR-516, FR-502, FR-255, merged #2227 (d26c890ad), main qsl-package/src/emit.rs and checked_views, and PR #2224 qsl-eval/src/temporal/evidence/{call,request,work}.rs and finite_window/progressed.rs. The author's central claim holds: the pre-emission CheckedPackage carries no clock rows (emit takes `temporal_selections` as an argument, emit.rs:5660) and FR-050 makes the clock an emission input, so replacing the #2227 wording is correct. FR-515-AC-18/19 and FR-516-AC-29/30 read correctly after the rewording. No id collision: FR-337-AC-23, FR-515-AC-21/22, FR-516-AC-36..41 are unused on main and on the heads of #2224, #2230, #2231. `quire validate` exits 0 on all six files and on `spec/**/*.md`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-516-AC-18 requires the Emission-constructed package and the read-back package to agree over "the twin package of AC-32", but that package is constructed because QSL's emitter cannot produce ordinals 7 then 3. `from_emission` takes only emitter output (FR-337), so no Emission exists for the twin. The criterion cannot be run as written. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:290 | wrong-requirement |
| FND-002 | high | FR-515-AC-22 says the derived set holds "exactly the atom indices of the three binder-reading atoms", but the clause has a guard (not a `holds` atom) and two capture-reading atoms, so only two atoms are instance-bound. The AC as written cannot pass. TC-1310 row 22 repeats the setup. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:231 | wrong-requirement |
| FND-003 | high | The `progress-ledger-limit` row makes a covered-position overflow an `Err(Admission)` refusal. FR-502 and FR-255 say the evaluator returns `Incomplete(LimitReached{limit, value, setting})` with the ledger unchanged (FR-502-AC-8). Two requirements contradict. Also AC-38 sets `progress.covered_positions` per call, but #2224 call.rs passes the constant DEFAULT_COVERED_POSITIONS and the request has no member for the setting. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:140 | wrong-requirement |
| FND-004 | medium | The refusal-cause set is not whole. The prose names `mismatched_dimension` as a cause, but it is a field; the cause in call.rs:241 is `binding-mismatch`. `activation-mapping-missing` and `activation-mapping-mismatch` (AC-16, FR-449) are in neither the table nor the exclusion list, so AC-41's "no request returns a cause outside the table and the FR-522, FR-524 and FR-526 causes" contradicts AC-16. AC-41 also lists `surplus-capture` as owned here while the prose assigns it to FR-524, and AC-40 repeats FR-524-AC-33. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:313 | wrong-requirement |
| FND-005 | medium | Not every new AC has a same-cause counterexample. AC-40 has no request with the exact declared capture count that is evaluated; AC-41 is a universal over causes with none; AC-39 has no readable clause that returns a result (only the different-cause overflow). | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:311-313 | missing-requirement |
| FND-006 | medium | Instance-bound derivation is under-specified. `binders()` holds `over`, trigger and capture binders; the subject view exposes `over()` and `captures()` but no trigger binder id. FR-515 does not say how the trigger binder is told from an `over` binder, nor that an atom reading only `over` is not instance-bound. AC-22 has no atom reading only `over`. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:197 | missing-requirement |
| FND-007 | medium | The read-back route's evaluator formula is not specified. `formula()` is a pre-order list of `{operation, interval}` with no operand edges, yet `clause-unreadable` names a cycle or an operand naming no node, and AC-18 requires byte equality of results. The spec does not say which package data gives the evaluator its formula in each route, or that both routes build the same one. A coder must invent it. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:178-200 | missing-requirement |
| FND-008 | medium | Stale in-memory wording remains. TC-1311 row 29 still reads "the in-memory package of the I2 write before emit"; FR-515-AC-20, FR-516-AC-32 and TC rows 20 and 32 say "in memory" without naming the Emission-constructed package. | spec/test-cases/TC-1311-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:49 | wrong-requirement |
| FND-009 | low | `progress-inconsistent` says a fixed-sample watermark or a `Complete{through}` below the last coordinate; progressed.rs refuses a `through` below the last coordinate under event-position too. AC-37 tests fixed-sample only. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:139 | missing-requirement |
| FND-010 | low | Five ACs end "Gated on QSL-1608 until the code lands": a ticket id and a delivery state inside normative criteria, and no Dependencies entry. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:227-231 | missing-requirement |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-011 | high | Widening FR-515 `formula()` to every formula node (connectives, constants, `holds`) falsifies merged FR-527, which states that `formula()` "lists the `quire.op.temporal.*` applications only" and that FR-515 "lists only the operators of the clause"; the PR does not touch FR-527, so the classifier's stated inputs now contradict the view it reads. | spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md:100-106 | wrong-requirement |
| FND-012 | low | FR-515-AC-22 says swapping the first and second atoms "moves the two indices"; with {0,3} the swap gives {1,3}, so one index moves and one stays. | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:242 | wrong-requirement |
| FND-013 | low | The cause table says `binding-mismatch` is owned by FR-522 and `delivery-contradiction` and `surplus-capture` by FR-524; those requirements name the refusals by catalog code (`invalid_runtime_input`/`invalid-value`, `contradicted_identity`, `surplus_capture_identity`) and never use these cause names. The names exist only in FR-516 and in call.rs. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:124-129 | wrong-requirement |
| FND-014 | low | The progress-ledger capacity stop is added to the Incomplete prose but not to the `incomplete` member row (FR-516 outputs), the stop list of FR-516-AC-25 or the "each stop kind of AC-25" of AC-31. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:111 | missing-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | AC-18 covers R's three clauses; AC-20 and AC-32 make the twin read-back only |
| FND-002 | fixed | AC-22 has four holds atoms and a guard; hand count gives two |
| FND-003 | fixed | Ok Incomplete(LimitReached) with ledgers unchanged, matches FR-502-AC-8 and FR-255 |
| FND-004 | fixed | table now whole with activation-mapping rows; cross-references listed |
| FND-005 | fixed | AC-39, AC-40, AC-41 each carry a met and unmet pair |
| FND-006 | fixed | trigger from activation() Each, captures(), over() excluded |
| FND-007 | fixed | one shared construction and AC-23; see FND-011 for the side effect |
| FND-008 | fixed | stale in-memory wording removed |
| FND-009 | fixed | AC-37 covers both sample profiles |
| FND-010 | fixed | gating moved to Dependencies Delivery entries |

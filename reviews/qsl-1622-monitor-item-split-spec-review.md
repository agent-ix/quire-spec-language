---
id: SR-2266001
title: "spec-review of quire-spec-language PR 2266 (QSL-1622 monitor item split)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md; spec/functional/FR-283-monitor-a-supplied-trace-offline.md; spec/test-cases/TC-770-library-outcomes-serialize-to-one-outcome-document-with-the-undefined-label-on-non-proof-outcomes-only.md; spec/test-cases/TC-765-monitor-reports-violations-pending-and-tested-verdicts-over-finite.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-283
    type: references
---

# Spec review of quire-spec-language PR 2266

## Summary

Ticket: QSL-1622. Head 82d848b89dffb235be3efab837b4b48e4f01890e. Verified: FR-286-AC-8 covers the clause-run producer only; AC-13 is delivered by the ticket that builds the monitor operation, named only in the Delivery line; the source runs FR-283-AC-1 (violated at position 2, three-position trace, witness), AC-5 (x is 0 at position 1, division-by-zero, no witness) and AC-11 (InsufficientData at 0, count 1, min_count 2, exit 10) produce the cases AC-10 to AC-12 state; the refuted cause rule matches FR-331's wire table (only undefined-evaluation); the undefined label only for the undefined category matches FR-283, FR-285 and FR-286 Behavior; no ticket number in any AC; AC-10 to AC-13 and FR-283-AC-14 are unused on main and on the open PR heads. quire validate exit 0 on the four changed files and on spec/** plus plan/**.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-10 requires the finite-trace counterexample to have "an empty `loop`", but FR-331 refuses a packet whose loop is present and empty (invalid_runtime_input/invalid-value); a finite trace's counterexample is read by QSpec FR-364's observed arm with the loop absent or null. State the spelling FR-364 gives a finite trace (absent/null) or say the item carries no `loop`, so the AC cannot require a shape FR-331 refuses. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md | missing-requirement |
| FND-002 | low | FR-283's Indeterminate verdict row lists only "the FR-501 reason" as members, while the new FR-286 member table gives the item `position` where FR-501's reason names one; align the FR-283 row (members: reason and position where it names one). | spec/functional/FR-283-monitor-a-supplied-trace-offline.md | missing-requirement |
| FND-003 | low | FR-286-AC-13 covers the runs of AC-8 to AC-12 (pending, indeterminate, violated, undefined, insufficient data) but FR-283-AC-14 and TC-765 step 10 cover only AC-1, AC-5 and AC-11, and TC-770 step 10 and TC-765 step 10 assert the same equality; name the same run set in both ACs or make one the owner. Steps 9 and 10 in TC-765 and TC-770 are also separated from the list by a blank line. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md | missing-requirement |
| FND-004 | low | The member table is prose with no Behavior `shall` statement, and FR-330 does not say ClauseRunReport holds the FR-351 witness (FR-283 says only that the evaluator retains it through FR-267's witness channel), so the source from which `from_clause_run` reads `witness` is unspecified. Add a Behavior bullet and name the report member. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md | missing-requirement |

## Verdict

Mergeable once FND-001 is resolved; FND-002 to FND-004 are low.

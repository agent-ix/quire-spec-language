---
id: SR-16217
title: "spec-review of the monitor trace document and MonitorOutcome record (quire-spec-language PR 1336)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@35115c7999e8269d9c43916e64bb443f7240957c; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md; spec/functional/FR-283-monitor-a-supplied-trace-offline.md; spec/test-cases/TC-765-monitor-reports-violations-pending-and-tested-verdicts-over-finite.md; spec/test-cases/TC-766-monitor-refuses-inadmissible-traces-and-agrees-with-replay-s-trace.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-283
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: references
---

# Spec review of the monitor trace document and MonitorOutcome record (quire-spec-language PR 1336)

## Summary

Ticket: STD-371 (QSL-597). This is one spec-review pass over
`git diff origin/main...35115c799`: FR-283's `monitor` signature,
`MonitorRequest`, `MonitorClause` and `MonitorLimits`, the
`quire.monitor-trace/1` trace document and its ordered defect list,
`MonitorOutcome`, `MonitorRecord` and the `MonitorVerdict` table,
`MonitorRefusal`, the Behavior changes, ADR-029 OP-3, and the TC-765 and
TC-766 steps. The oracles were STD-371's gap list, FR-106, FR-267,
FR-285, FR-286, FR-327 to FR-330, QSpec FR-351 (the separating witness
record), QSpec FR-364 (the observed counterexample arm) and open PR #1340
(STD-378, insufficient data).

Gates: `QSPEC_DIR=~/dev/quire-specification make check-spec-validation`
at 35115c799 exits 0 with no warning on a touched file. origin/main
(c21bc57a) is an ancestor of the head. FR-283-AC-7 to FR-283-AC-9 appear in
no other open PR.

Examined, clean:

- **Dropping "and an invocation" from each trace position removes no
  capability.** FR-327's Inputs read a `TemporalTrace` of FR-106-admitted
  observations, one per position, and nothing reads an invocation. FR-330's
  `TraceInput` names positions as snapshot `DocumentRef`s. QSpec FR-364's
  observed arm records each step as `{document}` alone. Fairness, the only
  construct that would need step information, already settles
  `missing-fairness-premise` over a supplied trace (QSpec FR-362).
- **The trace document.** It has a closed member set, the FR-329 lasso
  shape (`n >= 0`, `L >= 1`), the FR-261 shared reader under
  `ObservationLimits`, and defect codes and pointers reused from FR-106.
  AC-7 and AC-8 are testable.
- **`MonitorRefusal`** separates a document defect from a position's
  admission failure. Neither holds a clause record, as FR-283-AC-3
  requires.
- **`Completed(true)` over a finite trace reports `Tested`** (lead
  default 4). AC-9's bounded `eventually[0,2]` case covers it.
- **The lead defaults are flagged** in the PR body as "lead default, not an
  owner answer".

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Scope-narrowing. A violation's evidence loses QSpec FR-351's deciding element and value path. On main, FR-283 and ADR-029 OP-3 give a finite-trace violation "the QSpec FR-351 separating witness". QSpec FR-351 says that one record shape "backs every family's decisive settlement", and that a decisive temporal counterexample holds "only with ... this record". The record carries the deciding element, the FR-207 value path and the trace position. The PR replaces it with FR-330's `TemporalCounterexample` (QSpec FR-364 observed arm), which carries the trace digests and `trace_position` only, and keeps only the trace position ("QSpec FR-351's trace position is its `position`"). `monitor` no longer reports which value at which path made the clause false. Fix: keep the FR-351 record on `Violated` beside `counterexample`. Otherwise show where the deciding element and value path are carried. | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:155, :167-171; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:389 | wrong-requirement |
| FND-002 | medium | The `InsufficientData` row contradicts the text below it and open PR #1340. The row gives category `undefined`, exit 10. FR-285 requires a category-`undefined` item to keep the label `undefined`, and #1340 renders it `{"kind": "undefined", "reason": "insufficient-data"}`. Yet FR-283 says "a clause record never carries the label `undefined` (FR-286)". The row also maps the evaluator result "`Undefined` with `insufficient-data`", while #1340 makes `InsufficientData` FR-327's own outcome, separate from `Undefined`. Fix: map the row to #1340's `InsufficientData` result, and limit the label sentence to undefined evaluations. | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:157, :181-182 | wrong-requirement |
| FND-003 | medium | No test covers five new verdict rows or the internal-failure path. `InsufficientData`, `Incomplete`, `WindowLimitReached`, `ConditionRefused` and `TraceRefused`, each with members, a category and an exit, have no AC and no TC step. Neither has the internal-failure rule (an evaluator fault, `TemporalRefusal::Fault` or a refused `CheckedInvariant` fails the whole request with `StageFailure::Fault`). Fix: add ACs, with TC-765 or TC-766 steps, for each row and for the fault rule. | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:157-164, :174-176 | correct-requirement-no-evidence |
| FND-004 | medium | The defect order is ambiguous for a document with no `format`, and two AC-8 cases have no expected code. A missing `format` matches step 3 ("a `format` other than the string `quire.monitor-trace/1`": `unknown_wire`/`unsupported-wire`) and step 4 ("a missing `format`": `missing-member`), so two implementers would report different codes. AC-8 does not test it. AC-8 also gives no code or pointer for bytes `[` and for `[]`, though TC-766 step 5 expects "the code and pointer FR-283-AC-8 gives". Fix: make step 3 apply only to a present `format`, add a missing-`format` case, and give `[` (`invalid-value` at `""`) and `[]` (`wrong-value-kind` at `""`) in AC-8. | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:119-122, :240 | wrong-requirement |

## Verdict

Not mergeable. FND-001 is a scope-narrowing under the standing rule: it
removes the FR-351 deciding element and value path from a monitor
violation without replacing them. FND-002 to FND-004 are a label and
category contradiction (and a mismatch with #1340), five untested rows, and
an ambiguous defect order. Dropping the per-position invocation is correct
and loses nothing, and the document format and refusals are sound.

---
id: SR-12886
title: "integrity review of STD-163 FR-194 undefined window positions"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@15eca179893ce3d5c95742758a2476f907133f4d; Ticket STD-163; PR #979; spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md, spec/test-cases/TC-629-window-aggregates-in-monitors.md; context: spec/decisions/ADR-024-statistical-and-probabilistic-properties.md, spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md, agent-ix/quire-specification@967ee5a spec/functional/temporal/FR-414-evaluate-window-aggregates.md"
review_set: subset
---

## Summary

Ticket: STD-163. A consistency check of the amended FR-194 against the
decisions it depends on (ADR-024 WA-4/WA-5, ADR-014 TR-5) and against QSpec
FR-414, which FR-194's References name as the owner of aggregate semantics and
`InsufficientData`. The ticket names "Owners: FR-414, FR-194".

## Verdict

CONDITIONAL. FR-194 is internally consistent, but four statements elsewhere
still say "per value visited", and the owning QSpec FR-414 has no
undefined-position rule.

## Examined Scope

```yaml
scope:
  - {id: FR-194, path: spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md, role: examined, excerpt: "Evaluation SHALL charge one unit of ADR-014 TR-5 work per window position visited, whether or not the filter admits the position."}
  - {id: ADR-024, path: spec/decisions/ADR-024-statistical-and-probabilistic-properties.md, role: examined, excerpt: "WA-5 ... Evaluation charges TR-5 work per value visited."}
  - {id: ADR-014, path: spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md, role: examined, excerpt: "TR-5 ... Amended by ADR-024 WA-5: an aggregate atom also charges one unit per window value it visits"}
  - {id: FR-414, path: "quire-specification:spec/functional/temporal/FR-414-evaluate-window-aggregates.md", role: examined, excerpt: "A Boolean value of each aggregate atom at each position, or the kernel's no value with cause insufficient-data{count, min_count} when the window lacks data, or incomplete, limit-reached naming the limit. ... Evaluation charges one unit of clause work per value visited."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSpec FR-414 owns the aggregate semantics, and STD-163 names it as an owner, but it is unchanged. Its Outputs list only Boolean, `insufficient-data` or `limit-reached`, which leaves no room for a passed-through undefined cause. It has no rule against skipping an undefined position, and line 87 still says "one unit of clause work per value visited". A second implementation that conforms to QSpec could skip undefined positions. Amend FR-414 (and TC-366) to match, in this ticket or a tracked STD follow-up linked from STD-163. | quire-specification:spec/functional/temporal/FR-414-evaluate-window-aggregates.md:34-35,87 |
| FND-002 | medium | QSL's own decisions still say "per value": ADR-024 WA-5 ("charges TR-5 work per value visited"), ADR-024's amendments list ("aggregate atoms charge per value visited (WA-5)") and ADR-014 TR-5 ("one unit per window value it visits"). FR-194 depends_on both ADRs, so a reader of the ADRs charges the filtered count. Update all three to "per window position visited". | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:154,423; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:174 |
| FND-003 | low | ADR-024 WA-4 and FR-414-AC-3 say an `InsufficientData` atom is "never UndefinedEvaluation". The new FR-194 cause is an ordinary kernel undefined, which the temporal engine reports as `UndefinedEvaluation{cause}` (#966 `outcome.rs`). That reading is consistent, but neither ADR-024 nor FR-194 says it outright. One clause naming the undefined-position outcome category would remove the doubt. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:104-110 |

## Evidence and Limits

`git grep "value visited"` over QSL `spec/` and QSpec `origin/main` found
these hits and no others. QSpec was read only.

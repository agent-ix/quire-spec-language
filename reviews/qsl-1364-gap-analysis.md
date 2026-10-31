---
id: SR-1939005
title: QSL-1364 scoped H3 gap analysis
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1364; PR1945 frozen 04ef7ec205858543a8a76b0f2f839813e7a5dfb3; origin/main...HEAD; qsl-eval/src/value/expression/mod.rs and qsl-eval/tests/it/named_population.rs; TC-136 H3 only, FR-049-AC-1 context"
review_set: subset
relationships:
  - {target: ix://agent-ix/quire-spec-language/TC-136, type: reviews}
  - {target: ix://agent-ix/quire-spec-language/FR-049, type: references}
---

## Summary

Ticket: QSL-1364. Manual bounded requirement/test/source reconciliation for the rejected holds-node identity. This is a scoped delta assessment, not a whole-repository assurance audit. The owning TC-136 H3 already requires the changed behavior; no new specification cycle is needed.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

PASS for TC-136 H3 in this delta: both prescribed invalid handle categories now produce an exact named-node Fault, exclude Evaluation and preserve the meter. The valid holds control remains successful. This does not close all of FR-049-AC-1, QSL-1255 or predecessor SR-1939003; no predecessor artifact or disposition was edited.

## Coverage

Plan completion: not assessed

Reconciliation method: manual, as explicitly authorized in dispatch. No full matrix was run: the lead reports quire 0.37.1 / engine 0.50.3 hangs on the full repository; this reviewer did not reproduce that hang. No computed matrix result, evidence-store reconciliation, repository coverage percentage or whole-repository reverse-gap/stub completeness is claimed. The installed native CLI versions resolve to quoin 0.29.1 and quire 0.37.1 / engine 0.50.3. Scratch SpecReviews are validated at their explicit paths.

TC-136 is the existing test's literal trace tag; its H3 prose owns both invalid cases and its following statement requires typed field and meter assertions. Every H3 outcome has an executable nonignored assertion through the real evaluate_holds API. The changed production branch implements that same contract using the existing shared InternalFault detail API; no unspecified behavior or new stub is introduced. FR-049-AC-1 is context only, with broader raw/wire/foreign handle obligations outside this dispatch. The PR does not add a full-criterion trace tag or claim those obligations complete.

Reviewer independently ran the nine named_population tests: 9 passed, 0 failed (/tmp/qsl1364-reviewer-green.log); coder green and old-production identity controls were inspected (/tmp/qsl1364-final-green.log, /tmp/qsl1364-red-non-holds.log, /tmp/qsl1364-red-out-of-range.log). The old implementation yields [] against expected node 0 and node 3, demonstrating that both identity oracles detect the repaired omission. Review does not infer execution of the second loop case from the first old-code failure; the separate out-of-range control provides that evidence.

Optional separate semantic-analysis pass: not run; required code-review checks assertion intent. Spec-review and its subanalyses are inapplicable because no spec or plan changed. Aggregate checks were excluded by explicit dispatch due to controlled current-main failures outside the delta. No applicable AssuranceProfile was found. Existing QSL-1363 and QSL-1365 remain outside scope.

## Scope

```yaml
scope:
  - id: TC-136-H3
    path: spec/test-cases/TC-136-admit-composed-state-views.md
    role: examined
    excerpt: "| H3 | `evaluate_holds` | a formula node id that is no `holds` atom of the clause, and an id beyond the clause's node count | an `Err` fault naming the node; no `Evaluation` |"
  - id: TC-136-handle-assertions
    path: spec/test-cases/TC-136-admit-composed-state-views.md
    role: examined
    excerpt: "Each row asserts the typed refusal or fault as a field and the unchanged meter, never message text."
  - id: FR-049-AC-1
    path: spec/functional/FR-049-admit-composed-evaluation-inputs.md
    role: context_only
    excerpt: "Each checked-term evaluation selects an exact checked declaration/expression identity; raw bytes, freely constructed wire records and a foreign or out-of-range checked handle cannot enter an evaluation session."
bindings:
  - {test_id: "named_population::invalid_holds_handles_fault_without_spending_the_meter", ac_id: TC-136, trace: correct}
```

The H3 and handle-assertions ids above are local scope labels within TC-136, not additional minted artifacts.

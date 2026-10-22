---
id: SR-14051
title: "EARS review of STD-239 FR-223 and FR-437 edits (PR #1192, QSpec PR #331)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@39f1130926a6da9993c49a56fef0365d3a7eecb4; spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md; agent-ix/quire-specification@4010594880b59e0154cdc9f7e0a86d0aca18339b; spec/functional/memory/FR-437-explore-release-acquire-memory.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
---
# SR-14051: EARS review of STD-239 (PR #1192, QSpec PR #331)

## Summary

Ticket: STD-239. Statements examined:

- the FR-223 description, as changed;
- FR-223's two new behaviour bullets;
- FR-437's changed Description context and its new behaviour bullets.

The FR-437 bullets are descriptive rules under a SHALL description, not
separate SHALL statements, and are not EARS units. `make
check-spec-validation` reports no EARS diagnostic on FR-223.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Two of FR-223's new SHALL clauses do not name the system as their subject, and one carries its rationale inside the requirement. "A fence's `eco` clause SHALL build no edge, since the join of `S` ... already excludes the choices" makes a clause the subject, and its `since` clause is rationale. "The omission SHALL only enlarge the set of explored behaviours" puts a SHALL on a consequence, not on a behaviour `Ra` performs. Restate both with `Ra` as the subject ("`Ra` SHALL build no edge for the `eco` part of RC11's fence clause"), move the reason to the rationale, and make the enlargement a property that FR-223-AC-5 and FR-437-AC-6 verify. | spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:59-65 |

## Verdict

One low finding. The description and the `Ra SHALL NOT build` bullet are
well-formed ubiquitous statements with `Ra` as their subject.

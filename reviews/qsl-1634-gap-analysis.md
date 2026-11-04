---
id: SR-2288002
title: "Gap analysis of QSL-1634 temporal decisive information (quire-spec-language PR 2288)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-replay/src/spine/clause/temporal.rs; qsl-replay/src/spine/clause/tests/temporal.rs; FR-283-AC-10; FR-330-AC-12; FR-286-AC-10"
review_set: subset
evaluated_revision: "997eedb4f737c555e66544546924324f2a80159b"
---

## Summary

Ticket: QSL-1634. Planless gap analysis scoped to PR 2288's diff. Plan completion: not
assessed. Criteria examined: FR-283-AC-10, FR-330-AC-12, FR-286-AC-10. One new test,
`a_temporal_violation_carries_its_decisive_information`, is tagged with all three.

## Verdict

CONDITIONAL. The test proves the forall and the undecided shapes for `on each
always[0,0]` clauses, a held run, and that FR-267's record holds no `decisive`. It does
not exercise FR-330-AC-12's `Bounded` case or its pending and `Undefined` cases, and
FR-283-AC-10 / FR-286-AC-10's `always p` at position 2 cannot be met under FR-328
(see SR-2288001 FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-330-AC-12 is tagged but its `Bounded` violation (position 0, undecided decisive) and its pending and Undefined reports carrying no decisive are untested; only a held run is checked | qsl-replay/src/spine/clause/tests/temporal.rs:3769 |
| FND-002 | medium | The test is tagged FR-283-AC-10, FR-286-AC-10 and TC-765, but it runs `on each always[0,0]` clauses through run_clause, not AC-10's `always p` (position 2) or the stored `items` collection, so the tags claim criteria the test does not prove; FR-330-AC-12 belongs to TC-840 | qsl-replay/src/spine/clause/tests/temporal.rs:3761-3769 |
| FND-003 | low | The new lasso slot mapping and the Progressed/Timestamped branches of decisive_information have no test | qsl-replay/src/spine/clause/temporal.rs:1120-1136 |

## Fixes

- FND-001: add `Bounded` over 0, 1, 2 (expect `decisive` undecided, element `false`,
  root path), `Reaches` over Finite (pending, no `decisive`) and an `Undefined` run
  (FR-330-AC-5's clause, no `decisive`) to the test. All fixtures exist.
- FND-002: tag the test `TC-840` and `FR-330-AC-12`; drop `FR-283-AC-10` and
  `FR-286-AC-10` until the spec ticket from SR-2288001 FND-002 settles their clause and
  position.
- FND-003: one lasso run whose violation lands in the loop.

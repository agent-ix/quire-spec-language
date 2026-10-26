---
id: SR-60030
title: "gap analysis of QSL-1099 component certificate producer and checker (quire-spec-language PR 1603)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3414559bd129af5c60dfe512bdbbe7a353031f59; spec/functional/FR-339-check-an-en-1-component-certificate.md, FR-126 (AC-13, AC-18), spec/test-cases/{TC-526,TC-521,TC-1085,TC-1215,TC-1141}.md, qsl-analyze/tests/it/component_certificate.rs"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
---
## Summary

Ticket: QSL-1099. Tags checked against each AC's whole text. Plan completion: not assessed (planless).

- FR-339-AC-1, AC-2, AC-4, AC-5, AC-6 and FR-126-AC-13: tagged on tests that assert the criterion's vectors; AC-2's four vectors, AC-6's exact-limit, one-below, meter and cancel vectors all present.
- FR-126-AC-18 is tagged on two tests but its text also requires "FR-130-AC-5's entry lists acq(1) before acq(2)"; that clause is asserted by neither (FND-001), and "each binding's components in the order the rule computes" is asserted only through constraint indices on the example unit and the Fork unit's hand-derived order.
- FR-339-AC-3 stays on the qsl-replay scc.rs unit tests; FR-339-AC-7 and FR-228-AC-2 are QSL-1060's and not tagged here: honest.

Producer versus independent oracle: Fork and Ring expectations (component order, witnesses, state and edge counts) are hand-derived from the spec, and the Fork phases are matched against model digests taken from the subject system, so ordering is not checked by the producer against itself. The constraint numbering is shared: both sides call SubjectSystem::clause_constraints over the union of transitions, so an ordering bug in it is invisible to every accepted-vector test (FND-002).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | FR-126-AC-18's last sentence (FR-130-AC-5 lists acq(1) before acq(2)) has no assertion in the tests tagged AC-18, so the whole-AC tag overstates. Tag only when asserted, or split the sentence into its own AC. | qsl-analyze/tests/it/component_certificate.rs | correct-requirement-no-evidence |
| FND-002 | low | No test names the identity a UnfairWeak index resolves to independently of the shared clause_constraints (AC-1 says upd(b); the tests assert indices 0 and 1 and a flip to the other index). A swapped numbering in clause_constraints would keep every accepted vector green. Add one assertion of the constraint's operation and object identity. | qsl-eval/src/simulation/subject.rs, qsl-analyze/tests/it/component_certificate.rs | correct-requirement-no-evidence |
| FND-003 | medium | No test covers a Closure certificate for a TP-4 item being refused at settlement (see SR-60029 FND-001); FR-126's SHALL NOT settle Holds without the ComponentCertificate has no verifying row. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | correct-requirement-no-evidence |

## Verdict

PASS with findings on tag precision; FND-003 follows SR-60029 FND-001.

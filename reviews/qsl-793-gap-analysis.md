---
id: SR-11771
title: "Gap analysis of quire-spec-language PR #916 against FR-081-AC-17..21 and TC-1049"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@741ec99c0afe8a37acb81e4946a96c39a1bd7e91; PR #916 diff against origin/main; spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md; spec/test-cases/TC-1049-view-exposes-state-machine-states-transitions-and-conformance.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: reviews
---
# Gap analysis of quire-spec-language PR #916

## Summary

Ticket: QSL-793. Planless gap analysis of the PR diff against FR-081-AC-17,
AC-18, AC-19, AC-20, AC-21 and TC-1049 steps 2-4 and 6 (steps 1 and 5 via
M19 runs 1 and 7). Plan completion: not assessed. QSpec cited by id only
(FR-208, TC-235 M19).

Examined (each tagged and asserted):
- FR-081-AC-17: `tc235_m19_state_machine_specialization_vectors` run 1, exact
  ordered states and transitions with declaring key and position, authored
  `from`/`to`/`trigger`/`guard`/`emits`.
- FR-081-AC-18: `tc1049_view_exposes_machine_scope_and_conformance_by_key`,
  all seven `conforms` pairs including the `ExpressLifecycleCopy` display-name
  cases, identity/declarations/type_identities equality and the
  no-state-or-transition scan.
- FR-081-AC-19: M19 run 7 (own guard) and runs 2 and 4 (refusal, no view).
- FR-081-AC-20: tc1049, `state_machine(OrderLifecycle)` own entries only,
  `None` for `Order` and an absent key.
- FR-081-AC-21: M19 run 1 and tc1049, `OrderLifecycle [0]` guard
  `NotYetPlaced`, emits `OrderPlaced`.
- TC-1049 steps 2, 3, 4, 6.

No untraced production code: both new public methods own FR-081 / FR-082
behaviour; `conforms_admitted` is their shared helper.

## Verdict

PASS. Every examined AC has a tagged test whose assertion can fail. Only
residual: ordering by key bytes and a subtype-first walk coincide in the
fixture (see SR-11770 Verdict); recorded, not blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

---
id: SR-12886
title: "scope-boundary review of STD-165 state clause names a population (PR #982)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; spec/functional/FR-391-name-a-population-in-a-state-clause.md; quire-specification proposals/quire-v1/shared-grammar.md, spec/functional/type-model/FR-153-query-closed-model-environments.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: reviews
---
# SR-12886: scope-boundary review of STD-165 (PR #982)

## Summary

Ticket: STD-165. Checked the QSL/QSpec boundary. Syntax needs no QSpec
change, as the author claims: `primary` derives a call-less `qualified-name`
(shared-grammar.md:350-353). Semantics is a different matter: FR-391 adds a
QSL-local name-resolution rule that QSpec does not state.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-391 extends QSpec semantics without saying so. QSpec FR-153 requires `p` to be a population binding, "package typed binding role `population`", supplied with the request. shared-grammar.md says the `Population` spellings never "supply an ambient binding", and that "`M::Order` as a value" is "not an implicit population". FR-391 resolves a `qualified-name` primary to a domain population whose binding comes from the observation, a rule with no QSpec counterpart. QSpec's own FR-383-AC-13 still writes `forall o in counters`. Without a QSpec ticket or an explicit "QSL-local resolution; QSpec conformance vectors unaffected" statement, QSpec and QSL can diverge on what a clause may name. "QSpec's grammar is unchanged" is true only of syntax. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:33-41; quire-specification proposals/quire-v1/shared-grammar.md:450-452,906,1078-1082; quire-specification spec/functional/type-model/FR-153-query-closed-model-environments.md:23-26,96-98 |

## Verdict

Mergeable on this lens once the boundary is stated. The fix is a sentence
plus a QSpec follow-up ticket; the design needs no rework. Examined and clean:
the "outside a state clause body, unchanged" rule (AC-3's last case), which
keeps functions on explicit `Population<T>[N]` parameters as QSpec requires.

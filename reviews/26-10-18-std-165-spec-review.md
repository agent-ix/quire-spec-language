---
id: SR-12884
title: "base review of STD-165 state clause names a population (PR #982)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-391-name-a-population-in-a-state-clause.md, spec/test-cases/TC-1076-a-state-clause-names-a-population.md, spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md, spec/functional/FR-104-check-state-clauses.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1076
    type: reviews
---
# SR-12884: base review of STD-165 (PR #982)

## Summary

Ticket: STD-165. Base checklist over the diff `origin/main...bf4ca90b4`: the
new FR-391 (6 ACs), TC-1076 (7 rows), the FR-155-AC-2 respelling and new
`depends_on FR-391`, the FR-104 `references` edge and bullet, and the spec
index row. IDs are well formed and collision-free: FR-391 and TC-1076 exist
on no `origin/main` path and in no open PR's file list (TC-1075 is the
current maximum); the FR-391 mentions in FR-167/FR-168/FR-170/ADR-022 are all
qualified "QSpec FR-391" and do not collide. The QSpec grammar claim holds:
complete-V1 `primary` derives `qualified-name, [ '(' ... ')' ]` so `M::P`
parses with no call, and shared-grammar.md states no implicit
`allInstances()` (line 1080) and "`M::Order` as a value ... not an implicit
population" (line 906). FR-155-AC-2's footprint result (`AnyField{v}`,
`AnyMembership{counters}`) agrees with QSpec FR-383 ("a quantification ...
over a population P, or allInstances ... reads AnyMembership{P}"; AC-13) and
is state-free because `P` is static. Findings below are in traceability and
TC oracle strength.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-391-AC-6 names `Test (TC-574)`, but TC-574 verifies only FR-155 (its frontmatter has one `verifies` edge, to FR-155, and its scope is "FR-155-AC-1 to FR-155-AC-2"), and the PR does not touch TC-574. The matrix will show FR-391-AC-6 bound to a TC that does not claim it. AC-6 also only restates FR-155-AC-2's clause. Drop AC-6, or add FR-391 and FR-391-AC-6 to TC-574. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:123; spec/test-cases/TC-574-*.md:5-15 |
| FND-002 | medium | The second case of FR-391-AC-4 and TC-1076 row 6 cannot be built from the stated fixture. The fixture has `counters` and `archive` both of `Counter` and a clause `on M::Counter`, and that is the ambiguous case. "Only `archive` covers the context type" needs a different context type, or a `counters` whose member type does not cover it. Neither is given, so two implementers write different fixtures and the oracle is not reproducible. Name the context type and both member types. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:121; spec/test-cases/TC-1076-a-state-clause-names-a-population.md:28 |
| FND-003 | low | TC-1076 has two weak oracles. Row 4's last case expects "an unresolved-name refusal" and AC-3 says "refuses as an unresolved name did before"; neither names the code or locus. Row 7's last case expects "FR-084's typed incomplete result", but FR-084 names it (`FamilyResult::Undefined`, reason `incomplete-closure`, naming the queried type). Spell out both. | spec/test-cases/TC-1076-a-state-clause-names-a-population.md:26,29; spec/functional/FR-391-name-a-population-in-a-state-clause.md:120,122 |
| FND-004 | low | The Description says a state clause "has no parameters of that kind". On main, the checker admits `allInstances`/`lookup` in a state clause whose operation declares a `Population<T>[N]` parameter (`qsl-eval/src/value/expression/causes.rs:118-124`). Such a parameter can only never be supplied through an invocation document. The rationale should say that. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:28-30 |

## Verdict

This lens has two medium findings and no high finding. The PR is still not
mergeable, because of the high findings in SR-12885. Examined and clean: FR-391-AC-1, AC-2, AC-3 typing rows, AC-5's
first two cases, the FR-155-AC-2 respelling (the footprint is right under
FR-383 and PR #967's derivation for a one-population fixture), the FR-104
bullet and `references` edge (no `depends_on` cycle: FR-391 depends_on
FR-104, FR-104 only references FR-391), and the spec index row.

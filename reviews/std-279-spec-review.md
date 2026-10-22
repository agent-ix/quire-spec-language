---
id: SR-15990
title: "Spec review of quire-spec-language PR #1251: STD-279, the region of a frame node's generated occurrence"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2c1d98fd1e61f11982b09ee3a717864575b6adb6; PR #1251 (git diff origin/main...2c1d98fd1): spec/functional/FR-105-emit-state-nodes.md, spec/functional/FR-104-check-state-clauses.md, spec/test-cases/TC-462-s4-emits-state-nodes.md; read for context: QSL FR-062-AC-9 (UnlocatedOccurrence), FR-104 Requirements, FR-114 attempt syntax, FR-105-AC-1, TC-1043 fixture R; QSpec FR-322 generated occurrence region; agent-ix/quire-specification main 5ee0071f"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-462
    type: reviews
---
# SR-15990: Spec review of quire-spec-language PR #1251: STD-279, the region of a frame node's generated occurrence

## Summary

Ticket: STD-279. Sub-analyses folded in: integrity, EARS, criterion strength (by hand: no Jev client), failure-domain. The head 2c1d98fd1 is behind main 423ac6aa0 and merges cleanly. `make check-spec-validation` on the merge exits 0. The FR-104 line-43 EARS warnings are from before this PR. FR-105-AC-9 is new. #1245 (head c63718200) no longer touches FR-105: it changes FR-115, FR-121 and TC-1162 only, and a merge of main, #1245 and #1251 is clean. No other open PR touches FR-104, FR-105 or TC-462. The rule is the ticket's lead default: one region, the qualified operation reference in the first clause or attempt in source order, and the emitter keeps refusing a regionless generated occurrence (FR-062-AC-9). Clauses (`post ... on Config::ConfigVersion::attemptUpdate`) and attempts (`attempt ... on M::T::op`) both carry a qualified operation name, so 'the operation reference' has one meaning. The FR-104 hook matches the FR-105 Occurrences cell, and the region-follows-source-order sentence does not disturb the source-order-independent key. The moved-region vector catches an implementation that takes the last naming clause, and the unchanged-key half catches one that reorders ordinals.

Units examined:

- `FR-105-AC-9` (examined): spec/functional/FR-105-emit-state-nodes.md
- `FR-105` (examined): spec/functional/FR-105-emit-state-nodes.md
- `FR-104` (examined): spec/functional/FR-104-check-state-clauses.md
- `TC-462` (examined): spec/test-cases/TC-462-s4-emits-state-nodes.md
- `FR-062-AC-9` (context_only): spec/functional/FR-062-implement-checked-family-contract.md

## Verdict

**MERGEABLE after a sync with main, once FND-001 is fixed or accepted; one more minor finding.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | [minor] AC-9 and TC-462 step 6 name the fixture `state_clause_operation_and_population_variants`, which exists nowhere on main or on any open branch of the repo. The name comes from the QSL-676 report, quoted in STD-279. A test author cannot build it from the spec, so this part of the AC rests on an unpublished artifact. Describe the fixture's content (which clauses, operations and populations it holds), or drop that sentence: the ConfigVersion and R vectors already cover the rule. | spec/functional/FR-105-emit-state-nodes.md:162 |
| FND-002 | low | [minor] 'A `generated` frame occurrence with no region still refuses' is change-relative wording. Write 'refuses `UnlocatedOccurrence` (FR-062)'. | spec/functional/FR-105-emit-state-nodes.md:162 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | bc79a8af6: FR-105-AC-9 and TC-462 step 6 drop the unpublished fixture; the ConfigVersion and R vectors remain. |
| FND-002 | fixed | bc79a8af6: 'A `generated` frame occurrence with no region refuses `UnlocatedOccurrence` (FR-062).' |

Disposition pass 1 verdict (head bc79a8af6): **MERGEABLE.**

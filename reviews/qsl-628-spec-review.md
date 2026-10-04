---
id: SR-1290
title: "Spec review of quire-spec-language PR #628: quire-walk moves to its own repository"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4c418ede82df74fb1589f9367e16a13fa5a99c73; spec/functional/FR-356-*.md, spec/functional/FR-284-*.md, spec/decisions/ADR-011-*.md, ADR-029-*.md, ADR-030-*.md, spec/spec.md, spec/tests.md, spec/test-cases/TC-898, TC-899 (deleted)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-356
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: none resolvable from the branch; review slug qsl-628. PR: quire-spec-language#628.

Checked and clean:
- ADR-011: the W layer row, the crate map, the FB-05 row and the §7.1 edge table
  now say quire-walk is external, in agent-ix/quire-walk. FB-05's "IR never
  depends on the QSL repository" holds: the IR edge goes to the new repository.
- ADR-029 CB-2 and FR-284: quire-walk moves out of the "in QSL" core list into
  its own bullet ("in its own repository, which qsl-forms and qsl-semantics
  depend on"). This is true: both crates depend on it, and qsl-semantics uses
  it in check/*. FR-284-AC-4's ambient-input scan covers "each QSL core crate",
  so dropping quire-walk from CORE_CRATES does not contradict it.
- ADR-030: TC-898 and TC-899 are stated to live in agent-ix/quire-walk.
- FR-356 keeps AC-1 (arch-lint half), AC-5, AC-6 and AC-7, and declares
  `depends_on ix://agent-ix/quire-walk/FR-356`. That FR exists in the quire-walk
  checkout at 89d05df, with TC-898 and TC-899.
- tests.md drops the TC-898 and TC-899 rows and names AC-1's backing tests.
  spec.md's FR-356 row drops TC-898 and TC-899 from its planned list.

## Verdict

Approve with one medium finding and one low. The documents are honest about
where the toolkit lives. AC-1 still asserts an implementation fact that no
longer does anything, and the AC-number cross-references to the other
repository are imprecise.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-356-AC-1 opens with "arch-lint's `SHARED_LEAVES` holds `quire-walk`". Behavior 3 and ADR-011 §6.1's "W is a shared leaf" rule say the same. With quire-walk sourced from agent-ix/quire-walk, the entry has no effect: `classify` gives the crate no repository, so the edge is admitted with or without it (SR-1288 FND-001). The AC binds an internal list's contents rather than behaviour, and its test cannot fail on that clause. Restate AC-1 as behaviour only: the direction check admits an IR, RT or CG edge to quire-walk sourced from agent-ix/quire-walk, and refuses one to any qsl-* crate other than CG's normal edge to qsl-replay. Drop the SHARED_LEAVES sentences from Behavior 3 and ADR-011 §6.1, together with the code change. | spec/functional/FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md:53; spec/functional/FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md:86; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:815 |
| FND-002 | low | FR-356's Description, spec.md's FR-356 row and tests.md say the toolkit's "AC-2 to AC-4" (with TC-898 and TC-899) live in agent-ix/quire-walk. That repository's FR-356 holds AC-1 to AC-5: its AC-1 is the no_std build and dependency tree, and its AC-5 is arena identity. So QSL's FR-356-AC-1 and FR-356-AC-5 share ids with different criteria there. AC-1's Verification cell already points at "FR-356-AC-1 of agent-ix/quire-walk", so the Description and the index rows contradict it. Say "the toolkit's own criteria (agent-ix/quire-walk FR-356, all its ACs)" instead of "AC-2 to AC-4", and qualify cross-repository AC ids by repository. | spec/functional/FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md:28-31; spec/spec.md:1237; spec/tests.md:1017-1019 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 944ee8c14646469d3f62ed5e274ec06817c7bfc8 |
| FND-002 | fixed | 944ee8c14646469d3f62ed5e274ec06817c7bfc8 |

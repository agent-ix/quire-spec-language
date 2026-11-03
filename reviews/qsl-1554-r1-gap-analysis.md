---
id: SR-2207002
title: "Gap analysis of PR #2207: await after a compensation template (QSL-1554)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4314c1cad; PR #2207 diff; FR-519-AC-1, TC-1314 row 1; QSpec FR-466-AC-6, TC-948 AW6/AW7, FR-464 derived causal relation; ticket QSL-1554"
review_set: subset
date: '2026-11-03'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-519
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1314
    type: references
---

## Summary

Ticket: QSL-1554. This is a planless audit of the criteria the PR's one test claims, against the PR diff. The production change is a single behaviour: the checker lowers `await ... after <compensation template>` and writes `after.kind = compensation` with the template as target. One e2e test runs real source through check, emit, IR's reader and QSL's I2 reader, and asserts that behaviour and the resulting `AwaitAnchor` edge. The ticket's asks are met: the root cause is fixed rather than refused (QSpec FR-466-AC-6 says the form is admitted), a value-asserting e2e test is tagged to the owning criteria, and the view's `AwaitAnchor` compensation edge now has a real e2e test.

Reverse gap: the diff adds no production code without an owning requirement. `template_of`, the `build_order` dependency and the memo all serve the FR-466 await layout. There are no stubs or placeholders.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-519-AC-1 | spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md | examined |
| TC-1314 row 1 | spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md | examined |
| QSpec FR-466-AC-6 | quire-specification FR-466 (origin/main) | examined |
| QSpec TC-948 AW6, AW7 | quire-specification TC-948 (origin/main) | examined |
| QSpec FR-464 derived causal relation (await_anchor) | quire-specification FR-464 (origin/main) | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The test is tagged FR-519-AC-1, whose clause is that the relation "derived from the in-process package equals the one derived from the package read back". It asserts only the `AwaitAnchor` pair against a hand-written expectation, not the whole derived relation against the checker's compiled edges. The file already has the helpers for that (`derived_pairs`, `in_process_pairs`, used at line 239). Without the comparison, a divergence between S3's `enter(effect) -> enter(await)` rule and the view's rule on the compensation variant is caught only for the one edge the test spells out. A one-line `assert_eq!` over the two closes it. | qsl-package/src/emit/tests/protocol_view.rs:533-555 |
| FND-002 | low | Two template paths that the PR adds have no test. A second `await` on the same template takes the `Entry::Occupied` memo branch, and `compensation_nodes` reuses a key built in `run_nodes`. A template with no commit takes the `CheckedCommit::Never` branch in `build_order`. The fixture has one await and a template with a commit, so the memo, which keeps one template node and one declaration occurrence, is exercised only for a single await. | qsl-semantics/src/check/lowering/protocol/controls.rs:222-236,1070-1072 |

## Coverage

- Reconciliation: manual, against the PR diff and the test's trace tags.
- Plan completion: not assessed.
- FR-519-AC-1: tagged. The compensation-anchor edge is asserted e2e; the in-process versus read-back equality on this variant is not (FND-001).
- QSpec FR-466-AC-6: the "admitted after a compensation" clause is asserted through both readers. The refusal clauses belong to other TC-948 vectors.
- TC-948 AW6: asserted (kind `compensation`, target `Refund`). AW7 is a reader vector the checker cannot emit (`template_of` requires the template kind), so it has no checker-side test, and needs none.

## Verdict

The behaviour the ticket asks for is implemented and covered by a value-asserting e2e test. Both findings are small additions to the test, and neither blocks merge.

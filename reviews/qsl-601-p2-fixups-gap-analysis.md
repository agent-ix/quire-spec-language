---
id: SR-20261109202151
title: "Gap analysis — l5/p2-render follow-up: doctest lane declarations and checked_handoff fixtures"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; branch l5/p2-render vs origin/l5/integration; FR-518-AC-5; xtask/src/test_suites.rs; src/checked_handoff/corpus.rs, src/checked_handoff/inventory.rs; reviews/qsl-601-gap-analysis.md"
review_set: subset
---

## Summary

Ticket: QSL-601. Gap analysis scoped to the diff. The only production-code
change is in `xtask` lane declarations, which the guard checks against the
tree every time it runs. The Rust changes are test fixtures under
`#[trace("FR-518-AC-5", "TC-1313")]` and the inventory tests. The committed
gap-analysis record's dispositions were also read against what they claim.

## Verdict

**Approve with low findings.** The fixtures keep FR-518-AC-5's test
meaningful. The base and the case still differ only by the case's edit
(none), so the mismatch assertion still exercises the unchanged-bytes path.
The inventory fixture's single `operand_rows` row matches its single node. The
two tests and the other three inventory tests pass under `--features
checked-handoff`. Reverse gap: no production code in the diff lacks an owner.
The xtask lanes serve QSL#157's guard.

## Scope

- **FR-518-AC-5 — examined**: corpus test `a_case_that_leaves_the_base_bytes_unchanged_is_a_mismatch` (TC-1313) and its fixture.
- **inventory tests — examined**: `a_node_the_package_holds_is_named_by_its_wire_id` and three siblings over `PACKAGE`.
- **reviews/qsl-601-gap-analysis.md FND-001 disposition — examined**: "The pin was taken from a first run, so it fixes current output rather than an independently derived expectation."
- **docs/inspect-render.md — context_only**: "Known gap: the view JSON omits the requirement record's `requirements`, `result_bound`, `conversion_context`, `path_condition` and `references`, and no spec yet defines a view JSON schema."

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-298-AC-4 (JSON independent of the text renderer) is backed by a golden JSON snapshot captured from a first run, as the committed disposition itself states. The test catches drift, but it would have passed whatever the JSON was at capture time. The AC's actual claim, JSON unchanged with the text renderer swapped, still has no test that varies the text renderer. | reviews/qsl-601-gap-analysis.md:76; qsl-inspect/src/render.rs (on base) |
| FND-002 | low | The lossy view-JSON gap (missing requirement-record fields, no view JSON schema in FR-297/FR-298) is recorded only as prose in docs/inspect-render.md and as an undisposed review finding. No ticket owns it, though the repo's CLAUDE.md says remaining work belongs in tickets. | docs/inspect-render.md:13-14; reviews/qsl-601-code-review.md:58 |

## Coverage

- Reconciliation: manual trace-tag check of the changed tests; `quoin matrix` not run (diff is fixtures and xtask declarations only).
- Plan completion: not assessed
- FR-518-AC-5: backed, test passes with the new fixture.
- Semantic review: not run.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `outcome_json_does_not_change_when_the_text_rendering_does` renders one outcome under two source sets, asserts the text differs, and asserts the JSON bytes are equal both times and equal to `document.to_bytes()` (qsl-inspect/src/render.rs:1522-1553). |
| FND-002 | accepted-no-change | The lead decided to ship with the lossy view JSON documented in docs/inspect-render.md, because no spec yet defines a view JSON schema. No ticket was filed. No PR is open for l5/p2-render, so the "PR body" listing could not be checked. |

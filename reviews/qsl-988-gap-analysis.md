---
id: SR-17006
title: "Gap analysis of quire-spec-language PR #1433: QSL-988 HP-4 negotiation settlement and hyper tag gaps"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@23597d9fdbb46f1033f808c8bdd810aab973b3f5; PR #1433 (QSL-988) diff origin/main...HEAD; criteria FR-173-AC-2, FR-174-AC-5/6/8, FR-176-AC-1, FR-177-AC-7..12, FR-178-AC-5/7/8/9, FR-182-AC-3/9, FR-183-AC-7"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1433

## Summary

Ticket: QSL-988. This is a planless gap analysis of the ticket's three work
items against the computed matrix (`quire matrix --scope . --format json` at
the reviewed head) and the source.

- Work item 1 (HP-4 settles V-8 at negotiation, through qsl-route routing
  and hyper_settle.rs) is partly met. `settle_hyper_form` and
  `UnsupportedHyperForm` exist and are tested. No routing or negotiation
  path calls them, and the engine still refuses `Other` with
  `FormNotDecided` (FND-001).
- Work item 2 (tests for FR-174-AC-6's third sentence, FR-178-AC-7 and
  FR-182-AC-9) is met. All three are tagged, and mutations kill them.
- Work item 3 (tags) is met for FR-174-AC-5, FR-177-AC-7, FR-178-AC-8,
  FR-178-AC-9 and FR-183-AC-7. FR-183-AC-3 was already tagged. The stale
  14-state header in hyper.rs is fixed. FR-174-AC-8 (QSL-929),
  FR-176-AC-1 and FR-177-AC-8..12 stay untagged with stated reasons
  (FND-002).

The matrix rows read: FR-173-AC-2, FR-174-AC-5, FR-174-AC-6, FR-177-AC-7,
FR-178-AC-5, FR-178-AC-7, FR-178-AC-8, FR-178-AC-9, FR-182-AC-9 and
FR-183-AC-7 are tagged, each by one test function. FR-174-AC-8,
FR-176-AC-1, FR-177-AC-8..12 and FR-182-AC-3 are untagged. No doc id
inflates them.

Reverse gap: `settle_hyper_form` is public API with no production caller.
It is acceptable as groundwork once FND-001's wiring ticket exists. No stubs
were found, and the test-world changes are test-only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Work item 1 is half delivered. The ticket asks for HP-4 items to settle V-8 at negotiation, "qsl-route routing plus" hyper_settle.rs. Only the settlement function landed. `qsl-route` has no hyper form routing, `run_hyper` still refuses `HyperForm::Other` with `HyperRefusal::FormNotDecided`, and nothing calls `settle_hyper_form`. FR-173-AC-2's negotiation clause and FR-182-AC-3's HP-4 row therefore have no end-to-end path. No follow-up ticket for the CG-side wiring is filed yet. Fix: file it and cite it in the doc comment, or land the routing. Until then, QSL-988 should not close as complete. | qsl-replay/src/model_check/hyper_settle.rs:79-94; qsl-analyze/src/model_check/hyper.rs:240-248 |
| FND-002 | low | Work item 3's tags for FR-176-AC-1 and FR-177-AC-8..12 remain untagged. The PR body gives the reasons. FR-176-AC-1's "no reduction ... Holds{Exhaustive}" disagrees with the engine's default copy-swap (`HoldsReduced`), which is a spec-vs-engine question. FR-177-AC-8..12's tests are split, or use `1 / value` where the spec has `1 mod value`. Unlike FR-174-AC-8 (QSL-929), neither has a follow-up ticket, so the gaps can be lost when QSL-988 closes. | qsl-analyze/tests/it/hyper.rs:12-16; spec/functional/FR-176-check-a-universal-hyperproperty-by-self-composition.md |

## Verdict

**CONDITIONAL.** There are medium and low findings only. Every tag added is
backed by assertions that a mutation can fail. The one partial tag
(FR-173-AC-2) is SR-17005 FND-001.

## Coverage

- Criteria examined: 18. Tagged and complete: 9. Tagged but partial: 1
  (FR-173-AC-2). Deliberately untagged: 8 (FR-174-AC-8, FR-176-AC-1,
  FR-177-AC-8..12, FR-182-AC-3).
- Mutations: 6 run by the reviewer, all killed.
- Semantic review: skipped (not opted in). The clause-by-clause test-to-AC
  check is recorded in SR-17005.
- Plan completion: not assessed.

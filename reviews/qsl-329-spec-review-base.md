---
id: SR-820
title: "QSL-329 spec review of PR 537's spec edits (FR-105, FR-108, spec.md, tests.md, ADR-016, ADR-017)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5a08d72f78773410fac8c7b62978d814edd86f5a; spec/functional/FR-105-emit-state-nodes.md; spec/functional/FR-108-run-the-configversion-spine-corpus.md; spec/spec.md; spec/tests.md; spec/decisions/ADR-016-state-model-finite-execution-mapping.md; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md; spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md (unchanged); spec/test-cases/TC-469-configversion-spine-corpus-matches-native.md (unchanged)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
## Summary

Ticket: QSL-329. PR: quire-spec-language#537 at 5a08d72f.

What the edits get right: FR-105 AC-3 row and Status, FR-108 AC-6 row,
Dependencies and Status, the spec.md FR-105 row and the TC-463 matrix row
now state what is, with no pin, SHA or revision wording added (the old
`2a28643` wording is removed). ADR-016's "IR dependencies" paragraph, PI-1
and the removal of the IR-370 open dependency are accurate: IR-370 is
consumed, so keeping it as an open dependency would be false. The remaining
item's "Neither has a ticket" still reads correctly because that item names
two things. ADR-017's Tests bullet and the Protocol/frame summary row are
accurate.

Conflict risk with PR #535: both PRs edit ADR-017's §6 ticket table (#535
at TK-1, this PR at TK-4) and spec/tests.md. A three-way merge of this head
with #535's head produces no conflict markers; the two ADR-017 hunks are
two unchanged lines apart.

The findings below are status text the PR left stale, or edited and left
wrong.

## Verdict

The edits made are accurate. Five status statements that describe the same
two tests were not updated, so the spec now contradicts itself.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The spec.md FR-108 index row still says "AC-6 (pinned `package_id`, I04 `read`) pending STD-111; not yet implemented -- TC-469 planned". FR-108's own Status, edited in this PR, says "AC-1 to AC-6 are implemented". Fix the row to match. | spec/spec.md:531 |
| FND-002 | medium | The TC-469 matrix row stays "🚧 Planned". All six steps are traced tests that run and pass (step 6's last half un-ignored here), TC-469's own file says "Implemented (QSL-314)", and FR-108 now says AC-1 to AC-6 are implemented. Planned is wrong; it should read passed locally like TC-463. The M-6c retirement of step 2 is a future change, not a reason to call it Planned. | spec/tests.md:251 |
| FND-003 | medium | TC-463's Status still says "Partial" and that step 1 is `#[ignore]`d because the pinned `quire-contract-model` refuses every `reaches_field` application (IR-370). This PR un-ignored step 1 and it passes. | spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md:52-58 |
| FND-004 | medium | TC-469's Status still says "Step 6's I04 `read` half is pending IR-370". It now runs and passes. | spec/test-cases/TC-469-configversion-spine-corpus-matches-native.md:77-78 |
| FND-005 | low | The sentence this PR edited says "TC-463 and TC-466 have passed locally; the rest are `🚧 Planned`". TC-459, TC-460, TC-461 and TC-462 in the same range are also passed or covered (tests.md:242-245), and TC-469 is too once FND-002 is fixed. | spec/tests.md:652-653 |
| FND-006 | low | TK-4 now ends "Done." inside a table introduced as "These are proposed; the team lead files them." A proposed-ticket table carrying a done row mixes status into a proposal list. Either drop the TK-4 row (the Tests bullet and summary row already say TC-463 passes) or leave the row without the status word. | spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md:749 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 10b12bbd |
| FND-002 | fixed | 10b12bbd |
| FND-003 | fixed | 10b12bbd |
| FND-004 | fixed | 10b12bbd |
| FND-005 | fixed | 10b12bbd |
| FND-006 | fixed | 10b12bbd |

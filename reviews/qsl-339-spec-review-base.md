---
id: SR-918
title: "QSL-339 spec review of PR 544 (strip Linear ticket ids from spec/)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@52a4c1ab7b8d7e218029b46c67eda7ea6ef6c498; spec/** except spec/reviews/** (diff origin/main...HEAD, about 950 ids removed); spec/evidence/measurements/*.json (untouched, judged); top-level reviews/** and the TC-196 trace-tag question excluded by brief"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: reviews
---
## Summary

Ticket: QSL-339. PR: quire-spec-language#544 at 52a4c1ab. The PR removes
Linear ticket ids (QSL-NNN) from spec/ text. Where an id was the subject of a
sentence, the sentence was rewritten. Where an id named a change, it was
swapped for an ADR slice, milestone or GitHub label.

What was checked:

- **Label substitutions.** Each id-to-label pair in the word diff was checked
  against the Linear ticket title and the ADR-011 §7.3 table. All of these are
  correct: QSL-177..185 → X-2..X-10, QSL-6 → M-4/#242, QSL-7 → M-2,
  QSL-8 → M-6a/#240, QSL-131 → #213 S-1b, QSL-138/141 → M-3a/M-3b,
  QSL-139 → M-5, QSL-140 → S-6, QSL-159 → S-4, QSL-160 → S-5b,
  QSL-233 → S-4b, QSL-303 → M-6d, QSL-16..21 → #223..#218, QSL-25 → #214,
  QSL-46 → #185, QSL-57 → #164, and QSL-5 → M-6c in the native-deletion rows.
  `#120 (QSL-120)` → `#120` and `#185 (QSL-185)` → `#185` are also correct:
  the GitHub numbers were the right pointers, and the old Linear ids next to
  them were mispaired.
- **Status words.** FR-084-AC-7 "planned" is true: TC-410 is Planned and
  ADR-016 G-9 lists it as open. FR-088-AC-11/12 "implemented" is true:
  TC-409 and TC-411 are ✅ and the tests are traced. TC-246's header
  "Implemented." is true: FR-110 Status says Implemented, and QSL-234 and
  QSL-284 are Done. ADR-012 §14.1's "Done" cells are true for every ticket
  replaced (QSL-140, 143, 145, 149..156, 161, 242, 266, 283 are all Done in
  Linear).
- **Keep rule.** Every id the PR removed was checked against Linear. None of
  the removed ids belongs to an open ticket. The open ids QSL-20, 42, 43, 67,
  68, 265 and 290 were kept where they appear.
- **Measurement JSON (spec/evidence/measurements).** Judged not ceremony.
  `sharedLoad`/`role` and the file names use the ticket id as the only name of
  the change each A/B run measured. qsl-bench/BASELINE.md also cites those
  file names. No finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The PR rewrote QSL-165 as "ADR-011 M-2c" in FR-074 and FR-068, but ADR-011 §7.3 has no M-2c row: that row is still keyed `QSL-165`. Its neighbours `QSL-146` and `QSL-166` are also keyed by Done ticket ids. These are the "unsure" migration-table rows, and they are ceremony. Rename the row ids (QSL-165 → M-2c, and give QSL-146/QSL-166 labels) so the new references resolve. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1045-1048; spec/functional/FR-074-move-model-below-check.md:48-49; spec/functional/FR-068-split-expression-checking-into-check-stage.md:521 |
| FND-002 | medium | "QSL-156 slice A4b" became "slice A4b". The label is not defined anywhere in spec/, because it was a slice of one ticket. The status around it is also false: A4b merged as #384 (55db8a4c, QSL-156 Done), yet about 20 places still say "on the A4b branch, pending merge" or "Remaining work: slice A4b". Replace these with #384 and true statuses. | spec/test-cases/TC-415-checked-expressions-lower-to-catalogued-fr-322-nodes.md:155; FR-065:169,297,360,367; FR-088:278; FR-092:980; FR-093:335,755,782; FR-094:720; TC-163:59; TC-380:43; TC-413/414/417/418/419 Status; spec/spec.md:517-519; spec/tests.md:199-205,600,609 |
| FND-003 | medium | FR-088's Status says "AC-11 (TC-409) is not implemented". In the same PR, spec.md's FR-088 row now says AC-11 is implemented, and tests.md shows TC-409 as ✅ with traced tests. The PR edited this Status sentence and kept the false half. | spec/functional/FR-088-clause-name-and-type-identity.md:293-294 |
| FND-004 | medium | FR-065 says ADR-011's M-6a row "states its owner verbatim: \"QSL-8 (this repo's #240) with M-4, before #216\"". The PR changed that row to "#240 with M-4, before #216", so the quote no longer matches its source. | spec/functional/FR-065-migrate-function-application-to-checked-family.md:286 |
| FND-005 | medium | FR-078 quotes ADR-012's OBS-004 row: "so QSL carried both until QSL-131 removed them". The PR changed that row to "until #213 S-1b removed them", so the quote no longer matches its source. | spec/functional/FR-078-remove-qsl-negotiate-copies.md:78 |
| FND-006 | low | ADR-012 §14.1: the string-edge row is now marked Done but still says "The scan does not yet resolve named `const NAME: &str` operands". QSL-287 (Done) delivered that, and FR-064 Status says the detector covers same-crate named constants. The PR removed the owner pointer and left the false clause. | spec/decisions/ADR-012-semantic-family-extension-contracts.md (§14.1 string-edge row) |
| FND-007 | low | ADR-012 §14.1's replay-facade row: the owner cell became "#243". That is the GitHub number of QSL-5, which is Done, so it is the same tracking pointer under another name. The row's work is still open: TC-166 is "🚧 Planned" and "has zero tests". Name a real open owner, or state the status. | spec/decisions/ADR-012-semantic-family-extension-contracts.md (§14.1 replay-facade row) |
| FND-008 | low | Tracker notes about Done tickets were kept. ADR-016 Open dependencies item 2 says "Linear has QSL-20 blocking QSL-19", but QSL-19 is Done. ADR-017 §6 lists ticket-text corrections for QSL-16, which is Done. ADR-017 §8 item 4 says "QSL-20 blocks QSL-16". These are ceremony; delete them. The QSL-36/39/40 edge notes point at open tickets, so they stay. | spec/decisions/ADR-016-state-model-finite-execution-mapping.md:496-497; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md:760-761,794-795 |
| FND-009 | low | "QSL-248 (G2)" became a bare "G2". G2 is not defined in any ADR (it was only part of a ticket title). It also collides with FR-092's golden vector G2 (`record List`), which qsl-semantics lowering tests cite. Describe the change instead, e.g. "the deletion of qsl-eval's second v2 producer". | spec/functional/FR-060-check-qsl-api-surface-boundary.md:244; code comments listed in SR-919 |

## Verdict

Most of the sweep is correct. The label substitutions sampled from the diff are
right, the status words the PR added are true, and no removed id belonged to
an open ticket. Five medium findings remain. Each is a dangling reference or
false status that the PR created or left in a sentence it edited: an M-2c
label with no row, an undefined "slice A4b" label on merged work, FR-088's
Status contradicting spec.md, and two verbatim quotes whose sources the PR
changed. Four low findings cover leftovers. Every fix is text-only.

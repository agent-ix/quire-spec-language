---
id: SR-2239003
title: "spec-review of QSL-1566 FR-267 temporal rows and TC-1160 step 7 (quire-spec-language PR 2239)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f2de892fc132e076b6f43d8e39209b14a888ae66; spec/functional/FR-267-write-run-results-as-native-run-result-2.md, spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1160
    type: reviews
---
# SR-2239003: spec-review of QSL-1566 FR-267 temporal rows (PR 2239)

## Summary

Ticket: QSL-1566. Base spec review (integrity and cross-spec consistency) of
the FR-267 diff: six new temporal rows, the `same-stamp-order-unavailable`
variant, the `reason`/`guard`/`dimension` member definitions, the
counterexample step/locus/interval encoding, FR-267-AC-18 and TC-1160 step 7.
Checked against QSpec FR-352 "Temporal disposition" (read from a QSpec
`origin/main` worktree; cited by ID only). `quire validate` on both changed
files exits 0. The row count (sixteen) matches the table; the new
`relationships` edges (FR-501, FR-502, FR-285) are real dependencies.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-267 now contradicts QSpec FR-352, the owner of the wire's category and kind sets. QSpec FR-352 fixes `disposition.temporal` to exactly ten kinds and the `pending` row's category to `inconclusive`; FR-267 writes sixteen kinds and the categories `pending` and `indeterminate`, so a QSpec FR-352-conformant strict reader refuses every `pending`, `pending-window`, `pending-next-event`, `indeterminate`, `activation-incomplete`, `progress-limit-reached` and `progress-contradicted` document this PR writes. The PR body acknowledges only the `pending` label difference. Fix: file the QSpec change (extend FR-352's table and category set; ties to IR-1281 for `Category::Indeterminate`) and cite it from FR-267, or keep FR-267's shared rows on FR-352's labels. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:217-230, spec/functional/FR-267-write-run-results-as-native-run-result-2.md:235-241 |
| FND-002 | low | `activation-incomplete` and `progress-limit-reached` have category `incomplete` with `outcome` absent, while the `incomplete` and `window-limit-reached` rows carry FR-100's `incomplete` outcome with a limit. The spec does not say why these two incomplete rows have no outcome; state it, or give them an `incomplete` limit. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:225-227 |
| FND-003 | low | The `interval` sentence covers `Steps` intervals only; it does not say what a `violated` clause whose first interval-carrying operator has a time bound writes (the code writes `null`). | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:274-275 |
| FND-004 | low | FR-267-AC-18 is inserted before FR-267-AC-17 in the AC table; keep the table in id order. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:480-481 |

## Verdict

CONDITIONAL on FND-001 being ticketed upstream: the row definitions are
precise and testable, but the wire they define is not the one QSpec FR-352
currently admits.

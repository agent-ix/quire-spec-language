---
id: SR-2162001
title: "spec-review of FR-517 compensation rows, three-package recipe and inventory gate (quire-spec-language PR 2162)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md; spec/test-cases/TC-1312-write-a-checked-package-handoff-directory-from-source-in-process.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: references
---

# Spec review of FR-517 compensation rows, three-package recipe and inventory gate (quire-spec-language PR 2162)

## Summary

Ticket: QSL-1546. PR: quire-spec-language#2162, head 53997415ac3b895bdc044b34ecb921eccb1a1721, `git diff origin/main...HEAD` against main acf5cb518.

Checked against code PR #2160 at head 45ef24e32 (`src/checked_handoff/selection.rs`, `src/emit_checked_v2.rs`, `examples/checked-handoff/*.native`, `tests/it/checked_handoff.rs`).

Grounded and true:
- Sidecar wire names: `Selection` holds `format`, `package`, `clauses` and `compensations`; a compensation row is `node`, `name`, `binding` (`clock_name`, `clock`). `Selection::check` checks every compensation binding and compares the count of package `temporal_bindings` rows with the count of clause and compensation bindings.
- `temporal_selections` selects a `compensate` template by template name (`clock_selections` falls back to the template name).
- Row counts 4/1/1: the event-position unit has two `temporal` clauses (Due, DueLate) and two `compensate` templates (Full, Partial); the fixed-sample and timestamped-event units have one clause each and no protocol. #2160's `the_recipe_emits_...` asserts clauses + compensations rows, ascending `NodeRef` order.
- `inventory.json`: #2160 writes no inventory; its inventory test is ignored "gated on QSL-1512". QSL-1512 exists (In Progress). AC-1 and TC-1312 row 1 now exclude it; AC-11 is gated and failable.
- Renumbering: FR-517 line 43 (AC-1 to AC-9, AC-11), AC-10 (AC-1 to AC-9, AC-11 and FR-518-AC-1 to AC-12), Dependencies (QSL-1512), TC-1312 scope, row 11 and trace tag agree. No other spec file cites an FR-517 AC range. Tickets QSL-1555, 1556, 1557, 1558 and 1522 exist. No new id besides FR-517-AC-11. `quire validate` on both files: exit 0.
- FR-517 carries no count that FR-518 owns.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-517-AC-4 (and TC-1312 row 4) still require the `Exact` values of the two extreme rationals present in R, and the recipe table still lists `Ratio`, `Exact`, `Amount` and `Total` with unit `U` and the receipt-sum compensations, while the new departure list says the unit `U` and the bounded rationals are not ported (QSL-1558, QSL-1522) and the compensation conditions use `size` (QSL-1555). The text asserts and denies the same elements. AC-4 is not gated: #2160 has no rational in R and its test `the_extreme_rationals_are_package_values` is ignored "gated on QSL-1522", so the AC as written fails against the code it specifies. Gate the rational half of AC-4 and TC row 4 on QSL-1522 in the AC-11 pattern (the `Wide` extrema half is ported and runs), and make the recipe table rows say what R holds, with the unported elements named once as gated. | spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:167,133-137 | wrong-requirement |
| FND-002 | high | FR-517-AC-3, the recipe row and TC-1312 row 3 say R has "three temporal clauses" and "three clauses". R has four: two in the event-position package (Due, DueLate), one in each of the other two; the note in the same section says two and one each, and #2160's test asserts 2 for event position and 1 for the others. Also the sentence "FR-517-AC-3 and the Outputs table hold for each package" is false for AC-3: only the event-position package holds the `protocol_clause`, the two compensation nodes and the effect. Say four temporal clauses across the three packages, and scope each count in AC-3 to its package or to R as a whole. | spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:136,166 | wrong-requirement |
| FND-003 | low | The Recipe section narrates history rather than stating R: "R departs from the legacy recipe in these ways, each with its ticket", "is not ported", "instead of the receipt sum". A reader of the requirement needs what R is; the legacy recipe and the tickets are the history. State the elements R holds, and put each gated element in the AC that is gated on its ticket. | spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:146-156 | wrong-requirement |
| FND-004 | low | The sentence "The sidecar records the trigger binder's name of an `on each` activation." (end of the Recipe section) repeats the Outputs paragraph's "with the trigger binder name and whether a guard is written" and sits in the wrong section. Delete it. | spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:158 | wrong-requirement |
| FND-005 | low | TC-1312 row 9 changes "the `sequence_authority` of the event-position row", but the event-position package holds four event-position rows (two clauses, two compensations); say which. The spec also says each compensation template has an expected `binding`, while #2160 stores it optionally (a template with no clock has none); state which templates have a row. | spec/test-cases/TC-1312-write-a-checked-package-handoff-directory-from-source-in-process.md:28 | wrong-requirement |

## Verdict

Not mergeable. FND-001 and FND-002 are statements the code and the same section contradict. The sidecar wire names, the compensation-row counting, the 4/1/1 counts, the gated AC-11 and the renumbering are correct.

Cross-PR: FR-518 rows 14b and 15b on main still read `malformed_wire` at the entry; the code (#2160) gives `unknown_member` at `/lock/definition_selections/0/digest` and `/digest_domain`. #2162 does not touch FR-518, so that correction exists only in #2160's FR-518 edit; #2160 must keep it (and drop its FR-517 edit once #2162 lands, since both edit the same FR-517 paragraphs).

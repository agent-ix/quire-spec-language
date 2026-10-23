---
id: SR-16233
title: "spec-review of FR-091 unit-graph topology following QSpec FR-142 (quire-spec-language PR 1344)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ba5a75cb; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
  - target: ix://agent-ix/quire-specification/FR-142
    type: references
---

# Spec review of FR-091 unit-graph topology following QSpec FR-142 (quire-spec-language PR 1344)

## Summary

Ticket: STD-383 (QSL-936). This is one spec-review pass over
`git diff origin/main...ba5a75cb`. It covers FR-091's pre-check list, the
"Unit-graph topology" paragraph that defers to QSpec FR-142, the catalog-code
rows, the OQ-11 ruling, the removal of OQ-12 and of the interim cause, AC-21,
AC-34 and AC-35, the new AC-45 to AC-47, and TC-483. The oracle was QSpec
FR-142 "Dimension and conversion algebra" at quire-specification f7afad3c:
the turn list, step (-1), step (0), rows 1 to 6, the first-defect rule, the
least-unit cycle and the dimensionless `N`. FR-142-AC-15 to AC-19 and TC-187
U11h to U11z were also used.

Gates: `make check-spec-validation` at ba5a75cb exits 0, with no finding on a
touched file. The PR merges cleanly with origin/main and with #1345, which
also edits FR-091, but origin/main is not an ancestor of the head. FR-091-AC-45
to AC-47 are unique across open PRs.

Examined, clean. Every vector was recomputed against FR-142:

- **AC-34's topology run.** `Length`'s turn comes first. `b` and `c` have no
  root and form a cycle, so row 6 applies, from `b`, and later turns are not
  examined.
- **AC-35.** Rows 1 to 5 match FR-142's codes, causes and retained items, and
  `N = L / L` is admitted (U11g3).
- **AC-45** matches the base-dimension rule (FR-142-AC-16).
- **AC-46** matches U11h to U11k and step (-1)'s cycle edges from the
  dimension declared first.
- **AC-47** matches U11b and U11l, the cycle retained from the least unit by
  source position.
- **The catalog row** lists exactly FR-142's causes.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | TC-406 still expects the removed causes and the interim deferral. Its Expected Results list "the unresolved dimension or unit name cause", "the dimension or unit cycle cause `invalid_package`/`definition-cycle`" and "The unit-graph topology cause returns the STD-112 cause once QSpec publishes it". The rewritten FR-091-AC-21, which TC-406 verifies, has none of these. The test case contradicts its AC and keeps "pending QSpec" language. Fix: update TC-406 to AC-21's new cause list and FR-142's per-row causes. | spec/test-cases/TC-406-forms-and-assembler-catalog-codes.md:46-52 | missing-requirement |
| FND-002 | medium | FR-091 says it follows FR-142 "with FR-142's rules, order and causes and none of its own", but its restatement narrows FR-142 for imported nodes. FR-091 makes row 2 "a unit target that names no unit form" and keeps a pre-check for "a unit's `:` name that names no dimension form". FR-142 row 2 is "no unit of the package", and FR-142's turn list includes imported and domain-package dimensions as nodes. Under FR-142, local units may be typed by such a dimension or target such a unit. FR-142-AC-18 and AC-19 and U11r to U11u, U11y, U11z1 and U11z2 admit `unit tick : Rate = 1/2 * M::us`. QSL admits package-supplied time dimensions (FR-230, TC-685). Under FR-091's wording such a unit is refused `missing-name`, and no AC covers imported nodes. Fix: state FR-142's imported-node turns and admitted members in FR-091 and add the U11r to U11z2 vectors. Otherwise, if source units cannot name package dimensions, say so and file the STD ticket, because that is a narrowing of FR-142. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:489-490,498-511 | wrong-requirement |
| FND-003 | medium | The OQ-11 ruling row now records this PR's lead default under the old ruling's date and attribution. It still reads "(team lead ruling, 2026-09-26)". Its parts (a) (a cancelling dimension "is the dimensionless dimension") and (c) (topology "takes QSpec FR-142's rules, order and causes") are new content from this PR's lead default, which reverses (a). A reader takes the new text as the 2026-09-26 ruling. Fix: mark the amended parts with their own date and "lead default, not an owner answer", or move them to a new row. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1049 | wrong-requirement |

## Verdict

Mergeable after FND-001 to FND-003 are fixed. The local-node topology
matches QSpec FR-142 exactly: every vector recomputes to the stated row,
code and retained items. The remaining defects are a stale test case, an
unstated or narrowed imported-node case, and a misdated ruling. A main merge
is needed (clean).

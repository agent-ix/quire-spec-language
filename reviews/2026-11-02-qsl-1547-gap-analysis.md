---
id: SR-2026110215472
title: "gap-analysis of QSL-1547 protocol read surface (quire-spec-language PR 2158)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR 2158 diff at 6523d4a2c124d2ccfef09c774eb23e4e5068d447; FR-519, TC-1314"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-519
    type: reviews
---

# Gap analysis of QSL-1547 (PR 2158)

## Summary

Ticket: QSL-1547. Per criterion: AC-1 covered and value-asserting (equality against the in-process compiled edge relation, all ten edge kinds present, two back edges, strict ascending, IR reader and QSL I2 reader agree; mutation checks go red). AC-3 and AC-4 covered, with the weaknesses in the code review FND-005. AC-2 has no evidence (FND-001 here). AC-5 partial: views exist and are tested for declaration, operation identity, binder, owner model, delivery, queue capacity, locus, temporal binding, source, definition, dependency, but no view exists for Type, Value, Number, Population, Window or ArtifactRef records, and "remove the member and read again" is done on cloned nodes and lock, not on an admitted package. AC-6 partial: arch-lint direction was run in review over fresh main clones of the three backends and passed (FB-05, FB-11); the in-PR test is a grep. AC-7: QSL-1549 exists in the project with milestone, blocks QSL-1501, is blocked by QSL-1546, QSL-1547, QSL-1548 (TC row 7 says QSL-1544 to QSL-1548; the ticket lists three obligations as a to f items, not the three of FR-519); not part of this PR.

Verification run: `cargo check --workspace --all-targets --locked` green; `cargo clippy -p qsl-package --all-targets -D warnings` green; `cargo check -p qsl-package --no-default-features` green; `arch-lint api-surface --qsl-only` pass; protocol_view tests 8/8, doctests 5/5.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-519-AC-2 (no second derivation, FR-383-AC-16's test calls the function) has no inspection and no test: nothing in the tree checks it, the existing FR-383-AC-16 test in TC-1062 is not repointed, and a live second derivation exists in qsl-semantics graph.rs. Spec defect: FR-519 says the derivation "exists only in QSL's emit tests", which is false. | spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md; spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md | correct-requirement-no-evidence |
| FND-002 | medium | AC-5's per-kind coverage is incomplete: no typed view for `Type`, `Value`, `Number`, `Population`, `Window`, `ArtifactRef`, and no test removes a member from an admitted package (an admitted package cannot lack the member), so the AC's "package from which that member is removed is refused" is tested on in-memory clones only. Spec defect: the map names successors but defines no view type for these records and the removal is untestable on the admitted type. | spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md | correct-requirement-no-evidence |
| FND-003 | low | AC-6 is verified by grep in the PR; the real check (arch-lint direction over four checkouts) is not a repository test and needs local clones, so nothing in `make ci` enforces it. | qsl-package/src/emit/tests/protocol_view.rs:874 | correct-requirement-no-evidence |

## Verdict

PARTIAL: AC-1, AC-3, AC-4 covered; AC-2, AC-5, AC-6 partial; AC-7 outside the PR. No reverse gap: the extra helpers (`ControlOperation`, `Scheduling`, `Delivery`, `QueueCapacity`, `Binder`) are read views of node members FR-519's map lists (operation identities, scheduling, delivery text with FR-509's `[lo,hi]` form, queue bounds, binder levels).

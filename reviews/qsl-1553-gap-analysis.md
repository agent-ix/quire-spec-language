---
id: SR-202611040232
title: "Gap analysis of quire-spec-language PR 2242: QSL-1553"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7b48fcdcb; qsl-package/src/emit/tests/protocol_controls.rs; spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; spec/test-cases/TC-1063-protocol-emission-refusals-and-read-back.md; spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md"
review_set: subset
date: 2026-11-04
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1063
    type: references
---

# SR-202611040232: Gap analysis of quire-spec-language PR 2242 (QSL-1553)

## Summary

Ticket: QSL-1553. Plan completion: not assessed. This is a diff-scoped, planless check. The new FR-383-AC-61 has one tagged test, `each_position_of_an_application_has_its_own_occurrence_in_document_order` (`#[trace("TC-1063", "FR-383-AC-61")]`). That test asserts both two-occurrence sets exactly, so it can fail. No other changed AC (FR-383-AC-43..45, 50, 52, 53, 56, 57; FR-390-AC-35, 37, 39, 52, 53, 55..57) has a tagged test, at HEAD or before this PR. That is expected: the emitter writes `protocol_locations: Vec::new()` (qsl-package/src/emit.rs:2961), so no row-level vector can run yet. The PR body states this as remaining work.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The remaining work has no ticket: emitting `protocol_locations` rows and reader-view pairing. Every row vector this PR re-keys depends on it, and the PR body says "No ticket tracks it yet". Without one, the re-keyed vectors cannot be measured against the emitter and reader, and the gap is not tracked. Fix: file a Linear ticket for row emission and reader pairing, and cite it as `Remaining work:` in the PR body. | qsl-package/src/emit.rs:2961 | missing-requirement |

## Verdict

**Mergeable** once FND-001 has a ticket. The test for the new AC is real and backs the measured fixture E occurrences.

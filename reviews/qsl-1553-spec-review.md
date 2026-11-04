---
id: SR-202611040231
title: "Spec review of quire-spec-language PR 2242: QSL-1553"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7b48fcdcb; spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; spec/test-cases/TC-1063-protocol-emission-refusals-and-read-back.md; spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md"
review_set: subset
date: 2026-11-04
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1063
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1070
    type: references
---

# SR-202611040231: Spec review of quire-spec-language PR 2242 (QSL-1553)

## Summary

Ticket: QSL-1553. Head 7b48fcdcb, scoped to the PR diff. The general FR-383 keying rule (visits keyed in non-decreasing occurrence ordinal, consecutive `term_ordinal` within one occurrence, ascending (occurrence ordinal, `term_ordinal`) order equals visit order) is kept word for word at FR-383 lines 263-272, and FR-390's pairing text (line 572) still defers to it. The fixture E prose and vectors now match the occurrences the checker records (measured by the new test: `earlier.tally` occurrence 0 at `[0,0,0,0]` and 1 at `[0,0,1,1,0]` of the retry origin; `target >= 0` occurrence 0 in the trigger guard and 1 in the retry relation). The I2 vectors were re-derived against the QSpec FR-322 reader in quire-contract-model (`protocol_locations.rs` `validate_rows`, lock rev 64de9ec): rows are ordered by (application, occurrence, `term_ordinal`), each (application, occurrence) group must run from 0 with no gap, and the root pointer is used for a gap. Under the new keys FR-383-AC-43 (drop the occurrence-1 row) is admitted, FR-383-AC-44 / FR-390-AC-39 (occurrence-1 row moved to `term_ordinal` 1) and FR-383-AC-45 (rekeyed to occurrence 0 at `term_ordinal` 2) are refused `invalid-value` at `/checked_term_data/protocol_locations`, FR-383-AC-46 is unchanged, and FR-390-AC-33 / AC-54 location swaps stay admitted because I2 does not compare a row's location with its occurrence's. The old AC-44 (drop the ordinal-zero row) would now be admitted, so changing it was necessary. FR-390-AC-35, AC-37, AC-57 and AC-58 are consistent with the per-origin pairing. `quire validate` passes on the four files (warnings only, none from these files).

Does the change narrow FR-383? The general rule is not removed, but the new normative paragraph (FR-383 lines 274-280) is written unconditionally and contradicts the retained one-occurrence clause in the case it does not cover (FND-001). E-C keys are derived, not measured (FND-002).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The new paragraph says the emitter SHALL key each visit to the occurrence recorded at that visit's position and SHALL NOT key two visits to one occurrence. The retained general rule (lines 263-272) says that a node with one occurrence has every visit keyed to that occurrence, with consecutive `term_ordinal` values. When a node has fewer recorded `expression` occurrences than visits (one occurrence shared by several positions, or a position where lowering recorded nothing), the two clauses conflict: the new paragraph forbids the only keying the general rule allows, and no refusal is defined. The new paragraph also makes the group text at lines 256-261 ("runs across every origin the group holds, because an application can be held by several slots"; "rows of the origins in origin order") unreachable for slot expressions, while leaving its rationale in place. The rule is not deleted, but in that case it is narrowed in practice. Fix: make the paragraph conditional, e.g. "WHERE the checker recorded an `expression` occurrence at a visit's position, the emitter SHALL key that visit to it; otherwise the general rule above applies". Drop or reword "SHALL NOT key two visits to one occurrence the checker recorded once" so that it forbids only keying a visit to another position's occurrence when its own position has one. Reword the group rationale at lines 256-259 so it no longer implies that multi-slot applications share a group. | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md:274-280 | wrong-requirement |
| FND-002 | medium | The fixture E-C keys are not measured. These cover FR-383-AC-56/57, FR-390-AC-55..58, TC-1070 line 196 ("0 at `Tried` and 1 at `Picked`") and C14c..C14f. No E-C fixture exists in code (no `protocol Composed` source in the repo), and FR-383-AC-61 / TC-1063 L8h measures only fixture E. The values follow from the paragraph's claim that occurrences are "numbered in document order". The checker actually numbers by lowering insertion order (`OccurrenceMap::record`, qsl-semantics/src/check/family.rs:1728), and only fixture E's two cases confirm that this matches document order. The PR body says the E-C vectors use measured keys. Fix: either say in TC-1070 that the E-C keys are derived from the document-order rule until the E-C fixture lands, or extend AC-61/L8h to E-C's `order >= 0` when that fixture is built. | spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md:196 | missing-requirement |
| FND-003 | low | FR-390-AC-37 / TC-1070 C6 say the extra row is "appended" and that I2 admits it. The I2 reader requires rows in ascending (application, occurrence, `term_ordinal`) order and refuses an out-of-order row with `invalid-value` at that row's index (quire-contract-model `validate_rows`). A row appended literally at the end of the array is admitted only if `earlier.tally` is the last application in the array. The wording is older, but this PR rewrote the line. Fix: "inserted directly after the occurrence-1 `earlier.tally` row". | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:807 | wrong-requirement |

## Verdict

**Conditional.** Fixture E's prose and I2 vectors are measured and consistent with the checker and the I2 reader. FND-001 needs a conditional rewording, so the new paragraph does not override the retained one-occurrence rule. FND-002 needs the E-C keys to be marked as derived, or measured.

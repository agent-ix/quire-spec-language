---
id: SR-967
title: "QSL-358 slice 5 spec review of PR 570: FR-068 CON-3/AC-4 amendments, FR-090 path fix, ADR-011 SV module list"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0ff6644bdafd4b07469b86b08d993c1fd4bfa582; diff 0a399675...0ff6644b; spec/functional/FR-068-split-expression-checking-into-check-stage.md; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md (unchanged, checked for staleness)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 5). This review covers the two FR-068 amendments
(CON-3, AC-4) and the FR-090 path fix. ADR-011 was checked for staleness
because this PR leaves it untouched while adding four modules to SV.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-4 now contradicts itself. Its normative sentence still says check defines Location and Origin, value::expression defines InputRefusal, and the scan 'confirms the twelve-versus-one split'. The appended note says the opposite. The TC-173 tests now assert the note, so they fail the AC's main sentence. The note also says InputRefusal moved 'unchanged in shape', but its code() method was removed and replaced by qsl_eval::value::input_refusal_code. The unedited text has the same staleness: the Behavior SHALLs (312-318: 'InputRefusal SHALL stay defined in value::expression') and CON-4 (re-exports of InputRefusal, LocatedLoss, ValueLoss from value::expression, which this PR deletes). Rewrite AC-4, the Behavior paragraph and CON-4 to state the current split: ten check-cause types in check; Location, Origin and InputRefusal in quire_semantic_value, with the catalog code mapped in qsl-eval. Do not append a note that contradicts the sentence before it. Apply the same to CON-3. | spec/functional/FR-068-split-expression-checking-into-check-stage.md:312-318,483,493 |
| FND-002 | low | CON-3 and AC-4 put the ticket id QSL-358 in requirement prose ('Amended (QSL-358)'). Ticket ids belong only in References. Drop the id; once FND-001 rewrites the rows to state the current design, the amendment marker is not needed at all. | spec/functional/FR-068-split-expression-checking-into-check-stage.md:482,493 |
| FND-003 | medium | ADR-011 is now stale. The SV row (715), the §6.2 move table row (905: 'Every caller imports the item from quire_semantic_value::{semantic_node, stop, quantity, unit}') and X-11 (1104) list exactly semantic_node, stop, quantity and unit. After this PR the crate also holds checking (CheckMode, CheckingLimits, DepthAboveMaximum, MAX_CHECKING_DEPTH, DEFAULT_CHECKING_*), location (Origin, Location), call (InputRefusal) and loss (ValueLoss, LocatedLoss). These came out of layer 3 check and layer 5 value::expression, so the ADR's layer allocation for them is wrong too. The X-7 row gives qsl-package's SV edge as 'IDENTITY_LIMITS' only, but it now also names Location and CheckingLimits. Add the four modules to all three rows and the X-7 reason. Merging slice 3 (#567) touches the same rows. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:715,905,1100,1104 |
| FND-004 | low | The path fix pairs the name check::Location with the file quire-semantic-value/src/location.rs. qsl_semantics::check no longer exports Location (check/mod.rs imports it privately), so the name does not resolve. Write quire_semantic_value::location::Location here and at FR-090:25 and :121. The same stale name is outside this diff in FR-095:124, FR-096:67,81,116,174-175, FR-062:215, ADR-012:1649, ADR-013:330,426,617,934, TC-407:54 and TC-426:13. Leave those for a sweep unless the rename is cheap. | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:25,121,192 |

## Verdict

Changes requested (two medium). The FR-068 amendments add notes that
contradict the text they amend, instead of restating the requirement. ADR-011's
SV module list is stale. The FR-090 fix corrects the path but keeps a name that
no longer resolves.

## Dispositions

| ID | Outcome |
| --- | --- |
| FND-001 | fixed 9ca7dbe6: FR-068 AC-4, CON-3, CON-4, the Behavior refusal-split paragraph, the Outputs bullet and the summary now state the current layout; no appended notes, and "unchanged in shape" is gone. |
| FND-002 | fixed 9ca7dbe6: the "Amended (QSL-358)" markers are removed with the rewrite. |
| FND-003 | fixed 9ca7dbe6: ADR-011's SV row, the SV prose, the §6.2 move row, the `value::expression::refusal` row, X-7's qsl-package reason and X-11 list `checking`, `location`, `call` and `loss`. |
| FND-004 | fixed 9ca7dbe6: every `check::Location` in spec/ (FR-062, FR-090, FR-095, FR-096, FR-100, ADR-011, ADR-012, ADR-013, TC-407, TC-426) now reads `quire_semantic_value::location::Location`. |

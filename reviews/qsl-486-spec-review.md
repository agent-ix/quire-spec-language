---
id: SR-1348
title: "Spec review of quire-spec-language PR #647: FR-255 settings table and the B5 status notes"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1db5326b01a40a18f17bd5d408736f92fcf7d83d; spec/functional/FR-255, FR-258, FR-261, FR-263, FR-264, FR-277; spec/test-cases/TC-727, TC-733, TC-736, TC-739; context: FR-259, FR-260, TC-720, TC-721, Linear QSL-486 B5b ruling, QSL-639, QSL-640"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-739
    type: reviews
---
## Summary

Ticket: QSL-486. PR: quire-spec-language#647. Base spec review over the ten
spec files the PR touches, checked against the code at the reviewed sha.

- Deleted FR-255 row groups and their successors. lowering.* is replaced by
  the s3.* rows, all present, including the new `s3.decimal_scale`.
  validation.* and runtime.* input admission are replaced by observation.*
  and admission.*, both present. evaluation.* moves to the accounting
  counters, which by ruling are prose-only, and the prose now says so.
  temporal.* (QSL-509) and format.* (QSL-605) are covered by the new
  sentence "A limit of an operation QSL has not yet ported takes its row
  ... when that operation lands". package.*, composed.* and rule_model.* are
  root-only. No spec file still names a deleted setting (grep over spec/).
- Each FR-255 row matches the `Setting` macro in name, stage and kind, except
  `intake.input_bytes` (FND-001).
- FR-277's "the FR-255 setting that sets the bound" matches
  `LimitExceeded::setting`.
- FR-261-AC-3 and FR-263-AC-1 move to QSL-639 and QSL-640. Both tickets exist
  and state the same scope. The ACs stay as written, which is right: the
  behaviour is still required.

## Verdict

Request changes. The table is close, but one row and the status claims do
not match the code (FND-001, FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-255's table keeps the `intake.input_bytes` row, but no `Setting` exists for it and no limits type maps it (it waits for B6, QSL-487). So FR-255-AC-3 ("the union of those names equals the setting table") and Behavior 6 are false at HEAD: `--limit intake.input_bytes=<n>` is refused as `UnknownSetting`, although it names a setting from the table. The B5b ruling on QSL-486 says the row deletions keep AC-3 exact, but this row breaks that. Fix: either a Status line that says the intake row is not yet accepted by the settings operation (B6), or move the row out until B6 lands, as was done for the other unported rows. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md |
| FND-002 | medium | Status claims contradict each other. TC-739's verification status, FR-264-AC-4's Verification cell, TC-727's Status and FR-258's Status (AC-4) now say the setting names, `stage_limits` and the settings operation are backed by TC-720 and TC-721. But TC-720's and TC-721's own Status still say "Planned", FR-255 has no Status, and TC-721's test does not exercise AC-4 as written (SR-1347 FND-004). Fix: give TC-720, TC-721 and FR-255 a Status that says what is and is not backed, and point the dependents at it. | spec/test-cases/TC-739-v2-read-refuses-inline-terms-and-names-its-limits.md |
| FND-003 | low | FR-259 Behavior 3 still says "`IDENTITY_LIMITS` is its published default, 16777216 bytes". This PR removed `IDENTITY_LIMITS` from every production path and rewrote FR-255's `identity.input_bytes` row to name `AssemblyLimits` identity (`IdentityLimits` input_bytes). FR-259 is not in the diff, so it now names a constant QSL no longer uses. | spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md |
| FND-004 | low | FR-255's Description says "Every configurable resource limit in QSL has one stable setting name. The same name raises the limit from the library, from a replay request and from the driver CLI". With no qualification, that is now false three times: the accounting counters are configurable but not raised through `--limit` (the new prose says so); `replay.input_bytes` is not raised from a request; and the root crate's native limits, whose rows this PR deleted, are still in `src/` until QSL-498. Fix: qualify the Description to match the Setting-names prose. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | The fix for FND-001 deleted the `intake.input_bytes` row, and FR-255's Description now says "A limit a stage fixes itself, such as semantic-IR intake's input ceiling ... [has] no setting name here". That contradicts FR-260 Behavior 5 and FR-260-AC-4, which require `intake.input_bytes` to be a caller setting with no ceiling, set through the intake limits' builder and through FR-255's settings operation as `intake.input_bytes=<n>`. It also contradicts TC-732, ADR-030 D-3 (which gives `intake.input_bytes` as an example setting), FR-056 and the B5b ruling ("intake.* stays with FR-260 (B6, QSL-487)"). The fixed ceiling is today's code, not the target design. Fix: drop the intake clause from the Description and treat intake like the other unported operations (its row comes back with B6), or restore the row and mark it partial in FR-255's Status. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:25-34; spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md:42-47 |

## Dispositions

Round 1, reviewed at 0d47d0c1865d0243f4cceca4b0b246af6325e115 (`git diff 1db5326b..0d47d0c1`, spec commits 535697faa and 4d93d589e). FND-001 is fixed as stated (AC-3 is exact now that the row is gone), but the way it was fixed raises FND-005. The two rows FR-255's Status leaves partial (admission.ancestor_steps, model.family_steps) are in B5's scope; see SR-1346 FND-006.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 535697faa |
| FND-002 | fixed | 535697faa |
| FND-003 | fixed | 535697faa |
| FND-004 | fixed | 535697faa |

Round 2, reviewed at eab14d4b093a6fc1ebf93def26ee8946380bbd01 (`git diff 0d47d0c1..eab14d4b`, spec changes in 155aa2e78). FND-005: the intake clause is gone from the Description. The `intake.input_bytes` row, FR-260 in the owners list and intake's code in Behavior 1 are restored, and the row is marked "(pending QSL-487)" with a Status entry saying the settings operation refuses it until B6. That matches FR-260, TC-732 and ADR-030 D-3 as target design. The AC-3 and AC-6 tests skip only rows carrying that marker, and they fail if a marked row gains a setting.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | 155aa2e78 |

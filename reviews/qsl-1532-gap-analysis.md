---
id: SR-20261103213522
title: "Gap analysis — QSL-1532 FR-513-AC-9..15 successors (PR #2197)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2197 diff origin/main...ab648524a against FR-513-AC-9..15 and TC-1308 rows 18-25"
review_set: subset
---

## Summary

Ticket: QSL-1532. Planless gap analysis of PR #2197 against FR-513-AC-9 to
AC-15 (TC-1308 rows 18 to 25). Plan completion: not assessed. The PR is a
draft and does not claim QSL-1532 is complete. This analysis records which
successor criteria have a test on the successor path at `ab648524a`.

- AC-9: the selection defects (surplus, duplicate, profile or clock mismatch)
  are covered at emission by `tc134_explicit_temporal_declaration_refuses_each_unfit_clock_selection`
  and `temporal_clock_selections_are_unique_locus_bound_and_profile_typed`.
  The clock-value half has no emission test. That half covers empty and
  over-long names, bad periods, and substituted epoch, unit, sequence
  authority and definition fields.
- AC-12: covered except for two of the four skipped-operand cases.
- AC-13: covered by the new Contract IR reader test (assertion strength is in
  SR-20261103213521 FND-005).

## Verdict

**CONDITIONAL**. The PR is fine to land as partial coverage once the
inflated tags (code-review SR-20261103213521) are removed. FR-513-AC-9 (clock
values), AC-11 and AC-12 (skipped operands) remain open and need to be tracked
on QSL-1532 or follow-ups. AC-10, AC-14 and AC-15 already have follow-ups.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | high | FR-513-AC-9 / TC-1308 row 19: no emission-side test refuses an empty or 4097-byte `clock_name`, period 0, -1, 2/4 or beyond i64, or a substituted epoch, unit, sequence authority, definition identity, revision or artifact with `invalid_package`/`invalid-value` naming the field. The only coverage is on the retired v2 reader. | qsl-package/src/emit/tests/temporal_clause.rs; tests/it/compiled_protocol_v2.rs:2965-3071 |
| FND-002 | high | FR-513-AC-11 / TC-1308 row 21: no successor test checks a checked package's derived activation mapping, its round trip, or the 11 listed mutations. The only tag is on the retired v3 test TC-143. | tests/it/compiled_protocol_v2.rs:1313-1322 |
| FND-003 | medium | FR-513-AC-12 / TC-1308 row 22: the new skipped-operand cases cover an unlisted fact, another role's fact and an effect or commit binder. A binder of another branch and a same-spelled name of another owner are not exercised in a skipped operand, so the row's 8 refusals are not all present. | qsl-semantics/src/check/protocol_clause/tests/graph.rs:832-890 |
| FND-004 | medium | FR-513-AC-14 / TC-1308 row 24: two of the row's cases are not exercised. Two occurrences of one observation under distinct keys being admitted is untested, and the PR body defers it to QSL-1582. The surplus-member and duplicate-storage-key refusals are not identifiable in the tagged tests. Yet 8 tests now carry FR-513-AC-14, so the matrix shows it as covered. | qsl-semantics/tests/it/model_population.rs:1367; qsl-semantics/tests/it/relationship_snapshots.rs:745-901 |
| FND-005 | medium | FR-513-AC-10 / TC-1308 row 20: the `identity.input_bytes` half is untested and deferred to QSL-1583. The I2 half is covered by an untagged test (correctly left untagged while partial). | qsl-package/src/emit/tests/temporal_clause.rs:157-200 |
| FND-006 | medium | FR-513-AC-15 / TC-1308 row 25: there is no matrix-regression harness (deferred to QSL-1584), but behaviour tests carry the tag (see SR-20261103213521 FND-004). | tests/it/assessment_artifact_kinds.rs:7 |

## Verification

- Read FR-513 (AC-9..15 and the retained-criteria table) and TC-1308 rows 18-25 at the PR head.
- Focused tests for the new code passed (see SR-20261103213521).

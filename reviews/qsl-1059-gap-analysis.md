---
id: SR-17055
title: "Gap analysis of PR #1486: limit-name criteria after QSL-1059"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@99c6297208619ecedaedff5becabddbd98292d0b; PR #1486 (branch qsl-1059-limit-snake-case); spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md (AC-12, engine table), FR-250 (AC-2, AC-3), FR-191 (AC-1), FR-190 (AC-2), FR-192 (AC-1), FR-184 (AC-2), FR-182 (AC-3, AC-14), FR-126 (AC-19), ADR-022 GE-2/QS-7; spec/test-cases/TC-1161; qsl-replay/src/model_check/limits.rs, qsl-replay/src/statistical.rs, qsl-replay/src/hybrid.rs, qsl-replay/src/statistical/settle.rs"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1161
    type: reviews
---
## Summary

Ticket: QSL-1059. PR: quire-spec-language#1486. No plan bundle. Plan
completion: not assessed. No spec file changed, so spec-review was skipped.

`quoin matrix --json` (quoin 0.29.1) at the head, for the criteria whose limit
names the diff changes: FR-126-AC-19, FR-184-AC-2, FR-190-AC-2, FR-191-AC-1,
FR-192-AC-1, FR-250-AC-2, FR-250-AC-3 and FR-255-AC-12 are `tagged`;
FR-182-AC-3 is `untagged` (pre-existing, unchanged by this diff). All report
`no run evidence`, as on main.

Code against criteria:

- FR-255-AC-12, limit-name clause: every `ModelCheckLimit` variant now names
  its engine-table limit (`^[a-z][a-z0-9_]*$`) and the setting of the same row,
  so the (limit, setting) pair check holds for the EN-1, phase 0, hyper,
  hybrid and EN-4 variants. Before this PR it failed for all of them. The new
  test asserts the names and pattern; it is untagged (SR-17054 FND-002), so the
  matrix's `tagged` status for AC-12 still rests only on the settings-half test
  at qsl-replay/src/limits.rs:513.
- FR-250-AC-2/AC-3: `LimitReached{limit: max_solver_steps, value: 1000,
  setting: hybrid.max_solver_steps}`, now written and asserted on the wire by
  `a_stopped_solver_settles_incomplete_by_the_budget_that_stopped_it`
  (hybrid.rs:1040, TC-705). Closed by this PR: on main the test pinned
  `solver-steps`, contradicting the AC text.
- FR-191-AC-1 / FR-192-AC-1 / FR-190-AC-2: EN-4 stops now name `max_samples`,
  `max_draws`, `max_cycle_steps` as ADR-024 ST-7/ST-8 and FR-191:40-42 spell
  them; settle.rs:440 pins `max_samples`.
- FR-184-AC-2, FR-126-AC-19, FR-182-AC-3: the names they cite
  (`max_witness_set`, meter names, `max_relation_tuples`) now match.
- FR-182-AC-14 / FR-255-AC-14 (`smt.max_solver_steps`): no `ModelCheckLimit`
  variant exists for the SMT run limits, so these are untouched by the diff and
  not a gap it introduces.

No coverage inflation: the diff adds no trace tag.

## Verdict

The diff closes the limit-name half of FR-255-AC-12 for every implemented
`ModelCheckLimit` variant and the FR-250-AC-3 wire spelling, introduces no new
gap, and leaves TC-1161 step 2's evidence untagged. One low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-255-AC-12's limit-name clause is now implemented for every `ModelCheckLimit` variant and asserted by `every_limit_spells_its_name_in_snake_case`, but that test is untagged, so the Test Matrix credits AC-12 only to `the_settings_operation_and_a_request_take_every_setting` (limits.rs:513), which checks setting names and no limit name. TC-1161 ("Tag the tests `#[trace(\"TC-1161\", \"<criterion>\")]`") has no tagged test at all. Fix: tag the new test `#[trace("TC-1161", "FR-255-AC-12")]` (see SR-17054 FND-002). | qsl-replay/src/model_check/limits.rs:274-275, spec/test-cases/TC-1161-every-engine-limit-has-a-schema-conformant-name-and-setting.md:72 |

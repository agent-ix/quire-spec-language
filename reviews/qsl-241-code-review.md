---
id: SR-926
title: "QSL-241 code review (with rust-review lane) of PR 547: QSpec- trace-tag classification"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@35c74a26273504a59ff21cd54be903b727544189; the 48 Rust files of git diff origin/main...HEAD (1313 changed trace-tag strings, 2 of them the quire-exact/src/lib.rs doc example); every #[trace] tag in the workspace at that sha (classification sweep); QSpec ids resolved against agent-ix/quire-specification origin/main 4634f5fd; tests/it/complete_v1_plan.rs and spec/functional/FR-055 (outside the diff, read because Task-047/048 tags point into Plan-013)."
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
---
## Summary

Ticket: QSL-241. PR: quire-spec-language#547 at 35c74a26.

The PR changes trace tags only. Ruling under review: a tag that names a QSpec
(quire-specification) requirement or test case is written `QSpec-<id>`; a bare
id names a QSL artifact.

How the classification was checked:

- **Full mechanical sweep, all 5,223 bare and 1,311 prefixed tags in the
  workspace.** Every QSL TC/FR/AC id was indexed from `spec/` at 35c74a26 and
  every QSpec id from QSpec `origin/main` (4634f5fd; the local checkout's HEAD
  2449ceb is behind it, and the coder correctly used `origin/main`, e.g. for
  `QSpec-FR-341-AC-10`, which exists only there). Result: no bare tag names an
  id that exists only in QSpec. No `QSpec-` tag names an id QSpec lacks (the
  only two hits are the `QSpec-TC-NNN` placeholder in a lib.rs doc comment).
  Rule 1 was applied completely.
- **Verifies-edge consistency, every trace attribute.** For each attribute, a
  bare FR/AC must be verified by a bare TC in the same attribute when one is
  present, and a `QSpec-` FR/AC by a `QSpec-` TC. No attribute has a bare FR
  that only a QSpec TC verifies, or a `QSpec-` FR that only a QSL TC verifies.
- **Hand audit of 67 retagged tags and 25 both-repo ids kept bare.** I read the
  test name, its doc comment and both repos' AC/TC text. The audit was weighted
  toward the judgment cases. Retagged: TC-196 (model_dispatch,
  model_conformance, model_normalization, model_population r06), TC-195,
  TC-197, TC-192, TC-198, FR-042-AC-1..4 in model_reference_queries (all 15
  `pre_*` tests match QSpec FR-042 "select pre-state reads" exactly), and
  FR-046-AC-3 in model_population (all 13 frame-enforcement tests match QSpec
  FR-046-AC-3). Kept bare: TC-193..198 in qsl-route (QSL backend registry),
  TC-202 / FR-078-AC-3 in ieee_profiles and integer_division (QSL
  negotiate-removal regression), FR-042/046 in native_*_emission (QSL TC-121
  "publish compiled protocol artifacts"), FR-113/114 in protocol_clause and
  emit/tests (QSL scoped anchors and attempt frames), and TC-180..191 in
  qsl-replay witness/request/result (QSL replay types).
- A word-overlap heuristic (test context against each repo's AC text) flagged
  85 tags for a second look. All 85 resolved in favour of the coder's choice.

**Misclassifications found: zero.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Plan-013 bookkeeping test pins a commit SHA, a content digest and row counts. This is file/version-tracking ceremony, outside the diff; FR-055/TC-144 mandate it | tests/it/complete_v1_plan.rs:78-140; spec/functional/FR-055-govern-complete-v1-lane.md:47-51 |

## Verdict

The retagging is correct. No retagged tag names the wrong repository's
artifact, and no both-repo id that was kept bare is a QSpec reference. The
judgment cases (TC-192/195/196/197/198, FR-042-*, FR-046-*) were each decided
the right way. The FR-113/114 binder and attempt-frame tests and the
TC-202 / FR-078-AC-3 negotiate-removal rows stay QSL as they should.

Rust lane: the diff touches only `#[trace]` string literals, a rustfmt
reflow of one long attribute (`complete_cst.rs:43`), and two doc-comment
spellings. It adds no code, panics or behaviour. The gate `make ci` exit 0 at
35c74a26 was read from the coordinator's log
(`qsl-241-ci.log`, last line `head=35c74a26… exit=0`). It was not re-run.

FND-001 is outside the diff. It is reported because the brief asked for a ruling on
the `Task-047`/`Task-048` tags, and those tags point into the Plan-013 bookkeeping
this test enforces. `complete_v1_plan_preserves_the_serial_campaign_and_delivery_constraints`
asserts that the plan contains the commit `d49bbdc4…` and fixed row-count
sentences. `complete_v1_plan_allocates_every_agent_a_capability_once` asserts 83
capabilities and per-ticket counts. FR-055-AC-1 pins a `sha256:` digest over
`resources/complete-value`. Each of these passes on any plan that keeps those strings,
whatever the code does. Fix: delete `tests/it/complete_v1_plan.rs`, TC-144 and the
FR-055 ACs that require it. That needs its own ticket, because it amends FR-055.

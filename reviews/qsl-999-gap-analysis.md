---
id: SR-17121
title: "QSL-999 gap analysis of FR-226-AC-1 and FR-226-AC-2 (quire-spec-language PR 1495)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8244b0dcb06c2b63963913db22d19855cef5ce45; spec/functional/FR-226-*.md; spec/test-cases/TC-671-*.md; qsl-analyze/tests/it/load_buffering.rs; qsl-analyze/tests/it/analyze.rs; qsl-route/tests/it/memory_models.rs; qsl-eval/src/simulation/protocol/access.rs; qsl-route/src/lib.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-226
    type: references
---

## Summary

Ticket: QSL-999 (PR quire-spec-language#1495). Planless gap analysis. The
criteria in scope are FR-226-AC-1 and FR-226-AC-2. FR-226-AC-3 is out of scope
and was read for context only. The computed matrix (`quire matrix --scope .`,
quire 0.36.2, at HEAD) shows:

- FR-226-AC-1 is tagged by
  `load_buffering::a_relaxed_load_then_a_store_to_another_shared_location_warns_under_ra`.
- FR-226-AC-2 is tagged by `analyze::en1_advertises_the_memory_models_sc_tso_and_ra`
  and `memory_models::a_manifest_memory_models_member_is_read_and_its_absence_advertises_sc_only`.
- FR-226-AC-3 is untagged. Its test on main carries only `#[trace("TC-671")]`.
  This PR did not cause that.

Semantic review was done inline as part of the reviewer brief, by reading each
AC clause against the tests.

FR-226-AC-1, clause by clause:

- One warning naming both `t` attempts: asserted, with exact transitions and
  the code.
- Value, label and basis equal: asserted, but against a different package. See
  FND-002.
- Counterexample equal: asserted against that same different package.
- Obligation identity equal: not asserted.
- `acquire` load, `acquire` fence, same location, load only `t` touches and
  store only `t` touches: each asserted to give no warning.
- `sc` and `tso`: each asserted to give no warning.
- The Behavior's "non-atomic load" is not exercised.

FR-226-AC-2: the descriptor's `sc`, `tso` and `ra` members are asserted, and
so is the sc-only routing of a descriptor without the member. Both tests earn
the tag.

Reverse gap: at HEAD, the PR deletes the tests and spec of other criteria
because it reverts main. See FND-001.

Plan completion: not assessed

## Verdict

FAIL. FND-001 is high: the branch removes tagged evidence for criteria already
merged on main.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | The PR's tree reverts main: it deletes the FR-177-AC-8..12 tests (#1485), FR-485 and TC-1263 (#1470), `subject_identity.rs` (#1488) and others, so merging drops tagged evidence for merged criteria | qsl-analyze/tests/it/hyper_witness.rs; spec/test-cases/TC-1263-checked-closure-acquisition-and-application-through-replay.md; qsl-replay/src/model_check/subject_identity.rs |
| FND-002 | medium | FR-226-AC-1's "equal those of the same request settled without the warning" is tested against a different request (the load made `acquire`), and the obligation-identity clause is not asserted | qsl-analyze/tests/it/load_buffering.rs:91-107 |
| FND-003 | low | FR-226 Behavior's non-atomic load case has no test; the AC-1 test uses only `relaxed` loads | qsl-analyze/tests/it/load_buffering.rs:70-140 |

## Coverage

- Planless audit. Plan completion: not assessed.
- FR-226-AC-1: examined, tagged, 1 test.
- FR-226-AC-2: examined, tagged, 2 tests.
- FR-226-AC-3: context only, untagged on main and at HEAD. Out of this PR's
  scope.
- Semantic review: done inline (reviewer brief), not fanned out.
- No new production code lacks an owning requirement: the genuine 22-file
  change is all owned by FR-226. The reverted files are covered under FND-001.

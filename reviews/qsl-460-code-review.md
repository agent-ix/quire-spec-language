---
id: SR-2447
title: "Code review of quire-spec-language PR #668: delete ClauseRunProvenance and ClauseRunReport::source_digest (QSL-460)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0605565cd860d8873565c349e467b656626cca7b; PR #668 diff against origin/main: qsl-replay/src/execute/frame.rs, qsl-replay/src/execute/state_clause.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/clause.rs, qsl-replay/src/spine/clause/tests.rs, qsl-replay/src/spine/clause/tests/frame.rs, qsl-replay/src/spine/clause/tests/pre_call_invocation.rs, tests/it/config_version_spine.rs; spec read: FR-109, FR-108, FR-115, FR-330, FR-331, TC-468, TC-469, TC-514"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: reviews
---
# Code review of quire-spec-language PR #668

## Summary

Ticket: QSL-460. PR: quire-spec-language#668, one commit 0605565cd on
origin/main. The Rust lane (rust-review) is folded into this file. Reviewer
only: no build was run; the author's focused gate was clippy -p qsl-replay
--all-targets --all-features, qsl-replay lib tests and the root `it`
config_version_spine test.

The code change is a clean deletion. `ClauseRunProvenance`,
`ExtractionOrigin`, `ClauseRunReport::{source_digest, provenance}` and the
crate-private `UnitProvenance`, `CompiledRun::{unit, model_selections,
selection}` are gone. `run_clause`, `run_function`, `check_clause_admitted`
and `check_frame` build the report from the disposition, `package_id` and
usage only. The documents admission read are no longer a report member: they
move to `ClauseCheck::documents` and a new crate-private
`FrameCheck { report, documents }`, which is exactly what the two replay
executors consumed (`report.provenance.documents`) before.

**No behaviour change in the replay executors.** The deleted byte-provision
lookup in `replay_frame` and `replay_state_clause` re-read the same source
digest `recompile` already resolved through the same `byte_provision()` map
with the same fault (`decoded-request-byte-provision-complete`), so it could
never fire after `recompile` returned. The documents passed into
`FrameCounterexample` and `StateClauseCounterexample` are the same vectors as
before, in the same order, on every path (admit failure, missing name,
evaluated). The frame replay's three-document destructuring
(`an-evaluated-frame-check-read-three-documents`) is kept, and FR-116/FR-122's
replay tests still assert the documents (`frame_replay.rs:462`,
`state_clause_replay.rs:319,333,365,378,641`).

**Public API diff vs origin/main (`qsl_replay::spine`).** Removed:
`pub struct ClauseRunProvenance`, `pub struct ExtractionOrigin`, and the pub
fields `ClauseRunReport::source_digest` and `ClauseRunReport::provenance`.
`ClauseRunReport` is now `{package_id, disposition, usage, basis, witness}`.
Nothing is added to the public API; `FrameCheck`, `ClauseCheck::documents` and
`check_frame`'s new return type are `pub(crate)`.

**Consumers.** `git grep` over freshly fetched origin/main for
`ClauseRunProvenance|ExtractionOrigin|UnitProvenance|ClauseRunReport|run_clause|FrameCheck|ClauseCheck`
and for `.provenance.{documents,extraction,selection,model_selections,frame,source}`
and `report*.source_digest` found no use in quire-driver 5cddd5f6,
quire-integration 80431da1, quire-protocol 8b1dc4ef (its
`input.provenance.source_identity` hits are its own type), quire-contract-ir
74f9eade or quire-contract-codegen e5cbe075. QSpec (fbd6ea0f) names neither
type. The root crate `src/` does not read either member.

The defect is in the spec left behind: FR-115 and TC-514 still require the
deleted frame provenance, and FR-109's Behavior still requires the deleted
extraction origin.

## Verdict

Code: correct, a pure deletion with no executor behaviour change, and the
public API removal has no consumer in the five downstream repos. Not
mergeable as is because of FND-001: FR-115's Outputs, FR-115-AC-1 and TC-514
step 1 still require the report's provenance to hold the frame node and the
three documents, and the PR deletes both the member and the TC-514 assertion
while keeping the FR-115-AC-1 tag. Trim FR-115/TC-514 (and FR-109's Behavior
line, FND-002) in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-115's Outputs ("a ClauseRunReport whose provenance holds the operation, the identity of its frame node, and the identity and digest of the invocation and both snapshots"), FR-115-AC-1 ("with the frame node's identity and the three document identities in its provenance") and TC-514 step 1's expected result still require the report provenance this PR deletes. The PR also deletes the two assertions that checked it (`report.provenance.frame`, `report.provenance.documents`) from `a_frame_respecting_invocation_succeeds`, which keeps its `#[trace("TC-514", "FR-115-AC-1")]` tag. The code now contradicts FR-115-AC-1 as written. Fix in this PR: trim FR-115 Outputs, FR-115-AC-1 and TC-514 step 1 to FR-109's report (disposition, package_id, usage), as PR #585 did for FR-109. | spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md:54-56,104; spec/test-cases/TC-514-run-an-operation-frame-over-an-invocation.md:50-51; qsl-replay/src/spine/clause/tests/frame.rs:140-162 |
| FND-002 | medium | FR-109's Behavior still says an I3 source in a non-`ix:native` fence SHALL report `unknown_language` "carrying the extraction's original identity and digest". After this PR the report carries no extraction origin (`ExtractionOrigin` is deleted), and FR-109's own Outputs and AC-6 no longer list it, so FR-109 contradicts itself and the code. Fix: drop the "carrying ..." clause. | spec/functional/FR-109-run-a-state-clause-through-the-spine.md:94-98 |
| FND-003 | low | FR-109's Status still reads "Implemented, except the report's members FR-109 no longer lists ...". This PR deletes those members, so the exception is now false. Fix: drop that clause from Status. | spec/functional/FR-109-run-a-state-clause-through-the-spine.md:192 |
| FND-004 | low | The doc comment on `mod extraction` still says the I3 run gives "a different source identity and digest, and the extraction's original identity and digest in its provenance". The PR deleted those assertions; the test now checks the disposition only. Fix: describe what it checks (FR-108-AC-5: the direct run's disposition and package_id). | tests/it/config_version_spine.rs:592-595 |
| FND-005 | low | `run_function` keeps 11 parameters behind `#[allow(clippy::too_many_arguments)]`. With `unit`, `model_selections`, `selection` and `selection_documents` gone, its remaining inputs are the fields of `CompiledRun` (packages, model limits, snapshot provision, observation limits, accounting, package, package_id, sources), which the Clause and Frame arms already build. Passing `&CompiledRun` would drop the allow and the duplicated parameter list. | qsl-replay/src/spine/clause.rs:949-962,600-614 |

## Dispositions

Round 1, reviewed at 99b1da34e5e13181c51f7cb69d29cae479b02d0c (fix commits
6602dbd3a, 7f1dfd837, 95cab5e7b, ebaa1c737, 64e7af96e and 99b1da34e on
0605565cd; `git diff 0605565c..99b1da34e`). Read only, no build. This round
adds no new finding.

- FND-001: FR-115 Outputs and FR-115-AC-1 now state FR-109's report: the
  disposition, the compiled package's `package_id`, and usage. TC-514
  step 1 expects the compiled `package_id`. `a_frame_respecting_invocation_succeeds`
  (still tagged TC-514 / FR-115-AC-1) asserts
  `report.package_id == Some(compose(..).emitted.package_id())` from an
  independent `compose` of the request's unit (ebaa1c737). A re-grep of
  `spec/` at 99b1da34e finds no text requiring the deleted provenance:
  `ClauseRunProvenance`, `ExtractionOrigin`, a report `source_digest`, "in
  its provenance", or "original identity and digest". FR-116 and FR-122
  are untouched by the PR. FR-116-AC-1 still requires the three document
  identities and digests on the frame replay result, and the code keeps
  them through `FrameCheck::documents`.
- FND-002: FR-109's `unknown_language` bullet no longer carries the
  extraction origin.
- FND-003: FR-109 Status keeps only the AC-7 exception. The three QSL-460
  reference lines are deleted from FR-108, FR-109 and FR-331 (95cab5e7b).
- FND-004: still open. The `mod extraction` doc comment now reads
  "gives the direct run's disposition and `package_id`". After the spec
  ruling (64e7af96e), the test asserts the direct run's stage, category,
  truth and exit code, and the extracted body's own `package_id` from an
  independent compile, which is not the direct run's.
- FND-005: `run_function(run: &CompiledRun<'_>, name, arguments, snapshot)`
  drops the `too_many_arguments` allow. `run_clause` now builds one
  `CompiledRun` for all three arms.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6602dbd3a |
| FND-002 | fixed | 6602dbd3a |
| FND-003 | fixed | 6602dbd3a |
| FND-004 | still-open | The doc comment at tests/it/config_version_spine.rs:592-594 says the I3 run gives "the direct run's disposition and `package_id`", but after 64e7af96e FR-108-AC-5 and the test check the extracted body's own package_id (a direct compile under the body identity), not the direct run's. Reword it to: the direct run's stage, category, truth and exit code, and the extracted body's own package_id |
| FND-005 | fixed | 7f1dfd837 |

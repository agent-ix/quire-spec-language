---
id: SR-16210
title: "spec-review of FR-027 and FR-100 refusing a 0-draft source with unknown_edition (QSL PR 1303)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@939e5d9d; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/functional/FR-027-export-compiled-native-package.md; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/test-cases/TC-105, TC-430, TC-435, TC-446, TC-450"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-430
    type: references
---

# Spec review of the 0-draft unknown_edition refusal (QSL PR 1303)

## Summary

Ticket: STD-343 (spec gap for QSL-836). One spec-review pass over
`git diff origin/main...939e5d9d`, checked against the standing rule that
the spec lane never drops a feature.

Oracles: ADR-012 §15.8 step 2; ADR-011's 2026-09-24 ruling, §7.3 M-6a and
M-6c rows, OQ-1 to OQ-3 and ADR-011-OQ-2; QSL-498's slice plan (comment
5088b8d1: replacement map, "behaviour with no replacement", planner ruling
R8) and QSL-836; `src/command.rs` and `qsl-cst/src/lexer.rs` at the head.

Gates: `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation`
exits 0 at the head. origin/main (71a080048) is an ancestor of the head. No
other open QSL PR touches any changed file, and the PR adds no ids.

Ruling check. The deletion of the `0-draft` compile and run routes is
already ruled. ADR-012 §15.8 step 2: "M-6c retires the `0-draft` native
path as a whole: FR-100's `0-draft` route of CLI `run`, native `compile` and
`lower`, `NativePackage` and the native-linked-package/1 reader". ADR-011's
2026-09-24 ruling deletes native compile with M-6c. QSL-498's replacement
map pairs native compile of `0-draft` with the `1-draft` spine compile, and
native run with `spine::run_clause`. It lists `0-draft` sources as "retired
by decision, not replaced". On main, `src/command.rs` already refuses a
native-edition run ("native run is retired"). So `0-draft` native compile
and run, native-linked-package/1 output and its `NativePackage::read_verified`
reread (FR-027-AC-1, AC-6, FR-020) are covered by a ruling. Two struck
criteria are not: FND-004 and FND-005.

Examined, clean:

- FR-027 Behavior and AC-7, FR-100 Behavior, the refusal table and AC-2:
  `unknown_edition` at stage `profile` at the literal, exit 20, empty
  stdout, whatever members the request carries. This is the same refusal
  the existing `7-draft` rows give. QSL-498's B2b text says "at S1 (spine)".
  The PR's `profile` stage matches the existing edition reader.
- No-header routing to the spine: `qsl_cst::declared_edition` returns `None`
  for anything but `language "ix:native" edition "<e>"`, and the CST
  parser has `unknown_language` (qsl-cst parser.rs:924), so AC-7's
  `invalid_syntax` and `unknown_language` at stage `source` can be met. The
  PR body says this "matches what `declared_edition` returning `None` already
  leads to". That is inaccurate: on main `None` maps to `Edition::Native`
  (src/command.rs:586-588). The default is still flagged as the author's
  call.
- FR-027-AC-3 narrowed to `1-draft`, AC-10 and AC-12 losing their
  `0-draft` halves, TC-105, TC-435, TC-446 and TC-450 rewrites: consistent
  with the new Behavior, and no remaining spec text references a struck AC.
  Test tags on the struck ACs (`tests/it/compile_command.rs:30,98,233,264`)
  are left for the QSL-836 coder, as the PR body states.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-026 still owns the route FR-100 now refuses. Its Description says that a native-run/1 request "whose program source declares `0-draft` or no edition" compiles and executes the selected clause, and that "This requirement owns the `0-draft` route, which also takes a source declaring no edition". After this PR, FR-100 refuses `0-draft` with `unknown_edition` and sends a source with no header to the spine. The same request now has two opposite specified outcomes. This must be fixed in this PR, not as a follow-up: strike FR-026's route sentences and keep it as the owner of the native-run/1 envelope and limits that FR-100 cites. | spec/functional/FR-026-run-standalone-native-workflow.md:20-32; spec/functional/FR-100-run-a-named-function-through-the-spine.md:47-57,468-480 | wrong-requirement |
| FND-002 | high | TC-430 is edited in this PR, yet step 1 and its expected result still require that a native-run/1 request with a model source and a snapshot selection "runs" and renders their labels (FR-026-AC-6). Under FR-100 as changed, no edition can do that: `0-draft` and no-header sources refuse, and a `1-draft` request carrying `snapshots` refuses `invalid-request` (FR-100 refusal table). Fix in this PR: retarget FR-026-AC-6 and TC-430 steps 1 and 2 to a `1-draft` request that FR-100 admits, or strike them with FR-026's route. | spec/test-cases/TC-430-native-requests-and-outputs-carry-the-four-source-labels.md:24-30,40-45; spec/functional/FR-026-run-standalone-native-workflow.md:123 | wrong-requirement |
| FND-003 | medium | FR-031 still says a native-run/1 request that selects program extraction compiles its Markdown clause and "execute[s] the existing runtime workflow". The extraction request's program source is the Markdown document. It has no `ix:native` header, so FR-100 as changed sends it to the spine, where S1 refuses it. FR-031 and the extraction half of TC-430 (step 3) now contradict FR-100. QSL-835 (B2a) deletes `command::extraction`. Either strike FR-031's run route here, or state that this PR merges after B2a's spec change. | spec/functional/FR-031-run-extracted-native-source.md:8-10; spec/test-cases/TC-430-native-requests-and-outputs-carry-the-four-source-labels.md:30-34 | wrong-requirement |
| FND-004 | high | FR-027-AC-4 is struck as a whole, but its second sentence is not `0-draft` behaviour. "The same request without `identity` refuses with `invalid-request` and exits 20" is native-compile/1 request intake, which a `1-draft` compile still performs. FR-027's Description still requires both FR-001 labels on every selected source. No remaining AC or TC covers a compile request missing a label: TC-430 steps 3 and 4 are deleted, and FR-026-AC-6 covers run only. This is a dropped criterion for live behaviour, and no ruling covers it. Fix: keep a `1-draft` form of it, for example in FR-027-AC-3 or AC-5 (TC-435), so that a `1-draft` request without `identity` and one without `authority` refuse `invalid-request` with exit 20. | spec/functional/FR-027-export-compiled-native-package.md:21-25,109 (struck row); spec/test-cases/TC-430-native-requests-and-outputs-carry-the-four-source-labels.md (struck steps 3-4) | missing-requirement |
| FND-005 | high | FR-027-AC-2 is struck as a whole, but most of it applies to a `1-draft` compile. It requires that a directory holding only the selected sources and the request compiles, and that a native-run/1 request or unexpected runtime fields given to `compile` refuse at the command boundary. Behavior still requires both: "select native-compile/1 before decoding request fields" and "without reading or constructing runtime artifacts". The narrowed AC-3 covers only stale source, syntax, `dependent_bytes`, arity and output failure, and the TC-105 rewrite drops the fresh-directory and format/runtime-field steps. The test that protects this (`incompatible_request_and_static_failures_emit_no_package_bytes`, `Change::Format` and `Change::Runtime`) loses its criterion and is due for deletion in QSL-836. No ruling covers it. Fix: keep a `1-draft` form of AC-2 and the matching TC-105 steps. | spec/functional/FR-027-export-compiled-native-package.md:92-104,107 (struck row); spec/test-cases/TC-105-standalone-package-export.md:17-35 | missing-requirement |
| FND-006 | high | The ADR-011 "Ruling (2026-09-24)" bullet, which records an owner ruling on QSL-8, is rewritten in place. The recorded text, "Native `compile`, `lower` and native-compile/1 are deleted with M-6c", now reads "Native `compile` (the `0-draft` arm of native-compile/1) and `lower` are deleted ... native-compile/1 stays ... From M-6c a `0-draft` source ... refuse it with `unknown_edition`". That is lead and planner content (QSL-498 R8 "Compile stays for 1-draft"; this PR's decisions 1-2) written under the owner's ruling. The substance is sound: ADR-011 OQ-2, amended 2026-09-26, already names native-compile/1 as the spine compile request, and ADR-012 §15.8 retires `0-draft`. The form labels the lane's call as the owner's. Fix: restore the bullet verbatim and append a dated amendment line in the record's own style ("Amended <date>: ... lead default, pending owner confirmation (STD-343)"), or record Peter's confirmation before merge. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1446-1461 | wrong-requirement |
| FND-007 | low | FR-027's title, "Export a compiled native package from selected source files", names the output this PR removes. After the strike, FR-027 writes only `quire.checked-package/v2` bytes for a `1-draft` source. Fix in this PR: retitle it, for example "Compile a 1-draft program source into a checked package". | spec/functional/FR-027-export-compiled-native-package.md:3 | wrong-requirement |

## Verdict

Not mergeable yet. Removing `0-draft` native compile and run is ruled
(ADR-012 §15.8 step 2, ADR-011's 2026-09-24 ruling, the QSL-498 plan), and
the `1-draft` spine replaces the compile route. However, the PR drops two
criteria for behaviour a `1-draft` compile keeps (FND-004, FND-005). It also
leaves FR-026 and TC-430 contradicting FR-100 (FND-001, FND-002; these are
real contradictions, not follow-ups) and rewrites a recorded owner ruling
in place (FND-006). FND-003 and FND-007 belong in the same fix round.

## New findings (disposition pass 1)

Re-review of head fc9f3077 (fix 7efcd82e9, with origin/main merged). The
fix retires FR-011, FR-019 to FR-023, FR-028, FR-031, NFR-007, IT-003,
IT-007, their test cases, Plan-007 and the Plan-009 native tasks, and adds
ADR-011 "Native-lane requirements M-6c retires". Each row of that table was
checked against its named holder:

- FR-019 to FR-021 against FR-278, FR-264 and FR-096; this is the
  NativePackage retirement ADR-012 §15.8 step 2 rules.
- NFR-007 and IT-007 against FR-264's I2 read limits and FR-098's recompile.
- FR-022 and FR-023 against FR-109's I3 source and `run_clause` over FR-106
  snapshots and invocations, with frames per FR-115 and FR-312.
- FR-028 against FR-098, which recompiles and requires the `package_id` to
  be equal.
- FR-011 and IT-003 against FR-030 and FR-108-AC-5.

These rows hold. The NFR-011 and NFR-012 defaults keep their values
(16777216, 100000, 268435456).

No link in spec/ or plan/ points to a deleted file. The `spec/spec.md`
index has no missing target. `make check-spec-validation` exits 0. The PR
body's retag list matches the trace tags in the tree. The PR adds no ids.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-008 | high | FR-031 is retired, and the ADR-011 row says it is held by "FR-312's clause run of an I3 source". FR-312 has no I3 or extraction member: "Neither forms a `ClauseRunRequest` any other way". The holder is FR-109's library `run_clause`. FR-109 and FR-108-AC-5 do not hold every FR-031 behaviour. Not held: the report naming both the body identity, with the digest of the extracted body bytes, and the original document identity; an original byte or line limit ending a run incomplete with a fresh retry; refusal of an `extraction` member at the command. FR-109:104-107 still cites "the CLI's extracted-clause join", which no requirement now defines. Open PR #1313 (STD-344) rewrites FR-031 as exactly this spine Markdown clause run, with these criteria. Deleting it here drops them with no equal replacement. Fix: do not retire FR-031 in this PR. Point the ADR-011 row at FR-031 as rewritten by STD-344 (holder FR-109), and fix FR-109's "CLI's extracted-clause join" sentence. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md (FR-031 row); spec/functional/FR-312-read-a-clause-run-request-and-run-it.md:26-35; spec/functional/FR-109-run-a-state-clause-through-the-spine.md:104-107 | missing-requirement |
| FND-009 | high | FR-030-AC-2 is struck. Its success half is consumer behaviour that stays: Successful and failed native compilation retain Quire's original availability, complete clause population and diagnostics. FR-030's Outputs still say the consumer yields "the unchanged complete ClausesOutcome" and that the unchecked-language advisory and lossy availability "remain observable in the ClausesOutcome". No AC asserts this on success: AC-3 covers failures only. TC-108 also dropped "Compare retained extraction metadata with the actual producer output". This narrows a feature. Fix: keep a consumer form of AC-2. A successful extraction yields an ExtractedSource whose ClausesOutcome equals Quire's unchanged, including its availability, complete population, diagnostics and the unchecked-language advisory. Add the matching TC-108 step. | spec/functional/FR-030-consume-quire-extraction.md:28-33,72-76; spec/test-cases/TC-108-quire-native-workflow.md | missing-requirement |
| FND-010 | medium | FR-026's rewritten Outputs name `native-run-result/1` as the intake command-error envelope of both requests. FR-027 says compile's envelope is `native-run-result/2`. FR-267 says the change that deletes native `run` also deletes `native-run-result/1`. FR-100:408-424 uses /1 only "until the change that lands FR-100's clause runner", and FR-312 identifies that change as M-6c's deletion of native `run`, which is what this PR specifies. So run and compile intake failures now have conflicting envelope formats. Fix: name FR-267's `native-run-result/2`, or reuse FR-100's transition wording. | spec/functional/FR-026-run-standalone-native-workflow.md:53; spec/functional/FR-027-export-compiled-native-package.md:78; spec/functional/FR-267-write-run-results-as-native-run-result-2.md:36,49-50 | wrong-requirement |
| FND-011 | medium | Three open spec PRs modify files this PR deletes or rewrites. #1313 (STD-344) touches FR-031, TC-109, TC-430, FR-030, FR-108, FR-109, FR-100, TC-108 and TC-450. #1321 (STD-352) touches FR-026, FR-027, FR-028, TC-105 and TC-106. #1326 (STD-355) touches FR-026, FR-027, FR-028, FR-100, FR-267, FR-312 and TC-103 to TC-105. #1321 and #1326 still specify FR-026's `0-draft` route and FR-028, so whichever lands second either reintroduces the contradiction FND-001 removed or conflicts with these deletions. Fix: the lead sets a merge order and rebases the later PRs. FR-031 is resolved through FND-008. | open PRs #1313, #1321, #1326 | wrong-requirement |
| FND-012 | low | spec/reviews/dependency.md is edited by this PR, but it still lists the retired FR-011 in its edge table and graph. Fix: drop the FR-011 rows and nodes. | spec/reviews/dependency.md:70-71,87,104-105 | wrong-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-026 is now "Read a native-v1 request and its selected files". Only a `1-draft` program reaches a runner or compiler, and every other edition refuses `unknown_edition`. The `0-draft` route sentences are gone. |
| FND-002 | fixed | TC-430 runs a `1-draft` request over `tests/fixtures/spine-compile.native` with `call` `seven`, and expects `spine-run-result/1` `source` to carry the two labels. FR-026-AC-6 matches. |
| FND-003 | fixed | FR-031 no longer specifies a native run that contradicts FR-100, because it is deleted. The deletion itself is FND-008. |
| FND-004 | fixed | FR-027-AC-3 now includes "a program source without `identity`, and one without `authority`, refuses `invalid-request` and exits 20". TC-105 step 3 covers it. |
| FND-005 | fixed | FR-027-AC-2 has a `1-draft` form: a source-only directory compiles, and a native-run/1 request or `snapshots`, `invocations`, `selection` or `package` refuse `invalid-request` without reading a model or runtime file. TC-105 steps 1 and 2 cover it. |
| FND-006 | fixed | The QSL-8 bullet is restored word for word. A separate "Amended 2026-10-22 (STD-343; lead default, not an owner answer; Peter may veto)" bullet follows it and cites OQ-2. |
| FND-007 | fixed | The title is now "Compile a 1-draft program source into a checked package". |
| FND-008 | fixed | FR-031 and TC-109 are restored to their main text and this PR no longer edits them; STD-344 (#1313) rewrites them as the spine Markdown clause run. The ADR-011 row now names FR-031 as STD-344 rewrites it, held by FR-109's library `run_clause` over FR-030's extracted body, and lists its three behaviours: the report naming the body identity with the extracted body's digest and the original document identity, an original byte or line limit ending a run incomplete with a fresh retry, and the command refusing an `extraction` member. FR-109's "CLI's extracted-clause join" sentence is gone. FR-032's FR-031 link is restored. Task-030 stays deleted: #1313 does not rewrite it and its standalone-command scope is retired in both PRs. FND-003's native-run contradiction in FR-031's main text is removed by #1313, which merges first. |
| FND-009 | fixed | FR-030 and TC-108 carry #1313's text. FR-030-AC-2 states the consumer form: a successful extraction yields an ExtractedSource whose ClausesOutcome equals Quire's output unchanged, with its availability, complete clause population, diagnostics and the unchecked-language advisory, and that ClausesOutcome is unchanged after a successful and a failed spine compile. TC-108 compares the ExtractedSource's ClausesOutcome with `quire-rs`'s own over a population holding a second clause and an unchecked-language clause. FR-030-AC-1 is kept in #1313's spine form. |
| FND-010 | fixed | FR-026's intake failures carry a `native-run-result/2` command-error envelope (FR-267), the format FR-027 gives compile intake. FR-100 no longer attributes `native-run-result/1` to FR-026. |
| FND-011 | fixed | Merge order: #1321, #1326, #1313, then #1303. FR-026, FR-027, FR-100, FR-267, FR-312, TC-104, TC-105, TC-108, TC-430, TC-435, TC-446 and TC-450 carry those PRs' text plus only this PR's retirement, with #1326's FR-100/TC-450 overlap with #1313 joined. A simulated landing in that order, with each conflict resolved to this PR's side and FR-028, TC-103 and TC-106 kept deleted, reproduces this PR's files exactly and passes `make check-spec-validation`, with no `0-draft` route or FR-028 left. |
| FND-012 | fixed | spec/reviews/dependency.md no longer lists FR-011 in its edge table, graph or topological order. |

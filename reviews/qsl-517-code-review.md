---
id: SR-12057
title: "Code review of quire-spec-language PR #927: the spec-versioning refinement gate"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2eaaa4cd5c5a462053415cc7600f1ec8926d6ada; PR #927: xtask/src/refinement.rs, xtask/src/refinement/{versioning,compare,report}.rs, xtask/src/{main,lib,error}.rs, xtask/Cargo.toml, Makefile, xtask/src/ci_feature_lane_tests.rs, xtask/src/conformance/tests.rs, xtask/tests/refinement_versioning.rs, tests/fixtures/refinement/versioning/config-version-identity/"
review_set: subset
---
# Code review of quire-spec-language PR #927

## Summary

Ticket: QSL-517. Code review with the rust-review lane folded in. Read-only
review of the diff `origin/main...2eaaa4cd5` (18 files). No build was run.
The author's focused run on 852dcd1e2 was fmt, clippy -D warnings, 14
integration tests and string-edge, all exit 0.

The FR-343 comparison in `compare.rs` matches the table row by row. The
2352-case unit test is a real independent oracle: `TABLE` is a hand-written
literal, and its two helpers (`applies`, `absent_superseding`) only expand the
stage and kind columns. Nothing in it calls the gate's code. The verdict order
and the exit codes 0/10/21/22/30 match FR-344. The one exit function works:
`run()` returns `Result<Outcome>`, and `exit()` in `main.rs` is the only
place that chooses the process code. The new `Code::Refinement` maps to 30.
The USAGE rewrite removes the stray `\\` and the duplicate `test-suites`
line. The `CI_CHECKS` addition and the two CI-target test edits list
`refinement-versioning` in the same place, between `checked-input` and
`cargo-deny-bans`. The new xtask dependencies (`qsl-foundation`,
`qsl-replay`, `qsl-semantics`, `quire-canonical`) form no cycle: none of
those crates depends on `xtask`, and the root crate names `xtask` only as a
dev dependency. The ADR-011 §6 allocation row for `xtask` ("depends on no
stage module") is now stale against FR-340, which requires the
`run_clause` dependency. The fix is a one-line ADR-011 amendment, not a code
change. The binary search in the TC-865 test is deterministic, because
accounting is deterministic and admission is monotone in the budget.

## Verdict

Changes requested: two high findings, both repository-rule duplication, and
one medium. Note: an unpushed local commit 3d2a36419 in the author's worktree
appears to address FND-001 and FND-002. It was not reviewed here. The
disposition pass checks it once it is pushed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `observation()` in the xtask repeats the body of the root crate's private `src/command/clause.rs::observation` line for line: the canonical read, the `sha256-jcs` digest, the format check, the four identity labels and the role. The duplicated logic decodes FR-106 documents and will drift, for example on the next format or label change. Clean fix: move the pure bytes-to-`SuppliedObservation` decoder into `qsl_replay::spine`, beside `read_clause_run_request` (for example `SuppliedObservation::from_document(key, bytes)`), and have both callers use it. `request.rs` already says the adapter owns paths and `qsl-replay` owns the FR-106 mapping. Related: `selection_parts` and the `post_state` mapping in `Limits::of` also mirror `clause.rs`. | xtask/src/refinement/versioning.rs:616-653; src/command/clause.rs:24-81 |
| FND-002 | high | `tests/fixtures/refinement/versioning/config-version-identity/model.semantic-ir.json` is a byte-identical 7582-byte copy of `examples/config-version/model.semantic-ir.json` (`cmp` identical). `unit.native` also hard-codes that package's digest `ecfa214e...`. If the example package changes, the copy and its digest go stale and the corpus still passes, so the drift goes undetected. The copy is forced by FR-340 ("read no ... path outside the pair's directory"), which `read_in` enforces by canonicalizing. Clean fix: let `pair.json` name a repository file (for example a `repo:<path>` form resolved against the workspace root, still refusing absolute paths and escapes), amend FR-340 and TC-860 to match, and delete the copy. | tests/fixtures/refinement/versioning/config-version-identity/model.semantic-ir.json; xtask/src/refinement/versioning.rs:441-475 |
| FND-003 | medium | `SelectionName` names a case only by clause name and observation key (Clause), or by function name and snapshot key (Function). It drops `self`, `anchor`, `parameters` and `arguments`, all of which are part of the FR-109 selection. FR-340 says each case "SHALL be named in every result by the pair's two RawSourceRefs and its selection". Two cases of one pair that check `ParentOrder` for `self: child` and `self: root` over one snapshot, or `sameIdentity` with different arguments, get identical names. A regression in one cannot be told apart from the other in the report, and FR-344's by-name order falls back to `pair.json` order between them. Fix: carry the full selection in `SelectionName`, with self, anchor, parameters and arguments as ordered members. | xtask/src/refinement/report.rs:75-100; xtask/src/refinement/versioning.rs:692-707 |

## Dispositions

Round 1, reviewed at `fd2fa6bd42ce32f3c7739a8510122d9fd8b989fd` (commits
3d2a36419 and fd2fa6bd4 on top of 2eaaa4cd5; local branch, not yet the PR
head).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | fd2fa6bd4 |
| FND-002 | fixed | 3d2a36419 |
| FND-003 | fixed | fd2fa6bd4 |

FND-001: `SuppliedObservation::from_document` (3d2a36419),
`ClauseRunSelectionWire::into_parts` and `ObservationLimitsWire::resolve`
(fd2fa6bd4) live in `qsl-replay/src/spine/request.rs`. The root command
(`src/command/clause.rs`, `wire.rs`) and the xtask both call them. Root-crate
behaviour is unchanged: the bodies, the serde attributes (`deny_unknown_fields`,
`from_object`, `deserialize_objects`) and every refusal member and reason
string (`observations.<key>.*`, `selection`, `observation_limits.post_state`)
are carried over verbatim, and `invalid(error.member, error.reason)` maps
them back to the same `RunCause::ClauseRequest`.

FND-002: the copy is deleted. `pair.json` names
`repo:examples/config-version/model.semantic-ir.json`, and FR-340 and TC-860
are amended. The model digest literal still sits in the corpus units. That
is acceptable: if the example package changes, the digest no longer
matches, the prior compile is refused, the result is `tool failure`, and
the lane exits 30. Drift turns the lane red; it is never silent.

FND-003: `SelectionName` now carries `self`, `anchor` and `parameters` for a
Clause and `arguments` for a Function. The test
`cases_differing_only_in_their_self_object_have_distinct_names` pins it.

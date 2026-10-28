---
id: SR-1711001
title: "code review (with rust-review) of PR #1711 (QSL-1181 slice 1): IR bump and typed CheckedTerm::Refinement builder"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1711 git diff origin/main...9a6f966b1; qsl-semantics/src/check/refinement.rs, the five quire-contract-model manifests, Cargo.lock, qsl-package/src/emit/tests/{owner_model,admission_corpus}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-375
    type: reviews
---
# SR-1711001: code review of PR #1711 (QSL-1181 slice 1)

## Summary

Ticket: QSL-1181. Head reviewed: 9a6f966b1. Its parent is main 8e58b70dc. QSPEC_DIR is a fresh worktree of QSpec origin/main (3e4ccfd5).

**The commit reverts seven merged PRs (FND-001).** `git diff origin/main...9a6f966b1` touches 124 files (+656/−6921). The head's tree is main at 2c4aeda3c (#1705) plus the nine slice files; `git diff 2c4aeda3c 9a6f966b1` shows only those nine. Committed on top of 8e58b70dc, it undoes everything merged since #1705:
- #1706, #1708, #1667, #1671, #1690, #1707 and #1694;
- among the files removed: zone_check/refine.rs, qsl-route reductions, protocol_terminal tests, TC-1184, and the reviews/ SR files.

qsl-package runs 345 tests on the head and 350 on main.

**The intended slice is correct.** I reviewed it as `git diff 2c4aeda3c 9a6f966b1`:
- **Typed builder, byte-identical:** my probe in `a_row_empty_refinement_identity_excludes_its_source_region` rebuilt the old `json!` body from the emitted node. The emitted body equals it exactly. The JCS SHA-256 of `{version, semantic_type, old body}` is `0b89e85d…aa951a9`, equal to the emitted `node_id`. The `None` optional members are not serialized.
- **IR bump:** the lock moves from c103a5e to 796609b. That is #627's head (`qsl-1181-refinement-exports`), 1 ahead of and 0 behind df9ed4a; df9ed4a is 12 commits past c103a5e. The one changed test, `owner_model::each_defect_of_the_edge_is_refused_where_the_reader_reaches_it`, now expects `InvalidModelBinding … (Some(MalformedDeclaration))` for a non-model target, as QSpec TC-946 OM-07 states, and it passes.
- **Branch pin:** all five manifests name `qsl-1181-refinement-exports`. The body says: "Do not merge before agent-ix/quire-contract-ir#627 merges ... this PR repins every one of them to `branch = "main"` and re-locks". IR #627 is OPEN. That is acceptable until #627 merges; it is a merge precondition, not a finding.
- **Pre-existing failures:** `conformance_c14_source_map_lookup_over_qspec_positive_fixtures` and `conformance_i2_read_over_qspec_checked_package_v2_fixtures` fail identically on main 8e58b70dc and on the head, at checked_v2/tests.rs:1539 and :1702. Every other qsl-package test passes on both. This PR does not cause them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | **The PR reverts seven merged PRs.** The commit is the nine-file slice applied to a stale tree (main at #1705), committed on main 8e58b70dc. Merged as it is, it deletes #1706, #1708, #1667, #1671, #1690, #1707 and #1694. The body's gate results are for that stale tree. **Fix:** rebuild the branch as main plus only the slice (`git diff 2c4aeda3c 9a6f966b1`, nine files), then re-run the gates. The bump then also has to build against the code those PRs added. | git diff --stat origin/main...9a6f966b1 (124 files); git diff --stat 2c4aeda3c 9a6f966b1 (9 files) |

## Verdict

**Not mergeable.** FND-001 must be fixed first. The slice's own code is clean: the builder output is byte-identical, the owner_model test matches TC-946 OM-07, and the branch pin stays until #627 merges, with the repin promised in the body.

## Dispositions

Round 1 (head 3cdd54dcc).

Round 1. Rebuilt and force-pushed: head 3cdd54dcc on main 918e624f7.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `git diff --stat origin/main...3cdd54dcc` now lists exactly the nine slice files (+155/−43), and nothing from #1706/#1708/#1667/#1671/#1690/#1707/#1694 is touched. |

I re-ran the byte-identity probe. The emitted body equals the old `json!` body, and the old preimage's JCS identity equals the emitted `node_id` (`0b89e85d…aa951a9`). The owner_model test passes.

**Verdict: mergeable once IR #627 merges** and the five manifests are repinned to `branch = "main"` and re-locked, as the body promises. The PR still names `qsl-1181-refinement-exports` today.

Gates at 3cdd54dcc (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=~/dev/quire-specification at 308596b0):
- fmt: exit 0.
- workspace clippy `-D warnings`: exit 0.
- qsl-semantics `refinement`: 88/88.
- qsl-package: 349 pass, 2 fail (c14, i2). Both fail identically on main 918e624f7: `positive-model-tuple-union.json` (QSpec #468, STD-519) is refused `MalformedWire` at `/semantic_graph/nodes/0/owner` by both IR pins. #470 fixed the earlier fixture; this is a new IR/QSpec disagreement, not caused by this PR.


## Delta review, round 2 (IR main repin)


Delta review, round 2, of the QSL-1181 stack after the IR main repin. Correctness only. **All three are mergeable.** No new findings.

| PR | Head | Verdict | What I checked |
| --- | --- | --- | --- |
| #1711 | 3cf9ba503 | mergeable | Pin, `ExecutionPointName`, hybrid reader and `ExecutionPoint`. Details below. |
| #1716 | 094574159 | mergeable | `run_nodes` keying of written controls. Details below. |
| #1723 | 5c2af3dd7 | mergeable | The `abstract_protocol` loop now sits after main's attempt-protocol keying (`protocol_nodes_with_attempts`) and the concrete-protocol loop, and before `class_catalog_keys`, so an attempt-bearing abstract clause gets its key. |

**#1711:**
- **Pin.** All five manifests are `branch = "main"`. Cargo.lock locks 976e25eb, which is IR #627's merge commit (MERGED). No branch pin is left.
- **`ExecutionPointName`.** It derives `Eq` over `Box<str>`, so `AcceptedPoint`'s `Hash` over `as_str()` agrees with `Eq`.
- **Hybrid reader.** IR's `Serialize` writes the operation's `record`, with the execution point merged in when present. Reading `laws`/`member` from `serde_json::to_value` therefore sees the same members the old `get` did. That is acceptable as a stopgap; it costs one clone per hybrid node read. A public `record()` accessor in IR would remove it (IR keeps `record()` and `get()` `pub(super)`). That is a follow-up to ask IR for, not a blocker.
- **`ExecutionPoint`.** The `CheckedExecutionPoint` to `ExecutionPoint` change in emit.rs is a straight rename.
- **Pre-existing failure.** `composed_domain_models::domain_operation_parameters_and_result_bind_and_type_check` fails ("FillReady: two invocation parameters"). It fails identically on main 2d901bee4, this branch's base, and passes on current main 937c0ca91, so it is not caused here.

**#1716:**
- **Keying.** `run_nodes` returns every written control's key by `ProtocolNodeId`, and the clause inserts `finish` beside them.
- **Probe.** I gave `catalog_refinement` an `event Seen by w` control. The class `Local::Seen` maps to the `control` node declared `[Local, run, Main, Seen]`, and `Local::Done` maps to the finish obligation.
- **No double-keying.** All 23 wire node ids are unique, and `finish` is a different `ProtocolNodeId` from every run control.
- **Read-back.** The ignored IR-874 read-back still fails at IR main, so the ignore stands.

Gates (locked-build, jobs 4, `--all-features`, debug info off):
- **#1711:**
  - fmt: exit 0.
  - qsl-package `hybrid`/`timed_source`/`receiver_frames`/`execution_point`: 28/28.
  - qsl-semantics `execution_point`/`state_clause`/`node_key`: 38/38.
  - root `native_checking`/`package_construction`/`composed`/`domain_protocol`/`mapped`: 298 pass, plus the 1 pre-existing failure.
  - workspace clippy `-D warnings`: exit 0.
- **#1716:**
  - qsl-package `refinement`/`catalog`/`protocol`: 60 pass, 1 ignored.
  - qsl-semantics: lib 177, it 98, all passing.
- **#1723:**
  - qsl-package `refinement`/`catalog`/`protocol`/`internal_list`: 63 pass, 2 ignored.
  - qsl-semantics: lib 178, it 98, all passing.
  - clippy `-p qsl-semantics -p qsl-package`: exit 0.

---
id: SR-1723001
title: "code review (with rust-review) of PR #1723 (QSL-1181 slice 3): row-empty protocol subjects in V2"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1723 git diff origin/qsl-1181b-model-catalog...0c943010a (7 files); qsl-package/src/emit.rs, qsl-semantics/src/check/{mod.rs,refinement.rs,refinement/internal_list.rs} and tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1723001: code review of PR #1723 (QSL-1181 slice 3)

## Summary

Ticket: QSL-1181. Head reviewed: 0c943010a, a single commit on qsl-1181b-model-catalog (32cbba757). The scope is exactly the 7 files listed; nothing else changes.

- **`abstract` and `protocol`.** `abstract` stays the abstract `model_root`. `protocol` is `RefinementKey::Local(clause)`, and the clause is added to `dependencies`.
- **Clause lookup.** `abstract_protocol` is filled by name over the unit's protocols. That is the same lookup `resolve_protocol` uses, and that function has already refused a clause that does not act on the abstract model. A clause with no node leaves `abstract_protocol` as `None`, so the refinement is not emitted.
- **Gate.** `v2_row_empty_body` admits a protocol subject with a non-empty `abstract_internal` (FR-375-AC-12), and still refuses one for a model subject.
- **STD-524.**
  - `cend` now has empty `binder_types`.
  - `fork` and `join` now also get `Vec::new()`. A `parallel` node has no binder record, so nothing changes.
  - `effect` and `commit` keep their bare path and binder record.
  - `await` stays its `timeout`; that is already asserted by FR-147-AC-8's test and `an_await_needs_a_timeout_row_and_its_matched_event_a_row`.
- **Identity.** `abstract_internal` uses the alias-free spellings, sorted at check time. My probe (SR-1723002) shows it enters the node id.
- **Read-back.** `a_row_empty_protocol_subject_reads_back` is `#[ignore = "IR-874 ..."]`, the accepted pattern, and comment 3b349555 records when the ignore comes off. I re-measured it: IR refuses at `class_catalog/abstract/0`, as stated.

I found no code defects.

Gates at 0c943010a (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=~/dev/quire-specification 48373530):
- fmt: exit 0.
- qsl-semantics `refinement`/`protocol`/`catalog`: lib 177, it 98, all passing.
- qsl-package `refinement`/`catalog`/`protocol`/`owner`/`activate`/`third`: 57 pass, 2 ignored (IR-874).
- `cargo clippy -p qsl-semantics -p qsl-package --all-targets -D warnings`: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean.

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

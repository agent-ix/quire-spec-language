---
id: SR-763
title: "QSL-279 code review (with rust-review lane) of PR 504"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; examples/config-version/cases.rs; qsl-package/src/emit.rs; qsl-replay/src/spine/clause/tests.rs; qsl-semantics/src/check/identity.rs; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/lowering/state.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/node_key/mod.rs; qsl-semantics/src/check/node_key/tests.rs; qsl-semantics/src/value/member.rs; tests/it/config_version.rs; qsl-semantics/src/model/intake.rs (resolve_frame_array; unchanged); quire-contract-ir crates/quire-contract-model/src/checked_package/v2/{vocabulary.rs,mod.rs} (dependency, unchanged)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
---
## Summary

Ticket: QSL-279. PR: quire-spec-language#504.
Methods: code-review with the rust-review lane folded in.

The dispatcher asked for eight checks. Results:

1. Node-key totality. `SemanticTerm::Frame` is handled in `for_each_key`,
   `map_keys`, `PreimageTerm` and `Walk::term`. `Member::StateClause` is
   handled in `map_member`, `declaration()` and `to_wire`.
   `Operator::StateClause` is handled in `semantic_form`. None of these
   matches has a `_` arm, so a missing arm is a compile error. The preimage is
   JCS-encoded, so object key order does not matter. Array order is fixed at
   construction: `modifies` is sorted by `(NodeKey, name)`, and
   `creates`/`deletes` by `NodeKey`. That is FR-340's order key. The frame node
   is never a draft (`frame_occurrence` refuses a pending frame), so
   `map_keys` never reorders it after sorting. Hashing is deterministic.
   Entries are not deduplicated (FND-001).
2. `CheckedClauseKind`. The old per-half injectivity cannot hold once three
   variants share a node pair and an operation. The new test checks
   injectivity and backward decode over triples built from `all()` for all
   seven variants. It also checks a fixed table for each variant, and that
   (`state`, `frame`) and a state-clause triple with no member decode to
   none. This is a real strengthening. Arm deletion and member swaps are
   caught by the fixed tables. The removed `from_wire_operation_identity` and
   `from_node_tag_and_semantic_form` had no other callers, including PR #503.
3. The `UnlocatedOccurrence` fix. The bug was pre-existing: the SR-751
   round-2 comment on QSL-278 and `reviews/26-09-27-qsl-278-gap-analysis.md`
   FND-003 reproduced it on main. The ordinal is still fixed by
   the `(DeclarationKey, name)` sort in `register_frame_occurrences`.
   `anchor.location` is used only for `declaration_key_of`/`insert` fault
   locations and for the occurrence's region. The first clause in source
   order supplies the location, which is deterministic.
4. TC-462 hard-codes its expected values: the kinds (`invariant`,
   `invariant`, `postcondition`), `"attemptUpdate"`, `versionNumber`, and
   empty `creates`/`deletes`. It does not locate ConfigVersion independently
   (FND-005). TC-463's stable-compile test does what its name says: two
   compiles, same bytes, same `package_id`.
5. The boundary cases are in `CASES`, so the exhaustive `expected()` match
   and the `actual_native_commands_...` loop (and the markdown-agreement test
   under `quire-extraction`) pick them up. The outcomes are distinct and
   discriminating. 0 and 1000 complete true. -1 and 1001 refuse with
   exactly `{invalid_runtime_input}`, where ParentOrder alone would evaluate
   true. The native `Int[0, 1000]` bound and the `versionNumber`-only frame
   were already on main (TC-110).
6. Fixture change. No QSL-278 test assertion changed. Every run_clause test
   uses versions inside [0, 1000] (1, 2, 5, 99), and none depends on
   `parent` being in the frame. All of them pass at head.
7. The `#[ignore]` is justified. I ran it with `--ignored`. The refusal is
   `InvalidSemanticGraph` at `/semantic_graph/nodes/3/body`, from IR's
   reader. In IR, `ApplicationOperator` has no
   `state_clause` (vocabulary.rs:345-366). That blocker is external. The
   doc text is partly wrong (FND-003).
8. The `BodyNames` Frame arm is correct against FR-340's dependency rule.
   The generic serializer needed no other change, because the body goes
   through `wire_body()` and the serde tag `term: "frame"`. No test covers
   the arm (FND-002).

PR #503 overlap: both PRs touch `qsl-semantics/src/check/mod.rs`, in
different hunks (#503 at about line 1029, #504 at 128 and 1122-1170). #503
does not call the removed `CheckedClauseKind` decoders and does not match on
`SemanticTerm`/`Member`. There is no semantic conflict. A rebase may need a
trivial merge.

Gates, all re-run by me with a fresh `CARGO_TARGET_DIR`
(scratchpad):
- `make ci` exited 0. The log has 93 `test result: ok` lines and 0
  `FAILED`. `ci-docs` (`RUSTDOCFLAGS=-D warnings cargo doc`) passed. That run
  covers fmt, workspace clippy (default and all features), the workspace
  tests, and `tests/it/config_version.rs`.
- TC-462, TC-463 (stable), the TC-462 triple test, the TC-250 table test,
  the member test and `actual_native_commands_...` each show `ok` in both
  the default-feature and the all-feature runs.
- `cargo test -p qsl-replay --lib s4_state_package_reads_back_through_i2 --
  --ignored` failed as the doc comment describes.

## Verdict

Changes requested. There are no high findings in the code itself. The
emission is correct for the fixture. Hashing is total and deterministic.
The claimed bug fix is real and changes only regions. FND-001 is a real
wire-conformance defect with a two-line fix. FND-002 leaves the emitter
change unguarded, and the only emitted-bytes check is ignored. FND-003 to
FND-006 are cheap to fix in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Frame entries are sorted but never deduplicated. FR-340 says "no member holds a duplicate entry", and IR's reader rejects a repeated entry (mod.rs:1268 "uniqueItems"). Intake keeps duplicates too: `resolve_frame_array` pushes every entry. Scenario: a domain package whose `attemptUpdate` frame lists `modifies: [versionNumber, versionNumber]`, or `creates: [X, X]`, compiles and emits a frame body that a conformant I2 reader refuses. Its node id also differs from the deduplicated frame's, although the meaning is the same. Fix: call `dedup()` after each `sort()` in `frame_node`/`frame_objects`, or refuse duplicates at intake. | qsl-semantics/src/check/lowering/state.rs:238-249; qsl-semantics/src/check/lowering/state.rs:323-334; qsl-semantics/src/model/intake.rs:2364-2404 |
| FND-002 | medium | No test exercises the new `BodyNames` Frame dependency arm. TC-462 reads the in-process `SemanticGraph`, not the emitted wire. The only emitted-bytes check (the I2 read-back) is `#[ignore]`d, and no qsl-package test emits a frame. Replacing the arm body with `{}` would keep every test green while the emitted frame no longer lists ConfigVersion in `dependencies`, which violates FR-340 and TC-462 step 2 ("the ConfigVersion node is in the frame's dependencies"). Fix: decode `compiled.emitted.bytes()` in TC-462 and assert the frame node's `dependencies` and wire `body` shape (`{term:"frame", modifies:[{kind:"field", declaration, name:"versionNumber"}], creates:[], deletes:[]}`). | qsl-package/src/emit.rs:315-330; qsl-replay/src/spine/clause/tests.rs:2676-2810 |
| FND-003 | low | The `#[ignore]` doc comment and QSL-307 both say the pinned IR has no `frame` member in its `BodyTerm` vocabulary. It does: vocabulary.rs:326 `Frame => "frame"`, and `StateForm` lists `state_clause`/`frame`/`operation_anchor` (vocabulary.rs:240-243). The measured blocker is `ApplicationOperator` lacking `state_clause`, which gives the refusal at `nodes/3/body`. The pinned frame body probably also expects the pre-STD-111 `modifies` entry shape (see SR-752 FND-001). The ignore is justified, but correct both texts so the QSL-307 bump checks the right things. | qsl-replay/src/spine/clause/tests.rs:2826-2846 |
| FND-004 | low | The comment in `checked_invariant_and_call_fault_both_report_the_same_internal_failure_shape` still says "This test file also has no compiled package with a precondition/postcondition clause ... any clause on `attemptUpdate` fails `spine::compile` (SR-751 FND-003, deferred to QSL-279)". This PR makes that false: the shared fixture now carries `post VersionUnchanged` and compiles. Update the comment, or try the `CallFailure::Fault` path now that it is reachable. | qsl-replay/src/spine/clause/tests.rs:2246-2251 |
| FND-005 | low | TC-462 never checks the ConfigVersion identity independently. The anchor's `context` reference, the frame's `modifies[0].declaration` and the frame's `semantic_type` are compared only with each other. If all three consistently name the wrong node (for example the population node, or the `versionNumber` value-type node), the test still passes. Fix: find the `model`/`object_type` node for `ConfigVersion` (by nominal/declaration) and compare against it. Also check that each invariant's anchor argument is that node, as TC-462 step 2 states. | qsl-replay/src/spine/clause/tests.rs:2716-2770 |
| FND-006 | low | Rust lane: `Member::StateClause { clause: &'static str }` is stringly typed. Only `kind_spelling` keeps it inside the three FR-341 values, and any caller could build `clause: "bogus"`. `qsl_forms::StateClauseKind` already exists: carry it, and spell it in `to_wire`, so an invalid kind cannot be represented (rust-style "domain newtypes"). | qsl-semantics/src/value/member.rs:90-98; qsl-semantics/src/check/lowering/state.rs:97-104 |

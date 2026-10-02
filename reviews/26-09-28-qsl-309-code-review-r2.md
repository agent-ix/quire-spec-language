---
id: SR-772
title: "QSL-309 code re-review of PR 510 after the SR-770/SR-771 fix round"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-forms/src/protocol_clause.rs; qsl-forms/src/syntax.rs; qsl-cst/src/grammar.rs (read only, protocol productions); qsl-semantics/src/check/protocol_clause.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/assemble.rs; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/lowering/state.rs; qsl-semantics/src/check/region.rs; qsl-semantics/src/check/refusal.rs; qsl-semantics/src/check/state_clause.rs; qsl-package/src/emit/tests.rs; qsl-replay/src/spine.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: reviews
---
## Summary

Ticket: QSL-309. PR: quire-spec-language#510. This
is the code-review re-review after the fix commits, with the
rust-review lane folded in. SR-770's own dispositions are appended to SR-770.

What was checked, and holds:

- **The HIGH-1 split is complete.** Every `ProtocolNodeKind` (16 variants) is
  classified by the exhaustive `covered_kind` match. Only `Sequence`,
  `Attempt` and `Finish` are covered. Every `ProtocolConstructKind` (7
  variants) refuses. I walked every alternative of the `ProtocolClause`,
  `Role`, `Activation`, `Control`, `EventNode` and `Finish` productions in
  `qsl-cst/src/grammar.rs` against the S2 walker. Every production is either
  recorded as a refused node kind or construct, or read by `content` (the
  `using` alias, `role R on M::T`, the `over` binder, record binders, `by`
  role, bodies), or read by FR-113/FR-114 (names, anchors, operation,
  `contracts`). `on origin` binds nothing. No production is walked past
  unrecorded.
- **The reviewer probes refuse.** Temporary tests were reverted and never
  committed. The original probe refuses, both alone and with a valid attempt
  beside it. So does every checked-group defect at once (`Nope::Input`,
  `Nope::Actor`, `Undeclared`, `1 + true`) beside a valid attempt. A lone
  `relationship` refuses, and so does `using Config`, `by Q` or a duplicate
  role. A bare `run attempt ...` root with no `sequence` checks.
- **HIGH-2 is real.** Removing the `Origin::ProtocolAttempt` arm reproduces
  `UnlocatedOccurrence { role: Generated }` in the AC-2 emission test.
  Removing the `Boolean` `Type` record reproduces it on a different node.
  The population-object record is FND-001 below.
- **FND-003 to FND-008 of SR-770 are fixed in substance.** There is one
  `state_clause::frame_record`. Lowering runs in one source-ordered pass. A
  `contracts` entry naming two clauses refuses `Ambiguous`, and a unit test
  calls `check` directly to reach that branch. Clause and attempt assembly
  errors are gathered together. `.get()`/zip replace indexing, and an empty
  slot is an `InternalFault`, never a skip. `attempt_spans` is
  `Option<Span>`. `contracts` is read between `[` and `]`.
- **The gate passed.** A fresh `make ci`
  (`CARGO_TARGET_DIR=target-309`) exited 0, with 93 `test result: ok` and 0
  FAILED. `string-edge` reported "no unmarked, unlisted string comparison or
  string match found". `ci-docs` was then forced fresh (lib roots touched)
  under `RUSTDOCFLAGS=-D warnings` and was clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The population-object `Type` occurrence in `Lowering::protocol_attempt` has no test. No committed test fails when it is removed. It also stayed green in two reviewer probes: the AC-2 emission fixture given a closed `config_history` population over `ConfigVersion`, and `attemptUpdate` moved onto `Sub` with the population over the supertype. In both, the object node was already placed through the frame and anchor. The fix note's "removing either half fails the test" holds for the `Origin::ProtocolAttempt` arm and the `Boolean` record only. The record is harmless (one extra source-map occurrence). Either add a test that needs it, or drop it. | qsl-semantics/src/check/lowering/state.rs:247-253 |

## Verdict

Approve. No high or medium code defect remains. FND-001 is low and does not
block merge.

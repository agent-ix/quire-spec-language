---
id: SR-12153
title: "Code review of quire-spec-language PR #989: C1b2 clause-checker integration of protocol node kinds"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f29f1ebc4f33bb512f205b4323b9351a73884ebc; PR #989 diff against origin/main (31 files, +5298/-285): qsl-forms/src/{lib,protocol_clause,protocol_control,syntax}.rs; qsl-forms/tests/it/protocol_{clause,control}_forms.rs; qsl-package/src/emit/tests.rs; qsl-replay/src/spine.rs; qsl-semantics/src/check/{assemble,lowering,mod,protocol_controls,protocol_obligation,protocol_slots,refusal}.rs; qsl-semantics/src/check/lowering/protocol.rs; qsl-semantics/src/check/protocol_clause/{anchors,coverage,graph,mod,types}.rs and tests/; qsl-semantics/src/check/protocol_controls/model_state_tests.rs; qsl-semantics/src/check/protocol_obligation/{cause,tests}.rs and tests/recorded.rs; qsl-semantics/tests/fixtures/protocol_obligations.rs"
review_set: subset
---
# Code review of quire-spec-language PR #989

## Summary

Ticket: QSL-850 (the dispatching brief's choice; the PR body names no
ticket, and QSL-849 "C1b: control constructs and channels" may be the
better owner). One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` at the reviewed sha. Read-only: nothing
was built or run, at the dispatcher's instruction.

Checked and clean:

- The control tree (`graph.rs`): children are read from the scope each
  control's own name opens, in authored order, on an explicit stack. The
  structural edges match the QSpec FR-464 table for `sequence`, `choice`,
  `parallel` (join edges only for joined labels), `repeat` (progress, skip
  for `exhausted`, back edge) and `await` (success and timeout paths). The
  cross-reference edges are send.out to receive.in, attempt.out to
  effect.in and the await anchor edges, with a compensation `after` read
  as the template's forward effect.
- Cycle refusal: Tarjan's algorithm on an explicit stack, over the relation
  without `RepeatBack` edges, one `definition-cycle` per non-trivial
  component naming its nodes in tree order.
- Progress: a reverse tree walk sees children before parents. Leaves,
  `await` and `parallel` step; `check` and `repeat` do not; a `sequence`
  steps when any child does and a `choice` when every case does. This
  matches FR-218 "Constructs" and ADR-027 PS-6, including the nested
  repeat's skip edge (AC-7).
- Binder scope (`protocol_slots::Flow`) follows QSpec FR-466 In/Out: a
  sequence threads Out to the next child, `join all` adds every branch's
  Out, `choice`/`repeat`/`await` add nothing past themselves, the matched
  event is readable on `then` only, and `finish` reads Out(root) plus its
  own binder. Binder names are unique protocol-wide (FR-113 shadowing), so
  the name-to-local mapping cannot alias two binders.
- Slot refusal order is model-state read, then out-of-scope read
  (`missing-name` at every read), then decision facts, as the module doc
  and FR-218 state. `check`, guards and `visible` entries bar model state;
  event constraints and `finish` may read `over`, matching FR-464-AC-8.
- The choice partition is proved over independent Boolean atoms. Treating
  correlated atoms as independent can only refuse a real partition, never
  admit a non-partition, so the proof is sound. It is charged against the
  S3 work meter before it runs, and 2^atoms past `u64` is refused as a
  limit rather than overflowing.
- FIFO keys: binder type against `carries`, key typed over the binder only,
  model-state reads refused, and `=` admissibility tested by typing
  `left = right` over the key's type.
- `send`/`receive`: a channel name that is no channel refuses `missing-name`
  at the name. A binder type that differs from `carries` refuses
  `type-mismatch`. An object or unresolved `carries` is not compared, so
  AC-26's single refusal holds.
- Compensation templates lower through the same `protocol_attempt` path as
  attempts, so a template naming an attempt's operation shares its anchor
  and frame node. The frame record is inserted once and a differing record
  at the same key is an internal fault, never a silent overwrite.
  `CheckedCompensation` is only built when `frame_population` resolved.
- Commit dominance, checked against the spec text: `dominates` is "the
  effect's enter port is reachable from the root, and unreachable when
  every port of the commit is removed", which is exactly "lies on every path
  from the protocol's start to the effect". A commit first on the thread
  (directly or nested in a sequence) is refused with every retained member,
  and a commit after the effect, in a sibling branch or in one case of a
  choice is admitted. All five cases are tested. The spec-level gaps are
  FND-002.
- The content stage now refuses only `ReplicatedRole`, `Relationship`,
  `Requirement`, `Related`, `JoinPredicate` and `TerminalWhen`. Every body,
  guard, visible entry, loop proof, FIFO key, channel use and node role of
  the other kinds is read by a checker, except the `await` members in
  FND-001.
- `record_binder` (the first record binder after the node's name) is safe:
  the grammar requires `as (x: T)` on every event node, `commit` and
  `finish`.
- `#[trace]` bindings: every tag in the diff was read against its test (see
  the gap analysis, SR-12154, for the list). None over-claims: each tagged
  test can fail on the behaviour its AC states. The partial assertions are
  recorded as low findings there.

## Verdict

FIX. One blocking defect (FND-001): the PR drops the `not-yet-implemented`
guard for `await`, but no checker reads the `await`'s `within` interval,
its `clock` or the kind of its matched event. The checked clause does not
record them either. A protocol whose `await` QSpec FR-466 refuses now
checks, which is the SR-753 FND-002 / SR-770 FND-001 class. FND-002 is a
spec conflict: the code is correct to the letter of FR-381, and the fix
belongs in the spec. Defer it to a spec ticket. FND-003 to FND-006 are
recorded, not blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Before this PR, `content` refused every `await` as `not-yet-implemented`. Now an `await` checks, but nothing reads three of its members. (a) `within`: an interval with `lower > upper`, or an open upper bound, is admitted. QSpec FR-466 "Per-operator checks" step 6 refuses `lower > upper` with `ill_typed`/`type-mismatch` at the interval, and the compensation `within` is checked this way at `protocol_obligation.rs:850`. (b) The matched event: the grammar's `match` takes any `EventNode`, so `match send ...` and `match attempt ...` are admitted, although FR-466 step 2 refuses a `send` or `attempt` there with `unsupported_construct`/`declaration-form`. (c) `clock`: not resolved, and not recorded. `CheckedControl` holds only node, kind, name, path and children, so the checked clause carries neither `clock` nor `within`, which FR-466's `await` layout requires. QSpec FR-464 also requires a selected `temporal_profile` for an `await`, and no check reads one. The `coverage.rs` doc ("Every node kind and expression slot of the run tree is checked") therefore over-states. Fix: check the interval as the compensation's is checked, refuse a `send`/`attempt` match, and record `clock` and `within` on the checked clause. Or keep refusing `await` as an unchecked construct until a slice checks it. | qsl-semantics/src/check/protocol_clause/coverage.rs:35; qsl-semantics/src/check/protocol_clause/graph.rs:211; qsl-semantics/src/check/protocol_clause/types.rs:161; qsl-forms/src/protocol_control.rs:201 |
| FND-002 | medium | Commit dominance is path dominance over a relation whose edges are happens-before constraints. Some of those constraints hold together (a sequence edge plus the attempt-to-effect edge; the join edges of `join all`), but path dominance reads them as alternatives. Two cases follow where every execution takes the commit before the effect, yet the commit is admitted. (a) A commit sequenced between the attempt and its effect: the attempt.out to effect.in edge bypasses it. The PR body flags this one. (b) A commit in one branch of a `join all` parallel, with the effect after the parallel: the other branch's join edge bypasses it. The PR body does not flag this one. Both are correct to FR-381's literal "lies on every path", so this is a spec defect. Defer it to a spec ticket that defines dominance over the AND-joins (for example, the commit is a causal ancestor of the effect on every choice resolution). | qsl-semantics/src/check/protocol_clause/graph.rs:603-623 |
| FND-003 | medium | The partition proof is under-metered. The charge is 2^atoms × cases, but each valuation runs `evaluate` on every guard, and `evaluate` costs much more than one unit. At every node it does a linear `same_tree` search over all atoms, and each `same_tree` clones and compares a whole subtree. Every `value_of` lookup is a linear scan of the values vector. Per valuation the cost is therefore about O(guard nodes × atoms × subtree size + guard nodes²). A choice with large guards and an atom count under the budget can do far more work than it was charged. Fix: precompute each guard node's atom index once, outside the valuation loop. Then evaluate by index (a `Vec<bool>` keyed by `ExprId`), or charge by guard size × valuations. | qsl-semantics/src/check/protocol_slots.rs:458-510; qsl-semantics/src/check/protocol_slots.rs:886-951; qsl-forms/src/syntax.rs:1121 |
| FND-004 | low | Two replay messages lost a `\` line continuation, so they contain a run of about 22 spaces inside the string: "which is no channel of the                      protocol" and "not exhaustive and exclusive                      ({})". | qsl-replay/src/spine.rs:779; qsl-replay/src/spine.rs:820 |
| FND-005 | low | FR-218 "Channels" and AC-20 say the key-equality refusal retains `operator` (`=`) and the type. `ProtocolAnchorCause::KeyEquality` has `channel`, `key_type` and `span` only, so a consumer cannot read the operator as a field (FR-218 "Outputs": members are read as fields, never from a message). | qsl-semantics/src/check/refusal.rs:275 |
| FND-006 | low | Doc comments in the graph tests are stale. They say bodies that read names, and guards that read visible facts, are "not checked yet". This PR checks both, and AC-17 and AC-19 are now claimed by other tests in the same file. | qsl-semantics/src/check/protocol_clause/tests/graph.rs:81; qsl-semantics/src/check/protocol_clause/tests/graph.rs:454 |

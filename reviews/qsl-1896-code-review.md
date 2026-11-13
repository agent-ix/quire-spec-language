---
id: SR-2026111321481801
title: "code review of PR2701 realizability over the qsl-eval arena"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/src/protocol_analysis/{mod,realize}.rs, qsl-replay/src/protocol_analysis.rs, qsl-replay/src/protocol_analysis/strategy.rs, qsl-analyze/tests/it/{main,protocol_projection,protocol_realizability}.rs; PR2701 76d8a5a1ce34b00a49bcdce11ffe1decf5e92d2f"
review_set: subset
---

## Summary

Ticket: QSL-1896 (ADR-036 slice 4). Code review with the rust-review lane folded in, scoped to `git diff origin/main...HEAD`. The engine steps nothing itself: it explores the one `qsl-eval` `ProtocolSystem<Sc>` as a tree of plays (PA-22), evaluates by conjunction, groups histories with one `observation` function, commits one action per class and gates `proved` on the replay walk (PA-19). Collections are `BTreeMap`/`BTreeSet`, so results are deterministic (PA-15). `cargo fmt --check` is clean. No `/// Trace:` lines. Cargo clippy and tests were not run by the reviewer: the dispatching brief limits builds on the shared lock, and the lead runs the final gate.

The design analysis holds. `profile.rs` refuses `attempt`, `event`, `effect`, `commit` and `check`. FR-218's eligibility table (`qsl-semantics/src/check/protocol_slots.rs:591-629`) leaves a decision only its owner's `receive` records, protocol captures and constants: a `send` binder is `Eligibility::Never` and a read of `over` is refused. `qsl-eval` settles a choice inside `settle.rs:124-148`, entering the one case whose guard holds and faulting on overlapping or missing cases, so no `ProtocolTransition` is an owner's branch selection. Deleting the owner-move code and the `deciding_choices` accessor removes dead code. The same reading is the only one under which ADR-036's worked example refutes (FR-571-AC-2): with a free `Seller` choice, PA-18 (i) would call the cancel history avoidable and return `inconclusive`.

Two soundness findings block: a false `refuted` when protocol captures differ across initial states (FND-001), and a forced pair that can rest on a cut action (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `observation` records only a role's own sends and receives. Protocol captures are fixed per initial state, every role holds them (FR-218), and a guard may read them. A subject with two initial states that differ in a capture a guard reads gives two histories with equal observations, including for the owner, but different next actions. `forced_pair` then claims `IndistinguishableHistories` against a role that can tell the histories apart: a false `refuted`. Fix: start every role's observation with the instance's capture values, or answer `unsupported` when initial states differ in a capture a guard reads. | qsl-analyze/src/protocol_analysis/realize.rs:362-376 |
| FND-002 | medium | Forced refutation runs before the `root_value == Cut` check. A point's winning set holds only actions whose every child is `Win`. A role with two enabled actions (two `parallel` threads) whose second action leads to a `Cut` child gets a winning set without that action. `forced_pair` can then call two sets disjoint that would share the cut action, a refutation resting on a cut, which PA-18 says is `inconclusive`. Fix: skip a pair when either node has a `Cut` move for that role, or treat a `Cut` action as possibly winning in the disjointness test. | qsl-analyze/src/protocol_analysis/realize.rs:678-697, 777-786 |
| FND-003 | medium | `finding_value` maps every finding other than the three undetermined kinds to `Lose` with a wildcard `_`. `WeakAccessDeferred` and `WeakAccessShared` are FR-220 classification records, not obligation violations, so a lost play blamed on the clause would be a false `refuted`. They do not occur under `Sc` in the current profile, but the wildcard also maps any future `ModelFinding` variant to `Lose` without a decision. The walk has the same issue: `!expansion.findings.is_empty()` is treated as lost. Match exhaustively (rust-style: no catch-all over a domain enum). | qsl-analyze/src/protocol_analysis/realize.rs:299-312, 886-888 |
| FND-004 | medium | Premise groups (c) and (d) are checked only for `None`. Any `Some` set, such as `Some({"x"})`, is carried in the key and then ignored. No vocabulary for these strings is specified, so a declared fact or admitted outcome changes the key but never the analysis: an undefined premise is accepted silently. A non-empty set should be refused, or answered `unsupported`, until FR-571 defines the strings. Declared-empty still decides. | qsl-analyze/src/protocol_analysis/realize.rs:604-615 |
| FND-005 | low | String comparisons gate branches. `request.strategy_profile.as_deref() != Some(PROFILE)` gates admission on a string, repeating `project.rs:105`. FR-064 wants a closed enum read by a typed `from_wire` marked `#[ix_trace_rs::string_edge]`. `Names::node` and `Names::channel` join `qsl-eval`'s string paths (`rsplit('/')`, `rsplit("::")`) to package nodes. That is a genuine intake edge, since `qsl-eval` exposes only strings, but it carries no `#[string_edge]` marker. | qsl-analyze/src/protocol_analysis/realize.rs:597, 255-273 |
| FND-006 | low | `Observed::payload` is documented as "FR-181 typed canonical JSON bytes". `classify` re-serializes the `serde_json::Value` returned by `binder_record_json` with `serde_json::to_vec`, which is not the RFC 8785 encoder (number and escape forms can differ). A serialization failure is also reported as `UnnamedNode`, the wrong refusal. Take the canonical bytes from `qsl-eval` or encode with `quire_canonical`, and map the error to its own variant. | qsl-analyze/src/protocol_analysis/realize.rs:339-342; qsl-replay/src/protocol_analysis/strategy.rs:39-48 |
| FND-007 | low | A `Cut` root always settles `InconclusiveCause::Horizon`. `Cut` also comes from an undetermined finding (`finding_value`) and from a terminal classed `Undefined` or `Undecided` (`_ => Value::Cut`), so those results name the wrong cause. | qsl-analyze/src/protocol_analysis/realize.rs:303-307, 427-434, 706-711 |
| FND-008 | low | `explore` clones the whole history into every pending node and recomputes every role's observation from scratch at each node. That calls `classify`, `binder_record_json` and serde once per step per role per node, which is quadratic in depth. `walk` does the same at every state. Extend the parent's observations by the one new step. | qsl-analyze/src/protocol_analysis/realize.rs:407-409, 444-446, 890-899 |

## Scope and evidence

Read in full: `realize.rs` (949 lines), `strategy.rs`, the `mod.rs` and `protocol_analysis.rs` hunks, and `protocol_realizability.rs`. Callers and callees were read to establish behaviour: `qsl-eval` `settle.rs` (choice stepping), `steps.rs` (`successors_and_terminal`, findings), `model.rs` (`ModelFinding`), `terminal.rs` (`TerminalClass`), `protocol.rs` (`Unsupported`), and `qsl-semantics` `protocol_slots.rs` (decision eligibility).

- qsl-analyze/src/protocol_analysis/realize.rs (qsl-analyze/src/protocol_analysis/realize.rs): FR-571 engine: admission, arena exploration, decision points, refutations, replay walk. (examined)
- qsl-replay/src/protocol_analysis/strategy.rs (qsl-replay/src/protocol_analysis/strategy.rs): realizability artifacts StrategyAction, Observed, RoleStrategy, Closure, RealizabilityWitness, BranchDefeat, RealizabilityCounterexample. (examined)
- qsl-replay/src/protocol_analysis.rs (qsl-replay/src/protocol_analysis.rs): InconclusiveCause gains Unforced, StrategyReplayFailed, CutCount. (examined)
- qsl-analyze/src/protocol_analysis/mod.rs (qsl-analyze/src/protocol_analysis/mod.rs): module docs and re-exports. (examined)
- qsl-analyze/tests/it/protocol_realizability.rs (qsl-analyze/tests/it/protocol_realizability.rs): seven tests for TC-1411, TC-1412 and TC-1415. (examined)
- qsl-analyze/tests/it/protocol_projection.rs (qsl-analyze/tests/it/protocol_projection.rs): fixture helpers made pub(crate). (examined)

## Verdict

Not mergeable as is. FND-001 and FND-002 can return `refuted` where PA-18 requires `inconclusive`, or where the role can tell the histories apart. Fix those, plus FND-003 and FND-004; the low findings are cleanups. The `proved` direction looks sound: a `Win` root makes every play win, and the walk re-checks the table. Error types use `thiserror`, there is no `unwrap` on input-reachable paths, and the work is bounded by `max_states`/`max_transitions` on both explore and walk.

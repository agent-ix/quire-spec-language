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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | high | Regression from the FND-001 fix: a false `proved`. `observations` reads the captures from the current state through `ProtocolSystem::capture_bytes`, which encodes only live instances. The `Finish` step removes the instance (`qsl-eval/src/simulation/protocol/steps.rs:1864`), so at every terminal state the Captures item becomes the bytes of `[]`, unlike the live captures at every earlier node. Every role therefore "observes" that the protocol ended. Take ADR-036's hidden-choice example (`worked(PICK, "")`) and declare any capture: Warehouse's class at the cancel terminal is `[Captures([])]` (End), and at the ship receive it is `[Captures(live)]` (Receive). The classes differ, no forced pair exists, the walk passes and the result is `proved`, where FR-571-AC-2 requires `IndistinguishableHistories`. Fix: bind the Captures item once per play, from the instance's captures when it starts (the initial state under `on origin`), and carry it through `steps`; never recompute it from a later state. Add the hidden choice plus a capture as a test that must stay refuted. | qsl-analyze/src/protocol_analysis/realize.rs:396-414; qsl-eval/src/simulation/protocol.rs:1298-1337 |
| FND-010 | low | `finding_value` now ignores `WeakAccessShape`. That finding comes with an empty successor list (`steps.rs:181-188`), so the terminal class then decides the node, and a `Deadlocked` class would make it a loss blamed on `finish`. This is unreachable under `Sc`, but `Value::Cut(Open::Undecided)` would be the sound value for a shape that stops expansion. | qsl-analyze/src/protocol_analysis/realize.rs:334-336 |
| FND-011 | low | The payload is now RFC 8785, but it still round-trips the canonical bytes through `serde_json::Value` (`binder_record_json` parses them back). Without `arbitrary_precision`, an integer outside the i64/u64 range becomes f64 and re-encodes differently from the FR-181 bytes. Read the bytes from `qsl-eval` directly, as `capture_bytes` does. | qsl-analyze/src/protocol_analysis/realize.rs:367-373 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | ff413be91/44e8e3d35: `observations` puts `Observed::Captures` from `ProtocolSystem::capture_bytes` first in every role's class, and `a_capture_tells_initial_states_apart_for_every_role` decides a capture-read choice over two initial states as `proved`. The fix regresses at terminal states (FND-009). |
| FND-002 | fixed | `Point` gains `not_lost` (winning plus cut-open actions), and `forced_pair` requires non-empty winning sets with disjoint `not_lost` sets. The unit test `a_disjointness_resting_on_a_cut_action_is_not_forced` covers the parallel cut case. Sound both ways: `not_lost` contains `winning`, so a forced pair still has disjoint winning sets. |
| FND-003 | fixed | The match is now exhaustive: `InvariantViolated` gives Lose, the three undetermined findings give `Cut(Undecided)`, and the three weak-access records give nothing. `walk` fails on any `finding_value`. See FND-010 for the residue. |
| FND-004 | fixed | `admit` refuses a non-empty role_knowledge or environment set with `RealizabilityRefusal::PremiseFact(member)` (`invalid_runtime_input`/`invalid-value`), after the key and profile checks and before the None check. `a_premise_that_holds_a_fact_is_refused` tests it. |
| FND-005 | fixed | `Profile::from_wire` (`#[ix_trace_rs::string_edge]`) replaces `PROFILE` in both realize and project, and `Names::node`/`Names::channel` carry `#[ix_trace_rs::string_edge]`. |
| FND-006 | fixed | The payload is encoded with `quire_canonical::to_vec`, and a failure is the new `RealizabilityRefusal::Unencodable(path)`. See FND-011 for the residue. |
| FND-007 | fixed | `Value::Cut(Open)` carries Horizon or Undecided; `InconclusiveCause::Undecided` is added; terminal Undefined/Undecided and undetermined findings give Undecided, and NotTerminal-without-successors gives Horizon. The root's first open cause in move order settles. |
| FND-008 | fixed | `explore` and `walk` carry each role's own steps and extend them by the one step taken (`Game::extended`); `observations` only prepends the captures. |
| FND-009 | fixed | 12603ca42: `Seen` carries the captures, `Game::bind` inserts each instance's bytes with `or_insert` when the play first reaches it live and never re-reads them, and both `explore` and `walk` bind before classifying. The terminal state after `Finish` keeps the bound item, so classes no longer reveal termination. `a_capture_the_guard_does_not_read_leaves_the_hidden_choice_refuted` would fail under the regression. |
| FND-010 | fixed | `WeakAccessShape` now gives `Value::Cut(Open::Undecided)`; deferred and shared records give nothing. |
| FND-011 | fixed | New `ProtocolSystem::binder_record_bytes` returns the FR-181 canonical bytes directly; `classify` uses it, `binder_record_json` is derived from it, and there is no serde_json round trip. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | medium | `activation on each` reaches the arena: `profile.rs` refuses it only with compensation (via `qsl-eval`), and `qsl-eval` starts such a protocol with no instance and adds instances by `activate` steps (`protocol.rs:1143-1168`). Three things then go wrong. The engine keys a role by its role node alone (`Names::roles`), so one role's steps from two instances merge into one observation sequence. `bind` gives every role the captures of every instance, including instances it is not in, which FR-218 does not grant. And a new Captures item enters every role's class at each activation, so roles observe other instances starting. Merged instances can produce both a false `proved` and a false `refuted`. Answer `unsupported` for any activation other than `on origin` (a one-line check in `unsupported_construct` and a test), or key roles and captures by instance. This was present before the capture work; pass 0 missed it. | qsl-analyze/src/protocol_analysis/profile.rs:61-100; qsl-analyze/src/protocol_analysis/realize.rs:254-264, 412-426 |

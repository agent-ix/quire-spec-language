---
id: SR-12183
title: "Code review of quire-spec-language PR #1025: QSL-536 EN-1 state-graph engine (FR-166, FR-167, FR-168)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c4534444398992fa0af83915f91820ba21818d30; PR #1025 diff against origin/main (21 files, +4059/-18): qsl-analyze/src/model_check/{check,mod,state_graph}.rs; qsl-analyze/src/model_check/state_graph/{decide,explore,sample}.rs; qsl-analyze/tests/it/{graph_world,main,state_graph}.rs; qsl-eval/src/simulation/{mod,model,sample,trace}.rs; qsl-eval/src/simulation/model/state_predicate.rs; qsl-eval/src/value/{mod,expression/mod}.rs; qsl-replay/src/{lib,model_check}.rs; qsl-replay/src/model_check/{limits,state_graph}.rs; qsl-semantics/src/check/state_graph_claim.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1025

## Summary

Ticket: QSL-536. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` at the reviewed sha. Read-only: nothing
was built or run, at the dispatcher's instruction.

Checked and clean:

- Phase 0 (`sample.rs`): walks run only for `possible` instances. Walk `r`
  from initial state `i` uses trace index `i × witness_samples + r`. The
  greatest index, `(count − 1) × samples + samples − 1`, is checked with
  `checked_mul`/`checked_add` before any walk. On overflow the run returns
  `Stopped(ResourceExhausted, WitnessSamples)` naming `u64::MAX`, with no
  node explored. `samples − 1` is only reached when `samples != 0`. Each
  later `base + walk_index` stays at or below that checked bound. The step
  ceiling is `min(max_walk_steps, max_depth)`. Each instance keeps the
  witness of the lowest `r` and draws no later walk. An undefined or refused
  reading ends that instance's reading and decides nothing. `walks_stopped`
  counts only walks that `max_walk_steps`, not the horizon, ended. One
  walk serves every pending instance. The draws depend only on
  `(seed, trace)`, so each instance's prefix is the walk it would have drawn
  alone (but see FND-003 for the provenance it records).
- `sample_request_until`: the end condition is read at the top of the loop,
  before `expand` and before the step-limit check. Draws happen only after
  a successor set is computed, so `sample_request` (`|_| false`) keeps
  identical draws and stop reasons. `replay` skips expanding the last state
  of a `Reached` trace, and `check.finish()` still requires every recorded
  finding to match. The sampler recorded no findings for that state.
- Cancel: polled before every walk, at every state a walk visits, and
  before every node is labelled and expanded. A cancel during phase 0
  returns before exploration with an empty graph and the deadlock item
  `Stopped`.
- Exploration (`explore.rs`): initial states are sorted by key bytes and
  then by index. Equal keys coalesce, and `initial` maps every subject index
  to its node. Successors are sorted by transition bytes, then post-state
  key bytes. Every edge is retained. `MaxDepth` is set only for a horizon
  node that has a successor, and it never overwrites `UndecidedSuccessor`.
  A limit or cancel stop marks the partly expanded node and every queued
  node `NotExpanded`. `max_states`/`max_transitions` are checked before the
  insert or push, so the run never exceeds them. The deadlock item takes
  its first event in discovery order (undefined contract, then refused
  contract, then the FR-124 class), as FR-126 does.
- Decision (`decide.rs`): `E⁺` is a backward BFS from target and open nodes
  over every retained edge. `d_P` is a backward BFS from target nodes
  through closed predecessors only. The explored witness takes the first
  canonical edge that goes one nearer. That gives FR-168-AC-9's `toB, fromB`
  and ignores the open `pos = 3`. `possible`/`always possible` traps are
  closed nodes outside `E⁺`. Those cannot reach an open node, so they are
  sound on partial runs. On a tie, undefined wins over a trap or a path pair
  (`at <= node`). An earlier trap or pair wins, and on `possible` an
  undefined evaluation replaces `Witnessed`/`WitnessedUnchecked`.
  `Witnessed` and `Holds` need `end == Completed` and no open node.
  Kosaraju gives the components in topological order, and counts run in
  reverse, saturating at 2. A path pair needs a known node
  (`reaches_open == FAR`). The second path is the least `(length, canonical
  ranks)` over every first-deviation, so it always exists for a known
  count-2 node.
- `ModelCheckLimits`: `witness_samples` 64 and `max_walk_steps` 4,096 by
  default, `DEFAULT_WITNESS_SEED` 0, and `WitnessSamples`/`MaxWalkSteps` with
  keys and settings. The one other struct literal in the workspace
  (`tests/it/model_check.rs:690`, `closure_certificate.rs:772`) uses
  `..Default::default()`, so nothing else breaks.
- `qsl-semantics`: `CheckedPredicateBody` now carries `typed.slots`, as
  terminal and state-clause bodies already do. `evaluate_state_predicate`
  passes it to `Machine::run` with the `over` reference as the only
  argument. No identity or digest is derived from the struct.
- Panic surface: no `unwrap`/`expect`/`panic!`/`unsafe` in production code.
  The indexing is over vectors built to node or edge length in the same
  function.
- Determinism: only `BTreeMap`/`BTreeSet` and sorted vectors. Phase 0 is a
  function of subject, items, limits, horizon and seed.

Every `#[trace]` was read against its AC. The list is in the gap
analysis, SR-12184. No tag over-claims: each tagged test can fail on its
AC's behaviour. The test oracles are independent of the engine.
`reachable` and `expected_steps` drive `ModelSystem` directly, and
`positions` re-executes every expected path. The remaining weak assertions
are FND-004, FND-007 and FND-008.

## Verdict

FIX. One blocking defect (FND-001): `unique path` returns `Trapped`, which
is refuting evidence, at a closed node that only reaches open nodes. Under a
horizon, the sequenced §7.3 job, whose claim holds, is refuted. FND-002 to
FND-009 are medium and low. They are recorded and do not block under the
lane's major-only rule.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `unique_path` picks its trap as `!is_open(node) && from(node) && counts[node] == 0`. It does not require the node to be known. A closed node in `H` whose `H` successors are all open nodes with no retained out-edges (`MaxDepth`, `NotExpanded`) sums to 0, so it is returned as `Trapped`. Open nodes can still change what it reaches. Traced by reading: the sequenced §7.3 job under horizon `max_depth` 1 expands the root to `(a=1, b=0)`, which is open `MaxDepth`. The root is closed, `from` holds there and it counts 0, so the result is `Trapped` with an empty stem, although the claim holds (FR-168-AC-3). This violates FR-168's Description and GM-6 ("evidence only where open nodes cannot change it: a trap whose forward closure is closed"), and QSpec FR-391's trap definition, under which no open node is reachable from a trap. FR-168's `unique path` trap bullet is literally satisfied, and its wording should also say "known". Fix: also require `!in_h[node]`, or `reaches_open[node] == FAR`, and add a `unique path` test under a horizon. | qsl-analyze/src/model_check/state_graph/decide.rs:306-308 |
| FND-002 | medium | When the distinct initial states exceed `max_states`, `run` returns before inserting any node, so `graph.initial` is empty. `possible` then sees `witnesses.len() == graph.initial.len() == 0` and returns `WitnessedUnchecked { paths: [] }`. That is a claim of a witness for every initial state with no paths, which FR-169 would settle `well-definedness-unchecked`. The outcome should be `NoDecision` with the stop. Example: two snapshots and `max_states` 1. Fix: record the initial-state nodes up to the limit and mark them `NotExpanded`, or return `NoDecision` when the subject has initial states and the graph has none. | qsl-analyze/src/model_check/state_graph/explore.rs:149-156; qsl-analyze/src/model_check/state_graph/decide.rs:239-248 |
| FND-003 | medium | A sampled witness stores the shared walk's `SampleProvenance`, including its `stopped`. An instance whose target was found before the walk ended keeps a truncated path (`take(position)`), but `stopped` reads `StepLimit`, `NoSuccessors` or `Stopped(..)` from the other instances' longer walk. The walk that instance would have drawn alone records `Reached`, and FR-166 names each walk by its provenance. FR-170 replay of the witness would then expand its last state as a non-`Reached` trace. Fix: clone the provenance with `stopped: StopReason::Reached` for every witness. | qsl-analyze/src/model_check/state_graph/sample.rs:157-172 |
| FND-004 | medium | TC-618 step 3 (horizon `max_depth` 1: every walk takes at most 1 step) asserts only `walks_stopped == 0`. `budget_is_ceiling` is false whenever horizon < budget, so this passes even if the horizon were not applied to the walk ceiling. Walk length is not observed anywhere. Fix: expose the walk lengths, or the step ceiling, to the test, or assert a sampled witness bound by the horizon. | qsl-analyze/tests/it/state_graph.rs:757-762 |
| FND-005 | medium | `sample_request_until`, `StopReason::Reached` and `replay`'s skip of the last state of a `Reached` trace are new public `qsl-eval` behaviour with no direct test. Nothing tests the prefix property (an `until` trace equals the `sample_request` trace up to the end state), and nothing replays a `Reached` trace. The phase 0 tests never replay. | qsl-eval/src/simulation/sample.rs:356-386; qsl-eval/src/simulation/trace.rs:168-185 |
| FND-006 | low | A `Refused` reading of any item's predicate at a node marks the node open, `UndecidedSuccessor`, for every item. So one item's refused predicate blocks another item's `Witnessed`/`Holds` and shifts its traps. FR-167 names `UndecidedSuccessor` only for contract conjunctions. The cross-item effect is undocumented outside the module doc. | qsl-analyze/src/model_check/state_graph/explore.rs:212-242 |
| FND-007 | low | The FR-167-AC-2 "undecided contract conjunction" test uses `2 div (2 - v)`, which is undefined. It only takes the `undefined_contract` branch. The refused (undecided, not undefined) `ContractUndetermined` branch and the refused-predicate branch that also set `UndecidedSuccessor` are untested. | qsl-analyze/tests/it/state_graph.rs:365-391; qsl-analyze/src/model_check/state_graph/explore.rs:230-242 |
| FND-008 | low | FR-168-AC-9's test does not assert the AC's premise that `toA` precedes `toB` in canonical transition order, or that `pos = 1` and `pos = 2` both have `d` 1. If `toB` sorted first, descending `E⁺` toward the open node would also give `toB, fromB`, and the test could not tell the two rules apart. | qsl-analyze/tests/it/state_graph.rs:984-1007 |
| FND-009 | low | `sample`'s doc still says "Every state on the trace is expanded, the last included". That is no longer true for a `Reached` trace, whose last state is never expanded. | qsl-eval/src/simulation/sample.rs:195-198 |

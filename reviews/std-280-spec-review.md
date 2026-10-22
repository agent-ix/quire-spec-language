---
id: SR-15910
title: "spec review of STD-280 undefined guard, terminal predicate or state constraint refutes state-graph items (QSL PR #1264)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@35eb2443c27e10e12535560129dd486b7e1c3d8f; spec/functional/FR-167-explore-a-state-graph-subject-and-track-open-nodes.md, spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md, spec/functional/FR-170-replay-state-graph-evidence.md, spec/test-cases/TC-612-undefined-predicate-refutes-a-state-graph-claim.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-167
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-168
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-170
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-612
    type: reviews
---
# SR-15910: spec review of STD-280 undefined guard, terminal predicate or state constraint refutes state-graph items (QSL PR #1264)

## Summary

Ticket: STD-280 (unblocks QSL-901). The diff is `origin/main...35eb2443c`. `git ls-remote` confirms that head. Four files, spec only. It extends FR-168's "Undefined evaluation" section and adds FR-168-AC-11 to AC-13. It adds an exception to FR-167's undecided-expansion bullet, a replay bullet in FR-170 and FR-170-AC-7, and TC-612 steps 5 to 7. The head is four commits behind `origin/main` 423ac6aa0, but it merges cleanly. `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` on that merge exits 0, with no finding on a touched file.

These claims were checked by hand against QSL main (FR-120's precondition conjunction, FR-124, FR-126-AC-9, FR-167, FR-169, ADR-021 SC-1/SC-2/EI-4, ADR-022 §7.1, GM-6, GR-4) and QSpec FR-391 ("Open and closed nodes", "Undefined evaluation", AC-14):

- FR-168-AC-11. Universe `{a}`, with `upd` taking `v` to `(v + 1) mod 3`. The guard `2 div (2 - v) >= 1` is true at 0 (1) and at 1 (2), and has no value at 2. `ReachesTwo` and `CanReachTwo` both return `Undefined` at `va = 2`, with stem `(0) -upd(a)-> (1) -upd(a)-> (2)`. On main the same node is open, `UndecidedSuccessor`, so `ReachesTwo` gives `WitnessedUnchecked`. The AC can fail.
- FR-168-AC-12. `Counter` reaches 3, the only node with no enabled `inc`, and there `6 / (3 - 3)` has no value. The result is `Undefined` there although the target holds. This agrees with FR-126-AC-9's deadlock vector.
- FR-168-AC-13 agrees with ADR-021 SC-2, which already says an undefined constraint does not make a state a boundary state and refutes the item, and with GR-4, which admits a state constraint for SG-1.
- The `where` shape matches QSpec FR-391: digest and locus, plus the transition for a guard, and no predicate. FR-169's V-4 row is generic, so no settlement edit is needed. The FR-170 replay bullet is consistent with FR-170-AC-5.
- Lead default: "guard" means the effective precondition, with only an `Undefined` stopping clause refuting (FR-120 short-circuit). A `Refused` clause and an undecided postcondition stay `UndecidedSuccessor`. This is sound and matches QSpec, which lists only guards, `terminal when` predicates and state constraints. Adopting FR-391-AC-14 (the ticket's default) is right. The exception is the FR-167 part (FND-001).
- Ids FR-168-AC-11 to AC-13 and FR-170-AC-7 appear on no other open PR head. No other open PR touches FR-167, FR-168, FR-170 or TC-612. #1246 edits FR-169 (package identity, a new AC-12 and the V-6 row) with no textual or semantic overlap. #1242 touches only FR-410. There is no status or changelog wording.

The PR body's "ungrounded" question about FR-167-AC-2's fixture is grounded here (FND-002). The `code-review` method was skipped, because the diff contains no code. The criterion-strength analysis (Jev) was not run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: FR-167's new exception says an undefined guard "opens no node", so the node counts as closed. A closed node's edges are "exactly its successors", but here the successor through the undefined transition is unknown. QSpec FR-391 ("Open and closed nodes") and ADR-022 GM-6 list "a node whose expansion evaluated a contract conjunction undecided" as open. An undefined stopping clause makes the precondition undecided (FR-120). Trace on FR-168-AC-11's subject, with `ReachesThree` (no node has `v = 3`): if `(2)` is closed with no edge, `E⁺` is empty, and `(0)` is a trap at discovery index 0, before the undefined node at index 2. So the engine returns `Trapped`, `closed-scope`, and not `Undefined`. GX-3's closure replay of that trap then meets the undefined guard. Keeping the node open, `UndecidedSuccessor`, puts `(0)` in `E⁺` and gives `Undefined`, as QSpec does. A closed node with no edge also reads as a deadlock to the deadlock-freedom item. <br>Fix: keep the node open (`UndecidedSuccessor`) and state that the undefined guard is refuting evidence. FR-168 "Partial runs" already returns refuting evidence "whatever other nodes are open". Drop "and opens no node", drop FR-168-AC-11's "neither leaves the node open `UndecidedSuccessor`", and add `ReachesThree` over the guarded subject to AC-11, expecting `Undefined` at `va = 2`, not `Trapped`. | spec/functional/FR-167-explore-a-state-graph-subject-and-track-open-nodes.md:122-126, spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:204 |
| FND-002 | medium | major: The only implemented fixture of FR-167-AC-2's "undecided contract conjunction" is this undefined guard: `qsl-analyze/tests/it/state_graph.rs:380-410` expects `(2, UndecidedSuccessor)`. FR-169-AC-4's "undecided node" vector uses the same guard with `ReachesThree` and expects `inconclusive`, `UndecidedSuccessor` (`state_graph_settle.rs:831` and `900-930`). Under this PR both expected values become `refuted`, `UndefinedEvaluation`, and nothing then tests `UndecidedSuccessor`, which the lead default keeps for refused clauses and undecided postconditions. <br>Fix: name an undecided fixture that is not an undefined guard in TC-592 (FR-167-AC-2) and TC-594 (FR-169-AC-4), for example a postcondition clause `2 div (2 - self.versionNumber) >= 0` on `attemptUpdate`, or a refused precondition clause. Alternatively, record it on QSL-901 as an explicit fixture change for the code PR. | spec/functional/FR-167-explore-a-state-graph-subject-and-track-open-nodes.md:149, spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:223 |
| FND-003 | low | minor: FR-168's Outputs still define `Undefined` as ending "where a predicate of the claim evaluates undefined", and its Description and Outputs speak of an undefined evaluation "of the target" only. Both now contradict the new Behavior bullets. <br>Fix: "where a predicate of the claim or one of the subject's expressions (guard, `terminal when` predicate, state constraint) evaluates undefined". | spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:31, spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:57, spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:67 |
| FND-004 | low | minor: FR-168-AC-13 writes the state constraint as a bare expression over an unbound `c`. ADR-021 SC-1 says a constraint is declared in the unit and named by the request, in the form `constraint Low using v on M::Counter at current { self.v <= 1 }`. <br>Fix: "the state constraint `constraint Half using v on Config::ConfigVersion at current { 2 / (2 - self.versionNumber) >= 0 }`, selected by the request". | spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:206 |
| FND-005 | low | minor: FR-170's new replay bullet has three arms (guard, `terminal when`, state constraint), but FR-170-AC-7 replays only the guard and terminal evidence. The constraint arm, which needs the request's selected constraint at replay, has no AC. <br>Fix: add FR-168-AC-13's evidence to FR-170-AC-7 (and to TC-612 step 7), expecting `reproduced-with-evaluated-witness`. | spec/functional/FR-170-replay-state-graph-evidence.md:171-178, spec/functional/FR-170-replay-state-graph-evidence.md:199 |

## Verdict

Not mergeable as is. FND-001 is major: "opens no node" departs from QSpec FR-391's open-node list and gives a spurious `Trapped` verdict, traced on the PR's own fixture. The fix is small: keep the node open and refute. FND-002 is major, but it can be fixed here or carried to QSL-901 as an explicit fixture change, if the lead records it there. FND-003 to FND-005 are minor. The hand-checked vectors of AC-11 to AC-13, the `where` shape, the "guard = effective precondition" default, FR-169 needing no settlement change, id uniqueness and the absence of overlap with #1246 are all verified. No owner decision is needed. The PR body's "origin/main is an ancestor of the head" is stale (four commits behind, merges cleanly).

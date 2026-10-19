---
id: SR-13322
title: "base review of STD-214 FR-169-AC-10 loci and the BadState rule (PR #1048)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md, spec/test-cases/TC-644-state-graph-certificates-that-do-not-hold-are-rejected.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-644
    type: reviews
---
# SR-13322: base review of STD-214 (PR #1048)

## Summary

Ticket: STD-214. This is the base checklist over the diff
`origin/main...origin/std-214-spec` (731affc4a; main is a2d68af2e). The diff
covers FR-169's Outputs sentence, the new `BadState` rule bullet, the
restated FR-169-AC-10, the new FR-169-AC-11, and the TC-644 scope,
procedure and expected results.

Every locus was recomputed against the real engine on origin/main. A scratch
test in a scratch clone ran `ModelSystem` (`qsl-eval`) over the
`graph_world` fixtures (`config_world`, `job_world`, `config_world_over`).
It enumerated every reachable state with its `quire.simulation.state-key/v1`
digest, its successors and its expansion findings.

- §7.1 certificate order, by ascending digest: (2,0), (1,1), (0,1), (1,2),
  (2,1), (0,2), (2,2), (1,0), (0,0). That is 9 states and 18 edges.
- Unsequenced job order: tff, ttt, fff, ftf, ttf (5 states, 5 edges).
- Sequenced job order: tff, ttt, fff, ttf (4 states, 3 edges).
- Guarded `{a}` subject: va=2 expands to no successor and records one
  undefined-contract finding. va=0 and va=1 are clean. Without the guard,
  va=2 goes to va=0.

Every locus the PR states holds under FR-169's rules:

- `SuccessorMissing`: (1,2) and (2,1) are the only predecessors of (2,2).
- `RankBroken`: (0,0) is the only broken state, because (1,0) at rank 2
  keeps (1,1) at rank 1.
- `RankMissing` is at (0,0).
- `PredicateUndefined`: the target is undefined at exactly the three va=2
  states.
- `CountBroken` is at fff only.
- `OrderBroken`: every node that is not the `to` node has a forward edge
  under the reversed order.
- AC-11: `BadState` at va=2. The unguarded ranks 2/1/0 are valid.

No AC was renumbered. FR-169-AC-11 is not on origin/main, and no open PR
adds it. The findings below concern rule precedence and wording.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-169 says the checker rejects "at the first failing state in certificate order", but it never says how the listed rules take precedence. One reading checks the rules in list order, each over every state. The other takes the first state that fails any rule. The new `BadState` rule can fire together with `SuccessorMissing`, at the same reached state or at different ones, and AC-10's expected rules depend on which reading applies. The FR-168-AC-5 certificate states no ranks in the AC (TC-644 step 2 gives them), and the unsequenced `InOneWay` certificate states no order. PR #1046 checks the rules in list order: InitialMissing, SuccessorMissing, BadState, PredicateUndefined, then RankMissing/RankBroken, or OrderBroken then CountBroken. Each rule runs over all reached states. State that order as the rule. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:146-171 |
| FND-002 | medium | The new `BadState` bullet contradicts itself. It fires "when expanding a `closure` state the checker reached", but it also says "a certificate that holds such a state is never accepted, whatever its closure and ranks say". Take a closure entry that is bad but that no initial state reaches. The first clause does not reject that certificate, and the second says it is never accepted. PR #1046 never expands unreached entries. The `SuccessorMissing` bullet also still says "a `closure` state", not a reached one. Say "reached" in both places. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:152-160 |
| FND-003 | low | The ticket's default fix asked for AC-10's loci to be recomputed from an engine run and named. AC-10 instead names three of them only as "the first, in certificate order, of …". The engine on main fixes them: `SuccessorMissing` at (1,2), `PredicateUndefined` at (2,0) (the old locus was right), and `OrderBroken` at tff. Naming the states makes the oracle concrete. If the relational form is kept on purpose, record why. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:226; spec/test-cases/TC-644-state-graph-certificates-that-do-not-hold-are-rejected.md:42-47 |
| FND-004 | low | AC-10's unsequenced `InOneWay` certificate "with every count 1" does not say that its order is a valid reverse topological order of `H`. With an arbitrary order, `OrderBroken` fires before `CountBroken` in PR #1046's rule order. Say the order is valid. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:226 |
| FND-005 | low | AC-11 attributes its guard to "(FR-167-AC-2)", but FR-167-AC-2 names no guard: it says only "a subject with an undecided contract conjunction at one reachable node". The guard text exists only in TC-592's test fixture (`qsl-analyze/tests/it/state_graph.rs:383`). AC-11 quotes the guard in full, so the AC can still be built. Drop the attribution or point it at TC-592. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:227; spec/test-cases/TC-644-state-graph-certificates-that-do-not-hold-are-rejected.md:22,32 |

## Verdict

Two medium findings (FND-001 and FND-002), so the PR is not mergeable as is.
Each fix is one sentence in FR-169's Check bullet.

Examined and found clean:

- the Outputs sentence that moves `BadState` to FR-338's own rules (FR-338
  line 53 lists it);
- every number and locus in AC-10 and AC-11 against the engine;
- the AC-11 to TC-644 link and the TC-644 scope, steps and expected results,
  which match the ACs;
- AC id continuity (AC-1 to AC-11, none renumbered);
- the absence of any id collision on origin/main or in the 27 other open PRs.

The `ModelState(DigestRecord)` state-key digest is a canonical identity of
proof content, where trust is the feature. It is pre-existing and not in
this diff.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Disposition round 1, head 9838c06bb. FR-169's Check bullet now says the checker evaluates the rules in the listed order, each over every reached state, and rejects with the first rule that fails, with `OrderBroken` before `CountBroken`. That matches PR #1046's order. |
| FND-002 | fixed | Disposition round 1, head 9838c06bb. `SuccessorMissing`, `BadState` and `PredicateUndefined` now say "reached `closure` state". The "never accepted, whatever its closure" clause is gone. A new bullet says an unreached closure entry is never expanded and never decides `BadState`. |
| FND-003 | fixed | Disposition round 1, head 9838c06bb. AC-10 and TC-644 now name the engine's loci: (1, 2), (2, 0) and tff. |
| FND-004 | fixed | Disposition round 1, head 9838c06bb. The all-count-1 `InOneWay` certificate now carries "a valid reverse topological order", in AC-10 and in TC-644 step 3's expected result. |
| FND-005 | fixed | Disposition round 1, head 9838c06bb. AC-11 now cites "the guard of TC-592's FR-167-AC-2 fixture", and TC-644's fixtures and step 5 name TC-592's guarded subject. |

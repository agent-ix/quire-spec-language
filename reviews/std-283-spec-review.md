---
id: SR-15900
title: "spec review of STD-283 liveness over an on-each protocol under max_live_instances (QSL PR #1262)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@45055d8ddbe077c4e87d4245d134bec713cc0ff7; spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md, spec/functional/FR-212-derive-scheduler-fairness-with-an-adversarial-opt-out.md, spec/decisions/ADR-027-protocol-transition-system-for-parallel.md, spec/test-cases/TC-654-activation-on-each-and-instance-budget.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-212
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-654
    type: reviews
---
# SR-15900: spec review of STD-283 liveness over an on-each protocol under max_live_instances (QSL PR #1262)

## Summary

Ticket: STD-283 (unblocks QSL-559 and QSL-968). The diff is `origin/main...45055d8dd`. `git ls-remote` confirms that head. Four files, spec only. It adds the FR-209 section "Liveness under the bound", FR-209-AC-5 and AC-6, one bullet in FR-212, one sentence at the end of ADR-027 AE-4, and TC-654 steps 5 and 6. The head is four commits behind `origin/main` 423ac6aa0, but it merges cleanly. `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` on that merge exits 0, with no finding on a touched file.

These claims were checked by hand against QSL main (ADR-018 SM-3/SM-4, ADR-027 PA-1 to PA-3, PB-1, PB-4, AE-4, ST-13, FR-212) and QSpec FR-430:

- The bounded graph's steps are steps of the unbounded protocol. FA-2 reads `activate` as enabled at an instance-limited state in both graphs. PA-2, FR-212 and QSpec FR-430 (line 59) put `activate` in no class. So a lasso's fairness is the same in both graphs, and a fair lasso found under the bound is a real V-4 counterexample. The V-6 rule for a run with no fair lasso and at least one instance-limited state is sound.
- FR-209-AC-5 (`always eventually holds(k.v = 1)`), bound 1. States: `(v 0, no instance)`, then `activate`, then `(0, 1 instance)` (instance-limited), then `attempt(set)` and `finish` into `v = 1`. Nothing sets `v` back to 0. No state is terminal (PB-4: `activate` is enabled whenever fewer than the bound are live), so there is no stutter, and the `v = 0` region has no cycle. So there is no lasso, and at least one instance-limited state was explored: V-6 `InstanceBoundReached` with bound 1. With bound 2 the shape is the same, and the instance-limited state is the one with two instances at `set`: V-6 with bound 2. The AC can fail: an engine that still refuses `InstanceLiveness`, or one that returns `Holds`, fails it.
- FR-209-AC-6 (`always eventually holds(k.v = 0)`), bound 1. The only cycle in the `v = 1` region is `activate`, `attempt(set)`, `finish`, and it passes through the instance-limited states with one live instance. This gives V-4 after replay. The AC fails for an engine that settles V-6 whenever a lasso touches an instance-limited state.
- Ids FR-209-AC-5 and FR-209-AC-6 appear on no other open PR head. No other open PR touches FR-209, FR-212, ADR-027 or TC-654 (45 open PRs checked; #1242 touches only FR-410, and #1246 touches FR-169). There is no status or changelog wording in the added text.

Lead defaults: (1) Not counting an untaken `activate` against fairness deviates from the ticket's default. It is the reading that ADR-027 PA-2, FR-212 and QSpec FR-430 already state. The ticket's default ("weak scheduler fairness for the trigger") contradicts those settled texts, so this is not a new owner decision. The owner should still be told, because the ticket default said the opposite, and choosing environment fairness instead would be a QSpec FR-430 change. (2) V-4 for a fair lasso through instance-limited states is sound. (3) The vectors are sound, but neither one tests (1): see FND-001.

The `code-review` method was skipped, because the diff contains no code. The criterion-strength analysis (Jev) was not run. Testability was judged by hand.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | major: Neither new AC can fail on the rule this PR decides, that an `activate` left enabled and never taken at instance-limited states does not make a lasso unfair. FR-209-AC-6's loop takes `activate` itself (`activate(Tick)`, `attempt(set)`, `finish`). A checker that wrongly counts `activate` as weakly or strongly fair still accepts that lasso as fair, because `activate` is taken on every pass of the loop. FR-209-AC-5 has no cycle in its `v = 0` region at all, so fairness plays no part in it. <br>Fix: add a vector whose loop stays at instance-limited states and never takes `activate`. For example, FR-209-AC-1's protocol with body `repeat { attempt set }` (no maximum, PS-6), `always eventually holds(k.v = 0)` and `max_live_instances` 1 settles V-4. Its lasso's loop is the `attempt(set)` self-loop at the instance-limited state, where `activate` is enabled at every loop state and never taken. A checker that counts `activate` as fair returns V-6 instead. Add it to TC-654. | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:145 |
| FND-002 | low | minor: "A run of the protocol is free not to start an instance" is not true under PB-1 and PB-4. A behaviour is a maximal path, and an `on each` state whose only enabled step is `activate` is not terminal, so the behaviour must take it. Read literally, the sentence admits a behaviour that stays at the initial state forever. That reading would turn FR-209-AC-5's expected V-6 into V-4. <br>Fix: "A fair run may leave an enabled `activate` untaken while other steps run." | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:124 |
| FND-003 | low | minor: The new FR-212 bullet repeats the Classes bullet two lines above it, which already puts `activate` in no class. The only new content is "at an instance-limited state (FR-209) as anywhere else". <br>Fix: fold that clause into the existing bullet, or keep the bullet and accept the repetition. | spec/functional/FR-212-derive-scheduler-fairness-with-an-adversarial-opt-out.md:88-90 |
| FND-004 | low | minor: FR-209-AC-5's tail, "and V-6 with `bound: max_live_instances 2` with 2", is hard to parse. <br>Fix: "...with `max_live_instances` 1; with 2 it settles V-6 with `bound: max_live_instances 2`." | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:144 |

## New findings (disposition pass 1)

Head ffc486da6 (`git ls-remote` confirms it). It merges cleanly with `origin/main` 6fb751a31. `make check-spec-validation` on that merge exits 0, with no finding on a touched file. FR-209-AC-7 appears on no other open PR head.

FR-209-AC-7 was hand-checked. The body `repeat { attempt set }` with no maximum keeps no counter (PS-6). At bound 1, `activate` gives `(v 0, 1 instance)`, then `attempt(set)` gives `(v 1, thread back at set)`, and every later `attempt(set)` returns to that same key: a self-loop at an instance-limited state. The root class is taken on every pass, so the derived scheduler constraint is met. `activate` is enabled at the loop state (FA-2) and never taken. So the lasso is fair and gives V-4. If `activate` were counted in a class, that loop would be unfair. The instance never finishes, so no other cycle exists in the `v = 1` region. That checker then finds no fair lasso, explores instance-limited states and settles V-6. The AC discriminates.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | minor: FR-209-AC-7 writes the loop as `repeat { attempt set }`, with no guard. A QSL `repeat` has a `while` guard (PS-6's body graph has a back edge "to the guard", and the surface examples read `repeat R ... max 2 while { ... }`). The vector's lasso needs the guard to hold at `v = 1`. <br>Fix: write `repeat R while {true} { attempt set }` (or "a `repeat` with no maximum whose guard is `true`"), with the same spelling in TC-654 step 7. | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:146 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Disposition pass 1, head ffc486da6. FR-209-AC-7 and TC-654 step 7 add the `repeat` vector. Its loop is the `attempt(set)` self-loop at the instance-limited state, where `activate` is enabled and never taken. It settles V-4, and a checker that counts `activate` as fair settles V-6, which was hand-checked. |
| FND-002 | fixed | The sentence now reads "A fair run may leave an enabled `activate` untaken while other steps run." This agrees with PB-1/PB-4. |
| FND-003 | fixed | The repeated FR-212 bullet is gone. The Classes bullet now ends "at an instance-limited state (FR-209) as anywhere else". |
| FND-004 | fixed | FR-209-AC-5 now reads "...with `max_live_instances` 1; with 2 it settles V-6 with `bound: max_live_instances 2`". |
| FND-005 | fixed | Disposition pass 2, head aa33eeee1. FR-209-AC-7 and TC-654 step 7 now write `repeat R by p visible() while { true } attempt Done on set`. This matches the QSpec shared grammar's `repetition` (proposals/quire-v1/shared-grammar.md:532-535): `'repeat', ident, 'by', ident, visibility, [ loop-proof ], 'while', block, control`. The no-`max` arm is valid, `visible()` is an empty `visibility`, and the grammar is unchanged on QSpec origin/main. `attempt Done on set` shortens the `attempt` event node the same way FR-212-AC-1 does. The lasso is unchanged. With no maximum, PS-6 keeps no counter, and FR-206-AC-5 says a `repeat` adds no step. After the first `attempt(set)`, each pass settles back through the `{ true }` guard to the same rest point at `v = 1`. That gives the self-loop at the instance-limited state, with the root class taken and `activate` enabled and never taken: V-4 at bound 1. A checker that counts `activate` as fair finds no fair lasso and settles V-6. |

## Verdict

Disposition pass 2: head aa33eeee1 merges cleanly with `origin/main` 59ba94503, and `make check-spec-validation` exits 0 on that merge, with no finding on a touched file. FND-001 to FND-005 are all fixed, and the delta has no new finding. Mergeable.

Disposition pass 1: FND-001 to FND-004 are fixed. FND-005 is a new low-severity spelling nit: the `repeat` guard is unstated. The PR is mergeable, either with FND-005 fixed in a one-line edit or with it deferred to the QSL-559 coder's test, if the lead accepts that. No regression was found in the delta. Lead default (1) still needs owner awareness, as before.

Review pass: not mergeable as is. FND-001 is major: the PR's central decision, the fairness reading of `activate`, has no AC that could fail. One added vector fixes it. FND-002 to FND-004 are minor wording fixes and cheap to do in the same round. The soundness of V-4 and V-6 under the bound, the hand-traced vectors at bounds 1 and 2, the lead defaults' consistency with PA-2, FR-212 and QSpec FR-430, id uniqueness and the absence of overlap with other open PRs are all verified. Owner awareness is needed for lead default (1), which deviates from the ticket text but follows the settled specs. The PR body's "origin/main is an ancestor of the head" is stale (four commits behind, merges cleanly).

---
id: SR-16210
title: "Spec review of STD-368 guarded TP-4 liveness (QSL PR #1310)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@a4e6ae2d22fc954f3ad3d5e947b2c0412d8ae162; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md, spec/functional/FR-338-check-an-en-1-closure-certificate.md, spec/functional/FR-339-check-an-en-1-component-certificate.md, spec/test-cases/TC-521-explicit-state-model-check.md, spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md, spec/test-cases/TC-526-an-en-1-component-certificate-is-checked.md, spec/test-cases/TC-1085-liveness-model-only-pass-and-unsupported-refusals.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-525
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-526
    type: reviews
---
# SR-16210: Spec review of STD-368 guarded TP-4 liveness (QSL PR #1310)

## Summary

Ticket: STD-368. The diff is `origin/main...a4e6ae2d2` (main c21bc57a5 is an ancestor of the head). It is spec only, so `code-review` and `gap-analysis` were skipped. The reviewer checked it against FR-126, FR-128, FR-130, FR-338, FR-339 and ADR-018 on main, against merged PR #1271 (STD-296, SR-16090), and against the heads of all other open QSL PRs.

These claims hold:

- The guarded automaton is a correct reduction for liveness. Its language is exactly the behaviours with some position whose condition reads true and from which the formula is false. The waiting phase reads every letter and branches to active only where the condition is true. The active phase is absorbing and its successors do not depend on the condition. The activation acceptance set contains exactly the active states. So an accepting run must select an activation and then satisfy the negated formula's least-fixed-point sets, and a run that waits forever is never accepted. Because active states never return to waiting, no SCC mixes the two phases, so FR-339's `MissingAcceptance{set: 0}` decided from phases is exact. The checker recomputes both successors of every waiting state, and a missing active successor is `SuccessorMissing`, so no certificate can drop a violating activation and get a `proved`.
- The index-0 claim is true. Every least-fixed-point identity is `[o,s]`, and its path `o` is an array, so its canonical bytes start `[[` (0x5B). `["activation"]` starts `["` (0x22), so it sorts first. This matches FR-338's existing rule (line 365) that the component checker orders sets by canonical bytes. Unguarded TP-4 keys and set indices do not change.
- The vectors can be computed by hand. Under weak `each`, the active SCC `{(1,0),(1,1),(1,2)}` of binding `a` has internal edges that are only `upd(b)`, while `upd(a)` is enabled throughout, so `UnfairWeak upd(a)` is the right witness. The nine-state waiting SCC carries `MissingAcceptance{set: 0}`. The result is `proved`. Under weak whole, the lasso is the stem `(0,0) -upd(a)-> (1,0)` plus three `upd(b)` steps, length 4. It is minimal because `a` needs one step to reach version 1 and `b` needs three steps to cycle (the example has no self-loops, as ADR-018 §6 and FR-126-AC-1 show). The tie goes to `a`. A guard that is never true yields only waiting runs, so the clause holds. With the condition undefined at version 1, both bindings reach it at depth 1, and the tie goes to `a` at position 1. In the `Counter` stutter case, `not GF(v=0)` is `FG(v!=0)`, which holds from value 3 on the stutter. The key vectors `["guarded-buchi","waiting",[[],[],[],[]]]` and `["guarded-buchi","active",[[[[],"negative"]],[],[],[]]]` agree with the existing `G not p` vector.
- The PR is consistent with #1271. `"each-when"` is extended to TP-4, the re-read rule is unchanged, and the TP-4 empty-certificate locus follows FND-007's rule. The new key is still derived from the context alone, here the waiting configuration before position 0, because TP-4 has no rejecting state. FR-130's second-phase rule ("a state of every acceptance set") already covers the activation set. ADR-018's TP-4 row needs no change. The SMT path refuses all TP-4 items as `item-form` (FR-315), on main and outside this ticket.
- No `GuardedActivation` remains in `spec/` outside review artifacts. The code removal is listed for the coder.
- Scope: nothing in the diff defers, narrows or drops part of STD-368. The fixture note "S4 refuses an `on each` clause `NotEmitted` (FR-337, QSL-926)" restates a refusal that is already on main, owned by another ticket. It is not a narrowing of this ticket.

Gate: `QSPEC_DIR=~/dev/quire-specification make check-spec-validation` exits 0 at the head. The gate does not detect the duplicate AC id in FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new FR-338-AC-11 duplicates an id on main. FR-338 already has `FR-338-AC-11` (Package identity, TC-1141), and FR-127-AC-13, FR-282-AC-10 and TC-1141 reference it. The PR adds a second row with that id, and TC-525's scope now claims FR-338-AC-11, so one id names two criteria. The ticket text prescribed AC-11, and the PR body's uniqueness claim is wrong. Open PR #1311 adds FR-338-AC-12 to AC-14. <br>Fix: renumber the new row to FR-338-AC-15 (or the next id not used on main or in any open PR head), and update TC-525's scope and step 10 to match. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:819-820, spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md:16 |
| FND-002 | medium | The text disagrees on whether an infinite-trace stutter position can start an activation. FR-338's guarded TP-4 statement says the clause is violated "from some model position whose condition reads true". FR-338:413 and FR-126:185 say a guarded activation "starts only at a model position". But FR-338's stutter bullet and FR-126:211 let a waiting configuration select an activation "at a model or stutter position". For a formula with a past operator, past memory at a repeated stutter position differs from past memory at the last model position, so the two readings can give different verdicts. <br>Fix: state once, in both FR-126 and FR-338, that an infinite-trace stutter position is an activation position (and that only a false-extension position is not), and use the same phrase in the semantic statement. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:413-417, spec/functional/FR-338-check-an-en-1-closure-certificate.md:474-477, spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:185-190 |
| FND-003 | medium | Replay of a guarded TP-4 counterexample is specified only as one sentence in FR-126, and nothing tests it. FR-128 is unchanged and never mentions an `on each` activation or a `when` condition, so the replay requirement does not say how it finds the activation position (a position whose condition reads true and from which the formula is false, with the condition read through the letter seam). None of FR-126-AC-16's new TP-4 `Violated` cases asserts a replay outcome, unlike FR-126-AC-17. A replay that ignores the guard and reads the clause `on origin` would pass every AC. <br>Fix: state the guarded replay rule in FR-128, or point to it from there, and add a replay assertion (`reproduced-with-evaluated-witness`) for the weak-whole lasso and the `Counter` stutter lasso in FR-126-AC-16 and TC-521 step 11. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:432-437 |
| FND-004 | medium | The new TP-4 empty-certificate `BadState` locus (the waiting key `K_C(["guarded-buchi","waiting",[[],P,[],A]])` before position 0) has no acceptance vector. Neither does a guarded TP-4 condition that is undefined at an initial model state. FR-339-AC-5's undefined vector uses a non-empty certificate and a non-initial state, `(1,0)`. Two implementers could build `P` and `A` differently, for example with or without position 0's letter applied, and no test would tell them apart. <br>Fix: add a vector to FR-338's new TP-4 AC (or FR-339-AC-5) where the condition is undefined at version 0, giving `BadState` at the initial `(0,0)` with the empty certificate, and state the exact locus key bytes. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:534-540 |
| FND-005 | low | The new TP-4 key AC says the keys have "exactly the stated `guarded-buchi` waiting and active state bodies" but does not state them. The only stated bodies are the generic `eventually holds(p)` rows in the vector table, and the AC's fixture is FR-339-AC-5's binding-bearing example, so a reader has to infer which bytes are asserted. <br>Fix: write the two bodies in the AC, as FR-338-AC-10 does. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:819 |

## Verdict

The construction is sound. It reduces the clause to a correct liveness check and cannot produce an unsound `proved`. The index-0 byte-order claim holds, and every hand-computable vector checks out. The PR is not mergeable until FND-001 is fixed, because the duplicate id makes TC-525 and TC-1141 trace one id to two criteria. FND-002 to FND-004 are wording and coverage fixes inside FR-126, FR-128, FR-338 and TC-521/TC-525/TC-526, and none needs a QSpec change. No scope-deferral was found.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The fix defines "model position" so that it excludes stutter positions: "A model position is an activation position, and so is every infinite-trace terminal stutter position". But two older sentences in FR-338 still use "model position" for every position the monitor reads. FR-338:384 says unguarded `on each` "adds the root activation at every model position", and FR-338:283 says past memory is "updated once per model position". Under the new definition, unguarded `on each` adds no activation at stutter positions, while `on each ... when true` does. Past memory would also not advance at a stutter position, which contradicts the new bullet's "past memory advanced to that stutter position". For formulas with past operators, `on each` and `on each when true` would then give different verdicts. <br>Fix: use "activation position" (or "every position the monitor reads") in FR-338:283 and FR-338:384, so that unguarded and guarded `on each` share one position rule. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:283, spec/functional/FR-338-check-an-en-1-closure-certificate.md:384, spec/functional/FR-338-check-an-en-1-closure-certificate.md:413-421 |
| FND-007 | high | The renumbered FR-338-AC-15 collides with open PR #1343 (STD-382, `spec/std-382-cert-memory-bound`), which adds its own FR-338-AC-15 ("Memory bounds", TC-1200). Whichever PR merges second would duplicate the id on main, the same defect as FND-001. <br>Fix: coordinate with STD-382 so that one PR takes the next free FR-338 AC id (FR-338-AC-16 if #1343 keeps AC-15), and update TC-525's scope and step 10 to match. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:828, spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md:16 |

## Dispositions

Round 1 is at head 9d50011be. Main is an ancestor of the head. `QSPEC_DIR=~/dev/quire-specification make check-spec-validation` exits 0, with no diagnostic in a changed file.

The stutter-activation rule is sound for past operators. Under infinite-trace stutter extension, every stutter position is a real position of the infinite word. So activating there, with past memory advanced through the repeated letter, is the faithful `on each` reading. Every past-memory entry is a Boolean flag or a counter saturated at `b+1`, so the stutter self-loop reaches a fixed point and the product stays finite.

The `ReplayParity` claim holds. When the condition reads false at every position, no activation is selected, so FR-128 evaluates `true`, and FR-128 settles `true` as `inconclusive`/`ReplayParity`. An `on origin` replay would still find `eventually (a.v = 2)` false on the lasso, because `a` stays at 1. So the vector separates a guard-aware replay from one that ignores the guard.

The `BadState` vectors also hold. With the condition undefined at version 0 for both bindings, binding `a`'s key is least, because its context differs from `b`'s only in the instance key `"a"` against `"b"`. For this formula the waiting body before position 0 is `[[],[],[],[]]`, since it has no past occurrence and no least-fixed-point set.

Merge conflicts: #1330 (STD-373) conflicts textually with this PR in FR-339, TC-521 and TC-526 (the AC table and the scope lines). #1311 (STD-358) conflicts textually in FR-338's AC table. Neither conflict is semantic. #1330's component order (topological, least vertex index first) lists the waiting component before the active one, which FR-339-AC-5's certificates satisfy. #1311's locus pairs include `BadState` at `ProductState`. Whichever PR merges second resolves the conflict as main plus both sets of rows.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Renumbered to FR-338-AC-15; FR-338-AC-11 is again the single Package-identity row; TC-525's scope names FR-338-AC-15. The new collision with #1343 is FND-007. |
| FND-002 | fixed | FR-126:185 and :211, FR-338:404, :413-421, :441 and :481 now all use "activation position", meaning a model position or an infinite-trace stutter position, never a false-extension position. The remaining older uses are FND-006. |
| FND-003 | fixed | FR-128 states the guarded replay rule. FR-126-AC-16 and TC-521 step 11 assert `reproduced-with-evaluated-witness` for both lassos, and `inconclusive`/`ReplayParity` with the condition false everywhere. |
| FND-004 | fixed | FR-338-AC-15 states the empty-certificate `BadState` with the exact waiting key, and the initial-state locus on FR-339-AC-5's certificate. TC-525 step 10 asserts both. |
| FND-005 | fixed | FR-338-AC-15 and TC-525 step 10 state both bodies byte for byte. |

Round 2 is at head fa139b39a. Main has moved by one commit (425bab9bb, #1350) and merges into the head without conflict. `make check-spec-validation` exits 0 with no diagnostic in a changed file.

The FND-006 fix deviates from a bare "activation position", and the deviation is right. Past memory now advances at every position the monitor reads, including a terminal false-extension position, whose atoms read false. TP-2 needs this, because its past operators must see the extension positions that FR-338's "Terminal false-extension advances precisely the remaining bounded positions with atoms false" already walks. Activation stays limited to activation positions. Unguarded `on each` now activates at infinite-trace stutter positions, as the guarded form does. That matches the reading of `on each` as every position of the stutter-extended infinite word, so `on each` and `on each when true` agree again.

Merge check:
- Against #1311, #1330, #1343 (now FR-338-AC-17/18) and #1342, each pairwise merge is clean.
- Merging main, #1310, #1311, #1330, #1343 and #1342 in that order hits one conflict, in FR-338's AC table. It comes from #1311's rows AC-12 to AC-14 against #1343's rows AC-17 and AC-18, and it reproduces with #1311 and #1343 alone, without #1310.
- FR-338-AC-16, TC-1215, FR-339-AC-5 and the `guarded-buchi` tag are unique across main and every open PR head.
- FR-339-AC-5 is traced by TC-1215, and TC-526's scope is restored to main's.
- To avoid textual conflicts, the AC rows are placed out of numeric order: FR-338-AC-16 sits before AC-11, and FR-339-AC-5 before AC-4. This does not affect validity.

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | fixed | FR-338:283 and FR-338:384 now use the shared position rule, so unguarded and guarded `on each` agree. |
| FND-007 | fixed | Renumbered to FR-338-AC-16, which is unique across main and all open heads. #1343 moved to AC-17 and AC-18. TC-525's scope and step 10 name AC-16. |

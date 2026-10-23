---
id: SR-16263
title: "spec-review of FR-125 instancing aligned to QSpec FR-361 and the FR-338 key context (quire-spec-language PR 1377)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@497647c0; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md; spec/functional/FR-338-check-an-en-1-closure-certificate.md; spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md; spec/test-cases/TC-1164-an-instance-key-context-binds-its-key.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: references
---

# Spec review of FR-125 instancing aligned to QSpec FR-361 (quire-spec-language PR 1377)

## Summary

Ticket: STD-438. One spec-review pass over `git diff -M origin/main...497647c0`.
The oracle was QSpec FR-361 "Binding of the clause's `over` parameter" and
FR-361-AC-5 to AC-16, at QSpec main d2fab1b0.

Examined, clean:

- **Overlap, admitted keys, excluded keys, order and identity by key** match
  FR-361.
- **The outcome order** (`RequiresBound`, `NoInitialState`,
  `conflicting-identity`, `NoInstance`) matches FR-361 and FR-361-AC-14 and
  AC-16.
- **The guard**, the disjunction of `lookup<T>(p, x)` over every overlapping
  population, matches FR-361.
- **FR-125-AC-10**, derived by hand: `P1 [d, c]` and `P2 [b, a]` give
  `d, c, b, a`, and `P2 [c, a]` gives `d, c, a`.
- **FR-125-AC-12**, derived by hand: `animals` sorts before `dogs`, so the
  instances are `k, t, d`. The unguarded claim's first refuted instance is
  `t`. `G` is false at `t`, because `t` holds a `Cat` and `dogs` does not
  list it.
- **FR-125-AC-13 and AC-14** match FR-361-AC-14 and AC-15.
- **FR-338-AC-9 and TC-1164**: `["instance", key]` follows from identity by
  key. The rename-invariance and distinct-key vectors are sound.
- Every deferral marker ("QSL-proposed", "Remaining work", "no guard for it
  yet") is gone, and nothing in scope is deferred.

Gates: `make check-spec-validation` at 497647c0 exits 0, with no warning on a
touched file. The PR merges cleanly with origin/main and with #1332, which
edits FR-125-AC-6, and with #1310 and #1311, which edit other FR-338 rows.
origin/main is not an ancestor of the head. FR-125-AC-13 to AC-15 are unique.
The TC-1164 rename leaves no stale reference outside committed reviews.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-125 contradicts itself on whether a refuted binding carries a type. The identity bullet says the key is "the name of a refuted binding" and that "none of them carries a population or an object type". FR-125-AC-9 says the instance is named "in a refuted binding, with no population and no type". The binding bullet says "A binding written out of the position (a refuted binding, a replay `over_binding`) carries the type of the member record admitted for its key". QSpec FR-361 says that the wire reference of a binding carries the admitted record's type, and that only the instance identity, the key, names no type. Fix: say that the instance identity (obligation identity, FR-338 context) is the key alone, and that the refuted binding and the replay `over_binding` carry (universe, admitted type, key). Restate FR-125-AC-9 and TC-520 step 8 to test that, for example the `Switch` key's refuted binding carrying `Switch`. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:144-148,156-158,278 | wrong-requirement |
| FND-002 | medium | FR-125-AC-12 and TC-520 step 11 never say that `dogs`' key `d` is admitted by an initial record. Yet they expect `d` as an instance, `G` true at `d`, and the `animals`-only guard false at `d`. Under the PR's own rule, a key no initial record admits is an excluded key and not an instance. Read literally, the instances are `k, t` and the vector fails. Fix: state that an initial state holds a `Dog` under `d` in `dogs`. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:281; spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md | wrong-requirement |
| FND-003 | medium | "The reported instance of a refuted item is the first refuted instance in that order, and its counterexample, basis and cause are that instance's" has no vector with two refuted instances. QSpec FR-361-AC-10 tests it: the cause follows the order, so permuting a universe list changes the reported instance and cause. FR-125-AC-12 has one refuted instance only. Fix: add a vector in which two instances refute by different causes (one false evaluation, one `undefined-evaluation`), in both universe orders, with a TC-520 step. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:140-144 | missing-requirement |
| FND-004 | medium | The new FR-338-AC-9 vector (and TC-1164 step 4) uses "an operation that replaces it [the `Counter` under `a`] by a `Fast`". QSpec FR-361 says that a key's type is unique per key, and FR-125 says "A key has one type across the subject", and the binding written out carries the admitted record's type. It is not stated that a run may hold a `Fast` under a key admitted as `Counter`, or whether that is a runtime `conflicting-identity`. If it cannot happen, the vector cannot be built. Fix: either state in FR-125 that an operation may re-create a key's object with a subtype (and what the binding then carries), or change the vector to one QSpec admits. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:742; spec/test-cases/TC-1164-an-instance-key-context-binds-its-key.md | wrong-requirement |
| FND-005 | low | Excluded keys are "each listed once, in the instance order", but every excluded-key vector (AC-8, AC-14) has one population, and AC-15 reuses AC-10's populations, where every key is admitted, so the excluded keys are empty. No vector shows a non-admitted key listed by two populations appearing once, at its first occurrence. Fix: add that case to FR-125-AC-10 or AC-15. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:130-136,283-284 | missing-requirement |

## Verdict

Not mergeable until FND-001 to FND-004 are fixed. The instancing rules follow
QSpec FR-361, and the hand-derived order and guard vectors are correct.
FND-001 is an internal contradiction in the new text.

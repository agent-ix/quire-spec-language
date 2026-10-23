---
id: SR-16231
title: "spec-review of the order of undefined checks within one joint position (quire-spec-language PR 1342)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@de413f15; spec/functional/FR-128-replay-a-model-counterexample.md; spec/functional/FR-176-check-a-universal-hyperproperty-by-self-composition.md; spec/functional/FR-183-replay-a-hyper-counterexample.md; spec/test-cases/TC-539-replay-reproduces-an-undefined-evaluation.md; spec/test-cases/TC-601-universal-hyperproperty-by-self-composition.md; spec/test-cases/TC-608-replay-a-hyper-counterexample.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: references
---

# Spec review of the order of undefined checks within one joint position (quire-spec-language PR 1342)

## Summary

Ticket: STD-381 (QSL-930). This is one spec-review pass over
`git diff origin/main...de413f15`. It covers FR-183's new reading order
(`μ_U`, `μ_E`, the letter, then contract conjunctions in quantifier order),
FR-176's "Product" statement, FR-128's single-subject order (letter before
contracts for a formula item, contracts before `P` for deadlock freedom), the
new ACs (FR-183-AC-9 to AC-11, FR-176-AC-5, FR-128-AC-7) and the TC-539,
TC-601 and TC-608 steps. The oracles were ADR-018 UE-4 and UE-6, ADR-023 §8
and HV-8, FR-126 (letter read at product-state creation, contracts at
expansion) and FR-177 "Undefined and undetermined evaluation" and "Ranking
findings".

Gates: `QSPEC_DIR=~/dev/quire-specification make check-spec-validation` at
de413f15 exits 0, with no finding on a touched file. The PR merges cleanly
with origin/main, but origin/main is not an ancestor of the head. The new AC
ids appear in no other open PR.

Examined, clean:

- **Vectors recomputed over ADR-023 §8** (states `(h, l)`, leaky
  `l = (i + h) mod 2`, secure `l = i`). FR-183-AC-9: position 0 is defined,
  `step(0)` makes `μ_U` undefined and gives `b = (1, 1)`, so the letter is
  undefined at 1. AC-10: `b`'s `G` is defined at 0 and undefined at 1. AC-11:
  `step(1)` gives `l = 1` on both, so the letter holds and both `G`s are
  undefined. FR-176-AC-5's guard-undefined case is first found at position 0.
  Each expected settlement follows from the stated order.
- **FR-176's order** matches FR-126 and ADR-018 UE-4. The letter is read at
  creation, contracts at expansion and `μ_U` on the outgoing steps, so the
  step entering `p` is read before `p`'s letter and contracts.
- **Deadlock-freedom order.** Reading contracts before `P` follows from
  terminality being known only after expansion.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-128-AC-7 contradicts FR-128-AC-6, and it depends on a capability that the same FR defers. AC-7 requires an undefined authored `pre CanInc` on `inc` to replay to `reproduced-with-evaluated-witness`. AC-6 (unchanged) refuses the same construct, an undefined `pre CanInc` over `Counter`, with `unsupported_construct`/`not-yet-implemented` (`AuthoredUndefined`), "since replay of an authored clause's undefined evaluation is not built", and calls its reproduction "Remaining work: QSL-898". TC-539 step 6 says "Not built". No implementation can pass both ACs. The new order rule also needs that reproduction. Under the standing rule, a deferral inside the ticket's own scope blocks the PR. Fix: specify the reproduction of an authored clause's undefined evaluation here, and remove AC-6's `AuthoredUndefined` refusal, its QSL-898 remainder and TC-539's "Not built". Otherwise show a real distinction between the AC-6 and AC-7 constructs. | spec/functional/FR-128-replay-a-model-counterexample.md:208-209; spec/test-cases/TC-539-replay-reproduces-an-undefined-evaluation.md:33-38 | wrong-requirement |
| FND-002 | medium | The HP-3 reading order has no engine statement behind it. FR-183 says the order is "the order EN-1 reads them (FR-176 'Product')". FR-176 is the ∀∀ engine and has no `μ_E`. FR-177, the HP-3 engine, is not amended: it orders findings within one state only as "the first finding its expansion recorded". It also makes an existential component's undefined contract conjunction `Undecided(UndecidedSuccessor)`, not an undefined evaluation. Yet FR-183 step 3 reads "each component's" contract conjunctions as undefined evaluations. Fix: state the same `μ_U`, `μ_E`, letter, universal-contract order in FR-177, and limit step 3 to universal components or define how an existential member's contract is read. | spec/functional/FR-183-replay-a-hyper-counterexample.md:156-168; spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:98-104,139-147 | missing-requirement |
| FND-003 | medium | `μ_U` before `μ_E`, which the ticket names, has no vector. The PR body says "No HP-3 vector of its own for `μ_U` before `μ_E`". FR-183-AC-6 covers an undefined `μ_E` alone, so an implementation that reads `μ_E` first passes every AC. Fix: add an HP-3 envelope at whose position both `μ_U` and `μ_E` have no value, with the `μ_U` locus reproducing and the `μ_E` locus settling `inconclusive`, `Verdicts`. | spec/functional/FR-183-replay-a-hyper-counterexample.md:159-161,279-281 | missing-requirement |
| FND-004 | low | FR-176-AC-5 does not name the counterexample's tuple. It says "an initial tuple with an `h` 1" and "a component whose `h` is 1". The canonical first such tuple is `a = (0, 0)`, `b = (1, 0)`, so the counterexample names `b`. Naming it makes the vector exact. Also, FR-128-AC-7 calls the `terminal when` predicate "also undefined there", although `P` is never read: the undefined `CanInc` leaves the expansion undetermined, so terminality is not established. | spec/functional/FR-176-check-a-universal-hyperproperty-by-self-composition.md:144; spec/functional/FR-128-replay-a-model-counterexample.md:209 | wrong-requirement |

## Verdict

Not mergeable. FND-001 is a contradiction between two ACs of FR-128. It
rests on a deferral (QSL-898) inside the ticket's scope. FND-002 and FND-003
leave the HP-3 half of the order unbacked and untested. The ∀∀ order, the
single-subject order and the recomputed vault vectors are correct. A main
merge is also needed (clean).

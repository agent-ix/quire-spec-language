---
id: SR-13614
title: 'Spec review of PR #1125: hyperproperty spec gaps, failure domain'
type: SpecReview
analysis: failure-domain
scope: 'agent-ix/quire-spec-language@14ab4727d; git diff origin/main...14ab4727d (PR #1125, branch qsl-spec-hyper-followups, base main 0799f31d8): ADR-023, FR-174, FR-176, FR-177, FR-178, FR-183, FR-184, TC-599, TC-601, TC-602, TC-603, TC-608, TC-609; read against origin/main code and origin/qsl-555-copy-swap 63b4fd17c (#1123) and QSpec main 4826c36; tickets QSL-553, QSL-554, QSL-555'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-023
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-174
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-176
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-177
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-178
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-183
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-184
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-599
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-601
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-602
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-603
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-608
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-609
  type: reviews
---

## Summary

Ticket: QSL-553, QSL-554, QSL-555. PR quire-spec-language#1125, head 14ab4727d. Failure-domain lens: unstated outcomes and edge cases the new rules create: an undefined evaluation at an empty X, several copy classes under the acceptance-set and fairness rules, `Undefined` counterexamples of an align-skip clause, and the ranking caps across instances.

Covered: an existential member's undefined, refused or incomplete contract (UndecidedSuccessor), a later instance that stops on a limit (its Stopped wins), position-0 caps (horizon 1, exhaustion skipped), fairness on some classes but not others (#1123 `unfair_classes` swaps only the classes without fairness), and replay of an align-skip `Undefined` counterexample (refused).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-177 | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-AC-12 | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-instances | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-174-classes | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-accept | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-178-undef | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-183-align | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| ADR-023 CS-3 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The NoPartnerPath refusal leaves the item with no reportable outcome. FR-177 says the engine 'SHALL refuse the item with `HyperRefusal::NoPartnerPath` and return no outcome ... The refusal carries no catalog code or cause.' No V-x settlement, no QSpec result and no catalogued refusal code is given. A caller that settles every item (FR-182's terminal record, QSpec FR-399) has nothing to render for this item. The PR marks it as a conservative default pending the owner's FR-400 decision, which is acceptable. But the FR should say how the refusal reaches the caller until then (for example as an internal refusal of the item that surfaces with the run's refusal code), or it should flag the gap explicitly with the QSpec FR-400 item it waits on. | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:114-120; spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:204 |
| FND-002 | low | The acceptance-set rule is per class, but the code applies it all-or-nothing. 'EN-1 SHALL reduce by a class only if the swap fixes the initial state ... and maps each of its acceptance sets ... to itself' reads per class, like the fairness rule before it, where classes without fairness 'are still swapped'. The next sentence says a failing swap stores 'the product whole'. In #1123 one failing exchange discards every exchange (translate.rs `automorphic &=`), so a clause with one good class and one failing class reduces by neither. Say so: 'if the swap of any class fails, EN-1 stores the product whole and names no reduction'. | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:107-113; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:538; spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:99-106 |

## Verdict

Mergeable on this lens if FND-001 is either stated or explicitly deferred to the owner's QSpec FR-400 decision. FND-002 is a low wording fix.

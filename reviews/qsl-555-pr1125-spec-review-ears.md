---
id: SR-13613
title: 'Spec review of PR #1125: hyperproperty spec gaps, EARS conformance'
type: SpecReview
analysis: ears-conformance
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

Ticket: QSL-553, QSL-554, QSL-555. PR quire-spec-language#1125, head 14ab4727d. EARS lens over every SHALL statement the diff adds or rewrites in FR-174, FR-177, FR-178, FR-183 and FR-184. The ADR rows are decisions, not requirement statements.

`make check-spec-validation` reports no grammar diagnostic on any touched file. The new statements follow the event, unwanted-behaviour or ubiquitous pattern with the engine or EN-1 as subject: FR-177's contract, empty-X refusal and path bullets, FR-178 Vacuity and the Undefined-path bullet, FR-183's align-skip refusal and FR-184's count bullet. The findings below concern how responses are bundled, not grammar.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-174 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-rows | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-classes | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-accept | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-hp6 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-lasso | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-177-contract | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-paths | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177 | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-names | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-rank | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-instances | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-lasso | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-178-undef | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-178-rank | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-178-vacuity | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-183-align | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| FR-184-count | spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The rewritten statement gives no response for the 'yes' half. 'QSL SHALL settle a reduction a request selects ... by ADR-023 §5's row ...: a row that reads "no" refuses it, and a row that reads "yes" is a reduction EN-1 does not build for a hyper product.' The 'yes' clause states a fact and no settlement. The V-8 settlement appears only in Behavior. Put the response in the statement: '... and settles a selection whose row reads "yes" V-8 unsupported-requested-capability'. | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:25-31; spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:68-76 |
| FND-002 | low | One bullet holds three requirements. 'Where the table reads yes ...' requires the V-8 settlement, forbids ignoring the selection ('SHALL NOT be ignored'), and gives a separate rule for ordering several selected reductions. Split the ordering rule into its own bullet, so it can be traced on its own. It has no AC (SR-13611 FND-010). | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:68-76 |
| FND-003 | low | The ranking bullet chains four ordering rules with five SHALLs: least position, kind order at one position, discovery order at one rank, and instance order. Split them into one statement each. Then each tie rule can be cited by itself, and FR-177-AC-8 and AC-11 can each name the rule they check. | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:137-145 |

## Verdict

Mergeable on this lens alone: three low findings, each a split or a missing response clause. None changes behaviour.

## Dispositions

Round 1, head f4b02702b, checked against origin/main 0799f31d8 and origin/qsl-555-copy-swap 63b4fd17c.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed in f4b02702b: QSL SHALL settle a reduction a request selects for a hyper item by ADR-023 §5's row for the item's form (ADR-023 PH-1 to PH-6): a row that reads "no" settles it V-6 `ReductionNotPreserving`, and a row that reads "yes" settles it V-8 `unsupported-requested-capability`, since EN-1 builds no such reduc |
| FND-002 | fixed | Fixed in f4b02702b: A selected reduction SHALL NOT be ignored. |
| FND-003 | fixed | Fixed in f4b02702b: The engine SHALL return the finding at the least joint position. - At one position an undefined evaluation SHALL come before a refused or incomplete `μ` or universal contract conjunction, or a contract conjunction that leaves a member's steps unknown, which SHALL come before an empty `X`, since a re |

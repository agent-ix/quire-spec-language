---
id: SR-13698
title: 'Spec integrity review of PR #1161: inline integer literal operands (QSL-940), spec-review/spec-integrity-analysis'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@59112ebf2; git diff origin/main...59112ebf2 (PR #1161, branch qsl-940-inline-literal, merge base 0582efb14): spec/functional/FR-092-key-type-parameter-and-declared-nodes.md, spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md, spec/test-cases/TC-1053-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md, spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md; read for consistency: FR-357, FR-358, ADR-013 O-09, TC-413, TC-904; ticket QSL-940'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-092
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-093
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-367
  type: reviews
---

## Summary

Ticket: QSL-940. PR: quire-spec-language#1161, head 59112ebf2. Method: spec-review with the spec-integrity-analysis sub-analysis, chosen because the diff re-keys golden vectors and amends requirement and test-case text that other artifacts quote. The re-keyed FR-092 vectors were recomputed independently: every preimage is canonical RFC 8785 and hashes to its stated key, E12 and E13 were re-derived by hand from main's E12 with the L6 reference replaced by an inline literal term typed at T2, and the group order and group digests of G4-G6 and G10-G15 were recomputed from the FR-092 refinement algorithm. All of them match. Unchanged vectors (L5, L6, E11, T2, P1, G1-G3, G7-G9 and the rest, 44 of 55) are byte-identical to main. `make check-spec-validation` exits 0 with no diagnostic on a changed line, and `tools/check-index-completeness.sh` passes. The defects are in text that quotes the old group digest or the old dependency order and was not amended.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-092 vector table and blocks E12, E13, G4-G6, G10-G15, group tables | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md | examined |
| FR-092-AC-11 | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md | examined |
| FR-093 Literal row of the lowering table | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md | examined |
| FR-093-AC-7 | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md | examined |
| FR-093-AC-12 | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md | examined |
| FR-367 InlineLiteral source, Behavior 4, FR-367-AC-2, FR-367-AC-6 | spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md | examined |
| TC-1053 step 2 expected result | spec/test-cases/TC-1053-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md | examined |
| TC-416 steps 5 and 6 expected results | spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md | examined |
| TC-413 step 9 | spec/test-cases/TC-413-type-and-declared-node-keys-match-golden-vectors.md | examined |
| FR-357 inputs and AC-17, FR-358 step 6, ADR-013 O-09 parity preimage | spec/functional/FR-357-replay-scalar-parity-claims.md, spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-093-AC-7 still says the recursive `f` group members carry the label `0b9e8d18…` in graph order G5, G4, G6; the producer, FR-092 and TC-416 step 5 now give `d5012d73…` and G5, G6, G4. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:807 |
| FND-002 | medium | FR-093-AC-12 and TC-416 step 6 say "in ascending digest order … G5 lists L1, E11 and G6, and G6 lists G4 and E13"; with the new keys G6 (`0e9afedd…`) sorts before E11 (`68fb5483…`) and E13 (`9b59de5f…`) before G4 (`9fd4b3fb…`), so the order is L1, G6, E11 and E13, G4. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:812; spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md:130-133 |
| FND-003 | low | FR-367-AC-6 describes `OperandNotParameter` as a graph child "neither a literal nor a parameter"; an integer literal operand of an FR-357 application is no longer a graph child, so the "literal" alternative cannot occur. | spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md:228 |
| FND-004 | low | TC-413 step 9 still expects L6 among the vectors of the recursion-group units; L6 is no longer in the `f` unit, and FR-092-AC-11 and the test now check it in a separate unit whose body is the literal `1`. The procedure step was not amended. | spec/test-cases/TC-413-type-and-declared-node-keys-match-golden-vectors.md:69-74,109-111 |

## Verdict

Not clean. The FR-092 vector re-keying is correct: every changed key, both group digests and both group orders recompute independently, and no unchanged vector moved. FR-092-AC-11, the FR-093 Literal row, FR-367 Behavior 4, FR-367-AC-2 and TC-1053 step 2 describe the new behaviour consistently and as behaviour, not status. No stale `InlineLiteral { node }` or "every integer literal is a node" claim remains under spec/ outside historical spec/reviews. FND-001 is a requirement claim the code now violates; FND-002 is a stated order that no longer holds. Both quote values this PR changed and need a text update in this PR. The FR-358 step 6 scope question (integer `Equal`/`NotEqual`) is recorded in the code-review artifact SR-13696.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-093-AC-7 now gives `d5012d73e2202fec9a38f7a24993c89552d4594e9ffe665c03a167680c3f91d6` and graph order G5, G6, G4, matching the recomputed group digest and order. |
| FND-002 | fixed | FR-093-AC-12 and TC-416 step 6 now list G5 as L1, G6, E11 and G6 as E13, G4, which is ascending digest order under the new keys. |
| FND-003 | fixed | FR-367-AC-6 now says the graph child "is not a parameter (an integer literal operand is an inline literal term, never a graph child)". |
| FND-004 | fixed | TC-413 step 9 adds `function one using v(): Integer pure { 1 }`, and its expected result takes L6 from that unit. |

---
id: SR-12068
title: "QSL-523 PR #932 gap analysis: graph core against ADR-018 SM-3/SM-4/EN-1 and FR-162 RetainedEdge"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0284181836139f07147116d0aa86fd0e7328ae8c; ADR-018 SM-3, SM-4, EN-1, LA-1; FR-162 Outputs (RetainedEdge), FR-162-AC-2; FR-126-AC-1, FR-126-AC-3, FR-126-AC-5, FR-126-AC-7 (as tagged); FR-101-AC-1, -2, -6, -7, -9, -11, -12, -13 (as tagged); qsl-analyze/src/model_check/{graph,product,scc}.rs, qsl-analyze/tests/it/graph_core.rs"
review_set: subset
---

## Summary

Ticket: QSL-523. PR quire-spec-language#932. Planless, diff-scoped gap
analysis. Plan completion: not assessed. Criteria were traced to tagged tests
by reading. No matrix was run (read-only brief). QSpec is private and is cited
by id only. The ticket's scope is the graph core only: retained edges, an
iterative SCC, and a generic product over qsl-eval's existing
`TransitionSystem` seam. It does not cover the FR-162 reductions,
fairness, replay or certification.

Code against intent:

- FR-162 Outputs `RetainedEdge { source, transition, target, permutation }`:
  implemented with the identity permutation, which FR-162-AC-2's last
  sentence requires without symmetry. Asserted in
  `coalescing_retains_incoming_back_self_parallel_and_repeated_edges` and
  `product_preserves_distinct_property_states_and_all_model_edges`.
- ADR-018 EN-1, product key `(model key, automaton key)`, every explored edge
  retained, SCC second-phase support: implemented in `product.rs`,
  `graph.rs` and `scc.rs`. Asserted by the product, SCC, deep-chain and
  three-node-oracle tests.
- ADR-018 SM-3 (a position after a step is read with that step's identity):
  `advance` receives the actual `ProductTransition`. Asserted by
  `model_step_anchors_reach_the_property_component_even_at_the_same_model_state`.
- ADR-018 SM-4 (terminal states: bounded closes, infinite stutters with its
  own identity): asserted by
  `bounded_terminal_closes_while_infinite_terminal_advances_property` and
  `product_keys_are_unambiguous_and_stutter_cannot_alias_model_identity`.
- ADR-018 SM-1, SM-2, SM-5 to SM-8: these are evaluator, translation,
  verdict and replay rules. None is in this slice's scope, so there is no
  gap here.

Bindings examined: see the reviewer data block. The two `TC-587`/
`FR-162-AC-2` bindings assert that AC's last sentence (without symmetry
every `permutation` is the identity) and nothing else. That partial binding
follows the repo's "Partial for" precedent and matches this slice's
`RetainedEdge` scope, so it is not raised. The FR-101 bindings assert
the FR-101 contract that `retain` follows (EN-1 says "in FR-101's canonical
successor order and with FR-101's state keys"). They name ACs that
qsl-eval's own tests already cover, so they do not change any matrix
status. The FR-126 bindings are the defect below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Graph-core unit tests carry `#[trace("TC-521", "FR-126-AC-1/3/5/7")]`. They are the only bindings for those criteria in the tree, but they assert none of their verdicts (`Holds{Exhaustive}` with 15 product states, `Violated` loops, `BoundReached`/`Stopped{ResourceExhausted,...}`, byte-equal counterexamples). So the computed matrix reports `check_model` acceptance criteria as tested when that engine does not exist yet. | qsl-analyze/tests/it/graph_core.rs:176, 208, 241, 262, 307, 485, 592, 662, 737, 832 |

## Verdict

FIX. The code meets the graph-core scope: retained unreduced edges, an
iterative SCC and a generic product over the existing seam. The tests'
behaviour is strong, with hand-computed and independent oracles. But the
`TC-521` trace tags claim coverage of FR-126 criteria whose behaviour this
PR does not build. TC-521 says it verifies `check_model`'s verdicts over
ADR-018 §6's subjects. These tests check SCC partitions and product edges of
synthetic graphs.

The fix is to remove the ten `#[trace("TC-521", "FR-126-AC-*")]` attributes
from `graph_core.rs` and change nothing else. The FR-101 and FR-162-AC-2
tags may stay. The doc comment at line 175 already says the binding is "not
the whole temporal example in this criterion", which confirms the mismatch.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | The ten `#[trace("TC-521", "FR-126-AC-…")]` attributes are deleted from qsl-analyze/tests/it/graph_core.rs: those tests do not check what FR-126-AC-1/3/5/7 require, and `check_model` does not exist yet. Verified by the team leader after the reviewer's last round (cap). |

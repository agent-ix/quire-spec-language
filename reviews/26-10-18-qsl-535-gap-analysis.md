---
id: SR-12812
title: "Gap analysis of FR-165 state-graph claims (QSL-535, PR #968)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f414b7f16304cb757661adc907c524a534418d24; PR #968; spec/functional/FR-165, qsl-forms/tests/it/state_graph_forms.rs, qsl-semantics/tests/it/state_graph_claims.rs"
review_set: subset
evaluated_revision: "f414b7f16304cb757661adc907c524a534418d24"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: reviews
---

## Summary

Ticket: QSL-535. Planless. In scope: FR-165-AC-2 and the form part of
FR-165-AC-1. `quoin matrix --json` (quoin 0.28.3): FR-165-AC-1 tagged,
FR-165-AC-2 tagged, FR-165-AC-3 untagged (out of scope, not raised).
Plan completion: not assessed.

## Verdict

AC-2 is fully and strongly covered: each of the three named refusals asserts
code, cause tag, exact source text and cause arm, with sole-refusal checks;
`self`/`result` and a non-Boolean predicate are extra. The S2 tests check
span text of each predicate. AC-1's S3 form tests are weak oracles (FND-001).
The one-line FR-165 spec clarification is behaviour, not build log, and
agrees with the code except for non-leading temporal operators (SR-12811
FND-002). No stubs or untraced production code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | S3 AC-1 tests assert only `body().is_some()` for `from`/`target`/`to`: swapping `from` and `target`, swapping `unique path`'s `from` and `to`, or checking one predicate twice all pass. The tests use the ConfigVersion fixture only, so AC-1's `CanStillWin from (x.phase != Lost)` enum predicate and `InOneWay` over `M::Job` are never checked at S3. | qsl-semantics/tests/it/state_graph_claims.rs:104-135 |
| FND-002 | low | FR-165-AC-1 shows as tagged in the matrix although its two-instance and node-identity parts are untested (deferred to STD-162/QSL-863), and the requirement-record test is tautological: it builds the record from a caller-supplied node and checks that node back. | qsl-semantics/tests/it/state_graph_claims.rs:150-170 |

## Coverage

Examined FR-165-AC-1 (form part) and FR-165-AC-2 against the eleven S3 and
five S2 tests tagged TC-590. Ran them green. Reverse check: every new
production symbol in qsl-forms/src/state_graph.rs and
qsl-semantics/src/check/state_graph_claim.rs is reached by a tagged test.

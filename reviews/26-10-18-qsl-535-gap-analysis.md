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

## Dispositions

Round 1, at 5456c3b03f01941ac09a6aeca5bc3f7344fa4801.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Oracle part fixed: the S3 tests compare each slot's checked literals against a separately checked reference and assert the slots differ, so a from/target or from/to swap now fails. The §7 `CanStillWin` (G::Game) and `InOneWay` (M::Job) S3 fixtures are a legitimate deferral because no Game/Job domain package exists and AC-2's named refusals use ConfigVersion only. No follow-up ticket names them yet; QSL-863 (which needs the §7.2 game for AC-3) should take them. |
| FND-002 | still-open | The `#[trace]` tags dropped FR-165-AC-1, but the `///` doc comments on nine tests still say `FR-165-AC-1:`. `quoin matrix --json` at 5456c3b still binds FR-165-AC-1 to those tests (static_status `tagged`). Rewording the doc comments (for example "Part of FR-165 AC-1") would make the matrix honest. The tautological record check now also asserts a different node does not match. |

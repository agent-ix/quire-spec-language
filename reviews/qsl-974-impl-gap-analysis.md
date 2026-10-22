---
id: SR-12332
title: "Gap analysis of quire-spec-language PR #1218: FR-093-AC-33, AC-34, FR-094 and TC-483 against the code (QSL-974)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3b1482029510e619bf48de6d6acdd81012ff2835; qsl-semantics/src/check/node_key/mod.rs; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/probabilistic.rs; qsl-replay/src/spine.rs; tests in qsl-semantics, qsl-package and qsl-eval"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-483
    type: reviews
---
# Gap analysis of quire-spec-language PR #1218

## Summary

Ticket: QSL-974. Plan completion: not assessed (planless). No spec file
changed in this PR. The merged spec text it implements:

- FR-093 "Quantity literals", with AC-33 and AC-34
- the FR-094 quantity-literal paragraph
- TC-483 steps 5 and 6
- QSpec FR-322 "Scalar literal values" and FR-322-AC-79 (current QSpec main)
- IR FR-038-AC-299..302 (quire-contract-ir main)

The file name differs from `reviews/qsl-974-gap-analysis.md`. That file is
SR-15483, the merged gap analysis of the spec PR #1214, and it is not
overwritten here.

Examined, with the test that backs each:

- FR-093-AC-33:
  - The `h`, `h2`/`h3` (`"1/2"` shared), `h4` (shares `h`), `n` (`"-3"`),
    `k` (`"1/1000"` at `s`) and `c` vectors: `quantity_literals_lower_to_shared_quantity_literal_nodes_the_strict_reader_admits`.
  - No `rational` literal is emitted: the same test asserts `assert_ne!(value_kind, "rational")` over every literal node.
  - IR's v2 reader admits each package: the same test asserts the read `package_id`.
  - Checking: `admitted_quantity_literals_check`.
  - Evaluation: `a_quantity_literal_evaluates_with_its_own_unit_magnitude`.
- FR-093-AC-34:
  - The codes and loci of all nine bodies and `f(3 s)`, with exactly one refusal each: `quantity_literal_refusals_carry_their_codes_and_locations` and `a_quantity_argument_of_another_unit_is_a_type_mismatch`.
  - The order ceiling, then zero denominator, then unit, then position: `a_quantity_literal_refuses_with_the_first_violated_condition_in_order`.
  - Mutation-checked. Moving unit resolution first, or the zero-denominator check before the ceiling, fails these tests.
- FR-322-AC-79 spelling (QSpec): `quantity_literals_are_spelled_as_qspec_scalar_literal_values`. It covers `3`, `0`, `-5`, `1/1000`, `-7/2` and `1/2`, matching IR FR-038-AC-299's admitted set.
- FR-094 (the literal is typed at the unit node a `Quantity<u>` parameter names, and adds no new type node): the admission test asserts `semantic_type == type == units[u]`.
- TC-620's two rewritten tests now assert the admission. The ceiling test keeps its past-the-ceiling refusal.

No production code without an owning requirement was found. The removed
`ProbabilisticCause::QuantityLiteral` has no requirement left that states it.
FR-093 and FR-185 "Package emission" now scope the S4 refusal to
probabilistic positions only.

## Verdict

Mergeable. Every AC-33/AC-34 code and order is tested and mutation-checked.
Two clauses of the ACs are asserted only loosely (FND-001, FND-002). A probe
confirmed that the code meets both, so they are test-strength gaps, not
behaviour gaps.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Three structural clauses of FR-093-AC-33 and TC-483 step 6 are not asserted. (a) "the call's argument references that node", for `h`. (b) `k` lowers `quantity.eq` with arguments `[ref(d's node), ref(the literal's node)]`. (c) `c` lowers `quantity.convert` with mode `rounding` = `exact`, then `quantity.eq` over the convert node and the `3 s` literal. `a_converted_quantity_is_compared_with_a_quantity_literal` only checks that the two identities occur somewhere in the package. A lowering that compared `d` itself, dropped the mode, or inlined the literal would still pass. A probe dump shows the code is correct today (call and eq arguments are `reference` terms to the literal node). | qsl-package/src/emit/tests/admission_corpus.rs:965-985 |
| FND-002 | medium | FR-093-AC-34 and TC-483 step 6 say the refusals name things. `missing-name` names `ghost` and `Time`, and `rational-bits-exceeded` names `s3.rational_bits` and `16`. `quantity_refusals` reduces each refusal to its catalog code and a location flag, so neither name is asserted. A probe shows the code is correct: `MissingName("ghost")`, `MissingName("Time")`, and `StageLimitCause { kind: RationalBits, limit: 16, actual: 17 }`. | qsl-semantics/src/check/assemble/tests.rs:2323-2349, 2379-2450 |

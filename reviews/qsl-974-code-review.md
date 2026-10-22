---
id: SR-12331
title: "Code review of quire-spec-language PR #1218: quantity literals lower to value_kind quantity (QSL-974)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3b1482029510e619bf48de6d6acdd81012ff2835; qsl-semantics/src/check/node_key/mod.rs; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/probabilistic.rs; qsl-replay/src/spine.rs; qsl-semantics/src/check/assemble/tests.rs; qsl-semantics/src/check/node_key/tests.rs; qsl-semantics/tests/it/probabilistic_declarations.rs; qsl-package/src/emit/tests/admission_corpus.rs; qsl-eval/tests/it/quantity_literals.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1218

## Summary

Ticket: QSL-974. The code half of QSL-974. The spec half merged as #1214. Covers
`LiteralValue::Quantity`, the lowering of `Value::Quantity`, the removal of
`ProbabilisticCause::QuantityLiteral` and its replay arm, and the new and
rewritten tests. Includes the rust-review lane.

Measured at 3b1482029:

- Node key. QSpec FR-322 "Scalar literal values" (current QSpec main) says a
  `quantity` literal's `type` is the `scalar_type`/`unit` node of its unit. Its
  `value` is the canonical decimal integer at denominator 1, and otherwise
  `n/d` in lowest terms with the sign on the numerator. `LiteralValue::Quantity`
  spells exactly that, and `Rational` is already reduced with a positive
  denominator. A probe dumped the emitted bodies. `3 ms` gives
  `{"term":"literal","type":<ms unit node>,"value":"3","value_kind":"quantity"}`
  and `rational(1, 1000) s` gives `"1/1000"` at the `s` node. `node["semantic_type"] == body["type"]`
  holds. The spellings match IR FR-038-AC-299's admitted set (`3`, `0`, `-5`,
  `1/1000`, `-7/2`). The locked quire-contract-ir (on IR main) admits each package
  through the strict v2 reader. The test asserts the read `package_id`. I re-ran it, and the probe read returned `ok`.
- Sharing. A probe put `3 ms`, `rational(3, 1) ms`, `rational(6, 2) ms` and
  `3.0 ms` in four functions of one unit. They lower to one `"3"` node. `3 s`
  and `3 ms` are two nodes (test case `a`). The call argument and the
  `quantity.eq` arguments are `reference` terms to the literal node.
- AC-34 order. `typing.rs` measures, then reduces (`exact::value`: ceiling
  before zero denominator), then resolves the unit, and then the position
  type runs. A unary `-` over `3 ms` refuses `operator-ineligible` at `-3 ms`.
  `rational(-3, 1) ms` lowers to `"-3"` and evaluates `< 0 ms`.
- `ProbabilisticCause::QuantityLiteral` removal. No reference remains in any
  `.rs`, `.md` or `.json` file in the repo. The only hits are history in
  `reviews/qsl-949-*`. No match in local main checkouts of
  quire-contract-codegen, quire-driver, quire-integration, quire-cli and
  quire-contract-ir. The rewritten doc on `ProbabilisticCause`, "Each variant
  but `RandomReadInPrecondition` carries its own span", matches `span()`.
- No in-repo reader decodes a literal `value_kind` from the wire, so the new
  variant affects no other consumer.
- Mutations, each restored afterwards:
  - Always spelling `n/d`: fails `quantity_literals_are_spelled_as_qspec_scalar_literal_values`
    and the admission test.
  - Alternating `3`/`3/1` per occurrence, which breaks sharing: fails the admission test.
  - Moving unit resolution before the magnitude: fails both AC-34 tests.
  - Moving the zero-denominator check before the ceiling: fails the order test.
- Gates:
  - Focused tests (`-- quantity`, 4 crates) pass.
  - `make string-edge`: exit 0.
  - `make arch-lint-api-surface-qsl`: exit 0.
  - `make check-spec-validation`: exit 0. The warnings are pre-existing, outside this diff.
  - `make seam-probe`: exit 0.
  - Clippy `--all-targets --all-features -D warnings` on the 4 crates: exit 0.
  - `cargo fmt --check` on the 4 crates: exit 0.

## Verdict

Mergeable. No high findings. The two lows below are about consistency only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The two magnitude refusals have different loci. The ceiling refusal is located at the whole literal (`rational(1, 100000) ms`, through the literal's `Location`). The zero-denominator refusal is located at the magnitude only (`rational(1, 0)`, through `ProbabilisticCause::ZeroDenominator { span }`). FR-093 "Refusals" states no locus for either, and the tests pin each as built. Pick one locus, or state both in FR-093, so a consumer does not have to learn two. | qsl-semantics/src/check/probabilistic.rs:668-676; qsl-semantics/src/check/assemble/tests.rs:2412-2430 |
| FND-002 | low | A zero denominator in a value-expression quantity literal is still raised as `CheckCause::Probabilistic(ZeroDenominator)` with a root `Location::root(Origin::Expression)`. So a function-body refusal is attributed to the Probabilistic family, and it resolves only through its span. A plain `rational(1, 0)` in the same body refuses through FR-091's own cause. The catalog code is the same (`undefined_expression`/`unproved-nonzero`), so nothing is wrong on the wire. Only the family attribution and the replay message path differ. | qsl-semantics/src/check/check/typing.rs:944-948; qsl-semantics/src/check/probabilistic.rs:672 |

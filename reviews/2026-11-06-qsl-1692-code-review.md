---
id: SR-20261105101066
title: "Code review of QSL-1692 numeric family sums"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1692; PR #2447 head 6377e1058; qsl-eval/src/value/expression/evaluate.rs, qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/check/lowering.rs, qsl-package/src/introduced_binders.rs, qsl-package/src/reader_protocol_projection.rs, qsl-eval/tests/it/quantity_literals.rs, tests/it/checked_handoff.rs"
review_set: subset
---

## Summary

Ticket: QSL-1692. Code review with the rust-review lane folded in, scoped to the PR diff. Behaviour was probed with throwaway tests (not committed) run through the production checked-handoff writer and Contract IR reader.

## Verdict

**NOT MERGEABLE.** Quantity, rational and decimal sums check, lower and are admitted by the reader, and the evaluator arms, empty identities and reader projection are well formed. But every `sum<Float32>`/`sum<Float64>` package is refused by the Contract IR reader (`OperationLawMissing`), and bounded `Rational`/`Decimal` sums skip the FR-145 prefix range proof that `Int` sums carry. Rust style is otherwise consistent with the surrounding code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `quire.op.collection.sum.float32`/`float64` are lowered with a rounding mode but no `IeeeProfile` law; the Contract IR reader refuses every such package `InvalidPackage`/`OperationLawMissing` (reproduced through `write`). Other IEEE operations carry `laws: vec![self.law(LawRole::IeeeProfile, ..)]`. | qsl-semantics/src/check/lowering.rs:4982-5013 |
| FND-002 | high | Bounded `Rational`/`Decimal` sums are admitted with no prefix-range proof: the FR-146 definedness walk returns `Ok(())` for any non-`Int` sum. `sum<R>` over `Sequence<R>[0,2]`, `R = Rational[-100,100;1,100]` (prefixes reach ±200) is written and admitted, while the same `Int[-100,100]` sum is refused `Unproved(Range)`. QSpec FR-145 refuses a linked sum whose prefixes are not all proved members. | qsl-semantics/src/check/facts.rs:1195-1197 |
| FND-003 | medium | A non-member Rational or Decimal seed returns an unlocated `Stop::Undefined(SumOutOfDomain)`, located at the `sum` node; the Integer arm locates it at the summand via `Halt::Located`, as FR-145 requires (located undefined outcome at that summand). | qsl-eval/src/value/expression/evaluate.rs:4172-4174 |

## New findings (disposition pass 2)

Round 2 re-measures every finding on PR #2455 (head 384f6c362), which supersedes PR #2447. Round 1 (Linear, PR #2447 head 1812f938b) recorded FND-001 to FND-003 fixed. That head is not an ancestor of #2455, and #2455 drops its `facts.rs` prefix proof. Probes were run as throwaway tests through `PackageDeclarations::check`, `emit_checked` and `read_checked_package_v2`, and were removed afterwards.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The PR's own tests lock in FND-002. `decimal_and_rational_sums_lower_to_their_family_identity` writes and admits `sum<D>` over `Sequence<D>[0, 4]` with `D = Decimal[0, 1000; 2, 2]`, whose prefixes reach 4000, and `sum<Q>` with `Q = Rational[0, 10; 1, 12]`, whose prefixes reach 40. `a_decimal_sum_is_exact_and_leaves_its_domain_undefined` asserts that a linked package reaches `Undefined(SumOutOfDomain)` at run time. FR-100 says that `run` never yields `sum-out-of-domain`, because linked checking proves every running total. | tests/it/checked_handoff.rs:2325-2331 |
| FND-005 | medium | FR-517-AC-20 says the carried-quantity family is included, but the typer admits only `Quantity(_)` as a sum total. The typer also has no `summand_of_family` arm for a carried summand. Two probes were refused `IllTyped(TypeMismatch)`: `sum<N>` with `N = Quantity<m, Int[0, 100]>`, and the same carried summands summed into `L = Quantity<m>`. Lowering maps `CarriedQuantity` to `UnsupportedFeature`. The PR's prose rewrite of FR-517 drops "carried" without changing AC-20. | qsl-semantics/src/check/check/typing.rs:617-629 |
| FND-006 | low | The six `quire.op.collection.sum.*` identities are listed by hand three times, with no shared constant or predicate. A family added at one site and missed at another becomes a reader fault. | qsl-package/src/reader_protocol_projection.rs:1549-1579 |

## Dispositions

Round 2, PR #2455 head 384f6c362.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `Visit::Sum` lowering at lowering.rs:4985-4995 carries `self.law(LawRole::IeeeProfile, ..)` for `Float(_)`. A probe emitted `sum<Float64[nearest-even]>` and `sum<Float32[toward-zero]>` with `laws: [ieee_profile]` and `mode: rounding`, and `read_checked_package_v2` admitted both. |
| FND-002 | still-open: facts.rs:1195-1197 still returns `Ok(())` for every non-`Int` sum. The #2447 fix (`rational_sum_prefixes`/`decimal_sum_prefixes`, head 1812f938b) is not on #2455. A probe of `sum<N>` over `Sequence<N>[0, 4]` was refused `Unproved(Range{required [0,1000], proved [0,4000]})` for `Int[0, 1000]`, and admitted for `Decimal[0, 1000; 2, 2]` and `Rational[0, 10; 1, 12]`. | - |
| FND-003 | fixed | `sum_step` (evaluate.rs:1615-1630) checks the seed with `value_type.admits(&summand)` for every family and returns `Halt::Located` at `body.location()`, the summand node. |

---
id: SR-13702
title: 'Code review of PR #1166: lower a checked equality conversion as a typed numeric.convert operand (QSL-944), code-review with rust-review lane'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@fe77ab328; git diff origin/main...fe77ab328 (PR #1166, branch qsl-944-converted-equality, merge base 4cd371d40): qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/check/check.rs, qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/converted_equality.rs; ticket QSL-944'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-093
  type: reviews
---

## Summary

Ticket: QSL-944. PR: quire-spec-language#1166, head fe77ab328, one commit. Method: code-review with the rust-review lane folded in.

The fix is in the checker. `equality_typed` (typing.rs:1627-1670) now wraps a type-changing `convert<T>(e)` equality operand in a `NodeKind::ConvertScalar(converted(S, T), e)` node typed `T`. `equality` (check.rs:2544-2600) first checks the pair with the original converted operands, so refusals are unchanged. When an operand is such a node, it checks a second time with that operand as `typed(T)` and stores that result, so `qsl-eval` does not convert twice.

Evidence. Probes ran in scratch copies of the head and of the merge base 4cd371d, with QSpec 014f0ce:

- Crate suites at head: qsl-semantics lib 688/689 and it 1158/1158; qsl-package lib 218/219; qsl-eval 115 + 651 (plus 0); qsl-replay 748 plus integration targets, all pass; qsl-analyze 64 + 603, all pass. The only failures are `conformance_fr322_application_keys_match_qspec_operation_vectors` ("integer-add: preimage does not decode") and `conformance_emitted_application_nodes_match_qspec_positive_fixtures` ("positive-operation-identities.json: quire.op.integer.mul"). Both fail the same way on the merge base against QSpec #323 (014f0ce). They come from QSpec's inline literal operands, which QSL #1161 (QSL-940) addresses, and are unrelated to this PR.
- Runtime, through check and evaluate (`run_in_losses`): `convert<Decimal>(a) = b`, `b != convert<Decimal>(a)`, Int to Rational, Int to wider Int, and both operands converted give the same result and the same meter `consumed` vector on base and head. No conversion is applied twice and no charge changes.
- Refusals: rounding `convert<Decimal[..;0,0]>(a) = b` over a scale-2 Decimal, and an out-of-range nested convert, refuse `IllTyped(TypeMismatch)` at the same location on base and head.
- Every place the checker records a conversion in an equality goes through `equality_typed`. `EqualityOperand::converted` is built only there and in the ordinary `convert` arm (check.rs:3420), which already built `ConvertScalar`. `contains` uses `typed` operands only, and ordering or arithmetic converts take the ordinary arm. Ordering `convert<D>(a) < b` emits `numeric.convert` + `decimal.lt` on both base and head.
- Consumers of `Equality` operands: lowering (fixed by the new node), `claims.rs` (`scalar_equality` now reads the converted type, and the convert becomes its own scalar claim, as IR has it), `qsl-eval` (above), the `qsl-replay` FR-358 composite site (a converted parameter operand is now neither parameter nor literal, which matches the emitted IR), and `facts.rs` (FND-001).
- Mutations, each run on the new tests and on the qsl-eval suite. M1 (no `ConvertScalar` node): 5 of the 6 new tests fail and the negative control passes, as it should. M2 (skip the second check): all 6 new tests and every suite of qsl-semantics, qsl-package, qsl-eval, qsl-replay and qsl-analyze pass, yet every cross-family converted equality then faults at evaluation (FND-002). M3 (re-type only the left operand): all pass, and the right-converted forms fault.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| `equality_typed` ConvertScalar construction | qsl-semantics/src/check/check/typing.rs:1627-1670 | examined |
| `equality` second check with typed operands | qsl-semantics/src/check/check.rs:2544-2600 | examined |
| `converted_equality` tests (6) | qsl-package/src/emit/tests/converted_equality.rs | examined |
| `equality_operand`, `PeerForm::Equality`, `admit_sum_into_peer` | qsl-semantics/src/check/check/typing.rs:1598-1621,2350-2410 | context_only |
| FR-146 facts `Equality` arm, `stable_path`, `literal` | qsl-semantics/src/check/facts.rs:456-500,563-578,700-707 | examined |
| `ConvertScalar` and `Equality` lowering | qsl-semantics/src/check/lowering.rs:3371-3381,3606-3632 | context_only |
| `is_scalar_application` / `scalar_equality` | qsl-semantics/src/check/claims.rs:373-376,421-436 | context_only |
| `NodeKind::ConvertScalar`/`Equality` evaluation | qsl-eval/src/value/expression/evaluate.rs:1702-1729,2025-2028 | examined |
| FR-358 composite-site operands | qsl-replay/src/execute/composite_site.rs:120-185 | context_only |
| `check_equality_in`, `admits_equality_conversion`, `CheckedEquality::run` | quire-semantic-value@ac0e137 src/declaration.rs:4741-5100 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Facts regression: a converted integer equality no longer refines its operand. `function g using v(x: Int[-5,5]): Int[0,3] pure { if convert<Integer>(x) = 2 then x else 0 }` checks, emits and is strictly read Verified on the merge base, and at head refuses `Unproved(Range{required [0,3], proved [-5,5]})` at body index 0, depth 1. The same holds for `convert<Int[-10,10]>(x) = 2` and for `if convert<Integer>(x) != 2 then 0 else x`, while `if x = 2 then x else 0` still checks. The FR-146 facts arm passes the equality's left node, now a `ConvertScalar`, to `comparison`. There `subject` (which unwraps only `Coerce`) and `stable_path` do not see through a conversion. Source that checked before is now refused. `subject` needs to unwrap a value-preserving integer `ConvertScalar` (Int or Integer to Int or Integer) as it unwraps `Coerce`, and a test must pin it. | qsl-semantics/src/check/facts.rs:533-545,700-707; qsl-semantics/src/check/check/typing.rs:1652-1661 |
| FND-002 | medium | The second check in `equality` (the stored `CheckedEquality` with converted operands as `typed(T)`) has no test. Disabling it (M2), or applying it to the left operand only (M3), passes all six new tests and every suite of the five crates. Yet with M2, `convert<Decimal>(a) = b` faults at evaluation with `InternalFault{stage: S6a, invariant: checked-program-invariant, kernel_cause: EqualityOperandSourceNotAdmitted}`, because the kernel converts an already-converted value. A qsl-eval test that checks and evaluates a converted equality (Eq and Ne, either operand order, true and false) is needed. | qsl-semantics/src/check/check.rs:2566-2587; qsl-package/src/emit/tests/converted_equality.rs |
| FND-003 | low | `equality` works out "this operand was converted" again from the node shape (`matches!(node.kind, NodeKind::ConvertScalar(..))` plus `target == value_type`), although `equality_typed` already knows it, and it runs `check_equality_in` twice for every converted pair. The two functions are coupled by node shape. Returning the stored operand from `equality_typed` next to the operand used for checking would state the intent directly and drop the predicate. | qsl-semantics/src/check/check.rs:2557-2587; qsl-semantics/src/check/check/typing.rs:1622-1626 |
| FND-004 | low | The doc comments still describe the old contract. `equality_typed` ("A typed equality operand, the equality operand it compares as, ...") does not say that it returns a `ConvertScalar` node for a type-changing conversion. `equality` ("each with the equality operand it compares as") does not say that the stored check compares converted operands as typed. | qsl-semantics/src/check/check/typing.rs:1622-1626; qsl-semantics/src/check/check.rs:2541-2543 |

## Verdict

Not mergeable yet. The approach is sound. Building a real `ConvertScalar` node in the checker fixes every converted-equality family the checker records, because `equality_typed` is the only place that records one. A probe through `PackageDeclarations` shows that Decimal to Int, both operands converted, and Quantity to Quantity (now `quantity.convert` with mode `exact`, then `quantity.eq` over the convert node) are all fixed as well. Refusals, runtime results and meter charges are unchanged. The new tests assert node identity by digest reference, the argument position and the convert's type, and M1 shows they can fail. FND-001 is a real regression: previously admitted guarded code is now refused. It must be fixed before merge. FND-002 leaves the runtime half of the change untested. FND-003 and FND-004 are small.

## New findings (disposition pass 1)

Head 402df736e (fix commit 193a6b530, then a merge of origin/main af16a844a). Evidence for this round:

- All five crates' tests pass at head: qsl-semantics lib 695 and it 1160; qsl-package lib 225; qsl-eval 115 + 652; qsl-replay 752 plus its integration targets; qsl-analyze 65 + 615. The two conformance tests now pass, since #1161 has landed. `make check-spec-validation` passes, with no warning in FR-093 or TC-415.
- Mutants, each run on `converted_equality` (11 tests), `a_converted_equality_operand_is_converted_once_and_compared` and a guard probe:
  - M1 (no `ConvertScalar` node): 8 tests fail.
  - M2 (no second check): the qsl-eval test fails.
  - M3 (store the right operand as checked): the qsl-eval test fails.
  - M4 (no `through_conversion`): the guard test fails.
  - M6 (unwrap only the left operand): the guard test fails, through `2 = convert<Integer>(x)`.
  - M5 (unwrap every `ConvertScalar`): survives.
- Guard probes at head, with Rational or Decimal in the guard position:
  - `if convert<Integer>(r) = 2 then convert<Int[-5,5]>(r) else 0` over `r: Rational[-5,5;1,1]` refuses `Unproved(Range)`, as does the same over `d: Decimal[-5,5;0,0]`.
  - `if convert<Decimal[..]>(x) = d then x else 0` refuses as well.
  - `if convert<Integer>(x) = 5 then x else 0` refuses with proved `[5,5]`.
  - So no non-integer conversion is read through, and nothing unsound is admitted.
  - `convert<Integer>(convert<Int[-10,10]>(x)) = 2` is read through and admitted.
- M5 is not behaviourally equivalent. `if convert<Int[-5,5]>(convert<Decimal[-5,5;0,0;nearest-even]>(x)) = 2 then x else 0` refuses at head and is admitted and read Verified under M5. The coder's argument holds only for a single conversion. There, unwrapping into a non-integer node changes nothing, because `subject` needs an integer type and `literal` an integer literal. A chain through Decimal back to Int is different, and M5 reads through it. M5 is still sound, because every `ConvertScalar` is an exact FR-149 row; rounding conversions are `ConvertDecimal`. So the survivor is a stricter-than-needed choice at head, not a defect.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | `through_conversion` sits in `comparison`, so ordering guards over an integer conversion also gain facts. `if convert<Integer>(x) < 4 then (if convert<Integer>(x) > -1 then x else 0) else 0` is refused under M4 (the pre-fix facts) and admitted at head. The FR-093 prose ("a guard comparing it with an integer literal") covers this, but FR-093-AC-32 and its tests name only `=`/`!=`. No test pins the ordering half. | qsl-semantics/src/check/facts.rs:659-662; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:844 |
| FND-006 | low | The `through_conversion` doc gives the wrong reason: "A conversion to any other type may round". A `ConvertScalar` never rounds, since it is built only for exact FR-149 rows and rounding is `ConvertDecimal`. The real reason to stop is that facts bound only integer subjects. The survivor M5 shows the stop is stricter than needed (an Int to Decimal to Int chain). Either reword the comment, or adopt M5 and pin the chain with a test. | qsl-semantics/src/check/facts.rs:637-650 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `through_conversion` reads equality and ordering operands through value-keeping integer `ConvertScalar` nodes. The three regression forms, plus `2 = convert<Integer>(x)`, are pinned by `a_guard_over_an_integer_conversion_bounds_the_converted_operand`, and M4 and M6 are killed. |
| FND-002 | fixed | `a_converted_equality_operand_is_converted_once_and_compared` evaluates `=`/`!=`, both orders, equal and unequal values, over Decimal, Rational and wider Int. M2 and M3 are killed. |
| FND-003 | fixed | The `TypedOperand {node, checked, stored}` struct carries both operands from `equality_typed`, and the node-shape predicate is gone. |
| FND-004 | fixed | The doc comments of `equality_typed`, `equality` and `TypedOperand` now state the checked-versus-stored contract. |

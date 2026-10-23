---
id: SR-16218
title: "spec-review of infix decimal arithmetic typing (quire-spec-language PR 1319)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@653125b5; spec/functional/FR-104-check-state-clauses.md; spec/test-cases/TC-1084-equality-types-an-arithmetic-operand-against-a-bounded-integer.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: references
---

# Spec review of infix decimal arithmetic typing (quire-spec-language PR 1319)

## Summary

Ticket: STD-366 (spec gap behind QSL-956). One spec-review pass over
`git diff origin/main...653125b5` (2 files, +24/-2). origin/main is an
ancestor of the head. Oracle: QSpec FR-140 ("Decimal typing derives no
default type", membership, target scale `smax`, division rule) on QSpec
main.

Gates run by the reviewer: `make check-spec-validation` exits 0.

Examined, clean:

- The `+`, `-` and `*` rule matches FR-140. Operands share one named
  `Decimal[..]` type, which is the result type with target scale `smax`. A
  literal takes the other operand's type, distinct types are
  `type-mismatch`, and a literal-only operation with no unique expected type
  is `ambiguous-literal`. There is no range obligation, and an out-of-range
  result refuses `decimal_out_of_domain`/`outside-domain` when evaluated.
- FR-104-AC-13 hand-checked. `P = Decimal[-99,99;2,2;nearest-even]` admits
  -0.99 to 0.99. `0.01 + 0.02 = 0.03` is a member, so the run returns
  `true`. `0.99 + 0.01 = 1.00` has `c* = 100 > 99`, so it is refused
  `outside-domain`. With `b: R`, the operands' types differ, so the check
  gives `type-mismatch`. The unannotated `let` of two literals is FR-140's own
  `ambiguous-literal` example.
- FR-104-AC-13 is unused on main and in every open QSL PR. TC-1084 step 4 is
  new.
- Behaviour-only wording; the lead default is labelled. No deferral or
  narrowing: the no-widening rule is FR-140's, not new.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | [soundness] "A result that needs a wider type takes it from a typed position, such as a declared function result or a `convert` target" contradicts the bullet before it and FR-140. The bullet before types the arithmetic from its operands "whether or not its position supplies an expected type". FR-140 gives operands and result one named type, and only a literal takes an expected type. So in `function f(a: P, b: P): W { a + b }`, `a + b` is still `P`, and `convert(a + b, W)` evaluates `a + b` in `P` first and refuses `outside-domain` before the convert runs. A wider result comes only from converting the operands, as in `convert(a, W) + convert(b, W)`. Two coders would read this sentence differently, and one of the readings widens. Fix: say that a wider result needs its operands converted to the wider type, and add that form to FR-104-AC-13 with `a = 0.99, b = 0.01` giving `1.00`. | spec/functional/FR-104-check-state-clauses.md:178-182 | wrong-requirement |
| FND-002 | medium | [coverage] The bullet adds typing rules for `/` (rounding into the target scale by the type's mode) and for unary `-`, but no AC or TC step tests either. FR-104-AC-13 and TC-1084 step 4 test only `+`, `-` and `*`. Fix: add to FR-104-AC-13 `a / b` typed `P`, with a run whose quotient rounds under `nearest-even` (for example `decimal(1,2) / decimal(8,2)` = 0.125, a tie that rounds to `0.12` under `nearest-even`, and `decimal(5,2) / decimal(2,2)` = 2.50, a non-member that is refused `outside-domain`), `-a` typed `P`, and `a / b` with `b: R` refusing `type-mismatch`. Mirror them in TC-1084 step 4. | spec/functional/FR-104-check-state-clauses.md:164-176, 338; spec/test-cases/TC-1084-equality-types-an-arithmetic-operand-against-a-bounded-integer.md:24 | correct-requirement-no-evidence |

## Verdict

Not mergeable yet: two medium findings. The core operand-typing rule
matches QSpec FR-140, and the AC's values check out by hand. FND-001 needs
the wider-result sentence corrected so it does not imply widening. FND-002
needs vectors for `/` and unary `-`, which the bullet specifies but nothing
tests.

## Dispositions

Round 1, at head cd3b7687 (fix diff checked against the original review head).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | fixed; after: result that needs a wider type `W` SHALL be written with its operands converted to `W`, as in `convert<W>(a) + convert<W>(b)`. A declared result or `convert` target gives the arithmetic no type of its own: `convert<W>(a + b)` evaluates `a + b` in the operands' type first, and a sum outside that type… |
| FND-002 | fixed | fixed; after:  2)` returns `decimal(100, 2)`, while the body `convert<W>(a + b)` at the same values is refused `decimal_out_of_domain`/`outside-domain`. `function d using v(a: P, b: P): P pure { a / b }` checks with `a / b` typed `P`; evaluated at `a = decimal(1, 2)`, `b = decimal(8, 2)` (exactly 0.125, a tie at … |

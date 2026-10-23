---
id: TC-1084
title: "An equality types an arithmetic operand against a bounded integer wherever it is written"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: verifies
---
# TC-1084: An equality types an arithmetic operand against a bounded integer wherever it is written

## Description

Verify QSpec FR-149's expected-type rule in a guard, an invariant and a function body, and the forms it does not coerce; and that infix decimal arithmetic takes its type from its operands (QSpec FR-140).

Scope: FR-104-AC-12, FR-104-AC-13.

## Test Procedure

Fixture: the header of FR-108's unit and TC-459 step 7's extended fixture package with an object type `Counter { tally: Int[0,3]; next: Int[0,3]; }` and an operation `bump` declared on it, written `Config::Counter`; a record `record Pair { tally: Int[0,3]; }`; the function `f` is `function f using v(earlier: Pair, later: Pair, n: Integer): Boolean pure { <body> }`.

1. Check `pre G using v on Config::Counter::bump { self.tally < 3 and self.next = self.tally + 1 }` and `invariant I using v on Config::Counter at current { self.tally < 3 and self.next = self.tally + 1 }`; check `f` with body `earlier.tally < 3 and later.tally = earlier.tally + 1`. Then each without its `tally < 3 and` guard.
2. Check `later.tally = earlier.tally` as the body of `f`.
3. Check `later.tally = -earlier.tally` and `later.tally = n` as the body of `f`.
4. Add `type P = Decimal[-99, 99; 2, 2; nearest-even];` and `type R = Decimal[0, 1000; 1, 1; exact];`, and let `c` be `function c using v(a: P, b: P): Boolean pure { <body> }`. Check `c` with the bodies `a + b = b + a`, `a * b = b * a`, `a - b = a - b` and `a + decimal(1, 2) = b`; with `b: R` and the body `a + b = b + a`; and with the body `let s = decimal(1, 2) + decimal(2, 2) in s = a`. Run `c` with body `a + b = b + a` at `a = decimal(1, 2)`, `b = decimal(2, 2)` and at `a = decimal(99, 2)`, `b = decimal(1, 2)`. Add `type W = Decimal[-198, 198; 2, 2; nearest-even];` and run `function w using v(a: P, b: P): W pure { convert<W>(a) + convert<W>(b) }`, then with body `convert<W>(a + b)`, at `a = decimal(99, 2)`, `b = decimal(1, 2)`. Check `function d using v(a: P, b: P): P pure { a / b }` and run it at `a = decimal(1, 2)`, `b = decimal(8, 2)` and at `a = decimal(5, 2)`, `b = decimal(2, 2)`. Check `c` with the body `-a = b`, and with `b: R` and the body `a / b = a`.

## Expected Results

- Step 1: each guarded form checks; each unguarded form refuses `undefined_expression`/`unproved-range` at the sum.
- Step 2: checks.
- Step 3: each refuses `ill_typed`/`type-mismatch` at the comparison.
- Step 4: the four bodies check, each arithmetic node typed `P` and the literal typed `P`; with `b: R`, `ill_typed`/`type-mismatch` at the first `a + b`; the `let` form refuses `ill_typed`/`ambiguous-literal` at the sum. The first run returns `true`; the second is refused `decimal_out_of_domain`/`outside-domain`. `w` returns `decimal(100, 2)`; its `convert<W>(a + b)` body is refused `decimal_out_of_domain`/`outside-domain`. `d` checks with `a / b` typed `P`, returns `decimal(12, 2)` at the tie and is refused `decimal_out_of_domain`/`outside-domain` at 2.50. `-a = b` checks with `-a` typed `P`; `a / b = a` with `b: R` refuses `ill_typed`/`type-mismatch` at `a / b`.

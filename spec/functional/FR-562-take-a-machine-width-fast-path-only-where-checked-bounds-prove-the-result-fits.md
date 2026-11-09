---
id: FR-562
title: "Take a machine-width fast path only where checked bounds prove the exact result fits"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-295
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-561
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-495
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-306"
    type: depends_on
---
# FR-562: Take a machine-width fast path only where checked bounds prove the exact result fits

## Description

ADR-029 EB-4 rule 2 lets generated code use a machine-width fast path only
where a checked bound shows the exact result fits. This requirement states
that rule so it can be checked.

The JIT backend (`qsl-jit`) SHALL compute, at `prepare`, an interval for each
`Integer` operand from its checked type: `[lo, hi]` for an operand of type
`Int[lo, hi]`, and for an operand that is itself a fast-path result, the
interval the rule below gave that result. An operand of unbounded `Integer`
type has no interval.

An `Integer` operation takes the fast path, computed in signed 64-bit
machine integers, only when every operand has an interval inside
`[-2^63, 2^63 - 1]` and:

| Operation | Condition |
| --- | --- |
| `+`, binary `-`, `*` | interval arithmetic over the operand intervals gives a result interval inside `[-2^63, 2^63 - 1]` |
| unary `-` | the negated interval lies inside `[-2^63, 2^63 - 1]` |
| `div` | the divisor's interval excludes 0, and the result interval lies inside `[-2^63, 2^63 - 1]`; the result interval is the smallest interval holding `quire-exact`'s quotient at every pair of operand interval ends, which bounds it because the divisor's interval excludes 0 and the quotient is monotone in each operand over an interval of constant divisor sign |
| `mod`, `rem` | the divisor's interval excludes 0, and the result interval given below lies inside `[-2^63, 2^63 - 1]` |
| `=`, `!=`, `<`, `<=`, `>`, `>=` | none beyond the operand condition; the result is `Boolean` |

The result interval of `mod` and `rem` is not taken from the operand interval
ends, because a remainder is not monotone in its operands (`a mod 5` is 0 at
`a` = 0 and at `a` = 10, and 4 at `a` = 9). Let `M` be the largest absolute
value over the divisor's interval, `max(|lo|, |hi|)`. The result interval
contains the operator's result at every pair of values of the operand
intervals, and is:

| Operator and law | Result interval |
| --- | --- |
| `mod` (always the Euclidean remainder), and `rem` under the Euclidean law | `[0, M - 1]` |
| `rem` under the Floor law, divisor interval above 0 | `[0, M - 1]` |
| `rem` under the Floor law, divisor interval below 0 | `[-(M - 1), 0]` |
| `rem` under the Truncating law, dividend interval at or above 0 | `[0, M - 1]` |
| `rem` under the Truncating law, dividend interval at or below 0 | `[-(M - 1), 0]` |
| `rem` under every other case of the Floor and Truncating laws | `[-(M - 1), M - 1]` |

The law for `rem` is the package's one selected integer-division law
(FR-495); `mod` takes none. A divisor interval that spans 0 never reaches the
JIT with a fast path: the checker refuses it (`unproved-nonzero`, FR-495)
unless a guard proves it nonzero, and an operation whose divisor interval
spans 0 calls the kernel.

The result interval of any other fast-path operation is the interval the
condition computed. Every other value operation, including every `Integer` operation
that fails its condition and every operation over `Rational`, `Decimal`,
`Text` or unbounded `Integer` operands, calls the kernel through the
`KernelTable` (FR-561). The decision is made from checked types at `prepare`
alone. Generated code never takes a fast path on a run-time test of the
operand values, and never falls back to the kernel after a machine overflow.

A fast-path operation charges the meter exactly as the kernel call it
replaces: the same charge kind and amount at the same point, polling cancellation there through the meter. It replaces only the arithmetic. Every range obligation the
interpreter checks at that operation, such as a result bound of the
enclosing function, is checked at the same point with the same outcome.
Boolean operators over `Boolean` operands are always machine operations, with
the interpreter's charges.

## Inputs

The checked types of each operation's operands; the call's arguments and
limits.

## Outputs

For each operation, the fast-path or kernel-call decision made at `prepare`;
the run's outcome, equal to the interpreter's (FR-295).

## Behavior

- When every operand of an `Integer` operation has an interval inside the
  signed 64-bit range and the operation's condition holds, the JIT's
  `prepare` shall generate the machine-width operation for it.
- A fast-path `mod` or `rem` shall take its result interval from the
  divisor interval's largest absolute value and the kernel's law, and never
  from the operand interval ends.
- When an operation's condition does not hold, or an operand is
  `Rational`, `Decimal`, `Text` or unbounded `Integer`, the JIT's
  `prepare` shall generate a `KernelTable` call for it.
- The JIT shall decide each fast path from checked types at `prepare`
  only.
- A fast-path operation shall charge the meter with the kind, amount and
  charge point of the kernel call it replaces.
- A fast-path operation shall check each range obligation the interpreter
  checks at that operation, at the same point.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-562-AC-1 | With a recording `KernelTable`, `f(a: Int[0, 4611686018427387903], b: Int[0, 4611686018427387904]): Integer { a + b }` (the largest sum is 2^63 - 1) records no kernel call, and `g(a: Int[0, 4611686018427387904], b: Int[0, 4611686018427387904]): Integer { a + b }` (the largest sum is 2^63) records one kernel `add` call for every call; each returns the interpreter's value at both range ends of each parameter. | Test (TC-1384) |
| FR-562-AC-2 | `m(a: Int[-3037000499, 3037000499], b: Int[-3037000499, 3037000499]): Integer { a * b }` records no kernel call; the same function with both bounds at 3037000500 records one kernel `mul` call per call; `n(a: Int[-9223372036854775808, 0], b: Int[-1, -1]): Integer { a div b }` records one kernel `div` call; each returns the interpreter's value at the range ends. | Test (TC-1384) |
| FR-562-AC-3 | `d(a: Int[0, 9], b: Int[1, 9]): Integer { a div b }` records no kernel call; `d0(a: Int[0, 9], b: Int[0, 9]): Integer { a div b }` records one kernel `div` call, and with `b` = 0 returns the interpreter's `Undefined`, cause `division-by-zero`, at the `div`'s `Location`. | Test (TC-1384) |
| FR-562-AC-4 | For each function of AC-1 to AC-3, the recorded meter charge sequence (kind, amount and charge point) of each call equals the interpreter's for the same arguments; with `work_units` one below the interpreter's measured spend, each returns `Incomplete` naming the same limit at the same charge point as the interpreter. | Test (TC-1385) |
| FR-562-AC-5 | `r(a: Rational, b: Rational): Rational { a + b }` called with 1 and 2, `t(a: Text, b: Text): Boolean { a = b }`, and `u(a: Integer, b: Integer): Integer { a + b }` called with 1 and 2 each record one kernel call, although each value fits a machine word. | Test (TC-1384) |
| FR-562-AC-6 | `s(a: Int[0, 9], b: Int[0, 9]): Int[0, 9] { a + b }` is refused `unproved-range` at the `+` under `CheckMode::Linked` (FR-495), because the addition's interval `[0, 18]` proves no bound, so no backend sees it. Checked under `CheckMode::Kernel`, supplied to `PackageDeclarations::check` as FR-495 Inputs state, so that the result bound is a run-time obligation, the JIT decides the addition from its operand interval `[0, 18]`, inside the signed 64-bit range: it records no kernel call for the addition, and called with 5 and 5 it returns the interpreter's outcome, cause and `Location` for the result bound, checked at the point the interpreter checks it. | Test (TC-1384) |
| FR-562-AC-7 | `s7(a: Int[0, 4], b: Int[0, 5]): Int[0, 9] { a + b }` checks under `CheckMode::Linked`, because the addition's interval `[0, 9]` proves the result bound; it records no kernel call and returns the interpreter's value at each pair of range ends. | Test (TC-1384) |
| FR-562-AC-8 | `rm(a: Int[0, 10], b: Int[0, 4000000000000000000]): Integer { (a mod 5) * b }` records one kernel `mul` call and no kernel `mod` call per call, and called with `a` = 9 and `b` = 4000000000000000000 returns 16000000000000000000, the interpreter's value: the results of `a mod 5` at the two ends of `a`'s interval are both 0, so an interval taken from the ends would be `[0, 0]` and put the product on the fast path, while the sound interval `[0, 4]` gives a product up to 1.6e19, outside the signed 64-bit range. `rn(a: Int[-10, 10], b: Int[2, 5]): Int[-4, 4] { a rem b }` under the Truncating law, `rf(a: Int[-10, 10], b: Int[-5, -2]): Int[-4, 4] { a rem b }` under the Floor law and `rj(a: Int[-10, 10], b: Int[2, 5]): Int[0, 4] { a mod b }` each record no kernel call and return the interpreter's value at every integer pair in the ranges. A test translator that takes the `mod` result interval from the operand interval ends fails `rm`. | Test (TC-1384) |

## Dependencies

- ADR-029 EB-4 rules 2 and 4.
- [FR-495](FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md): the division law, the nonzero obligation and `CheckMode`.
- [FR-295](FR-295-return-interpreter-outcomes-from-jit-and-aot.md): outcome and budget parity.
- [FR-561](FR-561-call-the-exact-kernel-from-generated-code-through-one-table.md): the kernel table every other operation calls.
- QSpec FR-306: a machine-width fast path only where a checked bound shows
  the exact result fits.

## Overlap

The driver's `quire-aot` follows the same rule in the code it generates,
under its own record; FR-295's parity gate holds it to the interpreter
either way.

## References

- QSL-393 (V1-A06a).

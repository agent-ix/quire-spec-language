---
id: FR-495
title: "Reach the integer-division, zero-divisor and state-core rows from QSL source"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-044
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-140
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-147
    type: depends_on
---
# FR-495: Reach the integer-division, zero-divisor and state-core rows from QSL source

## Description

QSpec's scalar vectors (TC-192 integer division, TC-185 decimal, TC-187
quantity) name outcomes that a checked QSL program meets at two different
stages: a refusal the checker issues from source, and a kernel outcome the
evaluator meets only when the checker did not prove the obligation. This
requirement states which QSL source text reaches each row and which
observation each stage gives, so a row is never classified admission-only for
lack of a source route. It owns four things: the range the checker gives an
integer `div`, `rem` or `mod` (QSL-1389), the two observations of a bounded
result domain (QSL-1389), the two observations of a zero divisor (QSL-1393),
and the order in which a state-core declaration is refused (QSL-1392).

## Inputs

- A unit whose declarations use `div`, `rem`, `mod`, `/` over decimal,
  rational or quantity operands, or the clause forms `invariant`, `pre` and
  `post`.
- A check mode, `CheckMode::Linked` (the default) or `CheckMode::Kernel`
  (FR-068), supplied to the package-level check (`PackageDeclarations::check`).
- For a Kernel evaluation, the function or expression position whose checked
  `Int[lo, hi]` type is the consumer domain.

## Outputs

- Under `CheckMode::Linked`, a checked package or a located check refusal.
- Under `CheckMode::Kernel`, a checked package whose S6a evaluation returns the
  kernel outcome of QSpec FR-147 and FR-140 (a completed value, `Undefined`,
  `Refused` or `Incomplete`).

## Behavior

- **Range of an integer division.** The checker SHALL give `a div b`, `a rem b`
  and `a mod b` the smallest interval that contains the result of the operator
  for every pair `(x, y)` with `x` in the range of `a`, `y` in the range of `b`
  and `y` not zero, taking the retained division law for `div` and `rem`
  (QSpec FR-147) and the Euclidean remainder for `mod`. When both ranges are
  singletons the result range is the singleton of the exact result. The
  operator's exposed member alone decides the range (QSpec FR-147: the
  unexposed member is never checked). A result range inside a bounded
  `Int[lo, hi]` position is proved; otherwise the checker SHALL refuse
  `undefined_expression`/`unproved-range` at the operator. The nonzero
  obligation of the divisor is decided first.
- **Bounded result domain, two observations.** QSpec FR-044 states that an
  expression its check admitted has its result membership proved, so a bounded
  consumer's out-of-domain refusal of QSpec FR-147 is met only by an expression
  that did not pass that check. QSL gives each such row two observations:
  - Under `CheckMode::Linked` the observation is the check refusal of the
    previous bullet, located at the operator, with no evaluation.
  - Under `CheckMode::Kernel` the checker SHALL record no range or nonzero
    obligation for `div`, `rem` and `mod`, as it records none for `sum`
    (FR-100, FR-096-AC-14), and S6a evaluation SHALL return QSpec FR-147's
    outcome for the operands and the consumer domain with QSpec FR-147's
    charges. The consumer domain is the checked result type of the evaluated
    function. The evaluation entries (`call`, `evaluate`, `evaluate_closed`)
    take no result-domain parameter.
  - A kernel `DivisionOutOfDomain` and `ModuloOutOfDomain` render as FR-096
    states: `division_out_of_domain` with `quotient-outside-domain` or
    `remainder-outside-domain`, `modulo_out_of_domain`/`outside-domain`, each
    with `expected` the consumer domain and no member exposed.
- **Zero divisor, two observations.** A division carries a nonzero obligation
  (FR-107, FR-091, FR-104 "Definedness facts"), so a zero divisor is not met at
  S6a under `CheckMode::Linked`. For an integer `div`, `rem` or `mod`, a
  decimal or rational `/` and a quantity `/`:
  - Under `CheckMode::Linked` a divisor that is a zero literal, or whose range
    contains zero with no exact nonzero guard (QSpec FR-044-AC-7), refuses
    `undefined_expression`/`unproved-nonzero` at the operator before
    evaluation. A divisor whose range excludes zero, a nonzero literal or an
    exact nonzero guard discharges the obligation.
  - Under `CheckMode::Kernel` a zero divisor evaluates to the kernel
    `Undefined::DivisionByZero`, spelled `division-by-zero` (FR-100), at the
    charge QSpec states for the operation, with no value and no loss record.
- **Declaration form before expression form.** Under a header profile that
  names only `quire.state.core/v1`, S3 SHALL refuse each declaration whose form
  the layer does not admit (the layer admits `invariant`, `pre` and `post`;
  QSpec `state-core.md`) as `unsupported_construct`/`declaration-form` at the
  declaration, naming the layer, and SHALL NOT classify any expression of that
  declaration (FR-110 "Layer selection"). A `function` declaration is therefore
  refused as a declaration, and the operators in its body receive no
  `expression-form` refusal. This order is intended. An integer `div`, `rem`
  or `mod` in an admitted clause form refuses `unsupported_construct`/
  `expression-form` at the operator, naming the layer, including in a dead
  branch (FR-091-AC-39). The state-only subcases of QSpec TC-192 DIV-17 and
  DIV-18 are written as clause bodies; their complete-facet subcases are
  written as function bodies under `quire.value.complete/v1`.
- **SMT verdict row (QSpec TC-192 DIV-14).** The provider and route are the
  ones QSpec FR-349 and TC-266 already state: the SMT backend (cvc5, with Z3 as
  a cross-check whose absence is a warned outcome, FR-349-AC-6) registered
  under the AD-010 provider contract, an FR-147 mathematical-integer obligation
  carried as a value-validity item (QSpec FR-484), and the verdicts `proved`
  and `refuted`, a `refuted` result carrying a counterexample that replays
  through the QSL complete-V1 executor (QSpec FR-197, FR-098). The row is not
  a QSL evaluation and is not admission-only: until a cvc5 provider is in a
  consumer's lock the row is pending with that missing provider as its owner
  work, and no synthetic verdict counts. QSL adds no verdict route of its own.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-495-AC-1 | Under `quire.value.complete/v1` and each of the three division definitions, `function s1 using v(a: Int[10, 10], b: Int[3, 3]): Int[3, 3] pure { a div b }`, `function s2 using v(a: Int[10, 10], b: Int[7, 7]): Int[3, 3] pure { a rem b }` and `function s3 using v(a: Int[-7, -7], b: Int[3, 3]): Int[2, 2] pure { a mod b }` check, as does `function s4 using v(y: Int[1, 10]): Int[1, 10] pure { 10 div y }` (QSpec TC-192 DIV-15 and DIV-16: the quotient range is `[1, 10]`) and `function s5 using v(x: Int[-10, 5]): Int[-10, 5] pure { x rem (-1) }` (the remainder range is `[0, 0]`, although the quotient of `-10` would be `10`, outside the domain). `function s6 using v(a: Int[10, 10], b: Int[5, 5]): Int[1, 10] pure { a rem b }` refuses `undefined_expression`/`unproved-range` at the `rem` (DIV-15's `10 rem 5`, remainder `0`); `function s7 using v(a: Int[-9223372036854775808, -9223372036854775808], b: Int[-1, -1]): Int[-9223372036854775808, 9223372036854775807] pure { a div b }` refuses `unproved-range` at the `div` (DIV-05, DIV-06), while the same function with `b: Int[1, 1]` and `a` at either signed-64 endpoint checks; `function s8 using v(a: Int[-7, -7]): Int[0, 1] pure { a mod 3 }` refuses `unproved-range` (DIV-12: the Euclidean remainder `2`). `function s9 using v(a: Int[1, 10], b: Int[-2, 2]): Int[-10, 10] pure { a div b }` refuses `undefined_expression`/`unproved-nonzero` at the `div`, not `unproved-range`. | Test (TC-1284) |
| FR-495-AC-2 | Under `CheckMode::Kernel`, under each of the three division definitions and for the checked result type shown as the consumer domain, S6a evaluation of `(-9223372036854775808) div (-1)` over `Int[-9223372036854775808, 9223372036854775807]` returns `refused { code: division_out_of_domain, cause: quotient-outside-domain }` with `expected` that domain and no quotient, while `(-9223372036854775808) rem (-1)` completes with `0` (DIV-05); `MIN div 1` and `MAX div 1` complete and `MIN div -1` is refused as above (DIV-06); `(-7) mod 3` over `Int[0, 1]` with QSpec DIV-10's tuple except `result_units: 0` returns `refused { code: modulo_out_of_domain, cause: outside-domain }` after `integer-modulus.domain` with no `integer-modulus.result-retain` charge, and with `work_units: 2` returns `incomplete` at `integer-modulus.domain` (DIV-12); over `Int[1, 10]`, `10 div 3` returns `3`, `10 rem 7` returns `3` and `10 rem 5` returns `refused { code: division_out_of_domain, cause: remainder-outside-domain }` after `integer-division.domain` (DIV-15); `10 div y` at `y = 5` over `Int[1, 10]` returns `2` and `x rem (-1)` at `x = -10` over `Int[-10, 5]` returns `0` (DIV-16). No refusal exposes a quotient or remainder. | Test (TC-1284) |
| FR-495-AC-3 | `function d1 using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 9] pure { a div b }`, the same with `rem` and `mod`, `function d2 using v(a: P, b: P): P pure { a / b }` with `type P = Decimal[-1000, 1000; 0, 2; nearest-even];`, `function d3 using v(): Boolean pure { 1 cm / 0 cm = 1 cm / 0 cm }` (with TC-187's units) and `function d4 using v(): P pure { decimal(1, 0) / decimal(0, 3) }` each refuse `undefined_expression`/`unproved-nonzero` at the operator under `CheckMode::Linked`; `function d5 using v(a: Int[0, 9], b: Int[1, 9]): Int[0, 9] pure { a div b }` and `if b != 0 then a div b else 0` over `b: Int[0, 9]` check (the guard is an exact nonzero guard; the second function's result range is inside `Int[0, 9]`). Under `CheckMode::Kernel`, the same bodies evaluate at a zero divisor to `Undefined::DivisionByZero` (`division-by-zero`): `decimal(1, 0)` divided by `decimal(0, 3)` into `Decimal[-1000, 1000; 0, 2; nearest-even]` is undefined with no decimal value or loss record (TC-185 D06), and D15's tuple with `work_units: 1` is undefined after `decimal.operands` with no scale-expansion charge, with `work_units: 0` `incomplete` at `decimal.operands`; `7 div 0` and `mod(7, 0)` follow TC-192 DIV-01 and DIV-11, and `1 cm / 0 cm` follows TC-187 U27. | Test (TC-1284) |
| FR-495-AC-4 | Under a header profile `quire.state.core/v1` alone, `function f using v(a: Int[0, 9]): Int[0, 9] pure { a div 3 }` refuses `unsupported_construct`/`declaration-form` at the function declaration, naming `quire.state.core/v1`, and no `expression-form` refusal is made for its `div`; the same unit with `rem` and with `mod`, and a function whose only operator is the dead branch of `if true then 1 else (7 div 3)`, each refuse once, as a declaration. The same operators in the body of `invariant`, `pre` and `post` clauses over a model object type (FR-361-AC-5's unit) each refuse `unsupported_construct`/`expression-form` at the operator, naming `quire.state.core/v1`: `1 + 7 div 3 * 2 = 0`, `7 rem 3 = 0` and `-7 mod 3 = 0` (QSpec TC-192 DIV-17, state-only), and `if true then 1 else (7 div 3) = 0`, `if true then 1 else (7 rem 3) = 0` and `if true then 1 else (7 mod 3) = 0` (DIV-18, dead branch), the operator located at its own span. Under `quire.value.complete/v1` the same function bodies check and evaluate. | Test (TC-1284) |
| FR-495-AC-5 | With a cvc5 provider registered, QSpec TC-192 DIV-14's holding obligation settles `proved` and its violated obligation settles `refuted` through the FR-349 route, and the `refuted` counterexample replays through the QSL complete-V1 executor to the violation (FR-098); a lock with no provider reports DIV-14 pending on the provider and never `proved` or `refuted`; Z3 absent leaves cvc5's verdict settled with the absent-cross-check warning. | Test (TC-266, TC-1284) |

## Dependencies

- QSpec FR-147 and TC-192 own the division laws, the charges and the
  DIV rows; QSpec FR-140 and TC-185 own the decimal D rows; QSpec FR-142 and
  TC-187 own U27. FR-044 owns the proof obligation.
- FR-100 and FR-096 own the kernel outcome mapping and the Kernel check mode
  precedent for `sum`.
- FR-110 and QSpec `state-core.md` own the layer's admitted forms.
- Rows that need a state-only integer `/` refusal code (TC-192 DIV-18's slash
  subcase) and a source spelling for a Decimal- or Integer-domain quantity
  target (TC-187 U07, U08, U16 to U18, U29) are not specified here; see
  QSL-1389 for the open questions.

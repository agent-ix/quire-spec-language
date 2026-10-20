---
id: FR-185
title: "Declare random parameters, workloads and rewards"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-272
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-185: Declare random parameters, workloads and rewards

## Description

S3 SHALL admit the three declarations that make a QSL state model
probabilistic (ADR-024 §1): a distribution on an operation parameter (a
**random parameter**, PM-1), a **workload** that weights every operation of
a model (PM-2), and named **rewards** on an operation (PM-5). It SHALL check
each declaration's well-formedness and record it on the checked package, so
FR-187 can give transitions step probabilities and reward values. QSpec owns
the spelling and the semantics; this requirement specifies the compiler's
checks.

## Use case

A modeller writes `random Server::attempt(d) ~ { 1: 90, 3: 7, 12: 3 };`, with `d` typed
`Millis = Int[1, 12]`, a `workload Steady on Service` with one weight per operation and
`reward duration on Server::attempt = d;` (FR-311). S3 accepts the model, or refuses it with a diagnostic
that names the declaration and the rule it breaks.

## Inputs

- A parsed QSL package with state models, operations, workloads and
  rewards.

## Outputs

The checked package gains, per operation, its random parameters with their
distributions and its rewards, and per model its workloads:

```rust
pub struct RandomParameter {
    pub parameter: WireNodeId,
    pub support: Vec<(Value, Rational)>,   // declared order; weights > 0
}
pub struct Workload {
    pub id: WireNodeId,
    pub model: WireNodeId,
    pub weights: Vec<(WireNodeId, Rational)>, // one per operation of the model
}
pub struct Reward {
    pub name: Identifier,
    pub value_type: ValueType,             // integer or quantity
    pub expression: CheckedExpression,     // reads parameters only
}
```

`Rational` is an exact rational held as a pair of arbitrary-precision
integers in lowest terms.

## Behavior

Every refusal below that carries a bracketed string is an
`invalid_probabilistic_model` diagnostic whose cause is that string, and it retains the declaration locus and
the parameter, reward, workload or operation it names. Every cause except
`duplicate-reward` is in QSpec FR-272's closed set for that code (QSpec
FR-405). `duplicate-reward` is a QSL-proposed cause that QSpec does not yet
define; it is requested of QSpec in STD-188, and QSL emits it until QSpec
adds it or names another.

A weight is an `exact-value`, and `exact-number` admits a signed integer, so
`rational(-1, 1)` parses and S3 refuses it `invalid-weight`. A bare `-1` is
not an `exact-value`, so S1 refuses it `invalid_syntax`.

### Exact values

- S2 holds each weight as written (FR-393) with unbounded integers. Before
  S3 reduces a weight to lowest terms, it SHALL measure the weight as
  written: the bit length of the magnitude of each written integer, and for
  `decimal(m, s)` and a `decimal-literal` the bit length of the denominator
  `10^s`, which is `⌊s · log₂ 10⌋ + 1` (1 at `s = 0`). S3 SHALL compute
  that bit length in exact integer arithmetic from `s`, without expanding
  `10^s` and without floating point. The counter is the largest of those
  bit lengths.
- If that counter exceeds the rational-bits ceiling of the S3 checking limits
  (a `CheckingLimits` field, default 65,536 bits; its setting name and limit
  kind are specified with QSL-883, pending), S3 SHALL refuse the declaration
  with a limit refusal (`LimitExceeded`) pending QSL-883, before reducing or
  expanding the value. The ceiling is inclusive and is not the exact engine's
  own `max_rational_bits` budget (FR-203), which bounds values that engine
  computes. The bound is on the value as written, so
  `rational(n, n)` with an over-limit `n` is refused although it reduces to
  1, and `decimal(25, 3)` is within the bound and reduces to `1/40`.
  Remaining work: QSL-883 (the setting is not yet implemented).
- The same measure, bound and refusal apply to every `exact-value` that
  FR-186 reads. This refusal is a limit refusal, not an
  `invalid_probabilistic_model` cause.
- The total of a distribution's weights, which S3 derives to record each
  probability, SHALL be bounded by the rational-bits ceiling, measured after
  each addition as the larger bit length of the running sum's reduced
  numerator and denominator.
- If the running sum exceeds the ceiling, then S3 SHALL refuse the
  distribution with the same limit refusal (`LimitExceeded`, carried as
  `stage_limit_exceeded`/`work-budget-exceeded`), naming the parameter.
- S3 SHALL poll the checker's cancellation before each addition (FR-276); a
  check whose handle is tripped before it starts ends cancelled, with the
  handle reporting requested, and the limit it reports is not the 65,536-bit
  ceiling.

Notes. The ceiling is the fixed rational-bits ceiling (65,536 bits,
inclusive) of the rule above; its `CheckingLimits` field waits on QSL-883.
S3 derives no workload total (it records the raw weights; FR-187 divides by
the sum over the operations enabled at each state), and no reward sum. A
probability is the quotient of two values within the ceiling, so it is
bounded by twice the ceiling in bits, which is the bound FR-187's products
of probabilities start from.

### Random parameters

- S3 SHALL admit a distribution on an operation parameter as a finite list
  of (value, weight) pairs. Each weight SHALL be a positive exact rational;
  each value SHALL be of the parameter's type; the values SHALL be pairwise
  distinct.
- If a weight is not positive, S3 SHALL refuse the distribution
  [`invalid-weight`], naming the parameter.
- If a value is not of the parameter's type, S3 SHALL refuse the
  distribution [`value-outside-type`], naming the parameter.
- If a value is listed twice, S3 SHALL refuse the distribution
  [`repeated-support-value`], naming the parameter.
- If an operation parameter has a second distribution, S3 SHALL refuse the
  second [`duplicate-distribution`], naming the parameter.
- S3 SHALL record the probability of each value as its weight over the sum
  of the weights, as an exact rational in lowest terms.
- If an operation's precondition reads a random parameter, S3 SHALL refuse
  it [`random-read-in-precondition`], naming the parameter and the
  precondition.
- S3 SHALL let postconditions and rewards read random parameters.

### Workloads

- S3 SHALL admit a workload declaration on a model that gives each operation
  of the model a positive exact rational weight.
- If a workload omits an operation of its model, S3 SHALL refuse it
  [`incomplete-workload`], naming the workload and the operation.
- If a workload names an operation that is not in its model, S3 SHALL
  refuse it [`unknown-operation`], naming the workload and the operation.
- If a workload gives one operation two weights, S3 SHALL refuse it
  [`duplicate-weight`], naming the workload and the operation.
- If a workload gives an operation a weight that is not positive, S3 SHALL
  refuse it [`invalid-weight`], naming the workload and the operation.
- A workload SHALL be a named, checked node with an identity, so a claim can
  name it and the obligation identity can bind it (FR-186).
- If a package has a second workload of a name already declared on any of its
  models, then S3 SHALL refuse the second
  `conflicting_declaration`/`duplicate-declaration`, with `subject` the
  workload name and `related` both workload declarations, each with its label
  span, in source order.

Note. A workload name is a package-wide identifier (FR-186), so a same-named
workload on another model is a repeat too. No `invalid_probabilistic_model`
cause names a repeated workload, and the code and cause are those of QSpec
FR-272's `conflicting_declaration` row, as FR-246 uses them for a repeated
name.

### Rewards

- S3 SHALL admit a named reward on an operation whose value type is an
  integer type or a quantity type (QSpec FR-142).
- If a reward's expression reads a state field, an ambient value or
  anything other than the operation's parameters and constants, S3 SHALL
  refuse the reward [`state-read-in-reward`], naming the reward and the read.
- If a reward's value type is neither an integer nor a quantity type, S3
  SHALL refuse the reward [`reward-type`].
- If an operation declares two rewards of one name, S3 SHALL refuse the
  second [`duplicate-reward`] (QSL-proposed, STD-188), naming the operation and the reward.
- S3 SHALL leave the non-negativity check to model admission, which runs
  it over the parameters' finite domains (FR-187).

### Quantity-typed parameters

A quantity-typed operation parameter, such as ADR-024 §7.2's duration `d`, has
the type `Quantity<u>` of a declared unit `u`. The domain package names the
unit with a unit `typeRef` (`ix://quire/unit/<name>`, QSpec FR-322 "Model-owned
members" step 4 and FR-322-AC-80), which resolves to the unit's own type node
(QSpec FR-142). A distribution's support values are quantity literals of that
unit (`1 ms`, `3 ms`, `12 ms` for `Quantity<ms>`). S3 SHALL refuse a support
value that is not a `Quantity<u>` value, another unit of the same dimension
included, with `value-outside-type` (Random parameters), the cause QSpec
FR-405-AC-3 assigns to a support, not FR-142's `type-mismatch`.

A parameter whose declared type is an integer range, such as
`Millis = Int[1, 12]`, takes integer supports.

Dependencies: the unit `typeRef` and the quantity literal form are QSpec's
(FR-142, FR-322); reading the unit `typeRef` is an intake rule (FR-056-AC-42), and
resolving it is the assembler's (FR-091).

### Package emission

If a checked package holds a random parameter, a workload, a reward or a
checked `probabilistic` claim (FR-186), then
S4 SHALL refuse the package `unsupported_construct`/`not-yet-implemented`,
naming the first such declaration in source order, its feature name
(`random-parameter`, `workload`, `reward` or `probabilistic-claim`) and the selected profile
identity (QSpec FR-272). S3's checked record is unaffected, and FR-187 reads
it from the checked package, not from the emitted wire.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-185-AC-1 | ADR-024 §7.2's `Service` model checks. `Server::attempt` records `d` with support `1`, `3`, `12` (`d` typed `Millis = Int[1, 12]`) and probabilities `9/10`, `7/100`, `3/100`, and `outcome` with `Ok` at `49/50` and `Fail` at `1/50`; the reward `duration` has value type `Millis`; the workload `Steady` records weight 1 for each of the three operations. | Test (TC-620) |
| FR-185-AC-2 | Each of these random-parameter variants of `Service` is refused at S3 with a diagnostic naming the parameter and carrying the cause: a weight `0` and a weight `rational(-1, 1)` (`invalid-weight`); the value `1` listed twice (`repeated-support-value`); the value `true` on the `Millis` parameter (`value-outside-type`); a precondition `pre self.phase = Busy and d < 5` reading `d` (`random-read-in-precondition`); a second `random Server::attempt(d) ~ …` (`duplicate-distribution`). A bare weight `-1` is not an `exact-value`, so S1 refuses it `invalid_syntax` and S3 never sees it. Exact-value bounds, each as the weight of `Server::request`: `decimal(1, 4294967295)` and `rational(n, n)` with `n` of 70,000 bits are each refused with a limit refusal (`LimitExceeded`) pending QSL-883 (the second although it reduces to `1`); `rational(m, 1)` with `m` of exactly 65,536 bits and `decimal(1, 19728)` (65,535 bits) are admitted; `rational(m, 1)` with `m` of exactly 65,537 bits and `decimal(1, 19729)` (65,539 bits) are refused with the limit refusal; `decimal(25, 3)` is admitted as `1/40`; `decimal(5, 0)` is admitted with a denominator of 1 bit. | Test (TC-620) |
| FR-185-AC-3 | Each of these workload variants is refused at S3 naming the workload and the operation and carrying the cause: `Steady` without `Server::reset` (`incomplete-workload`); a weight for an operation not in `Service` (`unknown-operation`); two weights for `Server::request` (`duplicate-weight`); a weight `0` for `Server::request` (`invalid-weight`). | Test (TC-620) |
| FR-185-AC-4 | Each of these reward variants is refused at S3 naming the reward and carrying the cause: `reward duration on Server::attempt = self.attempts` (`state-read-in-reward`); `reward ok on Server::attempt = (outcome = Ok)` (`reward-type`); a second `reward duration on Server::attempt = …` (`duplicate-reward`). `reward cost on Server::request = 2` checks. | Test (TC-620) |
| FR-185-AC-5 | A second `workload Steady on Service` and a `workload Steady on Other` (a second model of the package) are each refused `conflicting_declaration`/`duplicate-declaration` with `subject` `Steady` and `related` both declarations in source order, and the first remains the package's `Steady`. | Test (TC-620) |
| FR-185-AC-6 | The distribution `{1: rational(1, 3), 3: rational(1, 2^65534 + 1)}` (running-sum denominator of 65,536 bits) checks; `{1: rational(1, 3), 3: rational(1, 2^65535 + 5)}` (65,537 bits) is refused with the limit refusal naming the parameter; a check whose cancellation handle is tripped before it starts ends cancelled (FR-276), the handle reporting requested and the reported limit not being the 65,536-bit ceiling. | Test (TC-620) |
| FR-185-AC-7 | `Service` (with `d` typed `Millis = Int[1, 12]`) checks at S3 and refuses at S4 with `unsupported_construct`/`not-yet-implemented` naming the first random parameter in source order, `random-parameter` and the selected profile, with the checked record intact. | Test (TC-620) |
| FR-185-AC-8 | With the dimension `Time`, the unit `s` (root) and the unit `ms` (target `s`, scale `1/1000`), `Server::attempt` with `d: Quantity<ms>` records `d` with the supports `1 ms`, `3 ms` and `12 ms` and the probabilities of AC-1; the values `1 s` and `true` listed for `d` are each refused `value-outside-type` naming the parameter; a precondition reading `d` as `d < 5 ms` is refused `random-read-in-precondition`. | Test (TC-620) |

## Dependencies

- ADR-024 PM-1, PM-2, PM-5, QS-1 to QS-3.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md): random
  parameters become support domains of its enumeration (FR-187).

## References

- QSpec half of the work, which owns the grammar and semantics of random
  parameters, workloads and rewards: QSpec FR-405 (Linear STD-137).
- Owning ticket: Linear QSL-371.

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
  - target: ix://agent-ix/quire-spec-language/FR-302
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

A modeller writes `random Server::attempt(d) ~ { 1 ms: 90, 3 ms: 7, 12 ms:
3 };`, a `workload Steady on Service` with one weight per operation and
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
- S3 SHALL bound every sum it derives from written weights by the same
  rational-bits ceiling: a distribution's total of weights (which gives each
  probability) and a workload's total of weights. After each addition S3
  SHALL measure the running sum as the larger bit length of its reduced
  numerator and denominator, and if that exceeds the ceiling S3 SHALL refuse
  the declaration with the same limit refusal (`LimitExceeded`) before adding
  the next weight or dividing any weight by the sum, naming the declaration;
  a sum exactly at the ceiling is admitted. S3 SHALL poll the checker's
  cancellation before each addition. Each probability is the quotient of two
  values within the ceiling, so it needs no further check. A reward is
  computed from no sum at S3 and needs no such bound.

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
- If a model has a second workload of one name, S3 SHALL refuse the second
  `conflicting_declaration`/`duplicate-declaration` (QSL FR-302; QSpec
  FR-272), with `subject` the workload name and `related` both workload
  declarations, each with its label span, in source order, and a workload of
  that name on another model is a different workload. This is not an
  `invalid_probabilistic_model` cause: no cause of that code names a repeated
  workload, and the repeated-name rule is the one FR-302 states for
  declarations that must be unique.

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

### Operation parameters of quantity type

- An operation parameter of quantity type, such as `d: Quantity<Time>` in
  ADR-024 §7.2, comes from the model's domain package (the model operation's
  declared parameter type), not from a type reference in QSL source:
  complete-V1 source has no quantity type reference (FR-091-OQ-11(d), STD-113),
  and this requirement adds none. S3 SHALL type a quantity literal such as
  `1 ms` as a quantity of the dimension its unit names (FR-230), and SHALL
  check a random parameter's support values and a reward's expression against
  the parameter's package type, refusing `value-outside-type` for a value of
  another type. A unit declared in source stays usable by name outside a type
  form only.

### Package emission

- Until QSpec FR-322 defines a node form for a random parameter, a workload
  and a reward, S4 SHALL refuse a package holding any of them with
  `unsupported_construct`/`not-yet-implemented`, naming the declaration, as it
  refuses any other form it does not yet emit; S3's checked record
  (above) is unaffected, and FR-187 reads it from the checked package, not
  from the emitted wire. FR-322 has no `node_tag`/`semantic_form` for these
  declarations today (its `model` forms list none of them).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-185-AC-1 | ADR-024 §7.2's `Service` model checks. `Server::attempt` records `d` with support `1 ms`, `3 ms`, `12 ms` and probabilities `9/10`, `7/100`, `3/100`, and `outcome` with `Ok` at `49/50` and `Fail` at `1/50`; the reward `duration` has value type `Quantity<Time>`; the workload `Steady` records weight 1 for each of the three operations. | Test (TC-620) |
| FR-185-AC-2 | Each of these random-parameter variants of `Service` is refused at S3 with a diagnostic naming the parameter and carrying the cause: a weight `0` and a weight `rational(-1, 1)` (`invalid-weight`); the value `1 ms` listed twice (`repeated-support-value`); the value `true` on a `Quantity<Time>` parameter (`value-outside-type`); a precondition `pre self.phase = Busy and d < 5 ms` reading `d` (`random-read-in-precondition`); a second `random Server::attempt(d) ~ …` (`duplicate-distribution`). A bare weight `-1` is not an `exact-value`, so S1 refuses it `invalid_syntax` and S3 never sees it. Exact-value bounds, each as the weight of `Server::request`: `decimal(1, 4294967295)` and `rational(n, n)` with `n` of 70,000 bits are each refused with a limit refusal (`LimitExceeded`) pending QSL-883 (the second although it reduces to `1`); `rational(m, 1)` with `m` of exactly 65,536 bits and `decimal(1, 19728)` (65,535 bits) are admitted; `rational(m, 1)` with `m` of exactly 65,537 bits and `decimal(1, 19729)` (65,539 bits) are refused with the limit refusal; `decimal(25, 3)` is admitted as `1/40`; `decimal(5, 0)` is admitted with a denominator of 1 bit. | Test (TC-620) |
| FR-185-AC-3 | Each of these workload variants is refused at S3 naming the workload and the operation and carrying the cause: `Steady` without `Server::reset` (`incomplete-workload`); a weight for an operation not in `Service` (`unknown-operation`); two weights for `Server::request` (`duplicate-weight`); a weight `0` for `Server::request` (`invalid-weight`). | Test (TC-620) |
| FR-185-AC-4 | Each of these reward variants is refused at S3 naming the reward and carrying the cause: `reward duration on Server::attempt = self.attempts` (`state-read-in-reward`); `reward ok on Server::attempt = (outcome = Ok)` (`reward-type`); a second `reward duration on Server::attempt = …` (`duplicate-reward`). `reward cost on Server::request = 2` checks. | Test (TC-620) |
| FR-185-AC-5 | A second `workload Steady on Service` is refused `conflicting_declaration`/`duplicate-declaration` with `subject` `Steady` and `related` both declarations in source order, and the first remains the model's `Steady`; a `Steady` on another model checks. | Test (TC-620) |
| FR-185-AC-6 | A distribution of coprime weights `rational(1, d_i)`, each `d_i` within the ceiling, whose running sum exceeds 65,536 bits is refused with the limit refusal (`LimitExceeded`) naming the parameter before any probability is computed and without exceeding the cancellation poll; the same total at exactly 65,536 bits is admitted; a workload whose weights sum past the ceiling is refused the same way. | Test (TC-620) |
| FR-185-AC-7 | `Service` with its random parameters, workload and rewards checks at S3 and refuses at S4 with `unsupported_construct`/`not-yet-implemented` naming the first declaration in source order, with the checked record intact. | Test (TC-620) |
| FR-185-AC-8 | `Server::attempt(d: Quantity<Time>, …)` with `d` typed by the domain package checks with `1 ms`, `3 ms` and `12 ms` as `Quantity<Time>` values; `true` for `d` is refused `value-outside-type`; no QSL source declares the parameter's type. | Test (TC-620) |

## Dependencies

- ADR-024 PM-1, PM-2, PM-5, QS-1 to QS-3.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md): random
  parameters become support domains of its enumeration (FR-187).

## References

- QSpec half of the work, which owns the grammar and semantics of random
  parameters, workloads and rewards: QSpec FR-405 (Linear STD-137).
- Owning ticket: Linear QSL-371.

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

Every refusal below is an `invalid_probabilistic_model` diagnostic whose
cause is the string in brackets, and it retains the declaration locus and
the parameter, reward, workload or operation it names. Every cause except
`duplicate-reward` is in QSpec FR-272's closed set for that code (QSpec
FR-405). `duplicate-reward` is a QSL-proposed cause that QSpec does not yet
define; it is requested of QSpec in STD-188, and QSL emits it until QSpec
adds it or names another.

A weight is an `exact-value`, and `exact-number` admits a signed integer, so
`rational(-1, 1)` parses and S3 refuses it `invalid-weight`. A bare `-1` is
not an `exact-value`, so S1 refuses it `invalid_syntax`.

### Exact values

- S2 holds each weight as written (FR-393) with unbounded integers. When
  S3 reduces a weight to lowest terms and the reduced numerator or
  denominator would exceed the check's `max_exact_value_bits` ceiling
  (default 4096), S3 SHALL refuse the declaration `resource_exhausted` with
  cause `exact-value-bits-exceeded`, naming the ceiling and the declaration,
  before computing the value. The same ceiling and refusal apply to every
  `exact-value` that FR-186 reads.

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

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-185-AC-1 | ADR-024 §7.2's `Service` model checks. `Server::attempt` records `d` with support `1 ms`, `3 ms`, `12 ms` and probabilities `9/10`, `7/100`, `3/100`, and `outcome` with `Ok` at `49/50` and `Fail` at `1/50`; the reward `duration` has value type `Quantity<Time>`; the workload `Steady` records weight 1 for each of the three operations. | Test (TC-620) |
| FR-185-AC-2 | Each of these random-parameter variants of `Service` is refused at S3 with a diagnostic naming the parameter and carrying the cause: a weight `0` and a weight `rational(-1, 1)` (`invalid-weight`); a weight `decimal(1, 4294967295)` (`resource_exhausted`/`exact-value-bits-exceeded`); the value `1 ms` listed twice (`repeated-support-value`); the value `true` on a `Quantity<Time>` parameter (`value-outside-type`); a precondition `pre self.phase = Busy and d < 5 ms` reading `d` (`random-read-in-precondition`); a second `random Server::attempt(d) ~ …` (`duplicate-distribution`). A bare weight `-1` is not an `exact-value`, so S1 refuses it `invalid_syntax` and S3 never sees it. | Test (TC-620) |
| FR-185-AC-3 | Each of these workload variants is refused at S3 naming the workload and the operation and carrying the cause: `Steady` without `Server::reset` (`incomplete-workload`); a weight for an operation not in `Service` (`unknown-operation`); two weights for `Server::request` (`duplicate-weight`); a weight `0` for `Server::request` (`invalid-weight`). | Test (TC-620) |
| FR-185-AC-4 | Each of these reward variants is refused at S3 naming the reward and carrying the cause: `reward duration on Server::attempt = self.attempts` (`state-read-in-reward`); `reward ok on Server::attempt = (outcome = Ok)` (`reward-type`); a second `reward duration on Server::attempt = …` (`duplicate-reward`). `reward cost on Server::request = 2` checks. | Test (TC-620) |

## Dependencies

- ADR-024 PM-1, PM-2, PM-5, QS-1 to QS-3.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md): random
  parameters become support domains of its enumeration (FR-187).

## References

- QSpec half of the work, which owns the grammar and semantics of random
  parameters, workloads and rewards: QSpec FR-405 (Linear STD-137).
- Owning ticket: Linear QSL-371.

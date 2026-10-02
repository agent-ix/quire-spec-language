---
id: FR-207
title: "Run compensation templates as protocol threads"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-057
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-058
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-427
    type: depends_on
---
# FR-207: Run compensation templates as protocol threads

## Description

The protocol system SHALL run compensation templates as QSpec FR-427
states: registration on the forward effect, activation on the first
eligible trigger, compensation attempts on a compensation thread under the
authored attempt count and retry relation, closure at the commit and
`cend` with its recovery status (ADR-027 CP-1 to CP-5, ST-11, ST-12),
folding registration and activation into the step that causes them
(ADR-027 FO-1).

## Use case

A verification operator models a payment that is refunded when a later
failure event occurs. They need the refund to be explored in every
interleaving where the failure happens, to run no more times than the
template allows, to stop being possible once the payment commits, and to
report whether the account recovered, so that a claim about recovery is
checked over every behaviour.

## Inputs

- A protocol state (FR-205) and the checked compensation templates of the
  protocol clause (FR-218): forward effect node, `as` binder, registration
  and activation captures, trigger type and activation guard, operation and
  role, attempt type, `attempts n`, retry relation, `commit` node or `commit
  never`, and recovery predicate.

## Outputs

- Compensation registrations in the state key's `control` member with
  status `registered`, `activated`, `recovered` or `unrecovered` and their
  captures; compensation threads with their attempt counts in
  `bounds`; `cattempt(c, r)` and `cend(c, r)` steps.

## Behavior

- The protocol system SHALL apply QSpec FR-427's registration, activation,
  `cattempt`, `cend` and commit rules, keying each registration by
  (template, the effect's thread path, ordinal).
- The protocol system SHALL apply each `cattempt` through FR-206's
  `attempt` rule for the template's operation by its role, with the attempt
  type as the record.
- The protocol system SHALL keep `cend` enabled whether or not a retry is.
- The protocol system SHALL mark a protocol instance finished in its
  `finish` step, which FR-206 enables only after every other thread of the
  instance, compensation threads included, has left `running`, and SHALL
  remove the instance with its registrations in that step (FR-209).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-207-AC-1 | ADR-027 §7.1 (`Pay`, `x.bal = 0`): the reachable states are exactly t0 to t8 with the enabled steps of its table, nine states and eight transitions. `event(Paid)` registers `Refund` in the same step; `event(Fail)` with `failed` true activates it in the same step and creates the compensation thread at count 0; with `failed` false the registration stays `registered` until `finish`, which removes the instance with its registration, so t5 holds no instance and no registration. | Test (TC-652) |
| FR-207-AC-2 | In §7.1, `finish` is not enabled at t4 while the compensation thread is `running`. At t6 the count has reached `attempts 1`, so `cend` is the only step, and the registration is `recovered` at t7 because `bal = 0`. With the refund's postcondition changed to leave `bal = 1`, the registration is `unrecovered` after `cend`. | Test (TC-652) |
| FR-207-AC-3 | With `attempts 2` and the retry relation `true`, both `cattempt(Refund)` and `cend(Refund)` are enabled at the retry point after the first attempt; after the second attempt only `cend` is. With the retry relation `false`, only `cend` is enabled after the first attempt. | Test (TC-652) |
| FR-207-AC-4 | A variant of §7.1 with a `commit Ok` node between `Paid` and `Fail` and the template's `commit Main::Ok`: the registration closes at `Ok`'s step, `event(Fail)` with `failed` true activates nothing, and no state holds a compensation thread. A second effect of the template's forward node after `Ok` registers nothing. | Test (TC-652) |

## Dependencies

- ADR-027 §2.2 ST-11 and ST-12, §2.2a CP-1 to CP-5, §2.1 FO-1, §3.1 PB-4.
- FR-205 (registrations in the key), FR-206 (the `attempt` rule), FR-218
  (checked templates).
- QSpec owns compensation's meaning in a model subject (ADR-027 QS-10):
  QSpec FR-056, FR-057, FR-058.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-427 (Linear STD-140).

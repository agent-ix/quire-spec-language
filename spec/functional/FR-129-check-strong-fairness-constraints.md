---
id: FR-129
title: "Check strong fairness constraints and the unmarked fairness kind"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-016
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-019
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
---
# FR-129: Check strong fairness constraints and the unmarked fairness kind

## Description

The S3 TemporalTrace `check` SHALL check a fairness constraint of kind
`strong` beside `weak` on an infinite-trace clause, with the same
granularities and the same unmarked granularity (ADR-019 SY-1 to SY-5).
The checker SHALL check a constraint written with no kind as `weak`
(ADR-019 SY-2), and the checker SHALL put each constraint's resolved kind
into the clause's checked identity (ADR-019 SV-5). It extends FR-123's fairness check.

## Use case

A verification operator models a mutex with two processes and states that
`acquire` is strongly fair for each receiver and argument vector, and that
`release` is weakly fair. They write a constraint with no kind where weak
fairness is enough. The checker gives every constraint its resolved kind and
granularity, so the claim's identity and its counterexample show exactly
which premise was checked.

## Semantic authority and boundary

QSpec owns the surface grammar of the `strong` kind and the unmarked kind,
and the meaning of strong fairness (ADR-019 QS-1, QS-2; References). This
requirement specifies what S3 checks once the shared grammar has parsed a
constraint form. Spellings in the acceptance criteria follow ADR-019 §1 and
are illustrative.

## Inputs

- Parsed fairness constraint forms (FR-123), each with an optional kind, an
  operation name and an optional granularity.
- The unit's resolved temporal profile selection (FR-110).

## Outputs

- `FairnessKind::{Weak, Strong}` on each checked `FairnessConstraint`
  (FR-123).
- The checked fairness set, in which every constraint holds its resolved
  kind and granularity.

## Behavior

- When a constraint form names no kind, the checker SHALL check it as
  `FairnessKind::Weak`.
- When a constraint form names `strong`, the checker SHALL check it as
  `FairnessKind::Strong`; when it names `weak`, as `FairnessKind::Weak`.
- The checker SHALL check the granularity of a strong constraint by
  FR-123's rule, so a strong constraint with no granularity checks as
  `Whole`.
- If a strong constraint sits on a clause whose unit selects a bounded
  profile, then the checker SHALL refuse it as FR-123 refuses a weak one.
- The checker SHALL keep weak and strong constraints on the same operation
  and granularity as two members of the fairness set (ADR-019 SY-4).
- The checker SHALL check two constraints with equal resolved kind,
  operation and granularity as one, whether or not the kind was written.
- The checker SHALL include each constraint's resolved kind in the clause's
  checked identity, so a clause under a strong constraint and the same
  clause under the weak constraint with the same operation and granularity
  have different node identities, and a constraint written with no kind has
  the identity of the same constraint written `weak`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-129-AC-1 | Over ADR-019 §6's mutex unit, a constraint on `acquire` with no kind and no granularity checks to `{Weak, acquire, Whole}`, and the clause holding it has the node identity of the same clause with `weak whole` written. A `strong` constraint with no granularity checks to `{Strong, acquire, Whole}`, and `strong each` to `{Strong, acquire, Each}`. The `strong each` clause and the `weak each` clause have different node identities. | Test (TC-530) |
| FR-129-AC-2 | A clause carrying `strong each` on `acquire` and `weak each` on `acquire` checks to a two-member fairness set in source order. A `strong` constraint written twice checks to one member. A `strong` constraint on a clause under `quire.temporal.event-position.false-extension/v1` refuses `unsupported_construct`/`expression-form` at the constraint's span. | Test (TC-530) |

## Dependencies

- ADR-019 §1 SY-1 to SY-5, §5 SV-5; ADR-018 §4 FA-1, FA-5, FA-6.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (the fairness check it extends),
  [FR-110](FR-110-resolve-header-profile-selections-at-e3.md) (profile
  selection).

## References

- QSpec half of ADR-019, carrying ADR-019 QS-1 to QS-8: Linear STD-132.

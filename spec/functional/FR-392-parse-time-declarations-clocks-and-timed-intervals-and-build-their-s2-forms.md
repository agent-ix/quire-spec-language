---
id: FR-392
title: "Parse time declarations, Clock fields, quantity literals and timed intervals, and build their S2 forms"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-311
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
---
# FR-392: Parse time declarations, Clock fields, quantity literals and timed intervals, and build their S2 forms

## Description

When a `1-draft` unit selecting the timed facet holds a `time-decl`, a
`time-invariant`, an `urgent-decl`, a field of type `Clock`, a
`quantity-literal` expression or a temporal operator with a `timed-interval`,
S1 (`qsl-cst`) SHALL parse it with the productions of QSpec's shared grammar
(§ "Probabilistic, timed and real-time forms"), and S2 (`qsl-forms`) SHALL
build one form for it, so that S3 can check it (FR-230, FR-233, FR-234). QSpec
owns the productions and their meaning; this requirement specifies the forms
QSL builds. Parsing selects no meaning: whether a time member, a clock read or
an interval is admitted is the S3 check's decision.

## Use case

A modeller writes `time Rpc dense unit ms;`, a clock field `x: Clock`, a
guard `x >= 1 ms`, `time invariant inflight when self.inflight { self.x <= 3
ms }`, `urgent timeout;` and the claim `eventually[0 ms, 3 ms) holds(...)`.
S2 builds one form per construct with every span S3 needs for a diagnostic.

## Inputs

- A recovery-free `LosslessCst` of a `1-draft` unit (ADR-011 E2).

## Outputs

`DeclarationForm` gains three variants, and the expression and type builders
two more constructs:

```rust
pub struct TimeSourceForm {             // time-decl
    pub model: Spanned<Identifier>,
    pub source: TimeSourceSyntax,       // Dense | Tick{operation, period}
    pub unit: Spanned<QualifiedName>,
}
pub enum TimeSourceSyntax {
    Dense,
    Tick { operation: Spanned<QualifiedName>, period: QuantityForm },
}
pub struct TimeInvariantForm {          // time-invariant
    pub name: Spanned<Identifier>,
    pub guard: Expression,              // the `when` block
    pub bounds: Vec<ClockBoundForm>,    // clock-bound, declared order
}
pub struct ClockBoundForm {
    pub clock: Expression,              // the postfix operand, as written
    pub strict: bool,                   // `<` (true) or `<=` (false)
    pub bound: QuantityForm,
    pub span: Span,
}
pub struct UrgentForm {                 // urgent-decl
    pub name: Spanned<Identifier>,
    pub target: UrgentTarget,           // When(Expression) | Operation(Spanned<QualifiedName>)
}
pub struct QuantityForm {               // quantity-literal
    pub magnitude: Spanned<ExactValue>, // uint, decimal-literal or rational, exact
    pub unit: Spanned<QualifiedName>,
}
pub struct TimedIntervalForm {          // timed-interval
    pub lower: QuantityForm, pub lower_closed: bool,
    pub upper: QuantityForm, pub upper_closed: bool,
    pub span: Span,
}
```

`Expression` gains a `Quantity(QuantityForm)` variant, the type builder maps
`Clock` to a clock type form, and `TemporalOperatorForm` (FR-325) gains
`timed_interval: Option<TimedIntervalForm>`, which is `Some` only when
`interval` is `None`.

## Behavior

- `LeadingTokenKind` SHALL gain `Time` and `Urgent`. A declaration beginning
  `time` followed by `invariant` SHALL dispatch to `time_invariant`, and any
  other `time` declaration to `time_decl`; `urgent` SHALL dispatch to
  `urgent_decl`. Each selects exactly one production.
- S1 SHALL recognise a temporal operator's `(` followed by a
  `quantity-literal` as the opener of a `timed-interval`, and a `(`
  followed by anything else as a parenthesized operand (the shared grammar's
  second-token rule). `[` followed by a `quantity-literal` is a
  `timed-interval`; `[` followed by a `uint` and `,` and a `uint` or `*` is
  FR-325's `interval`.
- S2 SHALL keep `decimal-literal` and `rational` magnitudes exactly (`0.0005`
  is `1/2000`) and SHALL record each interval end's openness from its
  bracket.
- S2 SHALL leave unchecked for S3: time-source uniqueness, unit
  dimension, clock typing, the negativity of a constant, interval ordering
  and punctuality, and which profile admits a timed interval (FR-230,
  FR-234).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-392-AC-1 | ADR-026 §11's `Rpc` unit, written in the shared grammar's productions, builds one `TimeSourceForm` (`Dense`, unit `ms`), one `TimeInvariantForm` per `time invariant` with its `when` expression and clock bounds in declared order, one `UrgentForm` for `urgent timeout;`, and a `Clock`-typed field; every form carries the span of its source text. A `time tick send period 1/2 ms unit ms;` builds `Tick{send, 1/2 ms}`. | Test (TC-1077) |
| FR-392-AC-2 | A unit whose declaration begins `time` then `invariant`, `time` then anything else, or `urgent` dispatches to `time_invariant`, `time_decl` or `urgent_decl` respectively, and to no other production. | Test (TC-1077) |
| FR-392-AC-3 | `eventually[0 ms, 3 ms) holds(p)`, `once(1 ms, 2 ms] holds(p)` and `always (holds(p))` parse; the first two hold `TimedIntervalForm` values with `(lower_closed, upper_closed)` of `(true, false)` and `(false, true)`, magnitudes `0`, `3`, `1`, `2` and unit `ms`, and `interval: None`; the third holds neither interval. `eventually[0, 3] holds(p)` still holds FR-325's `interval` and `timed_interval: None`. | Test (TC-1077) |
| FR-392-AC-4 | `eventually[0 s, 0.003 s]` builds an upper magnitude of `3/1000`; `(2 ms, 2 ms]`, `[3 ms, 2 ms]` and a second `time` declaration all build at S2 and are refused only at S3 (FR-234-AC-2, FR-230-AC-2). `eventually[0 ms, x] holds(p)` fails at S1 with a parse diagnostic at `x`'s span and builds no form. | Test (TC-1077) |

## Dependencies

- ADR-026 CK-1 to CK-9, §6; ADR-012 §3, §4.3.
- [FR-067](FR-067-add-s2-forms-and-retire-seam-5.md) (the S2 forms core),
  [FR-311](FR-311-build-random-reward-and-workload-forms-at-s2.md) (the
  declaration-form precedent and `ExactValue`),
  [FR-325](FR-325-parse-temporal-operators-with-an-optional-interval.md) (the
  operator form this extends),
  [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md) and
  [FR-234](FR-234-check-timed-intervals-and-classify-timed-property-forms.md)
  (the S3 checks of these forms).

## References

- QSpec half: the shared grammar's `time-decl`, `time-invariant`,
  `clock-bound`, `urgent-decl`, `quantity-literal` and `timed-interval`
  productions, and QSpec FR-142.
- Owning ticket: Linear QSL-580.

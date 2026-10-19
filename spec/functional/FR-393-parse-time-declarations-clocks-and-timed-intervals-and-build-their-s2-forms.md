---
id: FR-393
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
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-233
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
---
# FR-393: Parse time declarations, Clock fields, quantity literals and timed intervals, and build their S2 forms

## Description

When a `1-draft` unit holds a `time-decl`, a `time-invariant`, an
`urgent-decl`, a field of type `Clock`, a `quantity-literal` expression or a
temporal operator with a `timed-interval`, S1 (`qsl-cst`) SHALL parse it with
the productions of QSpec's shared grammar (§ "Probabilistic, timed and
real-time forms"), and S2 (`qsl-forms`) SHALL build one form for it, so that
S3 can check it (FR-230, FR-233, FR-234). QSpec owns the productions and
their meaning; this requirement specifies the forms QSL builds. Parsing
selects no meaning: whether a time member, a clock read or an interval is
admitted is the S3 check's decision. S2 builds these forms whether or not the
unit selects any facet. Whether a unit admits a `time-decl`, a `time-invariant`, an `urgent-decl` or the `Clock` type is decided after S2, by S3.

This requirement amends FR-325: a temporal operator's interval is one member
that holds either a step interval or a timed interval.

## Use case

A modeller writes `time Rpc dense unit ms;`, a clock field `x: Clock`, a guard
`self.x >= 1 ms`, `time invariant inflight when { self.inflight } { self.x <=
3 ms }`, `urgent hurry Call::timeout;` and the claim `eventually[0 ms, 3 ms)
holds(k.done)`. S2 builds one form per construct with every span S3 needs for
a diagnostic.

## Inputs

- A recovery-free `LosslessCst` of a `1-draft` unit (ADR-011 E2).

## Outputs

`DeclarationForm` gains three variants, and the expression and type builders
two more constructs. Every form below carries a whole-form `span`, the span of
its source text; this is the span the tests read.

```rust
pub struct TimeSourceForm {             // time-decl
    pub model: Spanned<Identifier>,
    pub source: TimeSourceSyntax,
    pub unit: Spanned<QualifiedName>,
    pub span: Span,
}
pub enum TimeSourceSyntax {
    Dense,
    Tick { operation: Spanned<QualifiedName>, period: QuantityForm },
}
pub struct TimeInvariantForm {          // time-invariant
    pub name: Spanned<Identifier>,
    pub guard: Expression,              // the `when` block
    pub bounds: Vec<ClockBoundForm>,    // clock-bound, declared order
    pub span: Span,
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
    pub span: Span,
}
pub struct QuantityForm {               // quantity-literal
    pub magnitude: Spanned<ExactValue>,
    pub unit: Spanned<QualifiedName>,
    pub span: Span,
}
pub struct TimedIntervalForm {          // timed-interval
    pub lower: QuantityForm, pub lower_closed: bool,
    pub upper: QuantityForm, pub upper_closed: bool,
    pub span: Span,
}
pub enum OperatorInterval {
    Steps(IntervalForm),                // FR-325's [a,b] and [a,*]
    Timed(TimedIntervalForm),
}
```

`ExactValue` is the `exact-value` that a `uint`, a `decimal-literal` or an
`exact-number` spells, held as written: a spelling kind (`uint`,
`decimal-literal`, `rational(n, d)` or `decimal(c, s)`) and two integers whose
meaning the kind fixes (the integer and 1; the digits without the point and
the count of fraction digits; `n` and `d`; the coefficient and the scale).
S2 neither reduces it nor raises ten to a scale, and refuses no scale, so a
scale of any size costs nothing at S2 and the form keeps what was written
(`decimal(1, 4294967295)` builds). S3 bounds the value as written (FR-185). S3 reduces it to a
numerator and denominator in lowest terms, after bounding the value as
written by `s3.rational_bits` and refusing one past it with the limit
outcome FR-185 states.
FR-311 uses the same type.

`Expression` gains a `Quantity(QuantityForm)` variant. The type builder maps
the type reference `Clock` to `TypeForm::Clock{span}`. `TemporalOperatorForm`
(FR-325) holds `interval: Option<OperatorInterval>`, replacing its former
`Option<IntervalForm>`; it has no second interval member.

## Behavior

- `time`, `urgent`, `dense`, `tick` and `period` SHALL be contextual words:
  `time` and `urgent` are keywords only at a declaration head, and `dense`,
  `tick` and `period` only inside a `time-decl`. Everywhere else they lex as
  identifiers, so a field, a parameter or a model named `time` or `period`
  keeps parsing.
- `Clock` SHALL be the built-in type only as an unqualified `type-ref`. A
  qualified reference such as `M::Clock` is a named type, and an unqualified
  `Clock` in a type position is always the built-in, whatever a unit
  declares. S1 SHALL refuse a unit declaration that would introduce an
  unqualified type named `Clock` with a parse diagnostic at the name.
- `LeadingTokenKind` SHALL gain `Time` and `Urgent`. A declaration beginning
  `time` followed by `invariant` SHALL dispatch to `time_invariant`, and any
  other `time` declaration to `time_decl`; `urgent` SHALL dispatch to
  `urgent_decl`. Each selects exactly one production.
- S2 SHALL build these forms under every facet selection, including none.
  It SHALL NOT refuse a form for its facet; whether the unit admits the form
  is decided after S2, by S3.
- S1 SHALL read a temporal operator's `(` followed by a `quantity-literal`
  as the opener of a `timed-interval`, and a `(` followed by anything else
  as a parenthesized operand (the shared grammar's second-token rule). A `[`
  whose first bound is a `quantity-literal` (a `uint`, a `decimal-literal` or
  a `rational`/`decimal` form, then a unit name) opens a `timed-interval`; a
  `[` whose first bound is a `uint` followed by `,` opens FR-325's step
  interval.
- A bracketed interval SHALL use one kind of bound at both ends. A timed
  interval whose upper end is a bare `uint` or `*`, and a step interval whose
  upper end is a `quantity-literal`, SHALL fail at S1 with a parse
  diagnostic located at the upper bound's span, and S2 SHALL build no form.
- S2 SHALL keep `decimal-literal` and `exact-number` magnitudes exactly as
  written (`0.0005` is held as digits `5` and `4` fraction digits, which S3
  reduces to `1/2000`) and SHALL record each interval end's openness from
  its bracket.
- S2 SHALL leave unchecked for S3: time-source uniqueness, unit dimension,
  clock typing, the negativity of a constant, interval ordering and
  punctuality, and which profile admits a timed interval (FR-230, FR-234).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-393-AC-1 | A unit with `time Rpc dense unit ms;`, `time invariant inflight when { self.inflight } { self.x <= 3 ms }` and `time invariant waiting when { self.phase = Waiting } { self.x <= 4 ms }` builds one `TimeSourceForm` (`Dense`, unit `ms`) and two `TimeInvariantForm`s in declared order, each with its `when` expression and one `ClockBoundForm` (`strict` false, bounds `3 ms` then `4 ms`); a field `x: Clock` builds `TypeForm::Clock` with the span of the word `Clock`. `urgent hurry Call::timeout;` builds `UrgentForm{name: hurry, target: Operation(Call::timeout)}` and `urgent stalled when { self.phase = Waiting };` builds a `When` target. `time Rpc tick Call::send period rational(1,2) ms unit ms;` builds `Tick{operation: Call::send, period: 1/2 ms}`. Each form carries its whole-form `span`. | Test (TC-1078) |
| FR-393-AC-2 | A unit whose declaration begins `time` then `invariant`, `time` then anything else, or `urgent` dispatches to `time_invariant`, `time_decl` or `urgent_decl` respectively, and to no other production. A unit with a field named `time` and a parameter named `period` parses and builds with those names as identifiers. | Test (TC-1078) |
| FR-393-AC-3 | `eventually[0 ms, 3 ms) holds(p)`, `once(1 ms, 2 ms] holds(p)` and `always (holds(p))` parse; the first two hold `Some(Timed(..))` with `(lower_closed, upper_closed)` of `(true, false)` and `(false, true)`, magnitudes `0`, `3`, `1`, `2` and unit `ms`; the third holds `None`. `eventually[0,3] holds(p)` holds `Some(Steps(IntervalForm{0, Finite(3)}))`. | Test (TC-1078) |
| FR-393-AC-4 | `eventually[0 s, 0.003 s] holds(p)` builds an upper magnitude of `3/1000`; `eventually(2 ms, 2 ms] holds(p)`, `eventually[3 ms, 2 ms] holds(p)` and a unit with a second `time` declaration all build at S2 and are refused only at S3 (FR-234-AC-2, FR-230-AC-2). `eventually[0 ms, x] holds(p)` fails at S1 with a parse diagnostic at `x`'s span and builds no form. | Test (TC-1078) |
| FR-393-AC-5 | `eventually[0, 3 ms] holds(p)`, `eventually[0 ms, 3] holds(p)` and `eventually[0 ms, *] holds(p)` fail at S1 with a parse diagnostic at `3 ms`, `3` and `*` respectively, and build no form. | Test (TC-1078) |
| FR-393-AC-6 | The `Rpc` unit of AC-1, once with no facet selected and once with the complete facets selected, builds equal forms under `PartialEq`; S2 reports no facet diagnostic for either. | Test (TC-1078) |
| FR-393-AC-7 | `x: Clock` builds `TypeForm::Clock`; `x: M::Clock` builds a named type form for the qualified name and no `TypeForm::Clock`; a unit that declares an unqualified type named `Clock` fails at S1 with a parse diagnostic at the declared name. | Test (TC-1078) |

## Dependencies

- ADR-026 CK-1 to CK-9, §6; ADR-012 §3, §4.3.
- [FR-067](FR-067-add-s2-forms-and-retire-seam-5.md) (the S2 forms core),
  [FR-311](FR-311-build-random-reward-and-workload-forms-at-s2.md) (the
  declaration-form precedent and `ExactValue`),
  [FR-325](FR-325-parse-temporal-operators-with-an-optional-interval.md) (the
  operator form this amends),
  [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md),
  [FR-233](FR-233-bind-a-model-claim-to-model-time-or-model-steps.md) and
  [FR-234](FR-234-check-timed-intervals-and-classify-timed-property-forms.md)
  (the S3 checks of these forms).

## References

- QSpec half: the shared grammar's `time-decl`, `time-invariant`,
  `clock-bound`, `urgent-decl`, `quantity-literal`, `exact-number` and
  `timed-interval` productions, and QSpec FR-142.
- Owning ticket: Linear QSL-580.

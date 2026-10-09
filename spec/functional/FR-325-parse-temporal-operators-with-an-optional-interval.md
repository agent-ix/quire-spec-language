---
id: FR-325
title: "Parse temporal operators with an optional interval into S2 TemporalTrace forms"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
---
# FR-325: Parse temporal operators with an optional interval into S2 TemporalTrace forms

## Description

S1 (`qsl-cst`) SHALL parse every unary temporal operator (`eventually`,
`always`, `once`, `historically`) and every binary temporal operator
(`until`, `release`, `since`, `triggered`) with an interval `[a,b]`, an
interval `[a,*]`, or no interval, under every profile (ADR-014 A-1). S2
(`qsl-forms`) SHALL build one TemporalTrace form per temporal clause, holding
each operator with an optional interval form, as the M-3b TemporalTrace forms
of ADR-011. Parsing selects no meaning: whether an interval is required,
admitted or refused is the S3 check's decision (FR-326, FR-123): `[a,*]`
is admitted under the infinite-trace profile and refused under the bounded
profiles (ADR-018 IV-1, QSpec FR-048-AC-10).

## Use case

An author writes `eventually holds(c.value = 3)` in a unit that selects the
infinite-trace profile and `eventually[0,5] holds(c.value = 3)` in one that
selects a bounded profile. Both parse to the same form shape, with and
without an interval, so the checker gives every author one diagnostic about
meaning, at the operator, instead of a parse error that depends on the
profile.

## Inputs

- A unit's source bytes (FR-001), with a `temporal` clause.

## Outputs

- A CST in which each temporal operator node has an optional interval child.
- An S2 `TemporalClauseForm` holding the clause name, its profile and model
  references, its `over` parameter, its activation, its fairness constraint
  forms (FR-123), its capture forms and a `TemporalFormulaForm` tree. Each
  fairness form is `FairnessConstraintForm{kind, granularity, operation,
  span}` with `kind: FairnessKind::{Weak, Strong}`, `granularity:
  Option<FairnessGranularity::{Whole, Each}>`, the operation's qualified
  name as written and the constraint's span. Each capture form is
  `CaptureForm{parameter, value, span}`: the capture's name and declared
  type as a parameter form, its value expression as written, and the
  capture's span. Each operator node is
  `TemporalOperatorForm{operator, interval: Option<IntervalForm>, operands}`
  with `IntervalForm{lower: u64, upper: IntervalUpper::{Finite(u64), Open}}`
  and the source span of the operator and of its interval.

## Behavior

- S2 SHALL build one fairness form per `fair` constraint, in source order
  (QSpec FR-362's `fairness` production). An unwritten kind SHALL build
  `Weak`, and an unwritten granularity SHALL build `None`, which S3 reads as
  `whole` (FR-123).
- S2 SHALL build one capture form per `capture`, in source order (the
  QSpec shared grammar's `capture` production, which FR-362's
  `temporal-clause` places after the fairness constraints). The capture's
  meaning, an immutable value fixed at activation, is QSpec FR-093's and
  S3's to check.
- When an operator is written with no interval, S2 SHALL build its form
  with `interval: None`.
- When an operator is written with `[a,b]`, S2 SHALL build `Some` with
  `upper: Finite(b)`; with `[a,*]`, `Some` with `upper: Open`. S2 SHALL
  build an interval with `a > b` as written, with no diagnostic; the S3
  check refuses an inverted interval.
- A bound that is not a decimal `u64` SHALL fail at S1 with a parse
  diagnostic located at the bound's span.
- Nesting SHALL be free: an operator with an interval and an operator
  without one may appear in any operand position of each other.
- S2 SHALL produce the same form for the same clause text under every
  profile selection.
- The TemporalTrace forms SHALL be the only S2 producer for temporal clauses
  (ADR-011 M-3b, FR-067).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-325-AC-1 | `always (holds(not s.healthy) implies eventually always[0,10] holds(s.healthy))` parses, and its form holds `interval: None` on the outer `always` and on `eventually`, and `Some{lower: 0, upper: Finite(10)}` on the inner `always`, each with its own span. | Test (TC-835) |
| FR-325-AC-2 | `holds(p) until holds(q)`, `holds(p) since[2,4] holds(q)` and `eventually[3,*] holds(p)` parse; their operator forms hold `None`, `Some{2, Finite(4)}` and `Some{3, Open}`. `eventually[5,3] holds(p)` parses to `Some{5, Finite(3)}` with no S1 or S2 diagnostic. | Test (TC-835) |
| FR-325-AC-3 | `eventually[0,x] holds(p)` fails at S1 with a parse diagnostic located at `x`'s span, and no S2 form is built. | Test (TC-835) |
| FR-325-AC-4 | Two units that differ only in the selected profile identity, one selecting the infinite-trace profile and one the event-position false-extension profile, padded so their headers have equal length and the clause starts at the same byte offset, build equal S2 forms for the clause, compared by `PartialEq` and by their `Debug` rendering. | Test (TC-835) |
| FR-325-AC-5 | A clause whose body holds `fair Config::ConfigVersion::attemptUpdate;`, `fair strong each Config::ConfigVersion::reset;` and `fair weak whole Config::ConfigVersion::tick;` before its formula builds three fairness forms in that order: (`Weak`, `None`, `Config::ConfigVersion::attemptUpdate`), (`Strong`, `Some(Each)`, `Config::ConfigVersion::reset`) and (`Weak`, `Some(Whole)`, `Config::ConfigVersion::tick`), each with the span of its own constraint. | Test (TC-835) |
| FR-325-AC-6 | A clause over `(p: Config::ConfigVersion)` whose body holds `fair Config::ConfigVersion::tick;`, then `capture before: Int[0, 1000] = p.value;` and `capture parent: Config::ConfigVersion = p.parent;`, then its formula, builds one fairness form and two capture forms in that order: `before` with type `Int[0, 1000]` and value `p.value`, and `parent` with type `Config::ConfigVersion` and value `p.parent`, each with the span of its own capture; its formula form equals, up to spans, the formula form of the same clause with the two captures removed. | Test (TC-835) |

## Dependencies

- ADR-014 §5 A-1 and §11; ADR-018 §11 IV-1 (operators
  that carry an interval); ADR-011 M-3b.
- [FR-067](FR-067-add-s2-forms-and-retire-seam-5.md) (the S2 forms stage).
- QSpec FR-090-AC-7 and FR-250-AC-6 (which profile admits the unbounded
  operator edition). QSpec owns the surface grammar, including the
  infinite-trace interval rule (ADR-018 QS-13).

## References

- Linear QSL-384 (this requirement's spec ticket); QSL-43 (implementation).
- QSpec FR-367 (interval operators under infinite-trace; Linear STD-131,
  ADR-018 QS-13) and Linear STD-99 (the `[a,*]`
  grammar note and the v2 temporal operation identities).

---
id: FR-310
title: "Build the S2 form of a refinement declaration"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: depends_on
---
# FR-310: Build the S2 form of a refinement declaration

## Description

The S2 value form builder SHALL build a `RefinementForm` for every
`refinement` declaration (ADR-020 §1 RM-1, RM-9; the surface spelling is
QSpec FR-375's), holding its header and every row as written with its span.
The form carries syntax only; S3 resolves names and checks it (FR-135 to
FR-139, FR-147).

## Use case

A specification author writes a refinement of the abstract counter by the
compare-and-set counter. The compiler builds one form that keeps every row
in source order and at its span, so S3 can report each mapping problem at
the row the author wrote.

## Inputs

- The S1 CST of a `refinement` declaration: its name, its profile
  selection, its abstract side (a model alias, or a model alias and a
  protocol clause name), its concrete model alias, and its `population`,
  `object` (with field rows), `step`, `history`, `assume` and `ensure` rows,
  the rows of QSpec FR-375's `refinement-row` production and, over a protocol
  subject, FR-432's `protocol-step-row`.

## Outputs

```rust
pub struct RefinementForm {
    pub name: Name,
    pub profile: ProfileRef,
    pub abstract_side: AbstractSideForm,       // Model(alias) | Protocol(alias, clause)
    pub concrete: Name,                        // model alias
    pub rows: Vec<RefinementRowForm>,          // in source order
    pub span: Span,
}

pub enum RefinementRowForm {
    Population { target: QualifiedName, source: QualifiedName, span: Span },
    Object { target: QualifiedName, source: QualifiedName,
             fields: Vec<FieldRowForm>, span: Span },
    Step { source: StepSourceForm, target: StepTargetForm, span: Span },
    History { field: QualifiedName, ty: TypeForm, initial: HistoryLiteralForm,
              updates: Vec<(QualifiedName, ExpressionForm)>, span: Span },
    Fairness { side: FairnessSide, constraint: FairnessConstraintForm, span: Span },
}

pub enum StepSourceForm {                      // QSpec FR-432 protocol-step-source
    // keyword optional: an operation, attempt node, event, send, receive,
    // timeout, finish node or fence is a bare qualified name; fork, join, ...
    Named { kind: Option<StepClassKeyword>, name: QualifiedName },
    Memory(Name),                              // `memory <kind>`
}

// QSpec FR-375's closed `history-literal` production
pub enum HistoryLiteralForm {
    Integer(SignedInt), Boolean(bool), None, Text(StringLiteral),
    Exact(ExactNumberForm), EnumMember(QualifiedName),
}

pub enum StepTargetForm {
    Apply { operation: QualifiedName, arguments: Vec<ArgumentForm> }, // `_` as ArgumentForm::Open
    Node { node: QualifiedName, arguments: Vec<ArgumentForm> },       // FR-432 `node ...`
    Stutter,
    Any,
}
```

## Behavior

- S2 SHALL build one `RefinementForm` for each `refinement` declaration,
  holding every row in source order with its span.
- S2 SHALL build each history row's initial value as the
  `HistoryLiteralForm` of QSpec FR-375's `history-literal` production,
  and SHALL NOT build it as an expression form: a collection, record,
  float or other non-literal spelling is not a form, because S1 refuses it
  (QSpec FR-375-AC-9). Whether the literal is a member of the declared type,
  and whether an enum path resolves, are S3's (FR-138).
- S2 SHALL build the rows QSpec FR-375 and FR-432 define and no others. The
  grammar has no `internal` or `visible` row: a concrete step is internal
  when its step row is `stutter`, and an abstract step is internal only when
  the refinement profile's list names it (QSpec FR-432), so the form holds
  neither a row nor a list for it.
- S2 SHALL build a step row's source as `Named` with the keyword exactly as
  written (absent when none is written) or as `Memory`, and SHALL NOT
  decide whether an unkeyworded name is an operation, a node or a fence:
  S3 classifies the source by resolving the name (FR-136, QSpec FR-432).
- The `AbstractSideForm::Protocol` side (`abstract <A>::<protocol>`) has no
  QSpec production yet: FR-375's header admits a model alias only. QSL
  builds the form and the side waits on that production, tracked as Linear
  STD-182 (ADR-020 QS-11).
- S2 SHALL build each field row's right side, each step-row argument and
  each history update as an expression form as written, the
  population-valued forms (FR-139) included, and an open argument `_` as
  `ArgumentForm::Open`.
- S2 SHALL build each `assume` and `ensure` row's kind (`weak` or `strong`)
  as written, and its granularity as written: an unwritten granularity is
  built as absent, and S3 reads absent as `whole` (FR-137).
- S2 SHALL build no form that resolves a name: whether an alias names a
  model, a row names a population, type, operation or protocol class, and
  whether the rows are complete, are S3's (FR-135 to FR-137, FR-147).
- Where the profile selection names a profile that admits no `refinement`
  declaration, S2 SHALL still build the form; S3 refuses it (FR-135).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-310-AC-1 | ADR-020 §8's `CasRefinesCounter` builds one `RefinementForm` with abstract side `Model(Spec)`, concrete `Impl`, and its population, object, seven step, `assume` and `ensure` rows in source order, each with its span; the `ensure` row's granularity is `each` as written. | Test (TC-889) |
| FR-310-AC-2 | `RingIsQueue`'s `items` row builds its `seq` and `only` forms as expression forms, and `put -> enq(self.ring, _)` builds `Apply` with arguments `[self.ring, Open]`. FR-147's `CasTwice` builds abstract side `Protocol(Spec, Twice)`; a protocol step row `step fork Impl::P::par -> node Spec::Twice::a1;` builds a `Named` source of kind `fork` and a `Node` target; `step Impl::P::a::S -> stutter;` builds a `Named` source with no kind, whichever class the name later resolves to. | Test (TC-889) |
| FR-310-AC-3 | A declaration whose abstract alias names no model, and one with a duplicate `population` row, both build forms; the refusals come from S3 (FR-135-AC-2). | Test (TC-889) |
| FR-310-AC-4 | A history row with initial `-3`, `true`, `none`, `"a"`, `rational(1, 2)` and `Impl::Mode::idle` builds `Integer`, `Boolean`, `None`, `Text`, `Exact` and `EnumMember` literal forms respectively, with no expression form; a fairness row `assume fair strong Impl::Counter::retryA;` builds kind `Strong` with granularity absent. | Test (TC-889) |

## Dependencies

- ADR-020 §1 RM-1 to RM-9, QS-1 and QS-11.
- [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md)
  to [FR-139](FR-139-type-and-evaluate-population-valued-expressions.md)
  and [FR-147](FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md)
  check the form.
- [FR-136](FR-136-check-a-refinement-s-step-rows.md) classifies step
  sources at S3.
- QSpec owns the surface grammar: FR-375 (rows, `history-literal`, fairness
  rows) and FR-432 (protocol step rows) (ADR-020 QS-1, QS-11).

## References

- ADR-020. QSpec half: QSpec FR-375 (Linear STD-133; ADR-020 QS-1, QS-11).

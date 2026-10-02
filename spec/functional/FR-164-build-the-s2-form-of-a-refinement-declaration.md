---
id: FR-164
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
# FR-164: Build the S2 form of a refinement declaration

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
  `object` (with field rows), `step`, `history`, `assume`, `ensure`,
  `internal` and `visible` rows.

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
    Step { concrete: QualifiedName, target: StepTargetForm, span: Span },
    History { field: QualifiedName, ty: TypeForm, initial: ExpressionForm,
              updates: Vec<(QualifiedName, ExpressionForm)>, span: Span },
    Fairness { side: FairnessSide, constraint: FairnessConstraintForm, span: Span },
    Internal { class: QualifiedName, span: Span },
    Visible { class: QualifiedName, span: Span },
}

pub enum StepTargetForm {
    Apply { operation: QualifiedName, arguments: Vec<ArgumentForm> }, // `_` as ArgumentForm::Open
    Stutter,
    Any,
}
```

## Behavior

- S2 SHALL build one `RefinementForm` for each `refinement` declaration,
  holding every row in source order with its span.
- S2 SHALL build each field row's right side, each step-row argument and
  each history update as an expression form as written, the
  population-valued forms (FR-139) included, and an open argument `_` as
  `ArgumentForm::Open`.
- S2 SHALL build each `assume` and `ensure` row's kind and granularity as
  written, with no granularity left absent for S3 to read as `whole`.
- S2 SHALL build no form that resolves a name: whether an alias names a
  model, a row names a population, type, operation or protocol class, and
  whether the rows are complete, are S3's (FR-135 to FR-137, FR-147).
- Where the profile selection names a profile that admits no `refinement`
  declaration, S2 SHALL still build the form; S3 refuses it (FR-135).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-164-AC-1 | ADR-020 §8's `CasRefinesCounter` builds one `RefinementForm` with abstract side `Model(Spec)`, concrete `Impl`, and its population, object, seven step, `assume` and `ensure` rows in source order, each with its span; the `ensure` row's granularity is `each` as written. | Test (TC-557) |
| FR-164-AC-2 | `RingIsQueue`'s `items` row builds its `seq` and `only` forms as expression forms, and `put -> enq(self.ring, _)` builds `Apply` with arguments `[self.ring, Open]`. FR-147's `CasTwice` builds abstract side `Protocol(Spec, Twice)` and an `Internal` row naming `Spec::Twice::finish`. | Test (TC-557) |
| FR-164-AC-3 | A declaration whose abstract alias names no model, and one with a duplicate `population` row, both build forms; the refusals come from S3 (FR-135-AC-2). | Test (TC-557) |

## Dependencies

- ADR-020 §1 RM-1 to RM-9, QS-1 and QS-11.
- [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md)
  to [FR-139](FR-139-type-and-evaluate-population-valued-expressions.md)
  and [FR-147](FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md)
  check the form.
- QSpec owns the surface grammar (ADR-020 QS-1, QS-11).

## References

- ADR-020. QSpec half: QSpec FR-375 (Linear STD-133; ADR-020 QS-1, QS-11).

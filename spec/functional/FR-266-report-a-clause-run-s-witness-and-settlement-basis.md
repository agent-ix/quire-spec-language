---
id: FR-266
title: "Report a clause run's settlement basis and separating witness"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-031
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-265
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-243
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-351
    type: depends_on
---
# FR-266: Report a clause run's settlement basis and separating witness

## Description

FR-109's `ClauseRunReport` SHALL carry a settlement basis (QSpec FR-243) on
every report and the separating witness record (QSpec FR-351) exactly when
that basis is decisive, as ADR-031 SW-3, SW-10 and its basis ruling decide.
A `Clause` selection takes both from FR-265's derivation. The basis is the
clause run's own; a model-check verdict's basis is FR-127's (ADR-031 SW-3).

## Inputs

- An FR-109 `ClauseRunRequest` and the report FR-109 builds for it.
- For a `Clause` selection that completes a Boolean at stage `evaluate`,
  FR-265's basis and record.

## Outputs

`ClauseRunReport` gains two members:

- `basis`: one QSpec FR-243 label, present on every report;
- `witness`: `Option<SeparatingWitnessRecord>`, present exactly when
  `basis` is `decisive-counterexample` or `decisive-witness`.

## Behavior

- For a `Clause` selection whose disposition is stage `evaluate`, category
  `success` or `violation`, the entry SHALL set `basis` and `witness` to
  FR-265's basis and record.
- For a `Function` selection whose disposition is stage `evaluate`,
  category `success` or `violation`, the entry SHALL set `basis`
  `closed-scope` and no `witness`: a function run as a claim has its result
  decided by the whole call.
- For a `Frame` selection whose disposition is stage `evaluate`, category
  `success` or `violation`, the entry SHALL set `basis` `closed-scope` and
  no `witness`; the frame witness stays FR-115's.
- For every other disposition (stage `compile`, `select` or `admit`, and
  category `refusal`, `undefined`, `incomplete` or `internal-failure` at
  stage `evaluate`), the entry SHALL set `basis` `unavailable` and no
  `witness`, and SHALL put no substitute value in the witness's place.
- The clause run's exit code (FR-285, FR-109) SHALL be unchanged by these members.
- Two runs of one request SHALL give equal `basis` and `witness`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-266-AC-1 | A `Clause` run of FR-265's `AllBelow` over `high` reports stage `evaluate`, `violation`, `truth: false`, exit 10, `basis` `decisive-counterexample` and FR-265-AC-1's record; `SomeAtLeast` over `high` reports `success`, exit 0, `decisive-witness` and FR-265-AC-2's record; `AllBelow` over `low` reports `success`, `closed-scope` and no `witness`. Running the `AllBelow` request twice gives equal reports. | Test (TC-741) |
| FR-266-AC-2 | FR-108's corpus requests keep their FR-109 dispositions and exit codes and add: healthy-parent `closed-scope`, violating-parent `closed-scope`, changed-version `closed-scope`, each with no `witness`; missing-model, dangling-parent, incomplete-population and exhausted-work `unavailable` with no `witness`. A `Function` selection of `sameIdentity` reporting `violation` carries `closed-scope` and no `witness`, and the same selection with `b` naming `ghost` (stage `admit`) carries `unavailable`. | Test (TC-741) |
| FR-266-AC-3 | A `Frame` selection whose invocation changes a member outside its frame (FR-115) reports `violation` with `basis` `closed-scope`, no `witness`, and its FR-115 frame witness unchanged. A report built for an `evaluate` `internal-failure` carries `unavailable` and no `witness`. | Test (TC-741) |

## Dependencies

- [ADR-031](../decisions/ADR-031-state-forall-separating-witness.md) SW-3,
  SW-10 and the basis ruling (R-3).
- [FR-109](FR-109-run-a-state-clause-through-the-spine.md) (the report this
  extends), [FR-115](FR-115-run-an-operation-frame-over-an-invocation.md)
  (the frame selection), [FR-265](FR-265-derive-a-state-clause-separating-witness.md)
  (the derivation), [FR-108](FR-108-run-the-configversion-spine-corpus.md)
  (the corpus).
- QSpec FR-243 (basis labels) and QSpec FR-351 (the record).

## References

- Owning ticket: Linear QSL-389. Implementation: Linear QSL-45.
- QSpec half (Linear STD-144): QSpec FR-352-AC-6 (the basis label on
  `native-run-result/2`).

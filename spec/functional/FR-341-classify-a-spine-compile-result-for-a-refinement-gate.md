---
id: FR-341
title: "Classify a spine compile result into one refinement class"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-034
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: traces_to
  - target: ix://agent-ix/quire-specification/AD-003
    type: depends_on
---
# FR-341: Classify a spine compile result into one refinement class

## Description

QSL SHALL provide one classification of a spine `compile` result
(`Result<CheckedPackage, CompileRefusal>`) into a refinement class, shared
by the spec-versioning gate (FR-342 reads it for a run that stops at
compile) and the profile-layering gate (FR-345). It reads every cause the
refusal holds, never only the first, and never reads message text (ADR-011
FB-02).

## Inputs

A spine `compile` result.

## Outputs

One `RefinementClass`: `admitted`, `refused`, `prohibited`, `unsupported`,
`incomplete` or `tool failure`, with the codes and causes of every cause
the refusal holds, in the refusal's order. An `incomplete` class also
carries, for each `stage_limit_exceeded` cause, the limit it names and that
limit's value.

## Behavior

- `CompileRefusal` SHALL expose an all-causes accessor that yields every
  cause of an `Assembly` or `Check` refusal's vector, and the one cause of
  every other variant, each with its code and cause.
- The classification SHALL apply these rules in order; the first rule that
  matches SHALL give the class:

| Rule | When | Class |
| --- | --- | --- |
| 1 | the result is `Ok` | `admitted` |
| 2 | any cause has code `runtime_invariant`; or the refusal is `Intake` with a cause other than its limit cause, `Profile`, `DependencyInput`, `Import` with a cause other than its limit cause, `Dependency`, `Link` or `Emit` | `tool failure` (the case's setup is broken) |
| 3 | the refusal is `Source`, `Forms`, `Assembly` or `Check`, and any cause has a typed refusal code: a code other than `stage_limit_exceeded`, `unsupported_construct` and `unsupported_projection` | `refused` |
| 4 | any cause has code `stage_limit_exceeded`, whichever stage raised it | `incomplete` |
| 5 | the refusal is `Omitted`; or any cause has code `unsupported_projection`; or any cause is `unsupported_construct` with cause `not-yet-implemented` | `unsupported` (a QSL implementation gap) |
| 6 | otherwise: every cause is `unsupported_construct` with a catalog cause (`declaration-form`, `expression-form`) | `prohibited` (the selected profile prohibits the form, QSpec AD-003) |

- The classification SHALL be one exhaustive `match` over `CompileRefusal`
  with no wildcard arm, so that a new refusal variant does not compile until
  it is classified.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-341-CON-1 | The classification matches `CompileRefusal` with no wildcard arm and reads codes and causes, never message text. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-341-AC-1 | A ConfigVersion unit that compiles classifies `admitted`. | Test (TC-861) |
| FR-341-AC-2 | Each of these classifies `tool failure`: the unit with a header profile at a `root` version other than the catalog's (`Profile`); the unit with its `model` package not supplied (`Intake`); a dependency input that refuses (`DependencyInput`); an `Import` naming a library the input does not hold; and a `Check` refusal holding a typed refusal cause and a `runtime_invariant` cause, in either order. | Test (TC-861) |
| FR-341-AC-3 | A unit with an ill-typed function body classifies `refused`, carrying `ill_typed`. A `Check` refusal whose first cause is `unsupported_construct`/`not-yet-implemented` and whose second is a typed refusal classifies `refused` and carries both causes in order. | Test (TC-861) |
| FR-341-AC-4 | The unit compiled with a forms-stage limit of one and, separately, with a type limit of one each classifies `incomplete`, naming the limit and its value one. | Test (TC-861) |
| FR-341-AC-5 | A unit using a construct S3 refuses `unsupported_construct`/`not-yet-implemented` classifies `unsupported`; an `Omitted` refusal and a refusal holding `unsupported_projection` each classify `unsupported`. | Test (TC-861) |
| FR-341-AC-6 | A refusal whose every cause is `unsupported_construct` with cause `declaration-form` or `expression-form` classifies `prohibited`. | Test (TC-861) |

## Dependencies

- ADR-011 (the spine stages and `CompileRefusal`), FR-110 (`Profile`).
- QSpec AD-003 (profile-prohibited forms are distinct from typed refusals).

## References

- ADR-017 §2 RF-2 "Compile classification", RF-3.
- Specification tickets QSL-386, QSL-387; implementation ticket QSL-40.

## Status

Specified; not yet implemented -- TC-861 planned.

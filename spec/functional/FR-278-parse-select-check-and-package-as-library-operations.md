---
id: FR-278
title: "Parse, select, check and package as typed library operations"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
---
# FR-278: Parse, select, check and package as typed library operations

## Description

QSL SHALL expose its front end as four library operations with the FR-275
call shape (ADR-029 OP-1):

- `parse` reads source bytes into syntax (S1, S2). It takes the source bytes,
  their `SourceIdentity` and the S1 and S2 limits, and returns
  `Staged<ParsedSource>`. Tooling reads S1's `LosslessCst` from the
  `ParsedSource`.
- `select` admits the domain packages a unit's `model` declarations select
  (I1 `model::intake`, FR-056). It takes the domain package documents by
  digest and the unit's model selections, and returns
  `Staged<AdmittedModels>`.
- `check` checks and links (S3, S4). It takes the `ParsedSource`, the
  `AdmittedModels`, the dependency input (ADR-015 D-1, FR-099) and the lock
  evidence (ADR-011 §2.4), and returns `Staged<CheckedPackage>`.
- `package` emits the checked package (E4). It takes `&CheckedPackage` and
  returns `Staged<EmittedPackage>`: the `quire.checked-package/v2` bytes with
  the digest-addressed source provision another party needs to import (I2)
  or replay (E9) the package.

The composition `parse`, `select`, `check`, `package` is the spine compile: S1
to E4 over one source. No single function runs the four; each caller composes
them. FR-027's native-compile/1 request for a `1-draft` program is the command
encoding of this composition.

None of the four takes a backend registry (FR-289).

## Inputs

As listed for each operation, plus its limits value and `&Cancel` (FR-275).

## Outputs

As listed for each operation. A refusal is `StageFailure::Refused` carrying
the refusing stage's own typed causes.

## Behavior

- The `parse` operation shall run S1 and S2 over the given source bytes and
  return the `ParsedSource` or S1's or S2's refusal.
- The `select` operation shall admit each domain package the unit's model
  selections name, by digest, and return the `AdmittedModels` or I1's
  refusal.
- The `check` operation shall check and link the `ParsedSource` against the
  `AdmittedModels` and the dependency input, and return the
  `CheckedPackage` or S3's or S4's refusal.
- The `package` operation shall emit the `CheckedPackage` as
  `quire.checked-package/v2` bytes with its source provision.
- The composition of the four operations over a source shall return the same
  bytes, and for a refused source the same stage and cause code, that FR-027's
  `compile` command writes and reports for that source and inputs.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-278-AC-1 | `parse`, `select`, `check` and `package` composed over `tests/fixtures/spine-compile.native`, over FR-027-AC-9's domain-package request and over FR-027-AC-10's library request each return the bytes FR-027's `compile` command writes for the same source and inputs (FR-027-AC-5, AC-9, AC-10), and QSL's I2 reader reads each back Verified. | Test (TC-759) |
| FR-278-AC-2 | A source with a syntax error is refused by `parse` with `invalid_syntax`; a `model` declaration naming a document the request does not supply is refused by `select` at the `model` declaration; FR-100-AC-5's `inv` function is refused by `check` with `ill_typed`. Each refusal's stage and cause code equal those FR-027's `compile` command reports for the same source. | Test (TC-759) |
| FR-278-AC-3 | The `EmittedPackage` from `package` carries a source provision whose digests name every source the package was compiled from, and FR-098's `replay`, given that provision, recompiles the package to the same `package_id`. | Test (TC-759) |

## Dependencies

- ADR-029 OP-1: the operation rows.
- ADR-011 §2.1, §2.4: the stage path and the lock evidence.
- ADR-015 D-1: the dependency input.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the call shape.
- [FR-027](FR-027-export-compiled-native-package.md): the command encoding.
- [FR-056](FR-056-admit-domain-package-model-declarations.md): domain package admission.
- [FR-099](FR-099-compile-against-supplied-libraries.md): supplied libraries.
- QSpec FR-300: the `parse`, `select_model` and `check` operations.

## References

- QSL-393 (V1-A06): typed library APIs.
- QSpec FR-300 (STD-141): the QSpec half.

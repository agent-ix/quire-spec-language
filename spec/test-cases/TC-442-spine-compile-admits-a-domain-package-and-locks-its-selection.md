---
id: TC-442
title: "Spine compile admits a domain package and locks its model selection"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-442: Spine compile admits a domain package and locks its model selection

## Description

Verify that a complete-V1 (`1-draft`) source whose `model M` declaration
selects a domain package by its identity compiles end to end through
the spine (S1, S2, I1, the assembler, check, link, the v2 emitter and QSL's I2
read), that the v2 bytes select that package in `model_selections`, and that
each defect refuses at the stage that owns it. This catches a model selection
dropped from the lock, a model type the assembler cannot name, and a refusal
reported at the wrong stage or region.

Scope: FR-027-AC-9, FR-056-AC-9.

## Test Procedure

1. Run `tests/fixtures/spine-model.native` (functions over `M::Gadget` and
   `Reference<M::Widget>`, `deref(g).code` over the `code` field `Gadget`
   inherits from `Widget`, and `g = w` over a `Gadget` and a `Widget`) with `tests/fixtures/spine-model.semantic-ir.json`
   as the package input through S1, S2, `model::intake::admit_unit`,
   `PackageDeclarations::assemble`, check, `CheckedPackage::link` and
   `emit_checked` (`qsl-package`). Read the bytes back through QSL's I2 reader
   first without `code` and `same`; then the fixture
   with `code` only, and with `same` only.
2. Compile a native-compile/1 request selecting that program and the document
   as a `semantic-ir/2.0.0` model, and compare the outcome's package bytes with
   the FR-278 composition over the same source and package input.
3. Assemble the fixture with `Reference<M::Nope>`, and with no admitted
   model; admit its selections with no package input, under a
   `declaration_records` ceiling of one, and against a document whose
   `Widget` and `Gadget` generalize each other.
4. Compile the fixture through `command` with `deref(g).nope`; with `g = w`
   over `w: M::Rock`; with `Reference<M::Nope>`; with no
   model; with a document of the same package whose version differs; and with
   a clause after the declaration's identity.

Tag the tests `#[trace("TC-442", "FR-027-AC-9", "FR-056-AC-9")]` (step 1,
including its two field-access and conforming-equality cases),
`#[trace("TC-442", "FR-027-AC-9")]` (step 2),
`#[trace("TC-442", "FR-027-AC-9", "FR-056-AC-9")]` (step 4) and
`#[trace("TC-442", "FR-056-AC-9")]` (step 3).

## Expected Results

- Step 1: nothing is omitted; `M::Gadget` and `M::Widget` are declared object
  types and `Gadget` conforms to `Widget`, not the reverse; the lock and the
  identity preimage `model_selections` are
  `[{"identity": "acme/orders", "version": <the document's version>}]`; a
  model node is emitted; without `code` and `same` the read is Verified and
  exports `keep` and `held`. With `code` only, the read is
  Verified and exports `code`: IR-285 and QSpec STD-100 resolve the field
  member through the lock-selected domain package. With `same` only, the
  read is Verified and exports `same`: IR-285's QVC checked-operation
  catalog (STD-101/102) admits conforming operands for
  `quire.op.reference.eq`.
- Step 2: exit 0; the package bytes equal the library bytes, and its `model_selections`
  entry is the one the program's `model` declaration names.
- Step 3: one assembler error, `UnresolvedTypeName("M::Nope")`, code
  `missing_declaration`, at `M::Nope`; with no admitted model,
  `UnadmittedModel`, `missing_import`, at the `model` declaration; intake
  refuses `missing_import`, then `stage_limit_exceeded`, then
  `invalid_model_binding`/`malformed-declaration`, each at the `model`
  declaration.
- Step 4: exit 20 and no package bytes each time; stage `check` with
  `ill_typed` at `deref(g).nope` and at `g = w`; stage `assembly` with
  `missing_declaration` at `M::Nope`; stage `intake` with `missing_import`
  for the absent model and `stale_dependency`/`content-mismatch` for the
  document whose version differs, each at the `model`
  declaration; `invalid_syntax` at the clause after the identity; every
  message is readable text.

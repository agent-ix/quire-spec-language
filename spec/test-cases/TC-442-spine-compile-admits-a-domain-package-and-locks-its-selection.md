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
selects a domain package by its `sha256-jcs` digest compiles end to end through
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
   with the document's `sha256-jcs` digest as domain package evidence, then
   with another digest.
2. Compile a native-compile/1 request selecting that program and the document
   as a `semantic-ir/2.0.0` model, and compare stdout with
   `command::spine::compile` over the same source and package input.
3. Assemble the fixture with `Reference<M::Nope>`, and with no admitted
   model; admit its selections with no package input, under a
   `declaration_records` ceiling of one, and against a document whose
   `Widget` and `Gadget` generalize each other.
4. Compile the fixture through the CLI with `deref(g).nope`; with `g = w`
   over `w: M::Rock`; with `Reference<M::Nope>`; with no
   model; with a document of the same package whose digest differs; and with
   the declaration's digest spelled `sha256:`.

Tag the tests `#[trace("TC-442", "FR-027-AC-9", "FR-056-AC-9")]` (step 1),
`#[trace("TC-442", "FR-027-AC-9")]` (step 2),
`#[trace("TC-442", "FR-027-AC-9", "FR-056-AC-9")]` (step 4) and
`#[trace("TC-442", "FR-056-AC-9")]` (step 3).

## Expected Results

- Step 1: nothing is omitted; `M::Gadget` and `M::Widget` are declared object
  types and `Gadget` conforms to `Widget`, not the reverse; the lock and the
  identity preimage `model_selections` are
  `[{"identity": "acme/orders", "version": "1.0.0", "digest_domain":
  "sha256-jcs", "digest": <the document's digest>}]`; a model node is emitted;
  the read is Verified and exports `keep`, `held`, `code` and `same`; the
  other digest is refused.
- Step 2: exit 0; stdout equals the library bytes, and its `model_selections`
  digest is the one the program's `model` declaration spells.
- Step 3: one assembler error, `UnresolvedTypeName("M::Nope")`, code
  `missing_declaration`, at `M::Nope`; with no admitted model,
  `UnadmittedModel`, `missing_import`, at the `model` declaration; intake
  refuses `missing_import`, then `stage_limit_exceeded`, then
  `invalid_model_binding`/`malformed-declaration`, each at the `model`
  declaration.
- Step 4: exit 20 and empty stdout each time; stage `check` with
  `ill_typed` at `deref(g).nope` and at `g = w`; stage `assembly` with
  `missing_declaration` at `M::Nope`; stage `intake` with `missing_import`
  (twice) and `invalid_model_binding`, each at the `model` declaration; every
  message is readable text.

## Status

Passed locally (QSL-249).

---
id: TC-1293
title: "Each retired family is deleted behind its named successor"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: verifies
---
# TC-1293: Each retired family is deleted behind its named successor

## Description

Verify FR-499-AC-1 to AC-5 for a deleting change. Catches a deletion whose row
has no green successor, a variant that survives behind a re-export, a
`crate::temporal` import left after the module goes, and a retained criterion
left untagged.

## Test Procedure

1. For the deleting change, read the FR-499 table for each removed path and run
   the successor test named in each row; confirm it carries the successor tag.
2. After QSL-841, compile a downstream crate naming `DeclarationKind::Protocol`,
   `Family::Protocol` and `RegisteredDefinition::Protocol`; compile a `protocol`
   unit with unsupported content through the spine.
3. After QSL-838 and QSL-858, search `src` for `crate::temporal`,
   `crate::state`, `protocol_artifact::wire` and `ProtocolNumber`; run the
   `Lasso::Model` cases of FR-329.
4. After QSL-499, confirm `src/native_model` is absent and run TC-469.
5. After QSL-830 and QSL-868, run TC-1027 steps 2 and 3 and parse a profile line
   carrying a version and a digest with every parser.

## Expected Results

- Step 1: no row is a GAP and every successor passes.
- Step 2: the imports fail; the unit refuses `unsupported_construct`.
- Step 3: no import remains and the model lasso passes.
- Step 4: the path is absent and TC-469 passes.
- Step 5: no parser admits the profile line.

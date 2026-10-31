---
id: TC-1276
title: "The native package, runtime-input, front-end and protocol-artifact surfaces are absent"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-493
    type: verifies
---
# TC-1276: The native package, runtime-input, front-end and protocol-artifact surfaces are absent

## Description

Verify that after the retirement the root package offers no way to build, read,
validate or run a native package or runtime-input artifact, no `0-draft` front
end, and no compiled-protocol reader, while the spine paths that replace them
keep their results. Scope: FR-493-AC-1 to FR-493-AC-5.

Catches a retired module kept behind a feature, a re-export or a test-only
copy; a retired wire-format name that survives in an enum
variant; a `0-draft` source that still compiles by any route; a build gate that
still invokes the retired parser; and a retained acceptance criterion left
without a test because its only test went with the retired path.

## Test Procedure

1. Compile one downstream crate per item and confirm each import fails because
   the item is absent: the root `package` and `runtime` modules,
   `NativePackage` and `runtime::execute`; the `0-draft` parser entry points,
   arena `syntax`, `linking::native`, native `checking`, `model_source` and
   `located_json`; and, once the handoff writers are deleted (FR-493-AC-4), any
   module that reads compiled-protocol v1, v2 or v3 bytes. Confirm the
   `composed` modules, `formal_source` and `native_model` still import.
2. Compile a downstream crate that matches exhaustively on `qsl-foundation`'s
   `WireFormat` and confirm it names no variant for `native-linked-package/1`
   or `native-state-input/1`. Run every Makefile target, `xtask` lane and
   example and confirm each builds and runs with the retired parser absent.
3. Run the CLI `compile` on a source declaring `edition "0-draft"`, on one
   declaring `edition "7-draft"`, and on a `1-draft` source. Confirm the first
   two refuse `unknown_edition` at stage `profile` with exit code 20 and write
   no package bytes, and that the third compiles as TC-435 specifies.
4. Read the checked package TC-1063 emits for fixture **W** through the I2
   reader and confirm it still returns the emitted protocol equal to the
   in-process checked clause.
5. Run `quire matrix` over the repository. Confirm no criterion of a retained
   requirement is reported without a tagged test because its test was deleted.

## Expected Results

- Step 1 fails to compile for every item; step 2 compiles
  and runs.
- Step 3 refuses the first two sources with the cause, stage and exit code
  given and compiles the third.
- Step 4 reads the protocol unchanged.
- Step 5 shows no uncovered retained criterion.

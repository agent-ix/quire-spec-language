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

Verify that after this slice the root package offers no way to build, read,
validate or run a native package or runtime-input artifact, while the spine
compile path keeps its result. Scope for this slice: FR-493-AC-1, FR-493-AC-2
and FR-493-AC-5. Parser and protocol-reader absence checks belong to the later
slices that delete those paths (FR-493-AC-3 and FR-493-AC-4).

Catches a retired module kept behind a feature, a re-export or a test-only
copy; a retired wire-format name that survives in an enum
variant; a `0-draft` source that still compiles by any route; a build gate that
still invokes the retired parser; and a retained acceptance criterion left
without a test because its only test went with the retired path.

## Test Procedure

1. Compile one downstream crate per package/runtime item and confirm each
   import fails because the item is absent: the root `package` and `runtime`
   modules, `NativePackage` and `runtime::execute`.
2. Compile a downstream crate that matches exhaustively on `qsl-foundation`'s
   `WireFormat` and confirm it names no variant for `native-linked-package/1`
   or `native-state-input/1`.
3. Run the CLI `compile` on a source declaring `edition "0-draft"`, on one
   declaring `edition "7-draft"`, and on a `1-draft` source. Confirm the first
   two refuse `unknown_edition` at stage `profile` with exit code 20 and write
   no package bytes, and that the third compiles as TC-435 specifies.
4. Read the checked package TC-1063 emits for fixture **W** through the I2
   reader and confirm it still returns the emitted protocol equal to the
   in-process checked clause.
5. Run `quire matrix` over the repository after the retained criteria and tests
   for this slice have been reconciled.

## Expected Results

- Step 1 fails to compile for every item; step 2 compiles
  and runs.
- Step 3 refuses the first two sources with the cause, stage and exit code
  given and compiles the third.
- Step 4 belongs to the protocol-reader retirement slice.
- Step 5 is the aggregate matrix check for the final slice head.

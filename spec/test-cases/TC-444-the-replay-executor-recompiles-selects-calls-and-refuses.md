---
id: TC-444
title: "The replay executor recompiles, selects, calls, and refuses each O-26 case"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: verifies
---
# TC-444: The replay executor recompiles, selects, calls, and refuses each O-26 case

## Description

Verify that `qsl_replay::replay` recompiles a complete-V1 unit from a replay
request's byte provision, requires its `package_id`, selects and calls the
function, settles the verdict, and refuses each ADR-013 O-26 case with a
typed variant and no partial result.

Scope: FR-098-AC-1 to FR-098-AC-7, and FR-001-AC-6 for the recompile's
source labels.

## Test Procedure

The unit declares `small(x: Int[0, 9]): Boolean { x < 5 }`,
`flag(b: Boolean): Boolean`, `id(x: Int[0, 9]): Int[0, 9]`,
`lt(a: Int[0, 9], b: Int[0, 9]): Boolean { a < b }`,
`maybe(t: Option<Boolean>): Boolean`, `f(x: Int[0, 9]): Integer { x + 1 }`
and `p(x: Int[0, 9]): Boolean { f(x) > 3 }` (QSL-257: `p`'s body is a `call`
expression node applying a distinct declared function, the one shape none
of the other declared functions exercise -- the QSL-22 Layer 3 exemplar's
own shape). Each request
is built from the unit's spine compile: its `package_id`, its one source
reference and its byte provision, with parameter node ids read from the
compiled package.

1. Replay `small` with an `Input` assignment `x = 7`; replay it with a
   `Witness` binding `x`'s node id to 8, then with a binding named `x`.
2. Replay `small(3)`, and `small(7)` with a zero work budget; replay
   `flag(0)` and `flag(1)`; replay `lt` with `b = 3` given before `a = 5`.
3. Replay against the original `package_id` with `x < 6` as the source;
   then with a blank line added, supplied under its own digest and under the
   original digest.
4. Compile a unit that selects `tests/fixtures/spine-model.semantic-ir.json`
   by `sha256-jcs`; replay it with and without the document in the byte
   provision.
5. Replay with an unknown contract version; the selections `large` and
   `module.small`; no argument, an extra argument naming `flag`'s parameter,
   and `x` bound twice; `maybe(1)`, `flag(2)` and `small(12)`; S1
   `text_input_bytes` above 1 MiB and at 16, and S3 `work_units` at 0; a
   whitespace-only authority label; a definition document or a second
   source in the package reference; and `id(4)` with a zero work budget.
6. Replay `p(1)`, whose body calls `f`; replay `p(5)`.
7. Library `test/units` version `2` declares
   `function big using v(x: Int[0, 9]): Boolean pure { x > 5 }`. A second
   unit imports it `as u` and declares
   `function q using v(x: Int[0, 9]): Boolean pure { u::big(x) }`. Build its
   request from its spine compile against `test/units`: `sources` names the
   unit's one source, `dependencies` holds `{test/units, 2, <its
   package_id>, [its one source]}`, and the byte provision carries both.
   Replay `q(3)`. Then replace `test/units`'s bytes with `x > 6`, with the
   entry's digest updated to the new bytes; then restore them and change
   only the entry's `package_id`; then add an extra entry no import
   reaches; then remove the entry; then give the entry a second source.

Tag the tests `#[trace("TC-444", ...)]` with the ACs each step backs.

## Expected Results

- Step 1: `reproduced-without-witness`, value `false`, the executor's pin and
  non-zero work charges; `reproduced-with-evaluated-witness` with a record
  whose deciding element is `false`; a missing-binding decode refusal naming
  `x`'s node id.
- Step 2: `inconclusive` with cause `Verdicts` (`violation`/`success`),
  then with cause `NoValue`, replayed verdict `incomplete` and no value;
  `flag(0)` agrees and `flag(1)` is `inconclusive` with value `true`; `lt`
  is `5 < 3`, `false`, and agrees.
- Step 3: `PackageIdMismatch` naming both identities; the edit keeps the
  `package_id`, and the two supplies refuse `IncompleteByteProvision` and
  `ByteDigestMismatch`.
- Step 4: agreement with the document; stage `intake`, `missing_import`
  without it.
- Step 5: in order, `UnknownContractVersion`; `UnknownFunction` naming the
  selection and the recompiled package; `UnboundParameter`,
  `UnknownParameter` and `DuplicateArgument` naming the node; `Input`
  `WrongValueKind` (`invalid_runtime_input`) three times, the first two
  before the call and the third at S6a admission; `LimitAboveReader`, a
  stage-`source` and a stage-`check` `stage_limit_exceeded` recompile
  refusal; a stage-`source` `invalid_source_identity` recompile refusal;
  `NotASource` and `SourceCount(2)`; `NotAPredicate` naming the selection
  and the recompiled package, not an `incomplete` settlement.
- Step 6: `p(1)` is `f(1) > 3` = `2 > 3` = `false`, agreeing with the
  refuted property (`reproduced-without-witness`); `p(5)` is `f(5) > 3` =
  `6 > 3` = `true`, `inconclusive` with cause `Verdicts`. The S4 emitter
  writes `p`'s `call` node (`node_tag` `expression`, `semantic_form` `call`)
  in exactly the checked-package/v2 shape codegen's FR-021 oracle generator
  reads (`quire-contract-codegen/src/exact_function.rs`, confirmed
  read-only against `origin/main`); no QSL emission change was needed, and
  the shape is separately confirmed structurally by
  `qsl-package/src/emit/tests.rs::emit_checked_places_the_calls_occurrence_at_its_own_source_span`,
  which round-trips a real function-calls-function package through a real
  `quire.checked-package/v2` emit and IR's own I2 decode.

- Step 7: `q(3)` is `false` and agrees (`reproduced-without-witness`). The
  edited bytes refuse `Recompile` carrying `DependencyIdentityMismatch`
  (`stale_dependency`) naming `test/units`, the recorded and the recompiled
  `package_id`; the changed entry refuses `DependencyIdentityMismatch`
  naming `test/units`; the extra entry refuses `DependencySelections`
  (`invalid_package`/`invalid-value` at `/package/dependencies`); the
  removed entry refuses `Recompile` carrying
  `missing_import`/`missing-selection` at the import; the second source
  refuses `SourceCount(2)`. None yields a verdict.

## Status

Passed locally (QSL-5, QSL-257), `qsl-replay/src/execute/tests.rs`, for
steps 1 to 6. Step 7 (FR-098-AC-6, FR-098-AC-7) is planned (QSL-255 part
b).

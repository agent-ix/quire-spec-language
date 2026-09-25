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

Scope: FR-098-AC-1 to FR-098-AC-5, and FR-001-AC-6 for the recompile's
source labels.

## Test Procedure

The unit declares `small(x: Int[0, 9]): Boolean { x < 5 }`,
`flag(b: Boolean): Boolean`, `id(x: Int[0, 9]): Int[0, 9]`,
`lt(a: Int[0, 9], b: Int[0, 9]): Boolean { a < b }` and
`maybe(t: Option<Boolean>): Boolean`. Each request
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

## Status

Passed locally (QSL-5), `qsl-replay/src/execute/tests.rs`.

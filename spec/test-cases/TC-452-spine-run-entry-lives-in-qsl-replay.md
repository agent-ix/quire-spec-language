---
id: TC-452
title: "The spine run entry is qsl_replay::spine::run, agrees with the CLI, and maps every outcome"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
---
# TC-452: The spine run entry is qsl_replay::spine::run, agrees with the CLI, and maps every outcome

## Description

Verify that the library entry `qsl_replay::spine::run` returns the same
result the CLI renders, that neither the root crate nor `qsl_replay`'s
public spine surface reaches `qsl-eval`, and that the one outcome mapping
converts every S6a outcome into its `outcome` member and exit status. This
catches CLI-only run behavior (ADR-011 §5: no behavior is reachable only
through the CLI), a layer-5 type leaking through `qsl_replay`, and an
outcome kind rendered with the wrong spelling or exit status.

Scope: FR-100-AC-7 to FR-100-AC-9.

## Test Procedure

1. In `qsl-replay`, call `qsl_replay::spine::run` directly over the inputs
   of TC-450 step 1 and TC-451 steps 1, 2, 3, 5, 6 and 7, with the default
   spine stage limits, an empty package input and an empty dependency input.
2. Run TC-390's dependency check
   (`tests/it/family_outcome_layering.rs`) after the CLI change.
3. Run the `tools/arch-lint` API-surface check over `qsl_replay`: every
   public item of `qsl_replay::spine`, and every `qsl_replay` re-export, is
   scanned for a `qsl_eval` path. Then run it over a probe that adds
   `pub use qsl_eval::value::Evaluation;` to `qsl_replay`.
4. Compile the fixture unit `F` below through `qsl_replay::spine::compile`
   as the program source (its bytes are exactly these three lines, each
   ended by one LF, 229 bytes in all):

   ```text
   language "ix:native" edition "1-draft";
   profile v = "quire.value.complete/v1" version "1" digest "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
   function f using v(x: Int[0, 9]): Integer pure { x + 5 }
   ```

   Construct each S6a outcome below with an `Evaluation.location` of
   `Body{function: "f", index: 0}`, path `[1]` (the literal `5` in `f`'s
   body), and convert it with the outcome mapping (the `qsl_replay` conversion
   `spine::run` applies, then the root crate's renderer and exit mapping):
   - `Outcome::Completed` of `true`, `false`, `0`, `-17` and `2^70`;
   - `Outcome::Refused(CardinalityOutOfBound)` with kind `Set`, bound
     `[1, 3]` and count 4 (`above-maximum`), then count 0 (`below-minimum`);
   - `Outcome::Refused` of each of the other twelve kernel refusals;
   - `Outcome::Undefined` of each of the four kernel reasons;
   - `Outcome::Incomplete` at `work_units`;
   - `FamilyResult::Refused` of `ModelRefusalCause::AbsentKey` with
     `binding` `"people"` and key bytes `p7`, and of
     `ModelRefusalCause::ForeignUniverse` with `actual` 32 bytes of `0x01`
     and `expected` the universe of 32 bytes of `0x02` (each with an FR-096
     key-table row);
   - `FamilyResult::Refused` of `ModelRefusalCause::TypeMismatch` and of
     `ModelRefusalCause::AncestorSteps` (neither with a row);
   - `FamilyResult::Undefined` with reason `precondition-false` and
     `absent-key`;
   - a `CallFailure::Fault` of `InternalFault::new("S6a",
     "checked-identity-not-resolved-by-package")`.

Tag the tests `#[trace("TC-452", "FR-100-AC-7")]` (steps 1 and 2),
`#[trace("TC-452", "FR-100-AC-8")]` (step 3) and
`#[trace("TC-452", "FR-100-AC-9")]` (step 4).

## Expected Results

- Step 1: each call returns the `package_id`, outcome category, value,
  catalog code, reason, counter, and parameter name or position that the
  CLI renders for the same input in TC-450 and TC-451.
- Step 2: the root crate names `qsl-eval` in no normal, dev or build
  dependency table.
- Step 3: the check passes over `qsl_replay`, and fails over the probe,
  naming the re-export.
- Step 4: each `refused` outcome also carries `location`
  `{"origin": {"kind": "body", "function": "f", "index": 0}, "path": [1]}`.
  - `completed` with `{"kind": "boolean", "value": true}`,
    `{"kind": "boolean", "value": false}`, and `{"kind": "integer",
    "decimal": ...}` of `"0"`, `"-17"` and `"1180591620717411303424"`,
    exit 0.
  - `CardinalityOutOfBound`: `code` `cardinality_out_of_bound`, `cause`
    `above-maximum`, `fields` exactly
    `{"collection": "set", "bound": "[1, 3]", "count": "4"}`, and `locus`
    exactly `{"source_digest":
    "sha256:5f2742391e3eaef04bc5dd7141fd639b1913dc821d14bb2f2ca618ad8598ca26",
    "span": {"start": {"byte": 225, "line": 3, "column": 54}, "end":
    {"byte": 226, "line": 3, "column": 55}}}`; then `cause` `below-minimum`, `count`
    `"0"`; exit 20.
  - Each other kernel refusal but `CheckedInvariant`: exactly
    `{"kind": "refused", "location": ...}`, no `code`, `cause` or `fields`;
    exit 20.
  - `CheckedInvariant`: no `spine-run-result/1` document; a
    `runtime_invariant` command error at stage `call` with `details`
    `{"stage": "S6a", "invariant": "checked-program-invariant"}`, at
    FR-100's internal-failure exit status.
  - `undefined` with each kernel reason's tabled spelling, exit 20;
    `incomplete` with `limit` `work_units`, exit 22.
  - `AbsentKey`: `code` `invalid_runtime_input`, `cause` `absent-key`,
    `fields` exactly `{"binding": "people", "key": "p7"}`; exit 20.
  - `ForeignUniverse`: `code` `foreign_reference`, `cause`
    `foreign-universe`, `fields` exactly `{"required": R, "supplied": S}`, where
    `R` is `"02"` repeated 32 times and `S` is `"01"` repeated 32 times
    (`"0202…02"` and `"0101…01"`, 64 characters each); exit 20.
  - `TypeMismatch`: `code` `ill_typed`, `cause` `type-mismatch`, no `fields`
    member; exit 20. `AncestorSteps`: `code` `resource_exhausted`, `cause`
    `ancestor-steps`, no `fields` member; exit 22.
  - `undefined` with `reason` `precondition-false` and `absent-key`, exit 20.
  - `CallFailure::Fault`: a `runtime_invariant` command error at stage
    `call` with `details` `{"stage": "S6a", "invariant":
    "checked-identity-not-resolved-by-package"}`, at the internal-failure
    exit status.

## Status

Passed locally under QSL-271.

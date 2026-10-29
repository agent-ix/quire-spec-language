---
id: TC-1026
title: "One source yields the graph and the type report in one checked unit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: verifies
---
# TC-1026: One source yields the graph and the type report in one checked unit

## Description

Verify FR-362-AC-1, FR-362-AC-2, FR-362-AC-4, FR-362-AC-7 and FR-362-AC-8. Catches a second grammar or parse and verifies that inventory rows come from the producer's one checked unit.

## Test Procedure

1. Read one source with a predicate and an invariant under one authority and identity. Run the producer. Inspect the returned `CheckedUnit` and verify that it carries one `RawSourceRef` holding exactly the supplied authority and identity labels (QSpec FR-001), and that its graph and type report came from that parse.
2. Run `qsl_cst::parse`, `build_unit` and the producer on the same source with its profile line rewritten to carry a `version` after the identity.
3. Take the inventory of the producer-created checked unit. Compare its rows with the rows retained by that same unit; no test constructs an independent graph/report/source tuple.
4. Read a unit with the header profile `quire.model.complete/v1`, a `model` aliased `V` whose object type `ConfigVersion` holds an operation `step(i)` and a field `versionNumber`, the header alias `v` for `quire.model.complete/v1` (used by `using v`), the temporal profile `quire.temporal.infinite-trace/v1` aliased `inf`, `predicate Positive` and an `invariant`, and one `relation Det using v over (x: V::ConfigVersion::step, y: V::ConfigVersion::step)` clause whose body checks. Read it again without the `relation` clause; once more with `hyper NonInterference using inf over behaviours (v: V::ConfigVersion) forall trace a of V forall trace b of V` and a checking `match` and body in its place; and once with a `relation` clause whose body reads `x.nope`.
5. Take the inventory of a unit the S3 type phase refuses. Take the inventory of a unit whose type phase stops on `TypeWorkExhausted`, and of one that stops on `TypeAdmissionUnfinished`.

## Expected Results

- Step 1: the checked unit carries one source reference holding exactly the supplied authority and identity labels (QSpec FR-001); its graph and type report come from the same source parse.
- Step 2: `qsl_cst::parse` returns a recovered `ParsedSource` with an `invalid_syntax` diagnostic at `version`; `build_unit` and the producer refuse `invalid_syntax`.
- Step 3: inventory rows are derived from the producer-created checked unit.
- Step 4: the first read yields both, its graph holding the `relation` clause's `claim`/`step_relation` node; the third read is refused only as `NotEmitted` naming the `hyper` clause at its span, with no graph and no type report; the second yields both; the fourth is refused with the hyper cause `BodyMissing` at the `nope` read (`missing_declaration`) and not `NotEmitted`; S2 refuses no read with `NoDispatchEntry` or `UnrepresentedConstruct`; no second parser runs.
- Step 5: the refused unit is an error outcome of `check` with no inventory rows; each exhaustion stop yields an `Incomplete` inventory with its partial rows and the typed cause.
- Assertions compare typed causes and references, never message text.

---
id: TC-1026
title: "One source yields the graph and the type report, and a checked unit is bound by digest"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: verifies
---
# TC-1026: One source yields the graph and the type report, and a checked unit is bound by digest

## Description

Verify FR-362-AC-1 to FR-362-AC-4, FR-362-AC-7 and FR-362-AC-8. Catches an implementation that compares only labels, one that reads the source twice with two grammars, and one that accepts a type report built from other bytes.

## Test Procedure

1. Read one source with a predicate and an invariant under one authority and identity. Run the producer. Compare the graph's source reference, the type report unit's reference and the formal source's reference with the `quire.source.bytes/v1` digest of the input bytes.
2. Run `qsl_cst::parse`, `build_unit` and the producer on the same source with its profile line rewritten to carry `version` and `digest` after the identity.
3. Build the graph from the text of step 1 and the type report from the same text with one character of a comment changed, same authority and identity. Call the checked-unit constructor. Repeat with the graph from the changed text and the type report from the original.
4. Build the checked unit from a graph and a type report of the same bytes and take the inventory. Compare its rows with the rows of the producer's unit.
5. Read a unit with the header profile `quire.model.complete/v1`, a `model` `V` with an operation `step(i)` and a field `versionNumber`, the temporal profile `quire.temporal.infinite-trace/v1` aliased `inf`, `predicate Positive` and an `invariant`, and one `relation Det using v over (x: V::ConfigVersion::step, y: V::ConfigVersion::step)` clause whose body checks. Read it again without the `relation` clause; once more with `hyper NonInterference using inf over behaviours (v: V::ConfigVersion) forall trace a of V forall trace b of V` and a checking `match` and body in its place; and once with a `relation` clause whose body reads `x.nope`.
6. Take the inventory of a unit the S3 type phase refuses. Take the inventory of a unit whose type phase stops on `TypeWorkExhausted`, and of one that stops on `TypeAdmissionUnfinished`.

## Expected Results

- Step 1: the three references are equal and carry the digest of the input bytes.
- Step 2: `qsl_cst::parse` returns a recovered `ParsedSource` with an `invalid_syntax` diagnostic at `version`; `build_unit` and the producer refuse `invalid_syntax`.
- Step 3: both calls refuse with the cause naming the differing side, the selected reference and the differing reference.
- Step 4: the rows are equal.
- Step 5: the first and third reads are refused only as `NotEmitted` naming the `relation` or `hyper` clause at its span, with no graph and no type report; the second yields both; the fourth is refused with the hyper cause `BodyMissing` at the `nope` read (`missing_declaration`) and not `NotEmitted`; S2 refuses no read with `NoDispatchEntry` or `UnrepresentedConstruct`; no second parser runs.
- Step 6: the refused unit is an error outcome of `check` with no inventory rows; each exhaustion stop yields an `Incomplete` inventory with its partial rows and the typed cause.
- Assertions compare typed causes and references, never message text.

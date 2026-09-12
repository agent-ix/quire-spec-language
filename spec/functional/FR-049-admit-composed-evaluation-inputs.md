---
id: FR-049
title: "Admit immutable composed evaluation inputs"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-002, type: implements }
  - { target: ix://agent-ix/quire-spec-language/US-003, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-040, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: references }
---
# FR-049: Admit immutable composed evaluation inputs

## Description

When a caller evaluates a value from an admitted composed artifact, the
compiler library SHALL validate an explicit immutable typed state view before
executing the selected value graph.

This requirement supplies the shared runtime-input seam needed by
[FR-046](FR-046-execute-predicates-and-ordered-queries.md) and
[FR-047](FR-047-evaluate-finite-object-reference-graphs.md). It does not create
an observation store, protocol executor, model producer or alternate source
representation.

## Inputs

The public Rust entry point takes an `AdmittedPackage`, an `EvaluationRequest`,
a borrowed `StateView` and caller-lowered `Limits`. The request selects one
declaration and one declaration-local value handle. The state view contains
explicit binder values and finite population/object views selected by their
declaration-local binding requirements.

D's Producer interface 1.2.0 at filament-core-data revision
`6259d3a5b99088740df9bcc8e8d60f3720aaa603` supplies model, object-type,
relationship, population-declaration, configuration and
producer/native-correspondence identity. F's selected observation binding and
assessment-input contract at the accepted native-v1 baseline
`4d6230eb8aa9766ff3017360962f2d6368d74cb3` supplies concrete records,
occurrence correlation, membership, observation anchors, progress and
completeness. The caller translates those authorities into this typed Rust
view; field spelling, JSON shape or the offered protocol artifact cannot appoint
them.

## Outputs

An `EvaluationReport` containing effective limits, deterministic usage and one
`EvaluationOutcome`: `Completed(Value)`, `Incomplete(MissingInput)`,
`Refused(Refusal)` or `Exhausted(Exhaustion)`. Only `Completed` contains a value.
Each non-complete outcome carries the exact selected declaration/value or input
identity and a closed typed reason; callers never classify outcomes from text.

## Behavior

The public evaluator SHALL accept only a package produced by the constructor-
private successful compiled-artifact reader.

The request validator SHALL reject an absent declaration, a foreign or
out-of-range value handle and a value whose owner differs from the selected
declaration.

The input validator SHALL bind every supplied binder and population to one exact
declaration-local compiled requirement and reject duplicate or surplus bindings.

The input validator SHALL validate every available Boolean, exact integer,
reduced rational, text, enum, record, option, ordered sequence, reference and
object value against its compiled nominal type, unit, bounds, field exports and
declared maximum before evaluation.

Every supplied value position SHALL be an explicit typed slot containing either
an `Available` payload or one `Unavailable` cause. Aggregate payloads SHALL
retain their child slots recursively in authored order, so an ordered sequence
or record can carry an unavailable child without inventing that child's value.

An `Unavailable` slot SHALL retain one typed missing observation, field, member,
membership, sequence, population or closure identity without constructing a
placeholder semantic value.

The input validator SHALL permit an unavailable value at a position whose
compiled expected type and authority are known, so evaluation can preserve
short-circuit behavior without inventing its payload.

The population validator SHALL use the complete storage key `(anchor, model,
universe, object type, object identifier)` and reject two entries with the same
key.

The population validator SHALL permit the same model/universe/type/object
identity under distinct pre, post or current anchors as distinct storage entries.

The identity comparator SHALL project away only the observation anchor when the
language explicitly compares pre/post object identity; field reads and graph
traversal SHALL retain the full storage key.

If a population is declared complete, then the population validator SHALL
reject a reference target absent from its exact full storage-key domain.

If population membership or closure is unavailable, then the input validator SHALL
retain `Incomplete` rather than reinterpret the offered subset as a complete
empty or closed view.

The evaluator SHALL validate request and structural/authority invariants before
executing the selected value, then preserve the first runtime incomplete,
refused or exhausted outcome encountered under the selected operator order.

The evaluator SHALL borrow the admitted package and state view without mutating
them or retaining mutable process-global evaluation state.

The evaluator SHALL apply the versioned charge-before-work rules in
[NFR-009](../non-functional/NFR-009-bound-composed-evaluation.md) and start every
call with fresh counters.

The public API SHALL expose no entry point that evaluates freely constructed
wire records, reparsed expression strings or unauthenticated payload bytes.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-049-AC-1 | Only an `AdmittedPackage` plus an exact declaration-local value selection reaches evaluation; raw bytes, freely constructed wire packages and crossed/out-of-range handles cannot. | Test (TC-136) |
| FR-049-AC-2 | Binder and population inputs retain exact declaration, compiled requirement, model, type, unit, anchor and authority; duplicate, surplus, foreign and wrong-type entries refuse independently. | Test (TC-136) |
| FR-049-AC-3 | Every available payload and recursively typed aggregate slot satisfies the exact compiled Boolean/numeric/text/enum/record/option/sequence/reference/object domain without implicit conversion, normalization or duplicate coalescing. | Test (TC-136) |
| FR-049-AC-4 | Unavailable values preserve typed missing identities and participate only if evaluation reaches them; they never become null, false, zero, empty or a completed output member. | Test (TC-136) |
| FR-049-AC-5 | Population storage rejects a duplicate full key, permits the same logical object under distinct anchors, compares permitted pre/post identity without retagging reads, and distinguishes missing complete-domain targets from unavailable closure. | Test (TC-136) |
| FR-049-AC-6 | Reports expose one closed completed/incomplete/refused/exhausted outcome plus exact effective limits and usage; diagnostic text is not an outcome discriminator. | Test (TC-136, TC-137) |
| FR-049-AC-7 | Every accounting dimension admits zero/no-work, exact and one-short boundaries under `quire.state.evaluation-work/1`, with charge-before-work and no partial completed result. | Test (TC-137) |
| FR-049-AC-8 | Repeating evaluation over the same borrowed package/view with different limits starts fresh counters, preserves inputs and returns the same result when both runs have sufficient limits. | Test (TC-137) |

## Dependencies

[FR-036](FR-036-link-composed-native-packages.md) and
[FR-040](FR-040-check-composed-values.md) own the static declaration/type graph.
The already implemented artifact-core portions of
[FR-042](FR-042-publish-compiled-protocol-artifacts.md)—canonical schema,
constructor-private emission, strict reader and bounded admitted package—supply
the input authority; FR-042's later family-completeness and consumer-handoff
criteria do not precede this requirement. Producer and observation systems
retain their own identities and meanings; this interface checks selected values
against the compiled requirements they supply.

The revision pins above select semantic producer and observation contracts, not
a crate dependency or copied wire representation. A revision change must be
reconciled explicitly before its values are admitted through `StateView`.

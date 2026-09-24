---
id: FR-051
title: "Publish checked native predicate and temporal handoffs"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-040, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-090, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-092, type: depends_on }
  - { target: ix://agent-ix/quire-contract-ir/FR-025, type: supports }
  - { target: ix://agent-ix/quire-contract-ir/FR-026, type: supports }
  - { target: ix://agent-ix/tl-syntax/IF-001, type: implements }
  - { target: ix://agent-ix/tl-syntax/VO-003, type: implements }
---
# FR-051: Publish checked native predicate and temporal handoffs

## Description

When a consumer requests an immutable native definition handoff, the compiler SHALL derive either `quire.checked-predicate/v1` or `quire.checked-temporal-subject/v1` from the exact admitted `quire.compiled-protocol/2` package and return canonical bounded bytes plus a constructor-private validated view.

## Contract set and public API

The owner SHALL publish immutable schema bytes and lowercase SHA-256 digests for
both contracts. The Rust API SHALL expose:

```text
protocol_artifact::checked_predicate::{SCHEMA_BYTES, SCHEMA_SHA256, Limits,
  derive(&v2::AdmittedPackage, ClauseSelection, Limits) -> Report<Document>,
  read(bytes, &v2::AdmittedPackage, ClauseSelection, Limits)
    -> Report<ValidatedCheckedPredicate>}
protocol_artifact::temporal_subject::{SCHEMA_BYTES, SCHEMA_SHA256, Limits,
  derive(&v2::AdmittedPackage, DeclarationSelection, Limits) -> Report<Document>,
  read(bytes, &v2::AdmittedPackage, DeclarationSelection, Limits)
    -> Report<ValidatedTemporalSubject>}
```

`Document` exposes immutable canonical bytes, their raw `ByteDigest`, and a
content identity. Validated views expose borrowed/owned immutable fields with no
public constructor and no evaluation callback.

## Common closed envelope

Both documents contain exactly, in canonical field order: `contract`,
`identity`, `package`, `subject`, `source`, `clause`, `expression`, `bindings`,
`type`, `profiles`, and `limits`. `package` retains the exact v2 artifact digest
and native static identity. `subject` retains the parent and leaf subject kinds
and identities. Source/clause/expression retain exact artifact references,
qualified authored identities and half-open UTF-8 byte spans. `bindings` is a
sorted distinct bounded population of exact model, declaration, expression,
runtime-requirement and capture identities. `type` is exactly the admitted
Boolean type identity. `profiles` selects the exact native evaluation and
definedness profiles. `limits` records the effective construction/read limits
and measured use.

The content identity is the lowercase hex SHA-256 over the contract-specific
domain label's byte length as a big-endian `u64`, the label, and the RFC 8785
encoding (ADR-013 §2, `quire-canonical`) of the document with `identity`
omitted.
It is not the package identity, source digest, expression identity or TL signal
identity. Canonical bytes are unique: object fields use the specified order,
sets are sorted and distinct, numbers use the existing exact-number encoding,
and no insignificant alternative encoding is accepted by `read`.

## Predicate document

The predicate document is admitted only for a constructor-private checked
Boolean `holds(expr)` leaf, the exact optional activation guard of a temporal
declaration, or a complete Boolean clause leaf. An activation guard remains
distinct from the formula's reachable `holds(expr)` population. It additionally
retains leaf kind, owning clause, the exact checked expression node, ordered
argument/binder identities, required valuation bindings and the complete static
definedness obligations discharged or left as explicit runtime requirements.
It asserts no runtime truth and carries no result vocabulary.

## Temporal-subject document

The temporal document is admitted only for a checked temporal declaration from
the v2 package. It additionally retains activation subject and trigger/capture
requirements, the checked root expression, the exact FR-050 definition/profile/
clock selection, inclusive intervals, operator population, required history,
evaluation anchor and the sorted checked predicate leaves reachable from the
root. Its history boundary kind is exactly `execution-origin` or
`history-cutoff` under `quire-specification/FR-293`. It asserts no observation,
progress, closure, completeness or temporal result.

## Admission, reading, and limits

Derivation and reading SHALL revalidate every field against the independently
admitted package and exact selection. The reader SHALL reject unknown,
duplicate, missing or out-of-order fields; trailing data; noncanonical bytes;
unknown contracts; invalid UTF-8 or spans; unsorted/duplicate populations;
non-Boolean leaves; wrong parent/leaf/profile/clock/capture bindings; identity or
digest mismatch; and any field not derivable from the admitted package.

Limits independently bound input bytes, output bytes, JSON depth, string bytes,
population entries, expression depth and total visited fields. Work is charged
before retention or traversal; exhaustion returns one resource-incomplete report
and no partial document or validated view. Caller limits may lower but not raise
the owner maxima, and a retry starts with fresh accounting.

No input field or flag can assert that a document is checked, trusted, total or
owner-produced. No source parser, evaluator, plugin, trait object, callback,
Contract-IR vocabulary, TL proposition value or Boolean coercion participates.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-051-AC-1 | Real admitted v2 predicate and temporal declarations derive deterministic canonical documents whose public readers return constructor-private views bound to the same package and selection. | Test (TC-139) |
| FR-051-AC-2 | Every common and kind-specific semantic field is independently omitted, duplicated, reordered, cross-wired and mutated; each mutation refuses with no partial view. | Test (TC-139) |
| FR-051-AC-3 | Unknown fields/contracts, trailing bytes, noncanonical encodings, invalid spans, duplicate/unbounded populations and exact-limit-plus-one inputs refuse before excess state is retained. | Test (TC-139) |
| FR-051-AC-4 | Independent schema and identity digest vectors match the producer, and same identity on unequal canonical bytes refuses. | Test (TC-139) |
| FR-051-AC-5 | A textual or externally built expression, self-asserted trust/total flag, non-Boolean leaf, evaluator callback, and copied consumer vocabulary cannot construct either validated view. | Test (TC-139) |
| FR-051-AC-6 | The temporal view preserves exact activation, clock, inclusive interval, history kind and reachable formula predicate identities without asserting observation or result state; the predicate view admits the exact optional temporal activation guard separately and asserts no truth. | Test (TC-139) |

## Ownership

This compiler owns checked native definition handoffs only. Quire Observation
owns runtime observation/progress/completeness/availability artifacts; Quire
Protocol owns canonical protocol results and mappings; TL crates own TL
artifacts/results; Contract IR owns deterministic correspondence and decisions.

## Dependencies

FR-040 supplies checked composed values; FR-042 and FR-050 supply the admitted
compiled-protocol package and temporal binding. The shared semantic objects are
owned by `quire-specification`; downstream projection and results remain owned
by Contract IR, Quire Observation, Quire Protocol and the TL crates. QSL pins
the cycle-free `quire-contract-model` package under its existing dependency
key; it does not depend on the compatibility bridge package that consumes these
handoffs.

## Status

Implemented for `quire-spec-language#90` under the reviewed `tl-syntax#52`
ecosystem architecture; temporal activation-guard selection is extended by
`quire-spec-language#98`.

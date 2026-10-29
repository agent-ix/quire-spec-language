---
id: TC-136
title: "Admit checked-term inputs and preserve typed outcomes"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-049, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-110, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-046, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-047, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-106, type: references }
---
# TC-136: Admit checked-term inputs and preserve typed outcomes

## Description

Scope: FR-049-AC-1 to FR-049-AC-6 and FR-049-AC-10 to FR-049-AC-11.

Verify the replacement boundary for the retired native `state` evaluator:
checked expression identity, typed environment and observation admission,
graph identity, authority selection and typed outcomes. It also verifies the
replacement for each former authority source through that source's owning
admission path. The test invokes the current checked-term/family evaluator and
never constructs the retired `EvaluationRequest`, `StateView`,
`StaticAuthority` or artifact-value graph.

## Test Procedure

Build a checked package through the supported source pipeline and evaluate an
exact selected expression through its family entry point. Exercise one Boolean,
exact number, text, enum, record, option, ordered sequence, reference and object
value, including nested values. For each kind, mutate its checked type, unit,
bounds or exported member independently; offer a foreign expression handle and
an out-of-range handle. Attempt to enter using raw bytes and freely constructed
wire records without the checked package/session.

For an expression requiring model or observation input, use the checked clause
environment and selected FR-056/observation authority. Supply a duplicate,
surplus, foreign and wrong-type binder separately. For object inputs, offer a
duplicate complete storage key, then the same logical object at distinct
pre/post observation occurrences. Compare identity across observations while
performing a field read under each original full occurrence. With complete
membership omit a referenced object; repeat with membership or closure
unavailable.

Exercise the four former `StaticAuthority` roles at their current owners:
(1) compile the source document through FR-036 and admit the model fixture
`{"objects": ["Order", "Line"], "id": "orders-model"}` as the selected
FR-056 `ModelSelection`, using its digest computed from the parsed package;
(2) select `quire.value.complete/v1` by a source profile and verify FR-110's
resolved identity in the checked package; (3) change only compile resource
configuration and verify FR-036-AC-5 retains the provenance change without
changing static meaning; (4) admit an FR-106 snapshot and invocation using
their computed `sha256-jcs` digests. For each digest-owning boundary, substitute
a raw-byte digest and malformed or wrong-domain spellings where that owner
defines such a refusal, and assert its specified code, cause and input locus.
Do not invent an FR-056 profile/configuration digest check: those are
selections or provenance, not model-package digests.

At the checked-evaluation boundary, attempt the former document, model,
profile and configuration digest pairs, including the old sample document
`{ "version": 1, "name": "orders" }`, profile
`{"limits": {"depth": 8}, "profile": "strict"}` and configuration
`{"enabled": true, "mode": "batch"}` values, as independent runtime
authority arguments in a compile-fail public-API fixture. Verify the checked
entry point has no such input and cannot reach evaluation by substituting a
digest. The old per-member
`StaticAuthorityDigest` wrong-domain/malformed refusal order is explicitly
retired with that input type; it is not an oracle for the FR-056/FR-106 owners.

For each supplied aggregate, make one nested member unavailable. Evaluate once
with a reached missing member and once where ordered short-circuiting skips the
same member. Observe typed results and ensure no fabricated payload is returned.

## Expected Results

Only the exact checked expression enters evaluation; raw bytes, free wire
records, foreign handles and each single-axis type/authority mutation are
rejected at the owning admission boundary. Duplicate full storage keys refuse;
distinct observation occurrences remain distinct storage entries; identity
comparison projects only the occurrence when the checked operator permits it,
while reads retain the full key. A missing target in a complete domain is
refused, whereas unavailable membership/closure remains incomplete. Reached
unavailable values retain their typed cause, skipped unavailable values do not
change a decisive result, and no incomplete/refused value is exposed as a
completed result. Model and observation digest mutations retain FR-056/FR-106's
typed refusal and exact locus; profile resolution and compile configuration
retain FR-110/FR-036 behavior. No checked-evaluation request accepts the four
legacy independent digest pairs or produces `StaticAuthorityDigest`.

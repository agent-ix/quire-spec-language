---
id: SR-1950002
title: gap-analysis review of QSL-42
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-42; tests/it/unbounded_declarations.rs; qsl-eval/tests/it/state_model_family.rs;
  FR-334; TC-844
review_set: subset
---

## Summary

Frozen candidate: 43e116962ffbe1b166b6c4402b7cfcd0f32ff67d. Ticket: QSL-42. No branch writes. No applicable AssuranceProfile exists under spec/. Review selection: subset (base and ears-conformance); no structural, object or dependency change requires another lens. Criterion-strength/Jev and the remaining unrelated optional lenses were not selected. This is a changed-surface review, not repository-wide assurance. FR-334-AC-4 is unchanged, unexecuted, and explicitly retained in QSL-1368 blocked by STD-610. CG negotiation/warning delivery belongs to FR-076 and integration; FR-335 permits fixture dispositions here. No SMT installation or downstream run is required for this delta.

## Verdict

**PASS** — No defects found in the reviewed delta.

## Visibility

GitHub repository visibility is PUBLIC. The old sync-thread claim is not treated as instructions or proof of current outward comment sync: the top-level Linear ADR-014 comment 9207e1b1 is absent from GitHub issue 189 comments, while the old GitHub-authored proof-backend comment is present on both. Findings are posted only as new top-level Linear comments; no findings go to GitHub.

## Review Evidence

Plan completion: not assessed

Scoped manual criterion-to-trace reconciliation replaces a whole-repository matrix invocation because the dispatch explicitly prohibits the known hanging matrix. New/modified assertions bind FR-334-AC-1/2/3 and FR-335-AC-3 correctly. The root/eval files retain FR-333 and FR-335 adjacent tests. No production behavior was introduced and no reverse ownership gap or hollow new evidence was found. This PASS applies only to the reviewed delta; AC-4 has no completed in-place vector test and remains deferred as described above. No aggregate or all-criteria completion claim. Independent checks: cargo fmt --check exits 0; focused Clippy for root and qsl-eval integration targets with -D warnings exits 0; root unbounded_declarations 8/8 and qsl-eval state_model_family 12/12 pass. Logs: /tmp/qsl42-review-{root,eval,clippy,spec,deny}.log. Root linker emits the existing __eh_frame >16MB warning. cargo deny check exits 4 with 255 rejected license requirements: advisories/bans/sources pass, licenses fail. Cargo manifests, Cargo.lock and deny.toml are identical to origin/main, so this dependency gate failure is outside the three-file delta; it is not claimed green. Whole-workspace tests/matrix were not run per dispatch; aggregate verification belongs to the lead. Quire 0.37.1 engine 0.50.3 / quoin 0.29.1 validation exits 0 with duplicate-archetype/inverse-edge and inline-data-schema module warnings.

## Scope

```yaml
scope:
- id: FR-334-AC-1
  path: spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: examined
  excerpt: 'A unit whose only difference from another is `s: Set<Integer>` against
    `s: Set<Integer>[0, 18446744073709551615]` emits a different structural node key
    for `s`''s type and a structurally unequal package value; the unbounded node has
    no `collection_bounds` member and the bounded node has one with `min` 0 and `max`
    18446744073709551615.'
- id: FR-334-AC-2
  path: spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: examined
  excerpt: '`p: Population<Account>` lowers to a population node with no maximum,
    whose node key differs from those of `Population<Account>[4]` and of an unbounded
    `Set<Reference<Account>>`; it lowers with no refusal.'
- id: FR-334-AC-3
  path: spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: examined
  excerpt: Compiling the same bounded unit twice gives the same structural node keys
    and structurally equal emitted package values.
- id: FR-335-AC-1
  path: spec/functional/FR-335-settle-a-claim-over-an-unbounded-declaration.md
  role: examined
  excerpt: Compiled from source and requested with an empty registry, `f`'s `+` claim
    settles `unsupported` with a warning naming `value-validity`; its record is (`value-validity`,
    `Unbounded`) with one collection domain at `s`.
- id: FR-335-AC-2
  path: spec/functional/FR-335-settle-a-claim-over-an-unbounded-declaration.md
  role: examined
  excerpt: The claim is written with `finite_bound_available` true; with only a test
    descriptor advertising (`value-validity`, `bounded`) and a fixture `requires-bound`
    disposition it settles `requires-bound`, and no harness is emitted.
- id: FR-335-AC-3
  path: spec/functional/FR-335-settle-a-claim-over-an-unbounded-declaration.md
  role: examined
  excerpt: 'A bounded request with `ProofBound{s, Cardinality{maximum: 8}}` writes
    a second item with its own request index, classified `bounded`, whose obligation
    identity differs from the unbounded item''s and from that of a request with `maximum:
    9`; the unbounded item still settles `requires-bound`.'
- id: FR-335-AC-4
  path: spec/functional/FR-335-settle-a-claim-over-an-unbounded-declaration.md
  role: examined
  excerpt: 'A requirement built over the collection domain at `s` and a domain of
    kind quantity (no source spelling yet types a parameter by a quantity, so the
    test builds the requirement directly) is written with `finite_bound_available`
    false, and its `unbounded-extent` disposition (a fixture) gets no routing target:
    `unsupported`, warned.'
- id: FR-333-AC-1
  path: spec/functional/FR-333-read-an-optional-collection-bound-and-population-maximum.md
  role: examined
  excerpt: '`function f using v(s: Set<Integer>): Integer pure { 0 }` compiles through
    the spine, and `s`''s resolved type is a `Set` with element `Int` and `bound:
    None`; with `Set<Integer>[0, 8]` the bound is `Some{0, 8}`, and the two parameters''
    types are unequal. Each of `Sequence`, `Bag` and `OrderedSet` with no bound resolves
    to `bound: None`.'
- id: FR-333-AC-2
  path: spec/functional/FR-333-read-an-optional-collection-bound-and-population-maximum.md
  role: examined
  excerpt: 'A parameter `p: Population<Account>` resolves to `ValueType::Population(None)`
    and `p: Population<Account>[4]` to `Population(Some(4))`.'
- id: FR-333-AC-3
  path: spec/functional/FR-333-read-an-optional-collection-bound-and-population-maximum.md
  role: examined
  excerpt: '`Set<Integer>[3, 2]` refuses `ill_typed`/`type-mismatch` at the type form''s
    span; `Set<Integer>[0, x]` fails at S1 with a parse diagnostic at `x`''s span.'
- id: FR-334-AC-4
  path: spec/functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md
  role: context_only
  excerpt: 'Conformance: for each collection and population type vector in QSpec FR-322,
    read in place from the QSpec repository by the test, QSL''s node key equals the
    vector''s expected key; the vectors exercise collection and population types covered
    by FR-144-AC-9, AC-13 and FR-153-AC-9.'
bindings:
- test_id: a_collection_bound_adds_minimum_and_maximum_to_the_type_node
  ac_id: FR-334-AC-1
  trace: correct
- test_id: compiling_a_bounded_unit_twice_gives_the_same_identities
  ac_id: FR-334-AC-3
  trace: correct
- test_id: an_unbounded_population_has_its_own_node_and_no_maximum
  ac_id: FR-334-AC-2
  trace: correct
- test_id: a_bounded_request_is_its_own_item_distinct_per_bound
  ac_id: FR-335-AC-3
  trace: correct
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

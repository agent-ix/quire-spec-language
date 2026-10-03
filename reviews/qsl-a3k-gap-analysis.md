---
id: SR-1255
title: "A3k gap analysis of PR #611 (FR-094-AC-1, FR-094-AC-5, TC-417 step 2, TC-418 step 2, ADR-011 E3, ADR-013 O-04/QC-18/QC-25)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d21fb55865c68e4fff12aee04a0c8fe8ab571fa3; PR #611 diff against origin/main (merge base 652ae3d5); spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md; spec/test-cases/TC-417-model-reference-and-population-nodes-match-golden-vectors.md; spec/test-cases/TC-418-clause-function-nodes-carry-model-owner.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md; spec-wide check for versioned-owner text left behind: spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
---
## Summary

Ticket: A3k. The Linear id is not yet known (placeholder: `QSL-TBD-A3k`), because
the Linear keyring is locked. PR: quire-spec-language#611. This is a manual
check of each AC against its tests, using the quoin gap-analysis method with no
plan bundle, plus a spec-to-code agreement sweep for the content-only
`ModelOwner`.

Trace, per unit:

- **FR-094-AC-1 / TC-417 step 2.** The AC now says 2.0.0 gives the same M1 and
  R1, and that M1's owner is `{kind, identity, node}` with no `version`.
  `reference_types_key_over_their_model_nodes_and_record_the_correspondence`
  (`#[trace("FR-094-AC-1", "FR-094-AC-2", "FR-094-CON-2", "TC-417")]`) checks
  that M1's owner equals the content-only JSON exactly. Under
  `admitted("2.0.0")` it asserts R1 as `r`'s type and M1, R1, M3 and R5 byte
  for byte, and that M1 resolves to `Order`'s `DeclarationKey`, which is TC-417
  step 2's "same correspondence entry". The binding is correct, and it covers
  every clause of the AC.
- **FR-094-AC-5 / TC-418 step 2.** The AC now says that under 2.0.0 the
  receiver keys to the same P7, and the clause functions to the same C1 and C2.
  `a_version_only_change_keys_the_same_clause_functions`
  (`#[trace("FR-094-AC-5", "TC-418")]`) asserts R1, P7, C1 and C2 byte for
  byte under 2.0.0, C1's content-only owner, and equal graph keys across 1.0.0
  and 2.0.0. `clause_functions_key_under_their_operation_members_model_owner`
  covers TC-418 step 1's owner with no `version` member. Both bindings are
  correct.
- **TC-442 (FR-027-AC-9, FR-056-AC-9).** The new assertion checks that every
  emitted model node's id is the content-only key IR's reader recomputes, and
  the read returns `Verified`. It backs the emit side of FR-094's owner rule.
  The trace attribute is unchanged, and AC-1 is already backed by TC-417, so
  this is not a gap.
- **FR-094 Behavior ("Model-owned nodes", "Model declaration nodes", "Reference
  and Population type nodes", "Clause function nodes", "Golden vectors").**
  These agree with the code: owner `{kind, identity, node}`, version kept only
  as lock evidence in `model_selections`, and every vector the same at 2.0.0.
  The vectors are verified in SR-1254: hashes, references,
  the version-only twin mapping, and M1/M3/M4/M5 equal to QSpec's
  `model_declaration_nodes`.
- **ADR-011 E3, ADR-013 O-04 Equality row, QC-18 and QC-25.** These are
  rewritten consistently. QC-25 drops the "one (`identity`, `version`) names one
  digest" ask. That ask is moot now, because FR-094 states that the node id is
  the same for every version and digest under one identity, which is what
  QSpec FR-322-AC-28 decides. ADR-013's Conversions row (line 179) already read
  `ModelOwner{identity, node}`.
- **The code side has no gap.** No producer or consumer carries a `ModelOwner`
  version (SR-1254).

The remaining gaps are versioned-owner or deleted-vector references that the
PR left behind in spec text it did not edit.

## Verdict

Changes requested, for spec text only. FND-001 (medium) leaves ADR-012 saying
that a model-owned node's owner includes the declared version, which
contradicts ADR-013 O-04, FR-094 and the code. FND-002 and FND-003 (low) are
stale vector ranges and a stale "QSL's proposal" claim. All three are text
edits inside this PR. No AC is untested.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-012 §2's Identity row still says the O-04 preimage's owner subject is, "for a model-owned node the domain package's identity and declared version plus the IR node identity". After this PR, ADR-013 O-04 (Equality row), QC-18, ADR-011 E3, FR-094 and `node_key::ModelOwner` all say identity plus IR node, with no version (QSpec FR-322-AC-28). So two ADRs now contradict each other on the preimage owner, and ADR-012 contradicts the code. Fix: change that clause to "for a model-owned node the domain package's identity plus the IR node identity, with no version (QSpec FR-322-AC-28)". | spec/decisions/ADR-012-semantic-family-extension-contracts.md:235 |
| FND-002 | low | ADR-013 QC-3 still reads "FR-094's vectors M1 to M4 and C1 to C4 are QSL's proposal for them". The ranges include M2 and C3, which this PR deletes. Also, QSpec now publishes M1, M3, M4 and M5 byte-identically (`model-member-type-vectors.json` `model_declaration_nodes`), so QC-3 is answered for model declaration nodes. That equality, which the PR claims, is recorded nowhere in the spec. Fix: say that M1, M3, M4 and M5 equal QSpec's published `model_declaration_nodes` vectors, and that the clause-function vectors C1, C2 and C4 to C6 remain QSL's proposal (QC-25). | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1124 |
| FND-003 | low | The FR-094 paragraph in spec/tests.md lists "vectors M1 to M5, R1 to R5, ... C1 to C6". Each range now includes a vector this PR deletes (M2, R2, C3). Fix: list "M1, M3 to M5, R1, R3 to R5, S1 to S3, PO1 to PO3, P5 to P8, L4, E4 to E10, C1, C2, C4 to C6 and U1 to U4". | spec/tests.md:888-889 |

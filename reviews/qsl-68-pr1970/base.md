---
id: SR-2028
title: "Base review of QSL-68 identity and provenance contracts"
type: SpecReview
analysis: base
scope: "spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md, spec/test-cases/TC-1023-model-identity-chain-is-unique-and-binds-ancestry.md; QSpec FR-150 context"
review_set: all
---

## Summary

The revised identity pair and the separate derivation provenance now align with QSpec FR-150, and the changed contract documents validate. The full-size local criteria FR-081-AC-12 and AC-13 remain outside the local QSL matrix because their only test uses QSpec-prefixed tags for local artifacts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-081-AC-12 and FR-081-AC-13 are not bound to local QSL evidence: `model_identity_full_size::ten_thousand_edge_chain` is tagged only with `QSpec-TC-195` and `QSpec-FR-081-AC-12/13`, while these criteria and their test case are local QSL artifacts (`FR-081-AC-12/13`, `TC-1025`). Add the local trace IDs so the full-size test appears in QSL's computed matrix; retain only upstream tags that resolve to actual QSpec artifacts. | FR-081-AC-12, FR-081-AC-13, TC-1025 |
## Scope

- **FR-081-AC-12** (examined; `spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md`): For a deterministic single-inheritance chain of 10,000 edges (10,001 effective types) run on a thread with a 524,288-byte (512 KiB) stack, normalization at the NFR-012 default limits eagerly computes every effective identity and the complete view with 20,001 retained facts.
- **FR-081-AC-13** (examined; `spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md`): The FR-081-AC-12 case, run in an isolated process, has a peak resident set of at most 21,474,836,480 bytes and a wall time of at most 30 seconds in the release profile on the reference host Terra.
- **TC-1023** (context_only; `spec/test-cases/TC-1023-model-identity-chain-is-unique-and-binds-ancestry.md`): TC-1023 verifies FR-081-AC-10 identity and provenance changes.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The full-size test now carries local `TC-1025`, `FR-081-AC-12`, and `FR-081-AC-13` trace tags. Scoped Quire validation passes with these references. |

## Current status

The original FND-001 is fixed on PR head `cf7b077e942d35475b2b88e812471b913276b5d0`; no unresolved finding remains in this method.

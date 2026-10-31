---
id: TC-149
title: "Preserve opaque trigger identity on the checked-package route"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-053, type: verifies }
---
# TC-149: Preserve opaque trigger identity on the checked-package route

## Description

Verify [FR-053-AC-7 through FR-053-AC-9](../functional/FR-053-preserve-opaque-semantic-trigger-identity.md)
where the Layer-4 checked-package route accepts semantic-trigger identities.
The observation owner supplies the expected opaque identities; QSL does not
authenticate that owner or construct QProtocol's FR-479 binding.

## Test Procedure

Evaluate event-triggered temporal obligations with two distinct identities
having otherwise equal timestamp, receipt, payload and captures; deliver one
identity twice; and change one identity byte while holding other fields fixed.
Attempt substitutions using timestamps, receipts, display text, payloads,
captures, static activation coordinates and declaration positions. Where the
route has an encoded/versioned identity representation, also submit missing,
empty, malformed, stale, mismatched and other-version values through the
applicable checked-package reader. Observe activation identities, typed
outcomes and whether temporal evaluation begins.

## Expected Results

Distinct identities remain distinct and a duplicate delivery does not create
a second instance. A one-byte mutation changes identity or refuses. No listed
non-identity field substitutes for semantic-trigger identity. Invalid or
cross-version representations yield typed refusal/incomplete outcomes, do not
produce aliased or partial activation, and are never translated to the
selected version. Where the route has no encoded/versioned representation,
record that case as not applicable rather than inventing one.

---
id: TC-237
title: "A derivation conflict with no descendant redefiner exposes no effective member for either path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: verifies
---
# TC-237: A derivation conflict with no descendant redefiner exposes no effective member for either path

## Description

Verify that when two declared redefinitions of one member reach the same
effective type and neither redefining owner is a proper descendant of the
other, normalization refuses derivation-conflict naming both redefiners and
exposes no effective member for either — distinct from FR-081-AC-3's
descendant case, where a genuine winner exists and the loser is retained
for provenance rather than refused. Scope: FR-081-AC-6.

Catches an implementation that, on encountering two undominated redefiners,
picks one arbitrarily (for example, by source order or by which redefiner
its internal map happened to visit last) instead of refusing the whole
member — a defect invisible to a test that only exercises the
proper-descendant case, where a real winner always exists.

## Test Procedure

1. Admit a domain package with a diamond shape: object type `A` declares
   field `x`; object types `B` and `C` (`supertypes: [A]`) each
   independently redefine `x`; object type `D` (`supertypes: [B, C]`)
   inherits both redefinitions with neither `B` nor `C` a descendant of the
   other.
2. Run the model binder's normalization.
3. Inspect the outcome.
4. Attempt to read the effective `x` member the correspondence would
   resolve for `D`'s effective type.

## Expected Results

Step 3 is a `Refused` outcome with a derivation-conflict cause naming `D`,
member `x`, and both redefiners (`B`'s and `C`'s). Step 4 finds no effective
member for `x` under `D` — not `B`'s redefinition, not `C`'s, and not an
arbitrarily chosen one. A mutant that resolves the conflict by picking the
first-encountered redefiner exposes a real effective member in step 4,
failing the no-chosen-member assertion.

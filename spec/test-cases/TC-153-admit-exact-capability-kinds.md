---
id: TC-153
title: "Admit exactly the six capability kinds and refuse every other label"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: verifies
---
# TC-153: Admit exactly the six capability kinds and refuse every other label

## Description

Verify that the canonical `Capability` value type admits exactly the six
quire-specification FR-290 labels, compares kinds by label, and refuses an
absent or unknown label without mapping it. Scope: FR-057-AC-1, FR-057-AC-2 and
FR-057-AC-4.

## Test Procedure

1. Read the six labels from the vendored FR-290 Values table. Parse each one,
   serialize the result, and compare the admitted set with the table's set.
2. Submit a requested pair with no label.
3. Submit, one at a time: `Global-Conformance`, `global_conformance`,
   `Global conformance`, ` composition`, `GlobalConformance`, `FamilyCheck`,
   `StateOperation`, `FiniteReplay` and `TemporalProjection`.
4. Build two sets holding the same kinds in different orders and compare them.
   Then permute three requested pairs over different kinds.

## Expected Results

- Step 1: every label round-trips to byte-identical JSON, and the admitted set
  equals the FR-290 set with no extra or missing member.
- Step 2: the pair is refused with `invalid_capability`/`absent-kind`, and no
  kind is defaulted.
- Step 3: each pair is refused with `invalid_capability`/`unknown-kind`, naming
  the exact received bytes, with no replacement kind.
- Step 4: the sets compare equal. Each permuted pair keeps its own kind, and
  only the request indices change.
- Assertions compare typed codes, causes and received bytes, never message
  text.

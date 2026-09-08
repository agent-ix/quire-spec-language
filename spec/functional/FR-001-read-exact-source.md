---
id: FR-001
title: "Read bounded immutable native source"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-001"
    type: traces_to
---
# FR-001: Read bounded immutable native source

## Description

When source is admitted, the compiler shall retain its exact bytes and identity metadata.

## Inputs

Source identity, opaque revision, path, UTF-8 bytes and caller ceilings.

## Outputs

Immutable source with digest/indexed locations or a diagnostic.

## Behavior

The input ceiling is 1 MiB and can be lowered. Invalid UTF-8, BOM and NUL refuse. Verified intake checks the selected raw SHA-256 digest. Byte spans remain original and half open; line/scalar-column coordinates are derived without changing source text.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-001-AC-1 | Matching selected bytes produce the expected SHA-256 digest. | Test |
| FR-001-AC-2 | Changed bytes under a selected digest receive source_digest_mismatch. | Test |
| FR-001-AC-3 | Invalid UTF-8 receives a source diagnostic. | Test |
| FR-001-AC-4 | Input beyond the selected byte ceiling receives resource_exhausted. | Test |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [Detailed contract or implementation evidence](../../src/source.rs) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.

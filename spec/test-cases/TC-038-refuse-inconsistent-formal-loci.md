---
id: TC-038
title: "Refuse structurally valid but false formal coordinates"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-014
    type: verifies
---
# TC-038: Refuse structurally valid but false formal coordinates

## Description

Integration, P1; verifies FR-014-AC-4 through actual IR SourceSpan constructors.

## Test Procedure

Construct valid IR spans that separately alter formal document, formal revision,
start line, start column, end line or end column. Include points within a
multibyte scalar, just beyond EOF and at u64::MAX. Include a valid empty point
and an exact multi-line span as positive controls after each refusal.

## Expected Results

False coordinates refuse with invalid_source_map/source_map and bound-native
byte-zero diagnostics. All supplied IR constructors succeed; a constructor
failure cannot substitute for exercising the bridge. Exact controls map back
to their original native spans after each adverse request.

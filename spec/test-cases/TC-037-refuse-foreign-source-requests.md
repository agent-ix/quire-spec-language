---
id: TC-037
title: "Refuse foreign native source requests"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-014
    type: verifies
---
# TC-037: Refuse foreign native source requests

## Description

Integration, P1; verifies FR-014-AC-3 at the native-to-IR trust boundary.

## Test Procedure

Bind one Source. Reload four requests, each changing exactly one of its native
identity, revision, path or same-length bytes. Map a valid span from each request
and then map the original span again. Also reload an independently allocated
Source with all four values identical and map the same span.

## Expected Results

Each changed request refuses with invalid_source_map/source_map at byte zero of
the bound source, retaining its original labels, no related declarations and no
upstream diagnostic. The original and independently reloaded identical requests
produce equal IR spans after every refusal.

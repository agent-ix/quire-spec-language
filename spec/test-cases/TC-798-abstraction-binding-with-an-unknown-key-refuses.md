---
id: TC-798
title: "An abstraction binding whose model key resolves to nothing refuses missing-name"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: verifies
---
# TC-798: An abstraction binding whose model key resolves to nothing refuses missing-name

## Description

Scope: FR-304-AC-3.

## Test Procedure

1. Check a relation binding an object type key absent from the domain
   package.
2. Check a relation binding `OperationKey { ConfigVersion, rename }`, an
   operation ConfigVersion does not declare.

## Expected Results

Each refuses `missing_declaration`/`missing-name`, naming the key and the
ConfigVersion `DomainPackageRef` identity.

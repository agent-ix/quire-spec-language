---
id: TC-764
title: "Certificate checkers accept genuine certificates and reject altered or misbound ones in a core-only process"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: verifies
---
# TC-764: Certificate checkers accept genuine certificates and reject altered or misbound ones in a core-only process

## Description

Verify the layer-6 certificate checkers on certificates `analyze` writes, run in a test binary that links only qualified-core crates.

Scope: FR-282-AC-1 to FR-282-AC-3 and FR-282-AC-5 to FR-282-AC-8.

## Test Procedure

1. Write TC-762 step 1's EN-5 certificate to bytes. In a separate test binary whose dependencies are only qualified-core crates, pass the bytes and the byte provision to the EN-5 checker.
2. Repeat with one probability bound in the certificate changed, then with its `package_id` replaced by another fixture package's.
3. Run ADR-026's zone search over a dense-time fixture model, write its certificate, and pass it to the zone checker; repeat with one zone removed.
4. Prove an invariant over a finite fixture model with EN-1 and pass its closure certificate to the EN-1 closure checker; repeat with one reachable product state removed.
5. Prove `always eventually q` under `fair weak inc` over a finite fixture model with EN-1 and pass its component certificate to the SCC and ranking checker; repeat with two components swapped out of topological order.
6. Check a refinement that holds and pass its simulation-relation certificate to the simulation-relation checker; repeat with one pair removed from the relation.
7. Prove a two-copy hyperproperty over a finite fixture model with EN-1 and pass its product-closure certificate to the product-closure checker; repeat with one product-state key removed.

Tag the tests `#[trace("TC-764", "<AC id>")]`.

## Expected Results

- Step 1: the checker accepts.
- Step 2: the first is rejected with a cause naming the failing obligation; the second with a cause naming both `package_id`s.
- Step 3: the genuine certificate is accepted; the altered one is rejected.
- Step 4: accepted; the altered certificate is rejected naming the missing successor.
- Step 5: accepted; the altered certificate is rejected naming the backward edge.
- Step 6: accepted; the altered certificate is rejected naming the unmatched concrete step.
- Step 7: accepted; the altered certificate is rejected naming the missing successor.

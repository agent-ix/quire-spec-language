---
id: IT-007
title: "Qualify native package reconstruction through runtime execution"
type: IT
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# IT-007: Qualify native package reconstruction through runtime execution

## Objective

Establish that a byte-selected native package reconstructs the real checked
source/model/obligation inventory and supports the existing runtime workflow
without trusting serialized semantic claims.

## Target Integration

The compiler's package boundary, actual native parse/link/check/runtime APIs,
the pinned Contract IR public declaration/proof API and the qualified
source-derived Rust rule-model fixture. This is real in-process library
integration; no service, browser, model-producer process or mocked interpreter
is introduced. B/C's independent consumers and the executable backend remain
separate acceptance work.

## Preconditions

Record source revision, Rust 1.98.1, IR 690bde7, adopted standard e897f81 and
all source/model/package/input digests. The new package specification and all
eight selected QUOIN reviews must pass before implementation. IT-006's existing
native pipeline is qualified. A missing package implementation leaves this
test planned; a setup failure cannot be substituted for a package refusal.

## Inputs

Healthy/violating parent and aggregate clauses, complete current snapshots,
pre/post clauses with a captured parameter resolving a deleted object and a
post result, and dangling/stale/incomplete/frame/budget adverse controls.
Use independently authored expected truth, event sequences and exact costs.
External CheckBindings and the admitted model inventory are selected separately
from the serialized package; wire values cannot supply their authority.

## Test Procedure

1. Admit the source-derived model and parse/link/check all selected clauses.
   - IT-007-SC-01: all setup stages succeed with exact authored/native/formal/model bindings.
2. Build the immutable package and independently inspect its complete manifest.
   - IT-007-SC-02: every clause, dependency, occurrence, runtime obligation and projection disposition is retained with exact byte identity and the independently expected native static identity.
3. Drop the original package and reconstruct from saved bytes and explicit external bindings.
   - IT-007-SC-03: actual parse/link/check executes and produces a fresh checked package matching the selected static inputs.
4. Validate and evaluate healthy and violating current snapshots through that reconstructed package.
   - IT-007-SC-04: independent expected truth, source lineage, event prefixes and expression/graph counts are observed.
5. Validate/evaluate pre/post invocation, deleted capture and false-result cases.
   - IT-007-SC-05: frame/delta validation precedes the correct pre/parameter/post/result observations.
6. Exercise dangling, incomplete, malformed-frame and exhausted evaluation controls, then retry.
   - IT-007-SC-06: refused/incomplete outcomes retain actual stage/usage/provenance with no Boolean; retries have fresh budgets.
7. Mutate serialized claims and raw wire while retaining valid unrelated setup and updating byte selectors where needed.
   - IT-007-SC-07: strict intake or fresh compiler reconstruction refuses the intended defect with no accepted package.
8. Record commands/revisions and inspect retained unlowered obligations.
   - IT-007-SC-08: no unsupported IR clause is dropped or relabeled executable; native identity is qualified against its own vectors, while independent shared-domain adoption and backend/portable acceptance retain their separate gates.

## Expected Results

All eight criteria have actual Rust observations after successful setup. The
reader is exercised independently of the original package's ownership/lifetime;
round-trip byte equality alone is insufficient. The full ConfigVersion/backend
and Quire workflow in IT-002 remains mandatory in the original assignment.

## Metadata

Priority: High. Status: planned. One Cargo job, one test thread, nice 10,
existing offline caches; no hosted dispatch or extra agents.

## Dependencies

- [IT-006](IT-006-native-reference-workflow.md).
- [FR-019](../functional/FR-019-package-checked-native-clauses.md).
- [FR-020](../functional/FR-020-read-and-rebind-native-packages.md).
- [FR-021](../functional/FR-021-derive-native-package-identity.md).
- [NFR-007](../non-functional/NFR-007-bound-native-packages.md).
- [TC-089](../test-cases/TC-089-qualify-native-package-runtime-reconstruction.md).

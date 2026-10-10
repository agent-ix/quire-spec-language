---
id: TC-918
org: agent-ix
title: "Received typed kernel invariant provenance survives fault mapping"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: references
  - target: ix://agent-ix/quire-exact/FR-369
    type: references
---
# TC-918: Received typed kernel invariant provenance survives fault mapping

## Description

Verify the received-kernel route separately from QSL-owned constructors.
Scope: FR-096-AC-19 and FR-090-AC-17; existing FR-121 fault terminal mapping
is a dependency, not a new replay-phase contract. Use the kernel's actual
typed cause set, without copying or extending its enum in QSL.

## Test Procedure

1. For every kernel CheckedInvariantCause admitted by the dependency type,
   supply a received `Refusal::CheckedInvariant { cause }` to S6a's stop
   conversion. Enumerate the causes exhaustively without a wildcard default.
   For payload-bearing causes, supply distinct valid typed IllTypedCause
   payloads and compare the retained typed values, not their display text.
2. Repeat at located and unlocated stop seams, and through public call and
   public evaluate fault propagation. Attempt kernel refusal-record creation;
   inspect the kernel code/cause accessors and the fault's provenance.
3. Send each resulting fault through replay's existing admission-fault and
   evaluation-fault paths. Inspect terminal classification, diagnostic,
   replay basis and retained typed provenance.
4. Compare meter events before and after each mapping. Pair with ordinary
   kernel refusal, Undefined and Incomplete stops and a typed family refusal.
   Include a QSL-owned `boolean-value-expected` fault as a provenance-negative
   control, without manufacturing a kernel payload for it.

## Expected Results

Every received cause becomes `Err(InternalFault)` at S6a, code
`runtime_invariant`, category internal failure, invariant
`checked-program-invariant`, with the identical typed cause (and identical
typed nested payload where present) accessible as fault provenance. Located
and unlocated stops both produce faults, not Evaluations or refusal records.
Kernel `code()` and `cause()` are absent for CheckedInvariant. A newly admitted
kernel cause requires explicit enumeration, not a fallback cause.

Public mappings retain `CallFailure::Fault` and the original stage, invariant
and typed provenance. Both replay fault paths produce Failed with an
internal-failure diagnostic and unavailable basis; neither produces an
ordinary-refusal Inconclusive terminal. Mapping adds no meter event and does
not erase earlier charges. Ordinary refusal/undefined/incomplete controls
retain their existing outcomes; an ordinary replay refusal remains
Inconclusive. The QSL-owned negative control has no synthetic kernel cause.

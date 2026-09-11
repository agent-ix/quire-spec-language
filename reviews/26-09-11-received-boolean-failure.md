---
id: SR-361
title: "Failure-domain analysis of native received-Boolean choice admission"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md; src/protocol_artifact/native/families.rs; src/protocol_artifact/native/families/decisions.rs; src/protocol_artifact/native/families/decisions/formula.rs; src/protocol_artifact/native/families/decisions/received.rs; tests/native_choice_emission.rs; tests/native_protocol_emission.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

The increment appropriately treats source provenance, lexical availability and
the finite valuation proof as admission boundaries rather than wire facts. The
main residual concern is the exponentially growing valuation traversal, whose
broader artifact-wide AC-9 vectors remain inherited debt. Capture is not a
positive source path here: protocol captures predate the run and compensation
captures are outside a choice's scope.

## Verdict

**PASS (scoped)** — no new failure-domain defect is demonstrated. The broader
per-dimension AC-9 matrix remains inherited evidence debt; this review does not
claim pending repository gates passed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No capture identity defect found: no valid source can capture a receive from the same protocol run into its choice, so let-only resolution does not drop an eligible atom. | FR-042:95-105; decisions.rs:161-168; runtime.rs:89-111 |
| FND-002 | low | Partition checking enumerates every selected visible atom; budget charging and the required twelve-atom References/locus/retry evidence are present, while complete per-dimension AC-9 vectors remain inherited broader debt. | compiled-protocol-v1.md:614-619; formula.rs:146-229; FR-042-AC-9; native_choice_emission.rs:1035-1110 |
| FND-003 | low | No additional owner/availability confusion was found: exact receiver ownership, control anchor and existing structural scope are checked before an atom becomes eligible. | FR-042:95-105; received.rs:83-125 |

### Trust and identity boundary

An atom is sound only if its receive binder, Boolean field/export, original
anchor, receiver role and causal scope agree. The implementation makes these
checks before field admission, so spelling and type coincidence cannot merge
distinct receives. Protocol captures occur before the run's receives and
compensation captures are not in choice scope. Consequently, let-only alias
traversal does not omit an eligible captured atom.

### Topology and resource failure

`partition` builds a bounded, non-recursive formula horizon and enumerates the
selected atom slots with charged visits. This avoids a fixed-width bit-mask
ceiling and returns `Incomplete` on the demonstrated reference exhaustion.
Nevertheless, the number of valuations doubles per selected atom. The contract
requires charge-before-work and a source locus, and the increment exercises the
required twelve-atom References exhaustion with locus and fresh retry.
Independent boundaries for all artifact dimensions remain useful, but their
absence is inherited AC-9 debt, not a demonstrated Boolean resource defect.

### Purity and conservative proof

The selected independent-atom abstraction is intentionally conservative:
correlated runtime data cannot turn a failing abstract partition into a concrete
overlap/hole. The implementation preserves that direction by returning
FamilyProof for an unproved symbolic partition and retaining Invalid(Control)
for fully closed overlap/hole. The focused tests cover that classification; this
review identifies no separate purity regression.

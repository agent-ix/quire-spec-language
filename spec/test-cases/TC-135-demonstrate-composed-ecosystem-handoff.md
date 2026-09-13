---
id: TC-135
title: "Demonstrate the composed compiler-to-assessment ecosystem handoff"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-048, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: references }
  - { target: ix://agent-ix/quire-protocol/IT-001, type: references }
  - { target: ix://agent-ix/quire-specification/IT-020, type: references }
  - { target: ix://agent-ix/quire-specification/IT-050, type: references }
---
# TC-135: Demonstrate the composed compiler-to-assessment ecosystem handoff

## Description

Verify the compiler's real, bounded contribution to the pinned order/payment/
fulfillment campaign scenario and define the external gate that consumes it,
without making A's local evidence claim B/F semantics.

## Test Procedure

1. Run A's release producer over the reusable native OrderFlow source containing
   split shipments, retry, refund, compensation and a temporal deadline. Select
   D's Producer interface 1.2.0 at revision
   `6259d3a5b99088740df9bcc8e8d60f3720aaa603` and merged L5 PR #70's
   FR-043/044/045 Rust-interface baseline at `72507f8`. Require the reviewed
   `quire.compiled-protocol/2` extension carrying every selected temporal
   definition identity/revision/raw-byte digest/artifact and exactly one tagged
   clock configuration; do not pass the timed subject through `/1`.
2. Record A's exact source/model/profile/configuration/correspondence selections,
   canonical artifact bytes/reference and required relationship, population,
   clock and observation-role handles. Re-run with sufficient compiler limits;
   changing only future runtime values produces the same bytes. One-short
   compiler limits produce no artifact.
3. Mutate one A-owned selection at a time: source/native revision or digest,
   model/profile/configuration/producer-correspondence identity or digest,
   compiled-artifact reference, and recanonicalized bytes. Assert FR-042's exact
   typed refusal/unsupported/resource-incomplete cause; do not accept a protocol
   result digest in a producer or artifact domain.
4. Offer the unchanged compiler output to B #11. B independently rederives its
   selection and artifact digest; byte identity at this intake is the terminal
   assertion of A's contribution.
5. Treat the composed run as a separate D-owned integration gate under
   [quire-research#39](https://github.com/agent-ix/quire-research/issues/39) and
   [IN01 #49](https://github.com/agent-ix/quire-research/issues/49). Until an
   integration repository transfer is approved, #49 owns the version-lock
   manifest, Rust driver and aggregate record; no local A fixture substitutes.
   That driver pins accepted B #6/#11/#12 and F revisions, binds concrete O1/O2
   instances, obtains F's record/correlation/availability/membership/
   completeness/progress/closure handoff under D's identities, and records B's
   PT02 batch/incremental results. Missing provider/member/completeness,
   ambiguous relationships, cross-order substitution, late/absent refund,
   compensation failure and assessment-work exhaustion use those owners' exact
   cases and result vocabularies; A neither predicts nor reimplements their
   outcomes.

## Expected Results

The local positive produces one exact static subject and B #11 receives the same
bytes and independently derived selection. Each A-owned mutation returns the
FR-042 cause for its exact stage and no substitute artifact; future runtime-value
changes do not change the static bytes, while compiler-limit exhaustion emits
none.

These assertions discharge only A's compilation, preservation and handoff
contribution. Campaign acceptance remains Planned under D's research #39/#49
until that owner records one version-locked run in which F supplies the admitted
runtime facts, B #6/#12 owns the conformance/result classifications, and batch/
incremental results agree. As of 2026-09-11 no accepted B #6/#12 plus F consumer
revision set exists to pin; branch or unreviewed snapshots cannot fill that slot.
That external record may reference this passing compiler leg; this TC cannot by
itself mark PT02, observation adequacy or the composed campaign complete.

---
id: SR-16303
title: "spec-review of STD-441 function-value parity obligation identity (quire-spec-language PR 1383)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b88c1ddf; spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md; spec/test-cases/TC-1252-function-value-parity-obligation-identity.md; spec/functional/FR-357-replay-scalar-parity-claims.md; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md; spec/functional/FR-098-execute-a-replay-request.md; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-484
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1252
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: references
---

# Spec review of STD-441 function-value parity obligation identity (quire-spec-language PR 1383)

## Summary

Ticket: STD-441. One spec-review pass over the PR's diff against origin/main (main is an ancestor of head b88c1ddf). Checked against the ticket's requirement from CG IR-983 / CG #553 (SR-16300 FND-001, SR-16301 FND-001) and CG main FR-024-AC-44/46: that QSL defines the function-value parity ObligationIdentity and the retained claim inputs so CG derives nothing of its own.

Examined, clean:

- Byte vectors derived by hand (RFC 8785 canonical JSON, members sorted, SHA-256 over the canonical bytes alone; QSpec FR-201 puts the domain label outside the hashed bytes, as the O-09 parity worked example does): the 2-parameter preimage is exactly 516 bytes with digest 793909279b4897ab8f03f3c14547fc08861d353d979e471aa423c2bce9d8b92c; empty arguments 908362fdf1c3d0bb20aadf2249e285964ad024f5fd994d6c01e88024839166b7; first argument only 3333f20dd244bd7043f410893713ed40ffc067b45cae410050898de89834cbc0; parameters declared in the other order (positions 0/1 reassigned) f052b4583fc320a5a470209188b911e75fe02d55a28d079a5e614ac0f44e76fd; the argument entry alone 34f0d98f1306d5e3720eec3469a6181c73f809ac51c6fe608e97129e7f94270f. All match FR-484-AC-1/2 and TC-1252 VP-01/02. The P4 and T4 ids match QSpec FR-322 / TC-233 SP-01.
- Check order in replay_value_parity: target, recompile, package_id, UnknownFunction, NotAValueFunction, then ClaimOccurrence, then the identity recomputation, then the argument join. Consistent with FR-098's selection order and FR-357-AC-20; AC-6 and AC-7 pin the order against UnknownParameter and PackageIdMismatch.
- Refusal codes: ClaimOccurrence (missing_declaration/missing-name) matches FR-098; ScalarIdentity (stale_dependency/content-mismatch) with causes Obligation and Encoding matches FR-357's operator arm.
- Claim identity composition: obligation identity, package_id, function target, occurrence key, source, effective accounting limits, generated outcome and generated-content identity. The content identity is carry-and-bind, like the operator arm's observation digest.
- Consistency with the operator-path O-09 identity: same encoder, JSON-integer positions and ordinal, occurrence_key {ordinal, role}; the subject_kind tag and no obligation_kind keep the two forms apart, and ADR-013 O-09 and FR-201 both name the sixth form.
- FR-121 FunctionSite.value_parity and FR-098 wording agree with FR-484. QSL-1016 is the code ticket.
- No drop, narrowing or deferral language in the diff. FR-484 and TC-1252 are defined on no other open QSL PR head.

Gates: `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` exit 0; with QSPEC_DIR at QSpec #422 head 150bcdf1 exit 0. 19 warnings, all on pre-existing files, none on FR-484 or TC-1252.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The retained claim inputs do not cover the Input-arm `WitnessPacket<NoPayload>` that CG's consumer must build. CG FR-024-AC-44 has the producer complete that packet "from the original checked claim", and FR-024-AC-46 compares its subject `clause_node`. QSL's `WitnessPacket` also requires `clause_node`, `profile_selections`, `run_limits`, `declared_domains` (finite `ProofBound`s), `backend` and `trace_position`. FR-484's table names none of these. It does not say what `clause_node` holds for this claim (the function node?), what `declared_domains` is when the identity deliberately has no harness bound (an unbounded `Int` has no `ProofBound`), or which `backend` string goes on a claim with no backend run. So CG must still decide packet members on its own, which the ticket rules out. Fix: add the packet members to the Retained claim inputs table, with their source and value for this form, and an AC that holds them. | spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md:94-115 | missing-requirement |
| FND-002 | medium | TC-904's function-level steps 1-6 and FR-357-AC-1..4 do not say the request carries the identity `call_site` minted and the `declaration` occurrence. Under the new check order, a request with any other identity refuses ScalarIdentity before the argument join. For example, TC-904 step 4 (inc with inv's parameter node) would then refuse Obligation, not UnknownParameter. QSL-1016 says to rebuild these tests on minted identities, but the TC does not state it. Fix: state in TC-904's function-level fixture that each request is built from FunctionSite (value_parity.obligation_identity and declaration). | spec/test-cases/TC-904-replay-scalar-parity-claims.md:21-29 | missing-requirement |
| FND-003 | low | The text says "These eight inputs are the whole claim ... a claim that differs from another in any one of them is a different claim". But input 1 includes the source digests, the dependencies entries and the byte provision, while `claim()` holds only `package_id` of that input. Either say that input 1 enters the claim through `package_id`, or list exactly what the claim holds. | spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md:113-115 | wrong-requirement |

## Verdict

Not mergeable: FND-001 (high) blocks the PR. FND-002 (medium) and FND-003 (low) should be fixed in the same round. The identity itself (preimage, vectors, minting site, recompute check, refusal codes, claim binding) is correct and consistent with the operator path.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-004 | medium | The retained inputs and the claim inventory leave out request members that QSpec FR-323 requires for a request that supplies function arguments. A value-parity Input-arm request does supply arguments, so it needs the complete `limits.supplied` object (A, H and the magnitude bound B), and the request also carries `state_environment` and `object_environment`. None of these appears in the Retained claim inputs table. "What the claim identity holds" says `claim()` holds "nothing else". Yet FR-323 makes the supplied ceilings part of request identity, and B decides whether an unbounded argument such as `grow`'s `n` is admitted. Two claims that differ only in B therefore read as one claim with different results. Fix: name these members and their source (FR-323 defaults, empty state and object environment for a pure function), and either put the effective supplied ceilings in `claim()` or state why they are not claim members. | spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md:101-168 | missing-requirement |
| FND-005 | low | "the FR-331 `BackendId`" resolves to QSL's own FR-331, "replay a temporal counterexample over an observed trace". The `BackendId` and provider envelope are QSpec FR-331. Write "QSpec FR-331". | spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md:111 | wrong-requirement |

## Dispositions

Round 1, fix head fb83a5b2 (QSpec #422 head 1c7e0677 adds only the SR-16304 file).

I hand-checked the packet round trip against `WitnessEnvelope::reconstruct` (qsl-replay/src/witness.rs:958-1038). Reconstruct checks that every member is present, refuses an empty `backend`, and checks the encoded size. It does not reject an empty `declared_domains` list.

- `inc` at x = 3 and `grow` at n = 41, with every member filled as "The input-arm packet" states (`backend` `generator-under-test`, `declared_domains` `Some([])`, `trace_position` `Some(None)`, `family_payload` `NoPayload`), are both admitted, and `to_packet` returns the same members.
- Replay from the admitted envelope gives `grow(41)` = 42 (`Agrees`) and `inc(3)` = 4 against 5 (`Diverged`).
- Leaving `declared_domains` empty is consistent with QSL. Only `composite_parity` reads `request.declared_domains()`; `replay_value_parity` never does, and S6a admission checks the arguments against the signature.

Gates:
- QSL `make check-spec-validation` exits 0 with QSPEC_DIR at the shared clone and at #422 merged with the current main (19 pre-existing warnings).
- QSpec #422 merged with main 4aa0af73 (the merge is clean): `quire validate` gives exactly 86 failure lines, and cargo passes 24 + 136.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | See after_excerpt in the Linear dispositions comment. |
| FND-002 | fixed | See after_excerpt in the Linear dispositions comment. |
| FND-003 | fixed | See after_excerpt in the Linear dispositions comment. |
| FND-004 | still-open | New finding of this round: `limits.supplied`, `state_environment` and `object_environment` are not named, and the supplied ceilings are outside `claim()`. |
| FND-005 | still-open | New finding of this round: the unqualified FR-331 reference. |

Round 1 verdict: Not mergeable. FND-004 (medium) and FND-005 (low) are open. FND-001 to FND-003 are fixed.

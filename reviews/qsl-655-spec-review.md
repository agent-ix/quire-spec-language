---
id: SR-12324
title: "Spec review of the FR-289 and TC-773 edits for QSL-655 (PR #1208)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1ef4a3cbc6f981f2209d912e382c243893b00faa; spec/functional/FR-289-keep-results-unchanged-by-provider-installation.md, spec/test-cases/TC-773-installing-removing-or-reordering-providers-changes-nothing-outside.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-773
    type: reviews
---
# SR-12324: Spec review of the FR-289 and TC-773 edits for QSL-655 (PR #1208)

## Summary

Ticket: QSL-655. The review covers the three new FR-289 behaviour bullets, FR-289-AC-4 to AC-6, and TC-773's step 4 and its expected result. `quire validate` passes on both files.

What was checked:

- **Consistency with ADR-029.** The new text agrees with ADR-029 PL-7 ("Counterexamples", "Trusted proofs"), RU-4 and PV-2 item 4. It also agrees with QSpec FR-305 and its AC-2 and AC-4.
- **Testability.** Each AC is a testable, single-subject statement. TC-773's scope line now reads AC-1 to AC-6, and step 4 covers each case.
- **Conflict risk.** ADR-029 PL-7 is draco's, and draco is writing FR text in parallel. This PR does not touch ADR-029. The only open PR touching ADR-029 is #1206 (STD-258, FR-280 provider entry), and it does not touch FR-289 or TC-773. Textual conflict risk is low. Two risks remain on the behaviour:
  - FR-289 now states the QSL-only cause rule before PL-7 does. If draco words PL-7 differently, FR-289's bullet must follow.
  - If draco's FR text also adds plugin terminal ACs, for example in FR-331 or a new FR, the two would duplicate each other.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-289 behaviour bullets and AC-4 say nothing about two outcomes the code settles as `failed`: a Witness-arm replay result for a process-provider counterexample, and an Input-arm replay that reproduced a non-violation. A reader of FR-289 alone cannot tell either is defined. Add "a replay on the witness arm, or one that reproduces no violation, settles `failed`" to the counterexample bullet. | spec/functional/FR-289-keep-results-unchanged-by-provider-installation.md:72-76 |
| FND-002 | low | The new behaviour bullets cite no ADR-029 anchor. The QSL-only cause rule has no PL-7 text yet (draco's lane), so FR-289 is the only place it is stated. Once PL-7 lands, add the anchor to the Dependencies line ("ADR-029 PV-2" today) so the two cannot drift. | spec/functional/FR-289-keep-results-unchanged-by-provider-installation.md:80-82,97 |
| FND-003 | low | FR-289-AC-4 is compound: four outcomes in one row. This matches FR-289-AC-3's style and stays testable, but a failure cannot name which outcome broke. Splitting it is optional. | spec/functional/FR-289-keep-results-unchanged-by-provider-installation.md:91 |

## Verdict

The spec edits are sound and match the code and the ADR. There are no high or medium findings, and the PR is not blocked on spec. Coordinate with draco so that PL-7's QSL-only cause wording, and any FR text draco adds, matches FR-289's bullets.

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed: FR-289 states witness-arm and reproduced non-violation settle failed (AC-7, AC-8) | 5acdfbc3d |
| FND-002 | fixed: bullets cite ADR-029 PL-7 by name; draco text lands later | 5acdfbc3d |
| FND-003 | fixed: AC-4 split into AC-4 to AC-8 and AC-9 to AC-11, TC-773 renumbered | 5acdfbc3d |

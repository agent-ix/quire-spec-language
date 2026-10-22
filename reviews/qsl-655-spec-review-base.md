---
id: SR-15440
title: "Spec review of the ADR-029 PL-7 plugin terminal rules for QSL-655 (PR #1210)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1892b8760472c2080c2d81b54ec08f47a9ff6a87; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: reviews
---
# SR-15440: Spec review of the ADR-029 PL-7 plugin terminal rules for QSL-655 (PR #1210)

## Summary

Ticket: QSL-655. The review covers the diff `origin/main...1892b8760`: the four
new or extended PL-7 bullets ("Counterexamples", "Trusted proofs", "QSL-only
causes", "Refused request body"), the ADR-013 O-25 and C-09 amendment bullet,
and the two new `relates_to` links (QSL FR-289, QSpec FR-462). It checks them
against ADR-013 QC-6, O-25 and C-09, ADR-018 §1 (PC-1), ADR-028, QSpec FR-331
(counterexamples, basis, certification and wire cause spellings), QSpec
FR-462-AC-7 and the FR-289 and TC-773 text of the open code PR #1208 (head
0e6462feb).

What was checked and holds:

- **Rule 1, counterexamples.** Complete: reproduced violation settles
  `refuted` naming the plugin `BackendId` and keeping the replay result;
  `inconclusive` replay settles `replay-parity` with its `DisagreementCause`
  and no replay result; refusal follows `from_replay_refusal` (fault
  `failed`); reproduced non-violation and the Witness arm settle `failed`.
  Agrees with QC-6 and O-25 and with FR-289-AC-4 to AC-8 in #1208.
- **Rule 2, trusted proofs.** `Proved{basis, certification: Trusted}` with
  the reported `ProofBasis` and the plugin `BackendId`; no made-up
  `success_checks`. The three refused bases match #1208's FR-289-AC-10 and
  its code. The rule's coverage of core-only bases is incomplete (FND-001).
- **Rule 3, QSL-only causes.** Stated, with the typed-cause refusal that
  FR-289-AC-12 tests. The spelling of the vacuous cause is ambiguous in
  practice (FND-002).
- **Rule 4, refused request body.** Matches FR-462-AC-7 word for word on the
  two diagnostics; `Failed` carries no cause; the diagnostic goes to the CB-4
  outcome document through `OutcomeDiagnostic::new`.
- **Anchors.** #1208's FR-289 bullets cite PL-7 "Counterexamples" and
  "Trusted proofs"; both bold labels exist at lines 583 and 601. TC-773 step 4
  in #1208 cites no PL-7 anchor (it needs none; noted only because the brief
  expected one).
- **Behaviour only.** No status, changelog or remaining-work language in the
  new text.
- **Gate.** `QSPEC_DIR=/Users/peter/dev/quire-specification make
  check-spec-validation` exits 0 on the head; no warning is in ADR-029.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The core-only basis list is incomplete. PL-7 refuses only `bounded-proof{checks}`, `closed-form{analysis}` and `witnessed{sources}`, but the records it cites make `exact-value` and `value-bounds` "always `Certified`" (ADR-018 §1 as amended by ADR-028 XV-1, XV-4) and `zone-certified` a proof "whose certificate the checker accepted" (ADR-026); QSpec FR-331 records each "only after the checker accepts it" (FR-331-AC-15). A plugin can advertise `probabilistic-satisfaction` with evidence `exact` (FR-462), so a plugin `proved` with `exact-value` would settle `Proved{ExactValue, Trusted}`, which ADR-028 forbids. Add these three bases to the refused list (or state that the core checks their certificates and settles them `certified`, and say how that squares with RU-4 "every plugin `proved` is `trusted`"). #1208's FR-289-AC-10 and `from_plugin_proof` follow the same list and need the same change. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:608-614 |
| FND-002 | high | The vacuous-proof spelling is not unambiguous, and the two QSL-655 PRs disagree on it. PL-7 writes `kani_vacuous_proof`, which matches QSpec FR-331 (the one closed cause spelled with `_`), next to `replay-parity` and `replay-refused`, but does not say that the reader compares FR-331 wire spellings. QSL's `ReportedInconclusiveCause::as_str` documents `kani-vacuous-proof` as "the cause's wire spelling", and #1208 followed it: FR-289-AC-11 and `read_plugin_cause_kind` refuse `kani-vacuous-proof`, so a plugin sending FR-331's `kani_vacuous_proof` passes the guard. State in PL-7 that the reader matches the FR-331 wire spellings exactly (`replay-parity`, `replay-refused`, `kani_vacuous_proof`), and that `kani-vacuous-proof` is not the wire spelling. #1208 must then change FR-289-AC-11 and its reader to match. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:615-623 |
| FND-003 | medium | The diagnostic for a refused plugin result has no code. PL-7 calls a QSL-only cause a "tool failure" and says a refused plugin result settles like a refused request body, "with the refusal in `diagnostics`", but names no catalog code or cause. FR-462's tool-failure cause is `tool-failure{defect}` with a closed defect table that has no row for this, and the FR-331 stage schema admits these spellings, so `malformed-body` does not apply. A test can check `failed` but not the diagnostic. Name the code and cause (for example an `invalid_result` cause), or drop "tool failure" and say only that the item settles `failed`. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:620-623,633-636 |
| FND-004 | medium | PL-7 does not settle a plugin `refuted` whose counterexample is not canonical assignments. FR-331 lets a `counterexamples` entry name a transcript artifact, or carry a `TemporalCounterexample`, `ProbabilisticCounterexample`, `GraphEvidence` or `HyperCounterexample`, and a `refuted` entry may point at no entry. PL-7 says a plugin's counterexample "is canonical assignments" and that "the replay alone settles the item", but not what happens when there is nothing to replay as `ReplaySource::Input`. State that such a result is refused and settles `failed` (or how it replays). | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:583-600 |

## Verdict

Not mergeable: FND-001 and FND-002 block. Rules 1 and 4 are complete,
atomic and testable and agree with ADR-013, FR-462-AC-7 and #1208. Rule 2
leaves three certificate-checked bases open to a `trusted` settlement that
ADR-018 as amended by ADR-028 and ADR-026 forbid. Rule 3's spelling of the
vacuous cause matches FR-331, but PL-7 must say the reader matches wire
spellings, and #1208 currently refuses the QSL `as_str` spelling instead.
FND-003 and FND-004 are minor.

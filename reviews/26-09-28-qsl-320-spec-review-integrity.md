---
id: SR-780
title: "QSL-320 spec review (integrity) of PR 518 ADR-013 replay-type ownership"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md (Status, Context, O-16 families, O-24, O-25, O-26, O-27, §4 flowchart, C-09 to C-11, #231 row, QC-20, Q209-5, OQ-H, TK-04, OBS-027, IR#137 row); spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md (FB-05, OBS-029, context only); agent-ix/quire-specification AD-016 (Packet owner rows, context only)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-320. PR 518 changes one file, ADR-013, to record the 2026-09-28
replay-type ownership ruling as OQ-H and to rewrite O-24 to O-27, TK-04 and
dependent rows. I ran the integrity sub-analysis (cross-row consistency and
cross-record consistency). EARS, object and dependency analyses don't apply:
the PR adds no requirement statement, no new domain object and no
`relationships:` edge.

Checked and consistent: the Status line; the Context witness bullet; O-24's
Owner, Implementing ticket, Public type and Conversions rows; O-25's Owner,
Carrier and Implementing ticket cells and the packet-members paragraph; O-26
and O-27 Owner rows; C-10 and C-11; the #231 row (IR#144 dependency dropped);
QC-20; OBS-027; the IR#137 row; TK-04 (narrowed, correct remaining work);
Q209-5. `quire validate` exits 0 on the file.

Rows the PR missed or left inconsistent are listed below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | C-09 was retargeted to QSL `TerminalValue`, but its invariant and test cells still describe the old target. `TerminalValue` has no `Inconclusive` variant: a vacuous run is `Proved { success_checks: 0 }`, and `TerminalValue::category` maps it to `inconclusive` (O-24 Public type row, `qsl-replay/src/proof_result.rs:104-147`, IR FR-031 table). The C-09 invariant says "a vacuous `Proved` maps to `Inconclusive`", and the test cell's mutation "(a vacuous `Proved` stays `Proved`) that turns C-09 red" now describes the correct behaviour. Rewrite: the map sends a vacuous proof to `Proved { success_checks: 0 }`; the mutation to test is losing the check count (for example mapping it to `Proved { success_checks: 1 }`). | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:961 |
| FND-002 | medium | The §4 conversion flowchart still has `K -->\|C-09 and C-10\| PKT[IR packet with ReplaySource]`. O-25 now says the carrier is QSL `WitnessEnvelope.source` and IR defines no packet or `ReplaySource`. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1001 |
| FND-003 | medium | OQ-H moves the counterexample packet from IR to QSL, which contradicts accepted AD-016 ("Owner \| Packet: IR", Replay-ownership row "Packet \| IR", Owner decision 3). ADR-013 records every cell that differs from AD-016 as a QC amendment request, and Context line 48 counts them ("Eight cells"). No QC row requests this amendment, the count is unchanged, and the Context bullet at line 44 still states "replay places the packet in IR" without noting that OQ-H amends it. Add a QC row (AD-016 amendment: Packet owner QSL `qsl-replay` `WitnessEnvelope`, per OQ-H) and update the count. This stays inside this repo; QSpec is not edited. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:44-48 |
| FND-004 | medium | O-25's conversion now assigns `decode` to QSL (`Witness::decode`) with input `WitnessBinding`s and output typed `WitnessValue`s, but no row says who owns `WitnessBinding` and `WitnessValue`. IR still exports both from its root crate (IR FR-039), and qsl-replay's frame payload says it builds over "IR's `WitnessBinding`". A QSL method that takes an IR type needs a QSL → IR edge, the reverse of the edge OQ-H relies on. OQ-H should name the owner of `WitnessBinding`/`WitnessValue` (or say `decode` takes QSL-owned bindings). | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:879-881 |
| FND-005 | medium | OQ-H contradicts ADR-011 FB-05 (no backend repository depends on QSL Rust types except CG's replay adapter) and ADR-011 OBS-029 (the IR root → QSL edge is removed). The PR flags this in Q209-5 and in OQ-H's reopen condition, and does not amend ADR-011. The flag is adequate for merge, but ADR-011 needs its own amendment ticket, because the two records now disagree about IR's allowed dependency edge and ADR-011 T-12's direction check will fail IR when IR adopts the QSL types. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1141,1202 |
| FND-006 | low | OQ-H's Ruling cell lists `Witness`, `ReplaySource`, `WitnessEnvelope`, `TerminalValue`, `TerminalRecord` and `ObligationIdentity` and cites O-24 to O-26. The O-24, O-26 and O-27 rows cite OQ-H for `ProofResultEnvelope`, `ReplayRequest` and `ReplayResult` too. Add those three types and O-27 to the Ruling cell so the ruling covers what the rows attribute to it. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1202 |

## Verdict

The rewrite is coherent across O-24 to O-27 and the rows the coder named, and
the coder's own-initiative edits (Context bullet, C-09 to C-11, #231, QC-20,
OBS-027, IR#137, Q209-5) are warranted: each removed a sentence that would
otherwise contradict the new O-24/O-25 text. Four rows were still missed or
left half-converted (FND-001 to FND-004, FND-006). All are one-line text fixes
inside this file and should be fixed in this PR. FND-005 is a cross-ADR
conflict that the PR correctly flags rather than resolves; defer it to an
ADR-011 amendment ticket.

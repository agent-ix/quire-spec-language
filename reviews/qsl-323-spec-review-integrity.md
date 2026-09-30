---
id: SR-811
title: "Integrity review of QSL-323: ADR-011 and ADR-013 after the C-09 map and O-25 packet move to CG"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@4c51991de72f82db507d584499f87a5fff7ae0ac; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, tools/arch-lint/graph.rs, tools/arch-lint/metadata.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-811: Integrity review of QSL-323: ADR-011 and ADR-013 after the C-09 map and O-25 packet move to CG

## Summary

Ticket: QSL-323. PR agent-ix/quire-spec-language#534, diff `origin/main...HEAD`.

The PR moves the C-09 `KaniOutcomeKind` → `TerminalValue` map and the O-25
packet (`WitnessEnvelope` over `qsl-replay` witness types) from IR to CG, and
states that IR names no QSL type and has no QSL dependency. This review checks
that every ADR-011 and ADR-013 row agrees with that design.

Measured facts: IR main `3e7935f` declares no QSL dependency in `Cargo.toml`
or `crates/quire-contract-model/Cargo.toml`. CG main `37ff360` depends on
`quire-contract-ir` and `qsl-replay`. The design's cycle argument holds.

Examined and consistent: ADR-013 O-16 proof row, the O-16 implementing-ticket
paragraph, the terminal-record decision table, the `Witness` carrier and
implementing-ticket rows, the O-25 decode paragraph, C-09, QC-9, QC-16, QC-29,
Q209-5, OQ-H, TK-04 and TK-05; ADR-011 FB-05, §7.1 edge table, OBS-029 and the
M-6b row.

Not consistent: ADR-011 §2 still places the packet and witness in IR (S7, E8,
E9), FB-08 names IR `Witness::parse`, and OBS-028 says IR holds the packet
and witness. Two arch-lint doc comments still call the IR → QSL edge the real,
currently observed violation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | ADR-011 §2 still makes IR the owner of the packet and witness. E8's owner is `IR` and its output is `CounterexamplePacket{source: ReplaySource}`; S7's owner is "IR `src/kani` (packet and witness types)"; E9 and its details say "the IR packet". ADR-013 OQ-H, QC-29 and TK-05 in this PR say CG builds the packet as QSL's `WitnessEnvelope` and IR names no QSL type. An IR implementer following E8 builds a packet over `ReplaySource`, a `qsl-replay` type, which adds the IR → `qsl-replay` edge and closes the QSL ⇄ IR cycle that FB-05 and FB-11 forbid. Fix: E8 owner CG, output QSL `WitnessEnvelope{source: ReplaySource}`; S7 owner QSL `qsl-replay` types built by CG's backend adapter; E9 "the packet" (not "the IR packet"). | ADR-011:195, ADR-011:261, ADR-011:262, ADR-011:266; ADR-013 OQ-H, QC-29, TK-05 |
| FND-002 | medium | FB-08 forbids "a witness typed by any means other than IR `Witness::parse` and `decode`". ADR-013 makes `Witness::parse` and `decode` QSL `qsl-replay` methods and has IR delete its `Witness` copy (TK-04). As written, FB-08 blesses witness typing through IR's copy, which is the copy OQ-H deletes. Fix: "other than QSL `Witness::parse` and `decode` (`qsl-replay`)". | ADR-011:568; ADR-013 OQ-H, TK-04, C-10 |
| FND-003 | medium | OBS-028 says "IR holds the packet and witness only. CG reconstructs." That contradicts OQ-H and QC-29: IR defines no packet, `Witness` or `ReplaySource`, and CG builds the packet. OBS-038 quotes the closed #205 text "Contract IR holds the counterexample packet" with no note that OQ-H superseded it. A reader resolving ownership from §9 gets the old answer. Fix: OBS-028 "CG builds the packet as QSL's `WitnessEnvelope`; QSL S6a executes"; add "superseded by ADR-013 OQ-H" to OBS-038. | ADR-011:1121, ADR-011:1127; ADR-013 OQ-H, QC-29 |
| FND-004 | low | T-5 now reads "none: the IR root → QSL edge is removed … \| none". A tickets row whose task and owner are both "none" records a settled question as a row. State what is: delete T-5 (OBS-029 already states the fact), or leave the id retired with no row. | ADR-011:1396 |
| FND-005 | low | The arch-lint doc comments still say the IR → QSL edge is "the real, currently observed FB-05 violation (ADR-011 OBS-029)" and "matching ADR-011 OBS-029's real, currently observed shape". Amended OBS-029 says the edge is removed. The tests stay valid as synthetic negative controls; only the comments are now false. Fix: call them synthetic IR → QSL fixtures, without the OBS-029 claim. | tools/arch-lint/graph.rs:247, tools/arch-lint/metadata.rs:218 |

## Verdict

Changes requested. The ADR-013 edits are internally consistent and match the
measured IR and CG manifests. ADR-011 still assigns the packet and witness to
IR in §2, FB-08 and OBS-028, which contradicts the ruling this PR records.

---
id: FR-290
title: "Settle plugin results as typed terminal records, with plugin proofs labelled trusted"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-298
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: references
  - target: "ix://agent-ix/quire-specification/FR-305"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-331"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-354"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-462"
    type: depends_on
---
# FR-290: Settle plugin results as typed terminal records, with plugin proofs labelled trusted

## Description

A plugin is either a compile-time Rust implementation of the driver's
`Provider` trait, linked into the binary, or a third-party executable that
runs as its own operating-system process and speaks the plugin wire
`quire.plugin-wire/v1` (ADR-029 PL-1 to PL-5). QSpec FR-305 owns the plugin
forms and QSpec FR-462 owns the wire. An installed plugin is a trusted
program, like any program the user installs, and runs with the user's own
authority.

This requirement states how a plugin's result becomes a QSL terminal record:

- Every result is read with the typed FR-331 reader. A plugin receives the
  package's v2 bytes and returns FR-331 results; it never produces a checked
  object, and IR's reader revalidates the `package_id` it was given
  (ADR-011 §4).
- A plugin's counterexample is canonical assignments. Its item settles
  `refuted` only when S6a replay reproduces it (FR-289 item 4).
- A plugin's `proved` result has no replay and no in-core certificate
  checker. Its terminal record keeps the result `proved`, names the plugin's
  `BackendId` (FR-288), and carries the basis label `trusted`. A renderer
  shows the label beside the verdict (FR-298).

The terminal record type and its `TrustBasis` member are QSL's, in layer 6
`qsl-replay`. QSL settles the records of its native engines and of process
plugins there; CG keeps the C-09 map that settles a Kani outcome (ADR-013
C-09), run by the orchestrating driver (ADR-011 T-13). The Kani prove path
is part of the qualified core (ADR-029 CB-2), so a Kani `proved` carries no
trust label. Every other `proved` record carries exactly one trust basis, by
its producer:

| Producer of the `proved` result | Settled by | Basis | The record also names |
| --- | --- | --- | --- |
| An `analyze` engine whose certificate a qualified-core checker accepted (FR-281, FR-282) | QSL, `qsl-replay` | `certificate-checked` | the checker that accepted it |
| A native `analyze` engine with no qualified-core certificate checker (FR-281) | QSL, `qsl-replay` | `uncertified` | the engine |
| A third-party out-of-process plugin | QSL, `qsl-replay` | `trusted` | the plugin's `BackendId` |
| The Kani backend | CG, C-09 | none: the qualified prove path | the backend's `BackendId` |
| The SMT provider route | CG, as QSpec FR-354 states | QSpec FR-354's `certificate-checked` (Carcara) or `solver-asserted` | as QSpec FR-354 states |

QSpec FR-354 owns the trust-basis vocabulary. It defines the basis of SMT
proofs and plugin proofs. The `uncertified` rows and the `analyze`
`certificate-checked` row need matching rows in QSpec FR-354, whose
`certificate-checked` then covers a qualified-core checker as well as
Carcara. Those rows are a QSpec follow-up.

## Inputs

An FR-331 result from a process provider, the items routed to it, and the
package's byte provision for replay.

## Outputs

One QSL terminal record per routed item.

## Behavior

- QSL shall read every process-provider result with the typed FR-331 reader
  before it builds a terminal record from it.
- When a process provider reports a counterexample, QSL shall replay it
  through layer-6 `replay` (FR-098) and settle the item `refuted` only if the
  replay reproduces it.
- If the replay does not reproduce the counterexample, then QSL shall settle
  the item `inconclusive` with cause `replay_parity`, or with cause
  `replay_refused` and the refusal's catalog code when the replay refuses.
- When a process provider reports `proved`, QSL shall settle the item
  `proved` with the provider's `BackendId` and the basis label `trusted`.
- When a qualified-core certificate checker accepts an `analyze` engine's
  certificate, QSL shall settle the item `proved` with the basis
  `certificate-checked` and the checker's name.
- When an `analyze` engine with no qualified-core certificate checker
  reports a proof, QSL shall settle the item `proved` with the basis
  `uncertified` and the engine's name.
- QSL shall settle no `proved` record of a native engine or a plugin
  without a trust basis.
- The terminal record type shall hold a Kani `proved` that CG's C-09 map
  settles with no trust label.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-290-AC-1 | A process-provider FR-331 result of `proved` for a routed item settles a terminal record with value `proved`, the provider's `BackendId` and basis `trusted`; the FR-281-AC-1 record's basis is `certificate-checked`, naming the EN-5 checker; FR-281-AC-4's record from the test engine with no checker has basis `uncertified`, naming that engine; a Kani `proved` record built as C-09 settles it carries no trust label, neither `uncertified` nor `trusted`. | Test (TC-774) |
| FR-290-AC-2 | A process-provider counterexample that replay reproduces settles `refuted`; one whose assignment replay evaluates to `true` settles `inconclusive` with cause `replay_parity`; one naming a parameter the function lacks settles `inconclusive` with cause `replay_refused`. | Test (TC-774) |
| FR-290-AC-3 | A process-provider result that fails the FR-331 reader builds no terminal record from its body, and its items settle as FR-291 states. | Test (TC-774) |
| FR-290-AC-4 | The text rendering (FR-298) of AC-1's plugin record shows `proved` with the label `trusted` beside it, and its JSON outcome document holds the basis `trusted`; the `uncertified` record renders with the label `uncertified` beside `proved`; the AC-1 Kani record renders `proved` with no label. | Test (TC-774) |

## Dependencies

- ADR-029 PL-1, PL-4, PL-5, PL-7: the plugin forms, the result rules and the
  trusted label; rulings RU-4.
- ADR-011 §4, FB-07: checked objects and replay.
- ADR-013 O-16, O-24: categories and terminal records.
- [FR-288](FR-288-build-the-registry-from-provider-manifests.md): `BackendId`.
- [FR-289](FR-289-keep-results-unchanged-by-provider-installation.md): the meaning invariant.
- [FR-098](FR-098-execute-a-replay-request.md): replay.
- [FR-298](FR-298-render-outcomes-and-views-from-typed-values.md): rendering.
- QSpec FR-305: the plugin forms; QSpec FR-462: the `quire.plugin-wire/v1`
  wire; QSpec FR-331: results; QSpec FR-354: the trust-basis vocabulary.

## Overlap

The driver repository owns the plugin host, `quire-plugin-host`: starting
the process, reading frames and adapting a plugin to the `Provider` trait.

## References

- QSL-393 (V1-A06a): the plugin wire.
- QSL-390 (ARCH-50): owner ruling 4 and RU-4, recorded on the ticket.
- QSpec FR-305, FR-462, FR-354 (STD-141): the QSpec half.
- QSpec follow-up from SR-1060 FND-003 and ruling RU-6: FR-354 rows for an
  `uncertified` proof and for a qualified-core checker's
  `certificate-checked` proof.
- QSL-390: ruling RU-6 (`uncertified`), recorded on the ticket.

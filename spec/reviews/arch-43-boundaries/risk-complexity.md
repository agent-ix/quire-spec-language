---
id: SR-805
title: "Risk and complexity review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries"
type: SpecReview
analysis: risk-complexity
scope: "spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, qsl-replay/src/witness.rs, qsl-replay/src/request.rs, qsl-replay/src/spine.rs"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
# SR-805: Risk and complexity review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries

## Summary

Reviewed the uncommitted ADR-017 draft on `spec/16-arch43-mapping` (QSL
`main` at `99e9b6c7`) against QSpec `origin/main` at `4634f5f`. Each
decision area was scored for technical risk and for volatility:

| Area | Tech risk | Volatility | Drivers |
| --- | --- | --- | --- |
| PF-1 to PF-3, PF-5, PF-6 (landed identities, entry selection) | Low | Low | Describes landed code; TK-1, TK-3, TK-4 are local |
| PF-4 / TK-2 (`ReplaySelection`) | High | Medium | Cross-repo wire member; open G3 gate |
| RF-1 to RF-7 (#191, #192 recorded-baseline gates) | High | High | Reference outcomes that no build produces; draft editions move often |
| AR-1 to AR-6 (#198 abstraction relation) | Medium | High | Rests on the unmade QSpec decisions Q-1 and Q-2 |

What holds:

- The protocol/frame half is low risk. It maps landed code, and the four
  defects G-1 to G-4 are small and local. TK-4 is feasible:
  `FieldMemberRecord.key` exists in the admitted domain package
  (`qsl-semantics/src/model/domain_package.rs:182-186`).
- Keeping the refinement gates out of the proof path (RF-1) removes solver,
  routing and family-hook risk from #191 and #192.
- Leaving Rust path resolution to CG (AR-2) keeps QSL free of any Rust
  toolchain dependency.

What does not hold:

- Three findings are high. #192's baseline is not measurable. #198 depends
  on QSpec decisions that do not exist yet, and its keys may be reshaped by
  them. TK-2's blast radius crosses three repositories and an open gate.
- The recorded-baseline design can drift or satisfy itself, and RF-4's
  classification is brittle as `CompileRefusal` grows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | #192 as designed has no reference producer. FR-110 makes `root` the only header-selectable row, and every other `DefinitionLock` row is fixed by the catalog. No QSL build compiles a case under an AD-003 parent (for example `quire.state.core/v1`) without its child. The only path that is not the spine, the root crate's native path, is being deleted under M-6c. Every "parent refused" record is then authored, not measured. The gate checks only that the child refuses what the author expected, which weakens V1-TOOL-012 without saying so. Mitigation: before building #192, spike on one AD-003 edge to show a build that yields a parent-profile outcome. If none exists, restate #192 as a child-refusal corpus with authored expectations, and record that limit against V1-TOOL-012. | ADR-017 §2 RF-2, RF-5; FR-110; QSpec AD-003 "Accepted profile hierarchy", V1-TOOL-012 |
| FND-002 | high | #198 depends on unmade QSpec decisions, but the ADR presents it as buildable (§5, §8, Consequences). Q-1 (surface spelling) and Q-2 (v2 node) are unfiled. Until Q-2, a checked relation node is omitted at E4, and spine `compile` refuses the unit (`CompileRefusal::Omitted`, `qsl-replay/src/spine.rs:139-143`). Q-2 may also reshape AR-2's keys and `RustPath`, which §6 says the node carries. So `FrameBindingKey` and the export's public signature are likely to churn. Mitigation: build only the enablement slice (checked type, AR-3 refusals, and the export over constructed input), kept crate-private, until Q-2 fixes the key shape. File Q-1 and Q-2 before QSL-36 moves out of Backlog. | ADR-017 §3 AR-1 to AR-4, §5, §6 Q-1, Q-2, §8; `qsl-replay/src/spine.rs:139-143` |
| FND-003 | high | TK-2's blast radius is wider than §6 states. `selected_function` is read in eight `qsl-replay` files: `witness.rs`, `request.rs` (including `ReplayRequestWire` at `:252` and its size accounting at `:439`), `execute.rs:263`, `call_site.rs:225`, `result.rs`, `lib.rs`, `execute/tests.rs` and `spine/clause/tests/frame_replay.rs`. Outside QSL, it is fixed by QSpec FR-323-AC-6 and TC-278, by CG FR-024-AC-4 (the 14 packet members) and by C-12 (agent-ix/quire-contract-codegen#50). The envelope is also what the open QSL-20 (ARCH-G3) gate validates. Mitigation: take the narrow fix for G-2 first. `replay_frame` compares the envelope's `occurrence_key` and `clause_node` with the payload's and refuses `revision-mismatch` on a difference. That removes the silent disagreement with no wire change. Defer the `ReplaySelection` restructuring until a QSpec FR-323 item and the CG change are filed. | ADR-017 §1 PF-4, §6 TK-2; ADR-013 O-26, C-12, QC-8; QSpec FR-323-AC-6, TC-278; `qsl-replay/src/request.rs:252`, `:439` |
| FND-004 | medium | The recorded baseline can drift, and it can satisfy itself. QSpec definitions are draft (`1-draft.N`, ADR-014 N-1 moved `root` to `1-draft.2`), and an edition move may refuse a case on purpose. RF-7 gives no rule for that case. The only way to turn the gate green is to edit the recorded outcome, which is the edit-the-expectation failure that RF-6's oracle-independence rule exists to stop. Mitigation: a recorded outcome changes only with a new reference side (a new edition identity or `DefinitionRef`). The gate refuses a record whose reference side equals the current build's side. An intentional break is recorded as a new reference, and old cases are never rewritten. | ADR-017 §2 RF-2, RF-6, RF-7; ADR-014 N-1 |
| FND-005 | medium | RF-4's classification is brittle as the spine grows. `CompileRefusal` gains variants as stages land. `Omitted` maps to `unsupported_projection` ("an emission path IR's pinned v2 vocabulary does not hold yet", `qsl-replay/src/spine.rs:168-169`). Under RF-4 that code is `refused`, not `unsupported`. An IR pin bump that drops an emission form then shows up as a false #191 regression, and a child `Omitted` result shows up as a false #192 hold. Mitigation: classify with an exhaustive `match` over `CompileRefusal` and its cause codes, with no wildcard arm, so that a new variant fails to compile. Rule on `unsupported_projection` explicitly (unsupported, never refused). | ADR-017 §2 RF-4, RF-5; `qsl-replay/src/spine.rs:62-170` |
| FND-006 | low | TK-1's typed key moves the name lookup but does not remove it. A layer-6 entry receives `M::T::op` as text and has to turn `M` and `T` into a `DomainPackageRef` and a `DeclarationKey` before TK-1's lookup can run. §6 does not say where that happens. If each caller does it, the string formatting that G-1 removes comes back at each call site. Mitigation: TK-1 defines one resolver from the typed `OperationName` to (`DomainPackageRef`, `DeclarationKey`, `Identifier`) in `check`. It is the single PF-3 selection, with `CheckedGraph::operation_frame` as its only consumer. | ADR-017 §1 PF-3 G-1, §6 TK-1; `qsl-semantics/src/check/mod.rs:1807-1831`; `qsl-replay/src/spine/clause.rs:883-894` |

## Resolution

Resolved by the author on `spec/16-arch43-mapping`. FND-001: #192 waits on
Q-5; no hand-written parent outcomes. FND-002: #198's enablement slice stays
crate-private until Q-2 fixes the wire shape; the feature slice waits on Q-1
and Q-2. FND-003: TK-2 is the narrow consistency check with no wire change;
`ReplaySelection` is deferred to Q-4. FND-004: moot; there is no recorded
baseline. FND-005: classifications are exhaustive matches with no wildcard,
and `unsupported_projection` is ruled once (unsupported). FND-006: TK-1 adds
one resolver in `check`.

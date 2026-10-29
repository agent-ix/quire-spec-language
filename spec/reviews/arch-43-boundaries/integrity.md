---
id: SR-801
title: "Integrity review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries"
type: SpecReview
analysis: integrity
scope: "spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
# SR-801: Integrity review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries

## Summary

Reviewed the uncommitted ADR-017 draft on `spec/16-arch43-mapping` (base
`99e9b6c7`) and its amendments to ADR-011, ADR-012, ADR-013 and
`spec/spec.md`. The review checked five things:

- completeness against Linear QSL-16, QSL-36, QSL-39 and QSL-40, read as
  data;
- consistency with ADR-011 to ADR-015 and QSL FR-057, FR-105, FR-110 and
  FR-115;
- consistency with QSpec `main` at `4634f5f`: AD-003, FR-290, FR-301,
  FR-353, the V1 inventory rows V1-TOOL-011/012 and V1-BACK-021/022, and the
  diagnostics catalog;
- that each decision has one interpretation and can be implemented against
  the interfaces at HEAD;
- failure domains.

What holds:

- The protocol/frame mapping (PF-1, PF-2, PF-6) matches the landed code and
  ADR-013's owners.
- The claim that no protocol redesign is needed is supported: SEAM-3 is
  deleted, and no `qsl-*` crate imports `protocol_artifact`.
- G-1, G-2 and G-3 are real defects.
- RF-1's "test gates emit no claim" is consistent with ADR-011's answers to
  ADR-012 §13.1.
- AR-2's mapping of FR-353's effective-declaration key to O-03
  `DeclarationKey` agrees with QSpec AD-006.

What does not hold:

- Three findings are high:
  - The RF-4 classification is written against a type that spine `compile`
    does not return.
  - Every #191 case turns into a regression the first time the edition
    moves.
  - #198's claimed in-process delivery before Q-1 and Q-2 cannot be built
    under C-03 and O-02.
- The rest are gaps in the abstraction-relation keys and values, where the
  #192 reference outcomes come from, the verdict rules, and one amendment
  left out.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | RF-4 classifies over the ADR-013 T-4 types `Ok(Staged<CheckedPackage>)` and `StageFailure::{Refused{causes}, Limit, Fault}`. Spine `compile` returns `Result<Compiled, Box<CompileRefusal>>` (`qsl-replay/src/spine.rs:908-915`). `CompileRefusal::code()` reports only the first cause for `Check`, `Assembly` and `Profile`. It has no fault variant. Limits reach it as `stage_limit_exceeded` codes inside stage failures. `Omitted` maps to `unsupported_projection`, so RF-4 would call it `refused`. The foundation `StageFailure` has only `Limit` and `Refused(C)` (`qsl-foundation/src/diagnostic/stage.rs:235-240`). Fix: restate RF-4 over `CompileRefusal`. Map `stage_limit_exceeded` to incomplete, `runtime_invariant` to tool failure, and a refusal whose every cause (reading the `Check` refusal list) is `unsupported_construct` to unsupported. Classify `unsupported_projection` explicitly. Everything else is refused. Otherwise name the ticket that moves `compile` to T-4, and make #191 wait on it. | ADR-017 RF-4, §5 (#191); ADR-013 T-4; `qsl-replay/src/spine.rs:62-170` |
| FND-002 | high | An edition move turns every #191 case into a regression. Each spine source's header pins the `root` version and digest (for example `tests/it/compile_command.rs:331`). QSL FR-110 refuses a different version with `stale_dependency`/`revision-mismatch`. After a `root` revision move, every case recorded as admitted is therefore refused, and RF-5 reports every case as a regression. That contradicts RF-7's "the case's source may need no change". Editing the header changes the case's `RawSourceRef`, which RF-3 makes the case identity. Fix: define a case as its body plus a header that the gate supplies from `DefinitionLock::pinned()`, and name the case by the body's identity. Or define how a header `revision-mismatch` classifies. Then restate RF-3 and RF-7. | ADR-017 RF-3, RF-5, RF-7; FR-110 Behavior table |
| FND-003 | high | #198's claimed delivery before Q-1 and Q-2 cannot be built. With no Q-1 spelling there is no source for S2 to parse, so §8 item 1's "S3 check … do not wait" has no input. A checked relation with no v2 arm breaks ADR-013 C-03 ("A family with no v2 arm fails to compile; no partial package"), and E4 refuses omitted nodes (`CompileRefusal::Omitted`). AR-3 says rebinding changes `package_id`, but with no v2 member the relation cannot enter the `quire.checked-package-id/v2` preimage (O-02). QSpec FR-353-AC-5 would then fail in exactly the phase §5 calls "met in process before Q-2". Fix: make #198's S2, S3 and export wait on Q-1 and Q-2, and drop the in-process exit claim. Otherwise define an interim carrier and state the AC-5 gap and when it expires. | ADR-017 AR-3, AR-4, §5, §8; ADR-013 C-03, O-02; ADR-011 §2.3; QSpec FR-353-AC-5 |
| FND-004 | high | `FrameBindingKey` keys on the frame and anchor `NodeKey`s. FR-105 emits those nodes only when a state clause or protocol attempt names the operation (`check/state_clause.rs:55-61`; `check/mod.rs:1800-1806`: "the package then holds no frame node for it"). A frame binding for an operation that no clause names therefore has no key. The ADR does not say whether authoring a binding causes the nodes to be emitted. Fix: key a frame binding by (declaring type `DeclarationKey`, operation `Identifier`) and derive the frame and anchor at S3. Or state that a frame binding causes FR-105 emission for its operation, and amend FR-105 to match. | ADR-017 AR-2, AR-3; FR-105; QSpec FR-353-AC-1 |
| FND-005 | medium | The binding values have no types. `CheckedAbstractionRelation` holds "one binding per key", and AR-2 types only the keys. QSpec FR-353 maps an object to one state representation, a population to one collection representation, and a frame to a function's parameters, result and framed state. QSL-36 (data) adds "how a population and a frame are read from implementation values". No type holds those per-parameter, result and framed-field mappings. No equality exists for deciding that two bindings `conflicting-binding`. Fix: define a closed binding type per element kind (object, population, frame) whose members are `RustPath`s and identifiers, and state its equality. | ADR-017 AR-2, AR-3, §5; QSpec FR-353 Behavior |
| FND-006 | medium | Nothing produces the #192 reference outcomes. RF-2 says they are "captured under the reference side". But spine `compile` accepts only the `root` row as a header profile (FR-110), and no clause-family catalog is registered, so no QSL build compiles a unit under a parent profile such as `quire.state.core/v1`. The ADR does not say how a parent outcome is obtained, or which AD-003 edges V1 covers. Fix: name the source of #192's recorded outcomes, for example a hand-authored expectation citing the QSpec rule that refuses the case, or a QSpec corpus fixture. List the edges in V1 scope. If no edge can be compiled, record that #192 waits on clause-family profile selection. | ADR-017 RF-2, RF-3, §5 (#192); FR-110; QSpec AD-003 profile hierarchy |
| FND-007 | medium | The ADR reads #191 in one way but does not show why. QSL-40 (data) says "for every versioned spec pair in the corpus, v2 admits everything v1 admitted". V1-TOOL-011 says "a superseding specification edition admits every result its prior edition admitted". Both read naturally as two revisions of one authored specification and the results each admits. RF-2 instead compiles one source under two language or `root` revisions, and §6 asserts that the two readings are the same without a source. Fix: cite the QSpec text that fixes the meaning, or add a Q-4 asking QSpec, and mark #191 as waiting on it. | ADR-017 RF-2, §6; QSpec `docs/v1-capability-inventory.md` V1-TOOL-011; Linear QSL-40 |
| FND-008 | medium | A recorded `incomplete`, `unsupported` or tool-failure reference makes a gate unable to pass. The #192 row "incomplete, any → unresolved" and the verdict rule "any unresolved case is unsupported or incomplete" mean one case recorded as incomplete blocks success permanently, and RF-3 lets any class be recorded. The rows also overlap with no precedence: #192 "incomplete, any" against "any, tool failure", and #191 "not admitted, any" against "any, tool failure". Fix: refuse to record `incomplete` or tool-failure references, so such a case is re-recorded under a larger limit. Say whether an `unsupported` reference is allowed. State that the tool-failure row wins. | ADR-017 RF-3, RF-5 |
| FND-009 | medium | RF-5 says the gate's exit code is "the FR-301 code of its O-16 category", but it ranks violation above unsupported and incomplete. QSpec FR-301's highest-severity order is tool failure, invalid, unsupported, incomplete, violation, success. Fix: adopt FR-301's order, or keep this order, drop the FR-301 claim, and state the gate's own codes. | ADR-017 RF-5; QSpec FR-301 "Command and exit contract" |
| FND-010 | medium | The ADR-012 §3 `Relation` row still gives its check column as "relation totality over its declared domain; unbound-element refusal", and still lists "corpus-differential gates" as a `Relation` responsibility. ADR-017 moves the unbound refusal to the layer-4 export (AR-4), defines no totality check (AR-3), and says the gates have no `Relation` family hook (RF-1). Only the claim sentence was amended. Fix: amend the row's check and evaluate cells to match AR-3, AR-4 and RF-1, or define the totality check in AR-3. | ADR-012 §3 `Relation` row; ADR-017 RF-1, AR-3, AR-4 |
| FND-011 | medium | G-4 misstates the defect, and the coupling it claims is not shown. `format!("{}/{name}", owner.node)` (`model/observation/frame.rs:97-100`) builds QSpec's member identity `<owner>/<name>`, not FCD's spelling. Intake enforces that form for every member (`member_identity_name`, `model/intake.rs:832-835`), and the module doc at `:17-23` names FCD's form as the one it refuses. The derived key therefore equals the admitted key by contract. The fields come from the type's checked attributes, so "refuses a field the type does not declare" already holds. Fix: correct G-4, or drop G-4 and TK-4 under the ticket's rule against refactors with no demonstrated coupling. | ADR-017 G-4, TK-4, PF-7 closing paragraph; Linear QSL-16 acceptance |
| FND-012 | low | The §4 Tests cell for the abstraction relation maps QSpec FR-353-AC-1 to AC-5 as a block onto #198's three-part exit. AC-2 (Kani and Verus consume the relation through AD-010 negotiation) belongs to CG, and AC-4 and AC-5 match no exit step. Fix: map each AC to its owning ticket and test: QSL #198 for AC-1, AC-3, AC-4 and AC-5, and CG#84 or the Kani path for AC-2. | ADR-017 §4, §5; QSpec FR-353 Acceptance Criteria |

## Resolution

Resolved by the author on `spec/16-arch43-mapping`. FND-001: RF-2 classifies
by an exhaustive match over `ClauseDisposition` and a shared compile
classification over `CompileRefusal` through an all-causes accessor. FND-002
and FND-007: #191 now reads spec versioning as two revisions of one authored
specification, both compiled live in one build; the edition reading is a
rejected alternative. FND-003: #198 is split into an enablement slice and a
feature slice that waits on Q-1 and Q-2, and AC-5 moves to the feature
slice. FND-004: frame bindings key on `OperationKey`, with frame and anchor
derived (Q-7). FND-005: binding value types and their equality are defined
(AR-2, AR-3). FND-006: #192 waits on Q-5; hand-written parent outcomes are a
rejected alternative. FND-008: no recorded outcomes remain; comparison rows
are first-match with tool failure first. FND-009: RF-4 adopts QSpec FR-301's
order and codes. FND-010: ADR-012 §3's check and evaluate columns amended.
FND-011: rejected with reason: G-4 builds QSpec's `<owner>/<name>` member
identity, which intake enforces, so there is no coupling; G-4 and TK-4 (old)
are dropped (Alternatives). FND-012: AR-7 maps each AC to its owner.

An independent second pass found seventeen follow-ups (ADR-012 §13.5
citation, compile-stage limits, `ClauseDisposition` fields, the
`unsupported_projection` ruling, stale claim text in ADR-012 §2, ADR-013
O-16, FR-090, TC-153 and the model-linking matrix, FR-057-AC-10 scope, the
FR-353-AC-1 key (Q-7), a driver ask, `FrameBinding` carriers, the O-25
pointer, the PF-3 name shapes, bare ids, front matter, frame absence, the
comparison control, the case input, and the TC-463 pin ticket). All are
fixed in the same revision.

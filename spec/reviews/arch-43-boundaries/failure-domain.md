---
id: SR-802
title: "Failure-domain review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries"
type: SpecReview
analysis: failure-domain
scope: "spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
# SR-802: Failure-domain review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries

## Summary

Reviewed the uncommitted ADR-017 draft on `spec/16-arch43-mapping` (QSL
`main` at `99e9b6c7`) and its pointer amendments to ADR-011 §1, ADR-012 §3
and §13.5, ADR-013 R-06, O-08 and O-25, and `spec/spec.md`. The review
checked unstated failure modes, identity confusion, purity gaps and edge
cases. It covered the refinement gate classification, recorded-baseline
staleness, frame replay identity, abstraction-relation keys, inheritance
and unbound-refusal scope. Code claims were opened at file and line. QSpec
was read at `origin/main` `4634f5f` (FR-353, AD-003, FR-110 context, the
native-diagnostics catalog). Linear ticket text was not relied on.

What holds:

- PF-1's identity table matches the code. G-1 (`check/mod.rs:1807-1831`
  matches a formatted string), G-3 (`check/identity.rs:287-348` has no
  consumer outside its re-export at `check/mod.rs:191-192`) and G-4
  (`model/observation/frame.rs:90-105` formats the member key) are real.
- The refinement gates never settle `proved`, and an unresolved case is
  never promoted to holds.
- AR-2's use of the O-03 `DeclarationKey` rather than O-05 `EffectiveId`
  agrees with QSpec FR-353 and ADR-013 O-03.

What does not hold:

- Three findings are high. The #192 current side is not the child profile
  for most AD-003 edges. RF-4 classifies over a result type spine `compile`
  does not return. AR-2's anchor key collides across the parameters of one
  frame.
- The rest are gaps in cause classification, per-case table precedence,
  baseline staleness, inheritance, replay-selection consistency, the
  staleness path of frame replay, and the unbound refusal's shape.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | #192 does not compare parent with child for most AD-003 edges. RF-3 says the current side is "the running build's spine `compile`", and RF-2 says the parent's outcome is "captured under the reference side". FR-110 lets a header select only the `root` row (`quire.value.complete/v1`), so every current-side compile runs under `root`. That is the child only for the edge "complete values requires state core". For "state queries requires state core" and "state graph requires state queries", neither side is ever compiled, so the gate compares a parent record with a `root` outcome. No QSL build can compile a non-`root` parent either, so "captured" is impossible and each parent outcome is hand-written expected data that no tool checks. Fix: limit #192 to edges whose child is a definition the build compiles under (`root`, and `quire.model.complete/v1` through I1 if so ruled). State that parent outcomes are authored expected data, not captured. Name each other AD-003 edge as out of V1 scope, or give it an owner. | ADR-017 RF-2, RF-3, RF-7; FR-110 (lines 31-35, 72-87); QSpec AD-003 "Accepted profile hierarchy" |
| FND-002 | high | RF-4 classifies `Ok(Staged<CheckedPackage>)` and `StageFailure::{Refused, Limit, Fault}`. Spine `compile` returns `Result<Compiled, Box<CompileRefusal>>` (`qsl-replay/src/spine.rs:908-915`). `CompileRefusal` has no limit or fault variant. Limits arrive nested: `FormsFailure::Limit`, `UnitIntakeCause::Limit`, `AssemblyCause::TypeLimit` and `ImportRefusal::DepthLimit` (`spine.rs:213-353`, `:764-838`). `CompileRefusal::code()` returns only the first cause's code for `Profile`, `Assembly` and `Check`, and `Source` holds only the first diagnostic (`spine.rs:62-170`). So the rules "every cause's `CatalogCode` is `unsupported_construct`", "limit is incomplete" and "fault is tool failure" cannot be computed as written. `StageFailure` itself has only `Limit` and `Refused` (`qsl-foundation/src/diagnostic/stage.rs:235-240`), not T-4's three variants. Fix: write RF-4 over `CompileRefusal`. Map each variant, and each nested limit cause (`stage_limit_exceeded`), to a class. Say which cause list "every cause" reads for each variant, or add an all-causes accessor to #191's scope. | ADR-017 RF-4, §5 (#191); ADR-013 T-4; `qsl-replay/src/spine.rs:62-170`, `:908-915` |
| FND-003 | medium | Keying the unsupported class on `CatalogCode` alone mixes up three causes. (1) QSL's own gap `unsupported_construct`/`not-yet-implemented` (`check/refusal.rs:244`, `:262`) is not the catalog's profile prohibition (`declaration-form`/`expression-form`, native-diagnostics row `unsupported_construct`). (2) `CompileRefusal::Omitted` (an IR v2 vocabulary gap) reports `unsupported_projection` (`spine.rs:168`), so it classifies as refused. (3) Setup refusals, meaning a missing domain package (`Intake`), a bad dependency input (`DependencyInput`/`Import`) or a header profile that does not resolve (`Profile`), also classify as refused. In #192, (2) and (3) make a parent-refused case "hold" with no profile meaning in play. In #191, (1) and (2) report as a typed regression when the cause is really an implementation gap. Fix: classify by (code, cause). Unsupported is `unsupported_construct` with a catalog cause. `not-yet-implemented` and `Omitted` form a QSL-gap class that resolves to unresolved (unsupported). Setup refusals are tool failure, because the case is broken. | ADR-017 RF-4, RF-5, alternative 3; `qsl-semantics/src/check/refusal.rs:244,262`; `qsl-replay/src/spine.rs:150-170`; native-diagnostics `unsupported_construct` row |
| FND-004 | medium | The RF-5 per-case tables overlap and give no precedence. Only the gate verdict says "first match wins". In #191, `R` refused with `C` tool failure matches both "not admitted / any → not applicable" and "any / tool failure → tool failure". In #192, `R` admitted, unsupported or incomplete with `C` tool failure matches two rows. A tool failure hidden by a "not applicable" row lets a broken case pass silently. Fix: say that the per-case rows are matched in order with the tool-failure row first, or make the rows disjoint. Add a control case (RF-6 item 3) whose recorded `R` is not admitted and whose current compile is a tool failure. | ADR-017 RF-5, RF-6 |
| FND-005 | medium | Recorded outcomes can go stale undetected. RF-3 names a case by its `RawSourceRef`, but nothing binds a recorded outcome to that digest. If a case's source, domain package or library bytes change, the old record is compared against new bytes. After an edition move (RF-7), the prior edition can no longer be compiled, so an edited case cannot be re-recorded. Re-recording it under the current edition would make the gate compare the build with itself. Fix: store each record with its case's `RawSourceRef` digest and the digests of its package and library inputs. A mismatch at gate time is tool failure. After an edition move, a changed case is retired or re-recorded only from a build of its own reference edition, never from the current build. | ADR-017 RF-2, RF-3, RF-7 |
| FND-006 | high | AR-2's FR-012 anchor key is (`anchor` `NodeKey`, role) "for each bound parameter, result and framed field". Every parameter observed `Pre` shares one key, and every framed field observed `Post` shares another. So binding two parameters of one operation is either a false `invalid_model_binding`/`conflicting-binding` under AR-3's "one binding per key", or leaves the binding ambiguous. FR-353 keys a frame binding by its anchor identities "the frame's parameters, result and framed state are read under". The bound element is part of the key. Fix: key each anchor binding by (`anchor`, role, element). The element is the parameter's position or `Identifier`, `Result`, or the framed field's O-06 (declaring node, member name). State that AR-3 uniqueness is per that triple. | ADR-017 AR-2, AR-3; QSpec FR-353 Behavior, FR-353-AC-1 |
| FND-007 | medium | Inheritance is unstated for bindings. `operation_frame` gives an inherited operation its declaring type's frame (`check/mod.rs:1800-1804`). `FrameBindingKey` keys on the declaring type, so a subtype whose implementation function differs cannot be bound separately. FR-353 requires exactly one implementation function per frame. AR-4 references "each field member's declaring node", so a claim over subtype `S` that reads only an inherited field references the supertype `T`. `T` must then be bound, even if abstract, and `S`'s binding is never consulted. Fix: state the rule. Either a binding keys on the effective (context) type and an inherited frame is re-bound per concrete type, or bindings are per declaring type and a subtype with its own representation is refused at S3 with a named cause. Also say whether an item over `S` references `S`, `T` or both. | ADR-017 AR-2, AR-4 "Referenced elements"; `qsl-semantics/src/check/mod.rs:1800-1831`; QSpec FR-353 Behavior |
| FND-008 | medium | PF-6 and PF-3 describe a staleness path the code does not take. `replay_frame` checks `package_id` before it resolves the operation or compares identities (`qsl-replay/src/execute/frame.rs:173-188`). A new domain package version changes the `package_id`, so an old packet refuses `PackageIdMismatch` (FR-098) and never reaches `revision-mismatch`. PF-6 row 1 ("an old packet refuses `revision-mismatch`") is wrong. The identity comparison fires only when the `package_id` matches, which is an internally inconsistent packet. The last alternative's reason ("a stale packet's ids name nothing in the recompiled package") has the same error. Fix: PF-6 row 1 should give the effect as the FR-098 `package_id` refusal. PF-3 and PF-4 should say that the E9 identity comparison detects a packet whose members disagree with a same-`package_id` recompile. Restate the alternative's reason as consistency, not staleness. | ADR-017 PF-3, PF-6, Alternatives (last); `qsl-replay/src/execute/frame.rs:167-188`; FR-098 |
| FND-009 | medium | After G-2, a frame packet's identity rests on envelope members whose meaning depends on the payload type, but no rule ties them together. `WitnessEnvelope<FrameCounterexample>` with `ReplaySelection::Function`, or a function payload with `ReplaySelection::Frame`, can be constructed. PF-4 also reuses `clause_node` as the frame node, and `obligation_identity` (O-09, clause identity) is not defined for a frame record. Fix: make the selection variant a function of `P` (for example an associated selection type on `FamilyPayload`, so a mismatch cannot be constructed). If it stays a runtime value, refuse a mismatch with a named cause. State what `obligation_identity` and `clause_node` hold for a frame packet in the amended O-25. | ADR-017 PF-4, TK-2; ADR-013 O-25, O-09; `qsl-replay/src/witness.rs:343-359` |
| FND-010 | low | AR-4's unbound refusal names "the element's key", in the singular. An item that references several unbound elements gets a refusal whose content depends on which one is found first, and that order is not stated (R-05). Fix: the refusal names every unbound element of the item, ordered by `DeclarationKey` (then `FrameBindingKey`) order, each with its owning `DomainPackageRef` identity. | ADR-017 AR-4; ADR-013 R-05; QSpec FR-353-AC-3 |
| FND-011 | low | ADR-012 §3's `Relation` row still lists "relation totality over its declared domain" as a check. AR-3 and AR-4 define no totality check. A relation that leaves elements unbound is admitted, and only referencing items refuse. §7 does not amend the check column, so it is unclear whether S3 refuses a relation that does not cover every declaration. Fix: in §7, amend the ADR-012 §3 check column to AR-3's refusals plus AR-4's per-item unbound refusal. State that S3 does not refuse an incomplete relation (FR-353 scopes the refusal to the item). | ADR-017 AR-3, AR-4, §7; ADR-012 §3 `Relation` row |

## Resolution

Resolved by the author on `spec/16-arch43-mapping`. FND-001: #192 waits on
Q-5 (RF-3). FND-002: RF-2's compile classification is an exhaustive match
over `CompileRefusal`, naming each nested limit cause, with an all-causes
accessor in #191. FND-003: classification is by code and cause;
`not-yet-implemented`, `Omitted` and `unsupported_projection` are unsupported;
setup refusals are tool failure. FND-004: per-case rows are first-match with
tool failure first, and a unit test over every class pair covers hidden tool
failures. FND-005: moot; there is no recorded baseline. FND-006: the frame
binding keys on `OperationKey`; parameters, result and framed state are
separate carriers (AR-2). FND-007: operations bind at the declaring type; an
item references the receiver's static type (AR-2, AR-4). FND-008: PF-3, PF-6
and the last alternative describe a consistency check after the `package_id`
check. FND-009: PF-4 fixes a frame packet's `clause_node`, occurrence key and
obligation identity; the selection member is Q-4; ADR-013 O-25 has a
pointer. FND-010: the unbound refusal names every unbound element in key
order. FND-011: no totality check; ADR-012 §3 amended.

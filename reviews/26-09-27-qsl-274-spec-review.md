---
id: SR-758
title: "Spec review of QSL-274 FR-120 model simulation and the FR-101 engine amendment"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md; spec/test-cases/TC-471-model-successors-follow-operations-arguments-and-frames.md; spec/test-cases/TC-472-invariant-violations-are-recorded-and-undecided-expansions-stop.md; spec/test-cases/TC-473-model-effects-are-trace-data-and-ambient-reads-refuse.md; spec/spec.md; spec/tests.md; spec/usecase/US-003-evaluate-bounded-state.md; context read: FR-097, FR-103, FR-104, FR-106, FR-107, FR-114, FR-115, qsl-semantics/src/model/population.rs, qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/key.rs, qsl-eval/src/simulation/explore.rs, qsl-eval/src/value/expression/mod.rs, qsl-package/src/checked.rs; QSpec origin/main FR-181, FR-013, FR-151, state-contract.md, native-diagnostics.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-471
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-472
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-473
    type: reviews
---
## Summary

Ticket: QSL-274 (A05-4). PR: quire-spec-language#496. Spec-only.
Analyses: integrity, scope and failure domain, interfaces, and EARS/testability
of the ACs, checked against QSpec FR-181, FR-013 and FR-151 on origin/main and
against the code on main.

Sound:
- Precondition reading. FR-120:234-244 states it: own `pre` clauses are
  conjoined, and that conjunction is OR-ed with the inherited effective
  preconditions. This matches FR-151:86-94 read with its postcondition
  sentence, TC-196 D06 ("absent (true) clause disjoined with PA") and
  `checked_dispatch.rs` `effective_terms`. FR-120-AC-2 and TC-471 step 2
  (`A` and `B`, enabled only at `v1`) test the conjunction. The disjunction
  cannot be reached on the spine today. Intake sets operation `redefines`
  to `None` (`qsl-semantics/src/model/intake.rs:1444`).
- Frames. FR-120:268-302 reuses FR-114's frame binding and FR-106 check 11
  through `decide_frame`. Its candidate construction only enumerates
  candidates; `decide_frame` decides each one. The `modifies` and redefines
  rule matches `field_write_covered` (`population.rs:1348-1354`).
- All catalog pairs FR-120 names are in `native-diagnostics.md` `1-draft.8`
  (:79, :84, :88, :89, :91, :95, :113). The four S3 refusals in AC-10 match
  FR-104:99-153.
- Emission. `evaluate_clause` is implemented on the S4 `CheckedPackage`
  (`qsl-eval/src/value/expression/mod.rs:441`), and `CheckedPackage::link`
  exists (`qsl-package/src/checked.rs:312`). FR-106 admission reads domain
  package bytes, not emitted bytes. Simulation needs no S5 emission.
- The AC fixture numbers check out by hand: AC-3 gives 19 and 3, AC-5 gives
  3/2/2 and then 3/5, and AC-6's `max_candidates` gives 3 > 2.
- Reference triples and the transition-identity shape match FR-181.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Every AC fixture uses bounded integers: `value: Int[0, 2]`, `label: Int[0, 1]` and `setTo(n: Int[0, 2])`. Spine intake cannot read an `Int[lower, upper]` field or parameter type yet, because FR-056's `value-type/v1` scalar reader is missing (QSL-289, In Progress, PR #498; FR-103-AC-1 and FR-104-AC-1 are marked Unverified for this reason). If a native `Integer` is substituted, as QSL-276 and QSL-277 did, `domains()` is unbounded and every AC returns `RequiresBound`. FR-120 and TC-471 to TC-473 do not name this dependency. Fix: add QSL-289 (FR-056) to Dependencies and Status, and add the blocked-by edge in Linear. Alternatively, rebuild the fixture on `Boolean`, `Option` and `Reference` fields. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:392-396,412-435; spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:96 |
| FND-002 | medium | The FR-101 amendment is written only in FR-120. FR-101's own normative text still contradicts it. FR-101:84-102 has `explore_request -> Result<Outcome, NotSimulated>` and `sample_request -> Trace<S::TransitionId>`, with no findings and no finding type parameter. FR-101:68-74 lists three outcomes. FR-101:230 says "`Outcome::category()` stays FR-097-AC-5's map". FR-097-AC-5 maps only Exhaustive, Bounded and Cancelled. The trait at `explore.rs:25-47` is FR-101's. Result: FR-101 and FR-120 are two sources of truth for one trait, and FR-101 was the one that was implemented. Fix: edit FR-101's signatures, Outputs, Stopped outcomes and replay errors, and FR-097-AC-5's map, in this PR. Leave FR-120 pointing at them. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:68-74,84-102,224-230; spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md:73; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:102-156 |
| FND-003 | medium | A trace that ends with `StopReason::Stopped(cause)` can never replay. Sampling expands the last state, and that expansion returns `ExpansionStop`. Replay expands each state again, and "an `ExpansionStop` refuses `ReplayError::Stopped`". This contradicts FR-181-AC-5: every generated trace replays with the same terminal disposition. Fix: when the recorded stop reason is `Stopped(c)`, replay succeeds if re-expanding the last state stops with the same `c`. It refuses `ReplayError::Stopped` only for a stop at any other step, or with a different cause. Say the same for the `StepLimit` case, where the last state's expansion stops. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:147-153 |
| FND-004 | medium | "Declaration identity" has no defined text form. `DeclarationKey` is `{package, node}` (`qsl-semantics/src/model/key.rs:84-89`). FR-120 uses it as one `text` for the `semantic` map key (:200) and for the record `name` (:201-203). It uses the node identity alone for the transition's operation name (:219-223). With more than one domain package selected (FR-056 `packages`), two node identities can be equal and transition identities can collide. AC-4 and TC-471 step 4 expect the placeholder `<counters' declaration identity>`, so the state-key bytes are not pinned. Fix: define one string form, for example FR-081's qualified effective identity or `package`, a separator, then `node`, and use it for all three. Give one literal expected `semantic` member, or its digest, in AC-4. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:197-209,219-223,404; spec/test-cases/TC-471-model-successors-follow-operations-arguments-and-frames.md:63-68 |
| FND-005 | medium | FR-120 does not name the in-memory seams it depends on. (a) `decide_frame` is `pub(crate)` in `qsl-semantics` (`population.rs:1440`), but `ModelSystem` lives in `qsl-eval`. FR-115 reaches `decide_frame` only through FR-106 document admission. (b) `evaluate_clause` takes FR-106 `AdmittedObservations`. Its `Observation` carries `identity: DocumentRef`, `populations` completeness flags and `usage` (`observation.rs:267-335`). FR-120 does not say how a synthesized state fills them. Fix: pick one path. Either expose a public `qsl-semantics` frame and observation constructor over in-memory objects, or render each state as a snapshot document and admit it. Name that path in FR-120. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:246-249,293-302,327-328 |
| FND-006 | medium | The anchor is defined only for initial states. QSpec requires "exact named initialization/handler binding" for invariants and says an "arbitrary current snapshot is not a substitute" (state-contract.md:189, :232). FR-120 evaluates every invariant at every expanded state "as the `current` observation" (:327-328). For a reached state it does not say which initialization or handler observation that is. It also cites state-contract.md for "An initial state is an initialization observation" (:173-174), but that section does not say so. Fix: state the anchor a reached state's current observation carries. Mark both it and the initial-state mapping as QSL's choice, per FR-120's own rule at :56-57. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:170-174,322-328 |
| FND-007 | low | FR-120:238-244 presents the own-clause conjunction as FR-151's rule. FR-151:88-90 says "the disjunction of its own precondition clauses with the effective preconditions of the members it redefines" and does not say how several own clauses combine. The reading is right, but it is an inference. Also, "(FR-103 intake sets none)" is backed by `intake.rs:1444`, not by FR-103's text. Fix: mark it as QSL's reading and cite the code, or get QSpec to add one sentence to FR-151. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:236-244 |
| FND-008 | low | Contract clauses stop at the first non-true result, in name order. So clause names decide whether a `ContractUndetermined` finding appears. If `A` is `Undefined` and `B` is `Completed(false)`, a finding is recorded, although the conjunction is false. With the names swapped, no finding is recorded. The result is deterministic, but the finding depends on names. Fix: state that this is intended, or evaluate every applicable clause and record `Undetermined` only when no clause is `Completed(false)`. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:246-262,306-316 |
| FND-009 | low | `sample_model` is used (:362) but never declared: no signature and no relation to `sample_request`. :362-368 says `ModelTrace` holds a `StepEffect` per step, and also says `StepEffect` is "recomputed from the trace by replay". TC-472 uses `sample_request`, and TC-473 uses `sample_model`. Fix: give the signature, and say whether `StepEffect` is stored or derived. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:95-98,362-368 |
| FND-010 | low | The domains' ids are not given. FR-101 `classify_extent` keys each domain by a `WireNodeId`. FR-120 does not say which node's id a population domain or a parameter domain carries. AC-7 and TC-472 step 4 expect `RequiresBound` "naming" the domain. Fix: name the node for each (the population declaration, the parameter node), for example as `WireNodeId::from_digest` of its S4 `NodeKey`. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:343-352,407 |
| FND-011 | low | AC-10 requires the refusal "at the source span of that form" for `self` outside a state clause. FR-104:99-100 names no location for that refusal; the other three forms do name one. Fix: add "at `self`" to FR-104, or cite the code that already does it. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:378-379,410; spec/functional/FR-104-check-state-clauses.md:99-100 |
| FND-012 | low | Some sentences name an alternative or an absence: "not the unit's alias spelling" (:222), "FR-115's `Frame` selection needs that operation's frame node, and simulation does not" (:273-274), "FR-120 defines no frame semantics of its own" (:433), and "so FR-105 and QSL-279 do not gate it" (:435). Fix: restate them positively. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:222,273-274,433-435 |
| FND-013 | low | A state is checked against invariants only when it is expanded. A violating state that is discovered but left in a `Bounded` or `Stopped` frontier has no finding. A stopped expansion's `ExpansionStop` carries no findings, so the invariant findings already computed for that state are dropped. This is defined by the types, but the text does not say it. Fix: state both, one sentence each. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:127-146,322-341 |

## Verdict

Request changes. FND-001 to FND-006 must be fixed in this PR. FND-001 can be
fixed by declaring the QSL-289 dependency. FND-007 to FND-013 are one-line
edits and belong in the same round. Nothing is structurally wrong. The
precondition reading, the reuse of the frame decision, the catalog codes and
the S4-only scope are all sound.

## Dispositions

Round 2. Each outcome below was checked against the spec text at the fix
head and, where the fix cites code, against that code on main.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-120 Dependencies :512-516 and Status :533-535 name QSL-289 (PR #498). TC-471 to TC-473 are marked runnable once QSL-289 lands. Linear shows QSL-289 blocks QSL-274. |
| FND-002 | fixed | FR-101 carries the trait, `Exploration`, `Trace<T, F>`, "Findings and stopped expansions" and AC-12 to AC-14. FR-097-AC-5 maps `Stopped`. |
| FND-003 | fixed | FR-101's replay rule accepts a stop at the last state with the same cause. FR-101-AC-14 and FR-120-AC-8 test it. |
| FND-004 | fixed | FR-120:140-148 defines the identity text as the JCS `$defs.DeclarationKey` preimage (`key.rs` `wire()`). It is used for population keys, record names, operations and anchors, and AC-1 and AC-4 give it literally. `<U>` and `<E>` are covered by R2-FND-002. |
| FND-005 | fixed | FR-120:216-290 defines the public seam `qsl_semantics::model::state` (`StateModel`, `ModelState`, `StateDelta`). `check_frame` calls `population::decide_frame`, the one frame decision. The `Observation`, `DocumentRef` and `AdmittedObservations` construction is spelled out field by field. |
| FND-006 | fixed | FR-120:206-214 gives initial states `{initialization, name}` and reached states `{handler, <operation>}`, both labelled as QSL's choice. The finding records the anchor. See R2-FND-003 for the wording on sampled traces. |
| FND-007 | fixed | FR-120:317-328 marks the own-clause conjunction as QSL's inference and cites `checked_dispatch.rs:477` and `intake.rs:1444`. |
| FND-008 | fixed | FR-120:332-349 evaluates every applicable clause, and the result depends only on the set of outcomes. FR-120-AC-9 tests it with `Never` and `Aaa`. |
| FND-009 | fixed | FR-120:91-134 gives the `sample_model` signature. It samples and then replays, and `ModelTrace.effects` is derived by replay. |
| FND-010 | fixed | FR-120:432-437 derives each root's `WireNodeId`. TC-472 step 4 names both roots. |
| FND-011 | fixed | FR-120:461-463 cites `check.rs:1667-1673`. `self_reference` refuses at the `self` location; checked. |
| FND-012 | fixed | the listed sentences were restated positively (:302, :357-360, :526-529). |
| FND-013 | fixed | FR-120:415-417 and FR-101 "Findings and stopped expansions" state both. FR-101-AC-13 tests them. |

Round 2 findings:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| R2-FND-001 | medium | FR-120-AC-7 and TC-472 step 3 expect `wrong_snapshot`/`wrong-anchor` for an initial snapshot whose anchor kind is `other`. The reader only accepts `initialization` and `handler` and refuses anything else with `wrong_kind("anchor")`, which is `invalid_runtime_input`/`wrong-value-kind` with field `anchor` (`qsl-semantics/src/model/observation/document.rs:525-528`, :401-403). That refusal comes at FR-106 check 1, the read, which `admit_initial` runs before check 3. The test as written fails. Fix: expect `invalid_runtime_input`/`wrong-value-kind` (field `anchor`) for that row, or drop it; the `{handler, validate}` row already covers `wrong-anchor`. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:494; spec/test-cases/TC-472-invariant-violations-are-recorded-and-undecided-expansions-stop.md (step 3, expected) |
| R2-FND-002 | low | FR-120-AC-4 and TC-471 step 4 leave `<U>` and `<E>` as placeholders and do not say where the test gets them. If the test reads them from the simulator's own key encoding, those two members are asserted against themselves. Fix: take them from the `ObjectReference` that FR-106 admission gives `c1` in `s0`'s admitted `Observation`, which is independent of the simulator. Alternatively, pin the two hex values, as FR-101-AC-2 pins digests. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:491; spec/test-cases/TC-471-model-successors-follow-operations-arguments-and-frames.md (step 4, expected) |
| R2-FND-003 | low | The anchor rule says "the transition by which exploration or the trace first reached it". That holds for exploration, whose engine keeps the first visited state (`explore.rs:219-235`). A sampled trace does not coalesce, and `successors` is pure, so a revisited state in a trace carries the anchor of the transition that produced it at that step, not the first one. Fix: for a trace, say the anchor is that step's transition. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:209-213 |
| R2-FND-004 | low | `check_frame` calls `decide_frame` "with the computed delta as the declared delta". But `decide_frame` takes the declared lists as input (`FrameDecision.declared_created`/`declared_deleted`, `population.rs:1374-1385`). Fix: say that `check_frame` first computes each population's created and deleted keys as the key-set difference and passes those. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:265-272 |

Verdict at round 2: all 13 round-1 findings are fixed. Fix R2-FND-001, a
one-row expected-code edit, before merge. R2-FND-002 to R2-FND-004 are
one-line edits for the same round.

Round 3. This file was renumbered from SR-753 to SR-758, because #498
merged SR-753 on main. Its round-2 findings are listed here as R2-FND-nnn.

| FND | outcome | reason |
| --- | --- | --- |
| R2-FND-001 | fixed | FR-120-AC-7 (:499) and TC-472 step 3 now expect `invalid_runtime_input`/`wrong-value-kind` with field `anchor` for anchor kind `other`, citing `document.rs:525-528`. That matches `wrong_kind` at `document.rs:401-403`. |
| R2-FND-002 | fixed | in AC-4 and TC-471 step 4, `<U>` and `<E>` now come from the `ObjectReference` that `StateModel::admit_initial` gives `c1`, which is independent of the simulator's key encoding. |
| R2-FND-003 | fixed | FR-120 limits "first reached" to exploration. A sampled step is anchored to its own transition. |
| R2-FND-004 | fixed | `check_frame` first computes each population's created and deleted keys as the key-set difference, then passes them as `FrameDecision.declared_created` and `declared_deleted`. |

Verdict at round 3: no open findings. Approve.

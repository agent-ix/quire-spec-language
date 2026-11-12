---
id: ADR-036
title: "Protocol analysis: the local-projection and realizability engine"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-519
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-053
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-054
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-170
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-172
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-174
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-175
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-288
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-360
    type: depends_on
---
# ADR-036: Protocol analysis: the local-projection and realizability engine

Status: Proposed (QSL-1889, split from QSL-1868 gaps G1 and G2). This record
decides the engine; it adds no requirement of its own. Slice 1 of the
decomposition in section 12 writes the QSL requirements and test cases.

## Context

**What exists.** QSL has vocabulary and routing for the two capability kinds
and no decision procedure. `local-projection` and `realizability` are
`Capability` kinds of FR-290 family `protocol`
(`qsl-semantics/src/check/capability.rs`, `capability_kinds!`), have
`ClaimKind` and `same_kind` arms in `qsl-replay/src/planner/key.rs` and
`qsl-route/src/lib.rs`, and are listed among the protocol kinds in
`qsl-analyze/src/analyze.rs`. A request for either settles
`NegotiationCause::UnsupportedRequestedCapability(kind, backend)`, code
`unsupported_projection` / `unsupported-requested-capability` (FR-057-AC-15),
because no candidate serves it. `NegotiationCause` has exactly three variants
(`UnknownBackend`, `AmbiguousBackend`, `UnsupportedRequestedCapability`).

**What QSpec requires.** QSpec FR-174 projects a checked global protocol
onto selected roles, preserving observable sends and receives, causal order,
queues and obligations; unobservable choice or an incompatible merge yields a
minimal global-path counterexample, an exhausted bound or an unsupported
construct yields a non-conclusive result with no substitute graph (FR-174-AC-2,
AC-3), and a branch may merge only on compatible next actions or explicit
distinguishing evidence (FR-174-AC-5). QSpec FR-175 asks whether compliant
local strategies realize the global protocol: a game in which the environment
chooses admitted external events and channel outcomes and each role chooses
from its local history only; witnesses map classes of indistinguishable
histories to actions, counterexamples are two indistinguishable histories
needing incompatible actions or an environment play that defeats every choice
(FR-175-AC-1, AC-2, AC-4, AC-5); a missing premise or a limit is
`inconclusive`, never realizable (FR-175-AC-3). Both results are derived
artifacts with a structural key and no digest (FR-174-AC-6, FR-175-AC-6, as
merged by QSpec PR 650): FR-174's key is the FR-288 qualified definition key,
the role, the profile, the channel, choice and visibility premises as selected
(FR-054) and the bounds; FR-175's is the definition key, the strategy profile,
the role knowledge and visibility, the channel and environment premises, the
bounds and the producing analyzer identity and version. The key is carried on
the result; an absent member is refused
`invalid_runtime_input`/`missing-member` naming it; the analyzer is
deterministic for a fixed key.

**What quire-protocol needs.** quire-protocol ADR-010 "QSL gaps" lists G1 and
G2 as the two decision procedures it does not build and will not work around:
it dispatches both kinds to a QSL backend, binds the communication and
visibility premises to each disposition (its FR-030), carries the witness with
trace-position correspondence to the linked graph (its FR-152 role), and
returns `unsupported` while no backend exists. It imports the checked protocol
through the FR-519 read surface (`qsl_package::protocol_view`), which now
includes `choice_visibility` (FR-519-AC-20, the evidence of QSpec
FR-172-AC-6). Projection and realizability corpora in quire-protocol are
blocked on this engine.

**What the other engines settled.** ADR-018 (EN-1), ADR-024, ADR-026 and
ADR-028 (EN-5) put in-process engines in layer A (`qsl-analyze`) and their
verdict types, replay arm and settlement map in layer 6 (`qsl-replay`). ADR-027
fixes one protocol transition system, in `qsl-eval`, with the FR-170 queue
semantics. The owner's standing rule is one evaluator and no duplicate
semantics.

## Decision

### 1. Placement

| ID | Rule |
| --- | --- |
| PA-1 | **Engine location.** The engine is a new module `protocol_analysis` of the layer-A crate `qsl-analyze` (ADR-011 section 6.1), beside `model_check`, with submodules `premises`, `projection`, `realizability` and `witness`. No new crate and no new layer edge: layer A already depends on layers 6, 5, 4, 3, R, F, C, E, SV and K, which is all the engine reads. It runs in stage S6c as the other native engines do. |
| PA-2 | **Types and checking in layer 6.** The key, bounds, premises, outcome, witness and counterexample types, the settlement map and the independent witness checker live in `qsl-replay` module `protocol_analysis`, as ADR-028 SP-2 places EN-5's types, so a reader needs no `qsl-analyze` code to check a result (ADR-018 LA-3 pattern). |
| PA-3 | **One graph reader, one semantics.** The engine reads the checked protocol only through `qsl_package::protocol_view` (FR-519): `protocol_view`, `causal_relation`, `channel_endpoints`, `delivery`, `queue_capacity`, `choice_visibility`. These are the accessors quire-protocol imports, so both sides name the same nodes. Where the engine needs a fact the surface lacks, the slice that needs it adds an accessor to `protocol_view` under FR-519; the engine does not read `qsl-semantics` internals. Channel queue behaviour, duplicate/loss alternatives and overflow come from the one `qsl-eval` protocol transition system (ADR-027, QSpec FR-170); `protocol_analysis` does not restate them. |
| PA-4 | **Backend boundary.** The engine is one provider manifest advertising `local-projection` and `realizability`. It adds no `NegotiationCause` variant: negotiation still returns the three existing causes when no candidate serves a kind. An unsupported construct found by a served engine is an engine result (section 6), not a negotiation cause. quire-protocol receives results as the driver's disposition and witness envelope, as it does for the other native engines, so no QSL type enters a backend repository (ADR-011 FB-05). |

### 2. Inputs and premises

| ID | Rule |
| --- | --- |
| PA-5 | **Request.** A projection request is (definition key, role, profile, premises, bounds); a realizability request is (definition key, strategy profile, premises, bounds). The definition key is the FR-288 qualified key type already used by the obligation activation binding (QSpec FR-479); the engine reads the global protocol through it and through no display name, path or span. |
| PA-6 | **Premise record.** `ProtocolPremises` has four groups, each explicit: (a) channel premises per channel: the five FR-054 members (sender, receiver, message type, ordering `unordered`/`fifo` with its key expression, delivery policy or `[lo,hi]` interval) plus the queue's capacity and overflow policy; (b) choice premises: per `choice` control, `owner` and `visible` as `choice_visibility` returns them (FR-172 "Choice visibility carrier", FR-053); (c) role knowledge and visibility: declared shared facts a role may condition on, beyond its own observations; (d) environment premises: the admitted external events and channel outcomes. Groups (a) and (b) are populated from the linked package through the PA-3 accessors; (c) and (d) are authored by the claim. |
| PA-7 | **How each premise acts.** Ordering: `fifo` orders deliveries only within the pair of exact channel key and evaluated key value (FR-054-AC-2); `unordered` imposes none. Delivery: a send has one alternative per delivery count `d` in the policy's range, `d = 0` its only loss and `d > 1` its only duplication (FR-054, FR-170-AC-7); `unknown` admits both and supports no delivery conclusion. Capacity and overflow bound each queue (literal capacity only, section 6). Merge visibility: a role distinguishes a choice's branches only through being the `owner`, being in `visible`, or an earlier received message that carries the distinguishing outcome (FR-053-AC-1); nothing else counts. Environment: realizability quantifies over exactly the admitted events and outcomes (FR-175-AC-5). |
| PA-8 | **No inferred premise.** A premise the request does not state is never taken from a default, from a successful sample or from replay history (FR-175 "Behavior"). If the decision needs a group (c) or (d) premise that is absent, the result is `inconclusive` (FR-175-AC-3). An absent *key member* is a different failure and is refused (PA-14). |

### 3. Projection (G1)

| ID | Rule |
| --- | --- |
| PA-9 | **Local graph.** From `causal_relation` build the global event structure (sends, receives, controls and the causal edges). For role `r`, keep the events `r` performs or owns, erase the rest, and keep the induced order: `r`'s event `a` precedes `b` when a causal path from `a` to `b` exists through any events. Every global communication yields exactly one local send at the sender and one local receive at the receiver, and a correspondence table maps each global event to its local event (FR-174-AC-4). `parallel ... join all` stays a local partial order: the engine never enumerates interleavings, and per-channel-key order from the FIFO premise is kept as local receive order (FR-174 "Projection correctness"). |
| PA-10 | **Choice and merge.** At a `choice`, `r` is the owner (local internal choice), or distinguishes the branches through `visible` or an earlier message (local external choice keyed by the distinguishing receive), or must merge. Branches merge for `r` only when their next visible actions have the same direction, peer and message type and their continuations merge recursively under the same rule (FR-174-AC-5). Otherwise the engine returns the counterexample of PA-11. No branch is dropped and no graph is returned in that case (FR-174-AC-2, AC-3). |
| PA-11 | **Counterexample.** The counterexample names the choice, its owner, the role, two global paths from the choice to the first divergent visible action of `r`, and the two incompatible actions. Paths are found by breadth-first search in canonical node order, so the shortest pair is returned and ties break by node order: the counterexample is minimal and unique. |
| PA-12 | **Soundness record.** A projection result is `proved` only with the soundness record: the correspondence table, the merge evidence per choice (`same-next-action` or `distinguished-by` with the visible-set member or earlier message node), and the composition check of PA-13. |
| PA-13 | **Composition check.** The two correctness halves of FR-174 "Projection correctness" are checked by bounded exhaustive exploration of the composition of the role-local graphs, with all roles projected, over the channel premises, run on the one `qsl-eval` protocol transition system: every composed trace is an admitted global trace (inclusion), and every globally required visible action occurs (preservation). Exhausting the request's bounds before the exploration closes yields `inconclusive` carrying the explored depth and state count; closing it within the bounds is a `closed-scope` basis. |

### 4. Identity, determinism and bounds

| ID | Rule |
| --- | --- |
| PA-14 | **Key.** `ProtocolAnalysisKey` is structural: the members of FR-174-AC-6 for a projection and of FR-175-AC-6 for a realizability result, each a typed value, none a digest. Construction is fallible (`new` returns the first absent member as `invalid_runtime_input`/`missing-member` naming it), nothing defaults a member, and the key is stored on the result as a structured value. A checker rebuilds and compares it and refuses an artifact with an absent member the same way. Display name, path, span, producing run and timestamp are not fields of the type. |
| PA-15 | **Bounds.** `Bounds` has `max_depth` (path length), `max_history` (local history length, realizability) and `max_states`; they are key members with no defaults. They are distinct from the run meter: a bound exhausted by a method that otherwise completed is `inconclusive` (FR-360: no counterexample up to a depth is never `proved`); a meter ceiling that stops the method before it completes is `failed` with execution `resource-incomplete`, carries no artifact, and is not a key member. |
| PA-16 | **Determinism.** One key yields one result. All collections are ordered by `CheckedNodeId` or by an explicit canonical order, no `HashMap`/`HashSet` iteration order, randomness, clock or thread schedule reaches the result, and search ties break by canonical order. The analyzer identity is `qsl-analyze::protocol_analysis` with the crate version. For a projection that identity is not a key member (FR-174-AC-6), so the projection algorithm must be a function of the five members only: a change that alters any projection output is a requirement change, not an engine-version change. For realizability the identity and version are a key member (FR-175 "Derived-artifact identity"). |

### 5. Realizability (G2)

| ID | Rule |
| --- | --- |
| PA-17 | **Game.** Positions are (global state, each role's local history). A role's observation is its own sends, receives and choices plus the visible choice outcomes of PA-7 and the declared shared facts of group (c). Two histories of one role are indistinguishable when their observation sequences are equal. The environment moves on the channel outcomes and external events of the premises. A joint strategy wins when every maximal play consistent with the premises satisfies the global safety (no step outside the global protocol), progress (no stuck state while an obligation is pending) and terminal obligations (FR-175 "Strategy game"). |
| PA-18 | **Decidable core.** In V1 each role's candidate strategy is fixed by the protocol: the owner of a choice picks a branch and every other role reacts to what it observes, i.e. the strategy is the PA-9/PA-10 local graph. Then the only adversary is the environment, and the engine decides the claim by exhaustive exploration of all environment plays against those fixed strategies, to the bounds. This is a finite model-checking problem. It does not claim to decide general distributed synthesis (PA-19). |
| PA-19 | **Refutation.** Unrealizable has two witnessed forms. (i) *Indistinguishable histories*: the PA-11 divergence restated as two histories of one role with equal observation sequence, the two incompatible next actions they require, and the violated global obligation (FR-175-AC-2). (ii) *Environment defeat*: a finite play of admitted environment moves, with the roles following their fixed strategies, ending in a violated safety, progress or terminal obligation. |
| PA-20 | **Strategy witness.** For each role, a table from canonical observation-history class to the enabled action, over every information set reachable in the exploration, plus the closure record (plays explored, bound, closed). Because a class is a key, a witness assigns one action per class by construction, which is FR-175-AC-4's property; the checker verifies it independently. A realizable result with an unclosed exploration is `inconclusive`. |

### 6. Settlement, honesty and refusal

| ID | Rule |
| --- | --- |
| PA-21 | **Settlement mapping.** Both kinds settle with the FR-360 labels, each read through the FR-241/242/243 combination FR-360 names. Projection: sound projection with closed composition check is `proved` (`closed-scope`); a PA-11 counterexample is `refuted` (`decisive-counterexample`); bounds or an absent group (c)/(d) premise is `inconclusive` (`unsettled`); an unsupported construct (PA-22) is `unsupported`; a meter stop or engine failure is `failed`. Realizability: realizable with a closed exploration is `proved` (`closed-scope`); unrealizable is `refuted` (`decisive-counterexample`); the rest as for projection. Only `proved` and `refuted` carry an artifact with a key; the others carry the located cause and no substitute graph (FR-174-AC-3). `proved` and `refuted` settle only after the PA-2 checker accepts the artifact. FR-360's Scope does not list these two kinds, so slice 1 raises the Scope amendment with QSpec rather than assuming it. |
| PA-22 | **Unsupported, not guessed.** In V1 the engine answers `unsupported`, naming a located construct cause, for: dynamic role populations; symbolic queue capacity (`Unsupported::SymbolicCapacity`, quire-protocol gap G4); `join any`, `quorum` and `predicate` and an unruled `outstanding` (G3); `repeat` (G5: no engine proves its invariant or variant); compensation and timed awaits; and any protocol where a role has an enabled local choice the protocol does not assign to an owner, since that is distributed synthesis with imperfect information, undecidable in general. The profile vocabulary is the single V1 profile `bounded-static` (static roles, literal capacities, sequence, send, receive, choice, `parallel` with `join all`); a profile outside it is refused at admission, as an unknown FR-054 ordering member is. Widening the profile is a later requirement and engine change, not a flag. |
| PA-23 | **No samples, no second evaluator.** The engine never concludes from replayed or sampled executions and never builds a second transition system (PA-3). |
| PA-24 | **No compatibility layer.** Nothing is kept for the pre-engine behaviour. When the engine lands, the manifest serves both kinds and the unserved-kind tests (TC-1302 family) are rewritten for the served case in the same change. |

### 7. Witness shape

| ID | Rule |
| --- | --- |
| PA-25 | **Shape.** Every node reference is a `CheckedNodeId` of the linked graph, giving quire-protocol its trace-position correspondence. `ProjectionWitness { key, local_graph, correspondence, merge_evidence, composition }`; `ProjectionCounterexample { key, choice, owner, role, path_a, path_b, action_a, action_b }`; `RealizabilityWitness { key, strategies, closure }`; `RealizabilityCounterexample` is `IndistinguishableHistories { key, role, history_a, history_b, action_a, action_b, obligation }` or `EnvironmentDefeat { key, moves, obligation }`. A `LocalGraph` holds typed local events (send, receive, internal choice, external choice, parallel region and join), their peer and message type, and causal edges. All are plain data, serializable through the existing result envelope, and carry no hash. |

## 8. Worked example

Global: `Buyer -> Seller: order`, then `Seller` owns a choice `ship | cancel`
with `visible = { Buyer }`, each branch `Seller -> Buyer: shipped | cancelled`.
Project onto `Warehouse`, which is neither owner nor in `visible` and whose
next action in `ship` is `Seller -> Warehouse: pick` and in `cancel` is
nothing. The next visible actions differ and no earlier message carries the
outcome, so PA-10 cannot merge: projection is `refuted` with the PA-11
counterexample (the choice, owner `Seller`, the two paths, `receive pick`
against end) and realizability is `refuted` with `IndistinguishableHistories`
for `Warehouse`. Add an earlier `Seller -> Warehouse: decision` message and the
branches are distinguished: both become `proved`, with the merge evidence
`distinguished-by` that message.

## Consequences

- G1 and G2 close in QSL; quire-protocol replaces its `unsupported` stub with
  dispatch and carrying, and TL-387 unblocks once slice 9 lands.
- V1 decides a deliberately narrow fragment and says `unsupported` outside it.
  The fragment excludes loops and non-assigned local choice; that cost is
  stated, not hidden.
- The soundness check reuses ADR-027's transition system, so the engine adds
  no queue or interleaving semantics of its own.
- `qsl-replay` grows a checker, so verdicts are verified without the engine,
  at the cost of one more module in the qualified path.
- The owner-assigned-choice rule makes realizability reduce to projection
  plus an environment search; richer strategy games need a new ADR.

## Open items for the requirement slice

1. FR-360 Scope must list local projection and realizability (QSpec change).
2. The V1 profile word and the unsupported-cause slugs are fixed by the QSL
   requirements, under the existing `unsupported_projection` code, with no new
   code family unless the QSpec catalog is amended.
3. The `EvidenceKind` the manifest advertises is an existing variant; slice 9
   chooses it from FR-290 and adds none.

## 12. Implementation slicing

Child tickets, in order. Each ticket is one PR carrying code, tests and the
spec lines the code needs.

| # | Title | Acceptance criteria | After |
| --- | --- | --- | --- |
| 1 | Specify protocol analysis: QSL FRs and TCs for local projection and realizability | Requirements for PA-1 to PA-25, each with an AC and a TC row; FR-174/175-AC-1 to AC-6 traced; profile word and unsupported slugs fixed; FR-360 Scope amendment raised with QSpec; `quire validate` clean (spec-only PR) | ADR |
| 2 | Add `ProtocolAnalysisKey`, `Bounds`, `ProtocolPremises` and outcome types to `qsl-replay` | Key members as FR-174-AC-6 / FR-175-AC-6; absent member refused `invalid_runtime_input`/`missing-member` naming it, by builder and by reader; display name, path, span, run never participate; no digest; channel and choice premises built from `protocol_view` (any missing accessor added under FR-519) | 1 |
| 3 | Project sequence, send, receive and parallel-join-all onto a role | One send/receive pair per global communication with correspondence table (FR-174-AC-4); induced causal order and FIFO-key receive order kept (FR-174 "Projection correctness"); `proved` only with the composition check stubbed to the closed case of a choice-free protocol | 2 |
| 4 | Project choice: ownership, visibility, merge and minimal counterexample | Owner, `visible` and distinguishing-message branches project; compatible-next-action merge accepted (FR-174-AC-5); hidden choice and incompatible merge return the shortest source-correspondent counterexample, unique under canonical order (FR-174-AC-2); worked example of section 8 | 3 |
| 5 | Check projection soundness on the `qsl-eval` transition system and settle unsupported and bounds | Trace inclusion and required-action preservation by bounded exploration on the one `qsl-eval` system (FR-174-AC-1); bounds exhausted is `inconclusive`, meter stop is `failed`/`resource-incomplete`; every PA-22 construct returns `unsupported` with its cause and no substitute graph (FR-174-AC-3) | 4 |
| 6 | Check projection results independently in `qsl-replay` | Checker accepts a witness and a counterexample with no `qsl-analyze` code; mutated correspondence, merge evidence or path is refused; `proved`/`refuted` settle only after acceptance | 5 |
| 7 | Decide realizability: observations, fixed strategies and environment search | Indistinguishability by observation sequence; exhaustive environment plays over FR-054 delivery alternatives and external events, not samples (FR-175-AC-5); strategy witness with one action per class (FR-175-AC-1, AC-4); both refutation forms (FR-175-AC-2); missing group (c)/(d) premise or exhausted bound is `inconclusive` (FR-175-AC-3); non-assigned local choice is `unsupported` | 5 |
| 8 | Check realizability results in `qsl-replay` and gate determinism | Independent checker for witnesses and both counterexample forms; two runs of one analyzer identity and version on one key return an identical result, also across thread counts; key members change the identity as FR-175-AC-6 states | 6, 7 |
| 9 | Serve both kinds: manifest, settlement and end-to-end driver path | Provider manifest advertises `local-projection` and `realizability`; the unserved-kind tests are rewritten for the served case, `NegotiationCause` unchanged; FR-360 labels settle through the PA-21 map; a CLI/driver run settles one item of each kind and each of the five labels; handoff note to quire-protocol (TL-387) naming the result envelope and the carried key | 8 |

Slices 3 to 5 and 7 may merge into fewer PRs if the diffs stay small. Slices 6
and 7 can proceed in parallel after 5.

## Alternatives Considered

- **A new crate `qsl-protocol-analysis`.** Rejected. ADR-011 section 7.2 lists
  the approved crate extractions; layer A already holds every engine and
  already has every edge this one needs.
- **Do the work in quire-protocol.** Rejected. quire-protocol ADR-010 consumes
  QSL's analysis and does no QSL-side work; two projection implementations
  would also drift.
- **General multi-role strategy synthesis in V1.** Rejected. It is undecidable
  with imperfect information in general, and no requirement asks for it;
  PA-22 answers `unsupported` and keeps the claim honest.
- **Treat an exhausted bound as `unsupported`.** Rejected. FR-360 gives an
  analysis that completes to a bound without a counterexample the label
  `inconclusive`; `unsupported` is for a construct no candidate decides.
- **A hash of the artifact as its identity.** Rejected by FR-174-AC-6 and
  FR-175-AC-6: the key is structural.
- **Sample executions to approximate the environment.** Rejected by FR-175
  "Behavior" and FR-175-AC-5.

## References

- QSpec FR-053, FR-054 (AC-2, AC-4), FR-170 (AC-4, AC-7), FR-172, FR-174
  (AC-1 to AC-6), FR-175 (AC-1 to AC-6), FR-241 to FR-243, FR-288, FR-360,
  FR-479.
- QSL FR-057-AC-15, FR-519 (AC-20), ADR-011 sections 6.1, 7.2 and FB-05,
  ADR-018 EN-1 and LA-3, ADR-027, ADR-028 SP-2.
- quire-protocol ADR-010 "QSL gaps" G1 and G2; its FR-030.
- QSL-1868, QSL-1889.

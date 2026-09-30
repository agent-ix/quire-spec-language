---
id: SR-787
title: "Integrity review of ADR-016 state, model and finite execution mapping"
type: SpecReview
analysis: integrity
scope: "spec/decisions/ADR-016-state-model-finite-execution-mapping.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md, spec/functional/FR-089-carry-population-identity-across-the-kernel-boundary.md, spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md, spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: reviews
---
# SR-787: Integrity review of ADR-016 state, model and finite execution mapping

## Summary

Reviewed ADR-016 on `spec/19-arch40-mapping`. The review checked
four things:

- completeness against Linear QSL-19 (#220), read as data;
- consistency with ADR-011 to ADR-015, FR-089, FR-101 and FR-120;
- the internal consistency of the local ids;
- every code citation, each opened at its file and line.

What holds:

- Every item in the ticket's required mapping has a section. These are §1
  (static versus runtime), §2 (identity and provenance), §3 (population,
  keys, seeds, frontiers, positions, cancellation, exhaustion, incomplete),
  §4 (frames and the clause path), §5 (nondeterminism), §6 (exhaustion is not
  proof), §9 (responsibilities), §10 (owners and oracles), §11 (pins and
  current head) and §12 (evidence).
- Most code citations are exact:
  - `admit` `intake.rs:564`, `admit_unit` `unit.rs:98`, `normalize`
    `normalize.rs:2780`, `key.rs:432-434` and `link_dispatch`
    `dispatch.rs:229`;
  - `ModelCorrespondence` `identity.rs:257`, `CheckedGraph` field
    `mod.rs:334`, `resolve_declaration` `mod.rs:1656`, and
    `TypeEnvironment::attribute`/`conforms` `declaration.rs:884`/`:966`;
  - the check arms `check.rs:1659`/`:2214`, and `population.rs` `:551`,
    `:714`, `:749`, `:806`, `:1259`, `:1718` and `:1888`;
  - `ObjectEnvironment` `:88`/`:156`, the evaluator arms and guards
    `evaluate.rs:1377`/`:1378`/`:1396`/`:1401`, and `model_query.rs:183`/`:211`;
  - the state-clause and frame sites in `qsl-forms`, `check`, `qsl-eval` and
    `qsl-replay`;
  - the simulation sites `explore.rs:46`/`:75`/`:111`/`:269`,
    `sample.rs:270`, `trace.rs:35`/`:117`, `frontier.rs:10` and
    `key.rs:63-71`;
  - the four fixtures `finite_simulation.rs:106`/`:148`/`:189`/`:216`, and
    `domain_package.rs:393`, `intake.rs:2089` and `causes.rs:140-170`;
  - every cited test line: `inherited_attributes.rs:152`,
    `equality_matrix.rs:1028`, `config_version_spine.rs:181`/`:324`,
    `finite_simulation.rs:728`/`:773`/`:826`/`:880`/`:975`/`:1113` and
    `evaluate.rs:2169`.
- The ADR-013 O-05, O-16 and O-25, ADR-014 §1, §7 and §8, and ADR-012 §15
  facts the record relies on are as stated, with the exceptions below.
- The FCD pins are exact revisions, used only from `model::intake` (PI-2).
  The diagnostics catalog `1-draft.8` and sampler `1-draft.1` are in code
  (PI-3).

What does not hold:

- One finding is high. The ID-5 decision rests on a false claim about where
  a binding's declared maximum comes from. Under the proposed preimage, two
  unequal bindings would still share one `PopulationId`, so the guard that
  G-3 turns into an `InternalFault` would stay reachable.
- The rest are an unamended ADR-013 paragraph, a term used in two senses,
  over-claimed evidence, lane-deletion gaps against ADR-011 §7.3 and the
  ticket, and several wrong or loose code claims.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The ID-5 decision says the declared maximum "is read from the declaration that `(domain_package_selection, population_key)` already names, so it cannot differ between two admissions that agree on those". The code does not do this. `admit_binding` takes `declared_maximum: Option<u64>` as a caller argument (`qsl-semantics/src/model/population.rs:811`) and stores it without checking it against the population record (`:1143`). `qsl-bench/src/model.rs:309-315` passes the document's own length. `PopulationBinding` derives `PartialEq` over every field (`population.rs:550`), including `declared_maximum`, `ancestor_steps` (taken from the caller's meter limits, `:849`) and `pre_anchor` (a `Post` binding's pre binding). So two admissions that agree on package, key, role and `members` can still build unequal bindings and get one id. `with_population`'s refusal stays reachable by input, and G-3's change to an `InternalFault` would report caller input as a broken invariant. Fix: in G-3, either (a) make admission read the maximum from the `PopulationRecord` and drop the parameter, remove `ancestor_steps` from binding equality or from the binding (it is B-2 configuration), and add the pre binding's id to a `Post` preimage; or (b) add `declared_maximum` and the pre-anchor id to the preimage. Keep the typed `PopulationConflict` refusal until one of these lands. Amend FR-089's new Status paragraph to match. | ADR-016 §2 ID-5, §9 G-3; FR-089 Status; `population.rs:550-575`, `:806-822`, `:1140-1150` |
| FND-002 | medium | The ADR-013 amendment changed only the §6 Outcomes row. O-16's invariants paragraph (ADR-013 line 417) still says "state `EvaluationOutcome` and simulation `Outcome` are lane-private (§6)", which now contradicts the amended row and ADR-016 FP-4. Fix: amend that sentence the same way ("the lane-D simulation `Outcome` before convergence; `qsl_eval::simulation::Outcome` is the converged S6a-layer outcome mapped by `Outcome::category()`"), and list it under "Amendments made with this record". | ADR-013 O-16 (line 417), §6; ADR-016 FP-4, Amendments |
| FND-003 | medium | "Exhaustion" has two meanings in one record. In EX-7 and OR-5 it means an exploration limit was reached (`Bounded{limit}`), as in QSpec FR-181 ("Exhaustion reports the unexplored frontier and cannot claim exhaustive success"). In §6's heading, the ticket item it answers and the Alternatives, "finite exhaustion" means the frontier went empty (`Outcome::Exhaustive`). A reader cannot tell which is meant in the ticket's acceptance list ("cancellation, exhaustion, unsupported"). Fix: rename §6 to "Exhaustive exploration is not proof". Use "bound exhaustion" in EX-7, EX-8 and OR-5, and add one sentence defining both terms. | ADR-016 §3 EX-7, §6, §10 OR-5; QSpec FR-181 Behavior |
| FND-004 | medium | §12 says the FR-101 tests (TC-453 to TC-455, over test systems) exercise "EX-2 to EX-8". EX-2 is FR-120's typed canonical state form, which does not exist (G-4). EX-7's `InvariantUndetermined` and `ExpansionStop` do not exist, and neither does EX-8's `Stopped` (G-1, `explore.rs:46`, `:75-100`). The ticket says the evidence "validates only the shared mechanism" and that unavailable scenarios are not asserted as completed. Fix: narrow the claim to what runs: EX-3, EX-4, EX-5(a), EX-6, FR-101's own key encoder under EX-2, and the `Bounded`/`Cancelled` halves of EX-7 and EX-8. | ADR-016 §12, §3; FR-101-AC-12 to AC-14; FR-120 |
| FND-005 | medium | EX-1 says a universe is "in kind an ADR-014 B-4 proof bound … carried as FR-120's `PopulationUniverse`". ADR-014 §1's B-4 row names one carrier, F `bound::ProofBound`, built by QSL-140, and says a B-4 value "is used only to build a bounded request". So ADR-016 adds a second B-4 carrier and a second use, but its Status says it "reopens none of their cells". Fix: either amend ADR-014 §1's B-4 row to name `PopulationUniverse` as the simulation-scoped carrier (and list it under Amendments), or classify a universe as a simulation input outside the B taxonomy and drop the B-4 claim. | ADR-016 §3 EX-1, Alternatives, Status; ADR-014 §1 B-4 row, "Relations between kinds" |
| FND-006 | medium | G-6 does not cover the ticket's M-6c deletion text or ADR-011 §7.3. The ticket and ADR-011's M-6c row (line 1072) delete "`state`, and the SEAM-3 reads, `native_model` and IR imports that feed it". G-6 names neither the SEAM-3 `protocol_artifact` reads that feed `state` (ADR-011 SEAM-3 row, line 842) nor the IR import (`src/state/evaluation.rs:15`, `use quire_contract_ir as ir`; ADR-011 line 886). ADR-011's M-6c row also says "The PR that lands spine clause execution deletes native `run`". That PR has landed (`run_clause`, `qsl-replay/src/spine/clause.rs:552`), and native `run` is still present. Fix: add the SEAM-3 reads and the IR import to G-6's deletion list. State that ADR-012 §15.8's QSL-5 coupling replaces ADR-011's M-6c trigger and moves `native_model` out of M-6c, and amend ADR-011's M-6c row to say so, listed under Amendments. | ADR-016 §9 G-6; ADR-011 §6.2 (lines 842, 886), §7.3 M-6c (line 1072); ADR-012 §15.8 |
| FND-007 | medium | G-7 gives the `native_model` deletion to "QSL-68 (#120) and QSL-67 (#121), each in the PR that lands its S3 checker", and names two importers, `checking/composed/solver.rs` and `protocol_artifact/mod.rs`. The SEAM-3 importers go with #218 (M-6d, ADR-011 line 842 and ADR-012 §15.8 step 3), not QSL-67 or QSL-68. There are also more importers than two. SEAM-2 has `src/checking/composed/solver.rs`, `solver/models.rs`, `checking/composed/sources.rs` and `linking/composed/models.rs`. SEAM-3 has ten files under `src/protocol_artifact/`. Fix: give the `native_model` deletion to "whichever of #218 (M-6d) and the `StateModel` M-6e PR lands last". Say "its last SEAM-2 or SEAM-3 importer" instead of listing files, or list them all. | ADR-016 §9 G-7; ADR-012 §15.8 step 3; ADR-011 §6.2 SEAM-3 row |
| FND-008 | medium | The ticket's acceptance says "Every unavailable scenario is represented by a bounded acceptance criterion in #120/#121/#164". §9 notes that the QSL-67 and QSL-68 ticket bodies still say "extend `runtime::execute`" and name the native lane, but it gives no replacement text. Of the seven G rows, G-2 and G-3 have no AC id (SR-786 FND-001, FND-002). Fix: add, per ticket, the exact acceptance lines to write into QSL-67 and QSL-68 (one per G row, citing the AC ids), and make the ticket edit part of landing this record. | ADR-016 §9; ticket QSL-19 acceptance |
| FND-009 | low | Context says `EffectiveId` "is minted only by `EffectiveDeclarationPreimage::identity_and_canonical_len` (`model/key.rs:432-434`)". `EffectiveView::identity` (`qsl-semantics/src/model/normalize.rs:703-704`) also calls `EffectiveId::from_digest`, for the effective-view identity. Both are in `model`, so O-05's rule holds, but the ADR's claim is wrong, and §8's "one minter each" repeats it. Fix: say "minted only in `model`: declaration identities at `key.rs:432-434`, the effective-view identity at `normalize.rs:703-704`". | ADR-016 Context, §8; ADR-013 O-05 |
| FND-010 | low | ID-5's reason says "`admit_binding` is called only from tests". `qsl-bench/src/model.rs:309` (`admit_population`) calls it as well. The no-migration conclusion still holds, because the bench admits one binding per evaluation. Fix: "`admit_binding` and `admit_invocation` are called only from tests and `qsl-bench`". | ADR-016 §2 ID-5 |
| FND-011 | low | FE-1 says FR-115 `evaluate_frame` calls `decide_frame`. It does not. `evaluate_frame` (`qsl-eval/src/value/expression/mod.rs:543-563`) takes an already `AdmittedInvocation` and evaluates the frame node through the `ProtocolClause` arm. The frame decision runs earlier: `admit_frame_invocation` (`observation.rs:1245`) calls `frame::enforce` (`observation.rs:1042`), and `enforce` calls `decide_frame` (`observation/frame.rs:252`). `decide_frame` is `pub(crate)` (`population.rs:1440`), so `qsl-eval` cannot call it at all. Fix: replace "FR-115 `evaluate_frame`" with "FR-115's invocation admission through `admit_frame_invocation`". | ADR-016 §4 FE-1 |
| FND-012 | low | Two line numbers are off. `FamilyKind::StateModel` is at `qsl-semantics/src/family/mod.rs:93`; line 90 is the `enum FamilyKind` line. `S6aFamilyKind`'s `Value` and `ProtocolClause` variants are at `qsl-eval/src/value/expression/s6a/mod.rs:102-108`, while `98-104` covers the macro's closing lines. Fix: cite `family/mod.rs:93` and `s6a/mod.rs:102-108`. | ADR-016 Context "Not implemented" |
| FND-013 | low | SC-6 and the agreement rule call `ModelIndex::conforms` a "metered" walk that stops "on its B-2 meter". It is a read-only ceiling: it takes a `max_steps` argument and charges nothing (`qsl-semantics/src/model/index.rs:270-277`). ADR-014 §7 classifies reaching it as a "read-only B-2 ceiling", `Refused` with the `ancestor-steps` cause, category refusal. Fix: say "bounded by the read-only B-2 `ancestor_steps` ceiling; reaching it refuses `resource_exhausted`/`ancestor-steps` (ADR-014 §7)". | ADR-016 §1 SC-6, Agreement rule; ADR-014 §7 |
| FND-014 | low | Context and SC-4 write `ValueType::Population(N)` and `Population(max)`. The kernel type is `Population(Option<u64>)`, and `None` types `allInstances` as an unbounded `Set<T>` (`check/check.rs:2175-2178`). SC-4 does not state the `None` case, which is the one that drives the requires-bound path (EX-1, §6). Fix: state both cases in SC-4. | ADR-016 Context, §1 SC-4; `quire-exact/src/value.rs`; ADR-014 §1 B-1 |
| FND-015 | low | `StateModel` names two things. It is the family (`FamilyKind::StateModel`; FP-1 to FP-3, G-2), and it is FR-120's struct `qsl_semantics::model::state::StateModel` (FE-1 `StateModel::check_frame`, ID-10 `StateModel::observation`, G-4). FP-3's "No S6a input is a `StateModel` item" can be read either way. Fix: call the struct "FR-120's `model::state::StateModel`" wherever the struct is meant. | ADR-016 §2 ID-10, §4 FE-1, §7, §9 G-4; FR-120 "Frame and observation seam" |
| FND-016 | low | §6's first consumer says "no clause outcome derived from it is promoted beyond `tested` (ADR-013 O-16 success row)". In O-16, `tested` is a backend's QSpec FR-331 result. No rule turns an exploration finding into a clause outcome or an FR-331 result, so this bullet cites a map that does not apply. Fix: say that exploration results never enter QSpec FR-331 results or clause outcomes. If a map is intended, name it and its owner. | ADR-016 §6; ADR-013 O-16 success row |
| FND-017 | low | OR-1 to OR-5 each name an owning ticket. OR-6 (unsupported), OR-7 (incomplete) and OR-8 (requires-bound) name only components. The ticket's acceptance says each named behavior has "an owner and oracle". Fix: add the ticket to each row, for example QSL-67 for OR-7's exploration and FR-120 parts, QSL-68 for admission, "delivered under FR-101" for OR-8, and CG negotiation plus QSL-67 for OR-6. | ADR-016 §10 OR-6 to OR-8 |
| FND-018 | low | ND-1 says "nothing prunes one except a false effective pre- or postcondition". FR-120 "Contract clauses" also leaves an application disabled when the conjunction is undecided (`ContractUndetermined`), and an `Incomplete` clause stops the whole expansion. Fix: add both cases to ND-1, citing FR-120 "Contract clauses". | ADR-016 §5 ND-1; FR-120 "Contract clauses", "Post-states" |
| FND-019 | low | PI-1 says "QSL pins `quire-contract-ir` in `Cargo.lock`". In `Cargo.lock`, that is the package `quire-contract-model` (workspace alias `quire-contract-ir`, `Cargo.toml:72`). A separate package named `quire-contract-ir` is locked (`quire-contract-ir-historical`, `Cargo.toml:105`). Fix: name the pinned package `quire-contract-model` (alias `quire-contract-ir`), as Context already does. | ADR-016 §11 PI-1, Context |
| FND-020 | low | The Alternatives say "QSpec FR-181 and ADR-014 §8 make finite results evidence for what was evaluated only". QSpec FR-181 says exhaustion "cannot claim exhaustive success" and that enumeration is deterministic. It has no evidence-scope rule. Fix: cite ADR-014 §8 alone, or quote the FR-181 sentence the claim relies on. | ADR-016 Alternatives; QSpec FR-181 Behavior; ADR-014 §8 |

## Disposition

Every finding above is fixed on `spec/19-arch40-mapping` in the ADR-016 rewrite and its listed amendments (ADR-011 M-6c and §8; ADR-012 §2, §3, §5.1, §13.5; ADR-013 O-13, T-6, QC-21, O-16, §6; FR-089; FR-120; `spec/tests.md`), except as noted below.

- FND-008: the per-ticket acceptance text is ADR-016 §9's G rows. Copying them into QSL-67 and QSL-68 is the team lead's action on acceptance.

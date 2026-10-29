---
id: SR-791
title: "Risk and complexity review of ADR-016 state, model and finite execution mapping"
type: SpecReview
analysis: risk-complexity
scope: "spec/decisions/ADR-016-state-model-finite-execution-mapping.md (new); its amendments to ADR-012 Status, ADR-013 §6 Outcomes row, FR-089 Status and spec/spec.md, at 47b1b806 on spec/19-arch40-mapping"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: reviews
---
# SR-791: Risk and complexity review of ADR-016

## Summary

Reviewed commit `47b1b806` on `spec/19-arch40-mapping`, with the code at
that head. ADR-016 decides no new mechanism. Its risk is in the work it hands
to QSL-68 and QSL-67 (G-1 to G-7), and in the one identity it changes (ID-5).

The static/runtime split (§1), the trace-position split (EX-5), the outcome
mapping (EX-8) and "finite exhaustion is not proof" (§6) are low risk. They
restate rules that implemented types already enforce. FP-3 is also low risk:
`FamilyContract` has no `evaluate` hook (`qsl-semantics/src/family/contract.rs:336-380`),
so leaving `StateModel` out of `S6aFamilyKind` leaves no dead contract part.
Adding the variant later is forced by the compiler.

Five items carry most of the risk:

1. **G-3 changes an identity.** The decision is incomplete, so G-3 can ship a
   collision.
2. **G-4 has a combinatorial candidate space.** FR-120 does not bound its
   construction work, only the pairs it evaluates.
3. **G-2 is a large refactor with no behavior change**, and its nested-family
   `check` shape has no precedent.
4. **G-6 and G-7 delete modules that the headline evidence (TC-469) still
   builds on.**
5. **PI-1 depends on an external IR fix (IR-370)** in a repository QSL pins
   at two revisions.

## Risk register

| Item | Tech risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| §1 SC-1 to SC-8, Agreement rule | Low | Low | Implemented; TC-219/TC-220 already assert agreement | Cite the existing oracle (SR-790 FND-003) |
| §2 ID-1 to ID-4, ID-6 to ID-11 | Low | Low | One minter each, implemented | None needed |
| §2 ID-5 / G-3 PopulationId preimage | High | Low | Changes a minted identity and its golden vector; the preimage decision misses the declared maximum and `pre_anchor` | FND-001 |
| §3 EX-1 to EX-8 | Low | Low | FR-101 types exist; tests run | None needed |
| §3 EX-9 evidence scope | Low | Medium | No carrier type; the rule is prose only | FND-009 |
| §4 FE-1 to FE-3 | Low | Low | One `decide_frame`, one `evaluate_clause`, already shared | None needed |
| §4 FE-4 population reads in clauses | Low | Medium | Reopens when QSpec admits a population-valued clause expression | The reopen condition is stated; nothing more needed |
| §5 ND-1 to ND-4 | Low | Low | The engine fixes the order | SR-790 FND-012 (closure pruning) |
| §6 exhaustion is not proof | Low | Low | Enforced by O-16 map, requires-bound, replay route | None needed |
| §7 FP-1, FP-2 / G-2 StateModel check hook | High | Medium | ~5,000 lines across `check.rs` (2,503) and `checked_dispatch.rs` (2,235), plus `lowering/model.rs` (441); no behavior change; `FamilyContract::check` is item-level | FND-003 |
| §7 FP-3 no S6a `StateModel` kind | Low | Low | Closed enum, no `evaluate` hook to leave dead | FND-006 (cause attribution test) |
| §7 FP-4 / G-4 `ModelSystem` | High | Medium | Candidate enumeration is a product over field domains and object subsets; 12 ACs; 539-line FR | FND-002, FND-005 |
| G-1 FR-101 findings and `ExpansionStop` | Medium | Low | Changes the `TransitionSystem::successors` signature (`explore.rs:46`), breaking four fixtures and explore, sample and replay | FND-005 |
| G-5 FR-085 resolver | Low | Low | Small, isolated `model` function | None needed |
| G-6 M-6c state lane deletion | Medium | Low | Deletes `runtime` and `model_source`, which TC-469's fixture generator imports | FND-004 |
| G-7 `native_model` deletion | Medium | Low | About 80 importing files (`grep -rl native_model`); importer list incomplete | FND-004 |
| §11 PI-1 IR-370 pin | Low | High | External repo; two pinned revisions of `quire-contract-ir` in one lock | FND-007 |
| §11 PI-4 heads run before bump | Low | Low | A process gate that no running code reads | FND-008 |
| §3 EX-6, EX-7 cancellation and exhaustion | Low | Low | Implemented and tested (TC-455) | None needed |

## Top hazards

1. G-3: an incomplete identity decision (FND-001).
2. G-4: an unbounded candidate construction cost (FND-002).
3. G-6 and G-7: deletion breaks the TC-469 corpus (FND-004).
4. G-2: a large refactor on an undecided seam, off the V1 path (FND-003).
5. PI-1: an external pin with two revisions of one repository (FND-007).

## Failure-domain gaps

No `spec-failure-domain-analysis` deliverable exists yet for ADR-016 in
`spec/reviews/state-model-mapping/`. Two failure-domain gaps found here feed
it: FND-001 (identity collision) and FND-002 (uncharged work in candidate
construction).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | G-3 changes the `PopulationId` preimage, but ID-5 leaves the collision cases open. The declared maximum is a caller argument to `admit_binding` (`qsl-semantics/src/model/population.rs:806-812`), not a fact of the declaration. `PopulationBinding` equality also compares `pre_anchor` and `ancestor_steps` (`population.rs:551-599`). Equal members can therefore still map to unequal bindings, and `with_population`'s refusal is not an invariant break (see SR-790 FND-001). The change also breaks the golden vector `population_id_matches_its_golden_vector` (`population.rs:2160`, domain tag `quire.population/v1`), and ID-5 does not say whether the tag changes. ADR-013 QC-21 describes the preimage and is not amended. Mitigation: state the full preimage in ID-5, including the declared maximum and the `Post` binding's pre identity, or remove the maximum parameter. State that the tag stays `quire.population/v1` and the vector is regenerated in the G-3 PR (`PopulationId` crosses no wire, per ID-5 "Absent"). Add a one-line note to QC-21. | ADR-016 §2 ID-5, §9 G-3; ADR-013 QC-21; `population.rs:551-599`, `:806`, `:2160` |
| FND-002 | high | G-4's candidate space grows combinatorially. FR-120 Behavior builds candidates from every value of every `modifies` field, every subset of deletable objects, and every subset of unused universe keys with every field value for each created object. `max_candidates` counts only the (candidate, result) pairs that are evaluated (FR-120 "Each expansion counts…"). Candidates dropped for not being closed ("Every reference of a candidate names an object of the candidate") are generated but never counted. An eager or filter-after-generate implementation can therefore do unbounded, uncharged work before the cap applies. FR-120-AC-6's cap test uses `max_candidates` 2 over three candidates, which cannot tell a lazy implementation from an eager one. Mitigation: in FR-120, require candidates to be generated lazily in canonical order, with each generated candidate charged, including rejected ones. Add an AC over a product larger than 10^9 (for example three `Int[0, 1000]` fields under `modifies`) with a small cap that must stop `resource_exhausted` without materializing the set. Slice G-4 into modifies-only, deletes and creates. | ADR-016 §9 G-4, §3 EX-7; FR-120 Behavior, FR-120-AC-3, FR-120-AC-6 |
| FND-003 | medium | G-2 is large and its seam shape is undecided. The model forms' S3 work sits in `check/check.rs` (2,503 lines; arms at `:1110`, `:1839-1845`, `attribute` `:1659`, `all_instances` `:2164`, `lookup` `:2214`), and FP-1 puts the dispatch table in scope too (`checked_dispatch.rs`, 2,235 lines). `FamilyContract::check` takes an item `Form` and a `CheckContext<Declarations>` (`family/contract.rs:336-380`). Model forms are nested subexpressions of `Value` and `ProtocolClause` bodies, and no family today calls another family's `check` for a nested form. G-2 changes no behavior and is on no FR-120 or FR-085 path. QSL-19 also requires that no interface decision be left to implementation. Mitigation: either decide in FP-2 how a nested `StateModel` check is called (its `Form`, its `Declarations`, and what `requirements` returns for a nested form), or record G-2 as sequenced after G-4 and G-6 and not blocking QSL-68's behavior work. | ADR-016 §7 FP-1, FP-2, §9 G-2; ADR-012 §4.3 |
| FND-004 | medium | The G-6 and G-7 deletions remove modules that the §12 headline evidence builds on. `examples/config-version/fixtures.rs` imports `runtime`, `model_source` and `native_model` (`fixtures.rs:6-15`), and TC-469's `build()` goes through it (`tests/it/config_version_spine.rs:170-175`). `native_model` has about 80 importers in `src/`, `tests/` and `examples/`, and G-7 names two. Without a prior split, the G-6 PR either leaves the build red or deletes the spine corpus with the native one. Mitigation: make "split `spine.rs` generation from the native fixtures" a named step before G-6, in QSL-67, with TC-469 steps 1 and 4 to 6 as its test (see SR-790 FND-002). | ADR-016 §9 G-6, G-7, §12; ADR-012 §15.8 |
| FND-005 | medium | The G-1 to G-4 ordering is left implicit. G-4 needs `Expansion`, `ExpansionStop` and `Outcome::Stopped` (FR-120 "Outputs", FR-120-AC-6 "`Outcome::Stopped`"), which G-1 adds. G-1 changes `TransitionSystem::successors` from `Vec<(TransitionId, State)>` (`qsl-eval/src/simulation/explore.rs:46`) to a `Result`, which breaks the four fixture systems in `qsl-eval/tests/it/finite_simulation.rs` (`:106`, `:148`, `:189`, `:216`). Both belong to QSL-67. Mitigation: state in §9 that G-1 lands first as its own PR, with TC-453 to TC-455 green on the new signature, and that G-4 depends on it. | ADR-016 §9 G-1, G-4; FR-101-AC-12 to AC-14 |
| FND-006 | low | FP-3 is sound and reversible. Its one exposure is cause attribution. `StateModel`-owned causes (`precondition-false`, `absent-key`) are raised inside the `Value` and `ProtocolClause` `evaluate` hooks, and ADR-016 names no test that pins those causes to the `state-model` family prefix. Mitigation: add a test to G-2 or G-4 that a `lookup … absent undefined` under `Value` evaluation reports the `StateModel` family's catalog prefix. | ADR-016 §7 FP-3, §1 SC-6, SC-7; ADR-013 O-16 |
| FND-007 | low | IR-370 is an external contract. Volatility is high and QSL-side risk is low. `Cargo.lock` carries `quire-contract-ir` at `04eb6f84` and `quire-contract-model`, from the same repository, at `2a286437` (`Cargo.lock:1546-1561`). The current-head lane pins `quire-contract-model` at `53cc03c` and patches it to a local head (`integration/current-head/Cargo.toml:34`, `:56-58`). A bump for IR-370 can bring unrelated IR changes and leave the two revisions skewed. Mitigation: bump in a dedicated PR that moves only `quire-contract-model` and un-ignores TC-463 step 1 and TC-469 step 6. Keep G-1 to G-7 independent of it, as §11 says. | ADR-016 §11 PI-1, Open dependencies 1; `Cargo.lock:1546-1561` |
| FND-008 | low | PI-4 makes "a green heads run precedes each pin bump" a precondition. Nothing that runs reads it, and the lane exists to show drift, not to gate (the ADR's own words: "It never substitutes for a pin"). As a gate it can stall a pin bump that G-1 to G-7 need over a head change unrelated to the bump. Mitigation: restate PI-4 as informational. The pin bump PR's own gate is the root `make ci` with the un-ignored tests. | ADR-016 §11 PI-4 |
| FND-009 | low | The name `StateModel` has three meanings in ADR-016: `FamilyKind::StateModel` (FP-1 to FP-3), G-2's future `FamilyContract` implementation, and FR-120's `pub struct StateModel` in `qsl_semantics::model::state` (FR-120 line 223), whose `check_frame` and `observation` ID-10 and FE-1 cite. FP-3's claim that no S6a input is a `StateModel` item reads differently for each. EX-9 has a related gap: it is a rule about comparing results, with no type that carries the request identity, so nothing stops a caller comparing two `Exhaustive` results across requests. Mitigation: qualify every `StateModel` in ADR-016 (family kind, family contract, or FR-120 seam struct). Rename FR-120's struct before code exists, for example to `ModelStateSeam`. That is a spec-only edit. Record EX-9 as analysis-only until G-4 adds a request identity to the exploration result. | ADR-016 §2 ID-10, §3 EX-9, §4 FE-1, §7; FR-120 "Frame and observation seam" |

## Disposition

Every finding above is fixed on `spec/19-arch40-mapping` in the ADR-016 rewrite and its listed amendments (ADR-011 M-6c and §8; ADR-012 §2, §3, §5.1, §13.5; ADR-013 O-13, T-6, QC-21, O-16, §6; FR-089; FR-120; `spec/tests.md`), except as noted below.

- FND-009: FR-120's `StateModel` struct is not renamed. ADR-016 disambiguates every use instead (`StateModel` for the family, `model::state::StateModel` for the seam type).

---
id: SR-790
title: "Evidence analysis of ADR-016 state, model and finite execution mapping"
type: SpecReview
analysis: evidence
scope: "spec/decisions/ADR-016-state-model-finite-execution-mapping.md (new); its amendments to ADR-012 Status, ADR-013 §6 Outcomes row, FR-089 Status and spec/spec.md, at 47b1b806 on spec/19-arch40-mapping"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: reviews
---
# SR-790: Evidence analysis of ADR-016

## Summary

Reviewed commit `47b1b806` on `spec/19-arch40-mapping`. ADR-016 has no AC
rows of its own. Its evidence claims are the §10 oracles (OR-1 to OR-8), the
§12 list of implemented behavior, and the Test column of the §9 gap table
(G-1 to G-7). Linear QSL-19 (#220) was read as data. Its acceptance asks for
an owner and an oracle for ambiguous dispatch, static conformance, closure,
cancellation, exhaustion, unsupported and incomplete, and for every
unavailable scenario to be a bounded AC on #120, #121 or #164.

Every cited file and line was opened. The code citations in Context are
accurate. The one drift is `FamilyKind::StateModel`, at
`qsl-semantics/src/family/mod.rs:94`, not `:90`. Every named test exists at
the cited line with the cited name:

- `inherited_attributes.rs:152`
- `equality_matrix.rs:1028` (TC-198 L08)
- TC-222 to TC-225 in `model_dispatch.rs`
- TC-455 at `finite_simulation.rs:728`, `:773`, `:826`, `:880` and `:975`
- TC-439 at `:1113`
- TC-467 at `evaluate.rs:2169`
- TC-469 step 1 at `config_version_spine.rs:181`
- the parity test at `:324`
- TC-514 in `qsl-replay/src/spine/clause/tests/frame.rs`
- TC-515 in `.../tests/frame_replay.rs`

OR-4, OR-5 and OR-8 are real, discriminating oracles that run today. The
rest have problems:

- **ID-5 contradicts the code.** Its claim that the declared maximum "needs
  no member" is false. `admit_binding` takes the maximum as a caller
  argument, and an existing test already admits the same members under two
  different maxima.
- **G-6 cannot pass its own test.** TC-469's fixture generator imports the
  native modules that G-6 and G-7 delete.
- **OR-2 misses the oracle it already has.** TC-219 and TC-220 are
  check-versus-evaluation agreement tests that exist today. The all-pairs
  test it plans instead is not in any G row.
- **OR-1 has no S3 oracle.** Dispatch linking reaches S3 only through a
  bridge that only tests call.
- **OR-7 has no test.** It cites a spec table and two planned ACs.
- **§9 misses planned #120 work.** FR-084-AC-2 (TC-227), FR-084-AC-7
  (TC-410) and FR-086 (TC-233, TC-234, TC-236, TC-241) are not in the table.
- **§12 overclaims.** TC-469 asserts dispositions only, not the identity
  rows it is said to exercise.

## Evidence map

| Item | Method | Artifact | Status |
| --- | --- | --- | --- |
| OR-1 ambiguous dispatch | test | `qsl-semantics/tests/it/model_dispatch.rs` `d02_…` (:307), `d03_…` (:399), `d04_…` (:451), `a_family_at_the_configured_bound_…` (:701); TC-222 to TC-225 | Runs. Model layer only; no S3 unit-level test (FND-004) |
| OR-2 static conformance | test | `model_conformance.rs`, `type_environment_model.rs`; TC-198 L08 `e16a_references_admit_a_conforming_upcast_in_either_order` (`equality_matrix.rs:1028`) | Runs |
| OR-2 runtime agreement | test | Today: TC-219/TC-220 `divergence_*_at_check_and_at_evaluation` (`type_environment_model.rs:206-400`). Planned: "a new agreement test in G-2", with no TC and no G-2 AC | Partly runs; planned half has no home (FND-003) |
| OR-3 closure | test | FR-084 TC-226, TC-228, TC-229 (`model_population.rs` l02, l03, l05); FR-106 `tc465_row23_…` (:3349), `tc465_row24_…` (:3366) in `state_clauses.rs`; TC-469 `Dangling`, `Incomplete` cases | Runs. FR-084-AC-2 (TC-227) is planned and missing from §9 (FND-005) |
| OR-4 cancellation | test | TC-455 `cancellation_stops_the_run_and_returns_the_frontier` (`finite_simulation.rs:728`) | Runs, discriminating |
| OR-5 exhaustion | test | TC-455 `:773`, `:826`, `:880` (N stops, N+1 exhaustive); TC-439 `:1113` | Runs, discriminating |
| OR-6 unsupported | test | TC-467 `evaluate.rs:2169`, `:2228`; TC-155 `only_the_supported_item_is_routed` (`qsl-route/tests/it/routing.rs:225`) | Runs. FE-4's unreachability is analysis only (FND-009) |
| OR-7 incomplete | test | Cited: FR-100 table (not a test), FR-101-AC-12 (planned), FR-120-AC-6 (planned) | No test cited; tests exist (FND-007) |
| OR-8 requires-bound | test | TC-455 `requires_bound_refuses_before_any_transition_system_call` (`:975`) | Runs, discriminating |
| G-1 | test | TC-474 | Planned |
| G-2 | test | TC-161 seam probe; FR-082 to FR-084 TCs unchanged | Does not discriminate the "one call" AC (FND-011) |
| G-3 | test | TC-291 extended | Planned; AC misses the known collision cases (FND-001) |
| G-4 | test | TC-471 to TC-473 | Planned |
| G-5 | test | "FR-085's TCs" = TC-230 to TC-232 | Planned; unnamed (FND-008) |
| G-6 | test | TC-469 step 1 stays green; parity test gone | Cannot hold as written (FND-002) |
| G-7 | inspection | build with `native_model` removed | Importer list incomplete (FND-002) |
| §12 ConfigVersion | test | TC-469 step 1 | Runs; asserts dispositions only (FND-006) |
| §12 frames | test | TC-514, TC-515 (identity mismatch at `frame_replay.rs:530-700`) | Runs, discriminating for ID-6 |
| §12 engine | test | TC-453 to TC-455 | Runs, over test systems only |
| §12 population id | test | TC-291 to TC-297 (TC-292 is inspection) | Runs |
| PI-1 | test | TC-463 step 1 (`qsl-replay/src/spine/clause/tests.rs:4931`) and TC-469 step 6 (`config_version_spine.rs:910`), both `#[ignore]` on IR-370 | Ignored today (FND-013) |
| EX-9, FE-1, §6 | analysis | none named | No artifact (FND-014) |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | ID-5 says the declared maximum "needs no member" because it "is read from the declaration". That is false. `admit_binding` takes `declared_maximum: Option<u64>` as a caller argument (`qsl-semantics/src/model/population.rs:806-812`), and FR-089 Inputs lists it as an admission input. The existing test `with_population_refuses_a_conflicting_binding_under_a_shared_id` (`qsl-eval/tests/it/model_reference_queries.rs:2834`) admits the same document, package, key and role with maxima 3 and 7. Under G-3 both get the same `members` digest and the same id, so `with_population`'s refusal stays reachable without a broken invariant, and turning it into an `InternalFault` would misreport a caller error. `PopulationBinding` equality also covers `pre_anchor` and `ancestor_steps` (`population.rs:551-599`). Two `Post` bindings with equal post members but different pre documents therefore collide the same way. Fix: decide the full preimage in ID-5. Either add the declared maximum and, for `Post`, the pre binding's id, or remove the `declared_maximum` parameter and read it from the declaration. Add both collision cases to G-3's AC and name the test that replaces the 3-versus-7 test. | ADR-016 §2 ID-5, §9 G-3; FR-089 Inputs; `population.rs:551-599`, `:806`; `model_reference_queries.rs:2834` |
| FND-002 | high | G-6's test says "TC-469 step 1 stays green" after `runtime` and `model_source` are deleted. TC-469's `build()` (`tests/it/config_version_spine.rs:170-175`) calls `support::config_version::model()` and `write()`. That module is `examples/config-version/fixtures.rs`, which imports `quire_spec_language::{model_source, native_model, runtime}` (`fixtures.rs:6-15`) and returns a `NativeModel`. So step 1, and steps 4 to 6 that share `build()`, stop compiling in the G-6 PR. G-7's importer list (SEAM-2, SEAM-3) also leaves out `fixtures.rs`. Fix: add to G-6's AC that the spine generator (`examples/config-version/spine.rs`) writes its files without the native model, and that TC-469 steps 1, 4, 5 and 6 build without `runtime`, `model_source` or `native_model`. Add `examples/config-version/fixtures.rs` to G-7's importer list. | ADR-016 §9 G-6, G-7, §12; `config_version_spine.rs:170-175`; `examples/config-version/fixtures.rs:6-15` |
| FND-003 | medium | OR-2's agreement oracle is given as "a new agreement test in G-2 over every pair of an FR-082 fixture's types". G-2's AC and Test columns do not list that test, so no ticket owns it. OR-2 also leaves out the agreement oracle that exists today: TC-219 and TC-220 `divergence_*_at_check_and_at_evaluation` (`qsl-semantics/tests/it/type_environment_model.rs:206-400`). They assert that `TypeEnvironment::conforms` and the S6a `lookup` agree on five fixtures: unknown supertype, known supertype, cycle, and a chain at and one past the ceiling. The chain past the ceiling is the B-2 versus B-3 case the agreement rule describes. Fix: cite TC-219 and TC-220 as OR-2's current oracle. Either put the all-pairs test into a G row's AC with a TC id, or drop it and state that the five divergence rows are the oracle. | ADR-016 §1 Agreement rule, §10 OR-2, §9 G-2; `type_environment_model.rs:206-400` |
| FND-004 | medium | SC-3 and OR-1 say ambiguous dispatch "refuses at S3 with no partial table". The TC-222 to TC-225 tests call `model::dispatch::link_dispatch` directly (`model_dispatch.rs:305-440`). The S3 bridge `check::checked_dispatch_operation` has callers only in tests (`qsl-eval/tests/it/dispatch_calls.rs`, `model_dispatch.rs:676`, `check/lowering/model/tests.rs:418`), and I found no call to either function from `check::assemble`. No oracle shows a `1-draft` unit with an ambiguous family refusing at S3, and wiring dispatch into the unit pipeline is in no G row. Fix: either add a G row (QSL-68) that links dispatch in the S1-to-S4 unit pipeline, with a unit-level ambiguity test, or restate OR-1 as a model-layer oracle and name the S3 wiring as unavailable work. | ADR-016 §1 SC-3, §10 OR-1, §9; `qsl-semantics/src/check/checked_dispatch.rs:1313` |
| FND-005 | medium | §9 does not list planned #120 model work that `spec/tests.md` already assigns. QSL-19's acceptance requires every unavailable scenario to be a bounded AC. Missing: FR-084-AC-2, TC-227 "allInstances requires both object and subtype closure, never a partial set" (Planned; #120), which is the closure behavior OR-3 names; FR-084-AC-7, TC-410 (Planned; QSL-131); FR-084-AC-5, TC-240 (partly passed); and FR-086-AC-1, AC-2, AC-4 and AC-5, TC-233, TC-234, TC-236 and TC-241 (Planned; #120). Context lists FR-086 systems classification as implemented and on the spine. Fix: add G rows or one grouped row for these, owned by QSL-68, with their ACs and TCs. Correct the FR-086 line in Context to say what is and is not tested. | ADR-016 Context, §9, §10 OR-3; `spec/tests.md` TC-227, TC-233 to TC-241, TC-410 |
| FND-006 | medium | §12 says TC-469 "exercises ID-1, ID-3, ID-6, ID-7, FE-3 and SC-5". Step 1 asserts only the stage, category, truth, catalog code and exit code (`config_version_spine.rs:65-137`). It never asserts a `DeclarationKey`, `NodeKey`, clause node id or document digest. A run that minted a wrong but consistent identity would still pass. §2 has no oracle column. Only some rows have direct evidence: ID-5 (TC-291, TC-296), ID-6 (TC-515 `FrameIdentityMismatch`, `frame_replay.rs:530-700`), and ID-8 and ID-9 (TC-453). ID-1 to ID-4, ID-7, ID-10 and ID-11 have none. Fix: restate the §12 bullet as end-to-end disposition evidence for FE-3 and SC-5. Add an Oracle column to §2 naming a test per row, or "none today" with the owning G row. | ADR-016 §2, §12; `config_version_spine.rs:65-137` |
| FND-007 | medium | OR-7's oracle is "FR-100 outcome table; FR-101-AC-12 (G-1); FR-120-AC-6 (G-4)". The first is a spec table, not a test, and the other two are planned. Tests for the implemented incomplete paths already exist. The S6a meter has TC-469's `Exhausted` case (`Incomplete{limit: "work_units"}`, `config_version_spine.rs:112-135`). Admission has TC-469's `Incomplete` case, TC-226 `l02_unknown_closure_is_incomplete_not_refused` (`model_population.rs:525`) and `tc465_row23_incomplete_population_…` (`state_clauses.rs:3349`). Exploration has TC-455's `Bounded` and `Cancelled` tests. Fix: cite these as OR-7's current oracle and keep G-1 and G-4 for `Stopped`. | ADR-016 §10 OR-7 |
| FND-008 | low | G-5 says "FR-085's ACs" and "FR-085's TCs". They are FR-085-AC-1 to AC-3 and TC-230 to TC-232, all Planned on #120. G-5's interface also promises the `relationship_end` member (ADR-013 O-06), which no FR-085 AC checks. Fix: name the ACs and TCs. Either add an FR-085 AC for the `relationship_end` member or drop it from G-5's interface cell. | ADR-016 §9 G-5; FR-085 Acceptance Criteria; `spec/tests.md` TC-230 to TC-232 |
| FND-009 | low | OR-6 cites "FR-057 backend-absence case" without a test. The test is TC-155 `only_the_supported_item_is_routed` (`qsl-route/tests/it/routing.rs:225`), which settles an `operation-contract` item with an empty candidate set as `unsupported`. FE-4 says no admitted clause reaches the `ProtocolClauseUnsupported` guard. TC-467 exercises the guard by building the expression directly and skipping admission (`evaluate.rs:2150-2166`), so the unreachability claim is verified by analysis, not by test. `spec/tests.md` also describes TC-467 as FR-107-AC-4 to AC-6 "S6a clause entry refuses bad selections", not this guard. Fix: name TC-155 in OR-6. Mark FE-4's unreachability as verified by analysis (grammar and `ValueTypeRef`). | ADR-016 §4 FE-4, §10 OR-6 |
| FND-010 | low | `spec/tests.md` lists TC-464 to TC-469 as Planned, but §12 cites TC-469 as running today and its tests exist in `tests/it/config_version_spine.rs`. A reader cannot tell from the matrix whether the §12 evidence is live. Fix: cite the test functions in §12, as OR-4 does, rather than the TC status. The matrix rows belong to the QSL-273 lane, not this record. | ADR-016 §12; `spec/tests.md` TC-464 to TC-469 |
| FND-011 | low | G-2's AC says the `Value` typer's arms "each make one call" and the seam probe "lists them". TC-161 (FR-063) checks exhaustive-match `E0004` sites against a checked-in list. It cannot tell a thin arm from a thick one. The Test cell "the existing FR-082 to FR-084 TCs unchanged" is a regression guard and does not discriminate the refactor. Fix: state the AC as "the seam list gains the `StateModel` check arms", which TC-161 checks. Move "each arm makes one call" to an inspection item. | ADR-016 §9 G-2; FR-063 |
| FND-012 | low | ND-1 says exploration enumerates every authored choice and "nothing prunes one except a false effective pre- or postcondition". FR-120 also drops candidates that are not closed: "Every reference of a candidate names an object of the candidate" (FR-120 Behavior), and in FR-120-AC-3 "deleting `c2` alone … is not a candidate". Fix: add closure of the candidate state to ND-1's pruning rule. | ADR-016 §5 ND-1; FR-120 Behavior, FR-120-AC-3 |
| FND-013 | low | PI-1 says "No state-node emission test is ignored or weakened in the meantime". Two are ignored today: TC-463 step 1 (`qsl-replay/src/spine/clause/tests.rs:4931`) and TC-469 step 6 (`config_version_spine.rs:910`), both `#[ignore = "blocked on IR-370 …"]`. Fix: say these two stay ignored with the IR-370 reason and are un-ignored in the pin-bump PR, and name that un-ignore as PI-1's oracle. | ADR-016 §11 PI-1, Context "Pinned blockers" |
| FND-014 | low | Three rulings have no named artifact. EX-9 (evidence scope) has no type that carries the request identity and no test. FE-1 ("one frame decision") is true on inspection, since `decide_frame` is `pub(crate)` at `population.rs:1440` and `observation/frame.rs:252` calls it, but no inspection item is named. §6's "never `proved`" relies on consumers that do not exist yet (G-4). Two smaller claims are also inaccurate: ID-5 says `admit_binding` "is called only from tests", but `qsl-bench/src/model.rs:309` calls it, and `FamilyKind::StateModel` is at `family/mod.rs:94`, not `:90`. Fix: mark EX-9, FE-1 and §6 as verified by analysis or inspection, and name G-4's TC-472 for the `Exhaustive` scoping. Correct the two citations. | ADR-016 §3 EX-9, §4 FE-1, §6, §2 ID-5, Context |

## Disposition

Every finding above is fixed on `spec/19-arch40-mapping` in the ADR-016 rewrite and its listed amendments (ADR-011 M-6c and §8; ADR-012 §2, §3, §5.1, §13.5; ADR-013 O-13, T-6, QC-21, O-16, §6; FR-089; FR-120; `spec/tests.md`), except as noted below.

No finding is declined.

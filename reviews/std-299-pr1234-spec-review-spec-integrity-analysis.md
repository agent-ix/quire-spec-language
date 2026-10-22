---
id: SR-15580
title: "integrity review of STD-299 no_std evaluator in quire-eval (QSL PR #1234)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@6c5849b84d35121ae8b4e135367082afd0c57b6c; spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md, spec/test-cases/TC-1121-the-no-std-evaluator-in-quire-eval-matches-the-std-reference.md, spec/assurance/AD-001-shared-evaluation-core-interface.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md, spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-403
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1121
    type: reviews
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: reviews
---
# SR-15580: integrity review of STD-299 no_std evaluator in quire-eval (QSL PR #1234)

## Summary

Ticket: STD-299 (S4 = IR-588, S6 = IR-590). Diff `origin/main...6c5849b84` (main 5a672d88a, an
ancestor of the head), 6 files, spec only. Every code claim was checked against QSL `origin/main`.
The new requirement was checked against merged FR-400, FR-401, FR-402, NFR-015 and ADR-011.

These claims hold:

- `value::definition::divide` and `modulo` exist (`qsl-semantics/src/value/definition.rs:759`,
  `:779`). They are thin wrappers over `quire_exact::divide` and `quire_exact::modulo`, and need only
  `AdmittedIntegerDivision` (moving to C under FR-400), K integers and the meter. `definition` is
  `pub(crate)`, so `qsl_semantics::value::divide` and `modulo` (re-exported at `value/mod.rs:167`)
  are the only public paths. Their only caller is the reference evaluator
  (`qsl-eval/src/value/expression/evaluate.rs:35`, `:1558`, `:1568`).
- `CallableFunction` (`check/mod.rs:720`, with `identity`, `parameters` and `result`),
  `imported_graph` (`check/check.rs:1193`), `value::model_query::{evaluate_lookup,
  evaluate_all_instances}` and the `ObjectEnvironment` reads that the PR body lists all exist.
- The `HashMap` at `model/population.rs:1497` is only used for lookups by key. `admitted` is
  iterated in key order, so the map's order reaches no refusal.
- Apart from the items in FND-002, FND-005 and FND-006, the reference evaluator names only K, SV, C
  terms and `alloc`-available collections (`BTreeMap`, `BTreeSet`, `VecDeque`, `Arc`). It needs no
  `quire-canonical` or `serde`.
- FR-403, TC-1121 and X-16 are unique on main and across all 20 open PR heads. No open PR touches
  AD-001, ADR-011, NFR-015, FR-262 or FR-402. X-16 is consistent with X-13 and X-14, and no item is
  re-exported (§7.2). The PR adds no status wording beyond the "as before the move" phrasing that
  FR-400 and FR-401 also use.

Methods: this artifact covers spec-review/spec-integrity-analysis, including the code grounding.
code-review was not run separately: the diff has no production code, and the code claims were
checked here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: This PR contradicts merged FR-402 and AD-001's S7 paragraph. FR-402 requires two separate runner builds because "one Cargo build unifies features and cannot hold both". It also says the `no_std` build "compiles nothing" and evaluates the cases that the std build admitted (FR-402-AC-9). FR-403 establishes that no byte form carries an evaluable checked graph, so the input can only be handed over inside the process that admitted it. FR-403 also notes that `quire-eval` has no `std` feature, so the unification premise does not hold. FR-402-AC-9 therefore cannot be met as written, and AD-001 lines 204-213 still describe "two separate std and `no_std` builds". <br>Fix: amend FR-402 in this PR, or in a follow-up that blocks IR-590 implementation. Use one runner test build that admits each case once. Run both implementations over the same in-process `CheckedPackage`: the reference through `CheckedPackageEvaluation` and `quire-eval` through `qsl-eval`'s entry. Give each implementation its own 512 KiB thread and a fresh meter. Drop the feature-unification rationale and "compiles nothing". Reword AC-9 to "produces a std and a `no_std` record for every admitted case". The on-target evidence stays FR-401-AC-1's `thumbv7em` gate. Update AD-001's S7 paragraph to match. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:206-214,356-363,423; spec/assurance/AD-001-shared-evaluation-core-interface.md:204-213; spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:231-237 |
| FND-002 | high | major: The environment contract is described as "the reads the std reference evaluator makes of `ObjectEnvironment`", but the list of seven reads is incomplete. Three reads the reference makes are missing. (1) `history(reference, field)`, the FR-138 refinement history read at `NodeKind::History` (evaluate.rs:1853). (2) `population_size(population)`, the authored population bound check (evaluate.rs:348, reached at evaluate.rs:2770 on the relationship-extent path). (3) Resolving a `PopulationId` to its binding's declared maximum: `Machine::resolve_population` (evaluate.rs:1194) and argument admission (mod.rs:435) read it outside the item-6 `allInstances`/`lookup` queries. The closure-only implementation's answers are undefined for all three. <br>Fix: add the three reads, each with its `Observation` where it has one, and state the closure-only answer for each. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:108-135; qsl-eval/src/value/expression/evaluate.rs:348,1194,1853,2770; qsl-eval/src/value/expression/mod.rs:435 |
| FND-003 | high | major: FR-403 does not say who admits call arguments for the `no_std` evaluator. In the reference, every `InputRefusal` comes from `qsl-eval`'s `admit_arguments` (mod.rs:409). That function calls `SuppliedRun::admit` in `qsl-semantics`' `value::supplied`, which FR-403 keeps in std (`call` is `call_supplied` at the default limits). Population pairing and dangling-reference checks also read `ObjectEnvironment`. FR-403-AC-9 forbids supplied-value admission in `quire-eval`. Yet FR-262's Begin returns a "pre-charge input failure", and FR-403-AC-4 expects `f(10)` to return `CallFailure::Input` from the `no_std` evaluator. As specified, either `qsl-eval`'s entry admits first and AC-4 never exercises `quire-eval`, or RT and CG on `no_std` get no argument admission at all. <br>Fix: state where `no_std` argument admission lives and what it checks: arity, kind, domain, population pairing and dangling references. State which environment reads it uses, and how FR-277's supplied-value budget relates to it. Then make AC-4's `f(10)` vector exercise that code. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:184,207-229,263,268; qsl-eval/src/value/expression/mod.rs:409-470; spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:146 |
| FND-004 | medium | FR-403-AC-6 and the closure-only paragraph do not match the reference's behaviour. The reference refuses an unrecorded `PopulationId` at argument admission, as `CallFailure::Input(InputRefusal::WrongValueKind { parameter })` (mod.rs:430-438), with no location and no catalog fields. Inside a body the same case is an `InternalFault` (evaluate.rs:1194-1211), not a family refusal. "The same refusal (code, cause, fields and location)" therefore names a result the reference never produces. <br>Fix: once FND-003 is decided, restate AC-6 against the reference's actual result: `CallFailure::Input` with `WrongValueKind` naming the parameter. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:141-146,265; qsl-eval/src/value/expression/mod.rs:430-438; qsl-eval/src/value/expression/evaluate.rs:1194-1211 |
| FND-005 | high | major: FR-403 does not fix the scope of the `no_std` evaluator, and FR-401's dependency set cannot hold the full machine. FR-403's Inputs are FR-262's (a callable or a `CheckedExpression`). However, the `Observation`-keyed contract and FR-403-AC-5 evaluate a state clause (the FR-107 `ProtocolClause` family) through the `no_std` evaluator. The reference machine builds trails, provenance and stop reports through `s6a::separation`. These name `qsl_foundation::witness::{InvocationBinding, ObservationIdentity, RuntimeValuePath, SeparationStep, ValuePathStep, ValuePathSubject}` and `model::observation::DocumentRef` (separation.rs:21-28, evaluate.rs:19-22). FR-400 moves none of these, and neither FR-401 nor FR-403 places them, so `quire-eval` could not name them. <br>Fix: choose one. (a) The `no_std` evaluator covers the `Value` family (FR-262 inputs), clause, frame, separation and value-invariant evaluation stay reference-only until a named later slice, and AC-5's state-clause half moves out. (b) The witness vocabulary and the observation identity move to SV, and FR-403 says so (ADR-011 row and TC). | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:50-62,108-118,264; qsl-eval/src/value/expression/s6a/separation.rs:21-28; qsl-eval/src/value/expression/evaluate.rs:19-22 |
| FND-006 | medium | The S4 disposition table is incomplete, so "No evaluator-needed part stays in `qsl-semantics` that the `no_std` evaluator calls directly" is false. The reference evaluator's own causes name four items the table omits. (1) `qsl_semantics::family::FamilyKind::StateModel.catalog_code_prefix()` (causes.rs:371,441); FR-401 keeps `FamilyKind` in `qsl-semantics`. (2) `model::refusal::render_identity` (causes.rs:389). (3) `model::observation::AdmissionRecord` (causes.rs:20). (4) `ModelRefusal::type_mismatch` and `absent_key`, which the evaluator itself raises on FR-391 named-population lookups (evaluate.rs:2505,2559) and not through the item-6 query. Under lead default 5, `quire-eval` has to reproduce these catalog codes, prefixes and renderings, which makes a second copy of the StateModel catalog. Also, `ModelRefusalCause` is defined in `model::refusal` (refusal.rs:338), not `model::normalize`. <br>Fix: give each of these a disposition: move it to SV or E, or have `quire-eval` duplicate it with a parity AC. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:157-172; qsl-eval/src/value/expression/causes.rs:20,371,389,441; qsl-eval/src/value/expression/evaluate.rs:2505,2559 |
| FND-007 | medium | No slice now owns `qsl-semantics`' `std` feature. NFR-015's `qsl-package` closure-tier condition still requires `qsl-semantics` to build with `std` disabled, with `serde_json`, `tempfile` and the FCD crates behind its `std` feature, and `qsl-package`'s `std` enabling `qsl-semantics/std`. This PR removes S4 as that condition's owner, and FR-403 says `qsl-semantics` gains no `std` feature. AD-001's S5 bullet ("needs `no_std` layers 3, 2 and 1") also names no owner. NFR-015's Rationale ("so S3 and S5 do not wait for S4") is left with nothing to refer to. <br>Fix: state that the `qsl-package` closure tier is outside AD-001's slices, or name the slice or ticket that owns it. Remove the stale Rationale clause. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:116,197-198; spec/assurance/AD-001-shared-evaluation-core-interface.md:179-180 |
| FND-008 | medium | FR-403's Description says the evaluator exists "so that RT and CG can evaluate checked code on a `no_std` target". FR-262 makes IR-590 deliver "the required evaluation, checked-input and generated-body surface" for RT. But FR-403 gives a `no_std` backend no way to obtain a `CheckedGraph`. FB-14 and FR-400 forbid backends from enabling `construct`, and no `no_std` reader of a checked graph exists. The PR body defers this to RT FR-273-AC-12, but the spec says nothing about it. <br>Fix: record the checked-input surface as an explicit open dependency of IR-590 in FR-403 and AD-001, naming the owning requirement. Or bring it to the owner as part of S6's scope. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:21-26; spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:88-93 |
| FND-009 | low | FR-403-AC-1's "names no `HashMap`, `HashSet` or `std::` path" clause cannot fail on its own: the crate is `#![no_std]`, and FR-401's dependency set has no hash-map crate. The rule that matters is Behavior item 2: a collection whose iteration order reaches a result is ordered. That rule has no AC. <br>Fix: replace the clause with a check that is tied to ordering, or drop it. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:74-82,260 |
| FND-010 | low | FR-403-AC-2's "as before the move" has no baseline: TC-1121 step 2 records no pre-move values or charges. <br>Fix: compare against `quire_exact::divide` and `modulo` called directly with the same law and meter, or against a golden recorded on main. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:261; spec/test-cases/TC-1121-the-no-std-evaluator-in-quire-eval-matches-the-std-reference.md:38-42 |
| FND-011 | low | "Compiles the same evaluator source as `quire-eval`'s `thumbv7em-none-eabi` build" is true of `quire-eval`'s own source only. In QSL's build, feature unification turns on `quire-checked`'s `std` and `construct` and `quire-exact`'s `test-support`. <br>Fix: state that the QSL parity run is a host build with unified dependency features, and that the target evidence is FR-401-AC-1's gate alone. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:231-237,246-254 |
| FND-012 | low | FR-403's frontmatter has no relationship to FR-402, which it names as the suite that compares the two implementations. FR-089, FR-106 and FR-364 are named in Dependencies without links. <br>Fix: add the FR-402 edge and the links. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:1-18,281-283 |

## Verdict

Not mergeable as is. There are four majors.

- FND-001: a direct contradiction with merged FR-402-AC-9 and AD-001's S7 paragraph.
- FND-002: the environment contract omits three reads that the reference makes.
- FND-003: no argument admission is assigned for the `no_std` evaluator.
- FND-005: the evaluator's family scope is open, and the witness vocabulary is unplaced.

The four mediums need decisions before the next round. The four lows can be fixed in the same round.

The author is right about the FR-402 conflict. One Cargo build can hold both implementations,
because `quire-eval` has no `std` feature and `quire-checked`'s `std` feature does not change
evaluation. FR-402's two-build premise and the "compiles nothing" rule should therefore go. The
handover in the admitting process stays.

Lead defaults (each is a lead default, not an owner answer):

1. The evaluator lives in `quire-eval`, and `qsl-eval` consumes it and holds the reference. This
   is sound and consistent with FR-401 and FR-402.
2. The model reads stay in `qsl-semantics` behind a contract. This is sound, but the contract
   needs FND-002's three reads.
3. S4 converts nothing to `no_std`. This is sound, but the disposition table needs FND-006, and
   NFR-015's orphaned closure-tier condition needs FND-007.
4. The closure-only environment is sound, but its stated refusal behaviour is wrong (FND-004).
5. Each implementation raises its own causes, and parity goes through `CatalogCoded` and
   `UndefinedCoded`. This is workable, but it duplicates the StateModel catalog codes, the prefix
   and `render_identity` in `quire-eval` (FND-006). That duplication is an owner decision.
6. The reference stays the default until S9, behind a separate entry with no re-export. This is
   sound.

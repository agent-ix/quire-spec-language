---
id: SR-13113
title: "Gap analysis of QSL-544 part 1 map_state against FR-140, TC-545, TC-555 and ADR-020 section 8 (PR #1032)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@54d8d72b6; spec/functional/FR-140-*.md, spec/test-cases/TC-545-*.md, spec/test-cases/TC-555-*.md (step 1), spec/decisions/ADR-020-*.md (sections 1 and 8), FR-141-AC-7 (context); qsl-replay/src/refinement/map.rs, qsl-eval/src/simulation/model/refine.rs, qsl-eval/src/simulation/model.rs, qsl-replay/tests/it/refinement_mapping.rs, qsl-replay/tests/it/refinement_fixture.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: reviews
---
# SR-13113: Gap analysis of QSL-544 part 1 map_state against FR-140 (PR #1032)

## Summary

Ticket: QSL-544 (E13), part 1. Tool versions: `quoin matrix --json`
(quoin 0.28.3, quire 0.36.2, engine 0.50.2), exit 0. Plan completion: not
assessed.

The matrix shows FR-140-AC-1 as `tagged` by
`refinement_mapping::a_concrete_state_maps_to_the_abstract_state_with_its_value`,
and that test asserts what AC-1 says. FR-140-AC-2 to AC-5 are `untagged`. The
other three tests carry TC-level tags only (`TC-545`, `TC-555`), so they do
not inflate any AC row. Each one tests what its TC describes, on the counter
fixture rather than the TC's own fixture, and the PR body says so. The PR
claims no more than it builds. FR-141, FR-142, FR-144 (safety), FR-145,
QSL-545 (history) and QSL-546 (population-valued forms) are declared out of
scope and are not claimed.

Probes ran in a scratch clone at 54d8d72b6 with its own `CARGO_TARGET_DIR`,
with the fixture's `post Inc` and mapping row swapped:

- ADR-020 §8's `post Inc { self.value = pre(self.value) + 1 }` refuses
  `Unproved(Range{required [0,2], proved [1,3]})` at `Inc`, as the coder
  reported. The fixture's `self.value - pre(self.value) = 1` checks.
- Row `value = 2 / self.value` and row `value = self.value / 1` each refuse
  `ill_typed`/`type-mismatch`.
- `self.value / 1 - pre(self.value) = 1` in a post clause refuses
  `IllTyped(TypeMismatch)`.
- `div` and `rem` refuse `MissingSelection{IntegerDivision}`.
- Row `value = self.value - self.tmpA` refuses `type-mismatch`, because its
  range is outside `Int[0, 2]`.

## Verdict

**PASS for part-1 scope, FAIL at FR-140 scope (expected).** The code has no
gap against what part 1 claims. The findings are spec and plan gaps for the
planner, posted as a Linear note. They cover:

- FND-001: reference lifting is only partly built, and no AC tests it.
- FND-002: AC-4's history half and AC-5 have no ticket that can build them.
- FND-003 and FND-004: ADR-020 §8 and FR-141-AC-7 cannot be checked as
  written.
- FND-005 to FND-008: smaller text gaps in FR-140 and FR-120.

The coder's listed deviations are judged as follows:

- Concrete `ModelSystem` argument: spec gap (FND-005).
- No `HistoryValues` argument: plan-consistent until QSL-545 (FND-005).
- `EvalFailure` variants: spec gap (FND-005).
- `Unsupported(NestedReference)`: partial build of a SHALL. It is sound, but
  it needs an AC and an owner (FND-001).
- `UnmappedType`: spec gap (FND-006).
- `#[allow(clippy::result_large_err)]`: reasoned, with workspace precedent.
  Not a defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-140 Behavior says "A concrete reference value SHALL map to the abstract reference with the same key". ADR-020 RM-3 does not limit this to top-level values. `lift` rewrites only a top-level `Value::Reference` and refuses a reference inside an option, record, union or collection as `Unsupported(NestedReference)`. So a field of type `Sequence<Reference<T>>` or `Option<Reference<T>>` maps to `Undetermined`, which is inconclusive and never a wrong answer. No FR-140 AC exercises reference lifting at all, top-level or nested. Spec: add an AC for each case. Plan: name the ticket that builds nested lifting. | qsl-replay/src/refinement/map.rs:301-321; spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:91-92 |
| FND-002 | medium | Ownership gap. QSL-546 owns FR-140-AC-2 and AC-3. QSL-545 implements FR-138 only and is blocked by QSL-544. FR-140-AC-4's first half (history rows, `RegisterHistory`) needs QSL-545, and FR-140-AC-5 needs FR-139's `RingIsQueue` (QSL-546). Both stay with QSL-544, which depends on neither. As planned, QSL-544 cannot close with AC-4 and AC-5 built: QSL-545 waits on QSL-544, and QSL-544 does not wait on QSL-546. The planner should move AC-4's first half to QSL-545 and AC-5 to QSL-546, or add those blockers to the part of QSL-544 that builds them. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:117-118 |
| FND-003 | medium | ADR-020 §8 cannot be checked as written. `post Inc { self.value = pre(self.value) + 1 }` refuses at S3 with `Unproved(Range{required [0,2], proved [1,3]})` (probed). `CommitAPost`'s `self.value = pre(self.tmpA) + 1` has the same range shape and was not probed. FR-140-AC-1 cites the §8 fixture, so the test fixture rewrites both clauses as subtractions: `self.value - pre(self.value) = 1` and `self.value - pre(self.tmpA) = 1`. The PR body lists the `Inc` rewrite but not the `Commit` one. ADR-020 §8 needs clause text that checks. | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:347; spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:352; qsl-replay/tests/it/refinement_fixture.rs |
| FND-004 | medium | FR-141-AC-7 and TC-555 step 2 cannot be checked as written. `put -> enq(self.ring, 2 / self.size)` cannot check: in this build `/` is ill-typed against an integer target (probed: `ill_typed`/`type-mismatch` in a mapping row, and `IllTyped(TypeMismatch)` in a post clause). `div` refuses `MissingSelection{IntegerDivision}` without a division definition selection. Even with one, `2 div size` over `size: Int[0, 2]` is in [1, 2] where defined, which does not conform to `enq`'s `x: Int[0, 1]`, so RM-5 typing refuses the row. A `division-by-zero` undefined argument needs a row that types against `Int[0, 1]`, under a selected integer-division law. The coder's wording "`/` is prohibited in state clauses" is more precisely "`/` does not type against an integer target". This is for the planner; it is out of scope here (FR-141). | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:185 |
| FND-005 | low | FR-140's Inputs and Outputs no longer match a workable build. (a) The rows run in the concrete package's evaluator over the concrete state's observation, so `map_state` needs the concrete `ModelSystem`. Inputs lists only the abstract one. (b) `history: &HistoryValues` is in the signature, but S3 refuses history rows until QSL-545. (c) `MappingFailure::Undefined.cause` is `UndefinedRecord` in the text and `UndefinedCause` (kernel or family) in the code. (d) `EvalFailure` appears only as a comment, "refused, incomplete, out of type". The RM-4 typing probes show out-of-type is refused statically, while the code adds `Unsupported`, `UnmappedType` and `Stopped`. The spec should enumerate `EvalFailure` and state the signature. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:58-76; qsl-replay/src/refinement/map.rs:66-77 |
| FND-006 | low | A concrete object whose most-specific type has no object row. RM-3 requires a row per abstract type, not per concrete subtype that a source population admits, so a population of a supertype with a subtype object has no rule. `map_state` reports `Undetermined` with `UnmappedType`, which is sound. The spec should either make S3 (FR-135) refuse a source population that admits an unmapped most-specific type, or map the subtype through its nearest mapped supertype. | qsl-replay/src/refinement/map.rs:159-165; spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md |
| FND-007 | low | `ModelSystem::over_model` (a system over one of the models of a unit that selects two, ADR-020 RM-1) is new public `qsl-eval` API that no requirement states. Its refusal reuses FR-120's `invalid_runtime_input`/`wrong-role-mapping`, which FR-120 defines for a universe naming an undeclared population. FR-120 (or FR-140's Inputs) should state how a subject's system is built from a two-model unit, and which code refuses an unknown model. | qsl-eval/src/simulation/model.rs:384-418 |
| FND-008 | low | The relationship extents of a mapped state are not specified. `map_state` always returns `extents: []`. FR-101's state key does not encode extents, so key equality (FR-140-AC-1) is unaffected. But FR-141 evaluates abstract clauses over a mapped state's observation, which would then show no relationship instances. FR-140 or ADR-020 RM-3 should say whether abstract relationships are mapped or refused. | qsl-replay/src/refinement/map.rs:143-146 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Spec gap: needs ACs for top-level and nested reference lifting and a ticket that builds nested lifting. Deferred to the planner's new STD ticket ("missing rules get ACs"); the code's `Unsupported(NestedReference)` refusal is sound until then. The brief gave no ticket id, and none was found in Linear. |
| FND-002 | deferred | Plan gap: deferred to the planner's re-slice of the QSL-544, QSL-545 and QSL-546 ACs (new STD ticket). |
| FND-003 | deferred | Spec defect: deferred to the planner's new STD ticket (ADR-020 §8 guard). The fixture keeps its checking subtraction form until then. |
| FND-004 | deferred | Spec defect: deferred to the planner's new STD ticket (FR-141-AC-7 rewrite). It is out of scope for part 1 (FR-141). |
| FND-005 | deferred | Spec gap: deferred to the planner's new STD ticket (FR-140 signature and `EvalFailure`). |
| FND-006 | deferred | Spec gap: deferred to the planner's new STD ticket ("missing rules get ACs"); `UnmappedType` is now tested (SR-13112 FND-002). |
| FND-007 | deferred | Spec gap: deferred to the planner's new STD ticket (an FR-120 rule for `over_model`). |
| FND-008 | deferred | Spec gap: deferred to the planner's new STD ticket (relationship extents of a mapped state). |

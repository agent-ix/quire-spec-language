---
id: SR-13120
title: "Gap analysis of QSL-546 E13d population-valued forms against FR-139, TC-544, FR-140-AC-2/3, QSpec FR-376/TC-311 and ADR-020 (PR #1035)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@214d15b31; spec/functional/FR-139-*.md (AC-1..5), spec/functional/FR-140-*.md (AC-2, AC-3), spec/test-cases/TC-544-*.md, spec/decisions/ADR-020-*.md (RM-8, QS-10, RU-5, section 8); QSpec agent-ix/quire-specification@2c2918b FR-376, TC-311, proposals/quire-v1/shared-grammar.md, proposals/quire-v1/definitions/native-diagnostics.md (context); qsl-semantics/src/check/{refinement,check/typing}.rs, qsl-eval/src/value/expression/{mod,evaluate}.rs, qsl-semantics/tests/it/refinement_populations.rs, qsl-eval/tests/it/refinement_populations.rs, qsl-forms/tests/it/refinement_forms.rs, tests/support/ring_queue.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-544
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: reviews
---
# SR-13120: Gap analysis of QSL-546 population-valued forms (PR #1035)

## Summary

Ticket: QSL-546 (E13d). Tool versions: `quoin matrix --json` (quoin 0.28.3),
exit 0. Plan completion: not assessed.

The matrix shows FR-139-AC-1 to AC-5 as `tagged`, and FR-140-AC-2 and AC-3 as
`untagged`. That matches the PR body: FR-140-AC-2 and AC-3 need `map_state`,
`MappedState` and `MappingFailure` from QSL-544 (PR #1032), and the PR
declares them not built. `evaluate_row` is the hook for them.

Each tagged test was read against its AC:

- AC-1: the S3 types of `items`, `count`, `exists` and the population, and
  the admitted `Set<Reference<R::Slot>>[0, 2]` and `Int[0, 2]`. Truthful.
- AC-2: the `count` invariant refuses `expression-form`, and `only` with
  `where s.index` refuses `type-mismatch` at `s.index`. Truthful.
- AC-3: `[1, 0]`, `[]`, 2 and `false`. Truthful for
  `the_ring_rows_evaluate_over_the_slots_in_the_observed_state`.
- AC-4: `found=2` and `found=0` give `only-not-one`. Truthful. The fixture
  uses two of the fixture's states, not every state.
- AC-5: S3, then `admit_universes` at 2 and 3. Truthful at the seam only
  (FND-003).

The QSpec tags and three of the FR-139-AC-3 tags claim more than their tests
show (FND-002, FND-004).

`admit_universes` has no production caller, and the PR says so. To finish
FR-139 "Subject admission" and FR-139-AC-5, QSL-544 must do the following
where a request admits the concrete subject's universes (the `ModelSystem`
built for the concrete model, `ModelSystem::over_model` on PR #1032), before
exploration and before any `map_state`:

1. Call `CheckedRefinement::admit_universes` with each concrete population's
   `DeclarationKey` mapped to its universe size.
2. Surface every returned `RefinementRefusal` as a request refusal.
3. Add a request-level test that refuses with `slots = {s0, s1, s2}` before
   exploration.

QSL-544 also has to:

- Map `StateModelUndefined::OnlyNotOne` to `MappingFailure::Undefined` for
  FR-140-AC-3.
- Keep exactly one of `evaluate_row` and `evaluate_refinement_row`
  (SR-13119 FND-005).
- Note that a whole-population row (`cells = R::slots`) yields a set of
  references, which PR #1032's `lift` currently reports as
  `Unsupported(NestedReference)`.

Probes ran in a scratch copy at 214d15b31 with its own `CARGO_TARGET_DIR`:

- ADR-020 §8's literal `items` row
  (`s.index = (self.head + i) mod 2`) refuses `ill_typed`/`type-mismatch` at
  the equality. An invariant `self.index = (self.index + 0) mod 2` also
  refuses, so this is pre-existing equality typing, not this PR. Without a
  clause that selects the complete value profile, `mod` refuses
  `expression-form`.
- AC-4's literal `s.index = self.head + 2` checks, so the fixture's `+ 0` is
  not needed for the `ghost` row.
- `sum(s in R::slots: s.value)` into `Int[0, 2]` checks and evaluates to 2.
  No committed test covers it (FND-001).

The coder's open questions are judged as follows:

- Q1: `only-not-one` is a spec gap (FND-007).
- Q2: the `sum` bound `[min(0, n*lo), max(0, n*hi)]` is correct. QSpec's
  literal "n times e's bounds" is unsound, because `S` can hold anywhere from
  0 to n objects, so the type must include 0 when lo > 0. This is a spec gap
  in QSpec FR-376 (FND-008).
- Q3: a pre-existing language rule plus a spec gap about which value profile
  governs refinement rows (FND-009).
- Q4: a QSpec spec gap (FND-010).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `sum` is untested at every stage. FR-139's Outputs and Behavior require `sum(x in P where c: e)` to type, record its size-dependent bound, be checked at admission, and evaluate to the sum over `S` (0 when `S` is empty). No committed test types a `sum` row, reads its `PopulationValue::Sum` use or `admitted_type`, or evaluates one. The only appearance is the qsl-forms shape test. The QSpec bound deviation (Q2) is therefore also unpinned by any test. | qsl-semantics/src/check/refinement.rs:250-330; qsl-semantics/tests/it/refinement_populations.rs |
| FND-002 | medium | Trace overclaim on `QSpec-FR-376-AC-1`, "the six forms have exactly the types and values of the table". It is tagged on `the_ring_rows_check_at_s3_with_the_universe_size_symbolic` and `the_ring_rows_evaluate_over_the_slots_in_the_observed_state`. Neither test types or evaluates `sum`. `count` is checked as `Integer` at S3 and as `Int[0, n]` only through `admitted_type`. TC-311 step 1's expressions (`sum`, `exists where s.index = 1`, `only where s.index = 0`, `seq(i in 0 .. 1: i)`, `seq(i in 1 .. 0: i)`) are not run, yet `QSpec-TC-311` is tagged on nine tests. Drop the AC-1 tag or add the missing forms. | qsl-semantics/tests/it/refinement_populations.rs:46-54; qsl-eval/tests/it/refinement_populations.rs:192-201 |
| FND-003 | medium | FR-139-AC-5 is verified at the seam, not as written. The AC says "a request whose subject admits `slots = {s0, s1}` admits it, and one that admits `{s0, s1, s2}` refuses ... before exploration". `a_universe_that_overflows_the_used_type_is_refused_at_admission` calls `CheckedRefinement::admit_universes` directly, and no request path calls it (unwired by design, per the PR body). The matrix now shows AC-5 as `tagged`, so QSL-544 must add the request-level assertion when it wires `admit_universes`. Otherwise AC-5 reads as covered while a request never runs the check. | qsl-semantics/tests/it/refinement_populations.rs:128-176; qsl-semantics/src/check/refinement.rs:361-408 |
| FND-004 | low | Three tests are tagged `FR-139-AC-3` but assert nothing that AC-3 states. AC-3 is the four values `[1, 0]`, `[]`, 2 and `false`. `a_population_name_evaluates_to_its_objects_in_ascending_key_order` and `a_form_charges_one_work_unit_per_object_visited` test FR-139 Behavior bullets (population order, metering). `renaming_the_slot_keys_leaves_every_mapped_value_unchanged` tests QSpec FR-376-AC-5. Tag them `TC-544` (or `FR-139`) only. | qsl-eval/tests/it/refinement_populations.rs:222-257 |
| FND-005 | low | The `ghost` fixture row differs from FR-139-AC-4's text: `s.index + 0 = self.head + 2` instead of `s.index = self.head + 2`. A probe shows the literal form checks, so the `+ 0` is unnecessary there and the AC-4 test does not test the AC's expression. The `items` row's `+ 0` is needed because of FND-009. | tests/support/ring_queue.rs:319 |
| FND-006 | medium | Plan gap (planner). QSL-546 states "Implements: FR-139, FR-140 (AC-2/3)", and its acceptance requires tests backing FR-140-AC-2/3. Those ACs need `map_state` and `MappingFailure` from QSL-544, but QSL-546's "Needs" lists only QSL-543 and STD-133. Either add QSL-544 as a blocker and keep QSL-546 open until AC-2/3 land, or move FR-140-AC-2/3 (and the `admit_universes` wiring) to QSL-544. As planned, merging PR #1035 cannot close QSL-546. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md |
| FND-007 | low | Spec gap (planner, Q1). `only` undefinedness has no catalog reason. QSpec native-diagnostics.md "Undefined reasons" is a closed set (`absent-key`, `precondition-false`), and neither QSpec FR-376 nor QSL FR-139 or FR-090 names a reason for `only`. The PR adds `UndefinedReason::OnlyNotOne` (`only-not-one`, payload `population`, `found`). QSpec native-diagnostics.md and FR-376, and QSL FR-139 (or FR-090's reason list), should add it with its payload. | qsl-foundation/src/diagnostic.rs:1297-1310 |
| FND-008 | medium | Spec gap (planner, Q2). QSpec FR-376's table types `sum` as "`e`'s numeric type, with bounds `n` times `e`'s bounds". For `e` in `[lo, hi]` with `lo > 0`, that is `[n*lo, n*hi]`, which excludes the 0 that FR-376 itself requires "when `S` is empty". `S` can hold anywhere from 0 to n objects, whether or not a `where` clause filters it, so the sound bound is `[min(0, n*lo), max(0, n*hi)]`. That is what the PR implements. Amend QSpec FR-376 (and TC-311's expectation, if it states a bound). | qsl-semantics/src/check/refinement.rs:313-327 |
| FND-009 | medium | Spec gap (planner, Q3). ADR-020 §8's `items` row, quoted verbatim by QSpec FR-376-AC-5 and TC-311 step 2, does not check in QSL. (a) `Int[0, 1] = <Integer> mod 2` refuses `ill_typed`/`type-mismatch`. This is pre-existing equality typing, and an invariant probe shows it outside refinements too. (b) `mod` requires the complete value profile, but refinement rows take the unit-wide value-profile flag, and ADR-020 and FR-135 do not say which value profile governs a refinement row (the declaration names only its temporal profile, `using inf`). So the fixture adds an unrelated `invariant ... using v` and writes `s.index + 0`. The spec needs a §8 row that checks, and a rule for a refinement row's value profile. | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:473-474; tests/support/ring_queue.rs:305-308 |
| FND-010 | low | Spec gap (planner, Q4, QSpec). FR-376's Dependencies say the Complete-V1 grammar (`proposals/quire-v1/shared-grammar.md`) "owns the expression productions this requirement extends". That file does not list the new `primary` alternatives, the `binder` production, or the `..` token, so the two documents disagree. | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md |

## Verdict

**FAIL at FR-139 scope for this round.** Code defects in SR-13119:

- FND-001 (high): the population-row bound uses equality.
- FND-002 and FND-003: a population name and `seq` outside a refinement do
  not refuse `expression-form`.
- FND-004: double-charged filtered visits.
- FND-005: the duplicate entry point and merge conflict with PR #1032.

Gap findings for the coder in this round:

- FND-001 (`sum` tests).
- FND-002 and FND-004 (tags).
- FND-005 (fixture text).
- FND-003, a trace caveat that QSL-544 must close.

FND-006 to FND-010 are spec and plan gaps. They go to the planner as a
Linear note and are not fixed in this PR.

FR-140-AC-2/3 are correctly declared not built and not claimed. The PR body
claims no more than built, except for the `expression-form` sentence (SR-13119
FND-002/FND-003). No compatibility shim was found.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | Spec gap (planner). FR-139-AC-4's second `only`, written `only(s in R::slots where s.ring = self and s.index = self.head + 2: s.value)`, cannot evaluate undefined as the AC requires. The equality coerces `self.head + 2` into `s.index`'s `Int[0, 1]`, and at `head = 1, size = 2` the row evaluates `Refused(IntegerOutOfDomain { target: [0, 1] })` (probed at e8a43b6e6). That is a refused row, which FR-140 reads as `map` undetermined, not refuted. The fixture's `s.index + 0` gives the AC's intended "no slot satisfies it". TC-544 step 4 has the same text. Rewrite the AC expression, or settle how an out-of-range comparison operand should behave. | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md; tests/support/ring_queue.rs:313-325 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | still-open | The two tests that keep `QSpec-FR-376-AC-1` (`the_ring_rows_check_at_s3_with_the_universe_size_symbolic`, `the_ring_rows_evaluate_over_the_slots_in_the_observed_state`) still exercise no `sum`. The new `sum` tests are tagged `TC-544` only, so no test tagged with the AC covers the sixth form. `QSpec-TC-311` stays on 8 tests while TC-311 step 1's expressions are not run. Add `QSpec-FR-376-AC-1` to `a_sum_records_its_bound_and_is_checked_at_admission` and `a_sum_adds_the_matching_objects_and_each_form_visits_an_object_once`, or drop it from the two ring tests. |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | rejected | The finding was wrong. The literal `s.index = self.head + 2` passes S3, but at evaluation it gives `Refused(IntegerOutOfDomain)` and not the undefined `only` that AC-4 tests (probed). The fixture's `+ 0` is needed and is now explained in a comment. The spec side is FND-011. |
| FND-006 | deferred | Planner re-plan: FR-139 stays with QSL-546; FR-140-AC-6/9/10/11 and FR-135-AC-6 move to QSL-546, and QSL-544 (#1032) lands first. The ticket's FR-140 scope belongs to the planner, not this PR. |
| FND-007 | deferred | Spec gap: posted in the planner note on QSL-546 (QSpec native-diagnostics and FR-376, QSL FR-139 or FR-090 `only-not-one` entry). |
| FND-008 | deferred | Spec gap: posted in the planner note on QSL-546 (amend QSpec FR-376's `sum` bound to `[min(0, n*lo), max(0, n*hi)]`). The PR's test cites it as a pending FR-139 AC. |
| FND-009 | deferred | Spec gap: posted in the planner note on QSL-546 (ADR-020 §8 row that checks, and a value-profile rule for refinement rows). |
| FND-010 | deferred | Spec gap: posted in the planner note on QSL-546 (QSpec shared-grammar.md productions and `..`). |

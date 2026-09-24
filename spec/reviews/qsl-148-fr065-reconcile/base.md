---
id: SR-618
title: "Base review of the QSL-148 FR-065 reconciliation"
type: SpecReview
analysis: base
scope: "Working-tree change on spec/148-fr065-reconcile: FR-065 (Description, the application-check and one-S3-checker Behavior sections, CON-2, CON-3, AC-4, AC-5, Status, OQ-1), TC-163, TC-164 (renamed), TC-376, TC-377, and the spec/tests.md and spec/spec.md rows"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-163
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-164
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-376
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-377
    type: reviews
---

## Summary

The new and changed IDs are well formed: FR-065-CON-3 and FR-065-OQ-1 follow
the flat `FR-NNN-CON-N` and `FR-NNN-OQ-N` shape that FR-092-OQ-1 and
FR-090-OQ-1 use. TC-164 keeps its ID under a new file name. Each of FR-065's
eight ACs names a TC. AC-4 names TC-376, and AC-5 names TC-164. The
spec/tests.md rows match: TC-163 covers AC-1 to AC-3 and AC-8, TC-164 covers
AC-5, and TC-376 covers AC-4.

Both amended ACs are now behavioural, and each fails when its behaviour is
removed:

- AC-4. The four `checking_tests` in `qsl-semantics/src/check/family.rs`
  (lines 1614 to 1716) build an `Expression::Call` and type it through
  `Typer::infer`. That reaches `infer_form`'s `Call` arm (`typing.rs:752`),
  then `Application::resolve`, `Typer::application` (`typing.rs:1112`),
  `Application::parameter` and `Application::finish`. Remove the arity check
  in `resolve` and `f(true, false)` is admitted with one typed argument.
  Remove the `MissingName` arm and `nowhere()` changes cause. Remove
  `parameter` and `f(1)` is admitted. Each test then fails. AC-4's fixtures
  match the tests exactly.
- AC-5. It checks each call against a named cause in both positions, so it
  fails when either path stops going through `Application`, and it also fails
  when both paths go wrong the same way.

No shape test is left. TC-163 step 6 is gone. The thin-arm rule is
FR-065-CON-3, which is verified by inspection. CON-3 matches the code: the
`Call` arm makes one `Application::resolve` call and builds an
`ApplicationFrame`.

The named symbols and paths exist: `Application::{resolve, parameter,
finish}` (`family.rs:710-816`), `check_declaration_body` (`family.rs:921`),
`ValueFunctionFamily::check` (`family.rs:1155`), `PackageDeclarations::check`
returning `Result<CheckedGraph, Vec<CheckRefusal>>` (`mod.rs:499`), its one
call to `ValueFunctionFamily::check` (`mod.rs:825`), and
`CheckedGraph::check_clause_expression(parameters, expression, expected,
clause_kind, mode, limits)` (`mod.rs:1102`). `ClauseKind::Precondition`
exists (`qsl-forms/src/syntax.rs:209`). `qsl-eval/tests/it/dispatch_calls.rs`
already calls `PackageDeclarations::check` and `check_clause_expression` on
a built package (lines 295 to 330 and 583), so TC-164 can be implemented
against the public API.

The TC-376 Passed row is backed by a test for every step. The FR-065-AC-4
"backed" claim does not follow the FR's own rule for what counts as backed
(FND-001). TC-164 is Planned and has no test, and the Status section says so.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-065 Status calls AC-4 "backed", and spec.md and tests.md repeat it. But the four TC-376 tests carry only `#[trace("TC-376")]`, with no `FR-065-AC-4` tag. The same Status section counts AC-1, AC-3 and AC-6 as unbacked because no `#[trace(..., "FR-065-AC-N")]` tag exists. Backed AC-7 carries `#[trace("TC-380", "FR-065-AC-7")]`. So AC-4 is counted by a looser rule than its siblings, and a trace-driven gap analysis will report it as uncovered. Fix: in the QSL-148 implementation PR, change the four `check_application_*` tests to `#[trace("TC-376", "FR-065-AC-4")]`. Until that lands, write AC-4 in Status as "behaviour tested by TC-376; AC tag pending". | FR-065:318-330, :363-366; family.rs:1614, :1641, :1667, :1695; spec.md FR-065 row |
| FND-002 | low | TC-164 leaves out details a test author needs. (1) Step 3 does not name the `CheckMode`, and `check_clause_expression` requires one. `Linked` runs `Definedness` and `Kernel` does not. (2) Step 3 does not say that `parameters` is empty. (3) "The same verdict" does not say what is compared. The two refusals' locations differ (`Origin::Body` in step 2, `Origin::Expression` in step 3), so only the cause can match. (4) Step 2 returns a `Vec<CheckRefusal>` and does not say how many entries it expects. Fix: step 3 should read "with no parameters, `CheckMode::Linked`, `CheckingLimits::default()`". The expected results should say that the refusal's `cause` is compared, that the location is not, and that step 2's refusal vector holds exactly one refusal. | TC-164 steps 2-3 and Expected Results; FR-065-AC-5; mod.rs:1102-1110 |
| FND-003 | low | The Behavior section says the typer checks every call "in a function declaration's body or `decreases` measure". No AC or TC step checks a call in a `decreases` measure, so a measure path that bypassed `Application` would pass every test. Fix: add a third position to AC-5 and TC-164, a `decreases` measure checked through `PackageDeclarations::check`, with the three refused calls (for example `decreases nowhere(true)` refused `missing-name` naming `nowhere`), or remove "or `decreases` measure" from the Behavior sentence. | FR-065:99-104; FR-065-AC-5; TC-164 |

## Resolution

- FND-001: the four TC-376 tests now carry
  `#[trace("TC-376", "FR-065-AC-4")]`, and FR-065's Status says so.
- FND-002: TC-164 now names the `check_clause_expression` arguments, compares
  verdicts by refusal cause only and expects exactly one refusal in step 2.
- FND-003: FR-065-AC-5 and TC-164 step 4 add the `decreases` measure position
  for the three refused calls.

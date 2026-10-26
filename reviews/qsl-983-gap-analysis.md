---
id: SR-1595002
title: "gap analysis of QSL-983 terminal declaration emission (QSL PR 1595)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5ae5da93c5309a0dd686c40ae564be264b37d2c4; spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md, spec/functional/FR-128-replay-a-model-counterexample.md (AC-6), spec/functional/FR-382-build-the-terminal-declaration-form-at-s2.md (AC-6), spec/test-cases/TC-1142-a-model-terminal-declaration-emits-a-v2-terminal-node.md, qsl-package/src/emit/tests/terminal.rs, qsl-analyze/tests/it/model_settle.rs, qsl-semantics/tests/it/{terminal_declarations,refinement_declarations}.rs"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-411
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-382
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1142
    type: references
---
# SR-1595002: gap analysis of QSL-983 (QSL PR 1595)

## Summary

Ticket: QSL-983. Plan completion: not assessed. `quoin matrix --json` (quoin 0.29.1) timed out
at 300 s on this repo, so the check of ACs against tests is manual: I read the `#[trace]` tags
and the test bodies at head 5ae5da93c.

Bindings found:

- `a_terminal_declaration_emits_its_node_and_the_i2_read_returns_it`, tagged TC-1142 and
  FR-411-AC-1: correct, with the fixture caveat in FND-002.
- `the_terminal_node_enters_the_package_id_and_its_written_name_does_not`, tagged TC-1142 and
  FR-411-AC-2: correct, covering the identity half.
- `a_terminal_declaration_changes_the_deadlock_item_identity`, tagged TC-1142 and FR-411-AC-2:
  correct, covering the deadlock-item half.
- `an_undefined_terminal_predicate_reproduces_and_a_deadlock_kind_settles_the_same_value`,
  tagged TC-539, FR-128-AC-6 and FR-411-AC-3: correct for the terminal half of FR-128-AC-6. The
  `pre CanInc` sentence stays with QSL-913.
- `a_counterexample_against_a_unit_with_another_terminal_declaration_is_refused`, tagged TC-1142
  and FR-411-AC-3: correct, partial (FND-003).
- `the_i2_read_refuses_a_malformed_terminal_node_at_its_locus`, tagged TC-1142 and FR-411-AC-4:
  correct.
- `only_the_refinement_beside_a_terminal_is_refused_not_emitted`, tagged TC-540: correct, but
  weak (FND-004).

None of these tests is a tautology. Each fails if the node were not emitted or not keyed:

- The AC-2 `package_id` inequalities need the node in the identity.
- The AC-3 `PackageIdMismatch` needs the `when` and `any` units to differ in identity.
- The AC-4 refusals come from IR's reader on edited bytes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No tagged test now covers FR-382-AC-6's second sentence: "with M naming a model, each checks and the package holds its temporal/terminal node (FR-411)". The PR deletes `a_checked_terminal_declaration_refuses_not_emitted_at_package_build` (TC-1059, FR-382-AC-6), which asserted the old refusal form of that sentence. The remaining FR-382-AC-6 test, `a_terminal_declaration_on_no_model_refuses_at_the_model_alias`, covers only the missing-model half, so the matrix reports the AC covered when half of it is asserted nowhere under its tag. Assert `when { true }` and `any` over a named model, each checking with a terminal node in the package, under `#[trace("TC-1059", "FR-382-AC-6")]`. Extending the FR-411-AC-1 test with a `when { true }` case and the tag also works. | spec/functional/FR-382-build-the-terminal-declaration-form-at-s2.md:88, qsl-semantics/src/check/assemble/tests.rs:610-632 |
| FND-002 | low | The FR-411-AC-1 and AC-2 package tests use the ConfigVersion unit and `self.versionNumber = 3`. AC-1 and TC-1142 name FR-126-AC-9's `Counter` unit and `self.value = 3`. The asserted shape is the same, so the substitution alone is acceptable. Two clauses are weaker than the AC states. First, `condition` is checked only to be some `expression` node, not the node of the predicate (an equality over `self.versionNumber` and 3). Second, the `self` parameter's type, `Reference<Config::ConfigVersion>` (FR-411 Behavior), is not checked. Assert the condition node's form and operands and the parameter's `semantic_type`, or move the test onto the Counter unit, as the qsl-analyze AC-2 and AC-3 tests already do. | qsl-package/src/emit/tests/terminal.rs:155-196 |
| FND-003 | low | FR-411-AC-3 is partly unasserted. The `PackageIdMismatch` test checks the code and cause but not that the refusal names both `package_id`s, which the AC requires. Neither AC-3 test asserts that the `when` unit "compiles to a package holding its terminal node": the replay test would also pass if S4 dropped the node silently, because replay reads the S3 value (SR-1595001 FND-001). Match the two ids in `ReplayRefusal::PackageIdMismatch`, and assert one `temporal`/`terminal` node in the recompiled package. | qsl-analyze/tests/it/model_settle.rs:960-1035 |
| FND-004 | low | The renamed refinement test asserts a single `(unsupported_construct, not-yet-implemented)` pair. It does not pin that the pair is `CheckCause::RefinementNotEmitted`, and `TimedNotChecked` yields the same pair. Match the cause variant, so the test says which declaration is refused. | qsl-semantics/tests/it/refinement_declarations.rs:1782-1804 |

## Verdict

FR-411-AC-1, AC-2 and AC-4 are asserted by their tagged tests, and so is the terminal half of
FR-128-AC-6. FR-411-AC-3 is asserted except for its "naming both package_ids" and "holding its
terminal node" clauses (FND-003). FR-382-AC-6's emission half lost its only tagged test
(FND-001). Every new code path traces to FR-411 or QSpec FR-476. The new code has no stub and
no dead branch, apart from the unread I2 reader recorded in SR-1595001 FND-001.

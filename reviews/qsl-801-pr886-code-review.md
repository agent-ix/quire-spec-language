---
id: SR-11682
title: "Code review — QSL-801 FR-363 non-Reference receiver in a function body (PR 886)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@498034d96d4cb44a39a6baabf29ca3a4b0ca7008; qsl-semantics/src/check/check.rs, qsl-semantics/tests/it/relationship_navigation.rs"
review_set: subset
---

## Summary

Ticket: QSL-801. Code review with the rust-review lane over the one-commit diff of PR 886 (`git diff origin/main...HEAD`), checked against FR-363, FR-300 and TC-1031. No build was run. The coder reports the all-features `it` binary passing (838 tests).

## Verdict

**PASS. No blocking findings.** `is_model_receiver` now sends an `Option` or collection of references, and any population, to the `StateModel` hook in every context. The Reference arm is unchanged. The match stays exhaustive, with no `_` arm. In the hook, `Form::Attribute` refuses a non-Reference `value_type` with `mismatch(operand_location)`. That is a `CheckCause::StateModel`-wrapped `ill_typed`/`type-mismatch` at `location.child(0)`, so the refusal sits on the receiver operand, as FR-363 and TC-1031 N13 state. A function-body root is path `[]`, so in `o.lines = o.lines` the receiver `o` is `[0,0]`, which is what the test asserts. Every production `Typer` has a `CheckContext`: the function body, the `decreases` measure, state clauses and `check_clause_expression`. So the hook cannot fault on a missing context. No existing behavior regressed: an Integer, a `Set<Integer>`, a tuple, a record and a bare `r.f` over a `Reference` still go to `field()`, and the unchanged tests `a_non_model_receiver_keeps_its_field_read_refusal` and `a_bare_field_read_in_a_function_body_keeps_its_refusal` pin them. `holds_references` is false for a record Option. No test or spec row expected the old Value-family refusal for an Option, a collection or a population of references in a function body; a git grep found none. Beyond relationship ends, an Option of references read with a field name (`o.code`) and `p.x` over a population also move to the StateModel operand refusal in a function body. That matches FR-363's "as it does for a field". `assert_refusal` checks code, cause, path, family and catalog prefix. The `assert_refused` refactor keeps every check. N13 fails under the old context-gated code, so it is not vacuous. The navigation tests sit behind `#[cfg(feature = "test-support")]`. Without that feature the module is skipped, not a compile error, and the `make ci` all-features lane runs it. The trace tags do not overclaim beyond FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | N14 is tagged FR-363-AC-1 (typing by multiplicity) but asserts only that the body checks, not the `Set<Reference<M::Line>>[0,3]` type. The vector is real (`Order` has no `lines` field, so only navigation admits it), but it was already true before this PR. Recorded | qsl-semantics/tests/it/relationship_navigation.rs:838-845 |
| FND-002 | low | The module doc still says "One test per vector N01-N12". Recorded | qsl-semantics/tests/it/relationship_navigation.rs:4 |

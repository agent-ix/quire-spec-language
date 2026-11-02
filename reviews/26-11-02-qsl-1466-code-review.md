---
id: SR-202611021417
title: "Code review of PR #2117: restate two qsl-eval tests that fed foreign checked expressions"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@99840350e; git diff origin/main...99840350e (PR #2117, branch qsl-1466): qsl-eval/src/value/expression/evaluate/invariants.rs, qsl-eval/tests/it/equality_matrix.rs, qsl-eval/tests/it/window_aggregates.rs; ticket QSL-1466"
review_set: subset
date: '2026-11-02'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: reviews
---
# SR-202611021417: code review of PR #2117 (QSL-1466)

## Summary

Ticket: QSL-1466. PR: quire-spec-language#2117, head 99840350e. Method:
code-review with the rust-review lane. The diff changes tests only. It
removes `equality_matrix::x04_an_unresolved_unit_past_admission_is_an_internal_fault`,
adds an in-crate producer probe
`invariants::an_unresolved_unit_past_admission_is_an_internal_fault`, and
re-checks the failing-call expression in
`window_aggregates::the_observed_source_reads_numbers_and_faults_on_anything_else`
against the `gap` package's own graph. No production code changed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| an_unresolved_unit_past_admission_is_an_internal_fault | qsl-eval/src/value/expression/evaluate/invariants.rs:744-774 | examined |
| x04 removal | qsl-eval/tests/it/equality_matrix.rs:2467-2505 (base) | examined |
| the_observed_source_reads_numbers_and_faults_on_anything_else | qsl-eval/tests/it/window_aggregates.rs:1265-1287 | examined |
| unresolved_unit producer and its four call sites | qsl-eval/src/value/expression/evaluate.rs:186-195, 2158, 2576, 2597, 3599 | context_only |
| CheckedPackage::evaluate foreign-expression guard (#2098) | qsl-eval/src/value/expression/mod.rs:1438-1447 | context_only |
| FR-090-AC-3 | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:605 | examined |
| FR-049-AC-2, FR-049-AC-3 | spec/functional/FR-049 (AC table lines 147-148) | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Pre-existing, outside the diff. `CheckedPackage::evaluate` does not admit the caller's `ObjectEnvironment` against the package's types (FR-049-AC-2, FR-049-AC-3). A closure built under a caller `TypeEnvironment` that gives field `q` a different unit is accepted. If the unit is outside the package's table, `e.q + e.q` and `e.q < e.q` return the S6a `quantity-unit-unresolved-past-admission` fault on caller input. If the unit is in the table, `e.q + e.q`, checked as `Quantity(metre)`, completes as `2 second`, which is a wrong result. | qsl-eval/src/value/expression/mod.rs:1448-1454; qsl-eval/src/value/expression/evaluate.rs:2318-2334 |
| FND-002 | medium | The new test's doc comment says "No caller input reaches this state". That is false: FND-001 reaches the fault through public `CheckedPackage::evaluate` with only caller-supplied objects. The PR therefore drops x04's public-API proof on a false premise. The same wrong claim is already at evaluate.rs:186-189. | qsl-eval/src/value/expression/evaluate/invariants.rs:748-751; qsl-eval/src/value/expression/evaluate.rs:186-189 |
| FND-003 | low | The trace tag `TC-384`/`FR-090-AC-3` was carried over from x04. FR-090-AC-3 and TC-384 list exactly two cases: a consumed environment and an unresolved checked identity. An unresolved quantity unit is neither, so the probe proves a producer that FR-090's producer table and TC-917 track (TC-917 step 5 names "independently named quantity-unit ... faults"). | qsl-eval/src/value/expression/evaluate/invariants.rs:753 |

## Verdict

The test changes are correct and do not weaken anything. FND-001 is a
production gap that predates this PR. It needs its own ticket and should not
block this test-only PR. FND-002 should be fixed in this PR by correcting the
comment. FND-003 is optional.

Evidence (head 99840350e, locked build lane):

- `cargo test -p qsl-eval --lib an_unresolved_unit_past_admission`: 1 passed.
- `cargo test -p qsl-eval --test it -- the_observed_source_reads`: passed.
- `cargo fmt --check -p qsl-eval` is clean, and
  `cargo clippy -p qsl-eval --all-targets -- -D warnings` is clean.
- Three mutations of `evaluate.rs` were each killed by the new test:
  - The ordering site (3599) returning a different invariant failed the
    second probe: `mutant-order` != expected.
  - The quantity-operation site (2158) returning a different invariant
    failed the first probe.
  - A `charge_call` inserted before unit resolution failed the probe's
    charge assertion: `[FunctionCall]` != `[]`.
- The new test therefore proves at least what x04 proved. It checks the
  stage S6a, the exact invariant, category InternalFailure,
  `kernel_cause == None`, and both the add and less-than paths. It also adds
  the no-charge assertion, which x04 did not have.
- Window test: `at(0)` returning `Defined(2)` shows that `gap_integer` is not
  refused as foreign. A foreign expression would fail at `at(0)`, which is
  what the regression did. The failure at `at(1)` is therefore the
  absent-object failed call, and `stopped()` is `Failed`. The test keeps what
  it proved.
- The deletion of x04 is justified for the route it used: #2098 rightly
  refuses a foreign checked expression. #2098 did not hide a regression,
  because the fault stays reachable only through the FND-001 object path.
- FND-001 was reproduced with a temporary integration test, reverted
  afterwards. It used `expression_package` with an object type `E { q:
  Quantity(metre) }` and the full unit table. The `ObjectClosure` was built
  under a caller `TypeEnvironment` whose `q` is `Quantity(kilogram)` from
  another unit graph, or `Quantity(second)`. The results:
  - With the kilogram unit, `Add` and `Less` both returned
    `Err(Fault(S6a, quantity-unit-unresolved-past-admission))`.
  - With the second unit, `Add` returned
    `Ok(Completed(Quantity { 2, second }))` for an expression typed
    `Quantity(metre)`.
- There is no `#[allow]`, no `#[ignore]`, and no weakened assertion.
  FR-090-AC-3 also keeps its canonical TC-384 tests at
  qsl-eval/src/value/expression/mod.rs:2322 and family.rs:426.

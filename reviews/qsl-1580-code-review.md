---
id: SR-20261104025134
title: "Code review — QSL-1580 predicate measures and rational nonzero guard (PR #2243)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c11158986; PR #2243 diff; qsl-cst/src/grammar.rs, qsl-forms/src/{syntax,value}.rs, qsl-semantics/src/check/facts.rs, tests/it/{call_requirements_items,config_version_spine}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-513
    type: reviews
---

## Summary

Ticket: QSL-1580. Code review with the rust-review lane, scoped to the PR diff. Quoin 0.29.1.

The grammar adds an optional `decreases(Expression)` to the `Predicate` production. The shared `function_like` reader already read the measure from the one `Expression` child that is not in the block, so predicates now carry it into the existing termination pass. That pass runs on functions and predicates alike.

The definedness checker now records QSpec FR-146's rational nonzero fact. `x != rational(0,1)` records it on the true outcome and `x = rational(0,1)` on the false outcome, for a rational stable path. A rational `/` accepts a divisor with that fact. When the divisor interval contains zero, `RationalDomain::result_of(Divide)` takes the hull of the magnitudes and a denominator of `Da_max × max|Nc|`. That matches FR-146's "zero removed" rule, so the newly reachable range path is sound.

Adverse probes were run in a scratch test, which was then reverted. Each was compiled through the spine:

- Refuse `unproved-nonzero`, as required: `x != 0r or r/x`; `if x != 0r then false else r/x`; `if x = 0r then r/x`; `not (x != 0r) and r/x`; a guard on another variable `y`; and a join `(if b then x != 0r else true) and r/x`.
- Admit, as FR-146 allows: `not (x = 0r)`; `0r != x`; `x != rational(0, 7)`, which normalises to zero; `implies`; and a field path `p.r`.
- Termination: mutual non-decreasing, one member missing a measure, a negative lower bound and a non-element measure each refuse `unproved-decrease`. A measured predicate/function mixed cycle admits, and so does a measure on a non-recursive predicate.

No guard shape admitted a possibly-zero divisor.

## Verdict

**MERGEABLE.** The code is sound. Both findings are low-severity test-strength nits.

## Scope

- qsl-cst/src/grammar.rs `Predicate` production: examined.
- qsl-forms/src/value.rs `function_like`: examined.
- qsl-semantics/src/check/facts.rs `guard` Equality arm, `rational_zero_comparand`, `Rational` Divide obligation: examined.
- tests/it/call_requirements_items.rs, tests/it/config_version_spine.rs new tests: examined.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `promoted` case of `run_report_separates_check_evidence_from_replay_evidence` asserts only `is_err()`, so any reader error, including one unrelated to the compile-stage rule, satisfies it. Pin the `NativeRunReadError::Invalid` member as the `relabelled` case does. | tests/it/config_version_spine.rs:1120-1123 |
| FND-002 | low | `recursive_predicate_cycles_with_decreasing_measures_admit_on_the_spine` tests a non-decreasing measure only on self-recursion. FR-513-AC-4 covers "the same cycles", but the mutual cycle `A`↔`B` passing `x` unchanged is not asserted. It does refuse correctly (probe). Add that case. | tests/it/call_requirements_items.rs:378-403 |

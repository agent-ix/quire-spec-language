---
id: SR-11179
title: "QSL-772 quire-spec-language PR 856 code review: reservation controls and Decimal type-link operands"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1166f187ce558cca83d0e0dcd1c0b05bf42a5c15; Cargo.lock, qsl-semantics/src/value/supplied.rs, qsl-eval/tests/it/supplied_phases.rs, qsl-replay/src/execute/tests/composite.rs"
review_set: subset
---

## Summary

Ticket: QSL-772 (closing PR). Code review with the rust-review lane, scoped
to `git diff origin/main...HEAD` (4 commits, head 1166f187). Read in place.
No build was run (brief: no builds). The coder's focused logs were not
re-run.

Checked:

- Pins. Cargo.lock moves QSV 542bc097 to d6fcf31c and quire-exact 9a3411d3
  to d18927e. Both are ancestors of their repos' origin/main.
- Production path. `with_reservation_permit` and the `permit` field exist
  only under `cfg(feature = "test-support")`. With the feature off, `admit`
  compiles to the same `admit_supplied_bounded_numeric` call as before. With
  the feature on and no permit, the `else` branch makes that same call.
  `test-support` is enabled only from `[dev-dependencies]` (root, qsl-eval,
  qsl-replay, qsl-package). The existing `no_shipped_dependency_enables_test_support`
  guard refuses a shipped edge.
- `bounded_failure_of`. A pure extraction of the old match. Every arm of
  `BoundedSuppliedAdmissionError` is named, with no wildcard. Each arm maps
  as before.
- Decimal endpoint operands. The four wire spellings match QSpec FR-323
  item 6 exactly. The match-arm order does not decide checking order. QSV
  does, and the public test pins the order: expected-upper at B=6 with
  expected-lower 1 bit; expected-lower at B=16; actual-lower with both
  expected endpoints within B; actual-upper with actual-lower within B.
- Reachability of Capacity. In QSV d6fcf31c `reserve_elements` and
  `append_child_owned` raise Capacity only when `len + additional` or the
  layout overflows, and that check runs before the permit. A `false` permit
  only ever yields Allocation. Decimal scratch failures come from
  `DecimalType::try_contains` on the global allocator, which never sees the
  permit, and its Capacity needs a coefficient wider than `usize` limbs.
  So mapping-only coverage of Capacity, DecimalAllocation and
  DecimalCapacity is honest.
- Panics. No `unwrap`, `expect`, `unreachable!` or indexing in new non-test
  code.

## Verdict

PASS. No blocking findings. Recorded (not blocking): under `test-support`
the boxed non-`Send` closure makes `SuppliedRun` `!Send`/`!Sync` in test
builds only; production auto traits are unchanged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

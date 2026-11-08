---
id: SR-13627
title: "FR-531 cold numeric boundary code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-package/src/emit/tests/native_row_budget.rs; FR-531-AC-33 incremental controls"
review_set: subset
---

## Summary

The frozen PR adds genuine value-asserting cold signed boundary controls. No defects were found in the scoped change.

## Verdict

PASS for this incremental PR; existing deferred acceptance work remains open.

Frozen head: `7361ebcb4768ecacb40ea265a870e36fe2051064`; base: `5ff999308`. Ticket: IR-1003. Scope is the changed cold numeric controls in `qsl-package/src/emit/tests/native_row_budget.rs`, not full-criterion or repository completeness.

## Examined scope

- FR-531-AC-33 (examined): Allocation observation encloses only the adapter/conversion call, with fixture preparation and assertions outside it. A successful owned materialization performs observed allocations; a denial before its first allocation performs none, and a later denial allocates neither the denied material nor an owned diagnostic. The controls include fresh owner-produced lazy numeric payloads, values `2^127` and `-(2^127 + 1)`, and larger heap-backed values without prior warming or formatting. These values remain exact on successful copies; insufficient work/storage retains the first real row locus and original bound.
- Changed tests: `cold_integer_payloads_refuse_inside_the_real_equality_adapter_without_allocating` and `fresh_signed_boundary_payloads_remain_exact_through_the_observed_equality_adapter`; TC-1331 bindings are correct, with incremental coverage only.

## Evidence

The typed declaration/check/link/emission route produces the real rows. `Emission` derives Clone over `Vec<EqualityFactWitnessRow>`; each row owns `Option<ValueType>`. Locked Exact `3aa2db57f79af92fb206dde6dc46e081d70d5812` clones `ValueType::Int` through owned `IntegerInterval` endpoints; `Integer::clone` resets the lazy cache, including LargeDigits. Thus the clones used for measurements and probes do not warm the independently cloned observed operands. No formatting or comparisons occur on observed clones before the call.

Controls include 2^127, -(2^127+1), and both signs of 2^130+1. The observed successful adapter result is independently compared with numeric endpoint oracles parsed afterward. Removing, truncating, swapping, or sign-changing the returned bounds fails those assertions. Empty typed output fails explicitly.

Zero work/storage denials observe zero allocation count and bytes, and preserve the actual first charge, locus and original bound. Later work/storage denials reach a real AdapterRow numeric operand after a positive paid prefix; the greatest bound selecting that row reaches its final applicable charge (integer upper-endpoint precharge). The real adapter preflights all charges before copying rows. Repeating the denied call retains error and consumed prefix with no allocation. Exact admission observes allocations and matches complete rows and exact consumption.

Trusted unchanged-source author logs: `/tmp/ir1003-fr531-native-row-tests-final.log` (8 passed), `/tmp/ir1003-fr531-clippy.log` (strict -D warnings completed). Reviewer independently ran `cargo fmt --all -- --check` and `git diff --check`, both passed. No expensive Cargo rebuild, aggregate gate or Kani run was repeated. Both source commits carry the required Claude Sonnet 5.5 trailer; PR ends with its nonempty Claude Code generated link.

## Limitations

AC-29..31 are already on main; no new credit. AC-32 remains deferred because the required public selected-monitor order/progress authority entry is absent. No production code, dependency, lockfile or harness changes. No full AC-33, class, cross-repository, or campaign completion claim. No applicable AssuranceProfile was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review methods

Code review with the Rust lane ran. Spec-review and optional specification analyses do not apply because no requirements or plans changed. Gap analysis is recorded separately.

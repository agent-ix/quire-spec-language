---
id: SR-2271001
title: "Code review: QSL-1492 FR-146 removal-filter decrease and quantity literal divisor (PR #2271)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@95585e8a8; PR #2271 diff: qsl-semantics/src/check/facts.rs, qsl-semantics/src/check/termination.rs, qsl-semantics/src/check/check.rs, qsl-semantics/src/check/type_form.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-eval/tests/it/removal_filter_decrease.rs, qsl-eval/tests/it/float_rounding.rs, qsl-eval/tests/it/float_callables.rs, qsl-eval/tests/it/equality_matrix.rs, qsl-eval/tests/it/main.rs"
review_set: subset
---

## Summary

Ticket: QSL-1492. Code review with the rust-review lane folded in, scoped to the PR #2271 diff at head 95585e8a8. QSpec is cited by ID only.

FR-146-AC-15 (removal filter). `membership` adds a `contains(c, v)` fact and narrows `size(c)` to `[1, +inf)` on the true outcome only. `join` intersects member sets per key, so only facts valid on every branch remain. `removal_filter` returns `Shrunk(p)` only when all of these hold:

- the filter source resolves to a bare parameter path;
- a membership fact for that parameter is present;
- the predicate is `x != e`, `e != x` or `not (x = e)` with `e` a stable path that is a known member, rooted away from the binder, over a non-float element; or it is `not contains(d, x)` where `d` is a stable path with the same element type and shares a known member with `p`;
- any optional convert is same-kind with the same element.

`termination.rs` treats `Shrunk` as a strict decrease only for a cardinality element. Slots are allocated monotonically per function (`fresh_slot`), so a fact cannot alias a reused slot. Each admitted shape is false for every occurrence equal to `e` under the membership relation, and `contains(p, e)` guarantees at least one such occurrence, so the bound count falls by at least one. The shapes are sound.

A probe in a scratch copy (deleted afterwards) gave these results:

- `contains(s, e) or size(s) > 5` refuses decrease.
- `not contains(t, x)` without a `contains(s, e)` fact refuses decrease.
- `Set<Set<Int>>` with `x != e` is admitted, which is correct because nested collection `=` is the membership relation.
- On a tuple with a `Float64` field, `!=` and `contains` both refuse `ill_typed/operator-ineligible` before the facts run.

FR-146-AC-9 (QSL part). A quantity division is now admitted only when the divisor node is a `Literal(Value::Quantity)` with nonzero magnitude. Checker typing gives `Quantity / Quantity` an unbounded `Quantity` result, so no range obligation is skipped. A parameter, a `let` name, a guarded parameter and a zero literal (`0 s`, `rational(0,1) s`, `0.0 s`) all still refuse, and the new test covers each. The admission is sound.

FR-149-AC-13 and AC-27 tests check what the ACs state. Rust: no unsafe, `expect` only in tests, `holds_float` is widened only to `pub(super)`.

## Verdict

**MERGEABLE.** The AC-15 removal-filter admission cannot admit a non-decrease in the current language, and the AC-9 admission is restricted to a nonzero quantity literal. The one low finding is latent defence in depth and does not block.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `x != e` gate uses `holds_float`, which does not descend into `Composite` element types. QSpec FR-146 requires "no Float32 or Float64 leaf". Today this cannot be reached: `=`, `!=` and `contains` on a float-carrying tuple refuse `operator-ineligible` before the facts run (probed). If FR-149 equality on float-carrying composites is ever admitted, `x != e` would count as a decrease while NaN fields keep the occurrence, so the recursion would not terminate. Fix: a float-leaf predicate that also walks composite fields, or a comment that pins the dependency on the operator refusal. | qsl-semantics/src/check/facts.rs:880 |

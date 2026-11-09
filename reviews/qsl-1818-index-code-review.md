---
id: SR-20261109175851
title: "code-review and rust-review of QSL-1818 clock place selectors (p[i], p[k])"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; git diff origin/main...qsl-1818-index (a58d9d0b6): qsl-forms syntax/value and value_forms test, qsl-semantics check/check.rs, check/typing.rs, family, hyper_clause, layer, assemble, checked_dispatch, clock_places test"
review_set: subset
---

## Summary

Ticket: QSL-1818 (Selection slice). The branch adds `ExprNode::Select { place, selector }`. S2 builds it for a `[s]` suffix that directly follows a field or another selector (`r.f[s]`); any other `[s]` (`xs[0]`) stays `UnrepresentedConstruct`. In the typer, `ref(...)` now walks fields and selectors, types each selector, and `clock_ref` matches selectors against the `Slot` steps of a clock place. A `Map` slot refuses `type-mismatch` (FR-230-AC-62, `ref` half). A non-`Integer` sequence index refuses `type-mismatch` at the selector. A valid indexed place refuses `unsupported-feature`, because the IR carrier is still missing. A `Select` anywhere outside a `ref` operand refuses `ill_typed`/`type-mismatch`.

The `ref` path is sound and its locations are right: the selector location is `at.child(1)` and the owner location is the innermost `child(0)`, and the test pins both. The open defects are about refusal causes and where S2 builds `Select`. S2's rule is syntactic: it builds `Select` after any field, so `o.lines[0]` in a function body builds. The typer then refuses every `Select` outside `ref` as `type-mismatch`. That cause is false for well-typed source. It covers the FR-230 "Selection" places that the checker SHALL admit (`self.ts[0].x >= 1 ms`, the AC-21 resets) and ordinary sequence indexing in a function body. Both are valid programs refused as user type errors where the honest interim is `unsupported-feature`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A `Select` outside a `ref` operand is refused `ill_typed`/`type-mismatch`. S2 now builds `Select` after any field, so this cause reaches valid source in two places. First, the FR-230 "Selection" places that the checker SHALL admit: a clock constraint `self.ts[0].x >= 1 ms` and FR-230-AC-21's resets `self.ts[1].x = 0 ms` / `self.m[k].x = 0 ms`, whose AC-7 says S2 builds them "for FR-230 to check". Second, ordinary indexing `o.lines[0]` in a function body, which was `UnrepresentedConstruct` before. These are unimplemented features, not type errors. The syntactic S2 rule is acceptable as an interim only if the typer is honest about it. Fix: refuse the `Select` arm `unknown_required_feature`/`unsupported-feature` at the selector, as `clock_ref` does for a valid indexed `ref`. Add a test that pins `o.lines[0]` in a function body and `self.ts[0].x >= 1 ms` in a clause to that cause | qsl-semantics/src/check/check/typing.rs:1524-1528, qsl-forms/src/value.rs:1784-1785 |
| FND-002 | medium | `clock_ref` refuses a path that names no place with `type-mismatch` whenever it holds a selector. So `ref(self.r.ts[0].x)`, a place under the reference field `r`, is called "not a place", while `ref(self.r.x)` is `unsupported-feature`. This repeats the FND-003 defect from the previous slice's review (SR-20261109101138), which was fixed for field-only paths. Fix: apply the `unsupported()` branch whenever `path.len() > 1`, whether or not selectors are present, or classify it by whether an intermediate field is a reference. Add `ref(self.r.ts[0].x)` to the test | qsl-semantics/src/check/check.rs:4437-4441 |
| FND-003 | low | The `ExprNode::Select` doc comment says "S2 builds it only as part of a clock place expression". That is false: S2 builds it after any field suffix, including `o.lines[0]` in a function body. The `value.rs` comment ("a clock place expression admits; a selector of anything else (`xs[0]`) is unrepresented") makes the same claim. Fix: state the actual rule, a selector that follows a field or selector, and say that the typer decides the rest | qsl-forms/src/syntax.rs:818-820, qsl-forms/src/value.rs:1780-1783 |
| FND-004 | low | The fallback span for a refused `[` searches `items` from the start of the postfix node and takes the first `]`. For `self.ts[0].n()[1]` the refusal span ends at `self.ts[0]` and does not cover the unrepresented `[1]`. Before this change it searched `rest`, after the failing `[`. Fix: snapshot `rest` (clone the iterator) before the two `next()` calls and search that | qsl-forms/src/value.rs:1797-1805 |
| FND-005 | low | `next_selector` was inserted between `unary`'s doc comment and `unary`. `next_selector` now begins with "Start a form over one operand, `goal`.", and `unary` has lost its doc | qsl-semantics/src/check/check/typing.rs:1092-1095 |
| FND-006 | low | `named_by` matches `(PlaceStep, read)` with a `_ => return None` arm. A future `PlaceStep` variant would silently be "no match" (then `type-mismatch`) instead of a compile error. Fix: list the mismatching pairs explicitly, for example `(PlaceStep::Field(_) \| PlaceStep::Union(_), _) \| (PlaceStep::Slot(_), _) => return None`, after the accepting arms | qsl-semantics/src/check/check.rs:2508 |
| FND-007 | low | Unrelated rustfmt churn in `integer_widens` and in a family.rs test (`argument_of`, the `NodeKind::Call` let-else). main already has fmt drift, so out-of-scope reformatting adds diff noise and possible merge conflicts. Fix: drop these hunks | qsl-semantics/src/check/check/typing.rs:557-563, qsl-semantics/src/check/family.rs:2779-2816 |

## Verdict

Changes requested on FND-001 and FND-002. The low findings can be fixed in the same round.

AC-7 claim: the forms test meets FR-091-AC-7's second sentence as written. Its body adds a `ref(self.q[0]) = ref(self.q[1])` clause, and the 5-Select count covers that clause too. The three AC-21 place expressions build, and `xs[0]` refuses `UnrepresentedConstruct` at its own span. The tag is honest. AC-7's phrase "builds the place expressions FR-230 checks" is met only at S2, because S3 refuses them `type-mismatch` (FND-001). The semantics test is tagged TC-685 only, and its doc says AC-62/24/43 are not fully met. That is honest.

Clean: the `Select` builder and shape arity, which mirror `Contains`. The exhaustive `Select` arms in syntax children, family encoding, hyper_clause (`check_match`, `is_opaque`, `leaf_payload`; the operands are appended after the payload, so `ts[0]` and `ts[1]` encode differently), layer, assemble and checked_dispatch, where FORM_NAMES is now 44. The `ClockRefFrame` state machine (owner first, then the selectors in path order). The `Map` slot refusal before the index-type check. The selector index type check (`Integer | Int(_)`). The `ref(self.ts[0])` refusal of a whole Timer element.

Branching on string edges: none was added. `name == read` compares declared field names, which is the place-identity check itself. No compatibility shims.

Gates run in this worktree: `cargo test -p qsl-forms --test it a_selector_of_a_clock_place` passes, and `cargo test -p qsl-semantics --features test-support --test it clock_places` passes (41 tests). quoin gap-analysis was not run, because the brief scoped this review to code-review plus rust-review.

Deferred by the coder and not counted as findings: the presence guard (`unproved-presence`), reset by selection (AC-21), and the IR carrier.

## Scope and evidence

- FR-091-AC-7: examined (the forms test at qsl-forms/tests/it/value_forms.rs:1741 and FND-003).
- FR-230 "Selection" (spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:729): examined (FND-001).
- FR-230-AC-62, the `ref` half: examined. The keyed `ref` refuses `type-mismatch` at the `ref`, and the test asserts location `[0]`.
- FR-230-AC-24, AC-43, AC-44 and AC-45: context_only. The IR carrier is blocked, and a valid `ref` stays `unsupported-feature`.

## Dispositions

Round 1, reviewed at d3254d3ee (fix commit d3254d3ee, parent a58d9d0b6).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d3254d3ee. The typer's `Select` arm now refuses `unknown_required_feature`/`unsupported-feature` at the selector (`location.child(1)`). In a timed-model state clause, `read_clause` refuses first with the new `TemporalCause::SelectionNotChecked` (`unsupported_construct`/`not-yet-implemented`) at the first selector outside a `ref`. This mirrors `PresenceNotChecked`, the existing "not checked in this position yet" cause, so it is the established honest interim and not a type error. The deviation is accepted. `a_selector_outside_a_ref_is_not_checked_yet` covers a guard read, an index reset, a keyed reset, a data read and a function body. The function-body assertion checks only that some refusal is `UnsupportedFeature`, not its location (an observation, not a new finding). |
| FND-002 | fixed | d3254d3ee: `clock_ref` refuses `unsupported()` for any `path.len() > 1` that names no place, with or without selectors. The test asserts `ref(self.r.ts[0].x)`, `ref(self.r.x)` and `ref(self.ts[0])` refuse `unsupported-feature` at `[0]`. |
| FND-003 | fixed | d3254d3ee: the `ExprNode::Select` doc and the value.rs comment state the actual rule: a `Select` builds only directly after a field access or another selector, and the checker decides whether the whole is a clock place expression. |
| FND-004 | fixed | d3254d3ee: the fallback span searches `ahead`, a clone of `rest` taken after the `[`. `a_selector_after_a_call_is_refused_at_its_own_span` asserts the span `self.ts[0].n()[1]`. |
| FND-005 | fixed | d3254d3ee: "Start a form over one operand, `goal`." is back on `unary`, and `next_selector` has only its own doc. |
| FND-006 | fixed | d3254d3ee: `named_by` lists the four mismatching `(PlaceStep, read)` pairs explicitly, and the match is exhaustive without a wildcard. |
| FND-007 | fixed | d3254d3ee: the `integer_widens` and family.rs test reformatting is reverted. Against origin/main, family.rs now carries only the `Select` encoding arm, and typing.rs no longer touches `integer_widens`. |

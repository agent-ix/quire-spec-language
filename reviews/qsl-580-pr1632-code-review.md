---
id: SR-1632001
title: "Code review of quire-spec-language PR #1632: FR-230 clock-reading declarations (580e)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ae196e6638e32dcf549b0619f7f92b1534d6f3ae; PR #1632 diff against qsl-580d-clock-types: qsl-semantics/src/check/assemble.rs, assemble/clock_forms.rs, check.rs, mod.rs, refusal.rs, timed_model.rs, timed_model/reading.rs; qsl-replay/src/spine.rs; qsl-semantics/tests/it/{timed_declarations,timed_model}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Code review of quire-spec-language PR #1632

## Summary

Ticket: QSL-580, slice 580e, stacked on #1629. The Rust review lane is folded
into this review. Reviewed only this PR's diff against `qsl-580d-clock-types`.

`TemporalCause::NotAPlaceExpression` is spec-backed. FR-230 "Parameters" and
"Results" require `ill_typed`/`type-mismatch` at a non-place argument or
body, and the new variant maps to exactly that code and tag. It is an
internal discriminant, not an invented wire cause.

The classification fixed point, the data-part rewrite of records, tuples and
unions, the DNF expansion with exclusive branch conditions, and the recursion
guard are sound. The expansion's work is charged to `PartBudget`, and
`check_declaration` uses an explicit work stack.

On whether a `Clock` inside an expression should refuse `TimedNotChecked`:
FR-230 does not say so. FR-230 admits a clock-bearing type "in every type
position a unit writes" and refuses "only an ill-formed use" with the causes
it names. Its stage boundary says S3 "does not leave a timed form unchecked".
`TimedNotChecked` (`unsupported_construct`/`not-yet-implemented`) is not one of
those causes (FND-001).

Gates: `cargo test -p qsl-semantics --all-features` ran at this head through
`locked-build.sh` and passed (781, 1284 and 30 tests, exit 0).

## Verdict

**CONDITIONAL.** No crash or wrong-acceptance bug was found. Four medium
findings concern spec-valid programs that get the wrong refusal, or a
not-yet-implemented refusal, where FR-230 defines the outcome.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Under the facet, a `Clock` inside an expression (`convert<Clock>(...)`, `allInstances`, accumulation types) refuses `TimedNotChecked`, and its whole clause or function is dropped. FR-230 names no not-yet-implemented outcome. It requires a whole clock-bearing read, including a conversion, to refuse `clock-read-outside-constraint`, and a literal with a clock component outside a postcondition to refuse `clock-write`. Use the named causes, or file the exact spec gap (a `Clock` type argument of `convert`/`allInstances`) on STD-493. The test `a_clock_inside_an_expression_is_not_checked` currently pins the placeholder. | qsl-semantics/src/check/mod.rs:1279-1293; qsl-semantics/src/check/assemble/clock_forms.rs:300-330; qsl-semantics/tests/it/timed_declarations.rs:800-834 |
| FND-002 | medium | `Expansion` returns `unexpanded` (`TimedNotChecked`) for clock-reading bodies that FR-230 defines and that `check_declaration` has already accepted. Examples: a data predicate other than a bare Boolean parameter (`predicate p(c: Clock, n: Integer): Boolean { n > 0 and c <= 3 ms }`, or `self.ready` in the body), a nested-call data argument that is neither a name nor a quantity literal, and an `if` condition that is a data comparison. FR-230 checks the body "by Clock constraints and guards": a Boolean combination of data predicates and atomic clock constraints. Such a unit passes the declaration check and then fails at every call site as not implemented. | qsl-semantics/src/check/timed_model/reading.rs:562-565, 595-602, 700-767 |
| FND-003 | medium | `Body::is_place` admits only `self.<clock field>`, a clock parameter, an `if` of places and a `Clock`-result call. FR-230 "Parameters" also makes "a place path ... from an object, record or tuple parameter" a place expression. As a result, `function h(o: Call): Clock { o.x }` refuses `ill_typed`/`type-mismatch` (NotAPlaceExpression), and a record-parameter read `t.x <= 3 ms` refuses `clock-read-outside-constraint`. Both are spec-valid. The second overlaps the AC-11 work already required. | qsl-semantics/src/check/timed_model/reading.rs:150-170, 228-240 |
| FND-004 | medium | A call of a clock-reading declaration is refused `clock-read-outside-constraint` only in a value function (`first_call`) and in a state clause or time-invariant/urgency predicate of a timed model. Elsewhere, for example a temporal clause, an operation body, or a clause of a model with no `time` member, the call reaches the value checker. The declaration has no function node there, so the call resolves as an unknown name. FR-230: "A call anywhere else SHALL refuse invalid_timed_model/clock-read-outside-constraint at the call." Found by reading the code; no test was run (confidence medium). | qsl-semantics/src/check/timed_model.rs:1316-1335; qsl-semantics/src/check/timed_model/reading.rs:811-824; qsl-semantics/src/check/check/typing.rs:1272-1290 |
| FND-005 | low | The FR-230-AC-10 `pick` test reduces each part's data predicate to its attribute names, so both parts read `["ready"]`. Putting `self.ready` instead of `not self.ready` on the `y >= 1` part, or giving both parts one polarity, would still pass. The AC requires `self.l` on one part and `not self.l` on the other. | qsl-semantics/tests/it/timed_model.rs:658-690 |
| FND-006 | low | `unexpanded` maps five unrelated conditions to one `TimedNotChecked { construct: Clock }`: a `Bearing` argument, recursion, a non-name data argument, a data predicate and a clock-selected place. Neither a test nor a caller can tell them apart. Also, `fn comparison` only wraps `ClockOp::of`, and the `let Some(op) = comparison(..) else { unexpanded }` in `boolean()` cannot be reached after its match guard. | qsl-semantics/src/check/timed_model/reading.rs:78-80, 360-368, 712-722 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | A recursive clock-reading declaration with no `decreases` measure (`predicate loop(c: Clock): Boolean { loop(c) }`, the fix round's test vector) refuses `TimedNotChecked { RecursiveClockReading }`, and STD-493 item 19 calls it a spec gap. The spec already defines this case: FR-091-AC-30 refuses a recursive function written without `decreases` with FR-146 `missing-measure`. A clock-reading declaration has no function node, so the value checker's measure check never reaches it. Item 19 is a real gap only for a measured recursive clock-reading declaration, whose expansion FR-230 does not define. | qsl-semantics/src/check/timed_model/reading.rs (`call`, `RecursiveClockReading`); spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1137 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit f74ba3fe5: `ClockSite::Expression(ClockUse)`. `convert`, `allInstances` and an accumulation's type refuse `clock-read-outside-constraint`, and a clock-bearing record literal outside a reset refuses `clock-write`. `allInstances` is filed as STD-493 item 18, and the test checks each use. |
| FND-002 | fixed | `Expansion::formed` grafts each data predicate, nested-call data argument and `if` data condition of the body at the call through `Expression::graft`, replacing parameters by their arguments. `boolean()` falls through to a data predicate instead of `unexpanded`, and `clock_reading_bodies_read_their_data_at_the_call` covers it. |
| FND-003 | deferred | Moved to slice 580h (coordinator ruling, 2026-10-26). The object-parameter half is fixed here: `is_clock_field` admits `o.x` for an object parameter called with `self` (`own(o: Config::Call): Clock { o.y }`). The record and tuple parameter paths move to 580h, which builds the place machinery they need. |
| FND-004 | fixed | `CheckCause::ClockReadingCall` (`invalid_timed_model`/`clock-read-outside-constraint`) is raised by the value checker for any call of a clock-reading declaration it meets, and operation bodies are scanned. The test covers a value function, a temporal clause, an operation body and a clause of a model with no `time` member. |
| FND-005 | fixed | `shown_parts` renders each part's predicate, and the test asserts `ready` on the `x >= 1` part and `not ready` on the `y >= 1` part. |
| FND-006 | fixed | `unexpanded` is replaced by `not_checked(TimedConstruct, span)` with the distinct constructs `ClockBearingArgument`, `RecursiveClockReading` and `ReferencedClock`; the other former cases now get their FR-230 causes. The `comparison` wrapper and the unreachable branch are removed. |
